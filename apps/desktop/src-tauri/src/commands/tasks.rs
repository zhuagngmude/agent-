use crate::{
    db::DbState,
    services::tasks::{
        create_task as create_task_record, list_tasks as list_task_records, CreateTaskInput,
        CreateTaskResponse, TaskSummary,
    },
};

#[tauri::command]
pub fn list_tasks(state: tauri::State<'_, DbState>) -> Result<Vec<TaskSummary>, String> {
    let connection = state.connection()?;
    list_task_records(&connection)
}

#[tauri::command]
pub fn create_task(
    state: tauri::State<'_, DbState>,
    input: CreateTaskInput,
) -> Result<CreateTaskResponse, String> {
    let mut connection = state.connection()?;
    create_task_record(&mut connection, input)
}
