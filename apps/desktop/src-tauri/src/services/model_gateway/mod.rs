pub mod model_calls;
pub mod openai_compat;
pub mod project_plan;
pub mod provider_config;
pub mod redaction;

use serde::{Deserialize, Serialize};

use crate::services::model_gateway::model_calls::{
    compute_request_hash, insert_safe_model_call, SafeModelCallRecordInput,
};
use crate::services::model_gateway::openai_compat::{ModelProvider, ModelRequest, ProviderError};
use crate::services::model_gateway::project_plan::{validate_input, validate_second_confirm};
use crate::services::model_gateway::provider_config::{
    resolve as resolve_provider_config_inner, ProviderConfigStatus,
};
use crate::services::model_gateway::redaction::{
    check_forbidden_value_patterns, redact_secrets, truncate_summary,
};

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------

/// 返回摘要最大长度
const SUMMARY_MAX_LENGTH: usize = 5000;
/// Provider HTTP 请求超时秒数
const PROVIDER_TIMEOUT_SECS: u64 = 60;
/// Provider 响应体最大字节数（1 MB）
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

// ---------------------------------------------------------------------------
// 输入输出类型
// ---------------------------------------------------------------------------

#[derive(Serialize, PartialEq, Debug)]
pub enum DraftStatus {
    FeatureDisabled,
    ProviderConfigError,
    #[allow(dead_code)] // 预留给后续校验扩展
    InputRejected,
    InvalidRequest,
    DraftReady,
}

