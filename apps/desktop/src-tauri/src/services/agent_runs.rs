use rusqlite::Connection;
use serde::Serialize;

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

pub fn list_agent_runs(connection: &Connection) -> Result<Vec<AgentRunSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT
                id, project_id, chain_id, root_run_id, parent_run_id,
                sequence, role, agent_id, agent_name, model, status,
                input_summary, output_summary, token_usage, cost_estimate,
                error_category, error_message, requested_by, chain_label,
                created_at, started_at, completed_at, failed_at, updated_at
             FROM agent_runs
             ORDER BY chain_id, sequence",
        )
        .map_err(|error| format!("读取 Agent Run 列表失败：{error}"))?;

    let rows = statement
        .query_map([], |row| {
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
    let sql = if entity_id.is_some() {
        "SELECT
            id, project_id, entity_type, entity_id, event_type,
            before_state, after_state, actor, reason, created_at
         FROM runtime_events
         WHERE entity_type = 'agent_run' AND entity_id = ?1
         ORDER BY created_at"
    } else {
        "SELECT
            id, project_id, entity_type, entity_id, event_type,
            before_state, after_state, actor, reason, created_at
         FROM runtime_events
         WHERE entity_type = 'agent_run'
         ORDER BY created_at"
    };

    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("读取运行时事件失败：{error}"))?;

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

    let rows = if let Some(id) = entity_id {
        statement
            .query_map([id], map_row)
            .map_err(|error| format!("读取运行时事件失败：{error}"))?
    } else {
        statement
            .query_map([], map_row)
            .map_err(|error| format!("读取运行时事件失败：{error}"))?
    };

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取运行时事件失败：{error}"))
}
