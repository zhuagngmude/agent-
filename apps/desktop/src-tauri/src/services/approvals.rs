use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct ApprovalSummary {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub request_agent_id: String,
    pub target_service: String,
    pub operation_types: Vec<String>,
    pub status: String,
    pub risk_level: String,
    pub reason: Option<String>,
    pub reject_reason: Option<String>,
    pub approved_at: Option<String>,
    pub rejected_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApprovalInput {
    #[serde(default)]
    pub task_id: Option<String>,
    pub request_agent_id: String,
    pub target_service: String,
    pub operation_types: Vec<String>,
    pub risk_level: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateApprovalResponse {
    pub approval: ApprovalSummary,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalIdInput {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectApprovalInput {
    pub id: String,
    #[serde(default)]
    pub reject_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovalTransitionResponse {
    pub approval: ApprovalSummary,
}

pub fn list_approvals(connection: &Connection) -> Result<Vec<ApprovalSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals
             ORDER BY created_at, id",
        )
        .map_err(|error| format!("database_error: read approval list failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let operation_types_json: String = row.get(5)?;

            Ok(ApprovalSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_id: row.get(2)?,
                request_agent_id: row.get(3)?,
                target_service: row.get(4)?,
                operation_types: parse_string_list(&operation_types_json),
                status: row.get(6)?,
                risk_level: row.get(7)?,
                reason: row.get(8)?,
                reject_reason: row.get(9)?,
                approved_at: row.get(10)?,
                rejected_at: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .map_err(|error| format!("database_error: read approval list failed: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: read approval list failed: {error}"))
}

pub fn create_approval(
    connection: &mut Connection,
    input: CreateApprovalInput,
) -> Result<CreateApprovalResponse, String> {
    let project_id = get_current_project_id(connection)?;
    let task_id = normalize_optional_id(input.task_id);
    let request_agent_id =
        normalize_required_text(input.request_agent_id, 1, 200, "request_agent_id")?;
    let target_service = normalize_enum(
        input.target_service,
        &[
            "task",
            "approval",
            "runner",
            "agent_config",
            "model_gateway",
        ],
        "target_service",
    )?;
    let operation_types = normalize_operation_types(input.operation_types)?;
    let risk_level = normalize_enum(input.risk_level, &["low", "medium", "high"], "risk_level")?;
    let reason = normalize_optional_text(input.reason, 2000, "reason")?;

    ensure_agent_belongs_to_project(connection, &project_id, &request_agent_id)?;
    if let Some(task_id) = task_id.as_deref() {
        ensure_task_belongs_to_project(connection, &project_id, task_id)?;
    }

    let operation_types_json = serde_json::to_string(&operation_types)
        .map_err(|error| format!("database_error: serialize operation types failed: {error}"))?;
    let id = generate_approval_id();
    let now = current_timestamp();

    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start create approval transaction failed: {error}")
    })?;
    tx.execute(
        "INSERT INTO approvals (
            id, project_id, task_id, request_agent_id, target_service, operation_types,
            status, risk_level, reason, reject_reason, approved_at, rejected_at,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id.as_str(),
            project_id.as_str(),
            task_id.as_deref(),
            request_agent_id.as_str(),
            target_service.as_str(),
            operation_types_json.as_str(),
            "pending",
            risk_level.as_str(),
            reason.as_deref(),
            Option::<String>::None,
            Option::<String>::None,
            Option::<String>::None,
            now.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: create approval failed: {error}"))?;
    tx.commit()
        .map_err(|error| format!("database_error: commit create approval failed: {error}"))?;

    let approval = get_approval_by_id(connection, &project_id, &id)?;
    Ok(CreateApprovalResponse { approval })
}

pub fn approve_approval(
    connection: &mut Connection,
    input: ApprovalIdInput,
) -> Result<ApprovalTransitionResponse, String> {
    transition_approval(connection, input.id, "approved", None)
}

pub fn reject_approval(
    connection: &mut Connection,
    input: RejectApprovalInput,
) -> Result<ApprovalTransitionResponse, String> {
    let reject_reason = normalize_optional_text(input.reject_reason, 2000, "reject_reason")?;
    transition_approval(connection, input.id, "rejected", reject_reason)
}

pub fn patch_only_approval(
    connection: &mut Connection,
    input: ApprovalIdInput,
) -> Result<ApprovalTransitionResponse, String> {
    transition_approval(connection, input.id, "patch_only", None)
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn get_current_project_id(connection: &Connection) -> Result<String, String> {
    connection
        .query_row(
            "SELECT id FROM projects ORDER BY created_at LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: read current project failed: {error}"))?
        .ok_or_else(|| "not_found: current project not found".to_string())
}

fn get_approval_by_id(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<ApprovalSummary, String> {
    connection
        .query_row(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals
             WHERE id = ?1 AND project_id = ?2",
            params![approval_id, project_id],
            |row| {
                let operation_types_json: String = row.get(5)?;

                Ok(ApprovalSummary {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    task_id: row.get(2)?,
                    request_agent_id: row.get(3)?,
                    target_service: row.get(4)?,
                    operation_types: parse_string_list(&operation_types_json),
                    status: row.get(6)?,
                    risk_level: row.get(7)?,
                    reason: row.get(8)?,
                    reject_reason: row.get(9)?,
                    approved_at: row.get(10)?,
                    rejected_at: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("database_error: read approval failed: {error}"))?
        .ok_or_else(|| "not_found: approval not found".to_string())
}

fn transition_approval(
    connection: &mut Connection,
    approval_id: String,
    next_status: &str,
    reject_reason: Option<String>,
) -> Result<ApprovalTransitionResponse, String> {
    let project_id = get_current_project_id(connection)?;
    let approval_id = normalize_required_text(approval_id, 1, 200, "id")?;
    let current_approval = get_approval_by_id(connection, &project_id, &approval_id)?;

    if next_status == "approved" && current_approval.target_service == "project_plan" {
        return Err(
            "invalid_input: project_plan approvals must use approve_project_plan".to_string(),
        );
    }

    ensure_approval_transition_allowed(&current_approval.status)?;

    let now = current_timestamp();
    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start update approval transaction failed: {error}")
    })?;

    let changed = match next_status {
        "approved" => tx
            .execute(
                "UPDATE approvals
                 SET status = ?1, approved_at = ?2, updated_at = ?2
                 WHERE id = ?3 AND project_id = ?4",
                params![
                    next_status,
                    now.as_str(),
                    approval_id.as_str(),
                    project_id.as_str()
                ],
            )
            .map_err(|error| format!("database_error: approve approval failed: {error}"))?,
        "rejected" => tx
            .execute(
                "UPDATE approvals
                 SET status = ?1, reject_reason = ?2, rejected_at = ?3, updated_at = ?3
                 WHERE id = ?4 AND project_id = ?5",
                params![
                    next_status,
                    reject_reason.as_deref(),
                    now.as_str(),
                    approval_id.as_str(),
                    project_id.as_str()
                ],
            )
            .map_err(|error| format!("database_error: reject approval failed: {error}"))?,
        "patch_only" => tx
            .execute(
                "UPDATE approvals
                 SET status = ?1, updated_at = ?2
                 WHERE id = ?3 AND project_id = ?4",
                params![
                    next_status,
                    now.as_str(),
                    approval_id.as_str(),
                    project_id.as_str()
                ],
            )
            .map_err(|error| format!("database_error: patch-only approval failed: {error}"))?,
        _ => return Err("invalid_input: approval status is not allowed".to_string()),
    };

    if changed != 1 {
        return Err("not_found: approval not found".to_string());
    }

    tx.commit()
        .map_err(|error| format!("database_error: commit update approval failed: {error}"))?;

    let approval = get_approval_by_id(connection, &project_id, &approval_id)?;
    Ok(ApprovalTransitionResponse { approval })
}

fn normalize_required_text(
    value: String,
    min_len: usize,
    max_len: usize,
    field: &str,
) -> Result<String, String> {
    let normalized = value.trim().to_string();
    let length = normalized.chars().count();

    if length < min_len || length > max_len {
        return Err(format!(
            "invalid_input: {field} length must be between {min_len} and {max_len}"
        ));
    }

    Ok(normalized)
}

fn normalize_optional_text(
    value: Option<String>,
    max_len: usize,
    field: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }

    if normalized.chars().count() > max_len {
        return Err(format!(
            "invalid_input: {field} length must be at most {max_len}"
        ));
    }

    Ok(Some(normalized))
}

fn normalize_enum(value: String, allowed: &[&str], field: &str) -> Result<String, String> {
    let normalized = value.trim().to_string();
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!("invalid_input: {field} is not allowed"))
    }
}

fn normalize_optional_id(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_operation_types(values: Vec<String>) -> Result<Vec<String>, String> {
    let allowed = [
        "task_create",
        "task_status_update",
        "approval_create",
        "approval_approve",
        "approval_reject",
        "approval_patch_only",
        "runner_request_readonly",
        "agent_config_review",
        "model_gateway_review",
    ];
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    if values.is_empty() {
        return Err("invalid_input: operation_types cannot be empty".to_string());
    }

    for value in values {
        let value = normalize_enum(value, &allowed, "operation_types")?;
        if !seen.insert(value.clone()) {
            return Err("invalid_input: operation_types cannot contain duplicates".to_string());
        }
        normalized.push(value);
    }

    Ok(normalized)
}

fn ensure_agent_belongs_to_project(
    connection: &Connection,
    project_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    // 先查旧 agents 表（向后兼容）
    let old_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agents WHERE id = ?1 AND project_id = ?2",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check agent failed: {error}"))?;

    if old_count == 1 {
        return Ok(());
    }

    // 再查 project_agents 表（新 P0 数据源；表不存在时视为 0）
    let pa_count = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE id = ?1 AND project_id = ?2 AND removed_at IS NULL",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if pa_count == 1 {
        return Ok(());
    }

    Err("not_found: request agent not found".to_string())
}

fn ensure_task_belongs_to_project(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check task failed: {error}"))?;

    if count == 1 {
        Ok(())
    } else {
        Err("not_found: task not found".to_string())
    }
}

fn ensure_approval_transition_allowed(current: &str) -> Result<(), String> {
    if current == "pending" {
        Ok(())
    } else {
        Err(format!(
            "invalid_transition: approval status cannot change from {current}"
        ))
    }
}

fn generate_approval_id() -> String {
    format!("approval_{}", timestamp_nanos())
}

fn current_timestamp() -> String {
    timestamp_nanos().to_string()
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::{
        approve_approval, create_approval, parse_string_list, patch_only_approval, reject_approval,
        ApprovalIdInput, CreateApprovalInput, RejectApprovalInput,
    };
    use rusqlite::{params, Connection};

    const INITIAL_MIGRATION_SQL: &str =
        include_str!("../../../../../data/migrations/001_initial_sqlite.sql");

    #[test]
    fn parse_string_list_reads_operation_types() {
        assert_eq!(
            parse_string_list(r#"["file_write","git_checkpoint"]"#),
            vec!["file_write".to_string(), "git_checkpoint".to_string()]
        );
    }

    #[test]
    fn create_approval_inserts_pending_record_without_side_effects() {
        let mut connection = setup_connection();
        let response = create_approval(
            &mut connection,
            CreateApprovalInput {
                task_id: Some("task_existing".to_string()),
                request_agent_id: "agent_architect".to_string(),
                target_service: "runner".to_string(),
                operation_types: vec!["runner_request_readonly".to_string()],
                risk_level: "high".to_string(),
                reason: Some("  Needs review  ".to_string()),
            },
        )
        .expect("approval should be created");

        assert_eq!(response.approval.status, "pending");
        assert_eq!(response.approval.task_id.as_deref(), Some("task_existing"));
        assert_eq!(response.approval.request_agent_id, "agent_architect");
        assert_eq!(response.approval.target_service, "runner");
        assert_eq!(
            response.approval.operation_types,
            vec!["runner_request_readonly".to_string()]
        );
        assert_eq!(response.approval.risk_level, "high");
        assert_eq!(response.approval.reason.as_deref(), Some("Needs review"));
        assert!(response.approval.approved_at.is_none());
        assert!(response.approval.rejected_at.is_none());
    }

    #[test]
    fn create_approval_rejects_invalid_target_service() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.target_service = "filesystem".to_string();

        let error = create_approval(&mut connection, input)
            .expect_err("invalid target service should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_empty_operation_types() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.operation_types = Vec::new();

        let error =
            create_approval(&mut connection, input).expect_err("empty operations should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_duplicate_operation_types() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.operation_types = vec!["approval_create".to_string(), "approval_create".to_string()];

        let error =
            create_approval(&mut connection, input).expect_err("duplicate operations should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_invalid_operation_type() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.operation_types = vec!["file_write".to_string()];

        let error =
            create_approval(&mut connection, input).expect_err("invalid operation should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_invalid_risk_level() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.risk_level = "critical".to_string();

        let error = create_approval(&mut connection, input).expect_err("invalid risk should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_too_long_reason() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.reason = Some("a".repeat(2001));

        let error = create_approval(&mut connection, input).expect_err("long reason should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_approval_rejects_unknown_agent() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.request_agent_id = "missing_agent".to_string();

        let error = create_approval(&mut connection, input).expect_err("unknown agent should fail");

        assert!(error.contains("not_found"));
    }

    #[test]
    fn create_approval_rejects_unknown_task() {
        let mut connection = setup_connection();
        let mut input = valid_input();
        input.task_id = Some("missing_task".to_string());

        let error = create_approval(&mut connection, input).expect_err("unknown task should fail");

        assert!(error.contains("not_found"));
    }

    #[test]
    fn approve_approval_marks_pending_approval_as_approved_without_runner_side_effects() {
        let mut connection = setup_connection();
        insert_approval(&connection, "approval_pending", "pending", "runner");

        let response = approve_approval(
            &mut connection,
            ApprovalIdInput {
                id: "approval_pending".to_string(),
            },
        )
        .expect("pending approval should be approved");

        assert_eq!(response.approval.status, "approved");
        assert!(response.approval.approved_at.is_some());
        assert!(response.approval.rejected_at.is_none());
        assert_eq!(response.approval.target_service, "runner");
    }

    #[test]
    fn approve_approval_rejects_terminal_approvals() {
        let mut connection = setup_connection();

        for status in ["approved", "rejected", "patch_only"] {
            let approval_id = format!("approval_{status}");
            insert_approval(&connection, &approval_id, status, "approval");

            let error = approve_approval(&mut connection, ApprovalIdInput { id: approval_id })
                .expect_err("terminal approval should not change");

            assert!(error.contains("invalid_transition"));
        }
    }

    #[test]
    fn approve_approval_rejects_unknown_approval() {
        let mut connection = setup_connection();

        let error = approve_approval(
            &mut connection,
            ApprovalIdInput {
                id: "missing_approval".to_string(),
            },
        )
        .expect_err("unknown approval should fail");

        assert!(error.contains("not_found"));
    }

    #[test]
    fn approve_approval_rejects_approval_from_another_project() {
        let mut connection = setup_connection();

        insert_other_project_approval(&connection, "approval_other_project");

        let error = approve_approval(
            &mut connection,
            ApprovalIdInput {
                id: "approval_other_project".to_string(),
            },
        )
        .expect_err("approval from another project should not be found");

        assert!(error.contains("not_found"));
        let status: String = connection
            .query_row(
                "SELECT status FROM approvals WHERE id = ?1 AND project_id = ?2",
                params!["approval_other_project", "other_project"],
                |row| row.get(0),
            )
            .expect("other project approval should still exist");
        assert_eq!(status, "pending");
    }

    #[test]
    fn reject_approval_marks_pending_approval_as_rejected() {
        let mut connection = setup_connection();
        insert_approval(&connection, "approval_pending", "pending", "approval");

        let response = reject_approval(
            &mut connection,
            RejectApprovalInput {
                id: "approval_pending".to_string(),
                reject_reason: Some("  Not safe yet  ".to_string()),
            },
        )
        .expect("pending approval should be rejected");

        assert_eq!(response.approval.status, "rejected");
        assert_eq!(
            response.approval.reject_reason.as_deref(),
            Some("Not safe yet")
        );
        assert!(response.approval.rejected_at.is_some());
        assert!(response.approval.approved_at.is_none());
    }

    #[test]
    fn reject_approval_rejects_too_long_reason() {
        let mut connection = setup_connection();
        insert_approval(&connection, "approval_pending", "pending", "approval");

        let error = reject_approval(
            &mut connection,
            RejectApprovalInput {
                id: "approval_pending".to_string(),
                reject_reason: Some("a".repeat(2001)),
            },
        )
        .expect_err("long reject reason should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn reject_approval_rejects_terminal_approvals() {
        let mut connection = setup_connection();

        for status in ["approved", "rejected", "patch_only"] {
            let approval_id = format!("approval_reject_{status}");
            insert_approval(&connection, &approval_id, status, "approval");

            let error = reject_approval(
                &mut connection,
                RejectApprovalInput {
                    id: approval_id,
                    reject_reason: None,
                },
            )
            .expect_err("terminal approval should not change");

            assert!(error.contains("invalid_transition"));
        }
    }

    #[test]
    fn patch_only_approval_marks_pending_approval_as_patch_only_without_file_side_effects() {
        let mut connection = setup_connection();
        insert_approval(&connection, "approval_pending", "pending", "approval");

        let response = patch_only_approval(
            &mut connection,
            ApprovalIdInput {
                id: "approval_pending".to_string(),
            },
        )
        .expect("pending approval should become patch_only");

        assert_eq!(response.approval.status, "patch_only");
        assert!(response.approval.approved_at.is_none());
        assert!(response.approval.rejected_at.is_none());
    }

    #[test]
    fn patch_only_approval_rejects_terminal_approvals() {
        let mut connection = setup_connection();

        for status in ["approved", "rejected", "patch_only"] {
            let approval_id = format!("approval_patch_{status}");
            insert_approval(&connection, &approval_id, status, "approval");

            let error = patch_only_approval(&mut connection, ApprovalIdInput { id: approval_id })
                .expect_err("terminal approval should not change");

            assert!(error.contains("invalid_transition"));
        }
    }

    #[test]
    fn patch_only_approval_rejects_unknown_approval() {
        let mut connection = setup_connection();

        let error = patch_only_approval(
            &mut connection,
            ApprovalIdInput {
                id: "missing_approval".to_string(),
            },
        )
        .expect_err("unknown approval should fail");

        assert!(error.contains("not_found"));
    }

    fn valid_input() -> CreateApprovalInput {
        CreateApprovalInput {
            task_id: Some("task_existing".to_string()),
            request_agent_id: "agent_architect".to_string(),
            target_service: "approval".to_string(),
            operation_types: vec!["approval_create".to_string()],
            risk_level: "medium".to_string(),
            reason: Some("review".to_string()),
        }
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        connection
            .execute_batch(INITIAL_MIGRATION_SQL)
            .expect("schema should initialize");
        connection
            .execute(
                "INSERT INTO projects (
                    id, name, status, phase, description, workspace_path, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "project_agent_swarm",
                    "agent-swarm",
                    "running",
                    "test",
                    Option::<String>::None,
                    Option::<String>::None,
                    "1",
                    "1"
                ],
            )
            .expect("project should insert");
        connection
            .execute(
                "INSERT INTO agents (
                    id, project_id, name, role, status, model, permissions, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "agent_architect",
                    "project_agent_swarm",
                    "Architect Agent",
                    "architect",
                    "running",
                    Option::<String>::None,
                    "[]",
                    "1",
                    "1"
                ],
            )
            .expect("agent should insert");
        connection
            .execute(
                "INSERT INTO tasks (
                    id, project_id, title, description, status, priority, assigned_agent_id,
                    depends_on, risk_level, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "task_existing",
                    "project_agent_swarm",
                    "Existing task",
                    Option::<String>::None,
                    "queued",
                    "medium",
                    Option::<String>::None,
                    "[]",
                    Option::<String>::None,
                    "1",
                    "1"
                ],
            )
            .expect("task should insert");

        connection
    }

    fn insert_approval(
        connection: &Connection,
        approval_id: &str,
        status: &str,
        target_service: &str,
    ) {
        connection
            .execute(
                "INSERT INTO approvals (
                    id, project_id, task_id, request_agent_id, target_service, operation_types,
                    status, risk_level, reason, reject_reason, approved_at, rejected_at,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    approval_id,
                    "project_agent_swarm",
                    Option::<String>::None,
                    "agent_architect",
                    target_service,
                    r#"["approval_approve"]"#,
                    status,
                    "medium",
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    "1",
                    "1"
                ],
            )
            .expect("approval should insert");
    }

    fn insert_other_project_approval(connection: &Connection, approval_id: &str) {
        connection
            .execute(
                "INSERT INTO projects (
                    id, name, status, phase, description, workspace_path, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "other_project",
                    "other",
                    "running",
                    "test",
                    Option::<String>::None,
                    Option::<String>::None,
                    "2",
                    "2"
                ],
            )
            .expect("other project should insert");
        connection
            .execute(
                "INSERT INTO agents (
                    id, project_id, name, role, status, model, permissions,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "agent_other",
                    "other_project",
                    "Other agent",
                    "reviewer",
                    "idle",
                    Option::<String>::None,
                    Option::<String>::None,
                    "2",
                    "2"
                ],
            )
            .expect("other project agent should insert");
        connection
            .execute(
                "INSERT INTO approvals (
                    id, project_id, task_id, request_agent_id, target_service, operation_types,
                    status, risk_level, reason, reject_reason, approved_at, rejected_at,
                    created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    approval_id,
                    "other_project",
                    Option::<String>::None,
                    "agent_other",
                    "approval",
                    r#"["approval_approve"]"#,
                    "pending",
                    "medium",
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    "2",
                    "2"
                ],
            )
            .expect("other project approval should insert");
    }
}
