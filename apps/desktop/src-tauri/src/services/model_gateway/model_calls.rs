// ---------------------------------------------------------------------------
// model_calls 审计落库
// 阶段 23：helper-only 草案（canWrite=false），不落盘。
// 阶段 25.3：真实模型调用进入 provider 阶段后写入 model_calls 审计记录。
// 仍不写 runtime_events、不传 raw key/prompt/response。
// ---------------------------------------------------------------------------

use rusqlite::params;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// 错误分类（13 类，以阶段 21 第七节为权威源）
#[derive(Debug, PartialEq)]
#[allow(dead_code)] // 预留给后续真实模型阶段写入 model_calls
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

// ---------------------------------------------------------------------------
// 阶段 25.3：安全审计写入
// ---------------------------------------------------------------------------

/// 安全模型调用记录输入——只包含脱敏字段，不含 raw key/base URL/prompt/response。
pub(crate) struct SafeModelCallRecordInput {
    pub project_id: String,
    pub purpose: String,
    pub provider: String,
    pub model: String,
    pub status: String, // "success" | "failed"
    pub error_category: Option<String>,
    pub structured_summary: Option<String>,
    pub request_hash: String,
}

/// 将一条脱敏后的 model_calls 审计记录写入 SQLite。
/// 返回值是写入记录的 id；不会写入 runtime_events、raw key、raw prompt 或 raw provider body。
pub(crate) fn insert_safe_model_call(
    conn: &rusqlite::Connection,
    input: SafeModelCallRecordInput,
) -> rusqlite::Result<String> {
    let id = generate_model_call_id(&input.purpose);
    let now = current_timestamp();
    let redaction_applied: i64 = if input.status == "success" { 1 } else { 0 };
    // 第一版 token_usage / cost_estimate 为空 JSON，不从 raw response 解析
    let token_usage = "{}";
    let cost_estimate = "{}";

    conn.execute(
        "INSERT INTO model_calls (
            id, project_id, purpose, provider, model, status,
            request_hash, structured_summary, token_usage, cost_estimate,
            error_category, error_message, redaction_applied, duration_ms,
            related_approval_id, runtime_event_id, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, NULL, NULL, NULL, ?13, ?13)",
        params![
            id.as_str(),
            input.project_id.as_str(),
            input.purpose.as_str(),
            input.provider.as_str(),
            input.model.as_str(),
            input.status.as_str(),
            input.request_hash.as_str(),
            input.structured_summary.as_deref(),
            token_usage,
            cost_estimate,
            input.error_category.as_deref(),
            redaction_applied,
            now.as_str(),
        ],
    )?;

    Ok(id)
}

