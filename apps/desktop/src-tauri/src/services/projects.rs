use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub phase: &'static str,
}

pub fn get_current_project() -> ProjectSummary {
    ProjectSummary {
        id: "project_agent_swarm",
        name: "agent蜂群",
        status: "active",
        phase: "tauri_skeleton",
    }
}
