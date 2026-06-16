// 阶段 32：Runner dry-run 预演。只生成预演计划，不执行。
use super::project_plan::ProjectPlanSideEffects;
use super::projects::get_current_project;
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const CREATE_CONFIRM: &str = "我确认只生成dry-run预演，不执行Runner";
const REVOKE_CONFIRM: &str = "我确认撤销dry-run预演";
const BLOCKED_STATUS: &str = "blocked_by_stage_boundary";
const REVOKED_STATUS: &str = "revoked";
const BLOCKED_BOUNDARY: &str = "runner_execution_disabled_by_stage_boundary";
const BLOCKED_NO_EXEC: &str = "dry_run_only_no_command_execution";
const BLOCKED_NO_WRITE: &str = "dry_run_only_no_file_write";
const SAFETY: &str = "dry-run 预演只生成计划、命令清单和影响文件清单；不会执行 Runner，不会执行命令，不会写文件，不会改 Git。";
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
const DANGEROUS_CMDS: &[&str] = &[
    "git push",
    "git reset",
    "git clean",
    "rm",
    "del",
    "Remove-Item",
    "curl",
    "wget",
    "ssh",
];

// 后端固定命令映射
fn role_commands(role: &str) -> Vec<&'static str> {
    match role {
        "frontend" => vec!["npm run typecheck", "npm run build"],
        "backend" => vec!["cargo fmt --check", "cargo check", "cargo test"],
        "qa" => vec!["cargo test", "npm run typecheck"],
        "docs" => vec!["git diff --check"],
        "reviewer" => vec!["git diff --check", "cargo test"],
        "security" => vec!["rg", "cargo test"],
        "devops" => vec!["git status --short"],
        "ux" => vec!["npm run build"],
        "data" => vec!["cargo test"],
        _ => vec!["cargo check"],
    }
}

fn role_change_type(role: &str) -> &'static str {
    match role {
        "qa" | "reviewer" | "security" => "planned_validation",
        "docs" | "ux" | "devops" | "data" => "planned_documentation",
        _ => "planned_review",
    }
}

