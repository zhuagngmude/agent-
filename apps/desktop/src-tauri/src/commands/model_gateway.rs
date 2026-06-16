use serde::Deserialize;

use crate::db::DbState;
use crate::services::model_gateway::{
    create_project_plan_draft_core as create_project_plan_draft_service,
    ProjectPlanModelDraftResponse,
};

/// 阶段 22：只接受 idea / constraints / second_confirm / confirm_text 四个字段。
/// 阶段 35：新增可选 model_record_id，来自后端受控模型目录。
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
    /// 阶段 35：可选，来自 model_catalog 目录的 id
    #[serde(default)]
    pub model_record_id: Option<String>,
}

#[tauri::command]
pub fn request_project_plan_model_draft(
    state: tauri::State<'_, DbState>,
    input: ProjectPlanModelDraftInput,
) -> Result<ProjectPlanModelDraftResponse, String> {
    let flag = std::env::var("AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN")
        .unwrap_or_else(|_| "false".into());
    let connection = state.connection()?;

    create_project_plan_draft_service(
        &input.idea,
        &input.constraints,
        input.second_confirm,
        &input.confirm_text,
        &flag,
        None, // config_key: read from env
        None, // config_base_url: read from env
        None, // provider_override: use real provider
        Some(&connection),
        input.model_record_id.as_deref(),
    )
}
