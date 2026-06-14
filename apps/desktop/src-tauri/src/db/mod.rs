use rusqlite::{params, Connection};
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

const DATABASE_FILE_NAME: &str = "agent-swarm.sqlite";
const INITIAL_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/001_initial_sqlite.sql");
const AGENT_RUN_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/002_add_agent_runs.sql");
const MODEL_CALLS_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/003_add_model_calls.sql");
const INITIAL_SEED_JSON: &str =
    include_str!("../../../../../data/seed/project_agent_swarm.seed.json");

type InitResult<T> = Result<T, Box<dyn Error>>;

pub struct DbState {
    connection: Mutex<Connection>,
}

impl DbState {
    pub fn connection(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "SQLite 连接锁异常，请重启桌面宿主后重试".to_string())
    }
}

pub fn initialize(app_data_dir: PathBuf) -> InitResult<DbState> {
    fs::create_dir_all(&app_data_dir)?;

    let database_path = app_data_dir.join(DATABASE_FILE_NAME);
    let mut connection = Connection::open(database_path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;

    run_initial_migration(&connection)?;
    run_agent_run_migration(&connection)?;
    run_model_calls_migration(&connection)?;
    seed_initial_data_if_needed(&mut connection)?;

    Ok(DbState {
        connection: Mutex::new(connection),
    })
}

fn run_initial_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(INITIAL_MIGRATION_SQL)?;
    Ok(())
}

fn run_agent_run_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(AGENT_RUN_MIGRATION_SQL)?;
    Ok(())
}

fn run_model_calls_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(MODEL_CALLS_MIGRATION_SQL)?;
    Ok(())
}

fn seed_initial_data_if_needed(connection: &mut Connection) -> InitResult<()> {
    let project_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;

    if project_count > 0 {
        return Ok(());
    }

    let seed: SeedRoot = serde_json::from_str(INITIAL_SEED_JSON)?;
    let tx = connection.transaction()?;
    let now = seed.seeded_at.as_str();
    let project = &seed.project;
    let project_id = project.id.as_str();

    tx.execute(
        "INSERT INTO projects (
            id, name, status, phase, description, workspace_path, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project_id,
            project.name.as_str(),
            project.status.as_str(),
            project.phase.as_deref(),
            project.description.as_deref(),
            project.workspace_path.as_deref(),
            now,
            now
        ],
    )?;

    for agent in &seed.agents {
        let permissions = serde_json::to_string(&agent.permissions)?;

        tx.execute(
            "INSERT INTO agents (
                id, project_id, name, role, status, model, permissions, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                agent.id.as_str(),
                project_id,
                agent.name.as_str(),
                agent.role.as_str(),
                agent.status.as_str(),
                agent.model.as_deref(),
                permissions,
                now,
                now
            ],
        )?;
    }

    for task in &seed.tasks {
        let depends_on = serde_json::to_string(&task.depends_on)?;

        tx.execute(
            "INSERT INTO tasks (
                id, project_id, title, description, status, priority, assigned_agent_id,
                depends_on, risk_level, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.id.as_str(),
                project_id,
                task.title.as_str(),
                task.description.as_deref(),
                task.status.as_str(),
                task.priority.as_str(),
                task.assigned_agent_id.as_deref(),
                depends_on,
                task.risk_level.as_deref(),
                now,
                now
            ],
        )?;
    }

    for approval in &seed.approvals {
        let operation_types = serde_json::to_string(&approval.operation_types)?;
        let created_at = approval.created_at.as_deref().unwrap_or(now);

        tx.execute(
            "INSERT INTO approvals (
                id, project_id, task_id, request_agent_id, target_service, operation_types,
                status, risk_level, reason, reject_reason, approved_at, rejected_at,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                approval.id.as_str(),
                project_id,
                approval.task_id.as_deref(),
                approval.request_agent_id.as_str(),
                approval.target_service.as_str(),
                operation_types,
                approval.status.as_str(),
                approval.risk_level.as_str(),
                approval.reason.as_deref(),
                approval.reject_reason.as_deref(),
                approval.approved_at.as_deref(),
                approval.rejected_at.as_deref(),
                created_at,
                created_at
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[derive(Deserialize)]
struct SeedRoot {
    #[serde(rename = "seededAt")]
    seeded_at: String,
    project: SeedProject,
    #[serde(default)]
    agents: Vec<SeedAgent>,
    #[serde(default)]
    tasks: Vec<SeedTask>,
    #[serde(default)]
    approvals: Vec<SeedApproval>,
}

#[derive(Deserialize)]
struct SeedProject {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    phase: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "workspacePath")]
    workspace_path: Option<String>,
}

#[derive(Deserialize)]
struct SeedAgent {
    id: String,
    name: String,
    role: String,
    status: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    permissions: Vec<String>,
}

#[derive(Deserialize)]
struct SeedTask {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    priority: String,
    #[serde(default, rename = "assignedAgentId")]
    assigned_agent_id: Option<String>,
    #[serde(default, rename = "dependsOn")]
    depends_on: Vec<String>,
    #[serde(default, rename = "riskLevel")]
    risk_level: Option<String>,
}

