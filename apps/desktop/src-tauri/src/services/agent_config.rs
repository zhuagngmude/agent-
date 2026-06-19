use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use super::model_catalog::validate_model_id;
use super::projects::get_current_project;

#[derive(Debug, Serialize, Clone)]
pub struct ExecutorConfigSummary {
    pub id: String,
    pub key: String,
    pub label: String,
    pub kind: String,
    pub provider: Option<String>,
    pub base_url_status: String,
    pub executable_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExecutorModelSummary {
    pub id: String,
    pub project_id: String,
    pub executor_key: String,
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub purpose: String,
    pub enabled: bool,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentTemplateSummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub category: String,
    pub specialty: Option<String>,
    pub stack: Option<String>,
    pub module_scope: String,
    pub allowed_task_types: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub default_executor_key: String,
    pub default_model_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectAgentSummary {
    pub id: String,
    pub project_id: String,
    pub agent_template_id: String,
    pub name: String,
    pub role: String,
    pub source: String,
    pub executor_key: String,
    pub model_id: Option<String>,
    pub module_scope: String,
    pub status: String,
    pub joined_at: String,
    pub removed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExecutorSkillSummary {
    pub id: String,
    pub executor_key: String,
    pub agent_template_id: Option<String>,
    pub skill_name: String,
    pub skill_scope: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentBoundaryCheckSummary {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub agent_id: String,
    pub requested_action: String,
    pub task_type: Option<String>,
    pub module_scope: String,
    pub target_path: Option<String>,
    pub decision: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertExecutorConfigInput {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub provider: Option<String>,
    pub base_url_status: Option<String>,
    pub executable_path: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteExecutorConfigInput {
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListExecutorModelsInput {
    pub project_id: Option<String>,
    pub executor_key: Option<String>,
    pub purpose: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertExecutorModelInput {
    pub project_id: Option<String>,
    pub executor_key: String,
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub purpose: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteExecutorModelInput {
    pub model_record_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertAgentTemplateInput {
    pub role: String,
    pub category: String,
    pub name: String,
    pub specialty: Option<String>,
    pub stack: Option<String>,
    pub module_scope: String,
    pub allowed_task_types: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub default_executor_key: String,
    pub default_model_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteAgentTemplateInput {
    pub template_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertProjectAgentInput {
    pub project_id: Option<String>,
    pub agent_template_id: String,
    pub name: String,
    pub role: String,
    pub source: String,
    pub executor_key: String,
    pub model_id: Option<String>,
    pub module_scope: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveProjectAgentInput {
    pub project_agent_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpsertExecutorSkillInput {
    pub executor_key: String,
    pub agent_template_id: Option<String>,
    pub skill_name: String,
    pub skill_scope: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteExecutorSkillInput {
    pub skill_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListBoundaryChecksInput {
    pub project_id: Option<String>,
    pub limit: Option<i64>,
}

pub fn list_executor_configs(
    connection: &Connection,
) -> Result<Vec<ExecutorConfigSummary>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, key, label, kind, provider, base_url_status, executable_path,
                status, created_at, updated_at
             FROM executor_configs
             ORDER BY key",
        )
        .map_err(|error| format!("database_error: list executor configs failed: {error}"))?;

    let rows = stmt
        .query_map([], map_executor_config_row)
        .map_err(|error| format!("database_error: list executor configs failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list executor configs failed: {error}"))
}

pub fn upsert_executor_config(
    connection: &Connection,
    input: UpsertExecutorConfigInput,
) -> Result<ExecutorConfigSummary, String> {
    let key = validate_key(&input.key, "key")?;
    let label = validate_text(&input.label, "label", 120)?;
    let kind = normalize_enum(
        &input.kind,
        &["model_gateway", "external_executor", "local_tool"],
        "kind",
    )?;
    let provider = normalize_optional_text(input.provider, "provider", 120)?;
    let base_url_status = normalize_optional_text(input.base_url_status, "base_url_status", 120)?;
    let executable_path = normalize_optional_text(input.executable_path, "executable_path", 260)?;
    let status = normalize_enum(&input.status, &["active", "disabled", "error"], "status")?;

    if let Some(value) = provider.as_deref() {
        reject_secret_like(value, "provider")?;
    }
    if let Some(value) = executable_path.as_deref() {
        reject_secret_like(value, "executable_path")?;
    }

    let now = current_timestamp();
    let existing_id: Option<String> = connection
        .query_row(
            "SELECT id FROM executor_configs WHERE key = ?1",
            params![key.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: lookup executor config failed: {error}"))?;
    let id = existing_id.unwrap_or_else(|| new_id("executor_config"));

    connection
        .execute(
            "INSERT INTO executor_configs (
                id, key, label, kind, provider, base_url_status,
                executable_path, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(key) DO UPDATE SET
                label = excluded.label,
                kind = excluded.kind,
                provider = excluded.provider,
                base_url_status = excluded.base_url_status,
                executable_path = excluded.executable_path,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                id.as_str(),
                key.as_str(),
                label.as_str(),
                kind.as_str(),
                provider.as_deref(),
                base_url_status.as_deref().unwrap_or("not_configured"),
                executable_path.as_deref(),
                status.as_str(),
                now.as_str(),
                now.as_str(),
            ],
        )
        .map_err(|error| format!("database_error: upsert executor config failed: {error}"))?;

    get_executor_config_by_key(connection, &key)
}

pub fn delete_executor_config(
    connection: &Connection,
    input: DeleteExecutorConfigInput,
) -> Result<(), String> {
    let key = validate_key(&input.key, "key")?;
    if key == "model_gateway_default" {
        return Err("invalid_input: builtin executor config cannot be deleted".into());
    }

    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM executor_configs WHERE key = ?1",
            params![key.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup executor config failed: {error}"))?;
    if exists == 0 {
        return Err(format!("not_found: executor config '{key}' not found"));
    }

    let template_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agent_templates WHERE default_executor_key = ?1",
            params![key.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;
    let agent_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE executor_key = ?1 AND removed_at IS NULL",
            params![key.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;
    let skill_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM executor_skills WHERE executor_key = ?1",
            params![key.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;
    let model_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM model_catalog WHERE executor_key = ?1",
            params![key.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;

    if template_refs + agent_refs + skill_refs + model_refs > 0 {
        return Err("invalid_state: executor config still has dependent records".into());
    }

    connection
        .execute(
            "DELETE FROM executor_configs WHERE key = ?1",
            params![key.as_str()],
        )
        .map_err(|error| format!("database_error: delete executor config failed: {error}"))?;
    Ok(())
}

pub fn list_executor_models(
    connection: &Connection,
    input: ListExecutorModelsInput,
) -> Result<Vec<ExecutorModelSummary>, String> {
    let project_id = match input.project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            validate_key(&project_id, "project_id")?
        }
        _ => current_project_id(connection)?,
    };
    let executor_key = match input.executor_key {
        Some(key) if !key.trim().is_empty() => Some(validate_key(&key, "executor_key")?),
        _ => None,
    };
    let purpose = match input.purpose {
        Some(purpose) if !purpose.trim().is_empty() => Some(validate_key(&purpose, "purpose")?),
        _ => None,
    };

    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, COALESCE(executor_key, 'model_gateway_default'),
                provider, model_id, display_name, purpose, enabled, is_builtin,
                created_at, updated_at
             FROM model_catalog
             WHERE project_id = ?1
               AND (?2 IS NULL OR executor_key = ?2)
               AND (?3 IS NULL OR purpose = ?3)
             ORDER BY purpose, is_builtin DESC, enabled DESC, model_id",
        )
        .map_err(|error| format!("database_error: list executor models failed: {error}"))?;
    let rows = stmt
        .query_map(
            params![
                project_id.as_str(),
                executor_key.as_deref(),
                purpose.as_deref()
            ],
            map_executor_model_row,
        )
        .map_err(|error| format!("database_error: list executor models failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list executor models failed: {error}"))
}

pub fn upsert_executor_model(
    connection: &Connection,
    input: UpsertExecutorModelInput,
) -> Result<ExecutorModelSummary, String> {
    let project_id = match input.project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            validate_key(&project_id, "project_id")?
        }
        _ => current_project_id(connection)?,
    };
    let executor_key = validate_key(&input.executor_key, "executor_key")?;
    let provider = normalize_enum(&input.provider, &["openai_compat"], "provider")?;
    let model_id = validate_text(&input.model_id, "model_id", 120)?;
    validate_model_id(&model_id)?;
    let display_name = validate_text(&input.display_name, "display_name", 120)?;
    let purpose = validate_key(&input.purpose, "purpose")?;
    ensure_executor_exists(connection, &executor_key)?;

    let now = current_timestamp();
    let existing_id: Option<String> = connection
        .query_row(
            "SELECT id FROM model_catalog
             WHERE project_id = ?1 AND executor_key = ?2 AND provider = ?3
               AND model_id = ?4 AND purpose = ?5",
            params![
                project_id.as_str(),
                executor_key.as_str(),
                provider.as_str(),
                model_id.as_str(),
                purpose.as_str()
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: lookup executor model failed: {error}"))?;
    let id = existing_id.unwrap_or_else(|| new_id("model_catalog"));

    connection
        .execute(
            "INSERT INTO model_catalog (
                id, project_id, executor_key, provider, model_id, display_name,
                purpose, enabled, is_builtin, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10)
            ON CONFLICT(project_id, executor_key, provider, model_id, purpose) DO UPDATE SET
                executor_key = excluded.executor_key,
                display_name = excluded.display_name,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
            params![
                id.as_str(),
                project_id.as_str(),
                executor_key.as_str(),
                provider.as_str(),
                model_id.as_str(),
                display_name.as_str(),
                purpose.as_str(),
                input.enabled as i64,
                now.as_str(),
                now.as_str(),
            ],
        )
        .map_err(|error| format!("database_error: upsert executor model failed: {error}"))?;

    get_executor_model_by_identity(
        connection,
        &project_id,
        &executor_key,
        &provider,
        &model_id,
        &purpose,
    )
}

pub fn delete_executor_model(
    connection: &Connection,
    input: DeleteExecutorModelInput,
) -> Result<(), String> {
    let model_record_id = validate_text(&input.model_record_id, "model_record_id", 200)?;
    let model = get_executor_model_by_id(connection, &model_record_id)?;
    if model.is_builtin {
        return Err("invalid_input: builtin model cannot be deleted".into());
    }

    let agent_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents
             WHERE project_id = ?1 AND executor_key = ?2 AND model_id = ?3
               AND removed_at IS NULL",
            params![
                model.project_id.as_str(),
                model.executor_key.as_str(),
                model.model_id.as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;
    if agent_refs > 0 {
        return Err("invalid_state: model is still assigned to project agents".into());
    }

    connection
        .execute(
            "DELETE FROM model_catalog WHERE id = ?1",
            params![model_record_id.as_str()],
        )
        .map_err(|error| format!("database_error: delete executor model failed: {error}"))?;
    Ok(())
}

pub fn list_agent_templates(connection: &Connection) -> Result<Vec<AgentTemplateSummary>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, name, role, category, specialty, stack, module_scope,
                allowed_task_types, allowed_paths, forbidden_actions,
                default_executor_key, default_model_id, enabled, created_at, updated_at
             FROM agent_templates
             ORDER BY category, role, name",
        )
        .map_err(|error| format!("database_error: list agent templates failed: {error}"))?;

    let rows = stmt
        .query_map([], map_agent_template_row)
        .map_err(|error| format!("database_error: list agent templates failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list agent templates failed: {error}"))
}

pub fn upsert_agent_template(
    connection: &Connection,
    input: UpsertAgentTemplateInput,
) -> Result<AgentTemplateSummary, String> {
    let role = validate_key(&input.role, "role")?;
    let category = normalize_enum(&input.category, &["core", "expert"], "category")?;
    let name = validate_text(&input.name, "name", 80)?;
    let module_scope = validate_key(&input.module_scope, "module_scope")?;
    let specialty = normalize_optional_text(input.specialty, "specialty", 200)?;
    let stack = normalize_optional_text(input.stack, "stack", 120)?;
    let default_executor_key = validate_key(&input.default_executor_key, "default_executor_key")?;
    let default_model_id = match input.default_model_id {
        Some(model_id) if !model_id.trim().is_empty() => {
            let value = validate_text(&model_id, "default_model_id", 120)?;
            validate_model_id(&value)?;
            Some(value)
        }
        _ => None,
    };
    let allowed_task_types = normalize_string_list(input.allowed_task_types, "allowed_task_types")?;
    let allowed_paths = normalize_string_list(input.allowed_paths, "allowed_paths")?;
    let forbidden_actions = normalize_string_list(input.forbidden_actions, "forbidden_actions")?;
    let enabled = input.enabled as i64;
    let now = current_timestamp();

    ensure_executor_exists(connection, &default_executor_key)?;

    let existing_id: Option<String> = connection
        .query_row(
            "SELECT id FROM agent_templates WHERE role = ?1 AND category = ?2",
            params![role.as_str(), category.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: lookup agent template failed: {error}"))?;
    let id = existing_id.unwrap_or_else(|| new_id("agent_template"));

    connection
        .execute(
            "INSERT INTO agent_templates (
                id, name, role, category, specialty, stack, module_scope,
                allowed_task_types, allowed_paths, forbidden_actions,
                default_executor_key, default_model_id, enabled, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(role, category) DO UPDATE SET
                name = excluded.name,
                specialty = excluded.specialty,
                stack = excluded.stack,
                module_scope = excluded.module_scope,
                allowed_task_types = excluded.allowed_task_types,
                allowed_paths = excluded.allowed_paths,
                forbidden_actions = excluded.forbidden_actions,
                default_executor_key = excluded.default_executor_key,
                default_model_id = excluded.default_model_id,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
            params![
                id.as_str(),
                name.as_str(),
                role.as_str(),
                category.as_str(),
                specialty.as_deref(),
                stack.as_deref(),
                module_scope.as_str(),
                json_list(&allowed_task_types)?,
                json_list(&allowed_paths)?,
                json_list(&forbidden_actions)?,
                default_executor_key.as_str(),
                default_model_id.as_deref(),
                enabled,
                now.as_str(),
                now.as_str(),
            ],
        )
        .map_err(|error| format!("database_error: upsert agent template failed: {error}"))?;

    get_agent_template_by_role_category(connection, &role, &category)
}

pub fn delete_agent_template(
    connection: &Connection,
    input: DeleteAgentTemplateInput,
) -> Result<(), String> {
    let template_id = validate_key(&input.template_id, "template_id")?;
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agent_templates WHERE id = ?1",
            params![template_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup agent template failed: {error}"))?;
    if exists == 0 {
        return Err(format!(
            "not_found: agent template '{template_id}' not found"
        ));
    }

    let project_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE agent_template_id = ?1 AND removed_at IS NULL",
            params![template_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;
    let skill_refs: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM executor_skills WHERE agent_template_id = ?1",
            params![template_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: dependency check failed: {error}"))?;

    if project_refs + skill_refs > 0 {
        return Err("invalid_state: agent template still has dependent records".into());
    }

    connection
        .execute(
            "DELETE FROM agent_templates WHERE id = ?1",
            params![template_id.as_str()],
        )
        .map_err(|error| format!("database_error: delete agent template failed: {error}"))?;
    Ok(())
}

pub fn list_project_agents(connection: &Connection) -> Result<Vec<ProjectAgentSummary>, String> {
    let project_id = current_project_id(connection)?;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, agent_template_id, name, role, source,
                executor_key, model_id, module_scope, status, joined_at,
                removed_at, created_at, updated_at
             FROM project_agents
             WHERE project_id = ?1
             ORDER BY removed_at IS NOT NULL, joined_at DESC, created_at DESC",
        )
        .map_err(|error| format!("database_error: list project agents failed: {error}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str()], map_project_agent_row)
        .map_err(|error| format!("database_error: list project agents failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list project agents failed: {error}"))
}

pub fn upsert_project_agent(
    connection: &Connection,
    input: UpsertProjectAgentInput,
) -> Result<ProjectAgentSummary, String> {
    let project_id = match input.project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            validate_key(&project_id, "project_id")?
        }
        _ => current_project_id(connection)?,
    };
    let agent_template_id = validate_key(&input.agent_template_id, "agent_template_id")?;
    let name = validate_text(&input.name, "name", 80)?;
    let role = validate_key(&input.role, "role")?;
    let source = normalize_enum(&input.source, &["core", "recommended", "manual"], "source")?;
    let executor_key = validate_key(&input.executor_key, "executor_key")?;
    let module_scope = validate_key(&input.module_scope, "module_scope")?;
    let status = normalize_enum(
        &input.status,
        &["active", "idle", "disabled", "removed"],
        "status",
    )?;
    let model_id = match input.model_id {
        Some(model_id) if !model_id.trim().is_empty() => {
            let value = validate_text(&model_id, "model_id", 120)?;
            validate_model_id(&value)?;
            ensure_enabled_model_exists(connection, &project_id, &executor_key, &value)?;
            Some(value)
        }
        _ => None,
    };
    let now = current_timestamp();

    ensure_executor_exists(connection, &executor_key)?;
    ensure_template_exists(connection, &agent_template_id)?;

    let existing_id: Option<String> = connection
        .query_row(
            "SELECT id FROM project_agents
             WHERE project_id = ?1 AND agent_template_id = ?2 AND removed_at IS NULL
             LIMIT 1",
            params![project_id.as_str(), agent_template_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: lookup project agent failed: {error}"))?;

    if let Some(existing_id) = existing_id {
        connection
            .execute(
                "UPDATE project_agents
                 SET name = ?1, role = ?2, source = ?3, executor_key = ?4,
                     model_id = ?5, module_scope = ?6, status = ?7,
                     removed_at = NULL, updated_at = ?8
                 WHERE id = ?9",
                params![
                    name.as_str(),
                    role.as_str(),
                    source.as_str(),
                    executor_key.as_str(),
                    model_id.as_deref(),
                    module_scope.as_str(),
                    status.as_str(),
                    now.as_str(),
                    existing_id.as_str(),
                ],
            )
            .map_err(|error| format!("database_error: update project agent failed: {error}"))?;
    } else {
        let id = new_id("project_agent");
        connection
            .execute(
                "INSERT INTO project_agents (
                    id, project_id, agent_template_id, name, role, source,
                    executor_key, model_id, module_scope, status,
                    joined_at, removed_at, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?11, ?11)",
                params![
                    id.as_str(),
                    project_id.as_str(),
                    agent_template_id.as_str(),
                    name.as_str(),
                    role.as_str(),
                    source.as_str(),
                    executor_key.as_str(),
                    model_id.as_deref(),
                    module_scope.as_str(),
                    status.as_str(),
                    now.as_str(),
                ],
            )
            .map_err(|error| format!("database_error: insert project agent failed: {error}"))?;
    }

    get_project_agent_by_template(connection, &project_id, &agent_template_id)
}

pub fn remove_project_agent(
    connection: &Connection,
    input: RemoveProjectAgentInput,
) -> Result<(), String> {
    let project_agent_id = validate_key(&input.project_agent_id, "project_agent_id")?;
    let project_id = current_project_id(connection)?;
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE id = ?1 AND project_id = ?2",
            params![project_agent_id.as_str(), project_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup project agent failed: {error}"))?;
    if exists == 0 {
        return Err(format!(
            "not_found: project agent '{project_agent_id}' not found"
        ));
    }

    let now = current_timestamp();
    connection
        .execute(
            "UPDATE project_agents
             SET status = 'removed', removed_at = ?1, updated_at = ?1
             WHERE id = ?2 AND project_id = ?3",
            params![now.as_str(), project_agent_id.as_str(), project_id.as_str()],
        )
        .map_err(|error| format!("database_error: remove project agent failed: {error}"))?;
    Ok(())
}

pub fn list_executor_skills(connection: &Connection) -> Result<Vec<ExecutorSkillSummary>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, executor_key, agent_template_id, skill_name, skill_scope,
                enabled, created_at, updated_at
             FROM executor_skills
             ORDER BY executor_key, skill_name",
        )
        .map_err(|error| format!("database_error: list executor skills failed: {error}"))?;

    let rows = stmt
        .query_map([], map_executor_skill_row)
        .map_err(|error| format!("database_error: list executor skills failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list executor skills failed: {error}"))
}

pub fn upsert_executor_skill(
    connection: &Connection,
    input: UpsertExecutorSkillInput,
) -> Result<ExecutorSkillSummary, String> {
    let executor_key = validate_key(&input.executor_key, "executor_key")?;
    let agent_template_id = match input.agent_template_id {
        Some(value) if !value.trim().is_empty() => Some(validate_key(&value, "agent_template_id")?),
        _ => None,
    };
    let skill_name = validate_key(&input.skill_name, "skill_name")?;
    let skill_scope = validate_key(&input.skill_scope, "skill_scope")?;
    ensure_executor_exists(connection, &executor_key)?;
    if let Some(ref template_id) = agent_template_id {
        ensure_template_exists(connection, template_id)?;
    }

    let now = current_timestamp();
    let existing_id: Option<String> = connection
        .query_row(
            "SELECT id FROM executor_skills
             WHERE executor_key = ?1
               AND COALESCE(agent_template_id, '') = COALESCE(?2, '')
               AND skill_name = ?3
             LIMIT 1",
            params![
                executor_key.as_str(),
                agent_template_id.as_deref(),
                skill_name.as_str()
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: lookup executor skill failed: {error}"))?;
    let id = existing_id.unwrap_or_else(|| new_id("executor_skill"));

    connection
        .execute(
            "INSERT INTO executor_skills (
                id, executor_key, agent_template_id, skill_name, skill_scope,
                enabled, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                executor_key = excluded.executor_key,
                agent_template_id = excluded.agent_template_id,
                skill_name = excluded.skill_name,
                skill_scope = excluded.skill_scope,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
            params![
                id.as_str(),
                executor_key.as_str(),
                agent_template_id.as_deref(),
                skill_name.as_str(),
                skill_scope.as_str(),
                input.enabled as i64,
                now.as_str(),
                now.as_str(),
            ],
        )
        .map_err(|error| format!("database_error: upsert executor skill failed: {error}"))?;

    get_executor_skill_by_identity(
        connection,
        &executor_key,
        agent_template_id.as_deref(),
        &skill_name,
    )
}

pub fn delete_executor_skill(
    connection: &Connection,
    input: DeleteExecutorSkillInput,
) -> Result<(), String> {
    let skill_id = validate_key(&input.skill_id, "skill_id")?;
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM executor_skills WHERE id = ?1",
            params![skill_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup executor skill failed: {error}"))?;
    if exists == 0 {
        return Err(format!("not_found: executor skill '{skill_id}' not found"));
    }
    connection
        .execute(
            "DELETE FROM executor_skills WHERE id = ?1",
            params![skill_id.as_str()],
        )
        .map_err(|error| format!("database_error: delete executor skill failed: {error}"))?;
    Ok(())
}

pub fn list_agent_boundary_checks(
    connection: &Connection,
    input: ListBoundaryChecksInput,
) -> Result<Vec<AgentBoundaryCheckSummary>, String> {
    let project_id = match input.project_id {
        Some(project_id) if !project_id.trim().is_empty() => {
            validate_key(&project_id, "project_id")?
        }
        _ => current_project_id(connection)?,
    };
    let limit = input.limit.unwrap_or(100).clamp(1, 500);
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, task_id, agent_id, requested_action,
                task_type, module_scope, target_path, decision, reason, created_at
             FROM agent_boundary_checks
             WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )
        .map_err(|error| format!("database_error: list boundary checks failed: {error}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str(), limit], map_boundary_check_row)
        .map_err(|error| format!("database_error: list boundary checks failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: list boundary checks failed: {error}"))
}

fn get_executor_config_by_key(
    connection: &Connection,
    key: &str,
) -> Result<ExecutorConfigSummary, String> {
    connection
        .query_row(
            "SELECT id, key, label, kind, provider, base_url_status, executable_path,
                status, created_at, updated_at
             FROM executor_configs WHERE key = ?1",
            params![key],
            map_executor_config_row,
        )
        .map_err(|error| format!("database_error: load executor config failed: {error}"))
}

fn get_agent_template_by_role_category(
    connection: &Connection,
    role: &str,
    category: &str,
) -> Result<AgentTemplateSummary, String> {
    connection
        .query_row(
            "SELECT id, name, role, category, specialty, stack, module_scope,
                allowed_task_types, allowed_paths, forbidden_actions,
                default_executor_key, default_model_id, enabled, created_at, updated_at
             FROM agent_templates WHERE role = ?1 AND category = ?2",
            params![role, category],
            map_agent_template_row,
        )
        .map_err(|error| format!("database_error: load agent template failed: {error}"))
}

fn get_executor_model_by_identity(
    connection: &Connection,
    project_id: &str,
    executor_key: &str,
    provider: &str,
    model_id: &str,
    purpose: &str,
) -> Result<ExecutorModelSummary, String> {
    connection
        .query_row(
            "SELECT id, project_id, COALESCE(executor_key, 'model_gateway_default'),
                provider, model_id, display_name, purpose, enabled, is_builtin,
                created_at, updated_at
             FROM model_catalog
             WHERE project_id = ?1 AND executor_key = ?2 AND provider = ?3
               AND model_id = ?4 AND purpose = ?5",
            params![project_id, executor_key, provider, model_id, purpose],
            map_executor_model_row,
        )
        .map_err(|error| format!("database_error: load executor model failed: {error}"))
}

fn get_executor_model_by_id(
    connection: &Connection,
    model_record_id: &str,
) -> Result<ExecutorModelSummary, String> {
    connection
        .query_row(
            "SELECT id, project_id, COALESCE(executor_key, 'model_gateway_default'),
                provider, model_id, display_name, purpose, enabled, is_builtin,
                created_at, updated_at
             FROM model_catalog
             WHERE id = ?1",
            params![model_record_id],
            map_executor_model_row,
        )
        .map_err(|_| format!("not_found: executor model '{model_record_id}' not found"))
}

fn get_project_agent_by_template(
    connection: &Connection,
    project_id: &str,
    agent_template_id: &str,
) -> Result<ProjectAgentSummary, String> {
    connection
        .query_row(
            "SELECT id, project_id, agent_template_id, name, role, source,
                executor_key, model_id, module_scope, status, joined_at,
                removed_at, created_at, updated_at
             FROM project_agents
             WHERE project_id = ?1 AND agent_template_id = ?2
             ORDER BY removed_at IS NOT NULL, updated_at DESC
             LIMIT 1",
            params![project_id, agent_template_id],
            map_project_agent_row,
        )
        .map_err(|error| format!("database_error: load project agent failed: {error}"))
}

fn get_executor_skill_by_identity(
    connection: &Connection,
    executor_key: &str,
    agent_template_id: Option<&str>,
    skill_name: &str,
) -> Result<ExecutorSkillSummary, String> {
    connection
        .query_row(
            "SELECT id, executor_key, agent_template_id, skill_name, skill_scope,
                enabled, created_at, updated_at
             FROM executor_skills
             WHERE executor_key = ?1
               AND COALESCE(agent_template_id, '') = COALESCE(?2, '')
               AND skill_name = ?3",
            params![executor_key, agent_template_id, skill_name],
            map_executor_skill_row,
        )
        .map_err(|error| format!("database_error: load executor skill failed: {error}"))
}

fn ensure_executor_exists(connection: &Connection, key: &str) -> Result<(), String> {
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM executor_configs WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup executor config failed: {error}"))?;
    if exists == 0 {
        return Err(format!("not_found: executor config '{key}' not found"));
    }
    Ok(())
}

fn ensure_template_exists(connection: &Connection, template_id: &str) -> Result<(), String> {
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agent_templates WHERE id = ?1",
            params![template_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup agent template failed: {error}"))?;
    if exists == 0 {
        return Err(format!(
            "not_found: agent template '{template_id}' not found"
        ));
    }
    Ok(())
}

fn ensure_enabled_model_exists(
    connection: &Connection,
    project_id: &str,
    executor_key: &str,
    model_id: &str,
) -> Result<(), String> {
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM model_catalog
             WHERE project_id = ?1 AND executor_key = ?2 AND model_id = ?3 AND enabled = 1",
            params![project_id, executor_key, model_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: lookup model catalog failed: {error}"))?;
    if exists == 0 {
        return Err(format!(
            "not_found: enabled model '{model_id}' not found in catalog"
        ));
    }
    Ok(())
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
    get_current_project(connection).map(|project| project.id)
}

fn map_executor_config_row(row: &Row<'_>) -> rusqlite::Result<ExecutorConfigSummary> {
    Ok(ExecutorConfigSummary {
        id: row.get(0)?,
        key: row.get(1)?,
        label: row.get(2)?,
        kind: row.get(3)?,
        provider: row.get(4)?,
        base_url_status: row.get(5)?,
        executable_path: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_executor_model_row(row: &Row<'_>) -> rusqlite::Result<ExecutorModelSummary> {
    Ok(ExecutorModelSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        executor_key: row.get(2)?,
        provider: row.get(3)?,
        model_id: row.get(4)?,
        display_name: row.get(5)?,
        purpose: row.get(6)?,
        enabled: row.get::<_, i64>(7)? != 0,
        is_builtin: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_agent_template_row(row: &Row<'_>) -> rusqlite::Result<AgentTemplateSummary> {
    Ok(AgentTemplateSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        role: row.get(2)?,
        category: row.get(3)?,
        specialty: row.get(4)?,
        stack: row.get(5)?,
        module_scope: row.get(6)?,
        allowed_task_types: parse_string_list(&row.get::<_, String>(7)?),
        allowed_paths: parse_string_list(&row.get::<_, String>(8)?),
        forbidden_actions: parse_string_list(&row.get::<_, String>(9)?),
        default_executor_key: row.get(10)?,
        default_model_id: row.get(11)?,
        enabled: row.get::<_, i64>(12)? != 0,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn map_project_agent_row(row: &Row<'_>) -> rusqlite::Result<ProjectAgentSummary> {
    Ok(ProjectAgentSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        agent_template_id: row.get(2)?,
        name: row.get(3)?,
        role: row.get(4)?,
        source: row.get(5)?,
        executor_key: row.get(6)?,
        model_id: row.get(7)?,
        module_scope: row.get(8)?,
        status: row.get(9)?,
        joined_at: row.get(10)?,
        removed_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn map_executor_skill_row(row: &Row<'_>) -> rusqlite::Result<ExecutorSkillSummary> {
    Ok(ExecutorSkillSummary {
        id: row.get(0)?,
        executor_key: row.get(1)?,
        agent_template_id: row.get(2)?,
        skill_name: row.get(3)?,
        skill_scope: row.get(4)?,
        enabled: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn map_boundary_check_row(row: &Row<'_>) -> rusqlite::Result<AgentBoundaryCheckSummary> {
    Ok(AgentBoundaryCheckSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        agent_id: row.get(3)?,
        requested_action: row.get(4)?,
        task_type: row.get(5)?,
        module_scope: row.get(6)?,
        target_path: row.get(7)?,
        decision: row.get(8)?,
        reason: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn json_list(items: &[String]) -> Result<String, String> {
    serde_json::to_string(items)
        .map_err(|error| format!("invalid_input: list serialization failed: {error}"))
}

fn normalize_string_list(items: Vec<String>, field: &str) -> Result<Vec<String>, String> {
    if items.len() > 50 {
        return Err(format!(
            "invalid_input: {field} must contain at most 50 items"
        ));
    }
    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let value = validate_text(&item, field, 200)?;
        reject_secret_like(&value, field)?;
        normalized.push(value);
    }
    Ok(normalized)
}

fn normalize_optional_text(
    value: Option<String>,
    field: &str,
    max_len: usize,
) -> Result<Option<String>, String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(Some(validate_text(&value, field, max_len)?)),
        _ => Ok(None),
    }
}

fn normalize_enum(value: &str, allowed: &[&str], field: &str) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(format!("invalid_input: {field} must not be empty"));
    }
    if allowed.iter().any(|candidate| candidate == &normalized) {
        Ok(normalized.to_string())
    } else {
        Err(format!(
            "invalid_input: {field} must be one of {}",
            allowed.join(", ")
        ))
    }
}

fn validate_key(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("invalid_input: {field} must not be empty"));
    }
    if trimmed.len() > 120 {
        return Err(format!(
            "invalid_input: {field} must be at most 120 characters"
        ));
    }
    for ch in trimmed.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.' {
            return Err(format!(
                "invalid_input: {field} contains forbidden character '{ch}'"
            ));
        }
    }
    reject_secret_like(trimmed, field)?;
    Ok(trimmed.to_string())
}

fn validate_text(value: &str, field: &str, max_len: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("invalid_input: {field} must not be empty"));
    }
    if trimmed.len() > max_len {
        return Err(format!(
            "invalid_input: {field} must be at most {max_len} characters"
        ));
    }
    reject_secret_like(trimmed, field)?;
    Ok(trimmed.to_string())
}

fn reject_secret_like(value: &str, field: &str) -> Result<(), String> {
    let lower = value.to_lowercase();
    let patterns = [
        "api_key=",
        "apikey=",
        "token=",
        "secret=",
        "password=",
        "bearer ",
        "sk-",
    ];
    if patterns.iter().any(|pattern| lower.contains(pattern)) {
        return Err(format!("invalid_input: {field} contains secret-like text"));
    }
    Ok(())
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", current_timestamp())
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

// ---------------------------------------------------------------------------
// Checkpoint 3：总控确定性分派规则
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
pub struct RecommendProjectAgentsOutput {
    pub project_id: String,
    pub recommended_core_agents: Vec<ProjectAgentSummary>,
    pub recommended_expert_agents: Vec<ProjectAgentSummary>,
    pub reason: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendProjectAgentsInput {
    pub project_id: Option<String>,
    pub project_type: String,
    pub tech_stack: Vec<String>,
    pub risk_level: String,
    pub phase: String,
}

/// 确定性规则分派：根据项目类型、技术栈、风险和阶段推荐核心员工和专家
pub fn recommend_project_agents(
    connection: &Connection,
    input: RecommendProjectAgentsInput,
) -> Result<RecommendProjectAgentsOutput, String> {
    let project_id = match input.project_id {
        Some(id) if !id.trim().is_empty() => validate_key(&id, "project_id")?,
        _ => current_project_id(connection)?,
    };
    let project_type = validate_text(&input.project_type, "project_type", 80)?;
    let risk_level = normalize_enum(&input.risk_level, &["low", "medium", "high"], "risk_level")?;
    let phase = validate_text(&input.phase, "phase", 80)?;
    let tech_stack: Vec<String> = input
        .tech_stack
        .into_iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .take(20)
        .collect();

    let mut warnings: Vec<String> = Vec::new();
    let mut reason_parts: Vec<String> = Vec::new();
    reason_parts.push(format!("项目类型: {project_type}"));
    reason_parts.push(format!("风险等级: {risk_level}"));
    reason_parts.push(format!("阶段: {phase}"));

    // 读取所有模板和已有项目成员
    let templates = list_agent_templates(connection)?;
    let existing_agents = list_project_agents(connection)?;
    let existing_template_ids: std::collections::HashSet<String> = existing_agents
        .iter()
        .filter(|a| a.removed_at.is_none())
        .map(|a| a.agent_template_id.clone())
        .collect();

    // --- 核心员工推荐 ---
    let mut recommended_core: Vec<ProjectAgentSummary> = Vec::new();
    let mut recommended_expert: Vec<ProjectAgentSummary> = Vec::new();

    for template in &templates {
        if !template.enabled {
            continue;
        }

        let is_match = match template.category.as_str() {
            "core" => {
                // 核心员工：始终推荐，无需匹配
                reason_parts.push(format!("核心员工 '{}' 默认包含", template.name));
                true
            }
            "expert" => {
                // 专家匹配：根据项目类型和技术栈决定
                let matched =
                    expert_matches_project(template, &project_type, &tech_stack, &risk_level);
                if matched {
                    reason_parts.push(format!(
                        "专家 '{}' 匹配项目类型 '{}'",
                        template.name, project_type
                    ));
                }
                matched
            }
            _ => false,
        };

        if !is_match {
            continue;
        }

        // 跳过已有项目成员
        if existing_template_ids.contains(&template.id) {
            continue;
        }

        // 构造推荐摘要
        let model_id = template.default_model_id.clone();
        let summary = ProjectAgentSummary {
            id: format!("recommended_{}", template.id),
            project_id: project_id.clone(),
            agent_template_id: template.id.clone(),
            name: template.name.clone(),
            role: template.role.clone(),
            source: "recommended".to_string(),
            executor_key: template.default_executor_key.clone(),
            model_id,
            module_scope: template.module_scope.clone(),
            status: "idle".to_string(),
            joined_at: current_timestamp(),
            removed_at: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        };

        match template.category.as_str() {
            "core" => recommended_core.push(summary),
            "expert" => recommended_expert.push(summary),
            _ => {}
        }
    }

    // 跨模块警告
    let modules: std::collections::HashSet<&str> =
        templates.iter().map(|t| t.module_scope.as_str()).collect();
    if modules.len() > 5 {
        warnings.push("项目跨模块范围较大，建议总控拆分任务后再分派给各角色。".into());
    }
    if risk_level == "high" {
        warnings.push("高风险项目：所有 Runner 执行前必须经过边界检查和审批。".into());
    }

    let reason = reason_parts.join("；");

    Ok(RecommendProjectAgentsOutput {
        project_id,
        recommended_core_agents: recommended_core,
        recommended_expert_agents: recommended_expert,
        reason,
        warnings,
    })
}

/// 专家模板是否匹配当前项目
fn expert_matches_project(
    template: &AgentTemplateSummary,
    project_type: &str,
    tech_stack: &[String],
    _risk_level: &str,
) -> bool {
    let pt = project_type.to_lowercase();
    let stack_lower: Vec<String> = tech_stack.iter().map(|s| s.to_lowercase()).collect();

    // 精确 rule 匹配
    let rules: &[(&str, &[&str])] = &[
        ("ux", &["frontend", "web", "mobile", "ui"]),
        ("desktop", &["desktop", "tauri", "electron", "native"]),
        ("security", &["auth", "security", "api"]),
        (
            "data",
            &["database", "sqlite", "postgres", "data", "analytics"],
        ),
        ("devops", &["docker", "ci/cd", "deploy", "server"]),
        ("qa", &["test", "qa", "quality"]),
        ("ai_prompt", &["ai", "llm", "prompt", "model"]),
        ("docs", &["docs", "documentation"]),
        ("reviewer", &["review", "audit"]),
    ];

    for (role, keywords) in rules {
        if template.role == *role {
            for kw in *keywords {
                if pt.contains(kw) || stack_lower.iter().any(|s| s.contains(kw)) {
                    return true;
                }
            }
        }
    }

    // 常规 fallback：高风险的 web 项目推荐安全专家
    false
}

// ---------------------------------------------------------------------------
// Checkpoint 4：Runner 执行前边界强校验
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckAgentBoundaryInput {
    pub agent_id: String,
    pub task_id: Option<String>,
    pub task_type: String,
    pub module_scope: String,
    pub target_path: Option<String>,
    pub forbidden_actions: Vec<String>,
    pub requested_action: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CheckAgentBoundaryOutput {
    pub check_id: String,
    pub decision: String,
    pub reason: String,
}

/// Runner 执行前边界检查：校验 agent、task_type、module_scope、target_path、forbidden_actions
pub fn check_agent_boundary(
    connection: &Connection,
    input: CheckAgentBoundaryInput,
) -> Result<CheckAgentBoundaryOutput, String> {
    let project_id = current_project_id(connection)?;
    let agent_id = validate_key(&input.agent_id, "agent_id")?;
    let task_type = validate_key(&input.task_type, "task_type")?;
    let module_scope = validate_key(&input.module_scope, "module_scope")?;
    let requested_action = validate_text(&input.requested_action, "requested_action", 200)?;
    let target_path = normalize_optional_text(input.target_path, "target_path", 500)?;

    // 1. Agent 是否存在且属于当前项目
    let agent = connection
        .query_row(
            "SELECT id, agent_template_id, status, module_scope, removed_at
             FROM project_agents
             WHERE id = ?1 AND project_id = ?2",
            params![agent_id.as_str(), project_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("database_error: lookup agent failed: {e}"))?;

    let (_agent_id_val, template_id, agent_status, agent_module, removed_at) = match agent {
        Some(v) => v,
        None => {
            let (decision, reason) = ("denied", "Agent 不存在或不属于当前项目");
            write_boundary_check(
                connection,
                &project_id,
                input.task_id.as_deref(),
                &agent_id,
                &requested_action,
                Some(&task_type),
                &module_scope,
                target_path.as_deref(),
                decision,
                reason,
            )?;
            return Ok(CheckAgentBoundaryOutput {
                check_id: format!("boundary_check_{}", current_timestamp()),
                decision: decision.into(),
                reason: reason.into(),
            });
        }
    };

    // 2. Agent 已软移除
    if removed_at.is_some() {
        let (decision, reason) = ("denied", "Agent 已被移除");
        write_boundary_check(
            connection,
            &project_id,
            input.task_id.as_deref(),
            &agent_id,
            &requested_action,
            Some(&task_type),
            &module_scope,
            target_path.as_deref(),
            decision,
            reason,
        )?;
        return Ok(CheckAgentBoundaryOutput {
            check_id: format!("boundary_check_{}", current_timestamp()),
            decision: decision.into(),
            reason: reason.into(),
        });
    }

    // 3. Agent 状态不是 active
    if agent_status != "active" {
        let (decision, reason) = ("needs_approval", "Agent 状态非活跃，需要审批");
        write_boundary_check(
            connection,
            &project_id,
            input.task_id.as_deref(),
            &agent_id,
            &requested_action,
            Some(&task_type),
            &module_scope,
            target_path.as_deref(),
            decision,
            reason,
        )?;
        return Ok(CheckAgentBoundaryOutput {
            check_id: format!("boundary_check_{}", current_timestamp()),
            decision: decision.into(),
            reason: reason.into(),
        });
    }

    // 4. task_type 是否在模板允许范围
    let template = connection
        .query_row(
            "SELECT allowed_task_types, allowed_paths, forbidden_actions
             FROM agent_templates WHERE id = ?1",
            params![template_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("database_error: lookup template failed: {e}"))?;

    if let Some((allowed_task_types_json, allowed_paths_json, forbidden_actions_json)) = template {
        let allowed_types: Vec<String> = parse_string_list(&allowed_task_types_json);
        let allowed_paths: Vec<String> = parse_string_list(&allowed_paths_json);
        let forbidden: Vec<String> = parse_string_list(&forbidden_actions_json);

        // task_type 不在允许范围 → denied 或 needs_approval
        if !allowed_types.is_empty() && !allowed_types.contains(&task_type) {
            let (decision, reason) = ("needs_approval", "任务类型不在 Agent 模板允许范围");
            write_boundary_check(
                connection,
                &project_id,
                input.task_id.as_deref(),
                &agent_id,
                &requested_action,
                Some(&task_type),
                &module_scope,
                target_path.as_deref(),
                decision,
                reason,
            )?;
            return Ok(CheckAgentBoundaryOutput {
                check_id: format!("boundary_check_{}", current_timestamp()),
                decision: decision.into(),
                reason: reason.into(),
            });
        }

        // 检查 target_path
        if let Some(ref path) = target_path {
            let path_allowed = allowed_paths.is_empty()
                || allowed_paths
                    .iter()
                    .any(|allowed| path_matches(path, allowed));
            if !path_allowed {
                let (decision, reason) = ("denied", "目标路径不在 Agent 允许范围内");
                write_boundary_check(
                    connection,
                    &project_id,
                    input.task_id.as_deref(),
                    &agent_id,
                    &requested_action,
                    Some(&task_type),
                    &module_scope,
                    target_path.as_deref(),
                    decision,
                    reason,
                )?;
                return Ok(CheckAgentBoundaryOutput {
                    check_id: format!("boundary_check_{}", current_timestamp()),
                    decision: decision.into(),
                    reason: reason.into(),
                });
            }
        }

        // 检查 forbidden_actions
        for action in &input.forbidden_actions {
            if forbidden.contains(action) {
                let (decision, reason) = ("denied", "操作命中模板禁止动作列表");
                write_boundary_check(
                    connection,
                    &project_id,
                    input.task_id.as_deref(),
                    &agent_id,
                    &requested_action,
                    Some(&task_type),
                    &module_scope,
                    target_path.as_deref(),
                    decision,
                    reason,
                )?;
                return Ok(CheckAgentBoundaryOutput {
                    check_id: format!("boundary_check_{}", current_timestamp()),
                    decision: decision.into(),
                    reason: reason.into(),
                });
            }
        }
    }

    // 5. module_scope 不匹配 → needs_approval
    if !agent_module.is_empty() && agent_module != module_scope && !module_scope.is_empty() {
        let (decision, reason) = ("needs_approval", "模块范围与 Agent 配置不匹配");
        write_boundary_check(
            connection,
            &project_id,
            input.task_id.as_deref(),
            &agent_id,
            &requested_action,
            Some(&task_type),
            &module_scope,
            target_path.as_deref(),
            decision,
            reason,
        )?;
        return Ok(CheckAgentBoundaryOutput {
            check_id: format!("boundary_check_{}", current_timestamp()),
            decision: decision.into(),
            reason: reason.into(),
        });
    }

    // 全部通过
    let (decision, reason) = ("allowed", "边界检查通过");
    write_boundary_check(
        connection,
        &project_id,
        input.task_id.as_deref(),
        &agent_id,
        &requested_action,
        Some(&task_type),
        &module_scope,
        target_path.as_deref(),
        decision,
        reason,
    )?;
    Ok(CheckAgentBoundaryOutput {
        check_id: format!("boundary_check_{}", current_timestamp()),
        decision: decision.into(),
        reason: reason.into(),
    })
}

/// 写入 agent_boundary_checks 表
fn write_boundary_check(
    connection: &Connection,
    project_id: &str,
    task_id: Option<&str>,
    agent_id: &str,
    requested_action: &str,
    task_type: Option<&str>,
    module_scope: &str,
    target_path: Option<&str>,
    decision: &str,
    reason: &str,
) -> Result<(), String> {
    let id = format!("boundary_check_{}", current_timestamp());
    let now = current_timestamp();
    connection
        .execute(
            "INSERT INTO agent_boundary_checks (
                id, project_id, task_id, agent_id, requested_action,
                task_type, module_scope, target_path, decision, reason, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id.as_str(),
                project_id,
                task_id,
                agent_id,
                requested_action,
                task_type,
                module_scope,
                target_path,
                decision,
                reason,
                now.as_str(),
            ],
        )
        .map_err(|e| format!("database_error: write boundary check failed: {e}"))?;
    Ok(())
}

/// 简化的 glob 匹配：支持 `**` 和 `*` 通配符
fn path_matches(path: &str, pattern: &str) -> bool {
    if pattern.contains("**") {
        let prefix = pattern.trim_end_matches("**").trim_end_matches('/');
        return path.starts_with(prefix);
    }
    if pattern.contains('*') {
        let prefix = pattern.trim_end_matches('*');
        return path.starts_with(prefix);
    }
    path == pattern || path.starts_with(&format!("{}/", pattern.trim_end_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_db() -> (db::DbState, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-agent-config-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let state = db::initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    #[test]
    fn list_defaults_are_seeded() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            assert_eq!(list_executor_configs(&conn).expect("configs").len(), 1);
            assert_eq!(list_agent_templates(&conn).expect("templates").len(), 10);
            assert_eq!(list_project_agents(&conn).expect("agents").len(), 4);
            assert_eq!(list_executor_skills(&conn).expect("skills").len(), 8);
            assert_eq!(
                list_agent_boundary_checks(
                    &conn,
                    ListBoundaryChecksInput {
                        project_id: None,
                        limit: None
                    }
                )
                .expect("checks")
                .len(),
                0
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn upsert_executor_config_persists_config_without_secrets() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let config = upsert_executor_config(
                &conn,
                UpsertExecutorConfigInput {
                    key: "local_llm".into(),
                    label: "本地 LLM".into(),
                    kind: "external_executor".into(),
                    provider: Some("openai_compat".into()),
                    base_url_status: Some("configured_by_system_settings".into()),
                    executable_path: None,
                    status: "active".into(),
                },
            )
            .expect("upsert");
            assert_eq!(config.key, "local_llm");
            assert_eq!(config.status, "active");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn upsert_and_delete_executor_model_round_trips_non_builtin_model() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let model = upsert_executor_model(
                &conn,
                UpsertExecutorModelInput {
                    project_id: None,
                    executor_key: "model_gateway_default".into(),
                    provider: "openai_compat".into(),
                    model_id: "custom-model-1".into(),
                    display_name: "Custom Model 1".into(),
                    purpose: "agent_task".into(),
                    enabled: true,
                },
            )
            .expect("model should upsert");
            assert_eq!(model.executor_key, "model_gateway_default");
            assert!(!model.is_builtin);

            let models = list_executor_models(
                &conn,
                ListExecutorModelsInput {
                    project_id: None,
                    executor_key: Some("model_gateway_default".into()),
                    purpose: Some("agent_task".into()),
                },
            )
            .expect("models should list");
            assert!(models
                .iter()
                .any(|entry| entry.model_id == "custom-model-1"));

            delete_executor_model(
                &conn,
                DeleteExecutorModelInput {
                    model_record_id: model.id,
                },
            )
            .expect("non-builtin model should delete");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn delete_executor_config_blocks_builtins() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let err = delete_executor_config(
                &conn,
                DeleteExecutorConfigInput {
                    key: "model_gateway_default".into(),
                },
            )
            .expect_err("builtin should not delete");
            assert!(err.contains("builtin"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn upsert_template_and_project_agent_are_round_trippable() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            let template = upsert_agent_template(
                &conn,
                UpsertAgentTemplateInput {
                    role: "data".into(),
                    category: "expert".into(),
                    name: "数据专家".into(),
                    specialty: Some("schema review".into()),
                    stack: Some("sqlite".into()),
                    module_scope: "database".into(),
                    allowed_task_types: vec!["schema_review".into()],
                    allowed_paths: vec!["data/migrations/**".into()],
                    forbidden_actions: vec!["git_push".into()],
                    default_executor_key: "model_gateway_default".into(),
                    default_model_id: Some("gpt-5.4-mini".into()),
                    enabled: true,
                },
            )
            .expect("template");
            assert_eq!(template.role, "data");

            let agent = upsert_project_agent(
                &conn,
                UpsertProjectAgentInput {
                    project_id: None,
                    agent_template_id: template.id.clone(),
                    name: "数据执行员".into(),
                    role: "data".into(),
                    source: "manual".into(),
                    executor_key: "model_gateway_default".into(),
                    model_id: Some("gpt-5.4-mini".into()),
                    module_scope: "database".into(),
                    status: "active".into(),
                },
            )
            .expect("agent");
            assert_eq!(agent.role, "data");

            let skill = upsert_executor_skill(
                &conn,
                UpsertExecutorSkillInput {
                    executor_key: "model_gateway_default".into(),
                    agent_template_id: Some(template.id.clone()),
                    skill_name: "schema_review".into(),
                    skill_scope: "database".into(),
                    enabled: true,
                },
            )
            .expect("skill");
            assert_eq!(skill.skill_name, "schema_review");

            let checks = list_agent_boundary_checks(
                &conn,
                ListBoundaryChecksInput {
                    project_id: None,
                    limit: Some(10),
                },
            )
            .expect("checks");
            assert!(checks.is_empty());
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn executor_models_are_scoped_by_executor_key() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            upsert_executor_config(
                &conn,
                UpsertExecutorConfigInput {
                    key: "local_executor".into(),
                    label: "本地执行器".into(),
                    kind: "external_executor".into(),
                    provider: Some("openai_compat".into()),
                    base_url_status: Some("configured_by_system_settings".into()),
                    executable_path: None,
                    status: "active".into(),
                },
            )
            .expect("executor should upsert");

            let default_model = upsert_executor_model(
                &conn,
                UpsertExecutorModelInput {
                    project_id: None,
                    executor_key: "model_gateway_default".into(),
                    provider: "openai_compat".into(),
                    model_id: "shared-model".into(),
                    display_name: "Shared Gateway".into(),
                    purpose: "agent_task".into(),
                    enabled: true,
                },
            )
            .expect("default executor model should upsert");
            let local_model = upsert_executor_model(
                &conn,
                UpsertExecutorModelInput {
                    project_id: None,
                    executor_key: "local_executor".into(),
                    provider: "openai_compat".into(),
                    model_id: "shared-model".into(),
                    display_name: "Shared Local".into(),
                    purpose: "agent_task".into(),
                    enabled: true,
                },
            )
            .expect("local executor model should upsert");

            assert_ne!(default_model.id, local_model.id);
            assert_eq!(default_model.executor_key, "model_gateway_default");
            assert_eq!(local_model.executor_key, "local_executor");

            let models = list_executor_models(
                &conn,
                ListExecutorModelsInput {
                    project_id: None,
                    executor_key: None,
                    purpose: Some("agent_task".into()),
                },
            )
            .expect("models should list");
            assert_eq!(
                models
                    .iter()
                    .filter(|entry| entry.model_id == "shared-model")
                    .count(),
                2
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn project_agent_model_must_belong_to_executor() {
        let (state, test_dir) = test_db();
        {
            let conn = state.connection().expect("connection");
            upsert_executor_config(
                &conn,
                UpsertExecutorConfigInput {
                    key: "isolated_executor".into(),
                    label: "隔离执行器".into(),
                    kind: "external_executor".into(),
                    provider: Some("openai_compat".into()),
                    base_url_status: Some("configured_by_system_settings".into()),
                    executable_path: None,
                    status: "active".into(),
                },
            )
            .expect("executor should upsert");
            upsert_executor_model(
                &conn,
                UpsertExecutorModelInput {
                    project_id: None,
                    executor_key: "model_gateway_default".into(),
                    provider: "openai_compat".into(),
                    model_id: "gateway-only-model".into(),
                    display_name: "Gateway Only".into(),
                    purpose: "agent_task".into(),
                    enabled: true,
                },
            )
            .expect("model should upsert");

            let err = upsert_project_agent(
                &conn,
                UpsertProjectAgentInput {
                    project_id: None,
                    agent_template_id: "agent_template_frontend".into(),
                    name: "隔离前端执行员".into(),
                    role: "frontend".into(),
                    source: "manual".into(),
                    executor_key: "isolated_executor".into(),
                    model_id: Some("gateway-only-model".into()),
                    module_scope: "frontend".into(),
                    status: "active".into(),
                },
            )
            .expect_err("model should not be assignable across executors");
            assert!(err.contains("enabled model"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }
}