// 类型
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerDryRunInput {
    pub gate_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerDryRunInput {
    pub dry_run_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlannedFileChangeSummary {
    pub path: String,
    pub change_type: String,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct RunnerDryRunSummary {
    pub id: String,
    pub project_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub risk_level: String,
    pub planned_operations: Vec<String>,
    pub planned_commands: Vec<String>,
    pub planned_file_changes: Vec<PlannedFileChangeSummary>,
    pub allowed_files: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub safety_summary: String,
    pub can_execute: bool,
    pub stage_boundary_locked: bool,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub requested_by: String,
    pub revoked_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct CreateRunnerDryRunResponse {
    pub dry_run: RunnerDryRunSummary,
    pub side_effects: ProjectPlanSideEffects,
}
#[derive(Debug, Serialize)]
pub struct RevokeRunnerDryRunResponse {
    pub dry_run: RunnerDryRunSummary,
    pub side_effects: ProjectPlanSideEffects,
}

// service
pub fn create_runner_dry_run(
    connection: &mut Connection,
    input: CreateRunnerDryRunInput,
) -> Result<CreateRunnerDryRunResponse, String> {
    let pid = get_current_project(connection)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    if input.confirm_text.trim() != CREATE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{CREATE_CONFIRM}'"
        ));
    }
    let requested_by = normalize_req(input.requested_by)?;
    let gid = normalize(&input.gate_id, "gate_id", 200)?;
    let gate = get_gate(connection, &pid, &gid)?
        .ok_or_else(|| "not_found: execution gate not found".to_string())?;
    if gate.status == REVOKED_STATUS {
        return Err("invalid_input: gate is revoked".into());
    }
    if gate.status != BLOCKED_STATUS {
        return Err(format!("invalid_input: gate status is {}", gate.status));
    }
    if !gate.can_execute_ok || !gate.stage_locked_ok {
        return Err("invalid_input: gate state invalid".into());
    }
    if !gate.blocked_reasons.contains(&BLOCKED_BOUNDARY.to_string()) {
        return Err("invalid_input: gate missing required blocked reason".into());
    }
    for op in &gate.operation_types {
        if FORBIDDEN_OPS.contains(&op.as_str()) {
            return Err(format!("invalid_input: forbidden op {op}"));
        }
    }
    for af in &gate.affected_files {
        validate_path(af)?;
    }
    if let Some(existing) = find_by_gate(connection, &pid, &gid)? {
        validate_summary_for_read(&existing)?;
        return Ok(CreateRunnerDryRunResponse {
            dry_run: existing,
            side_effects: all_false(),
        });
    }
    let rr = get_rr(connection, &pid, &gate.runner_request_id)?
        .ok_or_else(|| "not_found: runner request not found".to_string())?;
    if rr.status != "queued" {
        return Err(format!("invalid_input: rr status {}", rr.status));
    }
    if rr.task_id != gate.task_id {
        return Err("invalid_input: task_id mismatch".into());
    }
    if !rr.ops.contains(&"runner_request_write_files".to_string()) {
        return Err("invalid_input: rr not writable".into());
    }
    if rr.ops != gate.operation_types || rr.affected_files != gate.affected_files {
        return Err("invalid_input: scope mismatch".into());
    }
    let role = parse_role(&gate);
    let planned_commands = role_commands(&role)
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let planned_file_changes: Vec<PlannedFileChangeSummary> = gate
        .affected_files
        .iter()
        .map(|f| PlannedFileChangeSummary {
            path: f.clone(),
            change_type: role_change_type(&role).into(),
            reason: "来自只读 Runner request 的 affected_files".into(),
        })
        .collect();
    let allowed_files = gate.affected_files.clone();
    if allowed_files.is_empty() {
        return Err("invalid_input: allowed_files is empty".into());
    }
    let blocked = vec![
        BLOCKED_BOUNDARY.to_string(),
        BLOCKED_NO_EXEC.to_string(),
        BLOCKED_NO_WRITE.to_string(),
    ];
    let id = format!("runner_dry_run_{}", safe_slug(&gid));
    let now = now_str();
    let op_j = serde_json::to_string(&rr.ops).map_err(|e| format!("db: {e}"))?;
    let af_j = serde_json::to_string(&gate.affected_files).map_err(|e| format!("db: {e}"))?;
    let pc_j = serde_json::to_string(&planned_file_changes).map_err(|e| format!("db: {e}"))?;
    let cm_j = serde_json::to_string(&planned_commands).map_err(|e| format!("db: {e}"))?;
    let br_j = serde_json::to_string(&blocked).map_err(|e| format!("db: {e}"))?;
    connection.execute(
        "INSERT INTO runner_dry_runs (id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,0,1,1,1,?14,NULL,?15,?15,NULL)",
        params![id.as_str(),pid.as_str(),gid.as_str(),gate.runner_request_id.as_str(),gate.task_id.as_str(),BLOCKED_STATUS,gate.risk_level.as_str(),op_j.as_str(),cm_j.as_str(),pc_j.as_str(),af_j.as_str(),br_j.as_str(),SAFETY,requested_by.as_str(),now.as_str()],
    ).map_err(|e| format!("db: {e}"))?;
    let dr = find_by_id(connection, &pid, &id)?
        .ok_or_else(|| "not_found: dry-run not found after create".to_string())?;
    validate_summary_for_read(&dr)?;
    Ok(CreateRunnerDryRunResponse {
        dry_run: dr,
        side_effects: all_false(),
    })
}

pub fn auto_create_runner_dry_run(
    connection: &mut Connection,
    gate_id: String,
    requested_by: Option<String>,
) -> Result<CreateRunnerDryRunResponse, String> {
    create_runner_dry_run(
        connection,
        CreateRunnerDryRunInput {
            gate_id,
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.to_string(),
            requested_by,
        },
    )
}

pub fn list_runner_dry_runs(connection: &Connection) -> Result<Vec<RunnerDryRunSummary>, String> {
    let pid = get_current_project(connection)?.id;
    let mut s = connection.prepare(
        "SELECT id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at
         FROM runner_dry_runs WHERE project_id=?1 ORDER BY created_at DESC,id"
    ).map_err(|e| format!("db:{e}"))?;
    let rows = s
        .query_map(params![pid.as_str()], map_row)
        .map_err(|e| format!("db:{e}"))?;
    let dry_runs: Vec<RunnerDryRunSummary> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid_state: dry-run row invalid: {e}"))?;
    // 安全检查：拒绝被污染的 dry-run
    for dr in &dry_runs {
        validate_summary_for_read(dr)?;
    }
    Ok(dry_runs)
}

pub fn revoke_runner_dry_run(
    connection: &mut Connection,
    input: RevokeRunnerDryRunInput,
) -> Result<RevokeRunnerDryRunResponse, String> {
    let pid = get_current_project(connection)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm is required".into());
    }
    if input.confirm_text.trim() != REVOKE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{REVOKE_CONFIRM}'"
        ));
    }
    let did = normalize(&input.dry_run_id, "dry_run_id", 200)?;
    let reason = normalize_revoke_reason(input.revoked_reason)?;
    let mut dr = find_by_id(connection, &pid, &did)?
        .ok_or_else(|| "not_found: dry-run not found".to_string())?;
    if dr.status == REVOKED_STATUS {
        validate_summary_for_read(&dr)?;
        return Ok(RevokeRunnerDryRunResponse {
            dry_run: dr,
            side_effects: all_false(),
        });
    }
    // 撤销前先校验——污染数据不能伪装成正常后撤销
    validate_summary_for_read(&dr)?;
    let now = now_str();
    connection.execute("UPDATE runner_dry_runs SET status=?1,revoked_reason=?2,revoked_at=?3,updated_at=?3 WHERE id=?4 AND project_id=?5",
        params![REVOKED_STATUS, reason.as_deref(), now.as_str(), did.as_str(), pid.as_str()]).map_err(|e| format!("db:{e}"))?;
    dr.status = REVOKED_STATUS.into();
    dr.revoked_reason = reason;
    dr.revoked_at = Some(now);
    dr.updated_at = dr.revoked_at.clone().unwrap();
    validate_summary_for_read(&dr)?;
    Ok(RevokeRunnerDryRunResponse {
        dry_run: dr,
        side_effects: all_false(),
    })
}

