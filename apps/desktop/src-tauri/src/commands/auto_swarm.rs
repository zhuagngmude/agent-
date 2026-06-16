use crate::{
    db::DbState,
    services::auto_swarm::{
        auto_run_swarm_idea as run_auto_swarm, AutoRunSwarmIdeaInput, AutoRunSwarmIdeaResponse,
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
