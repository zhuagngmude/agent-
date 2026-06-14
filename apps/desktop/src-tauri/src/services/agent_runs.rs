use rusqlite::Connection;
use serde::Serialize;

use super::projects::get_current_project;

#[derive(Serialize)]
pub struct AgentRunSummary {
    pub id: String,
    pub project_id: String,
    pub chain_id: String,
    pub root_run_id: String,
    pub parent_run_id: Option<String>,
    pub sequence: i32,
    pub role: String,
    pub agent_id: Option<String>,
    pub agent_name: String,
    pub model: String,
    pub status: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub token_usage: String,
    pub cost_estimate: String,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
    pub requested_by: String,
    pub chain_label: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub failed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct RuntimeEventSummary {
    pub id: String,
    pub project_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub event_type: String,
    pub before_state: Option<String>,
    pub after_state: Option<String>,
    pub actor: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
    get_current_project(connection).map(|project| project.id)
}

pub fn list_agent_runs(connection: &Connection) -> Result<Vec<AgentRunSummary>, String> {
    let project_id = current_project_id(connection)?;

    let mut statement = connection
        .prepare(
            "SELECT
                id, project_id, chain_id, root_run_id, parent_run_id,
                sequence, role, agent_id, agent_name, model, status,
                input_summary, output_summary, token_usage, cost_estimate,
                error_category, error_message, requested_by, chain_label,
                created_at, started_at, completed_at, failed_at, updated_at
             FROM agent_runs
             WHERE project_id = ?1
             ORDER BY chain_id, sequence",
        )
        .map_err(|error| format!("读取 Agent Run 列表失败：{error}"))?;

    let rows = statement
        .query_map([&project_id], |row| {
            Ok(AgentRunSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chain_id: row.get(2)?,
                root_run_id: row.get(3)?,
                parent_run_id: row.get(4)?,
                sequence: row.get(5)?,
                role: row.get(6)?,
                agent_id: row.get(7)?,
                agent_name: row.get(8)?,
                model: row.get(9)?,
                status: row.get(10)?,
                input_summary: row.get(11)?,
                output_summary: row.get(12)?,
                token_usage: row.get(13)?,
                cost_estimate: row.get(14)?,
                error_category: row.get(15)?,
                error_message: row.get(16)?,
                requested_by: row.get(17)?,
                chain_label: row.get(18)?,
                created_at: row.get(19)?,
                started_at: row.get(20)?,
                completed_at: row.get(21)?,
                failed_at: row.get(22)?,
                updated_at: row.get(23)?,
            })
        })
        .map_err(|error| format!("读取 Agent Run 列表失败：{error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Agent Run 列表失败：{error}"))
}

