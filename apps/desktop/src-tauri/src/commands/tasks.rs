use crate::{
    db::DbState,
    services::tasks::{list_tasks as list_task_records, TaskSummary},
};

#[tauri::command]
pub fn list_tasks(state: tauri::State<'_, DbState>) -> Result<Vec<TaskSummary>, String> {
    let connection = state.connection()?;
    list_task_records(&connection)
}
