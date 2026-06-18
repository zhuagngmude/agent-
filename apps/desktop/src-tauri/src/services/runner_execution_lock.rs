// 阶段 33：Runner 执行范围锁。锁定文件范围，不执行。
use super::project_plan::ProjectPlanSideEffects;
use super::projects::get_current_project;
use super::runner_dry_run::PlannedFileChangeSummary;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const CREATE_CONFIRM: &str = "我确认锁定执行范围，不创建Git checkpoint";
const REVOKE_CONFIRM: &str = "我确认撤销执行范围锁";
const CHECKPOINT_STRATEGY: &str = "manual_checkpoint_required_before_stage34";
const WORKSPACE_REQS: &str = "clean_or_only_allowed_paths_dirty";
const BLOCKED_BOUNDARY: &str = "runner_execution_disabled_by_stage_boundary";
const BLOCKED_SCOPE: &str = "file_scope_locked_for_stage34";
const APPROVED_STATUS: &str = "approved";
const PROTECTED: &[&str] = &[
    "design/image2/",
    "_internal/",
    "data/mock/runtime-state.json",
    "data/local/",
    "logs/",
    ".playwright-cli/",
    ".git/",
    "node_modules/",
    "target/",
    "dist/",
    "build/",
    ".env",
];
const DANGEROUS: &[&str] = &[
    "git commit",
    "git push",
    "git reset",
    "git clean",
    "Remove-Item",
    "rm -rf",
    "curl",
    "wget",
    "ssh",
];

