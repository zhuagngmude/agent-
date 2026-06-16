use crate::db::DbState;
use crate::services::runner_execution_lock::{
    create_runner_execution_lock as create_lk, list_runner_execution_locks as list_lk,
    revoke_runner_execution_lock as revoke_lk, CreateRunnerExecutionLockInput,
    CreateRunnerExecutionLockResponse, RevokeRunnerExecutionLockInput,
    RevokeRunnerExecutionLockResponse, RunnerExecutionLockSummary,
};
#[tauri::command]
pub fn create_runner_execution_lock(
    s: tauri::State<'_, DbState>,
    i: CreateRunnerExecutionLockInput,
) -> Result<CreateRunnerExecutionLockResponse, String> {
    let mut c = s.connection()?;
    create_lk(&mut c, i)
}
#[tauri::command]
pub fn list_runner_execution_locks(
    s: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerExecutionLockSummary>, String> {
    let c = s.connection()?;
    list_lk(&c)
}
#[tauri::command]
pub fn revoke_runner_execution_lock(
    s: tauri::State<'_, DbState>,
    i: RevokeRunnerExecutionLockInput,
) -> Result<RevokeRunnerExecutionLockResponse, String> {
    let mut c = s.connection()?;
    revoke_lk(&mut c, i)
}
