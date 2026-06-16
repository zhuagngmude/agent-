use crate::db::DbState;
use crate::services::runner_dry_run::{
    create_runner_dry_run as create_dr, list_runner_dry_runs as list_dr,
    revoke_runner_dry_run as revoke_dr, CreateRunnerDryRunInput, CreateRunnerDryRunResponse,
    RevokeRunnerDryRunInput, RevokeRunnerDryRunResponse, RunnerDryRunSummary,
};

#[tauri::command]
pub fn create_runner_dry_run(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerDryRunInput,
) -> Result<CreateRunnerDryRunResponse, String> {
    let mut c = state.connection()?;
    create_dr(&mut c, input)
}

#[tauri::command]
pub fn list_runner_dry_runs(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerDryRunSummary>, String> {
    let c = state.connection()?;
    list_dr(&c)
}

#[tauri::command]
pub fn revoke_runner_dry_run(
    state: tauri::State<'_, DbState>,
    input: RevokeRunnerDryRunInput,
) -> Result<RevokeRunnerDryRunResponse, String> {
    let mut c = state.connection()?;
    revoke_dr(&mut c, input)
}