// types
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerExecutionLockInput {
    pub dry_run_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerExecutionLockInput {
    pub execution_lock_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct RunnerExecutionLockSummary {
    pub id: String,
    pub project_id: String,
    pub dry_run_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub allowed_files: Vec<String>,
    pub denied_paths: Vec<String>,
    pub planned_commands: Vec<String>,
    pub planned_file_changes: Vec<PlannedFileChangeSummary>,
    pub checkpoint_strategy: String,
    pub workspace_requirements: String,
    pub blocked_reasons: Vec<String>,
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
pub struct CreateRunnerExecutionLockResponse {
    pub execution_lock: RunnerExecutionLockSummary,
    pub side_effects: ProjectPlanSideEffects,
}
#[derive(Debug, Serialize)]
pub struct RevokeRunnerExecutionLockResponse {
    pub execution_lock: RunnerExecutionLockSummary,
    pub side_effects: ProjectPlanSideEffects,
}

// service
pub fn create_runner_execution_lock(
    c: &mut Connection,
    input: CreateRunnerExecutionLockInput,
) -> Result<CreateRunnerExecutionLockResponse, String> {
    let pid = get_current_project(c)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm required".into());
    }
    if input.confirm_text.trim() != CREATE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{CREATE_CONFIRM}'"
        ));
    }
    let req = normalize_req(input.requested_by)?;
    let did = normalize(&input.dry_run_id, "dry_run_id", 200)?;
    let dr =
        get_dry_run(c, &pid, &did)?.ok_or_else(|| "not_found: dry-run not found".to_string())?;
    if dr.project_id != pid {
        return Err("invalid_input: dry-run project mismatch".into());
    }
    if dr.status == "revoked" {
        return Err("invalid_input: dry-run is revoked".into());
    }
    let dry_run_is_blocked = dr.status == "blocked_by_stage_boundary";
    let dry_run_is_approved = dr.status == APPROVED_STATUS;
    if !dry_run_is_blocked && !dry_run_is_approved {
        return Err("invalid_input: dry-run state invalid".into());
    }
    if dry_run_is_blocked && (!dr.can_execute_ok || !dr.stage_locked_ok || !dr.requires_git_ck_ok) {
        return Err("invalid_input: dry-run state invalid".into());
    }
    if dry_run_is_approved && !dr.can_execute_open {
        return Err("invalid_input: dry-run state invalid".into());
    }
    if dry_run_is_blocked && !dr.blocked_reasons.iter().any(|r| r == BLOCKED_BOUNDARY) {
        return Err("invalid_input: dry-run missing blocked reason".into());
    }
    let gate =
        get_gate(c, &pid, &dr.gate_id)?.ok_or_else(|| "not_found: gate not found".to_string())?;
    if gate.project_id != pid {
        return Err("invalid_input: gate project mismatch".into());
    }
    let gate_is_blocked = gate.status == "blocked_by_stage_boundary";
    let gate_is_approved = gate.status == APPROVED_STATUS;
    if !gate_is_blocked && !gate_is_approved {
        return Err("invalid_input: gate state invalid".into());
    }
    if gate_is_blocked && (!gate.can_exec_ok || !gate.stage_locked_ok) {
        return Err("invalid_input: gate state invalid".into());
    }
    if gate_is_approved && !gate.can_exec_open {
        return Err("invalid_input: gate state invalid".into());
    }
    if gate.runner_request_id != dr.runner_request_id || gate.task_id != dr.task_id {
        return Err("invalid_input: gate/dry-run scope mismatch".into());
    }
    let rr = get_rr(c, &pid, &gate.runner_request_id)?
        .ok_or_else(|| "not_found: runner request not found".to_string())?;
    if rr.status != "queued" || !rr.ops.contains(&"runner_request_write_files".to_string()) {
        return Err("invalid_input: rr state invalid".into());
    }
    if rr.task_id != gate.task_id {
        return Err("invalid_input: rr/gate task mismatch".into());
    }
    // 校验 allowed_files: 只允许 virtual/... 真实路径拒绝
    if dr.allowed_files.is_empty() {
        return Err("invalid_input: allowed_files empty".into());
    }
    for f in &dr.allowed_files {
        validate_allowed_file(f)?;
        validate_virtual_only(f)?;
    }
    for f in &dr.allowed_files {
        if PROTECTED.iter().any(|p| f.starts_with(p)) {
            return Err(format!(
                "invalid_input: protected path in allowed_files: {f}"
            ));
        }
    }
    for cmd in &dr.planned_commands {
        let l = cmd.to_lowercase();
        if DANGEROUS.iter().any(|d| l.contains(&d.to_lowercase())) {
            return Err(format!("invalid_input: dangerous command: {cmd}"));
        }
    }
    for fc in &dr.planned_file_changes {
        validate_allowed_file(&fc.path)?;
        validate_virtual_only(&fc.path)?;
        if !dr.allowed_files.contains(&fc.path) {
            return Err(format!(
                "invalid_input: planned file change outside allowed: {}",
                fc.path
            ));
        }
    }
    if let Some(existing) = find_by_dry_run(c, &pid, &did)? {
        return Ok(CreateRunnerExecutionLockResponse {
            execution_lock: existing,
            side_effects: all_false(),
        });
    }
    let id = format!("exec_lock_{}", safe_slug(&did));
    let now = now_str();
    let blocked = vec![BLOCKED_SCOPE.to_string()];
    let al_j = serde_json::to_string(&dr.allowed_files).map_err(|e| format!("db:{e}"))?;
    let dp_j = serde_json::to_string(&denied_paths()).map_err(|e| format!("db:{e}"))?;
    let cm_j = serde_json::to_string(&dr.planned_commands).map_err(|e| format!("db:{e}"))?;
    let pc_j = serde_json::to_string(&dr.planned_file_changes).map_err(|e| format!("db:{e}"))?;
    let br_j = serde_json::to_string(&blocked).map_err(|e| format!("db:{e}"))?;
    c.execute("INSERT INTO runner_execution_locks (id,project_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,denied_paths,planned_commands,planned_file_changes,checkpoint_strategy,workspace_requirements,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at) VALUES (?1,?2,?3,?4,?5,?6,'locked',?7,?8,?9,?10,?11,?12,?13,1,0,0,0,?14,NULL,?15,?15,NULL)",
        params![id.as_str(),pid.as_str(),did.as_str(),dr.gate_id.as_str(),dr.runner_request_id.as_str(),dr.task_id.as_str(),al_j.as_str(),dp_j.as_str(),cm_j.as_str(),pc_j.as_str(),CHECKPOINT_STRATEGY,WORKSPACE_REQS,br_j.as_str(),req.as_str(),now.as_str()]).map_err(|e| format!("db:{e}"))?;
    let lock = find_by_id(c, &pid, &id)?
        .ok_or_else(|| "not_found: lock not found after create".to_string())?;
    Ok(CreateRunnerExecutionLockResponse {
        execution_lock: lock,
        side_effects: all_false(),
    })
}

