use crate::{
    db::DbState,
    services::project_plan::{
        approve_project_plan as approve_project_plan_record,
        create_project_plan_draft as create_project_plan_draft_record,
        list_project_plan_drafts as list_project_plan_draft_records,
        list_runner_requests as list_runner_request_records, ApproveProjectPlanInput,
        ApproveProjectPlanResponse, CreateProjectPlanDraftInput, CreateProjectPlanDraftResponse,
        ProjectPlanDraftSummary, RunnerRequestSummary,
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
pub fn list_project_plan_drafts(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ProjectPlanDraftSummary>, String> {
    let connection = state.connection()?;
    list_project_plan_draft_records(&connection)
}

#[tauri::command]
pub fn list_runner_requests(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerRequestSummary>, String> {
    let connection = state.connection()?;
    list_runner_request_records(&connection)
}
