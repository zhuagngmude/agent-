use crate::db::DbState;
use crate::services::runner_execution_gate::{
    create_runner_execution_gate as create_gate, list_runner_execution_gates as list_gates,
    revoke_runner_execution_gate as revoke_gate, CreateRunnerExecutionGateInput,
    CreateRunnerExecutionGateResponse, RevokeRunnerExecutionGateInput,
    RevokeRunnerExecutionGateResponse, RunnerExecutionGateSummary,
};

#[tauri::command]
pub fn create_runner_execution_gate(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerExecutionGateInput,
) -> Result<CreateRunnerExecutionGateResponse, String> {
    let mut connection = state.connection()?;
    create_gate(&mut connection, input)
}

#[tauri::command]
pub fn list_runner_execution_gates(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerExecutionGateSummary>, String> {
    let connection = state.connection()?;
    list_gates(&connection)
}

#[tauri::command]
pub fn revoke_runner_execution_gate(
    state: tauri::State<'_, DbState>,
    input: RevokeRunnerExecutionGateInput,
) -> Result<RevokeRunnerExecutionGateResponse, String> {
    let mut connection = state.connection()?;
    revoke_gate(&mut connection, input)
}