pub fn auto_create_runner_execution_lock(
    c: &mut Connection,
    dry_run_id: String,
    requested_by: Option<String>,
) -> Result<CreateRunnerExecutionLockResponse, String> {
    create_runner_execution_lock(
        c,
        CreateRunnerExecutionLockInput {
            dry_run_id,
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.to_string(),
            requested_by,
        },
    )
}

pub fn list_runner_execution_locks(
    c: &Connection,
) -> Result<Vec<RunnerExecutionLockSummary>, String> {
    let pid = get_current_project(c)?.id;
    let mut s = c.prepare("SELECT id,project_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,denied_paths,planned_commands,planned_file_changes,checkpoint_strategy,workspace_requirements,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at FROM runner_execution_locks WHERE project_id=?1 ORDER BY created_at DESC,id").map_err(|e| format!("db:{e}"))?;
    let locks: Vec<RunnerExecutionLockSummary> = s
        .query_map(params![pid.as_str()], map_row)
        .map_err(|e| format!("db:{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid_state: lock row parse failed: {e}"))?;
    for lk in &locks {
        if !lk.can_execute_ok() {
            return Err("invalid_state: lock polluted".into());
        }
        if lk.allowed_files.is_empty() {
            return Err("invalid_state: lock has empty allowed_files".into());
        }
        for cmd in &lk.planned_commands {
            let l = cmd.to_lowercase();
            if DANGEROUS.iter().any(|d| l.contains(&d.to_lowercase())) {
                return Err(format!("invalid_state: dangerous command in lock: {cmd}"));
            }
        }
    }
    Ok(locks)
}

pub fn revoke_runner_execution_lock(
    c: &mut Connection,
    input: RevokeRunnerExecutionLockInput,
) -> Result<RevokeRunnerExecutionLockResponse, String> {
    let pid = get_current_project(c)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm required".into());
    }
    if input.confirm_text.trim() != REVOKE_CONFIRM {
        return Err(format!(
            "invalid_input: confirm_text must be '{REVOKE_CONFIRM}'"
        ));
    }
    let lid = normalize(&input.execution_lock_id, "execution_lock_id", 200)?;
    let reason = normalize_revoke(input.revoked_reason)?;
    let mut lk = find_by_id(c, &pid, &lid)?
        .ok_or_else(|| "not_found: execution lock not found".to_string())?;
    if lk.status == "revoked" {
        return Ok(RevokeRunnerExecutionLockResponse {
            execution_lock: lk,
            side_effects: all_false(),
        });
    }
    let now = now_str();
    c.execute("UPDATE runner_execution_locks SET status='revoked',revoked_reason=?1,revoked_at=?2,updated_at=?2 WHERE id=?3 AND project_id=?4",params![reason.as_deref(), now.as_str(), lid.as_str(), pid.as_str()]).map_err(|e| format!("db:{e}"))?;
    lk.status = "revoked".into();
    lk.revoked_reason = reason;
    lk.revoked_at = Some(now);
    lk.updated_at = lk.revoked_at.clone().unwrap();
    Ok(RevokeRunnerExecutionLockResponse {
        execution_lock: lk,
        side_effects: all_false(),
    })
}