// helpers
#[allow(dead_code)]
struct GateInfo {
    id: String,
    project_id: String,
    runner_request_id: String,
    task_id: String,
    status: String,
    risk_level: String,
    operation_types: Vec<String>,
    affected_files: Vec<String>,
    blocked_reasons: Vec<String>,
    can_execute_ok: bool,
    stage_locked_ok: bool,
}
fn get_gate(c: &Connection, pid: &str, id: &str) -> Result<Option<GateInfo>, String> {
    c.query_row("SELECT id,project_id,runner_request_id,task_id,status,risk_level,operation_types,affected_files,blocked_reasons,can_execute,stage_boundary_locked FROM runner_execution_gates WHERE id=?1 AND project_id=?2",params![id,pid],|r|{
        let ce: i64 = r.get(9)?; let sl: i64 = r.get(10)?;
        let op_str: String = r.get(6)?; let af_str: String = r.get(7)?; let br_str: String = r.get(8)?;
        Ok(GateInfo{id:r.get(0)?,project_id:r.get(1)?,runner_request_id:r.get(2)?,task_id:r.get(3)?,status:r.get(4)?,risk_level:r.get(5)?,operation_types:parse_json_list(&op_str).map_err(|e| rusqlite::Error::InvalidParameterName(e))?,affected_files:parse_json_list(&af_str).map_err(|e| rusqlite::Error::InvalidParameterName(e))?,blocked_reasons:parse_json_list(&br_str).map_err(|e| rusqlite::Error::InvalidParameterName(e))?,can_execute_ok: ce==0,stage_locked_ok: sl==1})
    }).optional().map_err(|e| format!("db:{e}"))
}
struct RrInfo {
    status: String,
    task_id: String,
    ops: Vec<String>,
    affected_files: Vec<String>,
}
fn get_rr(c: &Connection, pid: &str, id: &str) -> Result<Option<RrInfo>, String> {
    c.query_row("SELECT status,task_id,operation_types,affected_files FROM runner_requests WHERE id=?1 AND project_id=?2",params![id,pid],|r|{let op_s: String = r.get(2)?; let af_s: String = r.get(3)?; Ok(RrInfo{status:r.get(0)?,task_id:r.get(1)?,ops:parse_json_list(&op_s).map_err(|e| rusqlite::Error::InvalidParameterName(e))?,affected_files:parse_json_list(&af_s).map_err(|e| rusqlite::Error::InvalidParameterName(e))?})}).optional().map_err(|e| format!("db:{e}"))
}
fn find_by_gate(
    c: &Connection,
    pid: &str,
    gid: &str,
) -> Result<Option<RunnerDryRunSummary>, String> {
    c.query_row(
        "SELECT id FROM runner_dry_runs WHERE project_id=?1 AND gate_id=?2",
        params![pid, gid],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| format!("db:{e}"))
    .and_then(|o| match o {
        Some(id) => find_by_id(c, pid, &id),
        None => Ok(None),
    })
}
fn find_by_id(c: &Connection, pid: &str, id: &str) -> Result<Option<RunnerDryRunSummary>, String> {
    c.query_row("SELECT id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at FROM runner_dry_runs WHERE id=?1 AND project_id=?2",params![id,pid],map_row).optional().map_err(|e| format!("db:{e}"))
}
fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerDryRunSummary> {
    let poj: String = r.get(7)?;
    let cmj: String = r.get(8)?;
    let pcj: String = r.get(9)?;
    let afj: String = r.get(10)?;
    let brj: String = r.get(11)?;
    Ok(RunnerDryRunSummary {
        id: r.get(0)?,
        project_id: r.get(1)?,
        gate_id: r.get(2)?,
        runner_request_id: r.get(3)?,
        task_id: r.get(4)?,
        status: r.get(5)?,
        risk_level: r.get(6)?,
        planned_operations: parse_json_array(&poj, 7)?,
        planned_commands: parse_json_array(&cmj, 8)?,
        planned_file_changes: parse_json_array(&pcj, 9)?,
        allowed_files: parse_json_array(&afj, 10)?,
        blocked_reasons: parse_json_array(&brj, 11)?,
        safety_summary: r.get(12)?,
        can_execute: r.get::<_, i64>(13)? != 0,
        stage_boundary_locked: r.get::<_, i64>(14)? != 0,
        requires_git_checkpoint: r.get::<_, i64>(15)? != 0,
        requires_second_confirm: r.get::<_, i64>(16)? != 0,
        requested_by: r.get(17)?,
        revoked_reason: r.get(18)?,
        created_at: r.get(19)?,
        updated_at: r.get(20)?,
        revoked_at: r.get(21)?,
    })
}
fn parse_json_list(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str::<Vec<String>>(s)
        .map_err(|e| format!("invalid_state: JSON parse failed: {e}"))
}
fn parse_json_array<T: DeserializeOwned>(s: &str, column: usize) -> rusqlite::Result<Vec<T>> {
    serde_json::from_str(s)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}
