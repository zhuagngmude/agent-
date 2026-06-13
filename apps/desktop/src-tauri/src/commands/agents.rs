use crate::{
    db::DbState,
    services::agents::{list_agents as list_agent_records, AgentSummary},
};

#[tauri::command]
pub fn list_agents(state: tauri::State<'_, DbState>) -> Result<Vec<AgentSummary>, String> {
    let connection = state.connection()?;
    list_agent_records(&connection)
}
