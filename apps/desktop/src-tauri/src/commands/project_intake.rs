use serde::Deserialize;

use crate::db::DbState;
use crate::services::project_intake::{self, ClassifyProjectIntakeResponse, ProjectIntakeSession};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassifyProjectIntakeInput {
    pub idea: String,
}

#[tauri::command]
pub fn classify_project_intake(
    state: tauri::State<'_, DbState>,
    input: ClassifyProjectIntakeInput,
) -> Result<ClassifyProjectIntakeResponse, String> {
    let connection = state.connection()?;
    project_intake::classify_project_intake(&connection, &input.idea)
}

#[tauri::command]
pub fn list_project_intakes(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ProjectIntakeSession>, String> {
    let connection = state.connection()?;
    project_intake::list_project_intakes(&connection)
}
