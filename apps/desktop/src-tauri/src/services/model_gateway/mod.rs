pub mod model_calls;
pub mod project_plan;
pub mod provider_config;
pub mod redaction;

use serde::Serialize;

use crate::services::model_gateway::project_plan::validate_input;
use crate::services::model_gateway::provider_config::{
    resolve_provider_config, ProviderConfigStatus,
};
use crate::services::model_gateway::redaction::check_forbidden_value_patterns;

// ---------------------------------------------------------------------------
// 输入输出类型
// ---------------------------------------------------------------------------

#[derive(Serialize, PartialEq, Debug)]
#[allow(dead_code)] // InputRejected/DraftReady 预留给后续真实模型阶段
pub enum DraftStatus {
    FeatureDisabled,
    ProviderConfigError,
    InputRejected,
    DraftReady,
}

impl std::fmt::Display for DraftStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DraftStatus::FeatureDisabled => write!(f, "feature_disabled"),
            DraftStatus::ProviderConfigError => write!(f, "provider_config_error"),
            DraftStatus::InputRejected => write!(f, "input_rejected"),
            DraftStatus::DraftReady => write!(f, "draft_ready"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ProjectPlanModelDraftResponse {
    pub status: String,
    pub error_category: Option<String>,
    pub summary: Option<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// 主入口
// ---------------------------------------------------------------------------

pub fn create_project_plan_draft(
    idea: &str,
    constraints: &Option<String>,
    second_confirm: bool,
    confirm_text: &Option<String>,
) -> Result<ProjectPlanModelDraftResponse, String> {
    // 1. 先做输入基础校验和安全检查（不依赖 feature flag）
    validate_input(idea, constraints)?;
    check_forbidden_value_patterns(idea)?;
    if let Some(c) = constraints {
        check_forbidden_value_patterns(c)?;
    }

    // 2. 预留二次确认字段（后续真实模型阶段前不强制校验）
    let _ = second_confirm;
    let _ = confirm_text;

    // 3. 检查 feature flag
    let flag = std::env::var("AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN")
        .unwrap_or_else(|_| "false".into());

    if flag != "true" {
        return Ok(ProjectPlanModelDraftResponse {
            status: DraftStatus::FeatureDisabled.to_string(),
            error_category: Some("feature_disabled".into()),
            summary: None,
            warnings: vec![],
        });
    }

    // 4. 解析 provider 配置
    let config = resolve_provider_config();

    if config.status != ProviderConfigStatus::Configured {
        return Ok(ProjectPlanModelDraftResponse {
            status: DraftStatus::ProviderConfigError.to_string(),
            error_category: Some(
                match config.status {
                    ProviderConfigStatus::MissingKey => "missing_key",
                    ProviderConfigStatus::MissingBaseUrl => "missing_base_url",
                    ProviderConfigStatus::InvalidBaseUrl => "invalid_base_url",
                    _ => "provider_config_error",
                }
                .into(),
            ),
            summary: None,
            warnings: vec![],
        });
    }

    // 5. 阶段 22 到此为止——不发起网络请求，不写 SQLite
    let _ = config; // 使用 config 消掉未读字段 warning

    Ok(ProjectPlanModelDraftResponse {
        status: DraftStatus::FeatureDisabled.to_string(),
        error_category: Some("feature_disabled".into()),
        summary: Some("Model Gateway 脚手架已就绪，真实调用仍需后续准入阶段启用。".into()),
        warnings: vec![],
    })
}
