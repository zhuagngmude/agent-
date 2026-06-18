use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{
    project_plan::{
        auto_generate_project_plan_tasks, ApproveProjectPlanResponse,
        AutoGenerateProjectPlanTasksInput,
    },
    runner_dry_run::{auto_create_runner_dry_run, RunnerDryRunSummary},
    runner_execution_gate::{auto_create_runner_execution_gate, RunnerExecutionGateSummary},
    runner_execution_lock::{auto_create_runner_execution_lock, RunnerExecutionLockSummary},
    runner_minimal_run::{auto_create_runner_minimal_run, RunnerMinimalRunSummary},
    runner_preflight::{auto_create_runner_preflight_review, RunnerPreflightReviewSummary},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoRunSwarmIdeaInput {
    pub idea: String,
    #[serde(default)]
    pub constraints: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutoRunSwarmTaskResult {
    pub runner_request_id: String,
    pub preflight_review: Option<RunnerPreflightReviewSummary>,
    pub execution_gate: Option<RunnerExecutionGateSummary>,
    pub dry_run: Option<RunnerDryRunSummary>,
    pub execution_lock: Option<RunnerExecutionLockSummary>,
    pub minimal_run: Option<RunnerMinimalRunSummary>,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AutoRunSwarmIdeaResponse {
    pub plan: ApproveProjectPlanResponse,
    pub task_results: Vec<AutoRunSwarmTaskResult>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinueSwarmTasksInput {
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContinueSwarmTasksResponse {
    pub task_results: Vec<AutoRunSwarmTaskResult>,
    pub skipped_task_ids: Vec<String>,
    pub status: String,
}

pub fn auto_run_swarm_idea(
    connection: &mut Connection,
    input: AutoRunSwarmIdeaInput,
) -> Result<AutoRunSwarmIdeaResponse, String> {
    let requested_by = input
        .requested_by
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "swarm_auto".to_string());

    let plan = auto_generate_project_plan_tasks(
        connection,
        AutoGenerateProjectPlanTasksInput {
            idea: input.idea,
            constraints: input.constraints,
            requested_by: Some(requested_by.clone()),
        },
    )?;

    let mut task_results = Vec::with_capacity(plan.created_runner_request_ids.len());
    for runner_request_id in &plan.created_runner_request_ids {
        task_results.push(run_one_runner_request(
            connection,
            runner_request_id,
            &requested_by,
        ));
    }

    let status = if task_results.iter().all(|item| item.status == "succeeded") {
        "succeeded"
    } else if task_results.iter().any(|item| item.status == "succeeded") {
        "partial"
    } else {
        "failed"
    }
    .to_string();

    Ok(AutoRunSwarmIdeaResponse {
        plan,
        task_results,
        status,
    })
}

pub fn continue_swarm_tasks(
    connection: &mut Connection,
    input: ContinueSwarmTasksInput,
) -> Result<ContinueSwarmTasksResponse, String> {
    let project_id = current_project_id(connection)?;
    let requested_by = input
        .requested_by
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "swarm_continue".to_string());

    let mut task_ids = Vec::new();
    let mut seen = HashSet::new();
    for task_id in input.task_ids {
        let normalized = normalize_id(task_id, "task_id")?;
        if seen.insert(normalized.clone()) {
            task_ids.push(normalized);
        }
    }
    if task_ids.is_empty() {
        return Err("invalid_input: task_ids must not be empty".to_string());
    }

    let mut skipped_task_ids = Vec::new();
    let mut task_results = Vec::new();
    for task_id in task_ids {
        let Some(task_status) = task_status(connection, &project_id, &task_id)? else {
            return Err(format!("not_found: task '{}' not found", task_id));
        };
        if task_status == "completed" || task_status == "cancelled" {
            skipped_task_ids.push(task_id);
            continue;
        }

        let Some(runner_request_id) =
            runner_request_id_for_task(connection, &project_id, &task_id)?
        else {
            skipped_task_ids.push(task_id);
            continue;
        };

        reset_failed_minimal_runs(connection, &project_id, &task_id)?;
        task_results.push(run_one_runner_request(
            connection,
            &runner_request_id,
            &requested_by,
        ));
    }

    let status = if task_results.is_empty() {
        "skipped"
    } else if task_results.iter().all(|item| item.status == "succeeded") {
        "succeeded"
    } else if task_results.iter().any(|item| item.status == "succeeded") {
        "partial"
    } else {
        "failed"
    }
    .to_string();

    Ok(ContinueSwarmTasksResponse {
        task_results,
        skipped_task_ids,
        status,
    })
}

fn run_one_runner_request(
    connection: &mut Connection,
    runner_request_id: &str,
    requested_by: &str,
) -> AutoRunSwarmTaskResult {
    let mut result = AutoRunSwarmTaskResult {
        runner_request_id: runner_request_id.to_string(),
        preflight_review: None,
        execution_gate: None,
        dry_run: None,
        execution_lock: None,
        minimal_run: None,
        status: "failed".to_string(),
        message: None,
    };

    let preflight = match auto_create_runner_preflight_review(
        connection,
        runner_request_id.to_string(),
        Some(requested_by.to_string()),
    ) {
        Ok(response) => response.review,
        Err(error) => return result.with_error(error),
    };
    if let Err(error) = auto_approve_preflight(connection, &preflight.approval_id) {
        result.preflight_review = Some(preflight);
        return result.with_error(error);
    }
    result.preflight_review = Some(preflight);

    let preflight_id = result
        .preflight_review
        .as_ref()
        .map(|review| review.id.clone())
        .unwrap_or_default();
    let gate = match auto_create_runner_execution_gate(
        connection,
        preflight_id,
        Some(requested_by.to_string()),
    ) {
        Ok(response) => response.gate,
        Err(error) => return result.with_error(error),
    };
    result.execution_gate = Some(gate);

    let gate_id = result
        .execution_gate
        .as_ref()
        .map(|gate| gate.id.clone())
        .unwrap_or_default();
    let dry_run =
        match auto_create_runner_dry_run(connection, gate_id, Some(requested_by.to_string())) {
            Ok(response) => response.dry_run,
            Err(error) => return result.with_error(error),
        };
    result.dry_run = Some(dry_run);

    let dry_run_id = result
        .dry_run
        .as_ref()
        .map(|dry_run| dry_run.id.clone())
        .unwrap_or_default();
    let execution_lock = match auto_create_runner_execution_lock(
        connection,
        dry_run_id,
        Some(requested_by.to_string()),
    ) {
        Ok(response) => response.execution_lock,
        Err(error) => return result.with_error(error),
    };
    result.execution_lock = Some(execution_lock);

    let execution_lock_id = result
        .execution_lock
        .as_ref()
        .map(|lock| lock.id.clone())
        .unwrap_or_default();
    let minimal_run = match auto_create_runner_minimal_run(
        connection,
        execution_lock_id,
        Some(requested_by.to_string()),
    ) {
        Ok(response) => response.run,
        Err(error) => return result.with_error(error),
    };

    let status = minimal_run.status.clone();
    update_task_after_minimal_run(connection, &minimal_run);
    result.minimal_run = Some(minimal_run);
    result.status = if status == "succeeded" || status == "created" || status == "running" {
        "succeeded".to_string()
    } else {
        status
    };
    result
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
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

fn normalize_id(value: String, field: &str) -> Result<String, String> {
    let normalized = value.trim().to_string();
    let length = normalized.chars().count();
    if length == 0 || length > 200 {
        return Err(format!(
            "invalid_input: {field} length must be between 1 and 200"
        ));
    }
    Ok(normalized)
}

fn task_status(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT status FROM tasks WHERE project_id = ?1 AND id = ?2",
            params![project_id, task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: read task status failed: {error}"))
}

fn runner_request_id_for_task(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT id FROM runner_requests
             WHERE project_id = ?1 AND task_id = ?2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            params![project_id, task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: read runner request failed: {error}"))
}

fn reset_failed_minimal_runs(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<(), String> {
    connection
        .execute(
            "DELETE FROM runner_minimal_runs
             WHERE project_id = ?1
               AND task_id = ?2
               AND status IN ('failed', 'failed_scope_violation', 'aborted')",
            params![project_id, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("database_error: reset failed minimal runs failed: {error}"))
}

fn update_task_after_minimal_run(connection: &Connection, minimal_run: &RunnerMinimalRunSummary) {
    let next_status = match minimal_run.status.as_str() {
        "succeeded" => "completed",
        "failed" | "failed_scope_violation" | "aborted" => "failed",
        _ => return,
    };
    let now = now_str();
    if let Err(error) = connection.execute(
        "UPDATE tasks
         SET status = ?1, updated_at = ?2
         WHERE project_id = ?3 AND id = ?4",
        params![
            next_status,
            now.as_str(),
            minimal_run.project_id.as_str(),
            minimal_run.task_id.as_str()
        ],
    ) {
        eprintln!(
            "agent-swarm: failed to sync task {} after minimal run {}: {}",
            minimal_run.task_id, minimal_run.id, error
        );
    }
}

fn auto_approve_preflight(connection: &mut Connection, approval_id: &str) -> Result<(), String> {
    let now = now_str();
    let current_status = connection
        .query_row(
            "SELECT status FROM approvals WHERE id = ?1",
            params![approval_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("database_error: read preflight approval failed: {error}"))?
        .ok_or_else(|| "not_found: preflight approval not found".to_string())?;

    match current_status.as_str() {
        "approved" => Ok(()),
        "pending" => {
            connection
                .execute(
                    "UPDATE approvals
                     SET status = 'approved', approved_at = ?1, updated_at = ?1
                     WHERE id = ?2 AND status = 'pending'",
                    params![now.as_str(), approval_id],
                )
                .map_err(|error| {
                    format!("database_error: auto approve preflight failed: {error}")
                })?;
            Ok(())
        }
        other => Err(format!(
            "invalid_transition: preflight approval status cannot change from {other}"
        )),
    }
}

fn now_str() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

impl AutoRunSwarmTaskResult {
    fn with_error(mut self, error: String) -> Self {
        self.status = "failed".to_string();
        self.message = Some(error);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_db() -> (crate::db::DbState, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("auto-swarm-{}", now_str()));
        let state = crate::db::initialize(dir.clone()).expect("sqlite should initialize");
        (state, dir)
    }

    fn count_rows(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable")
    }

    #[test]
    fn auto_run_swarm_idea_advances_generated_requests_to_minimal_runs() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let response = auto_run_swarm_idea(
                &mut connection,
                AutoRunSwarmIdeaInput {
                    idea: "build a small timer app".to_string(),
                    constraints: Some("auto mode test".to_string()),
                    requested_by: None,
                },
            )
            .expect("auto swarm should run");

            assert_eq!(response.plan.created_runner_request_ids.len(), 2);
            assert_eq!(response.task_results.len(), 2);
            assert_eq!(count_rows(&connection, "runner_preflight_reviews"), 2);
            assert_eq!(count_rows(&connection, "runner_execution_gates"), 2);
            assert_eq!(count_rows(&connection, "runner_dry_runs"), 2);
            assert_eq!(count_rows(&connection, "runner_execution_locks"), 2);
            assert_eq!(count_rows(&connection, "runner_minimal_runs"), 2);
            assert!(response
                .task_results
                .iter()
                .all(|item| item.preflight_review.is_some()));
            assert!(response
                .task_results
                .iter()
                .all(|item| item.minimal_run.is_some()));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }
}