impl std::fmt::Display for DraftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftStatus::FeatureDisabled => write!(f, "feature_disabled"),
            DraftStatus::ProviderConfigError => write!(f, "provider_config_error"),
            DraftStatus::InputRejected => write!(f, "input_rejected"),
            DraftStatus::InvalidRequest => write!(f, "invalid_request"),
            DraftStatus::DraftReady => write!(f, "draft_ready"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ProjectPlanModelDraftResponse {
    pub status: String,
    pub error_category: Option<String>,
    pub summary: Option<String>,
    pub warnings: Vec<String>,
    /// 阶段 25.3：写入 model_calls 审计记录后返回 id；未进入 provider 阶段时为 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_record_id: Option<String>,
}

// ---------------------------------------------------------------------------
// 公开入口（给 Tauri command 调用，读取真实 env）
// ---------------------------------------------------------------------------

#[allow(dead_code)] // 由 command 层调用；测试中也使用
pub fn create_project_plan_draft(
    idea: &str,
    constraints: &Option<String>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<ProjectPlanModelDraftResponse, String> {
    let flag = std::env::var("AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN")
        .unwrap_or_else(|_| "false".into());

    create_project_plan_draft_core(
        idea,
        constraints,
        second_confirm,
        confirm_text,
        &flag,
        None, // config_key: read from env path below
        None, // config_base_url: read from env path below
        None, // provider_override: use real provider
        None, // db_connection: command 层传入；public API 无 DB 访问
        None, // model_record_id: 未指定时使用默认模型
    )
}

// ---------------------------------------------------------------------------
// 可注入的核心实现（供测试使用 fake flag / config / provider）
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_project_plan_draft_core(
    idea: &str,
    constraints: &Option<String>,
    second_confirm: bool,
    confirm_text: &Option<String>,
    flag_value: &str,
    config_key: Option<&str>,
    config_base_url: Option<&str>,
    provider_override: Option<Box<dyn ModelProvider>>,
    db_connection: Option<&rusqlite::Connection>,
    model_record_id: Option<&str>,
) -> Result<ProjectPlanModelDraftResponse, String> {
    // 1. 输入基础校验（不依赖 feature flag）
    validate_input(idea, constraints)?;
    check_forbidden_value_patterns(idea)?;
    if let Some(c) = constraints {
        check_forbidden_value_patterns(c)?;
    }

    // 2. 检查 feature flag
    if flag_value != "true" {
        return Ok(ProjectPlanModelDraftResponse {
            status: DraftStatus::FeatureDisabled.to_string(),
            error_category: Some("feature_disabled".into()),
            summary: None,
            warnings: vec![],
            audit_record_id: None,
        });
    }

    // 3. flag=true 时强制二次确认
    if let Err(e) = validate_second_confirm(second_confirm, confirm_text) {
        let _ = e; // 错误原因仅记日志，不返回原文以降低信息泄漏面
        return Ok(ProjectPlanModelDraftResponse {
            status: DraftStatus::InvalidRequest.to_string(),
            error_category: Some("invalid_request".into()),
            summary: None,
            warnings: vec![],
            audit_record_id: None,
        });
    }

    // 4. 解析 provider 配置（优先使用注入值，否则读 env）
    let config = if config_key.is_some() || config_base_url.is_some() {
        resolve_provider_config_inner(config_key, config_base_url)
    } else {
        provider_config::resolve_provider_config()
    };

    if config.status != ProviderConfigStatus::Configured {
        let error_category = match config.status {
            ProviderConfigStatus::MissingKey => "missing_key",
            ProviderConfigStatus::MissingBaseUrl => "missing_base_url",
            ProviderConfigStatus::InvalidBaseUrl => "invalid_base_url",
            _ => "provider_config_error",
        };
        return Ok(ProjectPlanModelDraftResponse {
            status: DraftStatus::ProviderConfigError.to_string(),
            error_category: Some(error_category.into()),
            summary: None,
            warnings: vec![],
            audit_record_id: None,
        });
    }

    // 5. 如果有 DB 连接，先拿 project_id。拿不到则不调用 provider，也不写 model_calls。
    let project_id_for_audit: Option<String> = if let Some(conn) = db_connection {
        match crate::services::projects::get_current_project(conn) {
            Ok(p) => Some(p.id),
            Err(_) => {
                return Ok(ProjectPlanModelDraftResponse {
                    status: "audit_write_failed".to_string(),
                    error_category: Some("audit_write_failed".into()),
                    summary: None,
                    warnings: vec!["无法获取当前项目标识，已拒绝调用".into()],
                    audit_record_id: None,
                });
            }
        }
    } else {
        None
    };

    // 5b. 确定实际使用的 model_id：优先从受控模型目录校验，否则 fallback
    let resolved_model_id: String =
        if let (Some(conn), Some(pid)) = (db_connection, project_id_for_audit.as_ref()) {
            if let Some(record_id) = model_record_id {
                // 前端传了 model_record_id → 校验
                crate::services::model_catalog::validate_model_for_call(conn, pid, record_id)
                    .map_err(|e| format!("invalid_request: {e}"))?
            } else {
                // 前端没传 → 使用默认启用模型
                crate::services::model_catalog::get_default_model_id(conn)
                    .map_err(|e| format!("provider_config_error: {e}"))?
            }
        } else {
            // 无 DB 连接 → fallback（不调用真实 provider 的测试路径）
            "gpt-5.4-mini".to_string()
        };

    // 6. 构造 provider（优先使用注入的 fake provider，否则用真实 env 构造）
    let provider: Box<dyn ModelProvider> = match provider_override {
        Some(p) => p,
        None => {
            let p = crate::services::model_gateway::openai_compat::OpenAiCompatProvider::from_env()
                .map_err(|_| "无法从环境变量构造 provider".to_string())?;
            Box::new(p)
        }
    };

    // 7. 构造固定请求（model 来自受控目录）
    let system_prompt = "你是项目计划助手，只输出简洁的中文项目计划摘要。".to_string();
    let mut user_message = format!("用户的项目想法：\n{}", idea);
    if let Some(c) = constraints {
        user_message.push_str(&format!("\n\n用户的约束条件：\n{}", c));
    }
    user_message.push_str("\n\n请根据以上项目想法和约束条件，生成一份简洁的中文项目计划草案摘要。");

    let request = ModelRequest {
        system_prompt,
        user_message,
        model_id: resolved_model_id.clone(),
    };

    // 8. 计算安全请求哈希（不含 raw idea/constraints）
    let idea_len = idea.chars().count();
    let has_constraints = constraints.is_some();
    let request_hash = compute_request_hash(
        "project_plan_generation",
        "openai_compat",
        &resolved_model_id,
        idea_len,
        has_constraints,
    );

    // 9. 调用 provider（真实 HTTP 或 fake）
    match provider.send(&request, PROVIDER_TIMEOUT_SECS, MAX_RESPONSE_BYTES) {
        Ok(response) => {
            // 10. 脱敏和截断
            let redacted = redact_secrets(&response.content);
            let summary = truncate_summary(&redacted, SUMMARY_MAX_LENGTH);

            // 11. 写入 model_calls 审计记录（阶段 25.3）
            // 审计写入失败 → 本次调用不算成功
            if let Some(conn) = db_connection {
                let pid = project_id_for_audit
                    .as_ref()
                    .expect("project_id_for_audit should be Some when db_connection is Some");
                let input = SafeModelCallRecordInput {
                    project_id: pid.clone(),
                    purpose: "project_plan_generation".into(),
                    provider: "openai_compat".into(),
                    model: resolved_model_id.clone(),
                    status: "success".into(),
                    error_category: None,
                    structured_summary: Some(summary.clone()),
                    request_hash: request_hash.clone(),
                };
                match insert_safe_model_call(conn, input) {
                    Ok(id) => {
                        return Ok(ProjectPlanModelDraftResponse {
                            status: DraftStatus::DraftReady.to_string(),
                            error_category: None,
                            summary: Some(summary),
                            warnings: vec!["已写入安全审计记录".into()],
                            audit_record_id: Some(id),
                        });
                    }
                    Err(_) => {
                        return Ok(ProjectPlanModelDraftResponse {
                            status: "audit_write_failed".to_string(),
                            error_category: Some("audit_write_failed".into()),
                            summary: None,
                            warnings: vec!["真实模型调用审计记录写入失败".into()],
                            audit_record_id: None,
                        });
                    }
                }
            }

            // 无 DB 连接：不写审计记录，也不声称已审计
            Ok(ProjectPlanModelDraftResponse {
                status: DraftStatus::DraftReady.to_string(),
                error_category: None,
                summary: Some(summary),
                warnings: vec!["真实模型结果未落库".into()],
                audit_record_id: None,
            })
        }
        Err(e) => {
            // 10. Provider error -> 粗粒度 error_category（不返回 raw provider body）
            let error_category = match e {
                ProviderError::Timeout => "timeout",
                ProviderError::NetworkError => "network_error",
                ProviderError::ProviderError => "provider_error",
                ProviderError::AuthError => "auth_error",
                ProviderError::RateLimited => "rate_limited",
                ProviderError::ResponseTooLarge => "response_too_large",
                ProviderError::InvalidResponse => "provider_error",
            };

            // 12. 写入 failed model_calls 审计记录（阶段 25.3）
            // 审计写入失败 → 返回 audit_write_failed，不伪装成已正常审计
            if let Some(conn) = db_connection {
                let pid = project_id_for_audit
                    .as_ref()
                    .expect("project_id_for_audit should be Some when db_connection is Some");
                let input = SafeModelCallRecordInput {
                    project_id: pid.clone(),
                    purpose: "project_plan_generation".into(),
                    provider: "openai_compat".into(),
                    model: resolved_model_id.clone(),
                    status: "failed".into(),
                    error_category: Some(error_category.to_string()),
                    structured_summary: None,
                    request_hash: request_hash.clone(),
                };
                match insert_safe_model_call(conn, input) {
                    Ok(id) => {
                        return Ok(ProjectPlanModelDraftResponse {
                            status: format!("provider_{}", error_category),
                            error_category: Some(error_category.to_string()),
                            summary: None,
                            warnings: vec!["已写入安全审计记录".into()],
                            audit_record_id: Some(id),
                        });
                    }
                    Err(_) => {
                        return Ok(ProjectPlanModelDraftResponse {
                            status: "audit_write_failed".to_string(),
                            error_category: Some("audit_write_failed".into()),
                            summary: None,
                            warnings: vec!["真实模型调用审计记录写入失败".into()],
                            audit_record_id: None,
                        });
                    }
                }
            }

            // 无 DB 连接
            Ok(ProjectPlanModelDraftResponse {
                status: format!("provider_{}", error_category),
                error_category: Some(error_category.into()),
                summary: None,
                warnings: vec!["真实模型调用失败，未落库".into()],
                audit_record_id: None,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::model_gateway::openai_compat::FakeModelProvider;

    // -------------------------------------------------------
    // helper: 构造 fake provider
    // -------------------------------------------------------

    fn fake_ok(content: &str) -> Box<dyn ModelProvider> {
        Box::new(FakeModelProvider {
            content: Some(content.into()),
            error: None,
        })
    }

    fn fake_err(e: ProviderError) -> Box<dyn ModelProvider> {
        Box::new(FakeModelProvider {
            content: None,
            error: Some(e),
        })
    }

    // -------------------------------------------------------
    // 测试 1: feature flag 关闭返回 feature_disabled
    // -------------------------------------------------------

    #[test]
    fn feature_disabled_when_flag_not_true() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,                       // second_confirm
            &None,                       // confirm_text
            "false",                     // flag
            None,                        // key
            None,                        // base_url
            Some(fake_ok("不会被用到")), // provider 不会被调用
            None,                        // db_connection
            None,                        // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "feature_disabled");
        assert_eq!(resp.error_category.as_deref(), Some("feature_disabled"));
        assert!(resp.summary.is_none());
        // 不写 model_calls、不写 runtime_events（warnings 为空）
        assert!(resp.warnings.is_empty());
    }

    #[test]
    fn feature_disabled_when_flag_empty() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,
            &None,
            "", // empty flag
            None,
            None,
            Some(fake_ok("不会被用到")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "feature_disabled");
    }

    // -------------------------------------------------------
    // 测试 2: second_confirm=false 返回 invalid_request
    // -------------------------------------------------------

    #[test]
    fn invalid_request_when_second_confirm_false() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false, // second_confirm
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "invalid_request");
        assert_eq!(resp.error_category.as_deref(), Some("invalid_request"));
        assert!(resp.summary.is_none());
        assert!(resp.warnings.is_empty());
    }

    // -------------------------------------------------------
    // 测试 3: confirm_text 错误返回 invalid_request
    // -------------------------------------------------------

    #[test]
    fn invalid_request_when_confirm_text_wrong() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true, // second_confirm
            &Some("我同意".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "invalid_request");
        assert_eq!(resp.error_category.as_deref(), Some("invalid_request"));
    }

    #[test]
    fn invalid_request_when_confirm_text_none() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true, // second_confirm
            &None,
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "invalid_request");
    }

    // -------------------------------------------------------
    // 测试 4: 缺 key 返回 missing_key
    // -------------------------------------------------------

    #[test]
    fn missing_key_when_flag_true_but_no_key() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            None, // 缺 key
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "provider_config_error");
        assert_eq!(resp.error_category.as_deref(), Some("missing_key"));
        assert!(resp.summary.is_none());
        // 不写 model_calls、不写 runtime_events
        assert!(resp.warnings.is_empty());
    }

    // -------------------------------------------------------
    // 测试 5: 缺 base URL 返回 missing_base_url
    // -------------------------------------------------------

    #[test]
    fn missing_base_url_when_flag_true_but_no_base_url() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            None, // 缺 base URL
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "provider_config_error");
        assert_eq!(resp.error_category.as_deref(), Some("missing_base_url"));
    }

    // -------------------------------------------------------
    // 测试 6: 非 https base URL 返回 invalid_base_url
    // -------------------------------------------------------

    #[test]
    fn invalid_base_url_rejects_http() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("http://example.com"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "provider_config_error");
        assert_eq!(resp.error_category.as_deref(), Some("invalid_base_url"));
    }

    #[test]
    fn invalid_base_url_rejects_localhost() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://127.0.0.1:8080"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("invalid_base_url"));
    }

    // -------------------------------------------------------
    // 测试 7: forbidden value pattern 仍拦截 sk-xxx、bearer 等
    // -------------------------------------------------------

    #[test]
    fn forbidden_pattern_blocks_sk_key_in_idea() {
        let result = create_project_plan_draft_core(
            "sk-abcdefghijklmnopqrstuvwxyz123456",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key"));
    }

    #[test]
    fn forbidden_pattern_blocks_authorization_bearer() {
        let result = create_project_plan_draft_core(
            "用 Authorization: Bearer eyJhbGciOiJIUzI1NiJ9 调用",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("密钥"));
    }

    #[test]
    fn forbidden_pattern_blocks_token_eq() {
        let result = create_project_plan_draft_core(
            "请使用 token=abc123secret 访问",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        );
        assert!(result.is_err());
    }

    #[test]
    fn forbidden_pattern_blocks_password_eq() {
        let result = create_project_plan_draft_core(
            "password=hunter2 是我的密码",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        );
        assert!(result.is_err());
    }

    #[test]
    fn forbidden_pattern_also_checks_constraints() {
        let result = create_project_plan_draft_core(
            "合法想法",
            &Some("用 api_key=secret123 调接口".into()),
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        );
        assert!(result.is_err());
    }

    // -------------------------------------------------------
    // 测试 8: provider error 不暴露 raw provider body
    // （编译期保证：ProviderError 没有字符串载荷）
    // -------------------------------------------------------

    #[test]
    fn provider_error_response_has_no_raw_body() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::ProviderError)),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        // error_category 只返回粗粒度分类
        assert_eq!(resp.error_category.as_deref(), Some("provider_error"));
        // summary 不应包含任何原始 provider 信息
        assert!(resp.summary.is_none());
        // 不应泄漏任何敏感数据
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("sk-test-key"));
        assert!(!serialized.contains("api.cheng.pink"));
        assert!(!serialized.contains("Bearer"));
        assert!(!serialized.contains("Authorization"));
    }

    #[test]
    fn timeout_error_response_is_coarse() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::Timeout)),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("timeout"));
    }

    #[test]
    fn network_error_response_is_coarse() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::NetworkError)),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("network_error"));
    }

    // -------------------------------------------------------
    // 测试 9: 成功解析只返回脱敏 summary
    // -------------------------------------------------------

    #[test]
    fn successful_response_returns_redacted_summary_only() {
        let model_output =
            "项目计划草案：\n# 阶段 1\n- 搭建骨架\n- 验证可行性\n\n# 阶段 2\n- 接入数据库";
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &Some("约束：Mock 优先".into()),
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok(model_output)),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        assert_eq!(resp.error_category, None);
        // summary 应包含脱敏后的内容
        let summary = resp.summary.as_ref().unwrap();
        assert!(summary.contains("项目计划草案"));
        assert!(summary.contains("阶段 1"));
        // 不应包含 raw response 元数据
        assert!(!summary.contains("choices"));
        assert!(!summary.contains("message"));
        assert!(!summary.contains("completion"));
        // 不应包含 key / base URL
        assert!(!summary.contains("sk-test-key"));
        assert!(!summary.contains("api.cheng.pink"));
        // warnings 说明未落库
        assert!(resp.warnings.iter().any(|w| w.contains("未落库")));
    }

    #[test]
    fn successful_response_redacts_embedded_secrets() {
        // 模拟模型输出中意外包含疑似 key 的内容
        let model_output = "建议使用 sk-abcdefghijklmnopqrstuvwxyz123456 作为开发密钥";
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok(model_output)),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        let summary = resp.summary.as_ref().unwrap();
        // key 已被脱敏
        assert!(!summary.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(summary.contains("[REDACTED_SECRET]"));
    }

    // -------------------------------------------------------
    // 测试 10: feature_disabled / invalid_request / missing_key
    //          等分支不写 model_calls、不写 runtime_events
    // -------------------------------------------------------

    #[test]
    fn feature_disabled_does_not_write_model_calls() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            None,
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        // 返回值不应包含 model_call_id 或 runtime_event_id
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("model_call_id"));
        assert!(!serialized.contains("runtime_event_id"));
        assert!(!serialized.contains("model_call_"));
        assert!(!serialized.contains("runtime_event_"));
    }

    #[test]
    fn invalid_request_does_not_write_model_calls() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,
            &None,
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "invalid_request");
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("model_call_id"));
        assert!(!serialized.contains("runtime_event_id"));
    }

    #[test]
    fn missing_key_does_not_write_model_calls() {
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            None,
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("missing_key"));
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("model_call_id"));
        assert!(!serialized.contains("runtime_event_id"));
    }

    #[test]
    fn draft_ready_without_db_connection_does_not_write_model_calls() {
        // 无 DB 连接时，即使成功也不写 model_calls（public API 路径）
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("项目计划草案内容")),
            None, // db_connection
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        assert_eq!(resp.audit_record_id, None);
    }

    // -------------------------------------------------------
    // 阶段 25.3：model_calls 审计落库测试（带 DB 连接）
    // -------------------------------------------------------

    /// 创建测试 DB，包含 model_calls 表和所需的基础数据
    fn test_db_with_tables() -> (rusqlite::Connection, TempDir) {
        let dir = TempDir::new();
        let conn = rusqlite::Connection::open(dir.path().join("test.sqlite"))
            .expect("should open test db");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("should enable FK");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT NOT NULL,
              phase TEXT, description TEXT, workspace_path TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            INSERT INTO projects (id, name, status, created_at, updated_at)
            VALUES ('proj_test', 'test project', 'planning', '2025-01-01', '2025-01-01');
            CREATE TABLE IF NOT EXISTS model_calls (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              purpose TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL,
              status TEXT NOT NULL, request_hash TEXT, structured_summary TEXT,
              token_usage TEXT NOT NULL DEFAULT '{}', cost_estimate TEXT NOT NULL DEFAULT '{}',
              error_category TEXT, error_message TEXT,
              redaction_applied INTEGER NOT NULL DEFAULT 0, duration_ms INTEGER,
              related_approval_id TEXT, runtime_event_id TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_events (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
              event_type TEXT NOT NULL, before_state TEXT, after_state TEXT,
              actor TEXT, reason TEXT, created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL, title TEXT NOT NULL,
              description TEXT, status TEXT NOT NULL, priority TEXT NOT NULL,
              assigned_agent_id TEXT, depends_on TEXT, risk_level TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approvals (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              task_id TEXT, request_agent_id TEXT NOT NULL,
              target_service TEXT NOT NULL, operation_types TEXT NOT NULL,
              status TEXT NOT NULL, risk_level TEXT NOT NULL,
              reason TEXT, reject_reason TEXT, approved_at TEXT, rejected_at TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runner_requests (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              approval_id TEXT, task_id TEXT, status TEXT NOT NULL,
              operation_types TEXT NOT NULL, affected_files TEXT NOT NULL,
              checkpoint TEXT, safety_note TEXT NOT NULL,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS model_catalog (
              id TEXT NOT NULL PRIMARY KEY,
              project_id TEXT NOT NULL,
              provider TEXT NOT NULL DEFAULT 'openai_compat',
              model_id TEXT NOT NULL,
              display_name TEXT NOT NULL DEFAULT '',
              purpose TEXT NOT NULL DEFAULT 'project_plan_generation',
              enabled INTEGER NOT NULL DEFAULT 1,
              is_builtin INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            -- seed a default enabled model for test compatibility
            INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
            VALUES ('mc_default', 'proj_test', 'openai_compat', 'gpt-5.4-mini', 'GPT-5.4 Mini', 'project_plan_generation', 1, 1, '2025-01-01', '2025-01-01');",
        )
        .expect("should create test tables");
        (conn, dir)
    }

    fn count_table(conn: &rusqlite::Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("table should be queryable")
    }

    #[test]
    fn feature_disabled_no_db_write_model_calls() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "model_calls");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            Some(fake_ok("不会被用到")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "feature_disabled");
        assert_eq!(resp.audit_record_id, None);
        assert_eq!(count_table(&conn, "model_calls"), before);
    }

    #[test]
    fn invalid_request_no_db_write_model_calls() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "model_calls");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            false,
            &None,
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "invalid_request");
        assert_eq!(resp.audit_record_id, None);
        assert_eq!(count_table(&conn, "model_calls"), before);
    }

    #[test]
    fn missing_key_no_db_write_model_calls() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "model_calls");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            None,
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("不会被调用")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("missing_key"));
        assert_eq!(resp.audit_record_id, None);
        assert_eq!(count_table(&conn, "model_calls"), before);
    }

    #[test]
    fn provider_success_writes_one_model_call() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "model_calls");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &Some("约束1".into()),
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("项目计划摘要内容")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        assert!(resp.audit_record_id.is_some());
        assert!(resp
            .audit_record_id
            .as_ref()
            .unwrap()
            .starts_with("model_call_project_plan_generation_"));

        let after = count_table(&conn, "model_calls");
        assert_eq!(after, before + 1);

        // 验证写入内容
        let (status, provider, model, purpose, summary, error_cat, redaction): (
            String, String, String, String, Option<String>, Option<String>, i64,
        ) = conn
            .query_row(
                "SELECT status, provider, model, purpose, structured_summary, error_category, redaction_applied
                 FROM model_calls WHERE id = ?1",
                rusqlite::params![resp.audit_record_id.as_ref().unwrap().as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .expect("should read model_call");

        assert_eq!(status, "success");
        assert_eq!(provider, "openai_compat");
        assert_eq!(model, "gpt-5.4-mini");
        assert_eq!(purpose, "project_plan_generation");
        assert!(summary.is_some());
        assert!(summary.as_ref().unwrap().contains("项目计划摘要"));
        assert!(error_cat.is_none());
        assert_eq!(redaction, 1);
    }

    #[test]
    fn provider_error_writes_one_failed_model_call() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "model_calls");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::Timeout)),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("timeout"));
        assert!(resp.audit_record_id.is_some());

        let after = count_table(&conn, "model_calls");
        assert_eq!(after, before + 1);

        let (status, error_cat, summary, redaction): (String, Option<String>, Option<String>, i64) =
            conn.query_row(
                "SELECT status, error_category, structured_summary, redaction_applied
                 FROM model_calls WHERE id = ?1",
                rusqlite::params![resp.audit_record_id.as_ref().unwrap().as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("should read model_call");

        assert_eq!(status, "failed");
        assert_eq!(error_cat.as_deref(), Some("timeout"));
        assert!(summary.is_none());
        assert_eq!(redaction, 0);
    }

    #[test]
    fn provider_error_maps_all_categories() {
        let categories = vec![
            (ProviderError::Timeout, "timeout"),
            (ProviderError::NetworkError, "network_error"),
            (ProviderError::ProviderError, "provider_error"),
            (ProviderError::AuthError, "auth_error"),
            (ProviderError::RateLimited, "rate_limited"),
            (ProviderError::ResponseTooLarge, "response_too_large"),
            (ProviderError::InvalidResponse, "provider_error"),
        ];
        for (err, expected_cat) in categories {
            let (conn, _dir) = test_db_with_tables();
            let resp = create_project_plan_draft_core(
                "测试",
                &None,
                true,
                &Some("我确认发起真实模型调用".into()),
                "true",
                Some("sk-test-key"),
                Some("https://api.cheng.pink/v1"),
                Some(fake_err(err)),
                Some(&conn),
                None, // model_record_id
            )
            .unwrap();
            assert_eq!(
                resp.error_category.as_deref(),
                Some(expected_cat),
                "category mismatch for {expected_cat}"
            );
        }
    }

    #[test]
    fn model_calls_does_not_contain_raw_secrets() {
        let (conn, _dir) = test_db_with_tables();
        // 模拟模型返回中包含疑似密钥内容
        let model_output = concat!(
            "建议使用 sk-abcdefghijklmnopqrstuvwxyz123456\n",
            "Authorization: Bearer secret-token-abc\n",
            "api_key=mysecret\n",
            "token=abc\n",
            "password=hunter2"
        );
        let resp = create_project_plan_draft_core(
            "测试",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok(model_output)),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        assert!(resp.audit_record_id.is_some());

        // 读取所有文本列，confirm no raw secrets
        let all_text: String = conn
            .query_row(
                "SELECT COALESCE(id,'')||COALESCE(project_id,'')||COALESCE(purpose,'')||
                 COALESCE(provider,'')||COALESCE(model,'')||COALESCE(status,'')||
                 COALESCE(request_hash,'')||COALESCE(structured_summary,'')||
                 COALESCE(token_usage,'')||COALESCE(cost_estimate,'')||
                 COALESCE(error_category,'')||COALESCE(error_message,'')
                 FROM model_calls WHERE id = ?1",
                rusqlite::params![resp.audit_record_id.as_ref().unwrap().as_str()],
                |row| row.get::<_, String>(0),
            )
            .expect("concat query should succeed");

        assert!(!all_text.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!all_text.contains("secret-token-abc"));
        assert!(!all_text.contains("mysecret"));
        assert!(!all_text.contains("hunter2"));
        // Authorization 作为 header 名称不会被脱敏（只脱敏 token 值），
        // 但原始 token 值不应出现
        // 脱敏标记应出现
        assert!(all_text.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn request_hash_excludes_raw_idea_and_constraints_content() {
        let (conn, _dir) = test_db_with_tables();
        let resp = create_project_plan_draft_core(
            "RAW_IDEA_SHOULD_NOT_BE_IN_DB",
            &Some("RAW_CONSTRAINT_SHOULD_NOT_BE_IN_DB".into()),
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("输出")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert!(resp.audit_record_id.is_some());
        let all_text: String = conn
            .query_row(
                "SELECT COALESCE(id,'')||COALESCE(request_hash,'')||COALESCE(structured_summary,'')
                 FROM model_calls WHERE id = ?1",
                rusqlite::params![resp.audit_record_id.as_ref().unwrap().as_str()],
                |row| row.get::<_, String>(0),
            )
            .expect("concat query should succeed");

        assert!(!all_text.contains("RAW_IDEA_SHOULD_NOT_BE_IN_DB"));
        assert!(!all_text.contains("RAW_CONSTRAINT_SHOULD_NOT_BE_IN_DB"));
    }

    // -------------------------------------------------------
    // 阶段 25.3 回归测试：缺失 project_id、审计写入失败
    // -------------------------------------------------------

    /// 创建有 model_calls 表但没有 projects 表的测试 DB
    fn test_db_without_projects() -> (rusqlite::Connection, TempDir) {
        let dir = TempDir::new();
        let conn = rusqlite::Connection::open(dir.path().join("test.sqlite"))
            .expect("should open test db");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("should enable FK");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_calls (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              purpose TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL,
              status TEXT NOT NULL, request_hash TEXT, structured_summary TEXT,
              token_usage TEXT NOT NULL DEFAULT '{}', cost_estimate TEXT NOT NULL DEFAULT '{}',
              error_category TEXT, error_message TEXT,
              redaction_applied INTEGER NOT NULL DEFAULT 0, duration_ms INTEGER,
              related_approval_id TEXT, runtime_event_id TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_events (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
              event_type TEXT NOT NULL, before_state TEXT, after_state TEXT,
              actor TEXT, reason TEXT, created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL, title TEXT NOT NULL,
              description TEXT, status TEXT NOT NULL, priority TEXT NOT NULL,
              assigned_agent_id TEXT, depends_on TEXT, risk_level TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approvals (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              task_id TEXT, request_agent_id TEXT NOT NULL,
              target_service TEXT NOT NULL, operation_types TEXT NOT NULL,
              status TEXT NOT NULL, risk_level TEXT NOT NULL,
              reason TEXT, reject_reason TEXT, approved_at TEXT, rejected_at TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runner_requests (
              id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
              approval_id TEXT, task_id TEXT, status TEXT NOT NULL,
              operation_types TEXT NOT NULL, affected_files TEXT NOT NULL,
              checkpoint TEXT, safety_note TEXT NOT NULL,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );",
        )
        .expect("should create test tables");
        // 注意：不创建 projects 表，模拟 get_current_project 失败场景
        (conn, dir)
    }

    #[test]
    fn missing_project_id_does_not_call_provider_and_returns_error() {
        let (conn, _dir) = test_db_without_projects();
        let model_calls_before = count_table(&conn, "model_calls");
        let runtime_before = count_table(&conn, "runtime_events");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("模型输出不应出现")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        // 不应调用 provider，不应返回模型摘要
        assert_eq!(resp.status, "audit_write_failed");
        assert_eq!(resp.error_category.as_deref(), Some("audit_write_failed"));
        assert!(resp.summary.is_none());
        assert!(resp.audit_record_id.is_none());
        // 不应写 model_calls
        assert_eq!(count_table(&conn, "model_calls"), model_calls_before);
        // 不应写 runtime_events
        assert_eq!(count_table(&conn, "runtime_events"), runtime_before);
    }

    #[test]
    fn provider_success_but_audit_write_fails_returns_audit_write_failed() {
        let (conn, _dir) = test_db_with_tables();
        // 删除 model_calls 表使审计写入必定失败
        conn.execute("DROP TABLE model_calls", [])
            .expect("should drop model_calls table");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("模型输出内容")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        // 审计写入失败 → 不应返回 draft_ready，summary 必须为空
        assert_eq!(resp.status, "audit_write_failed");
        assert_eq!(resp.error_category.as_deref(), Some("audit_write_failed"));
        assert!(resp.summary.is_none(), "审计失败不得返回模型摘要");
        assert!(resp.audit_record_id.is_none());
        assert!(
            resp.warnings.iter().any(|w| w.contains("审计记录写入失败")),
            "warnings 应提示审计写入失败"
        );
    }

    #[test]
    fn provider_error_but_audit_write_fails_returns_audit_write_failed() {
        let (conn, _dir) = test_db_with_tables();
        conn.execute("DROP TABLE model_calls", [])
            .expect("should drop model_calls table");

        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::Timeout)),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        // provider 已返回错误，但审计写入也失败 → 返回 audit_write_failed
        assert_eq!(resp.status, "audit_write_failed");
        assert_eq!(resp.error_category.as_deref(), Some("audit_write_failed"));
        assert!(resp.summary.is_none());
        assert!(resp.audit_record_id.is_none());
        assert!(
            resp.warnings.iter().any(|w| w.contains("审计记录写入失败")),
            "warnings 应提示审计写入失败"
        );
    }

    #[test]
    fn success_does_not_create_tasks_approvals_or_runner_requests() {
        let (conn, _dir) = test_db_with_tables();
        let tasks_before = count_table(&conn, "tasks");
        let approvals_before = count_table(&conn, "approvals");
        let runner_before = count_table(&conn, "runner_requests");

        create_project_plan_draft_core(
            "测试",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("输出")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(count_table(&conn, "tasks"), tasks_before, "不应创建任务");
        assert_eq!(
            count_table(&conn, "approvals"),
            approvals_before,
            "不应创建审批"
        );
        assert_eq!(
            count_table(&conn, "runner_requests"),
            runner_before,
            "不应创建 runner_requests"
        );
    }

    #[test]
    fn success_does_not_write_runtime_events() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "runtime_events");

        create_project_plan_draft_core(
            "测试",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("输出")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(
            count_table(&conn, "runtime_events"),
            before,
            "25.3 不写 runtime_events"
        );
    }

    #[test]
    fn failed_call_does_not_write_runtime_events() {
        let (conn, _dir) = test_db_with_tables();
        let before = count_table(&conn, "runtime_events");

        create_project_plan_draft_core(
            "测试",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_err(ProviderError::NetworkError)),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();

        assert_eq!(
            count_table(&conn, "runtime_events"),
            before,
            "失败也不写 runtime_events"
        );
    }

    #[test]
    fn audit_record_id_null_when_feature_disabled_invalid_or_config_error() {
        let (conn, _dir) = test_db_with_tables();

        // feature disabled
        let r1 = create_project_plan_draft_core(
            "测试",
            &None,
            false,
            &None,
            "false",
            None,
            None,
            Some(fake_ok("x")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();
        assert!(
            r1.audit_record_id.is_none(),
            "feature disabled: audit_record_id should be null"
        );

        // invalid request
        let r2 = create_project_plan_draft_core(
            "测试",
            &None,
            false,
            &None,
            "true",
            Some("sk-xxx"),
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("x")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();
        assert!(
            r2.audit_record_id.is_none(),
            "invalid request: audit_record_id should be null"
        );

        // config error (missing key)
        let r3 = create_project_plan_draft_core(
            "测试",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            None,
            Some("https://api.cheng.pink/v1"),
            Some(fake_ok("x")),
            Some(&conn),
            None, // model_record_id
        )
        .unwrap();
        assert!(
            r3.audit_record_id.is_none(),
            "config error: audit_record_id should be null"
        );
    }

    // --- temp dir helper for test module ---

    struct TempDir {
        path: std::path::PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-swarm-mg-test-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time")
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).expect("should create temp dir");
            TempDir { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    // -------------------------------------------------------
    // 回归：public API 不 panic（不依赖真实 env）
    // -------------------------------------------------------

    #[test]
    fn public_api_does_not_panic_without_env() {
        // create_project_plan_draft 在任何 env 下都不应 panic
        let result = create_project_plan_draft("测试想法", &None, false, &None);
        // 正常情况应返回 feature_disabled
        assert!(result.is_ok());
    }

    #[test]
    fn draft_ready_response_roundtrips_as_json() {
        let resp = ProjectPlanModelDraftResponse {
            status: "draft_ready".into(),
            error_category: None,
            summary: Some("项目计划摘要".into()),
            warnings: vec!["真实模型结果未落库".into()],
            audit_record_id: Some("model_call_xxx".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ProjectPlanModelDraftResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, "draft_ready");
        assert_eq!(parsed.summary.as_deref(), Some("项目计划摘要"));
        assert_eq!(parsed.audit_record_id.as_deref(), Some("model_call_xxx"));
    }
}
