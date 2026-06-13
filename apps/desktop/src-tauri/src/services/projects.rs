use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub phase: String,
}

pub fn get_current_project(connection: &Connection) -> Result<ProjectSummary, String> {
    connection
        .query_row(
            "SELECT id, name, status, COALESCE(phase, '') FROM projects ORDER BY created_at LIMIT 1",
            [],
            |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status: row.get(2)?,
                    phase: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("读取项目失败：{error}"))?
        .ok_or_else(|| "SQLite 中还没有项目数据".to_string())
}
