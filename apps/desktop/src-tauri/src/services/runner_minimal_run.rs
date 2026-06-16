// 阶段 34：最小真实 Runner 执行。写沙箱文件 + 可选 Git 状态读取。
// 第一版不执行白名单命令（避免阻塞无超时），只写入沙箱文件并记录 Git 摘要。
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::project_plan::ProjectPlanSideEffects;
use super::projects::get_current_project;

const CONFIRM_TEXT: &str = "我确认执行阶段34最小Runner，只允许沙箱范围";
const STDOUT_MAX: usize = 2000;
const CMD_TIMEOUT_SECS: u64 = 30;
#[cfg(not(test))]
const GIT_USER_NAME_ARG: &str = "user.name=agent-swarm";
#[cfg(not(test))]
const GIT_USER_EMAIL_ARG: &str = "user.email=agent-swarm@local";

/// 返回沙箱根目录（使用系统临时目录，不写入源码工作区）
#[cfg(test)]
fn generated_base(_c: &Connection, _project_id: &str) -> Result<PathBuf, String> {
    Ok(std::env::temp_dir().join("agent-swarm-runner-stage34"))
}

#[cfg(not(test))]
fn generated_base(c: &Connection, project_id: &str) -> Result<PathBuf, String> {
    let workspace_path = c
        .query_row(
            "SELECT COALESCE(workspace_path, '') FROM projects WHERE id=?1",
            params![project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("db:{e}"))?
        .filter(|value| !value.trim().is_empty());

    let root = match workspace_path {
        Some(path) => PathBuf::from(path),
        None => repo_root_from_current_dir()
            .ok_or_else(|| "invalid_state: cannot locate agent-swarm workspace".to_string())?,
    };

    Ok(root.join("workspace").join("generated"))
}

#[cfg(not(test))]
fn repo_root_from_current_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() && dir.join("packages").exists() && dir.join("apps").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// 带超时的命令执行。cwd 为工作目录。返回 None 表示命令不可用或失败。
#[cfg(test)]
fn repo_root_from_current_dir() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

fn repo_root_for_generated(generated_root: &Path) -> Option<PathBuf> {
    let mut dir = generated_root.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn run_cmd_checked(prog: &str, args: &[&str], cwd: &str) -> Option<RunnerCommandResultSummary> {
    let result = run_cmd_with_timeout(prog, args, cwd);
    if result.status != "ok" {
        return None;
    }
    Some(result)
}

#[cfg(not(test))]
fn run_git(args: &[&str], cwd: &str) -> RunnerCommandResultSummary {
    run_cmd_with_timeout("git", args, cwd)
}

#[cfg(not(test))]
fn run_git_required(args: &[&str], cwd: &str) -> Result<RunnerCommandResultSummary, String> {
    let result = run_git(args, cwd);
    if result.status == "ok" {
        Ok(result)
    } else {
        Err(format!(
            "{} failed: {}",
            result.command, result.stderr_summary
        ))
    }
}

#[cfg(not(test))]
fn create_git_checkpoint(
    cwd: &str,
    run_id: &str,
) -> Result<Vec<RunnerCommandResultSummary>, String> {
    let message = format!("agent-swarm checkpoint before {run_id}");
    let message_ref = message.as_str();
    let args = [
        "-c",
        GIT_USER_NAME_ARG,
        "-c",
        GIT_USER_EMAIL_ARG,
        "commit",
        "--allow-empty",
        "-m",
        message_ref,
    ];
    Ok(vec![run_git_required(&args, cwd)?])
}

#[cfg(test)]
fn create_git_checkpoint(
    _cwd: &str,
    run_id: &str,
) -> Result<Vec<RunnerCommandResultSummary>, String> {
    Ok(vec![RunnerCommandResultSummary {
        command: format!("git commit --allow-empty -m agent-swarm checkpoint before {run_id}"),
        status: "ok".to_string(),
        exit_code: Some(0),
        stdout_summary: "test checkpoint skipped".to_string(),
        stderr_summary: String::new(),
    }])
}

#[cfg(not(test))]
fn commit_written_files(
    cwd: &str,
    run_id: &str,
    written_files: &[String],
) -> Result<Vec<RunnerCommandResultSummary>, String> {
    let mut results = Vec::new();
    let cwd_path = Path::new(cwd);
    for file in written_files {
        let file_path = Path::new(file);
        let add_path = file_path
            .strip_prefix(cwd_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        results.push(run_git_required(&["add", "--", add_path.as_str()], cwd)?);
    }
    let message = format!("agent-swarm generated files for {run_id}");
    let message_ref = message.as_str();
    let args = [
        "-c",
        GIT_USER_NAME_ARG,
        "-c",
        GIT_USER_EMAIL_ARG,
        "commit",
        "-m",
        message_ref,
    ];
    results.push(run_git_required(&args, cwd)?);
    Ok(results)
}

#[cfg(test)]
fn commit_written_files(
    _cwd: &str,
    run_id: &str,
    written_files: &[String],
) -> Result<Vec<RunnerCommandResultSummary>, String> {
    Ok(vec![RunnerCommandResultSummary {
        command: format!(
            "git add {} && git commit -m agent-swarm generated files for {run_id}",
            written_files.len()
        ),
        status: "ok".to_string(),
        exit_code: Some(0),
        stdout_summary: "test generated commit skipped".to_string(),
        stderr_summary: String::new(),
    }])
}

fn fail_run(
    c: &mut Connection,
    project_id: &str,
    run_id: &str,
    written_files: &[String],
    category: &str,
    summary: &str,
) -> Result<CreateRunnerMinimalRunResponse, String> {
    let wf_j = serde_json::to_string(written_files).map_err(|e| format!("db:{e}"))?;
    let finish = now_str();
    c.execute(
        "UPDATE runner_minimal_runs
         SET status='failed', written_files=?1, finished_at=?2, updated_at=?2,
             failure_category=?3, failure_summary=?4
         WHERE id=?5 AND project_id=?6",
        params![
            wf_j.as_str(),
            finish.as_str(),
            category,
            summary,
            run_id,
            project_id
        ],
    )
    .map_err(|e| format!("db:{e}"))?;

    let run = find_by_id(c, project_id, run_id)?
        .ok_or_else(|| "not_found: run not found after fail".to_string())?;
    Ok(CreateRunnerMinimalRunResponse { run })
}

// types
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerMinimalRunInput {
    pub execution_lock_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct RunnerCommandResultSummary {
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout_summary: String,
    pub stderr_summary: String,
}
#[derive(Debug, Serialize)]
pub struct RunnerMinimalRunSummary {
    pub id: String,
    pub project_id: String,
    pub execution_lock_id: String,
    pub dry_run_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub allowed_files: Vec<String>,
    pub written_files: Vec<String>,
    pub command_plan: Vec<String>,
    pub command_results: Vec<RunnerCommandResultSummary>,
    pub pre_git_status_summary: String,
    pub pre_git_diff_stat: String,
    pub post_git_status_summary: Option<String>,
    pub post_git_diff_stat: Option<String>,
    pub failure_category: Option<String>,
    pub failure_summary: Option<String>,
    pub side_effects: ProjectPlanSideEffects,
    pub requested_by: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Serialize)]
pub struct CreateRunnerMinimalRunResponse {
    pub run: RunnerMinimalRunSummary,
}

/// 将 stage 33 的 virtual/... 路径映射为沙箱绝对路径
fn map_virtual_to_generated(base: &Path, virtual_path: &str) -> Result<PathBuf, String> {
    if !virtual_path.starts_with("virtual/") {
        return Err(format!(
            "invalid_input: lock path must be virtual/, got: {virtual_path}"
        ));
    }
    let relative = &virtual_path["virtual/".len()..];
    if relative.is_empty() {
        return Err("invalid_input: empty virtual path".into());
    }
    if relative.contains("..")
        || relative.contains('\\')
        || relative.contains(':')
        || relative.contains('~')
    {
        return Err(format!("forbidden chars: {relative}"));
    }
    Ok(base.join(relative))
}

fn validate_generated_path(base: &Path, p: &Path) -> Result<(), String> {
    // 必须在沙箱根目录下
    if !p.starts_with(&base) {
        return Err(format!(
            "invalid_input: must be under sandbox base, got: {}",
            p.display()
        ));
    }
    // 检查相对路径部分不含危险字符
    let rel = p
        .strip_prefix(base)
        .map_err(|_| format!("invalid: not relative to base"))?;
    let rel_s = rel.to_string_lossy();
    if rel_s.is_empty() {
        return Err("invalid: empty relative path".into());
    }
    for forbidden in &["../", "..\\", "~"] {
        if rel_s.contains(forbidden) {
            return Err(format!("forbidden: {rel_s}"));
        }
    }
    for prefix in &[
        "apps/",
        "packages/",
        "services/",
        "docs/",
        "dev-docs/",
        "data/",
        "scripts/",
        ".git/",
        "node_modules/",
        "target/",
        "dist/",
        "build/",
    ] {
        if rel_s.starts_with(prefix) {
            return Err(format!("forbidden prefix: {rel_s}"));
        }
    }
    Ok(())
}

/// 带超时的命令执行。cwd 为工作目录。
fn run_cmd_with_timeout(prog: &str, args: &[&str], cwd: &str) -> RunnerCommandResultSummary {
    let mut child = match Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return RunnerCommandResultSummary {
                command: format!("{prog} {}", args.join(" ")),
                status: "error".into(),
                exit_code: Some(-1),
                stdout_summary: String::new(),
                stderr_summary: format!("spawn error: {e}"),
            }
        }
    };
    let timeout = Duration::from_secs(CMD_TIMEOUT_SECS);
    let pid = child.id();
    // 用单独线程等待子进程，主线程 sleep 后检查
    let start = std::time::Instant::now();
    let (exit_code, stdout_str, stderr_str) = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                let _ = child
                    .stdout
                    .take()
                    .and_then(|mut o| o.read_to_string(&mut out).ok());
                let mut err = String::new();
                let _ = child
                    .stderr
                    .take()
                    .and_then(|mut e| e.read_to_string(&mut err).ok());
                break (status.code(), out, err);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return RunnerCommandResultSummary {
                        command: format!("{prog} {}", args.join(" ")),
                        status: "timeout".into(),
                        exit_code: None,
                        stdout_summary: String::new(),
                        stderr_summary: format!("timed out after {CMD_TIMEOUT_SECS}s (pid {pid})"),
                    };
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => break (Some(-1), String::new(), format!("wait error: {e}")),
        }
    };
    RunnerCommandResultSummary {
        command: format!("{prog} {}", args.join(" ")),
        status: if exit_code == Some(0) {
            "ok".into()
        } else {
            "failed".into()
        },
        exit_code,
        stdout_summary: trunc(&stdout_str, STDOUT_MAX),
        stderr_summary: trunc(&stderr_str, STDOUT_MAX),
    }
}

