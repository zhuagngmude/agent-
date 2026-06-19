// 阶段 30：Runner 执行前审查闸门
// 只创建执行前审查记录和审批，不执行 Runner、不写文件、不改 Git。

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::agent_config::{check_agent_boundary, CheckAgentBoundaryInput};
use super::approvals::ApprovalSummary;
use super::project_plan::{ProjectPlanSideEffects, RunnerRequestSummary};
use super::projects::get_current_project;

const CONFIRM_TEXT: &str = "我确认只创建执行前审查，不执行Runner";
const SAFETY_SUMMARY: &str =
    "Runner 执行已由系统边界关闭。本审查记录只用于审查和讨论，不代表执行许可。";
const BLOCKED_REASON: &str = "runner_execution_disabled_by_stage_boundary";
const FORBIDDEN_OPERATIONS: &[&str] = &[
    "command_execute",
    "file_write",
    "file_delete",
    "git_commit",
    "git_push",
    "network_request",
    "model_call",
    "runner_execute",
];

// ---------------------------------------------------------------------------
// 类型
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerPreflightReviewInput {
    pub runner_request_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerPreflightReviewSummary {
    pub id: String,
    pub project_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub approval_id: String,
    pub status: String,
    pub risk_level: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub blocked_reasons: Vec<String>,
    pub safety_summary: String,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerPreflightReviewResponse {
    pub review: RunnerPreflightReviewSummary,
    pub approval: ApprovalSummary,
    pub side_effects: ProjectPlanSideEffects,
}

// ---------------------------------------------------------------------------
// service 函数
// ---------------------------------------------------------------------------

pub fn create_runner_preflight_review(
    connection: &mut Connection,
    input: CreateRunnerPreflightReviewInput,
) -> Result<CreateRunnerPreflightReviewResponse, String> {
    let project_id = get_current_project(connection)?.id;
    // 二次确认
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    if input.confirm_text.trim() != CONFIRM_TEXT {
        return Err(format!(
            "invalid_input: confirm_text must be '{CONFIRM_TEXT}'"
        ));
    }
    let rid = normalize(&input.runner_request_id, "runner_request_id", 200)?;
    let requested_by = normalize_optional(input.requested_by, "requested_by", 120)?
        .unwrap_or_else(|| "local_user".to_string());
    crate::services::model_gateway::redaction::check_forbidden_value_patterns(&requested_by)?;

    // 读取 runner_request
    let rr = get_runner_request(connection, &project_id, &rid)?
        .ok_or_else(|| "not_found: runner request not found".to_string())?;
    if rr.status != "queued" {
        return Err(format!(
            "invalid_input: runner request status is {}, must be queued",
            rr.status
        ));
    }
    if !rr
        .operation_types
        .contains(&"runner_request_write_files".to_string())
    {
        return Err("invalid_input: runner request is not writable".into());
    }
    // 禁止真实执行操作
    for op in &rr.operation_types {
        if FORBIDDEN_OPERATIONS.contains(&op.as_str()) {
            return Err(format!("invalid_input: forbidden operation type: {op}"));
        }
    }
    // affected_files 校验
    for af in &rr.affected_files {
        validate_affected_file(af)?;
    }

    // 幂等：同一 runner_request 已有 preflight
    if let Some(existing) = find_review_by_runner_request(connection, &project_id, &rid)? {
        let approval = get_approval_by_id(connection, &project_id, &existing.approval_id)?
            .ok_or_else(|| "not_found: approval not found".to_string())?;
        return Ok(CreateRunnerPreflightReviewResponse {
            review: existing,
            approval,
            side_effects: all_false(),
        });
    }

    // 读取关联 task
    let task = get_task(connection, &project_id, &rr.task_id)?
        .ok_or_else(|| "not_found: task not found".to_string())?;
    if let Some(agent_id) = task.assigned_agent_id.as_deref() {
        if project_agent_exists(connection, &project_id, agent_id)? {
            let target_path = rr
                .affected_files
                .iter()
                .find(|path| !path.starts_with("virtual/"))
                .cloned();
            let boundary = check_agent_boundary(
                connection,
                CheckAgentBoundaryInput {
                    agent_id: agent_id.to_string(),
                    task_id: Some(rr.task_id.clone()),
                    task_type: infer_task_type(&rr.operation_types),
                    module_scope: infer_module_scope(agent_id),
                    target_path,
                    forbidden_actions: rr.operation_types.clone(),
                    requested_action: "runner_preflight".to_string(),
                },
            )?;
            if boundary.decision == "denied" {
                return Err(format!(
                    "permission_denied: agent boundary check denied preflight: {}",
                    boundary.reason
                ));
            }
        }
    }

    // 计算风险
    let risk_level = if task.risk_level.as_deref() == Some("high")
        || rr
            .operation_types
            .contains(&"security_review_plan".to_string())
    {
        "high"
    } else {
        task.risk_level.as_deref().unwrap_or("medium")
    };

    // 创建 approval
    let approval_id = format!("approval_preflight_{rid}");
    let operation_types_json = serde_json::to_string(&["runner_preflight_review"])
        .map_err(|e| format!("database_error: serialize preflight operation types failed: {e}"))?;
    let reason = format!("申请对只读 Runner request 创建执行前审查：{rid}");
    let now = now_str();

    let tx = connection
        .transaction()
        .map_err(|e| format!("database_error: start preflight transaction failed: {e}"))?;

    tx.execute(
        "INSERT INTO approvals (
            id, project_id, task_id, request_agent_id, target_service, operation_types,
            status, risk_level, reason, reject_reason, approved_at, rejected_at,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, 'agent_architect', 'runner_preflight', ?4,
            'pending', ?5, ?6, NULL, NULL, NULL, ?7, ?7)",
        params![
            approval_id.as_str(),
            project_id.as_str(),
            rr.task_id.as_str(),
            operation_types_json.as_str(),
            risk_level,
            reason.as_str(),
            now.as_str()
        ],
    )
    .map_err(|e| format!("database_error: create preflight approval failed: {e}"))?;

    // 创建 review
    let review_id = format!("preflight_{rid}");
    let blocked_reasons_json = serde_json::to_string(&[BLOCKED_REASON])
        .map_err(|e| format!("database_error: serialize blocked reasons failed: {e}"))?;
    let op_json = serde_json::to_string(&rr.operation_types)
        .map_err(|e| format!("database_error: serialize ops failed: {e}"))?;
    let af_json = serde_json::to_string(&rr.affected_files)
        .map_err(|e| format!("database_error: serialize files failed: {e}"))?;

    tx.execute(
        "INSERT INTO runner_preflight_reviews (
            id, project_id, runner_request_id, task_id, approval_id, status,
            risk_level, operation_types, affected_files,
            requires_git_checkpoint, requires_second_confirm,
            blocked_reasons, safety_summary, requested_by,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'blocked', ?6, ?7, ?8, 1, 1, ?9, ?10, ?11, ?12, ?12)",
        params![
            review_id.as_str(),
            project_id.as_str(),
            rid.as_str(),
            rr.task_id.as_str(),
            approval_id.as_str(),
            risk_level,
            op_json.as_str(),
            af_json.as_str(),
            blocked_reasons_json.as_str(),
            SAFETY_SUMMARY,
            requested_by.as_str(),
            now.as_str()
        ],
    )
    .map_err(|e| format!("database_error: create preflight review failed: {e}"))?;

    tx.commit()
        .map_err(|e| format!("database_error: commit preflight failed: {e}"))?;

    let review = find_review_by_runner_request(connection, &project_id, &rid)?
        .ok_or_else(|| "not_found: review not found after create".to_string())?;
    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;

    Ok(CreateRunnerPreflightReviewResponse {
        review,
        approval,
        side_effects: all_false(),
    })
}

