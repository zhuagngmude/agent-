use serde::Deserialize;

use crate::services::model_gateway::{
    create_project_plan_draft, ProjectPlanModelDraftResponse,
};

/// 阶段 22：只接受 idea / constraints / second_confirm / confirm_text 四个字段。
/// `#[serde(deny_unknown_fields)]` 确保前端无法传入 apiKey、baseUrl、provider、model 等字段。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectPlanModelDraftInput {
    pub idea: String,
    #[serde(default)]
    pub constraints: Option<String>,
    #[serde(default)]
    pub second_confirm: bool,
    #[serde(default)]
    pub confirm_text: Option<String>,
}

#[tauri::command]
pub fn request_project_plan_model_draft(
    input: ProjectPlanModelDraftInput,
) -> Result<ProjectPlanModelDraftResponse, String> {
    create_project_plan_draft(
        &input.idea,
        &input.constraints,
        input.second_confirm,
        &input.confirm_text,
    )
}