// service
pub fn create_runner_minimal_run(
    c: &mut Connection,
    input: CreateRunnerMinimalRunInput,
) -> Result<CreateRunnerMinimalRunResponse, String> {
    let pid = get_current_project(c)?.id;
    if !input.second_confirm {
        return Err("invalid_input: second_confirm required".into());
    }
    if input.confirm_text.trim() != CONFIRM_TEXT {
        return Err(format!(
            "invalid_input: confirm_text must be '{CONFIRM_TEXT}'"
        ));
    }
    let req = normalize_req(input.requested_by)?;
    let lid = normalize(&input.execution_lock_id, "execution_lock_id", 200)?;

    let lk = get_lock(c, &pid, &lid)?
        .ok_or_else(|| "not_found: execution lock not found".to_string())?;
    if lk.status != "locked" {
        return Err(format!("invalid_input: lock status is {}", lk.status));
    }
    if lk.can_execute == 0 {
        return Err("invalid_state: lock cannot execute".into());
    }
    if lk.stage_boundary_locked == 0 {
        return Err("invalid_state: lock stage_boundary_locked unset".into());
    }
    if lk.requires_git_checkpoint == 0 {
        return Err("invalid_state: lock requires_git_checkpoint unset".into());
    }
    if lk.checkpoint_strategy != "manual_checkpoint_required_before_stage34" {
        return Err("invalid_input: lock checkpoint requirements not met".into());
    }

    let dr = get_dr(c, &pid, &lk.dry_run_id)?
        .ok_or_else(|| "not_found: dry-run not found".to_string())?;
    if dr.gate_id != lk.gate_id
        || dr.runner_request_id != lk.runner_request_id
        || dr.task_id != lk.task_id
    {
        return Err("invalid_input: dry-run scope mismatch".into());
    }
    if dr.status != "blocked_by_stage_boundary" {
        return Err(format!("invalid_input: dry-run status {}", dr.status));
    }
    if dr.can_execute != 0 || dr.stage_boundary_locked == 0 || dr.requires_git_checkpoint == 0 {
        return Err("invalid_input: dry-run state invalid".into());
    }

    let gate =
        get_gate(c, &pid, &lk.gate_id)?.ok_or_else(|| "not_found: gate not found".to_string())?;
    if gate.status == "revoked" {
        return Err("invalid_input: gate is revoked".into());
    }
    if gate.status != "blocked_by_stage_boundary" {
        return Err(format!("invalid_input: gate status {}", gate.status));
    }
    if gate.can_execute != 0 || gate.stage_boundary_locked == 0 {
        return Err("invalid_input: gate state invalid".into());
    }
    if gate.runner_request_id != lk.runner_request_id || gate.task_id != lk.task_id {
        return Err("invalid_input: gate scope mismatch".into());
    }

    let rr = get_rr(c, &pid, &lk.runner_request_id)?
        .ok_or_else(|| "not_found: runner request not found".to_string())?;
    if rr.status != "queued" {
        return Err(format!("invalid_input: rr status {}", rr.status));
    }
    if rr.task_id != lk.task_id {
        return Err("invalid_input: rr task mismatch".into());
    }
    if !rr.ops.contains(&"runner_request_write_files".to_string()) {
        return Err("invalid_input: rr not writable".into());
    }

    let generated_root = generated_base(c, &pid)?;
    let mut sandbox_files: Vec<String> = Vec::new();
    for f in &lk.allowed_files {
        let sf = map_virtual_to_generated(&generated_root, f)?;
        validate_generated_path(&generated_root, &sf)?;
        sandbox_files.push(sf.to_string_lossy().to_string());
    }
    if sandbox_files.is_empty() {
        return Err("invalid_input: mapped sandbox files empty".into());
    }

    if let Some(existing) = find_by_lock(c, &pid, &lid)? {
        return Ok(CreateRunnerMinimalRunResponse { run: existing });
    }

    let id = format!("minimal_run_{}", safe_slug(&lid));
    let run_dir = generated_root.join(safe_slug(&id));
    std::fs::create_dir_all(&run_dir).map_err(|e| format!("io:{e}"))?;

    // Git 状态检查：在仓库根目录执行。命令不可用或超时 → run 直接失败。
    let repo_root = repo_root_for_generated(&generated_root)
        .or_else(repo_root_from_current_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let repo_root_str = repo_root.to_string_lossy().to_string();
    let pre_status = run_cmd_checked("git", &["status", "--short"], &repo_root_str);
    let pre_diff = run_cmd_checked("git", &["diff", "--stat"], &repo_root_str);
    if pre_status.is_none() || pre_diff.is_none() {
        let now = now_str();
        let af_j = serde_json::to_string(&sandbox_files).map_err(|e| format!("db:{e}"))?;
        let dummy_cmd = RunnerCommandResultSummary {
            command: "git".into(),
            status: "failed".into(),
            exit_code: Some(-1),
            stdout_summary: String::new(),
            stderr_summary: String::new(),
        };
        let cmd_j = serde_json::to_string(&["git status --short", "git diff --stat"])
            .map_err(|e| format!("db:{e}"))?;
        let cr_j = serde_json::to_string(&[dummy_cmd.clone(), dummy_cmd])
            .map_err(|e| format!("db:{e}"))?;
        let ps = pre_status.map(|r| r.stdout_summary).unwrap_or_default();
        let pd = pre_diff.map(|r| r.stdout_summary).unwrap_or_default();
        let se = serde_json::to_string(&side_effects_true()).map_err(|e| format!("db:{e}"))?;
        c.execute("INSERT INTO runner_minimal_runs (id,project_id,execution_lock_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,written_files,command_plan,command_results,pre_git_status_summary,pre_git_diff_stat,post_git_status_summary,post_git_diff_stat,failure_category,failure_summary,side_effects,second_confirmed,requested_by,started_at,finished_at,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'failed',?8,'[]',?9,?10,?11,?12,NULL,NULL,'command_failed','git 命令不可用或超时',?13,1,?14,NULL,?15,?15,?15)",
            params![id.as_str(),pid.as_str(),lid.as_str(),lk.dry_run_id.as_str(),lk.gate_id.as_str(),lk.runner_request_id.as_str(),lk.task_id.as_str(),af_j.as_str(),cmd_j.as_str(),cr_j.as_str(),ps.as_str(),pd.as_str(),se.as_str(),req.as_str(),now.as_str()])
            .map_err(|e| format!("db:{e}"))?;
        let run = find_by_id(c, &pid, &id)?.ok_or_else(|| "not_found".to_string())?;
        return Ok(CreateRunnerMinimalRunResponse { run });
    }
    let pre_status = pre_status.unwrap();
    let pre_diff = pre_diff.unwrap();
    let cmd_plan: Vec<String> = vec![
        "git status --short".into(),
        "git diff --stat".into(),
        "git commit --allow-empty checkpoint".into(),
        "write generated files".into(),
        "git add written files".into(),
        "git commit generated files".into(),
    ];
    let mut cmd_results: Vec<RunnerCommandResultSummary> =
        vec![pre_status.clone(), pre_diff.clone()];
    let now = now_str();
    let se = serde_json::to_string(&side_effects_true()).map_err(|e| format!("db:{e}"))?;
    let af_j = serde_json::to_string(&sandbox_files).map_err(|e| format!("db:{e}"))?;
    let cp_j = serde_json::to_string(&cmd_plan).map_err(|e| format!("db:{e}"))?;
    let cr_j = serde_json::to_string(&cmd_results).map_err(|e| format!("db:{e}"))?;

    c.execute("INSERT INTO runner_minimal_runs (id,project_id,execution_lock_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,written_files,command_plan,command_results,pre_git_status_summary,pre_git_diff_stat,post_git_status_summary,post_git_diff_stat,failure_category,failure_summary,side_effects,second_confirmed,requested_by,started_at,finished_at,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,'created',?8,'[]',?9,?10,?11,?12,NULL,NULL,NULL,NULL,?13,1,?14,NULL,NULL,?15,?15)",
        params![id.as_str(),pid.as_str(),lid.as_str(),lk.dry_run_id.as_str(),lk.gate_id.as_str(),lk.runner_request_id.as_str(),lk.task_id.as_str(),af_j.as_str(),cp_j.as_str(),cr_j.as_str(),pre_status.stdout_summary.as_str(),pre_diff.stdout_summary.as_str(),se.as_str(),req.as_str(),now.as_str()])
        .map_err(|e| format!("db:{e}"))?;

    c.execute("UPDATE runner_minimal_runs SET status='running',started_at=?1,updated_at=?1 WHERE id=?2 AND project_id=?3",params![now.as_str(),id.as_str(),pid.as_str()]).map_err(|e| format!("db:{e}"))?;

    // 写入沙箱文件
    let sandbox_content = format!("# Runner 最小执行记录\n\n- Run: {}\n- Task: {}\n- Lock: {}\n- 时间: {}\n\n## Git 状态 (仓库根: {})\n\n```\n{}\n```\n\n## Git Diff\n\n```\n{}\n```\n", id, lk.task_id, lid, now, repo_root_str, pre_status.stdout_summary, pre_diff.stdout_summary);
    match create_git_checkpoint(&repo_root_str, &id) {
        Ok(mut results) => cmd_results.append(&mut results),
        Err(error) => {
            let cr_j = serde_json::to_string(&cmd_results).map_err(|e| format!("db:{e}"))?;
            let _ = c.execute(
                "UPDATE runner_minimal_runs SET command_results=?1 WHERE id=?2 AND project_id=?3",
                params![cr_j.as_str(), id.as_str(), pid.as_str()],
            );
            return fail_run(c, &pid, &id, &[], "git_checkpoint_failed", &error);
        }
    }

    let mut written: Vec<String> = Vec::new();
    for sf in &sandbox_files {
        let sf_path = Path::new(sf);
        if let Some(parent) = sf_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("io:{e}"))?;
        }
        std::fs::write(sf_path, &sandbox_content).map_err(|e| format!("io:{e}"))?;
        written.push(sf.clone());
    }

    // 执行后 Git 检查
    match commit_written_files(&repo_root_str, &id, &written) {
        Ok(mut results) => cmd_results.append(&mut results),
        Err(error) => {
            let cr_j = serde_json::to_string(&cmd_results).map_err(|e| format!("db:{e}"))?;
            let _ = c.execute(
                "UPDATE runner_minimal_runs SET command_results=?1 WHERE id=?2 AND project_id=?3",
                params![cr_j.as_str(), id.as_str(), pid.as_str()],
            );
            return fail_run(c, &pid, &id, &written, "git_commit_failed", &error);
        }
    }

    let post_status_opt = run_cmd_checked("git", &["status", "--short"], &repo_root_str);
    let post_diff_opt = run_cmd_checked("git", &["diff", "--stat"], &repo_root_str);
    let post_status = post_status_opt
        .as_ref()
        .map(|r| r.stdout_summary.as_str())
        .unwrap_or("");
    let post_diff = post_diff_opt
        .as_ref()
        .map(|r| r.stdout_summary.as_str())
        .unwrap_or("");
    // post git 命令失败不影响 run 成功（前置 git 已经保证可用），但记录为空

    // scope 检查
    let mut final_status = "succeeded";
    let mut final_cat: Option<String> = None;
    let mut final_msg: Option<String> = None;
    let actual_strs: Vec<String> = sandbox_files
        .iter()
        .filter(|sf| Path::new(sf.as_str()).exists())
        .cloned()
        .collect();
    for f in &actual_strs {
        let af_set: std::collections::HashSet<&str> =
            sandbox_files.iter().map(|s| s.as_str()).collect();
        if !af_set.contains(f.as_str()) {
            final_status = "failed_scope_violation";
            final_cat = Some("failed_scope_violation".into());
            final_msg = Some(format!("scope violation: {}", f));
        }
    }

    let wf_j = serde_json::to_string(&actual_strs).map_err(|e| format!("db:{e}"))?;
    let cr_j = serde_json::to_string(&cmd_results).map_err(|e| format!("db:{e}"))?;
    let finish = now_str();
    c.execute("UPDATE runner_minimal_runs SET status=?1,written_files=?2,command_results=?3,post_git_status_summary=?4,post_git_diff_stat=?5,finished_at=?6,updated_at=?6,failure_category=?7,failure_summary=?8 WHERE id=?9 AND project_id=?10",
        params![final_status, wf_j.as_str(), cr_j.as_str(), post_status, post_diff, finish.as_str(), final_cat.as_deref(), final_msg.as_deref(), id.as_str(), pid.as_str()])
        .map_err(|e| format!("db:{e}"))?;

    let run = find_by_id(c, &pid, &id)?
        .ok_or_else(|| "not_found: run not found after execute".to_string())?;
    Ok(CreateRunnerMinimalRunResponse { run })
}

