use serde::Deserialize;

use crate::db::DbState;
use crate::services::model_catalog::{self, ModelCatalogEntry, UpdateModelEnabledInput};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateModelEnabledCmdInput {
    pub model_record_id: String,
    pub enabled: bool,
    pub second_confirm: bool,
    pub confirm_text: String,
}

#[tauri::command]
pub fn list_project_plan_models(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let connection = state.connection()?;
    model_catalog::list_project_plan_models(&connection)
}

#[tauri::command]
pub fn update_project_plan_model_enabled(
    state: tauri::State<'_, DbState>,
    input: UpdateModelEnabledCmdInput,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let mut connection = state.connection()?;
    model_catalog::update_model_enabled(
        &mut connection,
        UpdateModelEnabledInput {
            model_record_id: input.model_record_id,
            enabled: input.enabled,
            second_confirm: input.second_confirm,
            confirm_text: input.confirm_text,
        },
    )
}
