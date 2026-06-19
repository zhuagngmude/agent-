use crate::{
    db::DbState,
    services::tasks::{
        assign_project_agents_to_task as assign_project_agents_to_task_record,
        create_task as create_task_record, delete_tasks as delete_task_records,
        get_task_agent_info as get_task_agent_info_record, list_tasks as list_task_records,
        open_task_output_folder as open_task_output_folder_record,
        update_task_status as update_task_status_record, AssignProjectAgentsToTaskInput,
        AssignProjectAgentsToTaskResponse, CreateTaskInput, CreateTaskResponse, DeleteTasksInput,
        DeleteTasksResponse, OpenTaskOutputFolderInput, OpenTaskOutputFolderResponse,
        TaskAgentInfo, TaskSummary, UpdateTaskStatusInput, UpdateTaskStatusResponse,
    },
};

#[tauri::command]
pub fn get_task_agent_info(
    state: tauri::State<'_, DbState>,
    task_id: String,
) -> Result<Option<TaskAgentInfo>, String> {
    let connection = state.connection()?;
    get_task_agent_info_record(&connection, &task_id)
}

#[tauri::command]
pub fn assign_project_agents_to_task(
    state: tauri::State<'_, DbState>,
    input: AssignProjectAgentsToTaskInput,
) -> Result<AssignProjectAgentsToTaskResponse, String> {
    let mut connection = state.connection()?;
    assign_project_agents_to_task_record(&mut connection, input)
}

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

#[tauri::command]
pub fn update_task_status(
    state: tauri::State<'_, DbState>,
    input: UpdateTaskStatusInput,
) -> Result<UpdateTaskStatusResponse, String> {
    let mut connection = state.connection()?;
    update_task_status_record(&mut connection, input)
}

#[tauri::command]
pub fn delete_tasks(
    state: tauri::State<'_, DbState>,
    input: DeleteTasksInput,
) -> Result<DeleteTasksResponse, String> {
    let mut connection = state.connection()?;
    delete_task_records(&mut connection, input)
}

#[tauri::command]
pub fn open_task_output_folder(
    state: tauri::State<'_, DbState>,
    input: OpenTaskOutputFolderInput,
) -> Result<OpenTaskOutputFolderResponse, String> {
    let connection = state.connection()?;
    open_task_output_folder_record(&connection, input)
}
