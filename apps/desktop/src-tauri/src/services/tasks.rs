use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assigned_agent_id: Option<String>,
    pub depends_on: Vec<String>,
    pub risk_level: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub priority: String,
    #[serde(default)]
    pub assigned_agent_id: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub risk_level: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    pub task: TaskSummary,
}

pub fn list_tasks(connection: &Connection) -> Result<Vec<TaskSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, title, description, status, priority, assigned_agent_id,
                COALESCE(depends_on, '[]'), risk_level, created_at, updated_at
             FROM tasks
             ORDER BY created_at, id",
        )
        .map_err(|error| format!("database_error: read task list failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let depends_on_json: String = row.get(7)?;

            Ok(TaskSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                assigned_agent_id: row.get(6)?,
                depends_on: parse_string_list(&depends_on_json),
                risk_level: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|error| format!("database_error: read task list failed: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: read task list failed: {error}"))
}

pub fn create_task(
    connection: &mut Connection,
    input: CreateTaskInput,
) -> Result<CreateTaskResponse, String> {
    let project_id = get_current_project_id(connection)?;
    let title = normalize_required_text(input.title, 1, 120, "title")?;
    let description = normalize_optional_text(input.description, 2000, "description")?;
    let priority = normalize_enum(input.priority, &["low", "medium", "high"], "priority")?;
    let assigned_agent_id = normalize_optional_id(input.assigned_agent_id);
    let risk_level =
        normalize_optional_enum(input.risk_level, &["low", "medium", "high"], "risk_level")?;
    let id = generate_task_id();
    let depends_on = normalize_depends_on(connection, &project_id, &id, input.depends_on)?;

    if let Some(agent_id) = assigned_agent_id.as_deref() {
        ensure_agent_belongs_to_project(connection, &project_id, agent_id)?;
    }

    let depends_on_json = serde_json::to_string(&depends_on)
        .map_err(|error| format!("database_error: serialize task dependencies failed: {error}"))?;
    let now = current_timestamp();

    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start create task transaction failed: {error}")
    })?;
    tx.execute(
        "INSERT INTO tasks (
            id, project_id, title, description, status, priority, assigned_agent_id,
            depends_on, risk_level, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id.as_str(),
            project_id.as_str(),
            title.as_str(),
            description.as_deref(),
            "queued",
            priority.as_str(),
            assigned_agent_id.as_deref(),
            depends_on_json.as_str(),
            risk_level.as_deref(),
            now.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: create task failed: {error}"))?;
    tx.commit()
        .map_err(|error| format!("database_error: commit create task failed: {error}"))?;

    let task = get_task_by_id(connection, &id)?;
    Ok(CreateTaskResponse { task })
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn get_current_project_id(connection: &Connection) -> Result<String, String> {
    connection
        .query_row(
            "SELECT id FROM projects ORDER BY created_at LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("database_error: read current project failed: {error}"))?
        .ok_or_else(|| "not_found: current project not found".to_string())
}

fn get_task_by_id(connection: &Connection, task_id: &str) -> Result<TaskSummary, String> {
    connection
        .query_row(
            "SELECT id, project_id, title, description, status, priority, assigned_agent_id,
                COALESCE(depends_on, '[]'), risk_level, created_at, updated_at
             FROM tasks
             WHERE id = ?1",
            [task_id],
            |row| {
                let depends_on_json: String = row.get(7)?;

                Ok(TaskSummary {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: row.get(4)?,
                    priority: row.get(5)?,
                    assigned_agent_id: row.get(6)?,
                    depends_on: parse_string_list(&depends_on_json),
                    risk_level: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("database_error: read task failed: {error}"))?
        .ok_or_else(|| "not_found: task not found".to_string())
}

fn normalize_required_text(
    value: String,
    min_len: usize,
    max_len: usize,
    field: &str,
) -> Result<String, String> {
    let normalized = value.trim().to_string();
    let length = normalized.chars().count();

    if length < min_len || length > max_len {
        return Err(format!(
            "invalid_input: {field} length must be between {min_len} and {max_len}"
        ));
    }

    Ok(normalized)
}

fn normalize_optional_text(
    value: Option<String>,
    max_len: usize,
    field: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }

    if normalized.chars().count() > max_len {
        return Err(format!(
            "invalid_input: {field} length must be at most {max_len}"
        ));
    }

    Ok(Some(normalized))
}

fn normalize_enum(value: String, allowed: &[&str], field: &str) -> Result<String, String> {
    let normalized = value.trim().to_string();
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!("invalid_input: {field} is not allowed"))
    }
}

fn normalize_optional_enum(
    value: Option<String>,
    allowed: &[&str],
    field: &str,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Ok(None);
    }

    if allowed.contains(&normalized.as_str()) {
        Ok(Some(normalized))
    } else {
        Err(format!("invalid_input: {field} is not allowed"))
    }
}

fn normalize_optional_id(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_depends_on(
    connection: &Connection,
    project_id: &str,
    new_task_id: &str,
    depends_on: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for dependency in depends_on {
        let dependency = dependency.trim().to_string();
        if dependency.is_empty() {
            return Err("invalid_input: depends_on cannot contain empty task ids".to_string());
        }

        if dependency == new_task_id {
            return Err("invalid_input: task cannot depend on itself".to_string());
        }

        if !seen.insert(dependency.clone()) {
            return Err("invalid_input: depends_on cannot contain duplicate task ids".to_string());
        }

        ensure_task_belongs_to_project(connection, project_id, &dependency)?;
        normalized.push(dependency);
    }

    Ok(normalized)
}

fn ensure_agent_belongs_to_project(
    connection: &Connection,
    project_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agents WHERE id = ?1 AND project_id = ?2",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check agent failed: {error}"))?;

    if count == 1 {
        Ok(())
    } else {
        Err("not_found: assigned agent not found".to_string())
    }
}

