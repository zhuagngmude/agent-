use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

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
    result.minimal_run = Some(minimal_run);
    result.status = if status == "succeeded" || status == "created" || status == "running" {
        "succeeded".to_string()
    } else {
        status
    };
    result
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

            assert_eq!(response.plan.created_runner_request_ids.len(), 5);
            assert_eq!(response.task_results.len(), 5);
            assert_eq!(count_rows(&connection, "runner_preflight_reviews"), 5);
            assert_eq!(count_rows(&connection, "runner_execution_gates"), 5);
            assert_eq!(count_rows(&connection, "runner_dry_runs"), 5);
            assert_eq!(count_rows(&connection, "runner_execution_locks"), 5);
            assert_eq!(count_rows(&connection, "runner_minimal_runs"), 5);
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
