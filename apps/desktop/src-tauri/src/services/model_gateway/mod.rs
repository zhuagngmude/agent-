pub mod model_calls;
pub mod openai_compat;
pub mod project_plan;
pub mod provider_config;
pub mod redaction;

use serde::{Deserialize, Serialize};

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
}

// ---------------------------------------------------------------------------
// 公开入口（给 Tauri command 调用，读取真实 env）
// ---------------------------------------------------------------------------

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
        });
    }

    // 5. 构造 provider（优先使用注入的 fake provider，否则用真实 env 构造）
    let provider: Box<dyn ModelProvider> = match provider_override {
        Some(p) => p,
        None => {
            let p = crate::services::model_gateway::openai_compat::OpenAiCompatProvider::from_env()
                .map_err(|_| "无法从环境变量构造 provider".to_string())?;
            Box::new(p)
        }
    };

    // 6. 构造固定请求
    let system_prompt = "你是项目计划助手，只输出简洁的中文项目计划摘要。".to_string();
    let mut user_message = format!("用户的项目想法：\n{}", idea);
    if let Some(c) = constraints {
        user_message.push_str(&format!("\n\n用户的约束条件：\n{}", c));
    }
    user_message.push_str("\n\n请根据以上项目想法和约束条件，生成一份简洁的中文项目计划草案摘要。");

    let request = ModelRequest {
        system_prompt,
        user_message,
    };

    // 7. 调用 provider（真实 HTTP 或 fake）
    match provider.send(&request, PROVIDER_TIMEOUT_SECS, MAX_RESPONSE_BYTES) {
        Ok(response) => {
            // 8. 脱敏和截断
            let redacted = redact_secrets(&response.content);
            let summary = truncate_summary(&redacted, SUMMARY_MAX_LENGTH);

            Ok(ProjectPlanModelDraftResponse {
                status: DraftStatus::DraftReady.to_string(),
                error_category: None,
                summary: Some(summary),
                warnings: vec!["真实模型结果未落库".into()],
            })
        }
        Err(e) => {
            // 9. Provider error -> 粗粒度 error_category（不返回 raw provider body）
            let error_category = match e {
                ProviderError::Timeout => "timeout",
                ProviderError::NetworkError => "network_error",
                ProviderError::ProviderError => "provider_error",
                ProviderError::ResponseTooLarge => "response_body_limit",
                ProviderError::InvalidResponse => "provider_error",
            };
            Ok(ProjectPlanModelDraftResponse {
                status: format!("provider_{}", error_category),
                error_category: Some(error_category.into()),
                summary: None,
                warnings: vec!["真实模型调用失败，未落库".into()],
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
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
            Some("https://api.openai.com"),
            Some(fake_err(ProviderError::ProviderError)),
        )
        .unwrap();

        // error_category 只返回粗粒度分类
        assert_eq!(resp.error_category.as_deref(), Some("provider_error"));
        // summary 不应包含任何原始 provider 信息
        assert!(resp.summary.is_none());
        // 不应泄漏任何敏感数据
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("sk-test-key"));
        assert!(!serialized.contains("api.openai.com"));
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
            Some("https://api.openai.com"),
            Some(fake_err(ProviderError::Timeout)),
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
            Some("https://api.openai.com"),
            Some(fake_err(ProviderError::NetworkError)),
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
            Some("https://api.openai.com"),
            Some(fake_ok(model_output)),
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
        assert!(!summary.contains("api.openai.com"));
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
            Some("https://api.openai.com"),
            Some(fake_ok(model_output)),
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
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
            Some("https://api.openai.com"),
            Some(fake_ok("不会被调用")),
        )
        .unwrap();

        assert_eq!(resp.error_category.as_deref(), Some("missing_key"));
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("model_call_id"));
        assert!(!serialized.contains("runtime_event_id"));
    }

    #[test]
    fn draft_ready_does_not_write_model_calls() {
        // 阶段 25.1 即使成功也不写 model_calls
        let resp = create_project_plan_draft_core(
            "测试项目想法",
            &None,
            true,
            &Some("我确认发起真实模型调用".into()),
            "true",
            Some("sk-test-key"),
            Some("https://api.openai.com"),
            Some(fake_ok("项目计划草案内容")),
        )
        .unwrap();

        assert_eq!(resp.status, "draft_ready");
        let serialized = serde_json::to_string(&resp).unwrap();
        assert!(!serialized.contains("model_call_id"));
        assert!(!serialized.contains("runtime_event_id"));
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
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ProjectPlanModelDraftResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, "draft_ready");
        assert_eq!(parsed.summary.as_deref(), Some("项目计划摘要"));
    }
}
