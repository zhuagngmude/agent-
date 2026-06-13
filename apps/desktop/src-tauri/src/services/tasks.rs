use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
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

pub fn list_tasks(connection: &Connection) -> Result<Vec<TaskSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, title, description, status, priority, assigned_agent_id,
                COALESCE(depends_on, '[]'), risk_level, created_at, updated_at
             FROM tasks
             ORDER BY created_at, id",
        )
        .map_err(|error| format!("读取任务列表失败：{error}"))?;

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
        .map_err(|error| format!("读取任务列表失败：{error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取任务列表失败：{error}"))
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_string_list;

    #[test]
    fn parse_string_list_reads_dependencies() {
        assert_eq!(
            parse_string_list(r#"["task_frontend_mock_data"]"#),
            vec!["task_frontend_mock_data".to_string()]
        );
    }
}
