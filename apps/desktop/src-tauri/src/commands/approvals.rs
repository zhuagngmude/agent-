use crate::{
    db::DbState,
    services::approvals::{
        create_approval as create_approval_record, list_approvals as list_approval_records,
        ApprovalSummary, CreateApprovalInput, CreateApprovalResponse,
    },
};

#[tauri::command]
pub fn list_approvals(state: tauri::State<'_, DbState>) -> Result<Vec<ApprovalSummary>, String> {
    let connection = state.connection()?;
    list_approval_records(&connection)
}

#[tauri::command]
pub fn create_approval(
    state: tauri::State<'_, DbState>,
    input: CreateApprovalInput,
) -> Result<CreateApprovalResponse, String> {
    let mut connection = state.connection()?;
    create_approval_record(&mut connection, input)
}
