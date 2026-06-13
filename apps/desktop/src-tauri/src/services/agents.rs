use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub model: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_agents(connection: &Connection) -> Result<Vec<AgentSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, name, role, status, model, COALESCE(permissions, '[]'),
                created_at, updated_at
             FROM agents
             ORDER BY role, name",
        )
        .map_err(|error| format!("读取 Agent 列表失败：{error}"))?;

    let rows = statement
        .query_map([], |row| {
            let permissions_json: String = row.get(6)?;

            Ok(AgentSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                role: row.get(3)?,
                status: row.get(4)?,
                model: row.get(5)?,
                permissions: parse_string_list(&permissions_json),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|error| format!("读取 Agent 列表失败：{error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Agent 列表失败：{error}"))
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_string_list;

    #[test]
    fn parse_string_list_returns_empty_vec_for_invalid_json() {
        assert!(parse_string_list("not-json").is_empty());
    }
}