pub fn auto_create_runner_preflight_review(
    connection: &mut Connection,
    runner_request_id: String,
    requested_by: Option<String>,
) -> Result<CreateRunnerPreflightReviewResponse, String> {
    create_runner_preflight_review(
        connection,
        CreateRunnerPreflightReviewInput {
            runner_request_id,
            second_confirm: true,
            confirm_text: CONFIRM_TEXT.to_string(),
            requested_by,
        },
    )
}

pub fn list_runner_preflight_reviews(
    connection: &Connection,
) -> Result<Vec<RunnerPreflightReviewSummary>, String> {
    let project_id = get_current_project(connection)?.id;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, runner_request_id, task_id, approval_id, status,
                risk_level, operation_types, affected_files,
                requires_git_checkpoint, requires_second_confirm,
                blocked_reasons, safety_summary, requested_by, created_at, updated_at
             FROM runner_preflight_reviews
             WHERE project_id = ?1
             ORDER BY created_at DESC, id",
        )
        .map_err(|e| format!("database_error: list preflight reviews failed: {e}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str()], map_review_row)
        .map_err(|e| format!("database_error: list preflight reviews failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list preflight reviews failed: {e}"))
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn find_review_by_runner_request(
    connection: &Connection,
    project_id: &str,
    runner_request_id: &str,
) -> Result<Option<RunnerPreflightReviewSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, runner_request_id, task_id, approval_id, status,
                risk_level, operation_types, affected_files,
                requires_git_checkpoint, requires_second_confirm,
                blocked_reasons, safety_summary, requested_by, created_at, updated_at
             FROM runner_preflight_reviews
             WHERE project_id = ?1 AND runner_request_id = ?2",
            params![project_id, runner_request_id],
            map_review_row,
        )
        .optional()
        .map_err(|e| format!("database_error: find review failed: {e}"))
}

