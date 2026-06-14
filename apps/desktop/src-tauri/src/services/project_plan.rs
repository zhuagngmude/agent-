use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{approvals::ApprovalSummary, projects::get_current_project};

const ARCHITECT_AGENT_ID: &str = "agent_architect";
const GENERATED_BY: &str = "local_deterministic_template";
const SAFETY_NOTE: &str =
    "Read-only Runner request. No command execution, file write, network request, or Git change occurs.";

const PROJECT_PLAN_OPERATION_TYPES: [&str; 3] = [
    "project_plan_approval",
    "agent_task_assignment",
    "runner_request_queue",
];

const PROJECT_PLAN_ASSIGNMENTS: [ProjectPlanAssignment; 5] = [
    ProjectPlanAssignment {
        role: "frontend",
        agent_id: "agent_frontend",
        title: "前端交互切片",
        description: "把审批后的项目计划整理为第一版可用 UI 流程和状态展示。",
        priority: "high",
        risk_level: "medium",
        affected_file: "virtual/frontend-plan.md",
        operation_type: "frontend_plan",
    },
    ProjectPlanAssignment {
        role: "backend",
        agent_id: "agent_backend",
        title: "后端状态切片",
        description: "整理本地命令、状态流转和 SQLite 持久化边界。",
        priority: "high",
        risk_level: "medium",
        affected_file: "virtual/backend-plan.md",
        operation_type: "backend_plan",
    },
    ProjectPlanAssignment {
        role: "qa",
        agent_id: "agent_qa",
        title: "验收检查切片",
        description: "整理验收步骤、禁止路径和无副作用检查。",
        priority: "medium",
        risk_level: "low",
        affected_file: "virtual/qa-plan.md",
        operation_type: "qa_plan",
    },
    ProjectPlanAssignment {
        role: "docs",
        agent_id: "agent_docs",
        title: "文档交接切片",
        description: "整理用户文档、AI 维护文档和阶段交接说明。",
        priority: "medium",
        risk_level: "low",
        affected_file: "virtual/docs-plan.md",
        operation_type: "docs_plan",
    },
    ProjectPlanAssignment {
        role: "reviewer",
        agent_id: "agent_reviewer",
        title: "风险审查切片",
        description: "审查任务、只读 Runner request 和阶段边界是否一致。",
        priority: "high",
        risk_level: "medium",
        affected_file: "virtual/review-plan.md",
        operation_type: "review_plan",
    },
];

