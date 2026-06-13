use crate::{
    db::DbState,
    services::approvals::{list_approvals as list_approval_records, ApprovalSummary},
};

#[tauri::command]
pub fn list_approvals(state: tauri::State<'_, DbState>) -> Result<Vec<ApprovalSummary>, String> {
    let connection = state.connection()?;
    list_approval_records(&connection)
}
