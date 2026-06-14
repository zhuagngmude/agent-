// ---------------------------------------------------------------------------
// model_calls helper-only 写入草案
// 当前阶段 canWrite=false，不落盘，不接收 raw key/prompt/response
// 所有类型和函数预留给阶段 24，通过测试覆盖，非测试代码尚未调用。
// ---------------------------------------------------------------------------

use serde::Serialize;

/// 错误分类（13 类，以阶段 21 第七节为权威源）
#[derive(Debug, PartialEq)]
#[allow(dead_code)]  // 预留给阶段 24 写入 model_calls
pub enum ModelCallErrorCategory {
    FeatureDisabled,
    MissingKey,
    MissingBaseUrl,
    InvalidBaseUrl,
    UnsupportedProvider,
    UnsupportedModel,
    InvalidPurpose,
    ForbiddenField,
    Timeout,
    ProviderError,
    ResponseTooLarge,
    RedactionFailed,
    Unknown,
}

impl ModelCallErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelCallErrorCategory::FeatureDisabled => "feature_disabled",
            ModelCallErrorCategory::MissingKey => "missing_key",
            ModelCallErrorCategory::MissingBaseUrl => "missing_base_url",
            ModelCallErrorCategory::InvalidBaseUrl => "invalid_base_url",
            ModelCallErrorCategory::UnsupportedProvider => "unsupported_provider",
            ModelCallErrorCategory::UnsupportedModel => "unsupported_model",
            ModelCallErrorCategory::InvalidPurpose => "invalid_purpose",
            ModelCallErrorCategory::ForbiddenField => "forbidden_field",
            ModelCallErrorCategory::Timeout => "timeout",
            ModelCallErrorCategory::ProviderError => "provider_error",
            ModelCallErrorCategory::ResponseTooLarge => "response_too_large",
            ModelCallErrorCategory::RedactionFailed => "redaction_failed",
            ModelCallErrorCategory::Unknown => "unknown",
        }
    }
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]  // 预留给阶段 24
pub struct ModelCallDraft {
    pub can_write: bool,
    pub reason: String,
    pub draft_fields: Option<ModelCallDraftFields>,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]  // 预留给阶段 24
pub struct ModelCallDraftFields {
    pub id: String,
    pub project_id: String,
    pub purpose: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
    pub redaction_applied: bool,
    pub token_usage: String,
    pub cost_estimate: String,
    pub structured_summary: Option<String>,
    pub request_hash: Option<String>,
    pub duration_ms: Option<i64>,
    pub related_approval_id: Option<String>,
    pub runtime_event_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 构建 model_calls 写入草案。
/// 当前阶段 canWrite 恒为 false，不落盘。
#[allow(dead_code)]  // 预留给阶段 24
pub fn build_model_call_draft(
    project_id: &str,
    purpose: &str,
    provider: &str,
    model: &str,
    error_category: ModelCallErrorCategory,
    error_message: Option<&str>,
) -> ModelCallDraft {
    let now = chrono_now();
    let id = format!(
        "model_call_{}_{}_{}",
        purpose,
        now.replace('-', "").replace(':', "").replace('T', "_"),
        &uuid_suffix()
    );

    ModelCallDraft {
        can_write: false,
        reason: "阶段 23：model_calls 仅 helper-only 草案，feature_disabled 时不落盘。阶段 24/25 真实调用开启且调用确实发生后，canWrite 才为 true。".into(),
        draft_fields: Some(ModelCallDraftFields {
            id,
            project_id: project_id.into(),
            purpose: purpose.into(),
            provider: provider.into(),
            model: model.into(),
            status: "blocked".into(),
            error_category: Some(error_category.as_str().into()),
            error_message: error_message.map(|s| s.into()),
            redaction_applied: false,
            token_usage: "{}".into(),
            cost_estimate: "{}".into(),
            structured_summary: None,
            request_hash: None,
            duration_ms: None,
            related_approval_id: None,
            runtime_event_id: None,
            created_at: now.clone(),
            updated_at: now,
        }),
    }
}

fn chrono_now() -> String {
    // 简化时间戳，测试中可用固定值
    "2026-06-15T00:00:00Z".into()
}

fn uuid_suffix() -> String {
    "00000000".into()
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_can_write_is_false() {
        let draft = build_model_call_draft(
            "proj_1",
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            ModelCallErrorCategory::FeatureDisabled,
            Some("真实调用未开启"),
        );
        assert_eq!(draft.can_write, false, "阶段 23 的 canWrite 应为 false");
        assert!(
            draft.reason.contains("阶段 23"),
            "reason 应说明当前阶段"
        );
    }

    #[test]
    fn draft_does_not_contain_raw_secrets() {
        let draft = build_model_call_draft(
            "proj_1",
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            ModelCallErrorCategory::MissingKey,
            None,
        );

        let fields = draft.draft_fields.expect("draft_fields should be Some");
        // 验证 draft_fields 不包含 raw key 字段
        // ModelCallDraftFields 结构体本身就没有 key 字段，这是编译期保证

        // token_usage / cost_estimate 默认值为空 JSON
        assert_eq!(fields.token_usage, "{}");
        assert_eq!(fields.cost_estimate, "{}");

        // structured_summary 默认为 None
        assert!(fields.structured_summary.is_none());

        // request_hash 默认为 None
        assert!(fields.request_hash.is_none());
    }

    #[test]
    fn draft_status_is_blocked_when_feature_disabled() {
        let draft = build_model_call_draft(
            "proj_1",
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            ModelCallErrorCategory::FeatureDisabled,
            None,
        );

        let fields = draft.draft_fields.expect("draft_fields should be Some");
        assert_eq!(fields.status, "blocked");
        assert_eq!(fields.error_category.as_deref(), Some("feature_disabled"));
    }

    #[test]
    fn draft_has_all_expected_fields() {
        let draft = build_model_call_draft(
            "proj_1",
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            ModelCallErrorCategory::Unknown,
            None,
        );

        let f = draft.draft_fields.expect("draft_fields should be Some");
        assert!(f.id.starts_with("model_call_project_plan_generation_"));
        assert_eq!(f.project_id, "proj_1");
        assert_eq!(f.purpose, "project_plan_generation");
        assert_eq!(f.provider, "openai_compat");
        assert_eq!(f.model, "gpt-5.4-mini");
        assert_eq!(f.status, "blocked");
        assert_eq!(f.redaction_applied, false);
    }

    #[test]
    fn error_category_as_str_covers_all_13_variants() {
        let categories = vec![
            ModelCallErrorCategory::FeatureDisabled,
            ModelCallErrorCategory::MissingKey,
            ModelCallErrorCategory::MissingBaseUrl,
            ModelCallErrorCategory::InvalidBaseUrl,
            ModelCallErrorCategory::UnsupportedProvider,
            ModelCallErrorCategory::UnsupportedModel,
            ModelCallErrorCategory::InvalidPurpose,
            ModelCallErrorCategory::ForbiddenField,
            ModelCallErrorCategory::Timeout,
            ModelCallErrorCategory::ProviderError,
            ModelCallErrorCategory::ResponseTooLarge,
            ModelCallErrorCategory::RedactionFailed,
            ModelCallErrorCategory::Unknown,
        ];
        assert_eq!(categories.len(), 13);

        for cat in &categories {
            let s = cat.as_str();
            assert!(!s.is_empty(), "每个变体都应有非空字符串");
        }
    }
}
