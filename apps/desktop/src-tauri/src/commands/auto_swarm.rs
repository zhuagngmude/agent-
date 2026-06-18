use crate::{
    db::DbState,
    services::auto_swarm::{
        auto_run_swarm_idea as run_auto_swarm, continue_swarm_tasks as continue_tasks,
        AutoRunSwarmIdeaInput, AutoRunSwarmIdeaResponse, ContinueSwarmTasksInput,
        ContinueSwarmTasksResponse,
    },
};

#[tauri::command]
pub fn auto_run_swarm_idea(
    state: tauri::State<'_, DbState>,
    input: AutoRunSwarmIdeaInput,
) -> Result<AutoRunSwarmIdeaResponse, String> {
    let mut connection = state.connection()?;
    run_auto_swarm(&mut connection, input)
}

#[tauri::command]
pub fn continue_swarm_tasks(
    state: tauri::State<'_, DbState>,
    input: ContinueSwarmTasksInput,
) -> Result<ContinueSwarmTasksResponse, String> {
    let mut connection = state.connection()?;
    continue_tasks(&mut connection, input)
}