fn validate_summary_for_read(dr: &RunnerDryRunSummary) -> Result<(), String> {
    if dr.status != BLOCKED_STATUS && dr.status != REVOKED_STATUS {
        return Err("invalid_state: dry-run status invalid".into());
    }
    if !dr.can_execute_ok() || !dr.stage_locked_ok() {
        return Err("invalid_state: dry-run state polluted".into());
    }
    if !dr.requires_git_checkpoint || !dr.requires_second_confirm {
        return Err("invalid_state: dry-run safety requirements polluted".into());
    }
    if !dr.blocked_reasons.contains(&BLOCKED_BOUNDARY.to_string()) {
        return Err("invalid_state: dry-run missing blocked reason".into());
    }
    if dr.allowed_files.is_empty() {
        return Err("invalid_state: dry-run allowed_files empty".into());
    }
    for op in &dr.planned_operations {
        if FORBIDDEN_OPS.contains(&op.as_str()) {
            return Err(format!("invalid_state: forbidden op in dry-run: {op}"));
        }
    }
    for file in &dr.allowed_files {
        validate_path(file).map_err(|e| format!("invalid_state: dry-run allowed file {e}"))?;
    }
    for change in &dr.planned_file_changes {
        validate_path(&change.path)
            .map_err(|e| format!("invalid_state: dry-run planned file {e}"))?;
        if !dr.allowed_files.contains(&change.path) {
            return Err("invalid_state: planned file outside allowed_files".into());
        }
        match change.change_type.as_str() {
            "planned_review" | "planned_validation" | "planned_documentation" => {}
            _ => return Err("invalid_state: planned file change_type invalid".into()),
        }
    }
    if dr.planned_file_changes.is_empty() {
        return Err("invalid_state: dry-run planned_file_changes empty".into());
    }
    for cmd in &dr.planned_commands {
        let lower = cmd.to_lowercase();
        for dc in DANGEROUS_CMDS {
            if lower.contains(&dc.to_lowercase()) {
                return Err(format!(
                    "invalid_state: dangerous command in dry-run: {cmd}"
                ));
            }
        }
    }
    Ok(())
}
fn validate_path(p: &str) -> Result<(), String> {
    if !p.starts_with("virtual/") {
        return Err(format!("invalid: {p}"));
    }
    if p.contains("..") || p.contains('\\') || p.contains(':') || p.contains('~') {
        return Err(format!("forbidden: {p}"));
    }
    if p.len() > 240 {
        return Err("too long".into());
    }
    Ok(())
}
fn normalize(v: &str, f: &str, m: usize) -> Result<String, String> {
    let v = v.trim().to_string();
    if v.is_empty() || v.len() > m {
        Err(format!("invalid {f}"))
    } else {
        Ok(v)
    }
}
fn normalize_req(v: Option<String>) -> Result<String, String> {
    let v = v
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "local_user".to_string());
    if v.len() > 120 {
        Err("too long".into())
    } else {
        super::model_gateway::redaction::check_forbidden_value_patterns(&v)?;
        Ok(v)
    }
}
fn normalize_revoke_reason(v: Option<String>) -> Result<Option<String>, String> {
    match v.filter(|s| !s.trim().is_empty()) {
        Some(r) => {
            if r.len() > 500 {
                return Err("too long".into());
            }
            super::model_gateway::redaction::check_forbidden_value_patterns(&r)?;
            Ok(Some(r))
        }
        None => Ok(None),
    }
}
fn parse_role(gate: &GateInfo) -> String {
    for role in &[
        "security", "devops", "ux", "data", "frontend", "backend", "qa", "docs", "reviewer",
    ] {
        if gate.operation_types.iter().any(|o| o.contains(role)) {
            return role.to_string();
        }
    }
    "unknown".into()
}
fn safe_slug(s: &str) -> String {
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

impl RunnerDryRunSummary {
    fn can_execute_ok(&self) -> bool {
        !self.can_execute
    }
    fn stage_locked_ok(&self) -> bool {
        self.stage_boundary_locked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{project_plan::*, runner_execution_gate::*, runner_preflight::*};
    use std::fs;
    fn td() -> (crate::db::DbState, std::path::PathBuf) {
        let d = std::env::temp_dir().join(format!("dry-{}", now_str()));
        (crate::db::initialize(d.clone()).expect("db"), d)
    }
    fn ct(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .expect("ct")
    }

    #[test]
    fn schema_rejects_polluted_requires_git_checkpoint() {
        let (s, d) = td();
        let c = s.connection().unwrap();
        // 直接 INSERT requires_git_checkpoint=0 应被 CHECK 拒绝
        let result = c.execute(
            "INSERT INTO runner_dry_runs (id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,created_at,updated_at) VALUES ('dr_test','project_agent_swarm','gx','rx','tx','blocked_by_stage_boundary','medium','[]','[]','[]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]','s',0,1,0,1,'u','1','1')",
            [],
        );
        assert!(
            result.is_err(),
            "CHECK should reject requires_git_checkpoint=0"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn schema_rejects_polluted_requires_second_confirm() {
        let (s, d) = td();
        let c = s.connection().unwrap();
        let result = c.execute(
            "INSERT INTO runner_dry_runs (id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,created_at,updated_at) VALUES ('dr_test2','project_agent_swarm','gx2','rx2','tx2','blocked_by_stage_boundary','medium','[]','[]','[]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]','s',0,1,1,0,'u','1','1')",
            [],
        );
        assert!(
            result.is_err(),
            "CHECK should reject requires_second_confirm=0"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    fn setup_gate(c: &mut Connection) -> (String, String) {
        let draft = create_project_plan_draft(
            c,
            CreateProjectPlanDraftInput {
                idea: "t".into(),
                constraints: None,
                requested_by: None,
            },
        )
        .expect("draft");
        approve_project_plan(
            c,
            ApproveProjectPlanInput {
                approval_id: draft.approval.id,
                second_confirm: true,
                confirm_text: "确认生成任务".into(),
            },
        )
        .expect("approve");
        let rid: String = c
            .query_row(
                "SELECT id FROM runner_requests ORDER BY id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .expect("rid");
        create_runner_preflight_review(
            c,
            CreateRunnerPreflightReviewInput {
                runner_request_id: rid.clone(),
                second_confirm: true,
                confirm_text: "我确认只创建执行前审查，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("pf");
        let (pf_id, pa_id): (String,String) = c.query_row("SELECT id,approval_id FROM runner_preflight_reviews WHERE project_id='project_agent_swarm' ORDER BY id LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).expect("pf");
        c.execute(
            "UPDATE approvals SET status='approved',approved_at='1' WHERE id=?1",
            params![pa_id.as_str()],
        )
        .expect("approve pf");
        let gi = create_runner_execution_gate(
            c,
            CreateRunnerExecutionGateInput {
                preflight_review_id: pf_id.clone(),
                second_confirm: true,
                confirm_text: "我确认只创建执行许可记录，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("gate");
        (gi.gate.id, rid)
    }
    fn valid_create(gid: &str) -> CreateRunnerDryRunInput {
        CreateRunnerDryRunInput {
            gate_id: gid.into(),
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.into(),
            requested_by: None,
        }
    }

    #[test]
    fn create_dry_run_requires_second_confirmation() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        assert!(create_runner_dry_run(
            &mut c,
            CreateRunnerDryRunInput {
                second_confirm: false,
                ..valid_create(&gid)
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(create_runner_dry_run(
            &mut c,
            CreateRunnerDryRunInput {
                confirm_text: "wrong".into(),
                ..valid_create(&gid)
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_rejects_unknown_gate() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        assert!(create_runner_dry_run(&mut c, valid_create("nonexistent"))
            .unwrap_err()
            .contains("not_found"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_rejects_revoked_gate() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        revoke_runner_execution_gate(
            &mut c,
            RevokeRunnerExecutionGateInput {
                gate_id: gid.clone(),
                second_confirm: true,
                confirm_text: "我确认撤销执行许可记录".into(),
                revoked_reason: None,
            },
        )
        .expect("revoke");
        assert!(create_runner_dry_run(&mut c, valid_create(&gid))
            .unwrap_err()
            .contains("revoked"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_creates_blocked_preview_without_execution_side_effects() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let b_e = ct(&c, "runtime_events");
        let b_m = ct(&c, "model_calls");
        let ts: String = c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_execution_gates WHERE id=?1)",params![gid.as_str()],|r|r.get(0)).unwrap();
        let rs: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_execution_gates WHERE id=?1)",params![gid.as_str()],|r|r.get(0)).unwrap();
        let gs: String = c
            .query_row(
                "SELECT status FROM runner_execution_gates WHERE id=?1",
                params![gid.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        let resp = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        assert_eq!(resp.dry_run.status, BLOCKED_STATUS);
        assert!(!resp.dry_run.can_execute);
        assert!(resp.dry_run.stage_boundary_locked);
        assert!(resp
            .dry_run
            .blocked_reasons
            .contains(&BLOCKED_BOUNDARY.to_string()));
        assert!(resp
            .dry_run
            .blocked_reasons
            .contains(&BLOCKED_NO_EXEC.to_string()));
        assert!(resp
            .dry_run
            .blocked_reasons
            .contains(&BLOCKED_NO_WRITE.to_string()));
        assert!(!resp.dry_run.planned_commands.is_empty());
        assert!(!resp.dry_run.planned_file_changes.is_empty());
        assert!(!resp.dry_run.allowed_files.is_empty());
        assert_eq!(ct(&c, "runner_dry_runs"), 1);
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        assert_eq!(ct(&c, "runtime_events"), b_e);
        assert_eq!(ct(&c, "model_calls"), b_m);
        assert_eq!(c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_execution_gates WHERE id=?1)",params![gid.as_str()],|r|r.get::<_,String>(0)).unwrap(),ts);
        assert_eq!(c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_execution_gates WHERE id=?1)",params![gid.as_str()],|r|r.get::<_,String>(0)).unwrap(),rs);
        assert_eq!(
            c.query_row(
                "SELECT status FROM runner_execution_gates WHERE id=?1",
                params![gid.as_str()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            gs
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_is_idempotent_for_same_gate() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r1 = create_runner_dry_run(&mut c, valid_create(&gid)).expect("c1");
        let r2 = create_runner_dry_run(&mut c, valid_create(&gid)).expect("c2");
        assert_eq!(r1.dry_run.id, r2.dry_run.id);
        assert_eq!(ct(&c, "runner_dry_runs"), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_rejects_polluted_gate_affected_files() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        c.execute(
            "UPDATE runner_execution_gates SET affected_files='[\"../secret\"]' WHERE id=?1",
            params![gid.as_str()],
        )
        .unwrap();
        let e = create_runner_dry_run(&mut c, valid_create(&gid)).unwrap_err();
        assert!(
            e.contains("forbidden") || e.contains("invalid"),
            "unexpected: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_dry_run_rejects_changed_runner_request_scope() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let rrid: String = c
            .query_row(
                "SELECT runner_request_id FROM runner_execution_gates WHERE id=?1",
                params![gid.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        c.execute(
            "UPDATE runner_requests SET operation_types='[\"frontend_plan\"]' WHERE id=?1",
            params![rrid.as_str()],
        )
        .unwrap();
        let e = create_runner_dry_run(&mut c, valid_create(&gid)).unwrap_err();
        assert!(
            e.contains("mismatch") || e.contains("not readonly") || e.contains("invalid"),
            "unexpected: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn dry_run_uses_backend_command_mapping_only() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        // 创建输入没有 command 字段（编译期保证）——验证生成的命令来自后端映射
        let resp = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        assert!(!resp.dry_run.planned_commands.is_empty());
        // backend role -> cargo fmt --check, cargo check, cargo test
        assert!(resp
            .dry_run
            .planned_commands
            .iter()
            .any(|c| c.contains("cargo")));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_dry_runs_filters_current_project() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        assert_eq!(list_runner_dry_runs(&c).unwrap().len(), 1);
        // 插入另一项目
        c.execute("INSERT INTO projects(id,name,status,created_at,updated_at) VALUES('op','O','ok','1','1')",[]).unwrap();
        c.execute("INSERT INTO approvals(id,project_id,request_agent_id,target_service,operation_types,status,risk_level,created_at,updated_at) VALUES('a3','op','agent_architect','runner_preflight','[]','approved','medium','1','1')",[]).unwrap();
        c.execute("INSERT INTO tasks(id,project_id,title,status,priority,created_at,updated_at) VALUES('t3','op','T','queued','medium','1','1')",[]).unwrap();
        c.execute("INSERT INTO runner_requests(id,project_id,approval_id,task_id,status,operation_types,affected_files,safety_note,created_at,updated_at) VALUES('rr3','op','a3','t3','queued','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','s','1','1')",[]).unwrap();
        c.execute("INSERT INTO runner_preflight_reviews(id,project_id,runner_request_id,task_id,approval_id,status,risk_level,operation_types,affected_files,blocked_reasons,safety_summary,requested_by,created_at,updated_at) VALUES('pf3','op','rr3','t3','a3','blocked','medium','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]','s','u','1','1')",[]).unwrap();
        c.execute("UPDATE approvals SET status='approved' WHERE id='a3'", [])
            .unwrap();
        c.execute("INSERT INTO runner_execution_gates(id,project_id,runner_request_id,task_id,preflight_review_id,preflight_approval_id,status,risk_level,operation_types,affected_files,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,created_at,updated_at) VALUES('g3','op','rr3','t3','pf3','a3','blocked_by_stage_boundary','medium','[\"runner_request_readonly\"]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]',0,1,1,1,'u','1','1')",[]).unwrap();
        c.execute("INSERT INTO runner_dry_runs(id,project_id,gate_id,runner_request_id,task_id,status,risk_level,planned_operations,planned_commands,planned_file_changes,allowed_files,blocked_reasons,safety_summary,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,created_at,updated_at) VALUES('dr3','op','g3','rr3','t3','blocked_by_stage_boundary','medium','[\"runner_request_readonly\"]','[\"cargo check\"]','[{\"path\":\"virtual/x.md\",\"change_type\":\"planned_review\",\"reason\":\"x\"}]','[\"virtual/x.md\"]','[\"runner_execution_disabled_by_stage_boundary\"]','s',0,1,1,1,'u','1','1')",[]).unwrap();
        assert_eq!(list_runner_dry_runs(&c).unwrap().len(), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_dry_runs_rejects_polluted_dangerous_command() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let did: String = c
            .query_row("SELECT id FROM runner_dry_runs LIMIT 1", [], |r| r.get(0))
            .unwrap();
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='[\"git push\"]' WHERE id=?1",
            params![did.as_str()],
        )
        .unwrap();
        assert!(list_runner_dry_runs(&c)
            .unwrap_err()
            .contains("dangerous command"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_dry_runs_rejects_invalid_json_pollution() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let did: String = c
            .query_row("SELECT id FROM runner_dry_runs LIMIT 1", [], |r| r.get(0))
            .unwrap();
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='not-json' WHERE id=?1",
            params![did.as_str()],
        )
        .unwrap();
        assert!(list_runner_dry_runs(&c)
            .unwrap_err()
            .contains("invalid_state"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_dry_runs_rejects_empty_allowed_files_pollution() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let did: String = c
            .query_row("SELECT id FROM runner_dry_runs LIMIT 1", [], |r| r.get(0))
            .unwrap();
        c.execute(
            "UPDATE runner_dry_runs SET allowed_files='[]' WHERE id=?1",
            params![did.as_str()],
        )
        .unwrap();
        assert!(list_runner_dry_runs(&c)
            .unwrap_err()
            .contains("allowed_files empty"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn revoke_dry_run_requires_second_confirmation() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        assert!(revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: r.dry_run.id.clone(),
                second_confirm: false,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: r.dry_run.id.clone(),
                second_confirm: true,
                confirm_text: "wrong".into(),
                revoked_reason: None
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn revoke_dry_run_marks_only_dry_run_revoked() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let ts: String = c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_dry_runs WHERE id=?1)",params![r.dry_run.id.as_str()],|r|r.get(0)).unwrap();
        let rs: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_dry_runs WHERE id=?1)",params![r.dry_run.id.as_str()],|r|r.get(0)).unwrap();
        let resp = revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: r.dry_run.id.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: Some("test".into()),
            },
        )
        .expect("revoke");
        assert_eq!(resp.dry_run.status, REVOKED_STATUS);
        assert!(!resp.dry_run.can_execute);
        assert!(resp.dry_run.stage_boundary_locked);
        assert_eq!(resp.dry_run.revoked_reason.as_deref(), Some("test"));
        assert!(resp.dry_run.revoked_at.is_some());
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        assert_eq!(c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_dry_runs WHERE id=?1)",params![r.dry_run.id.as_str()],|r|r.get::<_,String>(0)).unwrap(),ts);
        assert_eq!(c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_dry_runs WHERE id=?1)",params![r.dry_run.id.as_str()],|r|r.get::<_,String>(0)).unwrap(),rs);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn revoke_dry_run_is_idempotent() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let inp = RevokeRunnerDryRunInput {
            dry_run_id: r.dry_run.id.clone(),
            second_confirm: true,
            confirm_text: REVOKE_CONFIRM.into(),
            revoked_reason: None,
        };
        let r1 = revoke_runner_dry_run(&mut c, inp.clone()).expect("r1");
        let r2 = revoke_runner_dry_run(&mut c, inp).expect("r2");
        assert_eq!(r1.dry_run.status, REVOKED_STATUS);
        assert_eq!(r2.dry_run.status, REVOKED_STATUS);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn dry_run_inputs_reject_unknown_fields() {
        assert!(serde_json::from_str::<CreateRunnerDryRunInput>(r#"{"gate_id":"x","second_confirm":true,"confirm_text":"我确认只生成dry-run预演，不执行Runner","extra":1}"#).is_err());
        assert!(serde_json::from_str::<RevokeRunnerDryRunInput>(r#"{"dry_run_id":"x","second_confirm":true,"confirm_text":"我确认撤销dry-run预演","extra":1}"#).is_err());
    }
    #[test]
    fn dry_run_rejects_sensitive_requested_by() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        assert!(create_runner_dry_run(
            &mut c,
            CreateRunnerDryRunInput {
                requested_by: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into()),
                ..valid_create(&gid)
            }
        )
        .unwrap_err()
        .contains("API key"));
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        assert!(revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: r.dry_run.id.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into())
            }
        )
        .unwrap_err()
        .contains("API key"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn idempotent_create_rejects_polluted_existing_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let _ = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create1");
        // 污染已有的 dry-run
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='[\"git push\"]' WHERE gate_id=?1",
            params![gid.as_str()],
        )
        .unwrap();
        let e = create_runner_dry_run(&mut c, valid_create(&gid)).unwrap_err();
        assert!(
            e.contains("invalid_state"),
            "expected invalid_state, got: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn revoke_rejects_polluted_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let did = r.dry_run.id;
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='[\"git push\"]' WHERE id=?1",
            params![did.as_str()],
        )
        .unwrap();
        let e = revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: did.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None,
            },
        )
        .unwrap_err();
        assert!(
            e.contains("invalid_state"),
            "expected invalid_state, got: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn idempotent_revoke_rejects_polluted_already_revoked_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let r = create_runner_dry_run(&mut c, valid_create(&gid)).expect("create");
        let did = r.dry_run.id;
        revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: did.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None,
            },
        )
        .expect("revoke1");
        // 污染已撤销的 dry-run
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='[\"git push\"]' WHERE id=?1",
            params![did.as_str()],
        )
        .unwrap();
        let e = revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: did.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None,
            },
        )
        .unwrap_err();
        assert!(
            e.contains("invalid_state"),
            "expected invalid_state, got: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_dry_run_rejects_corrupt_gate_json() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        // 污染 gate 的 JSON 字段为非 JSON
        c.execute(
            "UPDATE runner_execution_gates SET affected_files='not-json' WHERE id=?1",
            params![gid.as_str()],
        )
        .unwrap();
        let e = create_runner_dry_run(&mut c, valid_create(&gid)).unwrap_err();
        assert!(
            e.contains("JSON") || e.contains("invalid_state"),
            "expected JSON error, got: {e}"
        );
        // 不应写入任何行
        assert_eq!(ct(&c, "runner_dry_runs"), 0);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn create_dry_run_rejects_corrupt_runner_request_json() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let (gid, _) = setup_gate(&mut c);
        let rrid: String = c
            .query_row(
                "SELECT runner_request_id FROM runner_execution_gates WHERE id=?1",
                params![gid.as_str()],
                |r| r.get(0),
            )
            .unwrap();
        c.execute(
            "UPDATE runner_requests SET operation_types='not-json' WHERE id=?1",
            params![rrid.as_str()],
        )
        .unwrap();
        let e = create_runner_dry_run(&mut c, valid_create(&gid)).unwrap_err();
        assert!(
            e.contains("JSON") || e.contains("invalid_state"),
            "expected JSON error, got: {e}"
        );
        assert_eq!(ct(&c, "runner_dry_runs"), 0);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
}