#[derive(Debug, Deserialize)]
pub struct CreateProjectPlanDraftInput {
    pub idea: String,
    #[serde(default)]
    pub constraints: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveProjectPlanInput {
    pub approval_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProjectPlanDraftSummary {
    pub id: String,
    pub project_id: String,
    pub approval_id: String,
    pub idea: String,
    pub constraints: Option<String>,
    pub summary: String,
    pub status: String,
    pub generated_by: String,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlannedTaskSummary {
    pub id: String,
    pub role: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assigned_agent_id: String,
    pub depends_on: Vec<String>,
    pub risk_level: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RunnerRequestSummary {
    pub id: String,
    pub project_id: String,
    pub approval_id: String,
    pub task_id: String,
    pub status: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
    pub checkpoint: Option<String>,
    pub safety_note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectPlanSideEffects {
    pub writes_project_files: bool,
    pub modifies_git: bool,
    pub executes_runner: bool,
    pub calls_real_model: bool,
    pub reads_raw_secrets: bool,
    pub makes_network_requests: bool,
    pub triggers_agents: bool,
    pub creates_tasks: bool,
    pub creates_runner_requests: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateProjectPlanDraftResponse {
    pub draft: ProjectPlanDraftSummary,
    pub approval: ApprovalSummary,
    pub planned_tasks: Vec<PlannedTaskSummary>,
    pub planned_runner_requests: Vec<RunnerRequestSummary>,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct ApproveProjectPlanResponse {
    pub approval: ApprovalSummary,
    pub draft: ProjectPlanDraftSummary,
    pub created_task_ids: Vec<String>,
    pub created_runner_request_ids: Vec<String>,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Clone, Copy)]
struct ProjectPlanAssignment {
    role: &'static str,
    agent_id: &'static str,
    title: &'static str,
    description: &'static str,
    priority: &'static str,
    risk_level: &'static str,
    affected_file: &'static str,
    operation_type: &'static str,
}

pub fn create_project_plan_draft(
    connection: &mut Connection,
    input: CreateProjectPlanDraftInput,
) -> Result<CreateProjectPlanDraftResponse, String> {
    let project_id = current_project_id(connection)?;
    let idea = normalize_required_text(input.idea, 1, 500, "idea")?;
    let constraints = normalize_optional_text(input.constraints, 2000, "constraints")?;
    let requested_by = normalize_requested_by(input.requested_by)?;

    ensure_required_agents(connection, &project_id)?;

    let slug = normalize_plan_slug(&idea);
    let draft_id = format!("project_plan_{slug}");
    let approval_id = format!("approval_{draft_id}");
    let now = current_timestamp();

    if let Some(draft) = get_draft_by_id(connection, &draft_id)? {
        let approval = get_approval_by_id(connection, &draft.approval_id)?
            .ok_or_else(|| "not_found: project plan approval not found".to_string())?;
        let planned_tasks = build_planned_tasks(&draft);
        let planned_runner_requests =
            build_runner_request_previews(&draft, &planned_tasks, &draft.created_at);

        return Ok(CreateProjectPlanDraftResponse {
            draft,
            approval,
            planned_tasks,
            planned_runner_requests,
            side_effects: side_effects(false, false),
        });
    }

    let summary = format!("本地确定性项目计划：{}", truncate_chars(&idea, 80));
    let operation_types_json = serde_json::to_string(&PROJECT_PLAN_OPERATION_TYPES)
        .map_err(|error| format!("database_error: serialize operation types failed: {error}"))?;
    let reason = format!("请确认项目计划草案：{}", truncate_chars(&idea, 160));

    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start project plan draft transaction failed: {error}")
    })?;

    tx.execute(
        "INSERT INTO approvals (
            id, project_id, task_id, request_agent_id, target_service, operation_types,
            status, risk_level, reason, reject_reason, approved_at, rejected_at,
            created_at, updated_at
        ) VALUES (?1, ?2, NULL, ?3, 'project_plan', ?4, 'pending', 'medium', ?5, NULL, NULL, NULL, ?6, ?6)",
        params![
            approval_id.as_str(),
            project_id.as_str(),
            ARCHITECT_AGENT_ID,
            operation_types_json.as_str(),
            reason.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: create project plan approval failed: {error}"))?;

    tx.execute(
        "INSERT INTO project_plan_drafts (
            id, project_id, approval_id, idea, constraints, summary, status, generated_by,
            requested_by, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, ?9, ?9)",
        params![
            draft_id.as_str(),
            project_id.as_str(),
            approval_id.as_str(),
            idea.as_str(),
            constraints.as_deref(),
            summary.as_str(),
            GENERATED_BY,
            requested_by.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: create project plan draft failed: {error}"))?;

    tx.commit()
        .map_err(|error| format!("database_error: commit project plan draft failed: {error}"))?;

    let draft = get_draft_by_id(connection, &draft_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;
    let approval = get_approval_by_id(connection, &approval_id)?
        .ok_or_else(|| "not_found: project plan approval not found".to_string())?;
    let planned_tasks = build_planned_tasks(&draft);
    let planned_runner_requests = build_runner_request_previews(&draft, &planned_tasks, &now);

    Ok(CreateProjectPlanDraftResponse {
        draft,
        approval,
        planned_tasks,
        planned_runner_requests,
        side_effects: side_effects(false, false),
    })
}

pub fn approve_project_plan(
    connection: &mut Connection,
    input: ApproveProjectPlanInput,
) -> Result<ApproveProjectPlanResponse, String> {
    let project_id = current_project_id(connection)?;
    let approval_id = normalize_required_text(input.approval_id, 1, 200, "approval_id")?;
    ensure_second_confirm(input.second_confirm, input.confirm_text)?;

    let approval = get_approval_by_id(connection, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    if approval.project_id != project_id {
        return Err("not_found: approval not found".to_string());
    }
    if approval.target_service != "project_plan" {
        return Err("invalid_input: approval is not a project_plan approval".to_string());
    }

    let draft = get_draft_by_approval_id(connection, &approval_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;
    if draft.project_id != project_id {
        return Err("not_found: project plan draft not found".to_string());
    }

    let planned_tasks = build_planned_tasks(&draft);
    let planned_runner_requests =
        build_runner_request_previews(&draft, &planned_tasks, &current_timestamp());
    let task_ids: Vec<String> = planned_tasks.iter().map(|task| task.id.clone()).collect();
    let runner_request_ids: Vec<String> = planned_runner_requests
        .iter()
        .map(|request| request.id.clone())
        .collect();

    if draft.status == "instantiated" {
        let approval = get_approval_by_id(connection, &approval_id)?
            .ok_or_else(|| "not_found: approval not found".to_string())?;
        return Ok(ApproveProjectPlanResponse {
            approval,
            draft,
            created_task_ids: task_ids,
            created_runner_request_ids: runner_request_ids,
            side_effects: side_effects(false, false),
        });
    }

    if approval.status != "pending" {
        return Err(format!(
            "invalid_transition: project_plan approval status cannot change from {}",
            approval.status
        ));
    }

    ensure_required_agents(connection, &project_id)?;

    let now = current_timestamp();
    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start approve project plan transaction failed: {error}")
    })?;

    tx.execute(
        "UPDATE approvals
         SET status = 'approved', approved_at = ?1, updated_at = ?1
         WHERE id = ?2 AND project_id = ?3 AND status = 'pending'",
        params![now.as_str(), approval_id.as_str(), project_id.as_str()],
    )
    .map_err(|error| format!("database_error: approve project plan approval failed: {error}"))?;

    tx.execute(
        "UPDATE project_plan_drafts
         SET status = 'instantiated', updated_at = ?1
         WHERE approval_id = ?2 AND project_id = ?3",
        params![now.as_str(), approval_id.as_str(), project_id.as_str()],
    )
    .map_err(|error| format!("database_error: instantiate project plan draft failed: {error}"))?;

    for task in &planned_tasks {
        let depends_on_json = serde_json::to_string(&task.depends_on).map_err(|error| {
            format!("database_error: serialize task dependencies failed: {error}")
        })?;
        tx.execute(
            "INSERT INTO tasks (
                id, project_id, title, description, status, priority, assigned_agent_id,
                depends_on, risk_level, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                task.id.as_str(),
                project_id.as_str(),
                task.title.as_str(),
                task.description.as_str(),
                task.priority.as_str(),
                task.assigned_agent_id.as_str(),
                depends_on_json.as_str(),
                task.risk_level.as_str(),
                now.as_str()
            ],
        )
        .map_err(|error| format!("database_error: create project plan task failed: {error}"))?;
    }

    for request in &planned_runner_requests {
        let operation_types_json =
            serde_json::to_string(&request.operation_types).map_err(|error| {
                format!("database_error: serialize runner request operations failed: {error}")
            })?;
        let affected_files_json =
            serde_json::to_string(&request.affected_files).map_err(|error| {
                format!("database_error: serialize runner request files failed: {error}")
            })?;
        tx.execute(
            "INSERT INTO runner_requests (
                id, project_id, approval_id, task_id, status, operation_types, affected_files,
                checkpoint, safety_note, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6, NULL, ?7, ?8, ?8)",
            params![
                request.id.as_str(),
                project_id.as_str(),
                approval_id.as_str(),
                request.task_id.as_str(),
                operation_types_json.as_str(),
                affected_files_json.as_str(),
                SAFETY_NOTE,
                now.as_str()
            ],
        )
        .map_err(|error| format!("database_error: create runner request failed: {error}"))?;
    }

    let event_id = format!("runtime_event_project_plan_{}", timestamp_nanos());
    tx.execute(
        "INSERT INTO runtime_events (
            id, project_id, entity_type, entity_id, event_type, before_state, after_state,
            actor, reason, created_at
        ) VALUES (?1, ?2, 'project_plan', ?3, 'project_plan_instantiated', 'draft', 'instantiated', ?4, ?5, ?6)",
        params![
            event_id.as_str(),
            project_id.as_str(),
            draft.id.as_str(),
            draft.requested_by.as_str(),
            draft.summary.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: create project plan runtime event failed: {error}"))?;

    tx.commit()
        .map_err(|error| format!("database_error: commit approve project plan failed: {error}"))?;

    let approval = get_approval_by_id(connection, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    let draft = get_draft_by_approval_id(connection, &approval_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    Ok(ApproveProjectPlanResponse {
        approval,
        draft,
        created_task_ids: task_ids,
        created_runner_request_ids: runner_request_ids,
        side_effects: side_effects(true, true),
    })
}

pub fn list_project_plan_drafts(
    connection: &Connection,
) -> Result<Vec<ProjectPlanDraftSummary>, String> {
    let project_id = current_project_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, created_at, updated_at
             FROM project_plan_drafts
             WHERE project_id = ?1
             ORDER BY created_at DESC, id",
        )
        .map_err(|error| format!("database_error: read project plan drafts failed: {error}"))?;

    let rows = statement
        .query_map([project_id.as_str()], map_draft_row)
        .map_err(|error| format!("database_error: read project plan drafts failed: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: read project plan drafts failed: {error}"))
}

pub fn list_runner_requests(connection: &Connection) -> Result<Vec<RunnerRequestSummary>, String> {
    let project_id = current_project_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, approval_id, task_id, status, operation_types, affected_files,
                checkpoint, safety_note, created_at, updated_at
             FROM runner_requests
             WHERE project_id = ?1
             ORDER BY created_at DESC, id",
        )
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))?;

    let rows = statement
        .query_map([project_id.as_str()], map_runner_request_row)
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))
}

fn get_draft_by_id(
    connection: &Connection,
    draft_id: &str,
) -> Result<Option<ProjectPlanDraftSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, created_at, updated_at
             FROM project_plan_drafts
             WHERE id = ?1",
            [draft_id],
            map_draft_row,
        )
        .optional()
        .map_err(|error| format!("database_error: read project plan draft failed: {error}"))
}