fn get_runner_request(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<Option<RunnerRequestSummary>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, approval_id, task_id, status, operation_types, affected_files,
                checkpoint, safety_note, created_at, updated_at
             FROM runner_requests WHERE id = ?1 AND project_id = ?2",
        )
        .map_err(|e| format!("database_error: read runner request failed: {e}"))?;
    let mut rows = stmt
        .query_map(params![id, project_id], |row| {
            let op_json: String = row.get(5)?;
            let af_json: String = row.get(6)?;
            Ok(RunnerRequestSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                approval_id: row.get(2)?,
                task_id: row.get(3)?,
                status: row.get(4)?,
                operation_types: parse_string_list(&op_json),
                affected_files: parse_string_list(&af_json),
                checkpoint: row.get(7)?,
                safety_note: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| format!("database_error: read runner request failed: {e}"))?;
    Ok(rows
        .next()
        .transpose()
        .map_err(|e| format!("database_error: read runner request failed: {e}"))?)
}

struct TaskInfo {
    risk_level: Option<String>,
    assigned_agent_id: Option<String>,
}

fn get_task(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<Option<TaskInfo>, String> {
    connection
        .query_row(
            "SELECT risk_level, assigned_agent_id FROM tasks WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |row| {
                Ok(TaskInfo {
                    risk_level: row.get(0)?,
                    assigned_agent_id: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("database_error: read task failed: {e}"))
}

fn project_agent_exists(
    connection: &Connection,
    project_id: &str,
    agent_id: &str,
) -> Result<bool, String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents
             WHERE id = ?1 AND project_id = ?2 AND removed_at IS NULL",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("database_error: lookup project agent failed: {e}"))?;
    Ok(count == 1)
}

fn get_approval_by_id(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Option<ApprovalSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals WHERE id = ?1 AND project_id = ?2",
            params![approval_id, project_id],
            |row| {
                let op_json: String = row.get(5)?;
                Ok(ApprovalSummary {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    task_id: row.get(2)?,
                    request_agent_id: row.get(3)?,
                    target_service: row.get(4)?,
                    operation_types: parse_string_list(&op_json),
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
        .map_err(|e| format!("database_error: read approval failed: {e}"))
}

fn validate_affected_file(path: &str) -> Result<(), String> {
    if !path.starts_with("virtual/") {
        return Err(format!(
            "invalid_input: affected_file must start with virtual/, got: {path}"
        ));
    }
    if path.contains("..") || path.contains('\\') || path.contains(':') || path.contains('~') {
        return Err(format!(
            "invalid_input: affected_file contains forbidden characters: {path}"
        ));
    }
    Ok(())
}

fn infer_task_type(operation_types: &[String]) -> String {
    if operation_types.iter().any(|op| op.contains("write")) {
        "runner_request_write_files".to_string()
    } else if let Some(first) = operation_types.first() {
        first.clone()
    } else {
        "runner_preflight".to_string()
    }
}

fn infer_module_scope(agent_id: &str) -> String {
    if let Some(scope) = agent_id.strip_prefix("project_agent_") {
        return scope.to_string();
    }
    "runner".to_string()
}

fn map_review_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerPreflightReviewSummary> {
    let op_json: String = row.get(7)?;
    let af_json: String = row.get(8)?;
    let br_json: String = row.get(11)?;
    Ok(RunnerPreflightReviewSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        runner_request_id: row.get(2)?,
        task_id: row.get(3)?,
        approval_id: row.get(4)?,
        status: row.get(5)?,
        risk_level: row.get(6)?,
        operation_types: parse_string_list(&op_json),
        affected_files: parse_string_list(&af_json),
        requires_git_checkpoint: row.get::<_, i64>(9)? != 0,
        requires_second_confirm: row.get::<_, i64>(10)? != 0,
        blocked_reasons: parse_string_list(&br_json),
        safety_summary: row.get(12)?,
        requested_by: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn normalize(value: &str, field: &str, max: usize) -> Result<String, String> {
    let v = value.trim().to_string();
    if v.is_empty() || v.chars().count() > max {
        return Err(format!("invalid_input: {field} length invalid"));
    }
    Ok(v)
}

fn normalize_optional(
    value: Option<String>,
    field: &str,
    max: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.chars().count() > max {
        return Err(format!("invalid_input: {field} length invalid"));
    }
    Ok(Some(normalized))
}

fn all_false() -> ProjectPlanSideEffects {
    ProjectPlanSideEffects {
        writes_project_files: false,
        modifies_git: false,
        executes_runner: false,
        calls_real_model: false,
        reads_raw_secrets: false,
        makes_network_requests: false,
        triggers_agents: false,
        creates_tasks: false,
        creates_runner_requests: false,
    }
}

fn now_str() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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
    use crate::services::project_plan::{
        approve_project_plan, create_project_plan_draft, ApproveProjectPlanInput,
        CreateProjectPlanDraftInput,
    };
    use std::fs;

    fn valid_draft_input() -> CreateProjectPlanDraftInput {
        CreateProjectPlanDraftInput {
            idea: "local customer lead tracker".to_string(),
            constraints: Some("SQLite only; no Runner execution".to_string()),
            requested_by: Some("local_user".to_string()),
        }
    }

    fn test_db() -> (crate::db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-preflight-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let state = crate::db::initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    fn count_rows(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable")
    }

    /// 创建 project plan 审批并返回第一条 runner_request 的 id
    fn create_sample_runner_request(connection: &mut Connection) -> String {
        let draft = create_project_plan_draft(connection, valid_draft_input())
            .expect("draft should be created");
        approve_project_plan(
            connection,
            ApproveProjectPlanInput {
                approval_id: draft.approval.id,
                second_confirm: true,
                confirm_text: "确认生成任务".to_string(),
            },
        )
        .expect("approve should succeed");
        // 返回第一条 runner request id
        let id: String = connection
            .query_row(
                "SELECT id FROM runner_requests ORDER BY id LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("should have a runner request");
        id
    }

    fn bind_runner_request_task_to_project_agent(connection: &Connection, runner_request_id: &str) {
        connection
            .execute(
                "UPDATE tasks
                 SET assigned_agent_id = 'project_agent_frontend'
                 WHERE id = (
                   SELECT task_id FROM runner_requests WHERE id = ?1
                 )",
                params![runner_request_id],
            )
            .expect("task should bind to project agent");
        let assigned_agent_id: String = connection
            .query_row(
                "SELECT assigned_agent_id FROM tasks
                 WHERE id = (SELECT task_id FROM runner_requests WHERE id = ?1)",
                params![runner_request_id],
                |row| row.get(0),
            )
            .expect("assigned agent should load");
        assert_eq!(assigned_agent_id, "project_agent_frontend");
    }

    fn valid_input(runner_request_id: &str) -> CreateRunnerPreflightReviewInput {
        CreateRunnerPreflightReviewInput {
            runner_request_id: runner_request_id.to_string(),
            second_confirm: true,
            confirm_text: CONFIRM_TEXT.to_string(),
            requested_by: None,
        }
    }

    #[test]
    fn create_preflight_input_rejects_unknown_fields() {
        let value = serde_json::json!({
            "runner_request_id": "runner_request_project_plan_frontend",
            "second_confirm": true,
            "confirm_text": CONFIRM_TEXT,
            "requested_by": "local_user",
            "api_key": "sk-abcdefghijklmnopqrstuvwxyz123456"
        });

        let err = serde_json::from_value::<CreateRunnerPreflightReviewInput>(value)
            .expect_err("unknown fields should be denied");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn create_preflight_requires_second_confirmation() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            let err = create_runner_preflight_review(
                &mut connection,
                CreateRunnerPreflightReviewInput {
                    second_confirm: false,
                    ..valid_input(&rid)
                },
            )
            .expect_err("missing second_confirm should fail");
            assert!(err.contains("second_confirm"));

            let err = create_runner_preflight_review(
                &mut connection,
                CreateRunnerPreflightReviewInput {
                    confirm_text: "wrong".to_string(),
                    ..valid_input(&rid)
                },
            )
            .expect_err("wrong confirm_text should fail");
            assert!(err.contains("confirm_text"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_rejects_unknown_runner_request() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let err =
                create_runner_preflight_review(&mut connection, valid_input("nonexistent_id"))
                    .expect_err("unknown runner request should fail");
            assert!(err.contains("not_found"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_rejects_invalid_requested_by() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);

            let err = create_runner_preflight_review(
                &mut connection,
                CreateRunnerPreflightReviewInput {
                    requested_by: Some("x".repeat(121)),
                    ..valid_input(&rid)
                },
            )
            .expect_err("too long requested_by should fail");
            assert!(err.contains("requested_by"));

            let err = create_runner_preflight_review(
                &mut connection,
                CreateRunnerPreflightReviewInput {
                    requested_by: Some("sk-abcdefghijklmnopqrstuvwxyz123456".to_string()),
                    ..valid_input(&rid)
                },
            )
            .expect_err("sensitive requested_by should fail");
            assert!(err.contains("疑似") || err.contains("API key") || err.contains("密钥"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_creates_review_and_pending_approval() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            let before_tasks = count_rows(&connection, "tasks");
            let before_requests = count_rows(&connection, "runner_requests");
            let before_events = count_rows(&connection, "runtime_events");
            let before_mc = count_rows(&connection, "model_calls");
            bind_runner_request_task_to_project_agent(&connection, &rid);

            let response = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("create preflight should succeed");

            assert_eq!(response.review.status, "blocked");
            assert!(response
                .review
                .blocked_reasons
                .contains(&BLOCKED_REASON.to_string()));
            assert_eq!(response.approval.target_service, "runner_preflight");
            assert_eq!(response.approval.status, "pending");
            assert_eq!(count_rows(&connection, "runner_preflight_reviews"), 1);
            // 4 seed + 1 new = 5
            assert_eq!(count_rows(&connection, "approvals"), 5);
            // 不动其他表
            assert_eq!(count_rows(&connection, "tasks"), before_tasks);
            assert_eq!(count_rows(&connection, "runner_requests"), before_requests);
            assert_eq!(count_rows(&connection, "runtime_events"), before_events);
            assert_eq!(count_rows(&connection, "model_calls"), before_mc);
            assert_eq!(count_rows(&connection, "agent_boundary_checks"), 1);
            // side effects
            let se = &response.side_effects;
            assert!(!se.creates_tasks);
            assert!(!se.creates_runner_requests);
            assert!(!se.executes_runner);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_is_idempotent_for_same_runner_request() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            let first = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("first should succeed");
            let review_count = count_rows(&connection, "runner_preflight_reviews");
            let approval_count = count_rows(&connection, "approvals");

            let second = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("second should be idempotent");
            assert_eq!(second.review.id, first.review.id);
            assert_eq!(second.approval.id, first.approval.id);
            assert_eq!(
                count_rows(&connection, "runner_preflight_reviews"),
                review_count
            );
            assert_eq!(count_rows(&connection, "approvals"), approval_count);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_blocks_real_execution_by_stage_boundary() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            let response = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("create should succeed");
            assert_eq!(response.review.status, "blocked");
            assert!(response
                .review
                .blocked_reasons
                .contains(&"runner_execution_disabled_by_stage_boundary".to_string()));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_rejects_polluted_affected_files() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            connection
                .execute(
                    "UPDATE runner_requests SET affected_files = '[\"../secret\"]' WHERE id = ?1",
                    params![rid.as_str()],
                )
                .expect("should update");
            let err = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect_err("polluted should fail");
            assert!(err.contains("forbidden") || err.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_rejects_forbidden_operation_type() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            connection
                .execute(
                    "UPDATE runner_requests SET operation_types = '[\"file_write\"]' WHERE id = ?1",
                    params![rid.as_str()],
                )
                .expect("should update");
            let err = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect_err("forbidden op should fail");
            assert!(
                err.contains("forbidden") || err.contains("invalid_input"),
                "unexpected error: {err}"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_preflight_rejects_denied_agent_boundary() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            bind_runner_request_task_to_project_agent(&connection, &rid);
            connection
                .execute(
                    "UPDATE agent_templates
                     SET allowed_task_types = '[\"runner_request_write_files\"]',
                         forbidden_actions = '[\"runner_request_write_files\", \"docs_write\", \"frontend_impl\"]'
                     WHERE id = 'agent_template_frontend'",
                    [],
                )
                .expect("forbid runner write operation");
            connection
                .execute(
                    "UPDATE project_agents
                     SET status = 'active'
                     WHERE id = 'project_agent_frontend'",
                    [],
                )
                .expect("activate project agent");

            let err = create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect_err("denied boundary should block preflight");
            assert!(err.contains("permission_denied"), "unexpected error: {err}");
            assert_eq!(count_rows(&connection, "agent_boundary_checks"), 1);
            assert_eq!(count_rows(&connection, "runner_preflight_reviews"), 0);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_preflight_reviews_filters_current_project() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("create should succeed");
            let reviews = list_runner_preflight_reviews(&connection).expect("list should succeed");
            assert_eq!(reviews.len(), 1);

            // 插入另一个项目的 review（先插入必要的 FK 引用）
            connection
                .execute(
                    "INSERT INTO projects (id, name, status, created_at, updated_at)
                     VALUES ('other_proj', 'O', 'ok', '1', '1')",
                    [],
                )
                .expect("insert project");
            connection
                .execute(
                    "INSERT INTO approvals (id, project_id, task_id, request_agent_id, target_service, operation_types, status, risk_level, reason, created_at, updated_at)
                     VALUES ('a_other', 'other_proj', NULL, 'agent_architect', 'project_plan', '[]', 'pending', 'medium', 'test', '1', '1')",
                    [],
                )
                .expect("insert approval");
            connection
                .execute(
                    "INSERT INTO tasks (id, project_id, title, status, priority, created_at, updated_at)
                     VALUES ('t_other', 'other_proj', 'Test', 'queued', 'medium', '1', '1')",
                    [],
                )
                .expect("insert task");
            connection
                .execute(
                    "INSERT INTO runner_requests (id, project_id, approval_id, task_id, status, operation_types, affected_files, safety_note, created_at, updated_at)
                     VALUES ('rr_other', 'other_proj', 'a_other', 't_other', 'queued', '[]', '[]', 's', '1', '1')",
                    [],
                )
                .expect("insert rr");
            connection
                .execute(
                    "INSERT INTO runner_preflight_reviews (id, project_id, runner_request_id, task_id, approval_id, status, risk_level, operation_types, affected_files, blocked_reasons, safety_summary, requested_by, created_at, updated_at)
                     VALUES ('pf_other', 'other_proj', 'rr_other', 't_other', 'a_other', 'blocked', 'medium', '[]', '[]', '[]', 's', 'u', '1', '1')",
                    [],
                )
                .expect("insert other review");
            let reviews2 = list_runner_preflight_reviews(&connection).expect("list should succeed");
            assert_eq!(reviews2.len(), 1); // 只返回当前项目的
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn preflight_does_not_change_task_or_runner_request_status() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let rid = create_sample_runner_request(&mut connection);
            let task_id_before: String = connection
                .query_row(
                    "SELECT task_id FROM runner_requests WHERE id = ?1",
                    params![rid.as_str()],
                    |row| row.get(0),
                )
                .expect("should get task_id");
            let task_status_before: String = connection
                .query_row(
                    "SELECT status FROM tasks WHERE id = ?1",
                    params![task_id_before.as_str()],
                    |row| row.get(0),
                )
                .expect("should get task status");
            let rr_status_before: String = connection
                .query_row(
                    "SELECT status FROM runner_requests WHERE id = ?1",
                    params![rid.as_str()],
                    |row| row.get(0),
                )
                .expect("should get rr status");

            create_runner_preflight_review(&mut connection, valid_input(&rid))
                .expect("create should succeed");

            let task_status_after: String = connection
                .query_row(
                    "SELECT status FROM tasks WHERE id = ?1",
                    params![task_id_before.as_str()],
                    |row| row.get(0),
                )
                .expect("should get task status");
            let rr_status_after: String = connection
                .query_row(
                    "SELECT status FROM runner_requests WHERE id = ?1",
                    params![rid.as_str()],
                    |row| row.get(0),
                )
                .expect("should get rr status");
            assert_eq!(task_status_after, task_status_before);
            assert_eq!(rr_status_after, rr_status_before);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }
}
