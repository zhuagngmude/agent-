// 阶段 31：Runner 执行许可 gate
// gate 仍被阶段边界锁定，不执行 Runner、不写文件、不改 Git。

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::project_plan::ProjectPlanSideEffects;
use super::projects::get_current_project;

const CREATE_CONFIRM: &str = "我确认只创建执行许可记录，不执行Runner";
const REVOKE_CONFIRM: &str = "我确认撤销执行许可记录";
const BLOCKED_REASON: &str = "runner_execution_disabled_by_stage_boundary";
const BLOCKED_STATUS: &str = "blocked_by_stage_boundary";
const REVOKED_STATUS: &str = "revoked";

const FORBIDDEN_OPS: &[&str] = &[
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
pub struct CreateRunnerExecutionGateInput {
    pub preflight_review_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerExecutionGateInput {
    pub gate_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerExecutionGateSummary {
    pub id: String,
    pub project_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub preflight_review_id: String,
    pub preflight_approval_id: String,
    pub status: String,
    pub risk_level: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub can_execute: bool,
    pub stage_boundary_locked: bool,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub revoked_reason: Option<String>,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerExecutionGateResponse {
    pub gate: RunnerExecutionGateSummary,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct RevokeRunnerExecutionGateResponse {
    pub gate: RunnerExecutionGateSummary,
    pub side_effects: ProjectPlanSideEffects,
}

// ---------------------------------------------------------------------------
// service 函数
// ---------------------------------------------------------------------------

pub fn create_runner_execution_gate(
    connection: &mut Connection,
    input: CreateRunnerExecutionGateInput,
) -> Result<CreateRunnerExecutionGateResponse, String> {
    let project_id = get_current_project(connection)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    if input.confirm_text.trim() != CREATE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{CREATE_CONFIRM}'"
        ));
    }
    let requested_by = normalize_requested_by(input.requested_by)?;
    let prid = normalize(&input.preflight_review_id, "preflight_review_id", 200)?;

    // 读取 preflight review
    let pf = get_preflight_by_id(connection, &project_id, &prid)?
        .ok_or_else(|| "not_found: preflight review not found".to_string())?;
    if pf.status != "blocked" {
        return Err(format!(
            "invalid_input: preflight status is {}, must be blocked",
            pf.status
        ));
    }
    if !pf.blocked_reasons.contains(&BLOCKED_REASON.to_string()) {
        return Err("invalid_input: preflight missing required blocked reason".into());
    }
    for op in &pf.operation_types {
        if FORBIDDEN_OPS.contains(&op.as_str()) {
            return Err(format!(
                "invalid_input: forbidden operation type in preflight: {op}"
            ));
        }
    }
    for af in &pf.affected_files {
        validate_affected_file(af)?;
    }

    // 读取 preflight approval
    let approval = get_approval_by_id(connection, &project_id, &pf.approval_id)?
        .ok_or_else(|| "not_found: preflight approval not found".to_string())?;
    if approval.target_service != "runner_preflight" {
        return Err("invalid_input: approval is not a runner_preflight".into());
    }
    if approval.status != "approved" {
        return Err(format!(
            "invalid_input: preflight approval status is {}, must be approved",
            approval.status
        ));
    }
    if approval.task_id.as_deref() != Some(pf.task_id.as_str()) {
        return Err("invalid_input: approval task_id mismatch".into());
    }

    // 读取 runner_request
    let rr = get_runner_request(connection, &project_id, &pf.runner_request_id)?
        .ok_or_else(|| "not_found: runner request not found".to_string())?;
    if rr.status != "queued" {
        return Err(format!(
            "invalid_input: runner request status is {}, must be queued",
            rr.status
        ));
    }
    if rr.task_id != pf.task_id {
        return Err("invalid_input: runner request task_id mismatch".into());
    }
    if !rr
        .operation_types
        .contains(&"runner_request_write_files".to_string())
    {
        return Err("invalid_input: runner request is not writable".into());
    }
    if rr.operation_types != pf.operation_types {
        return Err(
            "invalid_input: operation_types mismatch between preflight and runner request".into(),
        );
    }
    if rr.affected_files != pf.affected_files {
        return Err(
            "invalid_input: affected_files mismatch between preflight and runner request".into(),
        );
    }

    // 幂等
    if let Some(gate) = find_gate_by_preflight(connection, &project_id, &prid)? {
        return Ok(CreateRunnerExecutionGateResponse {
            gate: ensure_gate_locked(gate)?,
            side_effects: all_false(),
        });
    }
    if let Some(gate) = find_gate_by_runner_request(connection, &project_id, &rr.id)? {
        return Ok(CreateRunnerExecutionGateResponse {
            gate: ensure_gate_locked(gate)?,
            side_effects: all_false(),
        });
    }

    let gate_id = format!("runner_gate_{}", normalize_id_suffix(&rr.id));
    let now = now_str();
    let op_json =
        serde_json::to_string(&rr.operation_types).map_err(|e| format!("database_error: {e}"))?;
    let af_json =
        serde_json::to_string(&rr.affected_files).map_err(|e| format!("database_error: {e}"))?;
    let br_json =
        serde_json::to_string(&[BLOCKED_REASON]).map_err(|e| format!("database_error: {e}"))?;

    connection.execute(
        "INSERT INTO runner_execution_gates (
            id, project_id, runner_request_id, task_id, preflight_review_id, preflight_approval_id,
            status, risk_level, operation_types, affected_files, blocked_reasons,
            can_execute, stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
            revoked_reason, requested_by, created_at, updated_at, revoked_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, 1, 1, 1, NULL, ?12, ?13, ?13, NULL)",
        params![
            gate_id.as_str(), project_id.as_str(), rr.id.as_str(), rr.task_id.as_str(),
            prid.as_str(), pf.approval_id.as_str(),
            BLOCKED_STATUS, pf.risk_level.as_str(),
            op_json.as_str(), af_json.as_str(), br_json.as_str(),
            requested_by.as_str(), now.as_str()
        ],
    ).map_err(|e| format!("database_error: create gate failed: {e}"))?;

    let gate = find_gate_by_id(connection, &project_id, &gate_id)?
        .ok_or_else(|| "not_found: gate not found after create".to_string())?;
    Ok(CreateRunnerExecutionGateResponse {
        gate: ensure_gate_locked(gate)?,
        side_effects: all_false(),
    })
}

pub fn auto_create_runner_execution_gate(
    connection: &mut Connection,
    preflight_review_id: String,
    requested_by: Option<String>,
) -> Result<CreateRunnerExecutionGateResponse, String> {
    create_runner_execution_gate(
        connection,
        CreateRunnerExecutionGateInput {
            preflight_review_id,
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.to_string(),
            requested_by,
        },
    )
}

pub fn list_runner_execution_gates(
    connection: &Connection,
) -> Result<Vec<RunnerExecutionGateSummary>, String> {
    let project_id = get_current_project(connection)?.id;
    let mut stmt = connection.prepare(
        "SELECT id, project_id, runner_request_id, task_id, preflight_review_id, preflight_approval_id,
            status, risk_level, operation_types, affected_files, blocked_reasons,
            can_execute, stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
            revoked_reason, requested_by, created_at, updated_at, revoked_at
         FROM runner_execution_gates WHERE project_id = ?1 ORDER BY created_at DESC, id"
    ).map_err(|e| format!("database_error: {e}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str()], map_gate_row)
        .map_err(|e| format!("database_error: {e}"))?;
    let gates = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: {e}"))?;
    gates
        .into_iter()
        .map(ensure_gate_locked)
        .collect::<Result<Vec<_>, _>>()
}

pub fn revoke_runner_execution_gate(
    connection: &mut Connection,
    input: RevokeRunnerExecutionGateInput,
) -> Result<RevokeRunnerExecutionGateResponse, String> {
    let project_id = get_current_project(connection)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    if input.confirm_text.trim() != REVOKE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{REVOKE_CONFIRM}'"
        ));
    }
    let gid = normalize(&input.gate_id, "gate_id", 200)?;
    let reason = if let Some(ref r) = input.revoked_reason {
        let rr = r.trim().to_string();
        if rr.is_empty() {
            None
        } else if rr.len() > 500 {
            return Err("invalid_input: revoked_reason too long".into());
        } else {
            crate::services::model_gateway::redaction::check_forbidden_value_patterns(&rr)?;
            Some(rr)
        }
    } else {
        None
    };

    let mut gate = find_gate_by_id(connection, &project_id, &gid)?
        .ok_or_else(|| "not_found: execution gate not found".to_string())?;
    if gate.status == REVOKED_STATUS {
        return Ok(RevokeRunnerExecutionGateResponse {
            gate: ensure_gate_locked(gate)?,
            side_effects: all_false(),
        });
    }

    let now = now_str();
    connection.execute(
        "UPDATE runner_execution_gates SET status = ?1, revoked_reason = ?2, revoked_at = ?3, updated_at = ?3
         WHERE id = ?4 AND project_id = ?5",
        params![REVOKED_STATUS, reason.as_deref(), now.as_str(), gid.as_str(), project_id.as_str()],
    ).map_err(|e| format!("database_error: revoke gate failed: {e}"))?;

    gate.status = REVOKED_STATUS.to_string();
    gate.revoked_reason = reason;
    gate.revoked_at = Some(now);
    gate.updated_at = gate.revoked_at.clone().unwrap();
    Ok(RevokeRunnerExecutionGateResponse {
        gate: ensure_gate_locked(gate)?,
        side_effects: all_false(),
    })
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct PreflightInfo {
    id: String,
    #[allow(dead_code)]
    project_id: String,
    runner_request_id: String,
    task_id: String,
    approval_id: String,
    status: String,
    risk_level: String,
    operation_types: Vec<String>,
    affected_files: Vec<String>,
    blocked_reasons: Vec<String>,
}

fn get_preflight_by_id(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<Option<PreflightInfo>, String> {
    connection.query_row(
        "SELECT id, project_id, runner_request_id, task_id, approval_id, status, risk_level, operation_types, affected_files, blocked_reasons
         FROM runner_preflight_reviews WHERE id = ?1 AND project_id = ?2",
        params![id, project_id],
        |row| Ok(PreflightInfo {
            id: row.get(0)?, project_id: row.get(1)?, runner_request_id: row.get(2)?,
            task_id: row.get(3)?, approval_id: row.get(4)?, status: row.get(5)?,
            risk_level: row.get(6)?,
            operation_types: parse_list(&row.get::<_,String>(7)?),
            affected_files: parse_list(&row.get::<_,String>(8)?),
            blocked_reasons: parse_list(&row.get::<_,String>(9)?),
        }),
    ).optional().map_err(|e| format!("database_error: {e}"))
}

struct ApproveInfo {
    target_service: String,
    status: String,
    task_id: Option<String>,
}

fn get_approval_by_id(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<Option<ApproveInfo>, String> {
    connection.query_row(
        "SELECT target_service, status, task_id FROM approvals WHERE id = ?1 AND project_id = ?2",
        params![id, project_id],
        |row| Ok(ApproveInfo { target_service: row.get(0)?, status: row.get(1)?, task_id: row.get(2)? }),
    ).optional().map_err(|e| format!("database_error: {e}"))
}

struct RrInfo {
    id: String,
    task_id: String,
    status: String,
    operation_types: Vec<String>,
    affected_files: Vec<String>,
}

fn get_runner_request(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<Option<RrInfo>, String> {
    connection.query_row(
        "SELECT id, task_id, status, operation_types, affected_files FROM runner_requests WHERE id = ?1 AND project_id = ?2",
        params![id, project_id],
        |row| Ok(RrInfo {
            id: row.get(0)?, task_id: row.get(1)?, status: row.get(2)?,
            operation_types: parse_list(&row.get::<_,String>(3)?),
            affected_files: parse_list(&row.get::<_,String>(4)?),
        }),
    ).optional().map_err(|e| format!("database_error: {e}"))
}

fn find_gate_by_preflight(
    connection: &Connection,
    project_id: &str,
    prid: &str,
) -> Result<Option<RunnerExecutionGateSummary>, String> {
    connection.query_row(
        "SELECT id FROM runner_execution_gates WHERE project_id = ?1 AND preflight_review_id = ?2",
        params![project_id, prid], |row| row.get::<_,String>(0),
    ).optional().map_err(|e| format!("database_error: {e}"))
    .and_then(|oid| match oid { Some(id) => find_gate_by_id(connection, project_id, &id), None => Ok(None) })
}

fn find_gate_by_runner_request(
    connection: &Connection,
    project_id: &str,
    rrid: &str,
) -> Result<Option<RunnerExecutionGateSummary>, String> {
    connection.query_row(
        "SELECT id FROM runner_execution_gates WHERE project_id = ?1 AND runner_request_id = ?2",
        params![project_id, rrid], |row| row.get::<_,String>(0),
    ).optional().map_err(|e| format!("database_error: {e}"))
    .and_then(|oid| match oid { Some(id) => find_gate_by_id(connection, project_id, &id), None => Ok(None) })
}

fn find_gate_by_id(
    connection: &Connection,
    project_id: &str,
    id: &str,
) -> Result<Option<RunnerExecutionGateSummary>, String> {
    connection.query_row(
        "SELECT id, project_id, runner_request_id, task_id, preflight_review_id, preflight_approval_id,
            status, risk_level, operation_types, affected_files, blocked_reasons,
            can_execute, stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
            revoked_reason, requested_by, created_at, updated_at, revoked_at
         FROM runner_execution_gates WHERE id = ?1 AND project_id = ?2",
        params![id, project_id], map_gate_row,
    ).optional().map_err(|e| format!("database_error: {e}"))
}

fn map_gate_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerExecutionGateSummary> {
    Ok(RunnerExecutionGateSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        runner_request_id: row.get(2)?,
        task_id: row.get(3)?,
        preflight_review_id: row.get(4)?,
        preflight_approval_id: row.get(5)?,
        status: row.get(6)?,
        risk_level: row.get(7)?,
        operation_types: parse_list(&row.get::<_, String>(8)?),
        affected_files: parse_list(&row.get::<_, String>(9)?),
        blocked_reasons: parse_list(&row.get::<_, String>(10)?),
        can_execute: row.get::<_, i64>(11)? != 0,
        stage_boundary_locked: row.get::<_, i64>(12)? != 0,
        requires_git_checkpoint: row.get::<_, i64>(13)? != 0,
        requires_second_confirm: row.get::<_, i64>(14)? != 0,
        revoked_reason: row.get(15)?,
        requested_by: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        revoked_at: row.get(19)?,
    })
}

fn parse_list(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn ensure_gate_locked(
    gate: RunnerExecutionGateSummary,
) -> Result<RunnerExecutionGateSummary, String> {
    if gate.can_execute {
        return Err("invalid_state: execution gate can_execute must remain false".into());
    }
    if !gate.stage_boundary_locked {
        return Err("invalid_state: execution gate stage boundary must remain locked".into());
    }
    if gate.status != BLOCKED_STATUS && gate.status != REVOKED_STATUS {
        return Err(format!(
            "invalid_state: unsupported execution gate status: {}",
            gate.status
        ));
    }
    if !gate.blocked_reasons.contains(&BLOCKED_REASON.to_string()) {
        return Err("invalid_state: execution gate missing stage boundary blocked reason".into());
    }
    Ok(gate)
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
    if path.len() > 240 {
        return Err("invalid_input: affected_file too long".into());
    }
    Ok(())
}

fn normalize(value: &str, field: &str, max: usize) -> Result<String, String> {
    let v = value.trim().to_string();
    if v.is_empty() || v.len() > max {
        return Err(format!("invalid_input: {field} length invalid"));
    }
    Ok(v)
}

fn normalize_requested_by(value: Option<String>) -> Result<String, String> {
    let v = value
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "local_user".to_string());
    if v.len() > 120 {
        return Err("invalid_input: requested_by too long".into());
    }
    crate::services::model_gateway::redaction::check_forbidden_value_patterns(&v)?;
    Ok(v)
}

fn normalize_id_suffix(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
    use crate::services::{
        project_plan::{
            approve_project_plan, create_project_plan_draft, ApproveProjectPlanInput,
            CreateProjectPlanDraftInput,
        },
        runner_preflight::{create_runner_preflight_review, CreateRunnerPreflightReviewInput},
    };
    use std::fs;

    fn test_db() -> (crate::db::DbState, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("agent-swarm-gate-test-{}", now_str()));
        let state = crate::db::initialize(dir.clone()).expect("sqlite should initialize");
        (state, dir)
    }

    fn ct(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .expect("count")
    }

    /// 创建完整的审批链：draft → approve → preflight → approve preflight → 返回 gate input
    fn setup_approved_preflight(connection: &mut Connection) -> (String, String) {
        let draft = create_project_plan_draft(
            connection,
            CreateProjectPlanDraftInput {
                idea: "test project".into(),
                constraints: None,
                requested_by: None,
            },
        )
        .expect("draft");
        approve_project_plan(
            connection,
            ApproveProjectPlanInput {
                approval_id: draft.approval.id,
                second_confirm: true,
                confirm_text: "确认生成任务".into(),
            },
        )
        .expect("approve");
        let rid: String = connection
            .query_row(
                "SELECT id FROM runner_requests ORDER BY id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .expect("rid");
        create_runner_preflight_review(
            connection,
            CreateRunnerPreflightReviewInput {
                runner_request_id: rid.clone(),
                second_confirm: true,
                confirm_text: "我确认只创建执行前审查，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("preflight");
        let preflight: (String, String) = connection.query_row(
            "SELECT id, approval_id FROM runner_preflight_reviews WHERE project_id = 'project_agent_swarm' ORDER BY id LIMIT 1",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).expect("pf");
        // 审批通过
        connection
            .execute(
                "UPDATE approvals SET status = 'approved', approved_at = '1' WHERE id = ?1",
                params![preflight.1.as_str()],
            )
            .expect("approve pf");
        (preflight.0, rid)
    }

    fn valid_create(preflight_id: &str) -> CreateRunnerExecutionGateInput {
        CreateRunnerExecutionGateInput {
            preflight_review_id: preflight_id.to_string(),
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.to_string(),
            requested_by: None,
        }
    }

    #[test]
    fn create_gate_requires_second_confirmation() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        assert!(create_runner_execution_gate(
            &mut c,
            CreateRunnerExecutionGateInput {
                second_confirm: false,
                ..valid_create(&pf)
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(create_runner_execution_gate(
            &mut c,
            CreateRunnerExecutionGateInput {
                confirm_text: "wrong".into(),
                ..valid_create(&pf)
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_rejects_unknown_preflight() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        assert!(
            create_runner_execution_gate(&mut c, valid_create("nonexistent"))
                .unwrap_err()
                .contains("not_found")
        );
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_requires_approved_preflight_approval() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        // 重置为 pending
        let aid: String = c
            .query_row(
                "SELECT approval_id FROM runner_preflight_reviews WHERE id=?1",
                params![pf.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        c.execute(
            "UPDATE approvals SET status='pending' WHERE id=?1",
            params![aid.as_str()],
        )
        .unwrap();
        assert!(create_runner_execution_gate(&mut c, valid_create(&pf))
            .unwrap_err()
            .contains("must be approved"));
        // rejected
        c.execute(
            "UPDATE approvals SET status='rejected' WHERE id=?1",
            params![aid.as_str()],
        )
        .unwrap();
        assert!(create_runner_execution_gate(&mut c, valid_create(&pf))
            .unwrap_err()
            .contains("must be approved"));
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_rejects_non_runner_preflight_approval() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let aid: String = c
            .query_row(
                "SELECT approval_id FROM runner_preflight_reviews WHERE id=?1",
                params![pf.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        c.execute(
            "UPDATE approvals SET target_service='project_plan' WHERE id=?1",
            params![aid.as_str()],
        )
        .unwrap();
        assert!(create_runner_execution_gate(&mut c, valid_create(&pf))
            .unwrap_err()
            .contains("not a runner_preflight"));
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_creates_blocked_gate_without_execution_side_effects() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let b_tasks = ct(&c, "tasks");
        let b_rr = ct(&c, "runner_requests");
        let b_events = ct(&c, "runtime_events");
        let b_mc = ct(&c, "model_calls");
        let task_status: String = c.query_row("SELECT status FROM tasks WHERE id=(SELECT task_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();
        let rr_status: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();

        let resp = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");
        assert_eq!(resp.gate.status, BLOCKED_STATUS);
        assert!(!resp.gate.can_execute);
        assert!(resp.gate.stage_boundary_locked);
        assert!(resp
            .gate
            .blocked_reasons
            .contains(&BLOCKED_REASON.to_string()));
        assert_eq!(ct(&c, "runner_execution_gates"), 1);
        assert_eq!(ct(&c, "tasks"), b_tasks);
        assert_eq!(ct(&c, "runner_requests"), b_rr);
        assert_eq!(ct(&c, "runtime_events"), b_events);
        assert_eq!(ct(&c, "model_calls"), b_mc);
        let ts2: String = c.query_row("SELECT status FROM tasks WHERE id=(SELECT task_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();
        let rs2: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();
        assert_eq!(ts2, task_status);
        assert_eq!(rs2, rr_status);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_is_idempotent_for_same_preflight() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let r1 = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("c1");
        let r2 = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("c2");
        assert_eq!(r1.gate.id, r2.gate.id);
        assert_eq!(ct(&c, "runner_execution_gates"), 1);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_rejects_polluted_preflight_affected_files() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        c.execute(
            "UPDATE runner_preflight_reviews SET affected_files='[\"../secret\"]' WHERE id=?1",
            params![pf.as_str()],
        )
        .unwrap();
        let e = create_runner_execution_gate(&mut c, valid_create(&pf)).unwrap_err();
        assert!(
            e.contains("forbidden") || e.contains("invalid_input"),
            "unexpected: {e}"
        );
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_rejects_forbidden_operation_type() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        c.execute(
            "UPDATE runner_preflight_reviews SET operation_types='[\"file_write\"]' WHERE id=?1",
            params![pf.as_str()],
        )
        .unwrap();
        let e = create_runner_execution_gate(&mut c, valid_create(&pf)).unwrap_err();
        assert!(
            e.contains("forbidden") || e.contains("invalid_input"),
            "unexpected: {e}"
        );
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_gate_rejects_changed_runner_request_scope() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let rrid: String = c
            .query_row(
                "SELECT runner_request_id FROM runner_preflight_reviews WHERE id=?1",
                params![pf.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        c.execute(
            "UPDATE runner_requests SET operation_types='[\"frontend_plan\"]' WHERE id=?1",
            params![rrid.as_str()],
        )
        .unwrap();
        let e = create_runner_execution_gate(&mut c, valid_create(&pf)).unwrap_err();
        assert!(
            e.contains("mismatch") || e.contains("not readonly") || e.contains("invalid_input"),
            "unexpected: {e}"
        );
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn list_gates_filters_current_project() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");
        let gates = list_runner_execution_gates(&c).expect("list");
        assert_eq!(gates.len(), 1);
        // 插入另一个项目的 gate
        c.execute("INSERT INTO projects (id,name,status,created_at,updated_at) VALUES ('op','O','ok','1','1')",[]).unwrap();
        c.execute("INSERT INTO approvals (id,project_id,request_agent_id,target_service,operation_types,status,risk_level,created_at,updated_at) VALUES ('a2','op','agent_architect','runner_preflight','[]','approved','medium','1','1')",[]).unwrap();
        c.execute("INSERT INTO tasks (id,project_id,title,status,priority,created_at,updated_at) VALUES ('t2','op','T','queued','medium','1','1')",[]).unwrap();
        c.execute("INSERT INTO runner_requests (id,project_id,approval_id,task_id,status,operation_types,affected_files,safety_note,created_at,updated_at) VALUES ('rr2','op','a2','t2','queued','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','s','1','1')",[]).unwrap();
        c.execute("INSERT INTO runner_preflight_reviews (id,project_id,runner_request_id,task_id,approval_id,status,risk_level,operation_types,affected_files,blocked_reasons,safety_summary,requested_by,created_at,updated_at) VALUES ('pf2','op','rr2','t2','a2','blocked','medium','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]','s','u','1','1')",[]).unwrap();
        c.execute("UPDATE approvals SET status='approved' WHERE id='a2'", [])
            .unwrap();
        c.execute("INSERT INTO runner_execution_gates (id,project_id,runner_request_id,task_id,preflight_review_id,preflight_approval_id,status,risk_level,operation_types,affected_files,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,created_at,updated_at) VALUES ('g2','op','rr2','t2','pf2','a2','blocked_by_stage_boundary','medium','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]',0,1,1,1,'u','1','1')",[]).unwrap();
        let gates2 = list_runner_execution_gates(&c).expect("list");
        assert_eq!(gates2.len(), 1);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn revoke_gate_requires_second_confirmation() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let g = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");
        assert!(revoke_runner_execution_gate(
            &mut c,
            RevokeRunnerExecutionGateInput {
                gate_id: g.gate.id.clone(),
                second_confirm: false,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(revoke_runner_execution_gate(
            &mut c,
            RevokeRunnerExecutionGateInput {
                gate_id: g.gate.id.clone(),
                second_confirm: true,
                confirm_text: "wrong".into(),
                revoked_reason: None
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn revoke_gate_marks_only_gate_revoked() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let g = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let tid: String = c
            .query_row(
                "SELECT task_id FROM runner_execution_gates WHERE id=?1",
                params![g.gate.id.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let ts: String = c
            .query_row(
                "SELECT status FROM tasks WHERE id=?1",
                params![tid.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let rr: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();
        let resp = revoke_runner_execution_gate(
            &mut c,
            RevokeRunnerExecutionGateInput {
                gate_id: g.gate.id.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: Some("test".into()),
            },
        )
        .expect("revoke");
        assert_eq!(resp.gate.status, REVOKED_STATUS);
        assert!(!resp.gate.can_execute);
        assert!(resp.gate.stage_boundary_locked);
        assert_eq!(resp.gate.revoked_reason.as_deref(), Some("test"));
        assert!(resp.gate.revoked_at.is_some());
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        let ts2: String = c
            .query_row(
                "SELECT status FROM tasks WHERE id=?1",
                params![tid.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let rr2: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_preflight_reviews WHERE id=?1)", params![pf.as_str()], |r| r.get(0)).unwrap();
        assert_eq!(ts2, ts);
        assert_eq!(rr2, rr);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn revoke_gate_is_idempotent() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let g = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");
        let input = RevokeRunnerExecutionGateInput {
            gate_id: g.gate.id.clone(),
            second_confirm: true,
            confirm_text: REVOKE_CONFIRM.into(),
            revoked_reason: None,
        };
        let r1 = revoke_runner_execution_gate(&mut c, input.clone()).expect("r1");
        let r2 = revoke_runner_execution_gate(&mut c, input).expect("r2");
        assert_eq!(r1.gate.status, REVOKED_STATUS);
        assert_eq!(r2.gate.status, REVOKED_STATUS);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn gate_schema_rejects_executable_or_unlocked_pollution() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        let g = create_runner_execution_gate(&mut c, valid_create(&pf)).expect("create");

        c.execute(
            "UPDATE runner_execution_gates SET can_execute = 1 WHERE id = ?1",
            params![g.gate.id.as_str()],
        )
        .expect_err("schema should reject executable gate pollution");
        c.execute(
            "UPDATE runner_execution_gates SET stage_boundary_locked = 0 WHERE id = ?1",
            params![g.gate.id.as_str()],
        )
        .expect_err("schema should reject unlocked gate pollution");

        let gates = list_runner_execution_gates(&c).expect("clean gate should still list");
        assert_eq!(gates.len(), 1);
        assert!(!gates[0].can_execute);
        assert!(gates[0].stage_boundary_locked);
        drop(c);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn gate_inputs_reject_unknown_fields() {
        let json = r#"{"preflight_review_id":"x","second_confirm":true,"confirm_text":"我确认只创建执行许可记录，不执行Runner","extra_field":1}"#;
        assert!(serde_json::from_str::<CreateRunnerExecutionGateInput>(json).is_err());
        let json2 = r#"{"gate_id":"x","second_confirm":true,"confirm_text":"我确认撤销执行许可记录","extra":1}"#;
        assert!(serde_json::from_str::<RevokeRunnerExecutionGateInput>(json2).is_err());
    }

    #[test]
    fn gate_rejects_sensitive_requested_by() {
        let (s, d) = test_db();
        let mut c = s.connection().unwrap();
        let (pf, _) = setup_approved_preflight(&mut c);
        assert!(create_runner_execution_gate(
            &mut c,
            CreateRunnerExecutionGateInput {
                requested_by: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into()),
                ..valid_create(&pf)
            }
        )
        .unwrap_err()
        .contains("API key"));
        assert!(revoke_runner_execution_gate(
            &mut c,
            RevokeRunnerExecutionGateInput {
                gate_id: "x".into(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into())
            }
        )
        .unwrap_err()
        .contains("API key"));
        drop(c);
        let _ = fs::remove_dir_all(d);
    }
}
