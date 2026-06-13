use crate::services::projects::{get_current_project, ProjectSummary};

#[tauri::command]
pub fn get_project() -> ProjectSummary {
    get_current_project()
}