// helpers
fn denied_paths() -> Vec<String> {
    PROTECTED.iter().map(|s| s.to_string()).collect()
}
fn validate_allowed_file(p: &str) -> Result<(), String> {
    if p.is_empty() || p.len() > 240 {
        return Err(format!("invalid path: {p}"));
    }
    if p.contains("..") || p.contains('\\') || p.contains(':') || p.contains('~') {
        return Err(format!("forbidden chars in: {p}"));
    }
    if p.starts_with('/') || (p.len() > 1 && p.as_bytes()[1] == b':') {
        return Err(format!("absolute path: {p}"));
    }
    Ok(())
}
fn validate_virtual_only(p: &str) -> Result<(), String> {
    if !p.starts_with("virtual/") {
        return Err(format!(
            "invalid_input: only virtual/ paths allowed, got: {p}"
        ));
    }
    Ok(())
}
fn parse_json_list(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str::<Vec<String>>(s)
        .map_err(|e| format!("invalid_state: JSON parse failed: {e}"))
}
fn parse_json_pfc(s: &str) -> Result<Vec<PlannedFileChangeSummary>, String> {
    serde_json::from_str::<Vec<PlannedFileChangeSummary>>(s)
        .map_err(|e| format!("invalid_state: JSON parse failed: {e}"))
}
#[allow(dead_code)]
struct DrInfo {
    id: String,
    project_id: String,
    gate_id: String,
    runner_request_id: String,
    task_id: String,
    status: String,
    allowed_files: Vec<String>,
    planned_commands: Vec<String>,
    planned_file_changes: Vec<PlannedFileChangeSummary>,
    blocked_reasons: Vec<String>,
    can_execute_ok: bool,
    can_execute_open: bool,
    stage_locked_ok: bool,
    requires_git_ck_ok: bool,
}
fn get_dry_run(c: &Connection, pid: &str, id: &str) -> Result<Option<DrInfo>, String> {
    c.query_row("SELECT id,project_id,gate_id,runner_request_id,task_id,status,allowed_files,planned_commands,planned_file_changes,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint FROM runner_dry_runs WHERE id=?1 AND project_id=?2",params![id,pid],|r|{
        let af: String = r.get(6)?; let cm: String = r.get(7)?; let pc: String = r.get(8)?; let br: String = r.get(9)?;
        let e = |s: String| rusqlite::Error::InvalidParameterName(s);
        let ce: i64 = r.get(10)?;
        Ok(DrInfo{id:r.get(0)?,project_id:r.get(1)?,gate_id:r.get(2)?,runner_request_id:r.get(3)?,task_id:r.get(4)?,status:r.get(5)?,allowed_files:parse_json_list(&af).map_err(&e)?,planned_commands:parse_json_list(&cm).map_err(&e)?,planned_file_changes:parse_json_pfc(&pc).map_err(&e)?,blocked_reasons:parse_json_list(&br).map_err(&e)?,can_execute_ok:ce==0,can_execute_open:ce==1,stage_locked_ok:r.get::<_,i64>(11)?==1,requires_git_ck_ok:r.get::<_,i64>(12)?==1,})
    }).optional().map_err(|e| format!("db:{e}"))
}
struct GateInfo2 {
    project_id: String,
    status: String,
    runner_request_id: String,
    task_id: String,
    can_exec_ok: bool,
    can_exec_open: bool,
    stage_locked_ok: bool,
}
fn get_gate(c: &Connection, pid: &str, id: &str) -> Result<Option<GateInfo2>, String> {
    c.query_row("SELECT project_id,status,runner_request_id,task_id,can_execute,stage_boundary_locked FROM runner_execution_gates WHERE id=?1 AND project_id=?2",params![id,pid],|r| {
        let ce: i64 = r.get(4)?;
        Ok(GateInfo2{project_id:r.get(0)?,status:r.get(1)?,runner_request_id:r.get(2)?,task_id:r.get(3)?,can_exec_ok:ce==0,can_exec_open:ce==1,stage_locked_ok:r.get::<_,i64>(5)?==1})
    }).optional().map_err(|e| format!("db:{e}"))
}
struct RrInfo2 {
    status: String,
    task_id: String,
    ops: Vec<String>,
}
fn get_rr(c: &Connection, pid: &str, id: &str) -> Result<Option<RrInfo2>, String> {
    c.query_row(
        "SELECT status,task_id,operation_types FROM runner_requests WHERE id=?1 AND project_id=?2",
        params![id, pid],
        |r| {
            let o: String = r.get(2)?;
            Ok(RrInfo2 {
                status: r.get(0)?,
                task_id: r.get(1)?,
                ops: parse_json_list(&o).map_err(|s| rusqlite::Error::InvalidParameterName(s))?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("db:{e}"))
}
fn find_by_dry_run(
    c: &Connection,
    pid: &str,
    did: &str,
) -> Result<Option<RunnerExecutionLockSummary>, String> {
    c.query_row(
        "SELECT id FROM runner_execution_locks WHERE project_id=?1 AND dry_run_id=?2",
        params![pid, did],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| format!("db:{e}"))
    .and_then(|o| match o {
        Some(id) => find_by_id(c, pid, &id),
        None => Ok(None),
    })
}
pub fn find_by_id(
    c: &Connection,
    pid: &str,
    id: &str,
) -> Result<Option<RunnerExecutionLockSummary>, String> {
    c.query_row("SELECT id,project_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,denied_paths,planned_commands,planned_file_changes,checkpoint_strategy,workspace_requirements,blocked_reasons,can_execute,stage_boundary_locked,requires_git_checkpoint,requires_second_confirm,requested_by,revoked_reason,created_at,updated_at,revoked_at FROM runner_execution_locks WHERE id=?1 AND project_id=?2",params![id,pid],map_row).optional().map_err(|e| format!("db:{e}"))
}
fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerExecutionLockSummary> {
    let af: String = r.get(7)?;
    let dp: String = r.get(8)?;
    let cm: String = r.get(9)?;
    let pc: String = r.get(10)?;
    let br: String = r.get(13)?;
    Ok(RunnerExecutionLockSummary {
        id: r.get(0)?,
        project_id: r.get(1)?,
        dry_run_id: r.get(2)?,
        gate_id: r.get(3)?,
        runner_request_id: r.get(4)?,
        task_id: r.get(5)?,
        status: r.get(6)?,
        allowed_files: parse_json_list(&af).map_err(|e| rusqlite::Error::InvalidColumnName(e))?,
        denied_paths: parse_json_list(&dp).map_err(|e| rusqlite::Error::InvalidColumnName(e))?,
        planned_commands: parse_json_list(&cm)
            .map_err(|e| rusqlite::Error::InvalidColumnName(e))?,
        planned_file_changes: parse_json_pfc(&pc)
            .map_err(|e| rusqlite::Error::InvalidColumnName(e))?,
        checkpoint_strategy: r.get(11)?,
        workspace_requirements: r.get(12)?,
        blocked_reasons: parse_json_list(&br).map_err(|e| rusqlite::Error::InvalidColumnName(e))?,
        can_execute: r.get::<_, i64>(14)? != 0,
        stage_boundary_locked: r.get::<_, i64>(15)? != 0,
        requires_git_checkpoint: r.get::<_, i64>(16)? != 0,
        requires_second_confirm: r.get::<_, i64>(17)? != 0,
        requested_by: r.get(18)?,
        revoked_reason: r.get(19)?,
        created_at: r.get(20)?,
        updated_at: r.get(21)?,
        revoked_at: r.get(22)?,
    })
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
fn normalize_revoke(v: Option<String>) -> Result<Option<String>, String> {
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
impl RunnerExecutionLockSummary {
    fn can_execute_ok(&self) -> bool {
        self.can_execute
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        project_plan::*, runner_dry_run::*, runner_execution_gate::*, runner_preflight::*,
    };
    use std::fs;
    fn td() -> (crate::db::DbState, std::path::PathBuf) {
        let d = std::env::temp_dir().join(format!("lock-{}", now_str()));
        (crate::db::initialize(d.clone()).expect("db"), d)
    }
    fn ct(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .expect("ct")
    }
    fn setup_dry_run(c: &mut Connection) -> String {
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
                preflight_review_id: pf_id,
                second_confirm: true,
                confirm_text: "我确认只创建执行许可记录，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("gate");
        let dr = create_runner_dry_run(
            c,
            CreateRunnerDryRunInput {
                gate_id: gi.gate.id,
                second_confirm: true,
                confirm_text: "我确认只生成dry-run预演，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("dr");
        dr.dry_run.id
    }
    fn valid_create(dr_id: &str) -> CreateRunnerExecutionLockInput {
        CreateRunnerExecutionLockInput {
            dry_run_id: dr_id.into(),
            second_confirm: true,
            confirm_text: CREATE_CONFIRM.into(),
            requested_by: None,
        }
    }
    fn setup_lock(c: &mut Connection) -> RunnerExecutionLockSummary {
        let dr_id = setup_dry_run(c);
        create_runner_execution_lock(c, valid_create(&dr_id))
            .expect("create")
            .execution_lock
    }

    #[test]
    fn create_lock_requires_second_confirm() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        assert!(create_runner_execution_lock(
            &mut c,
            CreateRunnerExecutionLockInput {
                second_confirm: false,
                ..valid_create(&dr_id)
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(create_runner_execution_lock(
            &mut c,
            CreateRunnerExecutionLockInput {
                confirm_text: "wrong".into(),
                ..valid_create(&dr_id)
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_unknown_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        assert!(
            create_runner_execution_lock(&mut c, valid_create("nonexistent"))
                .unwrap_err()
                .contains("not_found")
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_revoked_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        revoke_runner_dry_run(
            &mut c,
            RevokeRunnerDryRunInput {
                dry_run_id: dr_id.clone(),
                second_confirm: true,
                confirm_text: "我确认撤销dry-run预演".into(),
                revoked_reason: None,
            },
        )
        .expect("revoke");
        assert!(create_runner_execution_lock(&mut c, valid_create(&dr_id))
            .unwrap_err()
            .contains("revoked"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_creates_scope_lock_without_execution_side_effects() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let b_e = ct(&c, "runtime_events");
        let b_m = ct(&c, "model_calls");
        let ts: String = c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_dry_runs WHERE id=?1)",params![dr_id.as_str()],|r|r.get(0)).unwrap();
        let rs: String = c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_dry_runs WHERE id=?1)",params![dr_id.as_str()],|r|r.get(0)).unwrap();
        let resp = create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("create");
        assert_eq!(resp.execution_lock.status, "locked");
        assert!(resp.execution_lock.can_execute);
        assert!(!resp.execution_lock.stage_boundary_locked);
        assert!(!resp.execution_lock.requires_git_checkpoint);
        assert!(resp
            .execution_lock
            .blocked_reasons
            .iter()
            .any(|r| r == BLOCKED_SCOPE));
        assert_eq!(resp.execution_lock.checkpoint_strategy, CHECKPOINT_STRATEGY);
        assert_eq!(resp.execution_lock.workspace_requirements, WORKSPACE_REQS);
        assert!(!resp.execution_lock.denied_paths.is_empty());
        assert_eq!(ct(&c, "runner_execution_locks"), 1);
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        assert_eq!(ct(&c, "runtime_events"), b_e);
        assert_eq!(ct(&c, "model_calls"), b_m);
        assert_eq!(c.query_row("SELECT status FROM tasks WHERE id IN (SELECT task_id FROM runner_dry_runs WHERE id=?1)",params![dr_id.as_str()],|r|r.get::<_,String>(0)).unwrap(),ts);
        assert_eq!(c.query_row("SELECT status FROM runner_requests WHERE id IN (SELECT runner_request_id FROM runner_dry_runs WHERE id=?1)",params![dr_id.as_str()],|r|r.get::<_,String>(0)).unwrap(),rs);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_is_idempotent_for_same_dry_run() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        let r1 = create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("c1");
        let r2 = create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("c2");
        assert_eq!(r1.execution_lock.id, r2.execution_lock.id);
        assert_eq!(ct(&c, "runner_execution_locks"), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_real_source_path() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        // 污染为真实源码路径
        c.execute("UPDATE runner_dry_runs SET allowed_files='[\"apps/src/main.rs\",\"packages/ui/index.ts\",\"docs/readme.md\"]',planned_file_changes='[{\"path\":\"apps/src/main.rs\",\"change_type\":\"planned_review\",\"reason\":\"x\"}]' WHERE id=?1",params![dr_id.as_str()]).unwrap();
        let e = create_runner_execution_lock(&mut c, valid_create(&dr_id)).unwrap_err();
        assert!(
            e.contains("only virtual/"),
            "expected virtual/ rejection, got: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_polluted_allowed_files() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        c.execute(
            "UPDATE runner_dry_runs SET allowed_files='[\"../secret\"]' WHERE id=?1",
            params![dr_id.as_str()],
        )
        .unwrap();
        let e = create_runner_execution_lock(&mut c, valid_create(&dr_id)).unwrap_err();
        assert!(
            e.contains("forbidden") || e.contains("invalid"),
            "unexpected: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_dangerous_planned_commands() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        c.execute(
            "UPDATE runner_dry_runs SET planned_commands='[\"git commit\"]' WHERE id=?1",
            params![dr_id.as_str()],
        )
        .unwrap();
        let e = create_runner_execution_lock(&mut c, valid_create(&dr_id)).unwrap_err();
        assert!(
            e.contains("dangerous") || e.contains("invalid"),
            "unexpected: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn create_lock_rejects_scope_mismatch() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        // 创建第二个项目计划，获得不同的 gate。修改 dry-run 指向错误的 gate
        let draft2 = create_project_plan_draft(
            &mut c,
            CreateProjectPlanDraftInput {
                idea: "second".into(),
                constraints: None,
                requested_by: None,
            },
        )
        .expect("draft2");
        approve_project_plan(
            &mut c,
            ApproveProjectPlanInput {
                approval_id: draft2.approval.id,
                second_confirm: true,
                confirm_text: "确认生成任务".into(),
            },
        )
        .expect("approve2");
        let rid2: String = c
            .query_row(
                "SELECT id FROM runner_requests ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        create_runner_preflight_review(
            &mut c,
            CreateRunnerPreflightReviewInput {
                runner_request_id: rid2,
                second_confirm: true,
                confirm_text: "我确认只创建执行前审查，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("pf2");
        let (pf2_id, pa2_id): (String, String) = c
            .query_row(
                "SELECT id,approval_id FROM runner_preflight_reviews WHERE project_id='project_agent_swarm' ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        c.execute(
            "UPDATE approvals SET status='approved',approved_at='1' WHERE id=?1",
            params![pa2_id.as_str()],
        )
        .unwrap();
        let gi2 = create_runner_execution_gate(
            &mut c,
            CreateRunnerExecutionGateInput {
                preflight_review_id: pf2_id,
                second_confirm: true,
                confirm_text: "我确认只创建执行许可记录，不执行Runner".into(),
                requested_by: None,
            },
        )
        .expect("gate2");
        // 把第一个 dry-run 的 gate_id 改为第二个 gate —— scope 不匹配
        c.execute(
            "UPDATE runner_dry_runs SET gate_id=?1 WHERE id=?2",
            params![gi2.gate.id.as_str(), dr_id.as_str()],
        )
        .unwrap();
        match create_runner_execution_lock(&mut c, valid_create(&dr_id)) {
            Err(e) => assert!(
                e.contains("mismatch") || e.contains("invalid_input"),
                "unexpected: {e}"
            ),
            Ok(_) => panic!("expected error but lock was created"),
        }
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_locks_rejects_invalid_json() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("create");
        let lid: String = c
            .query_row("SELECT id FROM runner_execution_locks LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        c.execute(
            "UPDATE runner_execution_locks SET allowed_files='not-valid-json' WHERE id=?1",
            params![lid.as_str()],
        )
        .unwrap();
        let e = list_runner_execution_locks(&c).unwrap_err();
        assert!(
            e.contains("invalid_state") || e.contains("JSON"),
            "expected JSON error, got: {e}"
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn execution_lock_denied_paths_contains_protected_paths() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        let resp = create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("create");
        let dp = &resp.execution_lock.denied_paths;
        assert!(dp.iter().any(|p| p == ".git/"));
        assert!(dp.iter().any(|p| p == "design/image2/"));
        assert!(dp.iter().any(|p| p == "node_modules/"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn list_execution_locks_filters_current_project() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        create_runner_execution_lock(&mut c, valid_create(&dr_id)).expect("create");
        assert_eq!(list_runner_execution_locks(&c).unwrap().len(), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn revoke_lock_requires_second_confirmation() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        assert!(revoke_runner_execution_lock(
            &mut c,
            RevokeRunnerExecutionLockInput {
                execution_lock_id: lk.id.clone(),
                second_confirm: false,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: None
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(revoke_runner_execution_lock(
            &mut c,
            RevokeRunnerExecutionLockInput {
                execution_lock_id: lk.id.clone(),
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
    fn revoke_lock_marks_only_lock_revoked() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let resp = revoke_runner_execution_lock(
            &mut c,
            RevokeRunnerExecutionLockInput {
                execution_lock_id: lk.id.clone(),
                second_confirm: true,
                confirm_text: REVOKE_CONFIRM.into(),
                revoked_reason: Some("test".into()),
            },
        )
        .expect("revoke");
        assert_eq!(resp.execution_lock.status, "revoked");
        assert!(resp.execution_lock.can_execute);
        assert!(!resp.execution_lock.stage_boundary_locked);
        assert_eq!(resp.execution_lock.revoked_reason.as_deref(), Some("test"));
        assert!(resp.execution_lock.revoked_at.is_some());
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn revoke_lock_is_idempotent() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let inp = RevokeRunnerExecutionLockInput {
            execution_lock_id: lk.id.clone(),
            second_confirm: true,
            confirm_text: REVOKE_CONFIRM.into(),
            revoked_reason: None,
        };
        let r1 = revoke_runner_execution_lock(&mut c, inp.clone()).expect("r1");
        let r2 = revoke_runner_execution_lock(&mut c, inp).expect("r2");
        assert_eq!(r1.execution_lock.status, "revoked");
        assert_eq!(r2.execution_lock.status, "revoked");
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn lock_inputs_reject_unknown_fields() {
        assert!(serde_json::from_str::<CreateRunnerExecutionLockInput>(r#"{"dry_run_id":"x","second_confirm":true,"confirm_text":"我确认锁定执行范围，不创建Git checkpoint","extra":1}"#).is_err());
        assert!(serde_json::from_str::<RevokeRunnerExecutionLockInput>(r#"{"execution_lock_id":"x","second_confirm":true,"confirm_text":"我确认撤销执行范围锁","extra":1}"#).is_err());
    }
    #[test]
    fn lock_rejects_sensitive_requested_by() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let dr_id = setup_dry_run(&mut c);
        assert!(create_runner_execution_lock(
            &mut c,
            CreateRunnerExecutionLockInput {
                requested_by: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into()),
                ..valid_create(&dr_id)
            }
        )
        .unwrap_err()
        .contains("API key"));
        let lk = setup_lock(&mut c);
        assert!(revoke_runner_execution_lock(
            &mut c,
            RevokeRunnerExecutionLockInput {
                execution_lock_id: lk.id.clone(),
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
}
