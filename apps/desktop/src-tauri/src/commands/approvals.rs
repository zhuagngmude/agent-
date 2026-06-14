use crate::{
    db::DbState,
    services::approvals::{
        approve_approval as approve_approval_record, create_approval as create_approval_record,
        list_approvals as list_approval_records, patch_only_approval as patch_only_approval_record,
        reject_approval as reject_approval_record, ApprovalIdInput, ApprovalSummary,
        ApprovalTransitionResponse, CreateApprovalInput, CreateApprovalResponse,
        RejectApprovalInput,
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

#[tauri::command]
pub fn approve_approval(
    state: tauri::State<'_, DbState>,
    input: ApprovalIdInput,
) -> Result<ApprovalTransitionResponse, String> {
    let mut connection = state.connection()?;
    approve_approval_record(&mut connection, input)
}

#[tauri::command]
pub fn reject_approval(
    state: tauri::State<'_, DbState>,
    input: RejectApprovalInput,
) -> Result<ApprovalTransitionResponse, String> {
    let mut connection = state.connection()?;
    reject_approval_record(&mut connection, input)
}

#[tauri::command]
pub fn patch_only_approval(
    state: tauri::State<'_, DbState>,
    input: ApprovalIdInput,
) -> Result<ApprovalTransitionResponse, String> {
    let mut connection = state.connection()?;
    patch_only_approval_record(&mut connection, input)
}