fn ensure_task_belongs_to_project(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check dependency task failed: {error}"))?;

    if count == 1 {
        Ok(())
    } else {
        Err("not_found: dependency task not found".to_string())
    }
}

fn generate_task_id() -> String {
    format!("task_{}", timestamp_nanos())
}

fn current_timestamp() -> String {
    timestamp_nanos().to_string()
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::{create_task, parse_string_list, CreateTaskInput};
    use rusqlite::{params, Connection};

    const INITIAL_MIGRATION_SQL: &str =
        include_str!("../../../../../data/migrations/001_initial_sqlite.sql");

    #[test]
    fn parse_string_list_reads_dependencies() {
        assert_eq!(
            parse_string_list(r#"["task_frontend_mock_data"]"#),
            vec!["task_frontend_mock_data".to_string()]
        );
    }

    #[test]
    fn create_task_inserts_queued_task_without_side_effects() {
        let mut connection = setup_connection();
        let response = create_task(
            &mut connection,
            CreateTaskInput {
                title: "  New task  ".to_string(),
                description: Some("  Details  ".to_string()),
                priority: "high".to_string(),
                assigned_agent_id: Some("agent_architect".to_string()),
                depends_on: vec!["task_existing".to_string()],
                risk_level: Some("medium".to_string()),
            },
        )
        .expect("task should be created");

        assert_eq!(response.task.title, "New task");
        assert_eq!(response.task.description.as_deref(), Some("Details"));
        assert_eq!(response.task.status, "queued");
        assert_eq!(response.task.priority, "high");
        assert_eq!(
            response.task.assigned_agent_id.as_deref(),
            Some("agent_architect")
        );
        assert_eq!(response.task.depends_on, vec!["task_existing".to_string()]);
        assert_eq!(response.task.risk_level.as_deref(), Some("medium"));
        assert_eq!(count_rows(&connection, "approvals"), 0);
    }

    #[test]
    fn create_task_rejects_blank_title() {
        let mut connection = setup_connection();
        let error = create_task(&mut connection, input_with_title("  "))
            .expect_err("blank title should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_task_rejects_too_long_title() {
        let mut connection = setup_connection();
        let error = create_task(&mut connection, input_with_title(&"a".repeat(121)))
            .expect_err("long title should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_task_rejects_too_long_description() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.description = Some("a".repeat(2001));

        let error = create_task(&mut connection, input).expect_err("long description should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_task_rejects_invalid_priority() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.priority = "urgent".to_string();

        let error = create_task(&mut connection, input).expect_err("invalid priority should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_task_rejects_invalid_risk_level() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.risk_level = Some("critical".to_string());

        let error = create_task(&mut connection, input).expect_err("invalid risk should fail");

        assert!(error.contains("invalid_input"));
    }

    #[test]
    fn create_task_rejects_unknown_agent() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.assigned_agent_id = Some("missing_agent".to_string());

        let error = create_task(&mut connection, input).expect_err("unknown agent should fail");

        assert!(error.contains("not_found"));
    }

    #[test]
    fn create_task_rejects_unknown_dependency() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.depends_on = vec!["missing_task".to_string()];

        let error =
            create_task(&mut connection, input).expect_err("unknown dependency should fail");

        assert!(error.contains("not_found"));
    }

    #[test]
    fn create_task_rejects_duplicate_dependencies() {
        let mut connection = setup_connection();
        let mut input = input_with_title("valid title");
        input.depends_on = vec!["task_existing".to_string(), "task_existing".to_string()];

        let error =
            create_task(&mut connection, input).expect_err("duplicate dependency should fail");

        assert!(error.contains("invalid_input"));
    }

    fn input_with_title(title: &str) -> CreateTaskInput {
        CreateTaskInput {
            title: title.to_string(),
            description: None,
            priority: "medium".to_string(),
            assigned_agent_id: None,
            depends_on: Vec::new(),
            risk_level: None,
        }
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite should open");
        connection
            .execute_batch(INITIAL_MIGRATION_SQL)
            .expect("schema should initialize");
        connection
            .execute(
                "INSERT INTO projects (
                    id, name, status, phase, description, workspace_path, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "project_agent_swarm",
                    "agent-swarm",
                    "running",
                    "test",
                    Option::<String>::None,
                    Option::<String>::None,
                    "1",
                    "1"
                ],
            )
            .expect("project should insert");
        connection
            .execute(
                "INSERT INTO agents (
                    id, project_id, name, role, status, model, permissions, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "agent_architect",
                    "project_agent_swarm",
                    "Architect Agent",
                    "architect",
                    "running",
                    Option::<String>::None,
                    "[]",
                    "1",
                    "1"
                ],
            )
            .expect("agent should insert");
        connection
            .execute(
                "INSERT INTO tasks (
                    id, project_id, title, description, status, priority, assigned_agent_id,
                    depends_on, risk_level, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    "task_existing",
                    "project_agent_swarm",
                    "Existing task",
                    Option::<String>::None,
                    "queued",
                    "medium",
                    Option::<String>::None,
                    "[]",
                    Option::<String>::None,
                    "1",
                    "1"
                ],
            )
            .expect("task should insert");

        connection
    }

    fn count_rows(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable")
    }
}
