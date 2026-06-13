use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApprovalSummary {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub request_agent_id: String,
    pub target_service: String,
    pub operation_types: Vec<String>,
    pub status: String,
    pub risk_level: String,
    pub reason: Option<String>,
    pub reject_reason: Option<String>,
    pub approved_at: Option<String>,
    pub rejected_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_approvals(connection: &Connection) -> Result<Vec<ApprovalSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals
             ORDER BY created_at, id",
        )
        .map_err(|error| format!("读取审批列表失败：{error}"))?;

    let rows = statement
        .query_map([], |row| {
            let operation_types_json: String = row.get(5)?;

            Ok(ApprovalSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_id: row.get(2)?,
                request_agent_id: row.get(3)?,
                target_service: row.get(4)?,
                operation_types: parse_string_list(&operation_types_json),
                status: row.get(6)?,
                risk_level: row.get(7)?,
                reason: row.get(8)?,
                reject_reason: row.get(9)?,
                approved_at: row.get(10)?,
                rejected_at: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })
        .map_err(|error| format!("读取审批列表失败：{error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取审批列表失败：{error}"))
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_string_list;

    #[test]
    fn parse_string_list_reads_operation_types() {
        assert_eq!(
            parse_string_list(r#"["file_write","git_checkpoint"]"#),
            vec!["file_write".to_string(), "git_checkpoint".to_string()]
        );
    }
}