#[derive(Deserialize)]
struct SeedApproval {
    id: String,
    status: String,
    #[serde(rename = "riskLevel")]
    risk_level: String,
    #[serde(rename = "requestAgentId")]
    request_agent_id: String,
    #[serde(default, rename = "taskId")]
    task_id: Option<String>,
    #[serde(rename = "targetService")]
    target_service: String,
    #[serde(rename = "operationTypes")]
    operation_types: Vec<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default, rename = "rejectReason")]
    reject_reason: Option<String>,
    #[serde(default, rename = "approvedAt")]
    approved_at: Option<String>,
    #[serde(default, rename = "rejectedAt")]
    rejected_at: Option<String>,
    #[serde(default, rename = "createdAt")]
    created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::initialize;
    use crate::services::{
        agents::list_agents, approvals::list_approvals, model_gateway::create_project_plan_draft,
        tasks::list_tasks,
    };
    use rusqlite::Connection;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn initialize_creates_minimal_tables_and_seed_data_once() {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-sqlite-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let state = initialize(test_dir.clone()).expect("sqlite should initialize");
        {
            let connection = state.connection().expect("connection should be available");
            assert_eq!(count_rows(&connection, "projects"), 1);
            assert_eq!(count_rows(&connection, "agents"), 6);
            assert_eq!(count_rows(&connection, "tasks"), 4);
            assert_eq!(count_rows(&connection, "approvals"), 3);
            assert_eq!(count_rows(&connection, "agent_runs"), 0);
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
            assert_eq!(count_rows(&connection, "model_calls"), 0);

            let agents = list_agents(&connection).expect("agents should be readable");
            assert_eq!(agents.len(), 6);
            assert!(agents
                .iter()
                .any(|agent| agent.id == "agent_architect" && agent.permissions.len() == 3));

            let tasks = list_tasks(&connection).expect("tasks should be readable");
            assert_eq!(tasks.len(), 4);
            assert!(tasks.iter().any(|task| {
                task.id == "task_runner_approval_page"
                    && task.depends_on == vec!["task_frontend_mock_data".to_string()]
            }));

            let approvals = list_approvals(&connection).expect("approvals should be readable");
            assert_eq!(approvals.len(), 3);
            assert!(approvals.iter().any(|approval| {
                approval.id == "approval_runner_permissions"
                    && approval
                        .operation_types
                        .contains(&"git_checkpoint".to_string())
            }));
        }
        drop(state);

        let state = initialize(test_dir.clone()).expect("sqlite should reinitialize");
        {
            let connection = state.connection().expect("connection should be available");
            assert_eq!(count_rows(&connection, "projects"), 1);
            assert_eq!(count_rows(&connection, "agents"), 6);
            assert_eq!(count_rows(&connection, "tasks"), 4);
            assert_eq!(count_rows(&connection, "approvals"), 3);
            assert_eq!(count_rows(&connection, "agent_runs"), 0);
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
            assert_eq!(count_rows(&connection, "model_calls"), 0);
        }
        drop(state);

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn model_calls_table_has_expected_columns() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let mut stmt = connection
                .prepare("PRAGMA table_info('model_calls')")
                .expect("should be able to query table_info");
            let columns: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .expect("should map columns")
                .filter_map(|r| r.ok())
                .collect();
            let expected = vec![
                "id",
                "project_id",
                "purpose",
                "provider",
                "model",
                "status",
                "request_hash",
                "structured_summary",
                "token_usage",
                "cost_estimate",
                "error_category",
                "error_message",
                "redaction_applied",
                "duration_ms",
                "related_approval_id",
                "runtime_event_id",
                "created_at",
                "updated_at",
            ];
            for col in &expected {
                assert!(
                    columns.contains(&col.to_string()),
                    "column {col} should exist"
                );
            }
            assert_eq!(
                columns.len(),
                18,
                "model_calls should have exactly 18 columns"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn model_calls_indexes_exist() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let mut stmt = connection
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='model_calls'",
                )
                .expect("should be able to query sqlite_master");
            let indexes: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .expect("should map indexes")
                .filter_map(|r| r.ok())
                .collect();
            assert!(
                indexes.iter().any(|i| i.contains("project_id")),
                "should have project_id index"
            );
            assert!(
                indexes.iter().any(|i| i.contains("status")),
                "should have status index"
            );
            assert!(
                indexes.iter().any(|i| i.contains("created_at")),
                "should have created_at index"
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn feature_disabled_does_not_write_model_calls() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let before = count_rows(&connection, "model_calls");

            let response = create_project_plan_draft("测试想法", &None, false, &None)
                .expect("should return feature_disabled response");
            assert_eq!(response.status, "feature_disabled");

            let after = count_rows(&connection, "model_calls");
            assert_eq!(before, after, "feature_disabled 不应写入 model_calls");
            assert_eq!(after, 0, "model_calls 仍应为空");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn feature_disabled_does_not_create_runtime_events() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let before = count_rows(&connection, "runtime_events");

            let response = create_project_plan_draft("测试想法", &None, false, &None)
                .expect("should return feature_disabled response");
            assert_eq!(response.status, "feature_disabled");

            let after = count_rows(&connection, "runtime_events");
            assert_eq!(before, after, "feature_disabled 不应写入 runtime_events");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    fn test_db() -> (crate::db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-db-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        let state = initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    fn count_rows(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable")
    }
}
