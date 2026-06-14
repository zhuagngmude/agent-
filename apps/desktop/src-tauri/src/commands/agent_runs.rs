use crate::{
    db::DbState,
    services::agent_runs::{
        list_agent_runs as list_agent_run_records, list_runtime_events as list_event_records,
        AgentRunSummary, RuntimeEventSummary,
    },
};

#[tauri::command]
pub fn list_agent_runs(state: tauri::State<'_, DbState>) -> Result<Vec<AgentRunSummary>, String> {
    let connection = state.connection()?;
    list_agent_run_records(&connection)
}

#[tauri::command]
pub fn list_runtime_events(
    state: tauri::State<'_, DbState>,
    entity_id: Option<String>,
) -> Result<Vec<RuntimeEventSummary>, String> {
    let connection = state.connection()?;
    list_event_records(&connection, entity_id.as_deref())
}