pub fn list_runtime_events(
    connection: &Connection,
    entity_id: Option<&str>,
) -> Result<Vec<RuntimeEventSummary>, String> {
    let project_id = current_project_id(connection)?;

    let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<RuntimeEventSummary> {
        Ok(RuntimeEventSummary {
            id: row.get(0)?,
            project_id: row.get(1)?,
            entity_type: row.get(2)?,
            entity_id: row.get(3)?,
            event_type: row.get(4)?,
            before_state: row.get(5)?,
            after_state: row.get(6)?,
            actor: row.get(7)?,
            reason: row.get(8)?,
            created_at: row.get(9)?,
        })
    };

    let rows: Vec<RuntimeEventSummary> = if let Some(id) = entity_id {
        let mut stmt = connection
            .prepare(
                "SELECT
                    id, project_id, entity_type, entity_id, event_type,
                    before_state, after_state, actor, reason, created_at
                 FROM runtime_events
                 WHERE project_id = ?1 AND entity_type = 'agent_run' AND entity_id = ?2
                 ORDER BY created_at",
            )
            .map_err(|error| format!("读取运行时事件失败：{error}"))?;
        let mapped = stmt
            .query_map(rusqlite::params![&project_id, id], map_row)
            .map_err(|error| format!("读取运行时事件失败：{error}"))?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("读取运行时事件失败：{error}"))?
    } else {
        let mut stmt = connection
            .prepare(
                "SELECT
                    id, project_id, entity_type, entity_id, event_type,
                    before_state, after_state, actor, reason, created_at
                 FROM runtime_events
                 WHERE project_id = ?1 AND entity_type = 'agent_run'
                 ORDER BY created_at",
            )
            .map_err(|error| format!("读取运行时事件失败：{error}"))?;
        let mapped = stmt
            .query_map(rusqlite::params![&project_id], map_row)
            .map_err(|error| format!("读取运行时事件失败：{error}"))?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("读取运行时事件失败：{error}"))?
    };

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::super::projects::get_current_project;
    use super::list_agent_runs;
    use rusqlite::Connection;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::db;

    fn test_db() -> (db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-agent-runs-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let state = db::initialize(test_dir.clone()).expect("sqlite should initialize");
        (state, test_dir)
    }

    fn current_project_id(connection: &Connection) -> String {
        get_current_project(connection)
            .expect("current project should exist")
            .id
    }

    #[test]
    fn agent_runs_table_exists_after_migration() {
        let (state, test_dir) = test_db();
        let count: i64 = {
            let connection = state.connection().expect("connection should be available");
            connection
                .query_row("SELECT COUNT(*) FROM agent_runs", [], |row| row.get(0))
                .expect("agent_runs table should exist")
        };
        assert_eq!(count, 0, "agent_runs should be empty after migration");
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn runtime_events_table_exists_after_migration() {
        let (state, test_dir) = test_db();
        let count: i64 = {
            let connection = state.connection().expect("connection should be available");
            connection
                .query_row("SELECT COUNT(*) FROM runtime_events", [], |row| row.get(0))
                .expect("runtime_events table should exist")
        };
        assert_eq!(count, 0, "runtime_events should be empty after migration");
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_agent_runs_returns_empty_vec_when_table_is_empty() {
        let (state, test_dir) = test_db();
        let runs = {
            let connection = state.connection().expect("connection should be available");
            list_agent_runs(&connection).expect("list_agent_runs should succeed on empty table")
        };
        assert!(runs.is_empty(), "empty table should return empty vec");
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_agent_runs_filters_by_current_project() {
        let (state, test_dir) = test_db();
        let pid = {
            let connection = state.connection().expect("connection should be available");
            current_project_id(&connection)
        };
        {
            let connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "INSERT INTO agent_runs (id, project_id, chain_id, root_run_id, sequence, role, agent_name, model, status, token_usage, cost_estimate, requested_by, created_at, updated_at)
                     VALUES ('run_a', ?1, 'chain_1', 'run_a', 1, 'architect', 'Test', 'gpt', 'succeeded', '{}', '{}', 'test', '2024-01-01', '2024-01-01')",
                    rusqlite::params![&pid],
                )
                .expect("insert should succeed");
            connection
                .execute(
                    "INSERT INTO agent_runs (id, project_id, chain_id, root_run_id, sequence, role, agent_name, model, status, token_usage, cost_estimate, requested_by, created_at, updated_at)
                     VALUES ('run_b', 'other_project', 'chain_2', 'run_b', 1, 'architect', 'Test', 'gpt', 'succeeded', '{}', '{}', 'test', '2024-01-01', '2024-01-01')",
                    [],
                )
                .expect("insert should succeed");
        }
        let runs = {
            let connection = state.connection().expect("connection should be available");
            list_agent_runs(&connection).expect("list_agent_runs should succeed")
        };
        assert_eq!(runs.len(), 1, "should only return runs for the current project");
        assert_eq!(runs[0].id, "run_a");
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_runtime_events_filters_by_entity_id() {
        let (state, test_dir) = test_db();
        let pid = {
            let connection = state.connection().expect("connection should be available");
            current_project_id(&connection)
        };
        {
            let connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "INSERT INTO runtime_events (id, project_id, entity_type, entity_id, event_type, created_at)
                     VALUES ('evt_a', ?1, 'agent_run', 'run_a', 'started', '2024-01-01')",
                    rusqlite::params![&pid],
                )
                .expect("insert should succeed");
            connection
                .execute(
                    "INSERT INTO runtime_events (id, project_id, entity_type, entity_id, event_type, created_at)
                     VALUES ('evt_b', ?1, 'agent_run', 'run_b', 'completed', '2024-01-02')",
                    rusqlite::params![&pid],
                )
                .expect("insert should succeed");
        }
        {
            let connection = state.connection().expect("connection should be available");
            let all = super::list_runtime_events(&connection, None)
                .expect("list_runtime_events should succeed");
            assert_eq!(all.len(), 2);

            let filtered = super::list_runtime_events(&connection, Some("run_a"))
                .expect("list_runtime_events should succeed");
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].id, "evt_a");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }
}