/// 基于安全字段计算请求哈希。
/// 只使用 purpose/provider/model/idea 长度/constraints 是否存在。
/// 不包含 raw idea 原文、raw constraints 原文、key、base URL、header 或 provider body。
pub(crate) fn compute_request_hash(
    purpose: &str,
    provider: &str,
    model: &str,
    idea_len: usize,
    has_constraints: bool,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    purpose.hash(&mut hasher);
    provider.hash(&mut hasher);
    model.hash(&mut hasher);
    idea_len.hash(&mut hasher);
    has_constraints.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn generate_model_call_id(purpose: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("model_call_{purpose}_{nanos}")
}

/// 验证 model_calls 记录安全可用：
/// 属于当前项目、purpose/provider/model 匹配、success 状态、summary 非空、error_category 为空。
/// 返回该记录的 structured_summary，供上层写入 project_plan_drafts。
pub(crate) fn get_validated_summary(
    conn: &rusqlite::Connection,
    audit_record_id: &str,
    project_id: &str,
) -> Result<String, String> {
    let (db_project_id, db_purpose, db_provider, db_model, status, summary, error_category): (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT project_id, purpose, provider, model, status, structured_summary, error_category
             FROM model_calls WHERE id = ?1",
            rusqlite::params![audit_record_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|_| "not_found: audit_record_id not found".to_string())?;

    if db_project_id != project_id {
        return Err("invalid_input: audit_record does not belong to current project".into());
    }
    if db_purpose != "project_plan_generation" {
        return Err(format!(
            "invalid_input: audit_record purpose is {db_purpose}, must be project_plan_generation"
        ));
    }
    if db_provider != "openai_compat" {
        return Err(format!(
            "invalid_input: audit_record provider is {db_provider}, must be openai_compat"
        ));
    }
    // 阶段 35：校验 model 字段格式合法性（拒绝污染模型名）
    crate::services::model_catalog::validate_model_id(&db_model)
        .map_err(|e| format!("invalid_input: audit_record model validation failed: {e}"))?;
    if status != "success" {
        return Err(format!(
            "invalid_input: audit_record status is {status}, must be success"
        ));
    }
    if error_category.is_some() {
        return Err("invalid_input: audit_record has error, not eligible".into());
    }
    let summary =
        summary.ok_or_else(|| "invalid_input: audit_record has no summary".to_string())?;
    if summary.trim().is_empty() {
        return Err("invalid_input: audit_record summary is empty".into());
    }
    Ok(summary)
}

fn current_timestamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos.to_string()
}

// ---------------------------------------------------------------------------
// 阶段 23 保留：helper-only 草案（不落盘，canWrite=false）
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug)]
#[allow(dead_code)] // 预留给后续真实模型阶段
pub struct ModelCallDraft {
    pub can_write: bool,
    pub reason: String,
    pub draft_fields: Option<ModelCallDraftFields>,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)] // 预留给后续真实模型阶段
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
/// error_message 不接收自由文本——只从 error_category 枚举取值，
/// 确保 draft 中不出现 raw key、raw prompt、raw response 或 provider error 原文。
#[allow(dead_code)] // 预留给后续真实模型阶段
pub fn build_model_call_draft(
    project_id: &str,
    purpose: &str,
    provider: &str,
    model: &str,
    error_category: ModelCallErrorCategory,
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
        reason: "阶段 23：model_calls 仅 helper-only 草案，feature_disabled 时不落盘。后续真实调用开启且调用确实发生后，canWrite 才为 true。".into(),
        draft_fields: Some(ModelCallDraftFields {
            id,
            project_id: project_id.into(),
            purpose: purpose.into(),
            provider: provider.into(),
            model: model.into(),
            status: "blocked".into(),
            error_category: Some(error_category.as_str().into()),
            error_message: Some(error_category.as_str().into()),
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

/// 固定时间戳，仅用于阶段 23 测试草案。
/// 后续真实写入 model_calls 时必须替换为 chrono::Utc::now() 或等价实时时间源。
fn chrono_now() -> String {
    "2026-06-15T00:00:00Z".into()
}

/// 固定 ID 后缀，仅用于阶段 23 测试草案。
/// 后续真实写入必须替换为 uuid v4/v7 或等价唯一 ID 生成。
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
        );
        assert_eq!(draft.can_write, false, "阶段 23 的 canWrite 应为 false");
        assert!(draft.reason.contains("阶段 23"), "reason 应说明当前阶段");
    }

    #[test]
    fn draft_does_not_contain_raw_secrets() {
        let draft = build_model_call_draft(
            "proj_1",
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            ModelCallErrorCategory::MissingKey,
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

    // -------------------------------------------------------
    // 阶段 25.3：insert_safe_model_call / compute_request_hash 测试
    // -------------------------------------------------------

    #[test]
    fn insert_success_writes_one_row() {
        let (conn, _temp) = test_db();
        let before = count_model_calls(&conn);
        let input = SafeModelCallRecordInput {
            project_id: "proj_test".into(),
            purpose: "project_plan_generation".into(),
            provider: "openai_compat".into(),
            model: "gpt-5.4-mini".into(),
            status: "success".into(),
            error_category: None,
            structured_summary: Some("脱敏后的项目计划摘要".into()),
            request_hash: compute_request_hash(
                "project_plan_generation",
                "openai_compat",
                "gpt-5.4-mini",
                8,
                false,
            ),
        };
        let id = insert_safe_model_call(&conn, input).expect("insert should succeed");
        assert!(id.starts_with("model_call_project_plan_generation_"));
        assert_eq!(count_model_calls(&conn), before + 1);
    }

    #[test]
    fn insert_failed_writes_error_category_and_null_summary() {
        let (conn, _temp) = test_db();
        let input = SafeModelCallRecordInput {
            project_id: "proj_test".into(),
            purpose: "project_plan_generation".into(),
            provider: "openai_compat".into(),
            model: "gpt-5.4-mini".into(),
            status: "failed".into(),
            error_category: Some("network_error".into()),
            structured_summary: None,
            request_hash: compute_request_hash(
                "project_plan_generation",
                "openai_compat",
                "gpt-5.4-mini",
                8,
                false,
            ),
        };
        let id = insert_safe_model_call(&conn, input).expect("insert should succeed");
        assert!(id.starts_with("model_call_project_plan_generation_"));

        // 验证写入内容
        let (status, error_cat, summary, redaction): (String, Option<String>, Option<String>, i64) = conn
            .query_row(
                "SELECT status, error_category, structured_summary, redaction_applied FROM model_calls WHERE id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("should read back");
        assert_eq!(status, "failed");
        assert_eq!(error_cat.as_deref(), Some("network_error"));
        assert!(summary.is_none());
        assert_eq!(redaction, 0);
    }

    #[test]
    fn insert_does_not_contain_raw_secrets_in_fields() {
        let (conn, _temp) = test_db();
        // 模拟调用方已脱敏后的摘要（insert 不负责脱敏，只负责写入）
        let redacted_summary = "建议使用 [REDACTED_SECRET] 作为开发密钥\nAuthorization: Bearer [REDACTED_SECRET]\napi_key=[REDACTED_SECRET]\ntoken=[REDACTED_SECRET]\npassword=[REDACTED_SECRET]";
        let input = SafeModelCallRecordInput {
            project_id: "proj_test".into(),
            purpose: "project_plan_generation".into(),
            provider: "openai_compat".into(),
            model: "gpt-5.4-mini".into(),
            status: "success".into(),
            error_category: None,
            structured_summary: Some(redacted_summary.into()),
            request_hash: compute_request_hash(
                "project_plan_generation",
                "openai_compat",
                "gpt-5.4-mini",
                10,
                false,
            ),
        };
        let id = insert_safe_model_call(&conn, input).expect("insert should succeed");

        // 读回所有文本列，检查无原始敏感内容
        let all_text: String = conn
            .query_row(
                "SELECT COALESCE(id,'')||COALESCE(project_id,'')||COALESCE(purpose,'')||COALESCE(provider,'')||COALESCE(model,'')||COALESCE(status,'')||COALESCE(request_hash,'')||COALESCE(structured_summary,'')||COALESCE(token_usage,'')||COALESCE(cost_estimate,'')||COALESCE(error_category,'')||COALESCE(error_message,'') FROM model_calls WHERE id = ?1",
                params![id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .expect("concat query should succeed");

        // 已脱敏内容不应包含原始密钥值
        assert!(!all_text.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!all_text.contains("secret-token"));
        assert!(!all_text.contains("mysecret"));
        assert!(!all_text.contains("hunter2"));
        // REDACTED_SECRET 标记应该存在
        assert!(all_text.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn request_hash_excludes_raw_idea_and_constraints() {
        let h1 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            50,
            true,
        );
        let h2 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            50,
            true,
        );
        assert_eq!(h1, h2, "相同输入应产生相同哈希");

        // 不同 idea 长度产生不同哈希
        let h3 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            51,
            true,
        );
        assert_ne!(h1, h3, "不同 idea 长度应产生不同哈希");

        // has_constraints 不同产生不同哈希
        let h4 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            50,
            false,
        );
        assert_ne!(h1, h4, "constraints 标志不同应产生不同哈希");
    }

    #[test]
    fn request_hash_does_not_contain_raw_idea_text() {
        // 即使 idea 原文很长，hash 也不会暴露原文
        let h1 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            5,
            false,
        );
        let h2 = compute_request_hash(
            "project_plan_generation",
            "openai_compat",
            "gpt-5.4-mini",
            5000,
            false,
        );
        // 两者都是合法十六进制
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(h2.chars().all(|c| c.is_ascii_hexdigit()));
        // 都不包含文本 "RAW_IDEA" 这类模式
        assert!(!h1.contains("RAW_IDEA"));
    }

    #[test]
    fn insert_safe_model_call_does_not_write_runtime_events() {
        let (conn, _temp) = test_db();
        let before = conn
            .query_row("SELECT COUNT(*) FROM runtime_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count should work");
        let input = SafeModelCallRecordInput {
            project_id: "proj_test".into(),
            purpose: "project_plan_generation".into(),
            provider: "openai_compat".into(),
            model: "gpt-5.4-mini".into(),
            status: "success".into(),
            error_category: None,
            structured_summary: Some("摘要".into()),
            request_hash: compute_request_hash(
                "project_plan_generation",
                "openai_compat",
                "gpt-5.4-mini",
                8,
                false,
            ),
        };
        insert_safe_model_call(&conn, input).expect("insert should succeed");
        let after = conn
            .query_row("SELECT COUNT(*) FROM runtime_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count should work");
        assert_eq!(before, after, "25.3 不写 runtime_events");
    }

    // --- helpers ---

    fn count_model_calls(conn: &rusqlite::Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
            .expect("model_calls should be queryable")
    }

    fn test_db() -> (rusqlite::Connection, TempDir) {
        let dir = TempDir::new();
        let conn = rusqlite::Connection::open(dir.path().join("test.sqlite"))
            .expect("should open test db");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("should enable FK");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_calls (
              id                    TEXT PRIMARY KEY,
              project_id            TEXT NOT NULL,
              purpose               TEXT NOT NULL,
              provider              TEXT NOT NULL,
              model                 TEXT NOT NULL,
              status                TEXT NOT NULL,
              request_hash          TEXT,
              structured_summary    TEXT,
              token_usage           TEXT NOT NULL DEFAULT '{}',
              cost_estimate         TEXT NOT NULL DEFAULT '{}',
              error_category        TEXT,
              error_message         TEXT,
              redaction_applied     INTEGER NOT NULL DEFAULT 0,
              duration_ms           INTEGER,
              related_approval_id   TEXT,
              runtime_event_id      TEXT,
              created_at            TEXT NOT NULL,
              updated_at            TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS runtime_events (
              id          TEXT PRIMARY KEY,
              project_id  TEXT NOT NULL,
              entity_type TEXT NOT NULL,
              entity_id   TEXT NOT NULL,
              event_type  TEXT NOT NULL,
              before_state TEXT,
              after_state  TEXT,
              actor       TEXT,
              reason      TEXT,
              created_at  TEXT NOT NULL
            );",
        )
        .expect("should create test tables");
        (conn, dir)
    }

    struct TempDir {
        path: std::path::PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-swarm-mc-test-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
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
}
