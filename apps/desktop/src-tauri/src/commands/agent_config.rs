use crate::{
    db::DbState,
    services::agent_config::{
        delete_agent_template as delete_agent_template_record,
        delete_executor_config as delete_executor_config_record,
        delete_executor_model as delete_executor_model_record,
        delete_executor_skill as delete_executor_skill_record,
        list_agent_boundary_checks as list_agent_boundary_check_records,
        list_agent_templates as list_agent_template_records,
        list_executor_configs as list_executor_config_records,
        list_executor_models as list_executor_model_records,
        list_executor_skills as list_executor_skill_records,
        list_project_agents as list_project_agent_records,
        remove_project_agent as remove_project_agent_record,
        upsert_agent_template as upsert_agent_template_record,
        upsert_executor_config as upsert_executor_config_record,
        upsert_executor_model as upsert_executor_model_record,
        upsert_executor_skill as upsert_executor_skill_record,
        upsert_project_agent as upsert_project_agent_record, AgentBoundaryCheckSummary,
        AgentTemplateSummary, DeleteAgentTemplateInput, DeleteExecutorConfigInput,
        DeleteExecutorModelInput, DeleteExecutorSkillInput, ExecutorConfigSummary,
        ExecutorModelSummary, ExecutorSkillSummary, ListBoundaryChecksInput,
        ListExecutorModelsInput, ProjectAgentSummary, RemoveProjectAgentInput,
        UpsertAgentTemplateInput, UpsertExecutorConfigInput, UpsertExecutorModelInput,
        UpsertExecutorSkillInput, UpsertProjectAgentInput,
    },
};

#[tauri::command]
pub fn list_executor_configs(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ExecutorConfigSummary>, String> {
    let connection = state.connection()?;
    list_executor_config_records(&connection)
}

#[tauri::command]
pub fn upsert_executor_config(
    state: tauri::State<'_, DbState>,
    input: UpsertExecutorConfigInput,
) -> Result<ExecutorConfigSummary, String> {
    let connection = state.connection()?;
    upsert_executor_config_record(&connection, input)
}

#[tauri::command]
pub fn delete_executor_config(
    state: tauri::State<'_, DbState>,
    input: DeleteExecutorConfigInput,
) -> Result<(), String> {
    let connection = state.connection()?;
    delete_executor_config_record(&connection, input)
}

#[tauri::command]
pub fn list_executor_models(
    state: tauri::State<'_, DbState>,
    input: ListExecutorModelsInput,
) -> Result<Vec<ExecutorModelSummary>, String> {
    let connection = state.connection()?;
    list_executor_model_records(&connection, input)
}

#[tauri::command]
pub fn upsert_executor_model(
    state: tauri::State<'_, DbState>,
    input: UpsertExecutorModelInput,
) -> Result<ExecutorModelSummary, String> {
    let connection = state.connection()?;
    upsert_executor_model_record(&connection, input)
}

#[tauri::command]
pub fn delete_executor_model(
    state: tauri::State<'_, DbState>,
    input: DeleteExecutorModelInput,
) -> Result<(), String> {
    let connection = state.connection()?;
    delete_executor_model_record(&connection, input)
}

#[tauri::command]
pub fn list_agent_templates(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<AgentTemplateSummary>, String> {
    let connection = state.connection()?;
    list_agent_template_records(&connection)
}

#[tauri::command]
pub fn upsert_agent_template(
    state: tauri::State<'_, DbState>,
    input: UpsertAgentTemplateInput,
) -> Result<AgentTemplateSummary, String> {
    let connection = state.connection()?;
    upsert_agent_template_record(&connection, input)
}

#[tauri::command]
pub fn delete_agent_template(
    state: tauri::State<'_, DbState>,
    input: DeleteAgentTemplateInput,
) -> Result<(), String> {
    let connection = state.connection()?;
    delete_agent_template_record(&connection, input)
}

#[tauri::command]
pub fn list_project_agents(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ProjectAgentSummary>, String> {
    let connection = state.connection()?;
    list_project_agent_records(&connection)
}

#[tauri::command]
pub fn upsert_project_agent(
    state: tauri::State<'_, DbState>,
    input: UpsertProjectAgentInput,
) -> Result<ProjectAgentSummary, String> {
    let connection = state.connection()?;
    upsert_project_agent_record(&connection, input)
}

#[tauri::command]
pub fn remove_project_agent(
    state: tauri::State<'_, DbState>,
    input: RemoveProjectAgentInput,
) -> Result<(), String> {
    let connection = state.connection()?;
    remove_project_agent_record(&connection, input)
}

#[tauri::command]
pub fn list_executor_skills(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<ExecutorSkillSummary>, String> {
    let connection = state.connection()?;
    list_executor_skill_records(&connection)
}

#[tauri::command]
pub fn upsert_executor_skill(
    state: tauri::State<'_, DbState>,
    input: UpsertExecutorSkillInput,
) -> Result<ExecutorSkillSummary, String> {
    let connection = state.connection()?;
    upsert_executor_skill_record(&connection, input)
}

#[tauri::command]
pub fn delete_executor_skill(
    state: tauri::State<'_, DbState>,
    input: DeleteExecutorSkillInput,
) -> Result<(), String> {
    let connection = state.connection()?;
    delete_executor_skill_record(&connection, input)
}

#[tauri::command]
pub fn list_agent_boundary_checks(
    state: tauri::State<'_, DbState>,
    input: ListBoundaryChecksInput,
) -> Result<Vec<AgentBoundaryCheckSummary>, String> {
    let connection = state.connection()?;
    list_agent_boundary_check_records(&connection, input)
}
