use crate::db::DbState;
use crate::services::runner_preflight::{
    create_runner_preflight_review as create_review, list_runner_preflight_reviews as list_reviews,
    CreateRunnerPreflightReviewInput, CreateRunnerPreflightReviewResponse,
    RunnerPreflightReviewSummary,
};

#[tauri::command]
pub fn create_runner_preflight_review(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerPreflightReviewInput,
) -> Result<CreateRunnerPreflightReviewResponse, String> {
    let mut connection = state.connection()?;
    create_review(&mut connection, input)
}

#[tauri::command]
pub fn list_runner_preflight_reviews(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerPreflightReviewSummary>, String> {
    let connection = state.connection()?;
    list_reviews(&connection)
}
