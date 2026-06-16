// ---------------------------------------------------------------------------
// 模型目录服务（阶段 35）
// 受控模型目录：前端只能从 enabled=true 的记录中选择模型。
// 后端负责校验 provider/purpose/enabled/project_id，不接收前端自由模型名。
// 不存储 raw key、base URL、prompt 或 provider error。
// ---------------------------------------------------------------------------

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::projects::get_current_project;

// ---------------------------------------------------------------------------
// 模型名校验
// ---------------------------------------------------------------------------

/// 校验模型名：非空、1-120 字符、只允许 ASCII 字母/数字/点/横线/下划线/斜杠/冒号。
/// 拒绝空格、换行、引号、花括号、反斜杠、路径穿越、URL、key/token/password 特征。
pub fn validate_model_id(model_id: &str) -> Result<(), String> {
    if model_id.is_empty() {
        return Err("invalid_input: model_id must not be empty".into());
    }
    if model_id.len() > 120 {
        return Err("invalid_input: model_id must be at most 120 characters".into());
    }

    // 只允许 ASCII 字母、数字、点、横线、下划线、斜杠、冒号
    for ch in model_id.chars() {
        if !ch.is_ascii_alphanumeric()
            && ch != '.'
            && ch != '-'
            && ch != '_'
            && ch != '/'
            && ch != ':'
        {
            return Err(format!(
                "invalid_input: model_id contains forbidden character '{}'",
                ch
            ));
        }
    }

    // 禁止常见敏感模式
    let lower = model_id.to_lowercase();
    let forbidden_substrings = [
        "sk-", "api_key", "token", "password", "secret", "bearer", "http://", "https://", "..",
        "\\", "~", "{", "}", "\n", "\r", "\t",
    ];
    for sub in &forbidden_substrings {
        if lower.contains(sub) {
            return Err(format!(
                "invalid_input: model_id contains forbidden pattern '{}'",
                sub
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 数据结构
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct ModelCatalogEntry {
    pub id: String,
    pub project_id: String,
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub purpose: String,
    pub enabled: bool,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateModelEnabledInput {
    pub model_record_id: String,
    pub enabled: bool,
    pub second_confirm: bool,
    pub confirm_text: String,
}

// ---------------------------------------------------------------------------
// 公开接口
// ---------------------------------------------------------------------------

/// 列出当前项目的所有模型目录条目
pub fn list_project_plan_models(connection: &Connection) -> Result<Vec<ModelCatalogEntry>, String> {
    let project_id = current_project_id(connection)?;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, provider, model_id, display_name, purpose,
                enabled, is_builtin, created_at, updated_at
             FROM model_catalog
             WHERE project_id = ?1 AND purpose = 'project_plan_generation'
             ORDER BY is_builtin DESC, enabled DESC, model_id",
        )
        .map_err(|e| format!("database_error: list model catalog failed: {e}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str()], map_model_row)
        .map_err(|e| format!("database_error: list model catalog failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list model catalog failed: {e}"))
}

/// 启用/停用模型。需要二次确认。不调用模型、不写 model_calls。
pub fn update_model_enabled(
    connection: &mut Connection,
    input: UpdateModelEnabledInput,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let project_id = current_project_id(connection)?;

    // 二次确认校验
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    let normalized_confirm = input.confirm_text.trim();
    if !normalized_confirm.contains("启用模型") && !normalized_confirm.contains("停用模型")
    {
        return Err("invalid_input: confirm_text must contain '启用模型' or '停用模型'".into());
    }

    // 校验 model_record_id
    let record_id = input.model_record_id.trim();
    if record_id.is_empty() || record_id.len() > 500 {
        return Err("invalid_input: model_record_id must be 1-500 characters".into());
    }

    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM model_catalog WHERE id = ?1 AND project_id = ?2",
            params![record_id, project_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|e| format!("database_error: check model exists failed: {e}"))?;
    if exists == 0 {
        return Err(format!("not_found: model record '{record_id}' not found"));
    }

    let now = current_timestamp();
    connection
        .execute(
            "UPDATE model_catalog SET enabled = ?1, updated_at = ?2
             WHERE id = ?3 AND project_id = ?4",
            params![
                input.enabled as i64,
                now.as_str(),
                record_id,
                project_id.as_str()
            ],
        )
        .map_err(|e| format!("database_error: update model enabled failed: {e}"))?;

    list_project_plan_models(connection)
}

/// 校验模型是否可用于调用（阶段 35 入口，purpose 固定 project_plan_generation）。
pub fn validate_model_for_call(
    connection: &Connection,
    project_id: &str,
    model_record_id: &str,
) -> Result<String, String> {
    validate_model_for_call_with_purpose(
        connection,
        project_id,
        model_record_id,
        "project_plan_generation",
    )
}

/// 校验模型是否可用于调用（阶段 37 扩展：支持传入 purpose）。
/// - provider=openai_compat
/// - purpose 必须匹配传入值
/// - enabled=1
/// - 当前 project_id 匹配
pub fn validate_model_for_call_with_purpose(
    connection: &Connection,
    project_id: &str,
    model_record_id: &str,
    expected_purpose: &str,
) -> Result<String, String> {
    let (db_project_id, db_provider, db_purpose, db_enabled, db_model_id): (
        String,
        String,
        String,
        i64,
        String,
    ) = connection
        .query_row(
            "SELECT project_id, provider, purpose, enabled, model_id
             FROM model_catalog WHERE id = ?1",
            params![model_record_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|_| "invalid_request: model record not found in catalog".to_string())?;

    if db_project_id != project_id {
        return Err("invalid_request: model record does not belong to current project".into());
    }
    if db_provider != "openai_compat" {
        return Err(format!(
            "invalid_request: model provider is {db_provider}, must be openai_compat"
        ));
    }
    if db_purpose != expected_purpose {
        return Err(format!(
            "invalid_request: model purpose is {db_purpose}, must be {expected_purpose}"
        ));
    }
    if db_enabled != 0 && db_enabled != 1 {
        return Err("invalid_state: model catalog enabled field is corrupt".into());
    }
    if db_enabled == 0 {
        return Err("invalid_request: model is not enabled".into());
    }

    // 二次校验模型名合法性
    validate_model_id(&db_model_id)?;

    Ok(db_model_id)
}

/// 获取默认启用的模型 model_id（purpose 固定 project_plan_generation）。
pub fn get_default_model_id(connection: &Connection) -> Result<String, String> {
    get_default_model_id_for_purpose(connection, "project_plan_generation")
}

/// 获取指定 purpose 的默认启用模型（阶段 37 扩展）。
pub fn get_default_model_id_for_purpose(
    connection: &Connection,
    purpose: &str,
) -> Result<String, String> {
    let project_id = current_project_id(connection)?;
    let model_id: String = connection
        .query_row(
            "SELECT model_id FROM model_catalog
             WHERE project_id = ?1 AND provider = 'openai_compat'
               AND purpose = ?2 AND enabled = 1
             ORDER BY is_builtin DESC, model_id
             LIMIT 1",
            params![project_id.as_str(), purpose],
            |row| row.get(0),
        )
        .map_err(|_| {
            format!("provider_config_error: no enabled model in catalog for purpose '{purpose}'")
        })?;
    Ok(model_id)
}

// ---------------------------------------------------------------------------
// Seed 内置模型
// ---------------------------------------------------------------------------

struct BuiltinModel {
    model_id: &'static str,
    display_name: &'static str,
    enabled: bool,
    #[allow(dead_code)]
    sort_order: i64,
}

pub fn seed_builtin_models(connection: &Connection) -> Result<(), String> {
    let project_id: String = connection
        .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
        .unwrap_or_else(|_| "project_agent_swarm".into());
    let now = "2025-01-01T00:00:00Z";

    let builtins: &[BuiltinModel] = &[
        BuiltinModel {
            model_id: "gpt-5.4-mini",
            display_name: "GPT-5.4 Mini",
            enabled: true,
            sort_order: 0,
        },
        BuiltinModel {
            model_id: "gpt-4o-mini",
            display_name: "GPT-4o Mini",
            enabled: false,
            sort_order: 1,
        },
        BuiltinModel {
            model_id: "gpt-4.1-mini",
            display_name: "GPT-4.1 Mini",
            enabled: false,
            sort_order: 2,
        },
        BuiltinModel {
            model_id: "gpt-5-mini",
            display_name: "GPT-5 Mini",
            enabled: false,
            sort_order: 3,
        },
    ];

    // 阶段 37：idea_guidance 和 project_seed_generation 两个新 purpose
    let all_purposes = [
        "project_plan_generation",
        "idea_guidance",
        "project_seed_generation",
    ];

    for purpose in &all_purposes {
        for m in builtins {
            let id = format!("model_catalog_{}_{}_{}", project_id, m.model_id, purpose);
            if let Err(e) = validate_model_id(m.model_id) {
                return Err(format!(
                    "database_error: builtin model validation failed: {e}"
                ));
            }
            connection
                .execute(
                    "INSERT OR IGNORE INTO model_catalog (
                        id, project_id, provider, model_id, display_name, purpose,
                        enabled, is_builtin, created_at, updated_at
                    ) VALUES (?1, ?2, 'openai_compat', ?3, ?4, ?5,
                        ?6, 1, ?7, ?7)",
                    params![
                        id.as_str(),
                        project_id.as_str(),
                        m.model_id,
                        m.display_name,
                        purpose,
                        m.enabled as i64,
                        now,
                    ],
                )
                .map_err(|e| format!("database_error: seed model catalog failed: {e}"))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn map_model_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModelCatalogEntry> {
    let enabled_raw: i64 = row.get(6)?;
    if enabled_raw != 0 && enabled_raw != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let is_builtin_raw: i64 = row.get(7)?;
    if is_builtin_raw != 0 && is_builtin_raw != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(ModelCatalogEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        provider: row.get(2)?,
        model_id: row.get(3)?,
        display_name: row.get(4)?,
        purpose: row.get(5)?,
        enabled: enabled_raw != 0,
        is_builtin: is_builtin_raw != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
    get_current_project(connection).map(|p| p.id)
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_db() -> (db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-mc-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let state = db::initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    // -------------------------------------------------------
    // model_id 校验
    // -------------------------------------------------------

    #[test]
    fn validate_model_id_accepts_valid_names() {
        assert!(validate_model_id("gpt-5.4-mini").is_ok());
        assert!(validate_model_id("gpt-4o-mini").is_ok());
        assert!(validate_model_id("claude-sonnet-4").is_ok());
        assert!(validate_model_id("deepseek-chat").is_ok());
        assert!(validate_model_id("gpt-4.1-mini").is_ok());
        assert!(validate_model_id("model_v2:latest").is_ok());
        assert!(validate_model_id("org/model-name").is_ok());
    }

    #[test]
    fn validate_model_id_rejects_empty() {
        assert!(validate_model_id("").is_err());
    }

    #[test]
    fn validate_model_id_rejects_too_long() {
        let long_name = "a".repeat(121);
        assert!(validate_model_id(&long_name).is_err());
    }

    #[test]
    fn validate_model_id_rejects_forbidden_patterns() {
        assert!(validate_model_id("sk-test-model").is_err());
        assert!(validate_model_id("api_key=mykey").is_err());
        assert!(validate_model_id("https://evil.com/model").is_err());
        assert!(validate_model_id("../secret-model").is_err());
        assert!(validate_model_id("model\nx").is_err());
        assert!(validate_model_id("{\"model\":\"x\"}").is_err());
        assert!(validate_model_id("token=abc").is_err());
        assert!(validate_model_id("password=hunter2").is_err());
        assert!(validate_model_id("bearer-token").is_err());
        assert!(validate_model_id("secret-model").is_err());
    }

    #[test]
    fn validate_model_id_rejects_spaces() {
        assert!(validate_model_id("gpt 5 mini").is_err());
    }

    // -------------------------------------------------------
    // migration 建表和索引存在
    // -------------------------------------------------------

    #[test]
    fn model_catalog_table_exists_with_expected_columns() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let mut stmt = conn
                .prepare("PRAGMA table_info('model_catalog')")
                .expect("should query table_info");
            let columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .expect("should map columns")
                .filter_map(|r| r.ok())
                .collect();
            let expected = vec![
                "id",
                "project_id",
                "provider",
                "model_id",
                "display_name",
                "purpose",
                "enabled",
                "is_builtin",
                "created_at",
                "updated_at",
            ];
            for col in &expected {
                assert!(
                    columns.contains(&col.to_string()),
                    "column {col} should exist"
                );
            }
            assert_eq!(
                columns.len(),
                10,
                "model_catalog should have exactly 10 columns"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn model_catalog_indexes_exist() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='model_catalog'")
                .expect("should query sqlite_master");
            let indexes: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .expect("should map indexes")
                .filter_map(|r| r.ok())
                .collect();
            assert!(
                indexes.iter().any(|i| i.contains("unique")),
                "should have unique index"
            );
            assert!(
                indexes.iter().any(|i| i.contains("enabled")),
                "should have enabled index"
            );
            assert!(
                indexes.iter().any(|i| i.contains("purpose")),
                "should have purpose index"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // seed 内置模型
    // -------------------------------------------------------

    #[test]
    fn seed_builtin_models_creates_entries_for_all_purposes() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_catalog", [], |row| row.get(0))
                .expect("count");
            assert_eq!(count, 12, "阶段37: 4个模型 × 3个purpose = 12条记录");

            // 默认只有 gpt-5.4-mini 在每个 purpose 下启用
            let enabled_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM model_catalog WHERE enabled = 1",
                    [],
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(enabled_count, 3, "3个purpose各1个enabled模型");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn seed_is_idempotent() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let before: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_catalog", [], |row| row.get(0))
                .expect("count");

            // 再 seed 一次
            seed_builtin_models(&conn).expect("second seed should succeed");
            let after: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_catalog", [], |row| row.get(0))
                .expect("count");
            assert_eq!(after, before, "seed should be idempotent");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // list_project_plan_models
    // -------------------------------------------------------

    #[test]
    fn list_returns_only_current_project_models() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");

            // 读取当前项目 ID
            let current_pid: String = conn
                .query_row(
                    "SELECT id FROM projects ORDER BY created_at LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .expect("current project");

            // 插入另一项目的模型（使用更晚的日期确保不成为当前项目）
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2099-01-01', '2099-01-01')",
                [],
            )
            .expect("insert other project");
            conn.execute(
                "INSERT INTO model_catalog (
                    id, project_id, provider, model_id, display_name, purpose,
                    enabled, is_builtin, created_at, updated_at
                ) VALUES ('mc_other', 'other_proj', 'openai_compat', 'other-model',
                    'Other Model', 'project_plan_generation', 1, 1,
                    '2025-01-01', '2025-01-01')",
                [],
            )
            .expect("insert other model");

            let models = list_project_plan_models(&conn).expect("list should succeed");
            // 所有返回的模型都应属于当前项目
            assert!(models.iter().all(|m| m.project_id == current_pid));
            // 不应包含另一项目的模型
            assert!(!models.iter().any(|m| m.id == "mc_other"));
            // 本项目的 4 个模型都在
            assert_eq!(models.len(), 4);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // update_model_enabled
    // -------------------------------------------------------

    #[test]
    fn update_enabled_requires_second_confirm() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("should exist");

            let err = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: true,
                    second_confirm: false,
                    confirm_text: "启用模型".into(),
                },
            )
            .expect_err("no second_confirm should fail");
            assert!(err.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_enabled_requires_correct_confirm_text() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("should exist");

            let err = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: true,
                    second_confirm: true,
                    confirm_text: "随便".into(),
                },
            )
            .expect_err("wrong confirm_text should fail");
            assert!(err.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_enabled_toggles_successfully() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("should exist");
            assert!(!gpt_4o.enabled);

            let result = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: true,
                    second_confirm: true,
                    confirm_text: "我确认启用模型 gpt-4o-mini".into(),
                },
            )
            .expect("enable should succeed");

            let updated = result
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("should exist");
            assert!(updated.enabled);

            // 再停用
            let result2 = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: false,
                    second_confirm: true,
                    confirm_text: "我确认停用模型 gpt-4o-mini".into(),
                },
            )
            .expect("disable should succeed");
            let disabled = result2
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("should exist");
            assert!(!disabled.enabled);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_rejects_unknown_model_record_id() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let err = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: "nonexistent_id".into(),
                    enabled: true,
                    second_confirm: true,
                    confirm_text: "启用模型".into(),
                },
            )
            .expect_err("unknown id should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_rejects_cross_project_model() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2099-01-01', '2099-01-01')",
                [],
            )
            .expect("insert other project");
            conn.execute(
                "INSERT INTO model_catalog (
                    id, project_id, provider, model_id, display_name, purpose,
                    enabled, is_builtin, created_at, updated_at
                ) VALUES ('mc_other_2', 'other_proj', 'openai_compat', 'other-model',
                    'Other', 'project_plan_generation', 1, 0,
                    '2025-01-01', '2025-01-01')",
                [],
            )
            .expect("insert other model");

            let err = update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: "mc_other_2".into(),
                    enabled: false,
                    second_confirm: true,
                    confirm_text: "停用模型".into(),
                },
            )
            .expect_err("cross-project should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // validate_model_for_call
    // -------------------------------------------------------

    #[test]
    fn validate_model_accepts_enabled_record() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let models = list_project_plan_models(&conn).expect("list");
            let gpt = models
                .iter()
                .find(|m| m.model_id == "gpt-5.4-mini")
                .expect("enabled builtin");
            let model_id = validate_model_for_call(&conn, "project_agent_swarm", &gpt.id)
                .expect("should validate");
            assert_eq!(model_id, "gpt-5.4-mini");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn validate_model_rejects_disabled_record() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("disabled");
            let err = validate_model_for_call(&conn, "project_agent_swarm", &gpt_4o.id)
                .expect_err("disabled should be rejected");
            assert!(err.contains("not enabled"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn validate_model_rejects_unknown_record() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let err = validate_model_for_call(&conn, "project_agent_swarm", "nonexistent")
                .expect_err("unknown should be rejected");
            assert!(err.contains("not found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn validate_model_rejects_cross_project() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            conn.execute(
                "INSERT INTO projects (id, name, status, created_at, updated_at)
                 VALUES ('other_proj', 'Other', 'planning', '2025-01-01', '2025-01-01')",
                [],
            )
            .expect("insert other project");
            conn.execute(
                "INSERT INTO model_catalog (
                    id, project_id, provider, model_id, display_name, purpose,
                    enabled, is_builtin, created_at, updated_at
                ) VALUES ('mc_cross', 'other_proj', 'openai_compat', 'cross-model',
                    'Cross', 'project_plan_generation', 1, 0,
                    '2025-01-01', '2025-01-01')",
                [],
            )
            .expect("insert cross model");

            let err = validate_model_for_call(&conn, "project_agent_swarm", "mc_cross")
                .expect_err("cross-project should fail");
            assert!(err.contains("does not belong"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn get_default_model_returns_enabled_model() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let default = get_default_model_id(&conn).expect("should have default");
            assert_eq!(default, "gpt-5.4-mini");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn get_default_model_fails_when_none_enabled() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            // 停用所有模型
            conn.execute("UPDATE model_catalog SET enabled = 0", [])
                .expect("disable all");

            let err = get_default_model_id(&conn).expect_err("should fail");
            assert!(err.contains("no enabled model"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 更新不写 model_calls / runtime_events
    // -------------------------------------------------------

    #[test]
    fn update_does_not_write_model_calls_or_runtime_events() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let model_calls_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM model_calls", [], |row| row.get(0))
                .expect("count");
            let runtime_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM runtime_events", [], |row| row.get(0))
                .expect("count");

            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("exists");
            update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: true,
                    second_confirm: true,
                    confirm_text: "启用模型 gpt-4o-mini".into(),
                },
            )
            .expect("should succeed");

            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM model_calls", [], |row| row
                    .get(0))
                    .expect("count"),
                model_calls_before,
                "不应写 model_calls"
            );
            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM runtime_events", [], |row| row
                    .get(0))
                    .expect("count"),
                runtime_before,
                "不应写 runtime_events"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_does_not_create_tasks_or_runner_requests() {
        let (state, test_dir) = test_db();
        {
            let mut conn = state.connection().expect("connection");
            let tasks_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                .expect("count");
            let rr_before: i64 = conn
                .query_row("SELECT COUNT(*) FROM runner_requests", [], |row| row.get(0))
                .expect("count");

            let models = list_project_plan_models(&conn).expect("list");
            let gpt_4o = models
                .iter()
                .find(|m| m.model_id == "gpt-4o-mini")
                .expect("exists");
            update_model_enabled(
                &mut conn,
                UpdateModelEnabledInput {
                    model_record_id: gpt_4o.id.clone(),
                    enabled: true,
                    second_confirm: true,
                    confirm_text: "启用模型 gpt-4o-mini".into(),
                },
            )
            .expect("should succeed");

            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
                    .expect("count"),
                tasks_before,
                "不应创建 tasks"
            );
            assert_eq!(
                conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM runner_requests", [], |row| row
                    .get(0))
                    .expect("count"),
                rr_before,
                "不应创建 runner_requests"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 阶段 35 返修：CHECK 约束 / 脏数据拒绝
    // -------------------------------------------------------

    /// 创建不含 CHECK 约束的 model_catalog 表，用于注入脏数据测试
    fn dirty_db() -> (rusqlite::Connection, TempDir) {
        let dir = TempDir::new();
        let conn =
            rusqlite::Connection::open(dir.path.join("test.sqlite")).expect("should open test db");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("should enable FK");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
              id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT NOT NULL,
              phase TEXT, description TEXT, workspace_path TEXT,
              created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            INSERT INTO projects (id, name, status, created_at, updated_at)
            VALUES ('proj_test', 'test', 'planning', '2025-01-01', '2025-01-01');
            -- 注意：此处故意不加 CHECK(enabled IN (0,1))，用于测试脏数据拒绝
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
            );",
        )
        .expect("should create tables");
        (conn, dir)
    }

    struct TempDir {
        path: std::path::PathBuf,
    }
    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agent-swarm-mc-dirty-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system time")
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).expect("should create temp dir");
            TempDir { path }
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn schema_rejects_enabled_2() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            // 直接 INSERT enabled=2，CHECK 应拒绝
            let result = conn.execute(
                "INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
                 VALUES ('mc_bad_enabled', 'project_agent_swarm', 'openai_compat', 'bad-model', 'Bad', 'project_plan_generation', 2, 0, '2025-01-01', '2025-01-01')",
                [],
            );
            assert!(result.is_err(), "enabled=2 should be rejected by CHECK");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn schema_rejects_is_builtin_2() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let result = conn.execute(
                "INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
                 VALUES ('mc_bad_builtin', 'project_agent_swarm', 'openai_compat', 'bad-model-2', 'Bad', 'project_plan_generation', 0, 2, '2025-01-01', '2025-01-01')",
                [],
            );
            assert!(result.is_err(), "is_builtin=2 should be rejected by CHECK");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_rejects_dirty_enabled_in_db() {
        let (conn, _dir) = dirty_db();
        conn.execute(
            "INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
             VALUES ('mc_dirty', 'proj_test', 'openai_compat', 'dirty-model', 'Dirty', 'project_plan_generation', 2, 0, '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("insert without CHECK should succeed");

        // 用当前 project 查询
        let result = list_project_plan_models(&conn);
        assert!(result.is_err(), "dirty enabled should cause list to fail");
        assert!(
            result.unwrap_err().contains("database_error"),
            "should be database_error"
        );
    }

    #[test]
    fn list_rejects_dirty_is_builtin_in_db() {
        let (conn, _dir) = dirty_db();
        conn.execute(
            "INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
             VALUES ('mc_dirty2', 'proj_test', 'openai_compat', 'dirty-model-2', 'Dirty', 'project_plan_generation', 1, 3, '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("insert without CHECK should succeed");

        let result = list_project_plan_models(&conn);
        assert!(
            result.is_err(),
            "dirty is_builtin should cause list to fail"
        );
    }

    #[test]
    fn validate_model_for_call_rejects_enabled_2() {
        let (conn, _dir) = dirty_db();
        conn.execute(
            "INSERT INTO model_catalog (id, project_id, provider, model_id, display_name, purpose, enabled, is_builtin, created_at, updated_at)
             VALUES ('mc_dirty3', 'proj_test', 'openai_compat', 'dirty-model-3', 'Dirty', 'project_plan_generation', 2, 0, '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("insert without CHECK should succeed");

        let result = validate_model_for_call(&conn, "proj_test", "mc_dirty3");
        assert!(result.is_err(), "dirty enabled should be rejected");
        assert!(
            result.unwrap_err().contains("invalid_state"),
            "should be invalid_state"
        );
    }
}
