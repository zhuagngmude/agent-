use crate::{
    db::DbState,
    services::projects::{get_current_project, ProjectSummary},
};

#[tauri::command]
pub fn get_project(state: tauri::State<'_, DbState>) -> Result<ProjectSummary, String> {
    let connection = state.connection()?;
    get_current_project(&connection)
}
