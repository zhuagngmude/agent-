use crate::{
    db::DbState,
    services::project_plan::{
        approve_project_plan as approve_project_plan_record,
        auto_generate_project_plan_tasks as auto_generate_tasks_record,
        create_project_plan_draft as create_project_plan_draft_record,
        delete_project_plan_draft as delete_project_plan_draft_record,
        get_project_plan_execution_preview as get_preview,
        list_project_plan_drafts as list_project_plan_draft_records,
        list_project_plan_task_templates as list_templates,
        list_runner_requests as list_runner_request_records,
        save_project_plan_model_draft as save_project_plan_model_draft_record,
        update_project_plan_task_template as update_template, ApproveProjectPlanInput,
        ApproveProjectPlanResponse, AutoGenerateProjectPlanTasksInput, CreateProjectPlanDraftInput,
        CreateProjectPlanDraftResponse, DeleteProjectPlanDraftInput,
        DeleteProjectPlanDraftResponse, ProjectPlanDraftSummary, ProjectPlanExecutionPreview,
        ProjectPlanTaskTemplateSummary, RunnerRequestSummary, SaveProjectPlanModelDraftInput,
        UpdateProjectPlanTaskTemplateInput,
    },
};

#[tauri::command]
pub fn create_project_plan_draft(
    state: tauri::State<'_, DbState>,
    input: CreateProjectPlanDraftInput,
) -> Result<CreateProjectPlanDraftResponse, String> {
    let mut connection = state.connection()?;
    create_project_plan_draft_record(&mut connection, input)
}

#[tauri::command]
pub fn approve_project_plan(
    state: tauri::State<'_, DbState>,
    input: ApproveProjectPlanInput,
) -> Result<ApproveProjectPlanResponse, String> {
    let mut connection = state.connection()?;
    approve_project_plan_record(&mut connection, input)
}

#[tauri::command]
pub fn auto_generate_project_plan_tasks(
    state: tauri::State<'_, DbState>,
    input: AutoGenerateProjectPlanTasksInput,
) -> Result<ApproveProjectPlanResponse, String> {
    let mut connection = state.connection()?;
    auto_generate_tasks_record(&mut connection, input)
}

#[tauri::command]
pub fn list_project_plan_drafts(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ProjectPlanDraftSummary>, String> {
    let connection = state.connection()?;
    list_project_plan_draft_records(&connection)
}

#[tauri::command]
pub fn delete_project_plan_draft(
    state: tauri::State<'_, DbState>,
    input: DeleteProjectPlanDraftInput,
) -> Result<DeleteProjectPlanDraftResponse, String> {
    let mut connection = state.connection()?;
    delete_project_plan_draft_record(&mut connection, input)
}

#[tauri::command]
pub fn list_runner_requests(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerRequestSummary>, String> {
    let connection = state.connection()?;
    list_runner_request_records(&connection)
}

#[tauri::command]
pub fn save_project_plan_model_draft(
    state: tauri::State<'_, DbState>,
    input: SaveProjectPlanModelDraftInput,
) -> Result<CreateProjectPlanDraftResponse, String> {
    let mut connection = state.connection()?;
    save_project_plan_model_draft_record(&mut connection, input)
}

#[tauri::command]
pub fn list_project_plan_task_templates(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ProjectPlanTaskTemplateSummary>, String> {
    let connection = state.connection()?;
    list_templates(&connection)
}

#[tauri::command]
pub fn update_project_plan_task_template(
    state: tauri::State<'_, DbState>,
    input: UpdateProjectPlanTaskTemplateInput,
) -> Result<Vec<ProjectPlanTaskTemplateSummary>, String> {
    let mut connection = state.connection()?;
    update_template(&mut connection, input)
}

#[tauri::command]
pub fn get_project_plan_execution_preview(
    state: tauri::State<'_, DbState>,
    #[allow(non_snake_case)] approval_id: String,
) -> Result<ProjectPlanExecutionPreview, String> {
    let connection = state.connection()?;
    get_preview(&connection, approval_id)
}