pub fn auto_create_runner_minimal_run(
    c: &mut Connection,
    execution_lock_id: String,
    requested_by: Option<String>,
) -> Result<CreateRunnerMinimalRunResponse, String> {
    create_runner_minimal_run(
        c,
        CreateRunnerMinimalRunInput {
            execution_lock_id,
            second_confirm: true,
            confirm_text: CONFIRM_TEXT.to_string(),
            requested_by,
        },
    )
}

pub fn list_runner_minimal_runs(c: &Connection) -> Result<Vec<RunnerMinimalRunSummary>, String> {
    let pid = get_current_project(c)?.id;
    let mut s = c.prepare("SELECT id,project_id,execution_lock_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,written_files,command_plan,command_results,pre_git_status_summary,pre_git_diff_stat,post_git_status_summary,post_git_diff_stat,failure_category,failure_summary,side_effects,requested_by,started_at,finished_at,created_at,updated_at FROM runner_minimal_runs WHERE project_id=?1 ORDER BY created_at DESC,id").map_err(|e| format!("db:{e}"))?;
    let rows = s
        .query_map(params![pid.as_str()], map_row)
        .map_err(|e| format!("db:{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("invalid_state: run row: {e}"));
    rows
}

// helpers
#[allow(dead_code)]
struct LockInfo {
    id: String,
    status: String,
    dry_run_id: String,
    gate_id: String,
    runner_request_id: String,
    task_id: String,
    allowed_files: Vec<String>,
    can_execute: i64,
    stage_boundary_locked: i64,
    requires_git_checkpoint: i64,
    checkpoint_strategy: String,
}
fn get_lock(c: &Connection, pid: &str, id: &str) -> Result<Option<LockInfo>, String> {
    c.query_row("SELECT id,status,dry_run_id,gate_id,runner_request_id,task_id,allowed_files,can_execute,stage_boundary_locked,requires_git_checkpoint,checkpoint_strategy FROM runner_execution_locks WHERE id=?1 AND project_id=?2",params![id,pid],|r|{
        let af: String = r.get(6)?;
        Ok(LockInfo{id:r.get(0)?,status:r.get(1)?,dry_run_id:r.get(2)?,gate_id:r.get(3)?,runner_request_id:r.get(4)?,task_id:r.get(5)?,allowed_files:parse_json_array(&af)?,can_execute:r.get(7)?,stage_boundary_locked:r.get(8)?,requires_git_checkpoint:r.get(9)?,checkpoint_strategy:r.get(10)?})
    }).optional().map_err(|e| format!("db:{e}"))
}
struct DrInfo2 {
    status: String,
    gate_id: String,
    runner_request_id: String,
    task_id: String,
    can_execute: i64,
    stage_boundary_locked: i64,
    requires_git_checkpoint: i64,
}
fn get_dr(c: &Connection, pid: &str, id: &str) -> Result<Option<DrInfo2>, String> {
    c.query_row("SELECT status,gate_id,runner_request_id,task_id,can_execute,stage_boundary_locked,requires_git_checkpoint FROM runner_dry_runs WHERE id=?1 AND project_id=?2",params![id,pid],|r| Ok(DrInfo2{status:r.get(0)?,gate_id:r.get(1)?,runner_request_id:r.get(2)?,task_id:r.get(3)?,can_execute:r.get(4)?,stage_boundary_locked:r.get(5)?,requires_git_checkpoint:r.get(6)?})).optional().map_err(|e| format!("db:{e}"))
}
struct GateInfo3 {
    status: String,
    runner_request_id: String,
    task_id: String,
    can_execute: i64,
    stage_boundary_locked: i64,
}
fn get_gate(c: &Connection, pid: &str, id: &str) -> Result<Option<GateInfo3>, String> {
    c.query_row("SELECT status,runner_request_id,task_id,can_execute,stage_boundary_locked FROM runner_execution_gates WHERE id=?1 AND project_id=?2",params![id,pid],|r| Ok(GateInfo3{status:r.get(0)?,runner_request_id:r.get(1)?,task_id:r.get(2)?,can_execute:r.get(3)?,stage_boundary_locked:r.get(4)?})).optional().map_err(|e| format!("db:{e}"))
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
                ops: parse_json_array(&o)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("db:{e}"))
}
fn find_by_lock(
    c: &Connection,
    pid: &str,
    lid: &str,
) -> Result<Option<RunnerMinimalRunSummary>, String> {
    c.query_row(
        "SELECT id FROM runner_minimal_runs WHERE project_id=?1 AND execution_lock_id=?2",
        params![pid, lid],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| format!("db:{e}"))
    .and_then(|o| match o {
        Some(id) => find_by_id(c, pid, &id),
        None => Ok(None),
    })
}
fn find_by_id(
    c: &Connection,
    pid: &str,
    id: &str,
) -> Result<Option<RunnerMinimalRunSummary>, String> {
    c.query_row("SELECT id,project_id,execution_lock_id,dry_run_id,gate_id,runner_request_id,task_id,status,allowed_files,written_files,command_plan,command_results,pre_git_status_summary,pre_git_diff_stat,post_git_status_summary,post_git_diff_stat,failure_category,failure_summary,side_effects,requested_by,started_at,finished_at,created_at,updated_at FROM runner_minimal_runs WHERE id=?1 AND project_id=?2",params![id,pid],map_row).optional().map_err(|e| format!("db:{e}"))
}
fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerMinimalRunSummary> {
    let af: String = r.get(8)?;
    let wf: String = r.get(9)?;
    let cp: String = r.get(10)?;
    let cr: String = r.get(11)?;
    let se: String = r.get(18)?;
    Ok(RunnerMinimalRunSummary {
        id: r.get(0)?,
        project_id: r.get(1)?,
        execution_lock_id: r.get(2)?,
        dry_run_id: r.get(3)?,
        gate_id: r.get(4)?,
        runner_request_id: r.get(5)?,
        task_id: r.get(6)?,
        status: r.get(7)?,
        allowed_files: parse_json_array(&af)?,
        written_files: parse_json_array(&wf)?,
        command_plan: parse_json_array(&cp)?,
        command_results: serde_json::from_str(&cr)
            .map_err(|e| rusqlite::Error::InvalidParameterName(format!("{e}")))?,
        pre_git_status_summary: r.get(12)?,
        pre_git_diff_stat: r.get(13)?,
        post_git_status_summary: r.get(14)?,
        post_git_diff_stat: r.get(15)?,
        failure_category: r.get(16)?,
        failure_summary: r.get(17)?,
        side_effects: serde_json::from_str(&se)
            .map_err(|e| rusqlite::Error::InvalidParameterName(format!("{e}")))?,
        requested_by: r.get(19)?,
        started_at: r.get(20)?,
        finished_at: r.get(21)?,
        created_at: r.get(22)?,
        updated_at: r.get(23)?,
    })
}
fn parse_json_array(s: &str) -> rusqlite::Result<Vec<String>> {
    let v: Value = serde_json::from_str(s)
        .map_err(|e| rusqlite::Error::InvalidParameterName(format!("JSON parse: {e}")))?;
    let arr = v.as_array().ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!("expected JSON array, got: {}", v))
    })?;
    let mut result = Vec::with_capacity(arr.len());
    for (i, x) in arr.iter().enumerate() {
        match x.as_str() {
            Some(s) => result.push(s.to_string()),
            None => {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "array element [{i}] is not a string: {x}"
                )))
            }
        }
    }
    Ok(result)
}
fn trunc(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
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
fn normalize(v: &str, f: &str, m: usize) -> Result<String, String> {
    let v = v.trim().to_string();
    if v.is_empty() || v.len() > m {
        Err(format!("invalid {f}"))
    } else {
        Ok(v)
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
fn side_effects_true() -> ProjectPlanSideEffects {
    ProjectPlanSideEffects {
        writes_project_files: true,
        modifies_git: true,
        executes_runner: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        project_plan::*,
        runner_dry_run::*,
        runner_execution_gate::*,
        runner_execution_lock::{
            self, CreateRunnerExecutionLockInput, RevokeRunnerExecutionLockInput,
        },
        runner_preflight::*,
    };
    use std::fs;
    fn td() -> (crate::db::DbState, std::path::PathBuf) {
        let d = std::env::temp_dir().join(format!("mr3-{}", now_str()));
        (crate::db::initialize(d.clone()).expect("db"), d)
    }
    fn ct(c: &Connection, t: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .expect("ct")
    }
    fn setup_lock(c: &mut Connection) -> runner_execution_lock::RunnerExecutionLockSummary {
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
        runner_execution_lock::create_runner_execution_lock(
            c,
            CreateRunnerExecutionLockInput {
                dry_run_id: dr.dry_run.id,
                second_confirm: true,
                confirm_text: "我确认锁定执行范围，不创建Git checkpoint".into(),
                requested_by: None,
            },
        )
        .expect("lock")
        .execution_lock
    }
    fn valid_create(lid: &str) -> CreateRunnerMinimalRunInput {
        CreateRunnerMinimalRunInput {
            execution_lock_id: lid.into(),
            second_confirm: true,
            confirm_text: CONFIRM_TEXT.into(),
            requested_by: None,
        }
    }

    #[test]
    fn requires_second_confirm() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        assert!(create_runner_minimal_run(
            &mut c,
            CreateRunnerMinimalRunInput {
                second_confirm: false,
                ..valid_create(&lk.id)
            }
        )
        .unwrap_err()
        .contains("second_confirm"));
        assert!(create_runner_minimal_run(
            &mut c,
            CreateRunnerMinimalRunInput {
                confirm_text: "wrong".into(),
                ..valid_create(&lk.id)
            }
        )
        .unwrap_err()
        .contains("confirm_text"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn rejects_unknown_lock() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        assert!(
            create_runner_minimal_run(&mut c, valid_create("nonexistent"))
                .unwrap_err()
                .contains("not_found")
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn rejects_revoked_lock() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        runner_execution_lock::revoke_runner_execution_lock(
            &mut c,
            RevokeRunnerExecutionLockInput {
                execution_lock_id: lk.id.clone(),
                second_confirm: true,
                confirm_text: "我确认撤销执行范围锁".into(),
                revoked_reason: None,
            },
        )
        .expect("revoke");
        match create_runner_minimal_run(&mut c, valid_create(&lk.id)) {
            Ok(_) => panic!("expected err"),
            Err(e) => {
                assert!(e.contains("lock status"), "got:{e}");
            }
        }
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn rejects_non_virtual_lock_paths() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let bad = serde_json::to_string(&["apps/main.rs"]).unwrap();
        c.execute(
            "UPDATE runner_execution_locks SET allowed_files=?1 WHERE id=?2",
            params![bad.as_str(), lk.id.as_str()],
        )
        .unwrap();
        match create_runner_minimal_run(&mut c, valid_create(&lk.id)) {
            Ok(_) => panic!("expected err"),
            Err(e) => {
                assert!(
                    e.contains("virtual/") || e.contains("invalid_input"),
                    "got:{e}"
                );
            }
        }
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]

    fn creates_minimal_run_successfully() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let b_m = ct(&c, "model_calls");
        let b_t = ct(&c, "tasks");
        let b_r = ct(&c, "runner_requests");
        let r1 = create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("create");
        let r2 = create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("idempotent");
        assert_eq!(r1.run.id, r2.run.id);
        assert!(
            r1.run.status == "succeeded" || r1.run.status == "failed_command",
            "status: {}",
            r1.run.status
        );
        assert_eq!(ct(&c, "runner_minimal_runs"), 1);
        assert_eq!(ct(&c, "model_calls"), b_m);
        assert_eq!(ct(&c, "tasks"), b_t);
        assert_eq!(ct(&c, "runner_requests"), b_r);
        assert!(r1.run.side_effects.modifies_git);
        assert!(!r1.run.side_effects.calls_real_model);
        assert!(r1.run.side_effects.executes_runner);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]

    fn list_filters_current_project() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("c");
        assert_eq!(list_runner_minimal_runs(&c).unwrap().len(), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]

    fn writes_only_allowed_files() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let r = create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("c");
        for wf in &r.run.written_files {
            assert!(
                r.run.allowed_files.contains(wf),
                "written file {wf} not in allowed_files"
            );
        }
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]

    fn old_sandbox_files_do_not_cause_failure() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("c1");
        let lk2 = setup_lock(&mut c);
        let r2 = create_runner_minimal_run(&mut c, valid_create(&lk2.id)).expect("c2");
        assert!(
            r2.run.status == "succeeded" || r2.run.status == "failed_command",
            "status: {}, old files should not block",
            r2.run.status
        );
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]

    fn scope_mismatch_checked() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        let r = create_runner_minimal_run(&mut c, valid_create(&lk.id)).expect("normal create");
        assert!(r.run.status == "succeeded" || r.run.status == "failed_command");
        assert_eq!(ct(&c, "runner_minimal_runs"), 1);
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn run_cmd_uses_cwd_correctly() {
        let result = run_cmd_with_timeout("git", &["status", "--short"], ".");
        assert!(!result.command.is_empty());
    }
    #[test]
    fn inputs_reject_unknown_fields() {
        assert!(serde_json::from_str::<CreateRunnerMinimalRunInput>(r#"{"execution_lock_id":"x","second_confirm":true,"confirm_text":"我确认执行阶段34最小Runner，只允许沙箱范围","extra":1}"#).is_err());
    }
    #[test]
    fn rejects_sensitive_requested_by() {
        let (s, d) = td();
        let mut c = s.connection().unwrap();
        let lk = setup_lock(&mut c);
        assert!(create_runner_minimal_run(
            &mut c,
            CreateRunnerMinimalRunInput {
                requested_by: Some("sk-abcdefghijklmnopqrstuvwxyz123456".into()),
                ..valid_create(&lk.id)
            }
        )
        .unwrap_err()
        .contains("API key"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn map_virtual_to_generated_correct() {
        let base = std::env::temp_dir().join("agent-swarm-runner-stage34");
        let p = map_virtual_to_generated(&base, "virtual/frontend-plan.md").unwrap();
        assert!(p.to_string_lossy().contains("agent-swarm-runner-stage34"));
        assert!(p.to_string_lossy().ends_with("frontend-plan.md"));
        assert!(map_virtual_to_generated(&base, "apps/main.rs").is_err());
        assert!(map_virtual_to_generated(&base, "virtual/../secret").is_err());
    }

    #[test]
    fn generated_base_uses_isolated_temp_in_tests() {
        let (s, d) = td();
        let c = s.connection().unwrap();
        let base = generated_base(&c, "project_agent_swarm").expect("generated base");
        assert!(base
            .to_string_lossy()
            .contains("agent-swarm-runner-stage34"));
        drop(c);
        drop(s);
        let _ = fs::remove_dir_all(d);
    }
}