fn get_draft_by_approval_id(
    connection: &Connection,
    approval_id: &str,
) -> Result<Option<ProjectPlanDraftSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, created_at, updated_at
             FROM project_plan_drafts
             WHERE approval_id = ?1",
            [approval_id],
            map_draft_row,
        )
        .optional()
        .map_err(|error| format!("database_error: read project plan draft failed: {error}"))
}

fn get_approval_by_id(
    connection: &Connection,
    approval_id: &str,
) -> Result<Option<ApprovalSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals
             WHERE id = ?1",
            [approval_id],
            |row| {
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
            },
        )
        .optional()
        .map_err(|error| format!("database_error: read approval failed: {error}"))
}

fn map_draft_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectPlanDraftSummary> {
    Ok(ProjectPlanDraftSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        approval_id: row.get(2)?,
        idea: row.get(3)?,
        constraints: row.get(4)?,
        summary: row.get(5)?,
        status: row.get(6)?,
        generated_by: row.get(7)?,
        requested_by: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_runner_request_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunnerRequestSummary> {
    let operation_types_json: String = row.get(5)?;
    let affected_files_json: String = row.get(6)?;
    Ok(RunnerRequestSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        approval_id: row.get(2)?,
        task_id: row.get(3)?,
        status: row.get(4)?,
        operation_types: parse_string_list(&operation_types_json),
        affected_files: parse_string_list(&affected_files_json),
        checkpoint: row.get(7)?,
        safety_note: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn build_planned_tasks(draft: &ProjectPlanDraftSummary) -> Vec<PlannedTaskSummary> {
    let frontend_id = task_id_for_plan(&draft.id, "frontend");

    PROJECT_PLAN_ASSIGNMENTS
        .iter()
        .map(|assignment| {
            let task_id = task_id_for_plan(&draft.id, assignment.role);
            let depends_on = if assignment.role == "frontend" {
                Vec::new()
            } else {
                vec![frontend_id.clone()]
            };
            let operation_types = vec![
                "runner_request_readonly".to_string(),
                assignment.operation_type.to_string(),
            ];
            let affected_files = vec![assignment.affected_file.to_string()];

            PlannedTaskSummary {
                id: task_id,
                role: assignment.role.to_string(),
                title: assignment.title.to_string(),
                description: build_task_description(draft, assignment),
                status: "queued".to_string(),
                priority: assignment.priority.to_string(),
                assigned_agent_id: assignment.agent_id.to_string(),
                depends_on,
                risk_level: assignment.risk_level.to_string(),
                operation_types,
                affected_files,
            }
        })
        .collect()
}

fn build_runner_request_previews(
    draft: &ProjectPlanDraftSummary,
    tasks: &[PlannedTaskSummary],
    now: &str,
) -> Vec<RunnerRequestSummary> {
    tasks
        .iter()
        .map(|task| RunnerRequestSummary {
            id: runner_request_id_for_task(&task.id),
            project_id: draft.project_id.clone(),
            approval_id: draft.approval_id.clone(),
            task_id: task.id.clone(),
            status: "queued".to_string(),
            operation_types: task.operation_types.clone(),
            affected_files: task.affected_files.clone(),
            checkpoint: None,
            safety_note: SAFETY_NOTE.to_string(),
            created_at: now.to_string(),
            updated_at: now.to_string(),
        })
        .collect()
}

fn build_task_description(
    draft: &ProjectPlanDraftSummary,
    assignment: &ProjectPlanAssignment,
) -> String {
    let mut description = format!(
        "{}\n\n项目想法：{}\n\n生成方式：{}",
        assignment.description, draft.idea, draft.generated_by
    );
    if let Some(constraints) = draft.constraints.as_deref() {
        description.push_str("\n\n约束：");
        description.push_str(constraints);
    }
    description
}

fn ensure_required_agents(connection: &Connection, project_id: &str) -> Result<(), String> {
    ensure_agent_belongs_to_project(connection, project_id, ARCHITECT_AGENT_ID)?;
    for assignment in PROJECT_PLAN_ASSIGNMENTS {
        ensure_agent_belongs_to_project(connection, project_id, assignment.agent_id)?;
    }
    Ok(())
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
        .map_err(|error| format!("database_error: check project plan agent failed: {error}"))?;

    if count == 1 {
        Ok(())
    } else {
        Err(format!(
            "not_found: project plan agent not found: {agent_id}"
        ))
    }
}

fn ensure_second_confirm(second_confirm: bool, confirm_text: String) -> Result<(), String> {
    let normalized = confirm_text.trim();
    if !second_confirm {
        return Err("invalid_input: second_confirm is required".to_string());
    }
    if !normalized.contains("生成任务") && !normalized.contains("确认生成任务") {
        return Err("invalid_input: confirm_text must contain 生成任务".to_string());
    }
    Ok(())
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

fn normalize_requested_by(value: Option<String>) -> Result<String, String> {
    Ok(normalize_optional_text(value, 120, "requested_by")?
        .unwrap_or_else(|| "local_user".to_string()))
}

fn normalize_plan_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for ch in value.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_separator = false;
        } else if (ch == '-' || ch == '_') && !last_was_separator && !slug.is_empty() {
            slug.push('_');
            last_was_separator = true;
        } else if !last_was_separator && !slug.is_empty() {
            slug.push('_');
            last_was_separator = true;
        }
        if slug.len() >= 64 {
            break;
        }
    }

    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() {
        format!("local_{:x}", stable_hash(value))
    } else {
        slug
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn task_id_for_plan(plan_id: &str, role: &str) -> String {
    format!("task_{plan_id}_{role}")
}

fn runner_request_id_for_task(task_id: &str) -> String {
    format!("runner_request_{task_id}")
}

fn current_project_id(connection: &Connection) -> Result<String, String> {
    get_current_project(connection).map(|project| project.id)
}

fn parse_string_list(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn side_effects(creates_tasks: bool, creates_runner_requests: bool) -> ProjectPlanSideEffects {
    ProjectPlanSideEffects {
        writes_project_files: false,
        modifies_git: false,
        executes_runner: false,
        calls_real_model: false,
        reads_raw_secrets: false,
        makes_network_requests: false,
        triggers_agents: false,
        creates_tasks,
        creates_runner_requests,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
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
    use super::{
        approve_project_plan, create_project_plan_draft, list_project_plan_drafts,
        list_runner_requests, ApproveProjectPlanInput, CreateProjectPlanDraftInput,
    };
    use crate::{db, services::approvals};
    use rusqlite::Connection;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn migration_creates_project_plan_tables_empty() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_draft_creates_pending_project_plan_approval_without_tasks_or_requests() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let before_tasks = count_rows(&connection, "tasks");
            let before_requests = count_rows(&connection, "runner_requests");

            let response = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            assert_eq!(response.approval.status, "pending");
            assert_eq!(response.approval.target_service, "project_plan");
            assert_eq!(response.draft.status, "draft");
            assert_eq!(response.draft.generated_by, "local_deterministic_template");
            assert_eq!(response.planned_tasks.len(), 5);
            assert_eq!(response.planned_runner_requests.len(), 5);
            assert_eq!(count_rows(&connection, "tasks"), before_tasks);
            assert_eq!(count_rows(&connection, "runner_requests"), before_requests);
            assert!(!response.side_effects.creates_tasks);
            assert!(!response.side_effects.creates_runner_requests);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn create_draft_rejects_invalid_input() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let blank = create_project_plan_draft(
                &mut connection,
                CreateProjectPlanDraftInput {
                    idea: "  ".to_string(),
                    constraints: None,
                    requested_by: None,
                },
            )
            .expect_err("blank idea should fail");
            assert!(blank.contains("invalid_input"));

            let long_idea = create_project_plan_draft(
                &mut connection,
                CreateProjectPlanDraftInput {
                    idea: "a".repeat(501),
                    constraints: None,
                    requested_by: None,
                },
            )
            .expect_err("long idea should fail");
            assert!(long_idea.contains("invalid_input"));

            let long_constraints = create_project_plan_draft(
                &mut connection,
                CreateProjectPlanDraftInput {
                    idea: "local todo app".to_string(),
                    constraints: Some("b".repeat(2001)),
                    requested_by: None,
                },
            )
            .expect_err("long constraints should fail");
            assert!(long_constraints.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_project_plan_requires_second_confirmation() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            let error = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: false,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("second confirmation should be required");

            assert!(error.contains("invalid_input"));
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_project_plan_rejects_non_project_plan_approval() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let error = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: "approval_runner_permissions".to_string(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("ordinary approval should not instantiate project plan");

            assert!(error.contains("invalid_input"));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_project_plan_creates_five_tasks_requests_and_runtime_event() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let before_tasks = count_rows(&connection, "tasks");
            let before_events = count_rows(&connection, "runtime_events");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            let response = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "我确认生成任务".to_string(),
                },
            )
            .expect("project plan should be approved");

            assert_eq!(response.approval.status, "approved");
            assert_eq!(response.draft.status, "instantiated");
            assert_eq!(response.created_task_ids.len(), 5);
            assert_eq!(response.created_runner_request_ids.len(), 5);
            assert_eq!(count_rows(&connection, "tasks"), before_tasks + 5);
            assert_eq!(count_rows(&connection, "runner_requests"), 5);
            assert_eq!(count_rows(&connection, "runtime_events"), before_events + 1);
            assert!(response.side_effects.creates_tasks);
            assert!(response.side_effects.creates_runner_requests);

            let requests = list_runner_requests(&connection).expect("runner requests should read");
            assert_eq!(requests.len(), 5);
            assert!(requests.iter().all(|request| request
                .operation_types
                .contains(&"runner_request_readonly".to_string())));
            assert!(requests.iter().all(|request| request.checkpoint.is_none()));
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_project_plan_is_idempotent_after_instantiation() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            let input = ApproveProjectPlanInput {
                approval_id: draft.approval.id,
                second_confirm: true,
                confirm_text: "确认生成任务".to_string(),
            };

            let first = approve_project_plan(&mut connection, input)
                .expect("first approve should instantiate");
            let task_count = count_rows(&connection, "tasks");
            let request_count = count_rows(&connection, "runner_requests");
            let event_count = count_rows(&connection, "runtime_events");

            let second = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: first.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("second approve should be idempotent");

            assert_eq!(second.created_task_ids, first.created_task_ids);
            assert_eq!(
                second.created_runner_request_ids,
                first.created_runner_request_ids
            );
            assert_eq!(count_rows(&connection, "tasks"), task_count);
            assert_eq!(count_rows(&connection, "runner_requests"), request_count);
            assert_eq!(count_rows(&connection, "runtime_events"), event_count);
            assert!(!second.side_effects.creates_tasks);
            assert!(!second.side_effects.creates_runner_requests);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn generic_approve_approval_rejects_project_plan_without_instantiating() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            let error = approvals::approve_approval(
                &mut connection,
                approvals::ApprovalIdInput {
                    id: draft.approval.id,
                },
            )
            .expect_err("generic approval must not approve project_plan");

            assert!(error.contains("approve_project_plan"));
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
            let drafts = list_project_plan_drafts(&connection).expect("drafts should read");
            assert_eq!(drafts[0].status, "draft");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    fn valid_draft_input() -> CreateProjectPlanDraftInput {
        CreateProjectPlanDraftInput {
            idea: "local customer lead tracker".to_string(),
            constraints: Some("SQLite only; no Runner execution".to_string()),
            requested_by: Some("local_user".to_string()),
        }
    }

    fn test_db() -> (db::DbState, std::path::PathBuf) {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-project-plan-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let state = db::initialize(test_dir.clone()).expect("sqlite should initialize");
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
