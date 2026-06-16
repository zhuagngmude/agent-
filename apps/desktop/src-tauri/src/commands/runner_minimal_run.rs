use crate::db::DbState;
use crate::services::runner_minimal_run::{
    create_runner_minimal_run as create_mr, list_runner_minimal_runs as list_mr,
    CreateRunnerMinimalRunInput, CreateRunnerMinimalRunResponse, RunnerMinimalRunSummary,
};
#[tauri::command]
pub fn create_runner_minimal_run(
    s: tauri::State<'_, DbState>,
    i: CreateRunnerMinimalRunInput,
) -> Result<CreateRunnerMinimalRunResponse, String> {
    let mut c = s.connection()?;
    create_mr(&mut c, i)
}
#[tauri::command]
pub fn list_runner_minimal_runs(
    s: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerMinimalRunSummary>, String> {
    let c = s.connection()?;
    list_mr(&c)
}
