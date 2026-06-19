use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{approvals::ApprovalSummary, model_catalog, projects::get_current_project};

const ARCHITECT_AGENT_ID: &str = "agent_architect";
const GENERATED_BY: &str = "local_deterministic_template";
const GENERATED_BY_REAL_MODEL: &str = "real_model_preview";
const SAVE_CONFIRM_TEXT: &str = "我确认保存真实模型草案";
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
        title: "生成可打开页面",
        description: "生成 index.html、style.css 和 main.js，做出用户能直接打开查看的第一版页面。",
        priority: "high",
        risk_level: "medium",
        affected_files: &["virtual/index.html", "virtual/style.css", "virtual/main.js"],
        operation_type: "frontend_impl",
    },
    ProjectPlanAssignment {
        role: "backend",
        agent_id: "agent_backend",
        title: "后端状态切片",
        description: "整理本地命令、状态流转和 SQLite 持久化边界。",
        priority: "high",
        risk_level: "medium",
        affected_files: &["virtual/server.js"],
        operation_type: "backend_plan",
    },
    ProjectPlanAssignment {
        role: "qa",
        agent_id: "agent_qa",
        title: "验收检查切片",
        description: "整理验收步骤、禁止路径和无副作用检查。",
        priority: "medium",
        risk_level: "low",
        affected_files: &["virtual/test.js"],
        operation_type: "qa_plan",
    },
    ProjectPlanAssignment {
        role: "docs",
        agent_id: "agent_docs",
        title: "文档交接切片",
        description: "整理用户文档、AI 维护文档和阶段交接说明。",
        priority: "medium",
        risk_level: "low",
        affected_files: &["virtual/README.md"],
        operation_type: "docs_plan",
    },
    ProjectPlanAssignment {
        role: "reviewer",
        agent_id: "agent_reviewer",
        title: "风险审查切片",
        description: "审查任务、只读 Runner request 和阶段边界是否一致。",
        priority: "high",
        risk_level: "medium",
        affected_files: &["virtual/review-plan.md"],
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

pub type AutoGenerateProjectPlanTasksInput = CreateProjectPlanDraftInput;

#[derive(Debug, Deserialize)]
pub struct ApproveProjectPlanInput {
    pub approval_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteProjectPlanDraftInput {
    pub draft_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveProjectPlanModelDraftInput {
    pub idea: String,
    #[serde(default)]
    pub constraints: Option<String>,
    pub audit_record_id: String,
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
    pub model_call_id: Option<String>,
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct DeleteProjectPlanDraftResponse {
    pub deleted_draft_id: String,
    pub deleted_approval_id: String,
    pub side_effects: ProjectPlanSideEffects,
}

// ---------------------------------------------------------------------------
// 阶段 29：任务和只读 Runner request 查看入口
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProjectPlanTaskInstanceSummary {
    pub id: String,
    pub project_id: String,
    pub role: String,
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

#[derive(Debug, Serialize)]
pub struct ProjectPlanExecutionPreview {
    pub draft: ProjectPlanDraftSummary,
    pub approval: ApprovalSummary,
    pub tasks: Vec<ProjectPlanTaskInstanceSummary>,
    pub runner_requests: Vec<RunnerRequestSummary>,
    pub side_effects: ProjectPlanSideEffects,
}

// ---------------------------------------------------------------------------
// 阶段 28：可配置任务角色模板
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectPlanTaskTemplateSummary {
    pub id: String,
    pub project_id: String,
    pub role: String,
    pub agent_id: String,
    pub title: String,
    pub description: String,
    pub priority: String,
    pub risk_level: String,
    pub affected_file: String,
    pub operation_type: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectPlanTaskTemplateInput {
    pub role: String,
    pub enabled: bool,
}

const ALLOWED_ROLES: &[&str] = &[
    "frontend", "backend", "qa", "docs", "reviewer", "security", "devops", "ux", "data",
];
const ALLOWED_PRIORITIES: &[&str] = &["low", "medium", "high"];
const ALLOWED_RISK_LEVELS: &[&str] = &["low", "medium", "high"];
const ALLOWED_OPERATION_TYPES: &[&str] = &[
    "frontend_plan",
    "frontend_impl",
    "backend_plan",
    "backend_impl",
    "qa_plan",
    "qa_test",
    "docs_plan",
    "docs_write",
    "review_plan",
    "code_review",
    "security_review_plan",
    "devops_plan",
    "ux_plan",
    "data_plan",
];

/// 从已落库的 runner_requests 读取真实 task_id 列表，按模板 sort_order 排序（与首次创建顺序一致）
fn get_task_ids_for_approval(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT DISTINCT rr.task_id
             FROM runner_requests rr
             JOIN project_plan_task_templates t
               ON rr.task_id = 'task_' || (SELECT id FROM project_plan_drafts WHERE approval_id = ?2 AND project_id = ?1) || '_' || t.role
             WHERE rr.project_id = ?1 AND rr.approval_id = ?2
             ORDER BY t.sort_order",
        )
        .map_err(|e| format!("database_error: read task ids failed: {e}"))?;
    let ids = stmt
        .query_map(params![project_id, approval_id], |row| row.get(0))
        .map_err(|e| format!("database_error: read task ids failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

/// 从已落库的 runner_requests 读取真实 request_id 列表（与 task 排序一致）
fn get_runner_request_ids_for_approval(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT DISTINCT rr.id
             FROM runner_requests rr
             JOIN project_plan_task_templates t
               ON rr.task_id = 'task_' || (SELECT id FROM project_plan_drafts WHERE approval_id = ?2 AND project_id = ?1) || '_' || t.role
             WHERE rr.project_id = ?1 AND rr.approval_id = ?2
             ORDER BY t.sort_order",
        )
        .map_err(|e| format!("database_error: read runner request ids failed: {e}"))?;
    let ids = stmt
        .query_map(params![project_id, approval_id], |row| row.get(0))
        .map_err(|e| format!("database_error: read runner request ids failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

fn validate_template_field(value: &str, allowed: &[&str], field: &str) -> Result<(), String> {
    if !allowed.contains(&value) {
        return Err(format!("invalid_input: {field} '{value}' is not allowed"));
    }
    Ok(())
}

fn validate_affected_file(path: &str) -> Result<(), String> {
    if !path.starts_with("virtual/") {
        return Err(format!(
            "invalid_input: affected_file must start with virtual/, got: {path}"
        ));
    }
    if path.contains("..") || path.contains('\\') || path.contains(':') || path.contains('~') {
        return Err(format!(
            "invalid_input: affected_file contains forbidden characters: {path}"
        ));
    }
    Ok(())
}

pub fn list_project_plan_task_templates(
    connection: &Connection,
) -> Result<Vec<ProjectPlanTaskTemplateSummary>, String> {
    let project_id = current_project_id(connection)?;
    ensure_templates_seeded(connection, &project_id)?;
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, role, agent_id, title, description,
                priority, risk_level, affected_file, operation_type,
                enabled, sort_order, is_builtin, created_at, updated_at
             FROM project_plan_task_templates
             WHERE project_id = ?1
             ORDER BY sort_order, role",
        )
        .map_err(|e| format!("database_error: list templates failed: {e}"))?;
    let rows = stmt
        .query_map(params![project_id.as_str()], |row| {
            Ok(ProjectPlanTaskTemplateSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                role: row.get(2)?,
                agent_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                priority: row.get(6)?,
                risk_level: row.get(7)?,
                affected_file: row.get(8)?,
                operation_type: row.get(9)?,
                enabled: row.get::<_, i64>(10)? != 0,
                sort_order: row.get(11)?,
                is_builtin: row.get::<_, i64>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|e| format!("database_error: list templates failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list templates failed: {e}"))
}

pub fn update_project_plan_task_template(
    connection: &mut Connection,
    input: UpdateProjectPlanTaskTemplateInput,
) -> Result<Vec<ProjectPlanTaskTemplateSummary>, String> {
    let project_id = current_project_id(connection)?;
    ensure_templates_seeded(connection, &project_id)?;
    // 白名单校验
    let role = normalize_required_text(input.role, 1, 50, "role")?;
    if !ALLOWED_ROLES.contains(&role.as_str()) {
        return Err(format!("invalid_input: unknown role '{role}'"));
    }
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_plan_task_templates WHERE project_id = ?1 AND role = ?2",
            params![project_id.as_str(), role.as_str()],
            |row| row.get(0),
        )
        .map_err(|e| format!("database_error: check template exists failed: {e}"))?;
    if exists == 0 {
        return Err(format!("not_found: template for role '{role}' not found"));
    }
    let now = current_timestamp();
    connection
        .execute(
            "UPDATE project_plan_task_templates SET enabled = ?1, updated_at = ?2
             WHERE project_id = ?3 AND role = ?4",
            params![
                input.enabled as i64,
                now.as_str(),
                project_id.as_str(),
                role.as_str()
            ],
        )
        .map_err(|e| format!("database_error: update template failed: {e}"))?;
    list_project_plan_task_templates(connection)
}

fn ensure_templates_seeded(connection: &Connection, project_id: &str) -> Result<(), String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_plan_task_templates WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("database_error: count templates failed: {e}"))?;
    if count > 0 {
        return Ok(());
    }
    // 表存在但当前项目没有模板 → 重新跑 seed（只 INSERT OR IGNORE）
    crate::db::seed_builtin_task_templates(connection)
        .map_err(|_| "database_error: seed templates failed".to_string())
}

fn validate_template_row(
    connection: &Connection,
    t: &ProjectPlanTaskTemplateSummary,
) -> Result<(), String> {
    validate_template_field(&t.role, ALLOWED_ROLES, "role")?;
    validate_template_field(&t.priority, ALLOWED_PRIORITIES, "priority")?;
    validate_template_field(&t.risk_level, ALLOWED_RISK_LEVELS, "risk_level")?;
    validate_template_field(&t.operation_type, ALLOWED_OPERATION_TYPES, "operation_type")?;
    validate_affected_file(&t.affected_file)?;
    // 校验 agent 存在（旧表 + 新 project_agents 表）
    let agent_exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agents WHERE id = ?1 AND project_id = ?2",
            params![t.agent_id.as_str(), t.project_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|e| format!("database_error: check agent exists failed: {e}"))?;
    let pa_exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE id = ?1 AND project_id = ?2 AND removed_at IS NULL",
            params![t.agent_id.as_str(), t.project_id.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if agent_exists == 0 && pa_exists == 0 {
        return Err(format!(
            "invalid_input: agent '{}' not found for template role '{}'",
            t.agent_id, t.role
        ));
    }
    Ok(())
}

/// 从 DB 读取启用模板，构建 planned tasks 列表
fn build_planned_tasks_from_templates(
    connection: &Connection,
    draft: &ProjectPlanDraftSummary,
) -> Result<Vec<PlannedTaskSummary>, String> {
    let project_id = current_project_id(connection)?;
    ensure_templates_seeded(connection, &project_id)?;
    let templates = list_enabled_templates(connection, &project_id)?;
    if templates.is_empty() {
        return Err(
            "invalid_input: at least one project plan task template must be enabled".to_string(),
        );
    }
    let mut tasks = Vec::new();
    let frontend_exists = templates.iter().any(|t| t.role == "frontend");
    let base_id = &draft.id;
    for t in &templates {
        validate_template_row(connection, t)?;
        let task_id = task_id_for_plan(base_id, &t.role);
        let depends_on = if frontend_exists && t.role != "frontend" {
            vec![task_id_for_plan(base_id, "frontend")]
        } else {
            Vec::new()
        };
        let operation_types = vec![
            "runner_request_write_files".to_string(),
            "runner_request_git_checkpoint".to_string(),
            t.operation_type.clone(),
        ];
        let affected_files = parse_template_affected_files(&t.affected_file)?;
        tasks.push(PlannedTaskSummary {
            id: task_id,
            role: t.role.clone(),
            title: t.title.clone(),
            description: build_task_description_from_template(draft, t),
            status: "queued".to_string(),
            priority: t.priority.clone(),
            assigned_agent_id: t.agent_id.clone(),
            depends_on,
            risk_level: t.risk_level.clone(),
            operation_types,
            affected_files,
        });
    }
    Ok(tasks)
}

fn list_enabled_templates(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ProjectPlanTaskTemplateSummary>, String> {
    let mut stmt = connection
        .prepare(
            "SELECT id, project_id, role, agent_id, title, description,
                priority, risk_level, affected_file, operation_type,
                enabled, sort_order, is_builtin, created_at, updated_at
             FROM project_plan_task_templates
             WHERE project_id = ?1 AND enabled = 1
             ORDER BY sort_order, role",
        )
        .map_err(|e| format!("database_error: list enabled templates failed: {e}"))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok(ProjectPlanTaskTemplateSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                role: row.get(2)?,
                agent_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                priority: row.get(6)?,
                risk_level: row.get(7)?,
                affected_file: row.get(8)?,
                operation_type: row.get(9)?,
                enabled: row.get::<_, i64>(10)? != 0,
                sort_order: row.get(11)?,
                is_builtin: row.get::<_, i64>(12)? != 0,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|e| format!("database_error: list enabled templates failed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("database_error: list enabled templates failed: {e}"))
}

fn build_planned_tasks_fallback(
    connection: &Connection,
    draft: &ProjectPlanDraftSummary,
) -> Vec<PlannedTaskSummary> {
    // 优先使用 AI 生成智能任务（需要显式开启，默认关闭以保证速度）
    let use_ai = std::env::var("AGENT_SWARM_ENABLE_SMART_TASK_GENERATION")
        .unwrap_or_else(|_| "false".to_string());

    if use_ai == "true" {
        // 调用 AI 生成任务
        match generate_smart_tasks_with_ai(connection, draft) {
            Ok(tasks) => return tasks,
            Err(e) => {
                eprintln!("[project_plan] AI 任务生成失败，回退到模板: {}", e);
                // 回退到模板方式
            }
        }
    }

    // 回退：使用模板生成任务
    build_planned_tasks_from_templates(connection, draft)
        .unwrap_or_else(|_| build_planned_tasks(draft))
}

fn build_task_description_from_template(
    draft: &ProjectPlanDraftSummary,
    t: &ProjectPlanTaskTemplateSummary,
) -> String {
    let mut description = format!(
        "{}\n\n项目想法：{}\n\n生成方式：{}",
        t.description, draft.idea, draft.generated_by
    );
    if let Some(constraints) = draft.constraints.as_deref() {
        description.push_str("\n\n约束：");
        description.push_str(constraints);
    }
    if draft.generated_by == "real_model_preview" {
        description.push_str("\n\n真实模型草案摘要：\n");
        description.push_str(&truncate_chars(&draft.summary, 1000));
    }
    description
}

/// 使用 AI 根据项目想法智能生成任务清单
fn generate_smart_tasks_with_ai(
    connection: &Connection,
    draft: &ProjectPlanDraftSummary,
) -> Result<Vec<PlannedTaskSummary>, String> {
    // 1. 调用 AI 模型生成任务清单
    let tasks_json = call_ai_to_generate_tasks(connection, draft)?;

    // 2. 解析 AI 返回的任务清单（JSON 格式）
    let ai_tasks: Vec<AiGeneratedTask> =
        serde_json::from_str(&tasks_json).map_err(|e| format!("AI 返回的任务格式无效: {}", e))?;

    // 3. 转换为 PlannedTaskSummary
    let mut tasks = Vec::new();
    let base_id = &draft.id;

    for (index, ai_task) in ai_tasks.iter().enumerate() {
        let task_id = format!("task_{}_{}", base_id, index + 1);

        // 验证角色是否合法
        validate_template_field(&ai_task.role, ALLOWED_ROLES, "role")?;

        // 验证 agent 是否存在（旧表 + project_agents）
        let agent_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE id = ?1 AND project_id = ?2",
                params![ai_task.agent_id.as_str(), draft.project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| format!("database_error: check agent exists failed: {}", e))?;
        let pa_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_agents WHERE id = ?1 AND project_id = ?2 AND removed_at IS NULL",
                params![ai_task.agent_id.as_str(), draft.project_id.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if agent_exists == 0 && pa_exists == 0 {
            return Err(format!("AI 生成的 agent_id '{}' 不存在", ai_task.agent_id));
        }

        tasks.push(PlannedTaskSummary {
            id: task_id,
            role: ai_task.role.clone(),
            title: ai_task.title.clone(),
            description: ai_task.description.clone(),
            status: "queued".to_string(),
            priority: ai_task.priority.clone(),
            assigned_agent_id: ai_task.agent_id.clone(),
            depends_on: Vec::new(), // TODO: 根据 AI 返回的依赖关系设置
            risk_level: ai_task.risk_level.clone(),
            operation_types: vec![
                "runner_request_write_files".to_string(),
                "runner_request_git_checkpoint".to_string(),
            ],
            affected_files: ai_task.affected_files.clone(),
        });
    }

    Ok(tasks)
}

/// AI 生成的任务结构
#[derive(Debug, Deserialize)]
struct AiGeneratedTask {
    role: String,
    agent_id: String,
    title: String,
    description: String,
    priority: String,
    risk_level: String,
    affected_files: Vec<String>,
}

fn parse_template_affected_files(value: &str) -> Result<Vec<String>, String> {
    let files = value
        .lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err("invalid_input: affected_file must contain at least one path".to_string());
    }
    for file in &files {
        validate_affected_file(file)?;
    }
    Ok(files)
}

/// 调用 AI 模型生成任务清单
fn call_ai_to_generate_tasks(
    connection: &Connection,
    draft: &ProjectPlanDraftSummary,
) -> Result<String, String> {
    // 从环境变量构造 provider
    let provider = crate::services::model_gateway::openai_compat::OpenAiCompatProvider::from_env()
        .map_err(|e| format!("AI provider 初始化失败: {}", e))?;

    // 构造提示词
    let system_prompt = r#"你是一个项目管理助手。根据用户的项目想法，生成合理的任务清单。

要求：
1. 任务数量合理（3-10个），不要生成过多无意义的任务
2. 每个任务要有清晰的标题和详细的描述
3. 任务应该覆盖项目的关键部分（前端、后端、测试、文档等）
4. 返回的必须是合法的 JSON 数组格式

JSON 格式：
[
  {
    "role": "frontend",
    "agent_id": "agent_frontend",
    "title": "任务标题",
    "description": "任务详细描述",
    "priority": "high|medium|low",
    "risk_level": "high|medium|low",
    "affected_files": ["virtual/file1.md", "virtual/file2.js"]
  }
]

注意：
- role 必须是: frontend, backend, qa, docs, reviewer, architect 之一
- agent_id 必须是: agent_frontend, agent_backend, agent_qa, agent_docs, agent_reviewer, agent_architect 之一
- priority 必须是: high, medium, low 之一
- risk_level 必须是: high, medium, low 之一
- affected_files 使用 virtual/ 前缀，表示沙箱内的文件"#.to_string();

    let mut user_message = format!("项目想法：{}\n\n请生成这个项目的任务清单。", draft.idea);

    if let Some(constraints) = &draft.constraints {
        user_message.push_str(&format!("\n\n约束条件：{}", constraints));
    }

    let request = crate::services::model_gateway::openai_compat::ModelRequest {
        system_prompt,
        user_message,
        model_id: runtime_model_id(connection)?,
    };

    // 调用 AI（需要 Box<dyn ModelProvider> 来调用 send 方法）
    let provider_box: Box<dyn crate::services::model_gateway::openai_compat::ModelProvider> =
        Box::new(provider);
    let response = provider_box
        .send(&request, 15, 1024 * 1024)
        .map_err(|e| format!("AI 调用失败: {:?}", e))?;

    Ok(response.content)
}

fn runtime_model_id(connection: &Connection) -> Result<String, String> {
    if let Some(model_id) = std::env::var("AGENT_SWARM_RUNNER_MODEL_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(model_id);
    }

    model_catalog::get_default_model_id(connection).or_else(|_| Ok("deepseek-chat".to_string()))
}

#[derive(Clone, Copy)]
struct ProjectPlanAssignment {
    role: &'static str,
    agent_id: &'static str,
    title: &'static str,
    description: &'static str,
    priority: &'static str,
    risk_level: &'static str,
    affected_files: &'static [&'static str],
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

    if let Some(draft) = get_draft_by_id(connection, &project_id, &draft_id)? {
        if let Some(approval) = get_approval_by_id(connection, &project_id, &draft.approval_id)? {
            let planned_tasks = build_planned_tasks_fallback(connection, &draft);
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

        let runner_request_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM runner_requests WHERE project_id = ?1 AND approval_id = ?2",
                params![project_id.as_str(), draft.approval_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| {
                format!("database_error: count orphan project plan requests failed: {error}")
            })?;
        if runner_request_count > 0 {
            return Err("invalid_state: project plan draft approval is missing".to_string());
        }

        connection
            .execute(
                "DELETE FROM project_plan_drafts WHERE id = ?1 AND project_id = ?2",
                params![draft.id.as_str(), project_id.as_str()],
            )
            .map_err(|error| {
                format!("database_error: remove orphan project plan draft failed: {error}")
            })?;
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
            requested_by, model_call_id, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, NULL, ?9, ?9)",
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

    let draft = get_draft_by_id(connection, &project_id, &draft_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;
    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: project plan approval not found".to_string())?;
    let planned_tasks = build_planned_tasks_fallback(connection, &draft);
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

    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    if approval.target_service != "project_plan" {
        return Err("invalid_input: approval is not a project_plan approval".to_string());
    }

    let draft = get_draft_by_approval_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    // 阶段 27：校验 generated_by 来源
    match draft.generated_by.as_str() {
        "local_deterministic_template" => {}
        "real_model_preview" => {
            // 真实模型草案必须通过安全校验
            if draft
                .model_call_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(
                    "invalid_input: real_model_preview draft must have model_call_id".to_string(),
                );
            }
            if draft.summary.trim().is_empty() {
                return Err("invalid_input: real_model_preview draft summary is empty".to_string());
            }
            // 敏感值拦截
            crate::services::model_gateway::redaction::check_forbidden_value_patterns(
                &draft.summary,
            )?;
            // 校验 model_calls 记录
            let _ = super::model_gateway::model_calls::get_validated_summary(
                connection,
                draft.model_call_id.as_ref().unwrap(),
                &project_id,
            )?;
        }
        other => {
            return Err(format!(
                "invalid_input: unsupported project plan draft source: {other}"
            ));
        }
    }

    // 幂等：如果已实例化，从 DB 读回真实创建的 ID，不重新计算模板
    if draft.status == "instantiated" {
        let approval = get_approval_by_id(connection, &project_id, &approval_id)?
            .ok_or_else(|| "not_found: approval not found".to_string())?;
        let task_ids: Vec<String> =
            get_task_ids_for_approval(connection, &project_id, &approval_id)?;
        let runner_request_ids: Vec<String> =
            get_runner_request_ids_for_approval(connection, &project_id, &approval_id)?;
        if task_ids.is_empty() || runner_request_ids.is_empty() {
            connection
                .execute(
                    "UPDATE project_plan_drafts
                     SET status = 'draft', updated_at = ?1
                     WHERE id = ?2 AND project_id = ?3",
                    params![
                        current_timestamp().as_str(),
                        draft.id.as_str(),
                        project_id.as_str()
                    ],
                )
                .map_err(|error| {
                    format!("database_error: repair empty instantiated draft failed: {error}")
                })?;
        } else {
            return Ok(ApproveProjectPlanResponse {
                approval,
                draft,
                created_task_ids: task_ids,
                created_runner_request_ids: runner_request_ids,
                side_effects: side_effects(false, false),
            });
        }
    }

    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    let draft = get_draft_by_approval_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    if approval.status == "approved" && draft.status == "draft" {
        connection
            .execute(
                "UPDATE approvals
                 SET status = 'pending', approved_at = NULL, updated_at = ?1
                 WHERE id = ?2 AND project_id = ?3",
                params![
                    current_timestamp().as_str(),
                    approval_id.as_str(),
                    project_id.as_str()
                ],
            )
            .map_err(|error| {
                format!("database_error: repair empty instantiated approval failed: {error}")
            })?;
        let approval = get_approval_by_id(connection, &project_id, &approval_id)?
            .ok_or_else(|| "not_found: approval not found".to_string())?;
        return instantiate_project_plan(connection, project_id, approval_id, approval, draft);
    }

    instantiate_project_plan(connection, project_id, approval_id, approval, draft)
}

fn instantiate_project_plan(
    connection: &mut Connection,
    project_id: String,
    approval_id: String,
    approval: ApprovalSummary,
    draft: ProjectPlanDraftSummary,
) -> Result<ApproveProjectPlanResponse, String> {
    if draft.status == "instantiated" {
        return Ok(ApproveProjectPlanResponse {
            approval,
            draft,
            created_task_ids: Vec::new(),
            created_runner_request_ids: Vec::new(),
            side_effects: side_effects(false, false),
        });
    }
    let planned_tasks = build_planned_tasks_from_templates(connection, &draft)?;
    let planned_runner_requests =
        build_runner_request_previews(&draft, &planned_tasks, &current_timestamp());
    let task_ids: Vec<String> = planned_tasks.iter().map(|task| task.id.clone()).collect();
    let runner_request_ids: Vec<String> = planned_runner_requests
        .iter()
        .map(|request| request.id.clone())
        .collect();

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

    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    let draft = get_draft_by_approval_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    Ok(ApproveProjectPlanResponse {
        approval,
        draft,
        created_task_ids: task_ids,
        created_runner_request_ids: runner_request_ids,
        side_effects: side_effects(true, true),
    })
}

pub fn auto_generate_project_plan_tasks(
    connection: &mut Connection,
    input: AutoGenerateProjectPlanTasksInput,
) -> Result<ApproveProjectPlanResponse, String> {
    let draft_response = create_project_plan_draft(
        connection,
        CreateProjectPlanDraftInput {
            idea: input.idea,
            constraints: input.constraints,
            requested_by: input
                .requested_by
                .or_else(|| Some("swarm_auto".to_string())),
        },
    )?;

    approve_project_plan(
        connection,
        ApproveProjectPlanInput {
            approval_id: draft_response.approval.id,
            second_confirm: true,
            confirm_text: "自动生成任务".to_string(),
        },
    )
}

/// 阶段 26：将已验证的真实模型草案保存为待审批项目计划草案。
/// 只写 approvals + project_plan_drafts，不写 tasks / runner_requests / runtime_events。
pub fn save_project_plan_model_draft(
    connection: &mut Connection,
    input: SaveProjectPlanModelDraftInput,
) -> Result<CreateProjectPlanDraftResponse, String> {
    let project_id = current_project_id(connection)?;
    let idea = normalize_required_text(input.idea, 1, 500, "idea")?;
    let constraints = normalize_optional_text(input.constraints, 2000, "constraints")?;

    // 敏感值拦截（阶段 25.3 复用）
    crate::services::model_gateway::redaction::check_forbidden_value_patterns(&idea)?;
    if let Some(ref c) = constraints {
        crate::services::model_gateway::redaction::check_forbidden_value_patterns(c)?;
    }

    // 二次确认校验
    ensure_save_second_confirm(input.second_confirm, &input.confirm_text)?;

    // audit_record_id 存在且安全
    let audit_id = normalize_required_text(input.audit_record_id, 1, 500, "audit_record_id")?;
    let model_summary = super::model_gateway::model_calls::get_validated_summary(
        connection,
        &audit_id,
        &project_id,
    )?;

    // 幂等：同一 audit_record_id 不能重复保存
    if let Some(existing) = find_draft_by_model_call_id(connection, &project_id, &audit_id)? {
        let approval = get_approval_by_id(connection, &project_id, &existing.approval_id)?
            .ok_or_else(|| "not_found: approval not found".to_string())?;
        let planned_tasks = build_planned_tasks(&existing);
        let planned_runner_requests =
            build_runner_request_previews(&existing, &planned_tasks, &existing.created_at);
        return Ok(CreateProjectPlanDraftResponse {
            draft: existing,
            approval,
            planned_tasks,
            planned_runner_requests,
            side_effects: side_effects(false, false),
        });
    }

    // 构造草案和审批 ID（含 audit_record_id 后缀避免同 idea 多次生成冲突）
    let slug = normalize_plan_slug(&idea);
    let audit_suffix = if audit_id.len() > 12 {
        &audit_id[audit_id.len() - 12..]
    } else {
        audit_id.as_str()
    };
    let draft_id = format!("project_plan_model_{slug}_{audit_suffix}");
    let approval_id = format!("approval_{draft_id}");
    let now = current_timestamp();
    let reason = format!(
        "请确认真实模型生成的项目计划草案：{}",
        truncate_chars(&idea, 160)
    );
    let operation_types_json = serde_json::to_string(&PROJECT_PLAN_OPERATION_TYPES)
        .map_err(|error| format!("database_error: serialize operation types failed: {error}"))?;

    ensure_required_agents(connection, &project_id)?;

    let tx = connection.transaction().map_err(|error| {
        format!("database_error: start save model draft transaction failed: {error}")
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
    .map_err(|error| format!("database_error: save model draft approval failed: {error}"))?;

    tx.execute(
        "INSERT INTO project_plan_drafts (
            id, project_id, approval_id, idea, constraints, summary, status, generated_by,
            requested_by, model_call_id, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, ?9, ?10, ?10)",
        params![
            draft_id.as_str(),
            project_id.as_str(),
            approval_id.as_str(),
            idea.as_str(),
            constraints.as_deref(),
            model_summary.as_str(),
            GENERATED_BY_REAL_MODEL,
            "local_user",
            audit_id.as_str(),
            now.as_str()
        ],
    )
    .map_err(|error| format!("database_error: save model draft failed: {error}"))?;

    tx.commit()
        .map_err(|error| format!("database_error: commit save model draft failed: {error}"))?;

    let draft = get_draft_by_id(connection, &project_id, &draft_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;
    let approval = get_approval_by_id(connection, &project_id, &approval_id)?
        .ok_or_else(|| "not_found: project plan approval not found".to_string())?;
    let planned_tasks = build_planned_tasks_fallback(connection, &draft);
    let planned_runner_requests = build_runner_request_previews(&draft, &planned_tasks, &now);

    Ok(CreateProjectPlanDraftResponse {
        draft,
        approval,
        planned_tasks,
        planned_runner_requests,
        side_effects: side_effects(false, false),
    })
}

fn ensure_save_second_confirm(second_confirm: bool, confirm_text: &str) -> Result<(), String> {
    if !second_confirm {
        return Err("invalid_input: second_confirm is required".to_string());
    }
    if confirm_text.trim() != SAVE_CONFIRM_TEXT {
        return Err(format!(
            "invalid_input: confirm_text must be '{SAVE_CONFIRM_TEXT}'"
        ));
    }
    Ok(())
}

fn find_draft_by_model_call_id(
    connection: &Connection,
    project_id: &str,
    model_call_id: &str,
) -> Result<Option<ProjectPlanDraftSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, model_call_id, created_at, updated_at
             FROM project_plan_drafts
             WHERE model_call_id = ?1 AND project_id = ?2",
            params![model_call_id, project_id],
            map_draft_row,
        )
        .optional()
        .map_err(|error| format!("database_error: find draft by model_call_id failed: {error}"))
}

pub fn list_project_plan_drafts(
    connection: &Connection,
) -> Result<Vec<ProjectPlanDraftSummary>, String> {
    let project_id = current_project_id(connection)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, model_call_id, created_at, updated_at
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

pub fn delete_project_plan_draft(
    connection: &mut Connection,
    input: DeleteProjectPlanDraftInput,
) -> Result<DeleteProjectPlanDraftResponse, String> {
    const CONFIRM_TEXT: &str = "我确认删除项目计划草案";
    if !input.second_confirm || input.confirm_text.trim() != CONFIRM_TEXT {
        return Err("invalid_request: second confirmation required".into());
    }

    let project_id = current_project_id(connection)?;
    let draft_id = normalize_required_text(input.draft_id, 1, 200, "draft_id")?;
    let draft = get_draft_by_id(connection, &project_id, &draft_id)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    if draft.status != "draft" {
        return Err("invalid_state: only draft project plans can be deleted".into());
    }

    let approval = get_approval_by_id(connection, &project_id, &draft.approval_id)?
        .ok_or_else(|| "invalid_state: project plan approval is missing".to_string())?;
    if approval.target_service != "project_plan" {
        return Err("invalid_state: linked approval is not a project_plan approval".into());
    }
    if approval.status == "approved" {
        return Err("invalid_state: approved project plan drafts cannot be deleted".into());
    }

    let runner_request_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM runner_requests WHERE project_id = ?1 AND approval_id = ?2",
            params![project_id.as_str(), draft.approval_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check runner requests failed: {error}"))?;
    if runner_request_count > 0 {
        return Err("invalid_state: project plan already has runner requests".into());
    }

    let task_prefix = format!("task_{}_%", draft.id);
    let task_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND id LIKE ?2",
            params![project_id.as_str(), task_prefix.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check tasks failed: {error}"))?;
    if task_count > 0 {
        return Err("invalid_state: project plan already has tasks".into());
    }

    let transaction = connection
        .transaction()
        .map_err(|error| format!("database_error: start delete transaction failed: {error}"))?;
    let deleted_drafts = transaction
        .execute(
            "DELETE FROM project_plan_drafts WHERE id = ?1 AND project_id = ?2",
            params![draft.id.as_str(), project_id.as_str()],
        )
        .map_err(|error| format!("database_error: delete project plan draft failed: {error}"))?;
    if deleted_drafts != 1 {
        return Err("database_error: delete project plan draft affected unexpected rows".into());
    }

    let deleted_approvals = transaction
        .execute(
            "DELETE FROM approvals
             WHERE id = ?1 AND project_id = ?2 AND target_service = 'project_plan' AND status != 'approved'",
            params![draft.approval_id.as_str(), project_id.as_str()],
        )
        .map_err(|error| format!("database_error: delete project plan approval failed: {error}"))?;
    if deleted_approvals != 1 {
        return Err("database_error: delete project plan approval affected unexpected rows".into());
    }

    transaction
        .commit()
        .map_err(|error| format!("database_error: commit delete transaction failed: {error}"))?;

    Ok(DeleteProjectPlanDraftResponse {
        deleted_draft_id: draft.id,
        deleted_approval_id: draft.approval_id,
        side_effects: side_effects(false, false),
    })
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

/// 阶段 29：按 approval_id 查看审批后真实落库的 tasks 和只读 runner_requests。
/// 只读，不写任何表、不重新计算模板。
pub fn get_project_plan_execution_preview(
    connection: &Connection,
    approval_id: String,
) -> Result<ProjectPlanExecutionPreview, String> {
    let project_id = current_project_id(connection)?;
    let aid = normalize_required_text(approval_id, 1, 200, "approval_id")?;

    let approval = get_approval_by_id(connection, &project_id, &aid)?
        .ok_or_else(|| "not_found: approval not found".to_string())?;
    if approval.target_service != "project_plan" {
        return Err("invalid_input: approval is not a project_plan approval".into());
    }
    let draft = get_draft_by_approval_id(connection, &project_id, &aid)?
        .ok_or_else(|| "not_found: project plan draft not found".to_string())?;

    // 未实例化：返回空列表
    if draft.status != "instantiated" {
        return Ok(ProjectPlanExecutionPreview {
            draft,
            approval,
            tasks: vec![],
            runner_requests: vec![],
            side_effects: side_effects(false, false),
        });
    }

    // 从 runner_requests 反查已落库的真实结果
    let runner_requests = list_runner_requests_for_approval(connection, &project_id, &aid)?;
    let mut tasks = Vec::new();
    for req in &runner_requests {
        let task = get_task_by_id(connection, &project_id, &req.task_id)?
            .ok_or_else(|| "database_error: runner request task is missing".to_string())?;
        tasks.push(task);
    }

    Ok(ProjectPlanExecutionPreview {
        draft,
        approval,
        tasks,
        runner_requests,
        side_effects: side_effects(false, false),
    })
}

fn list_runner_requests_for_approval(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Vec<RunnerRequestSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, approval_id, task_id, status, operation_types, affected_files,
                checkpoint, safety_note, created_at, updated_at
             FROM runner_requests
             WHERE project_id = ?1 AND approval_id = ?2
             ORDER BY created_at, id",
        )
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))?;
    let rows = statement
        .query_map(params![project_id, approval_id], map_runner_request_row)
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("database_error: read runner requests failed: {error}"))
}

fn get_task_by_id(
    connection: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<Option<ProjectPlanTaskInstanceSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, title, description, status, priority, assigned_agent_id,
                COALESCE(depends_on, '[]'), risk_level, created_at, updated_at
             FROM tasks WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |row| {
                let depends_on_json: String = row.get(7)?;
                Ok(ProjectPlanTaskInstanceSummary {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    role: parse_role_from_task_id(row.get::<_, String>(0)?.as_str()),
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
        .map_err(|error| format!("database_error: read task failed: {error}"))
}

/// 从 task id 解析 role。task id 格式：task_{plan_draft_id}_{role}
fn parse_role_from_task_id(task_id: &str) -> String {
    // 找到最后一个 '_' 之后的部分作为 role
    task_id.rsplit('_').next().unwrap_or("unknown").to_string()
}

fn get_draft_by_id(
    connection: &Connection,
    project_id: &str,
    draft_id: &str,
) -> Result<Option<ProjectPlanDraftSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, model_call_id, created_at, updated_at
             FROM project_plan_drafts
             WHERE id = ?1 AND project_id = ?2",
            params![draft_id, project_id],
            map_draft_row,
        )
        .optional()
        .map_err(|error| format!("database_error: read project plan draft failed: {error}"))
}

fn get_draft_by_approval_id(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Option<ProjectPlanDraftSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, approval_id, idea, constraints, summary, status,
                generated_by, requested_by, model_call_id, created_at, updated_at
             FROM project_plan_drafts
             WHERE approval_id = ?1 AND project_id = ?2",
            params![approval_id, project_id],
            map_draft_row,
        )
        .optional()
        .map_err(|error| format!("database_error: read project plan draft failed: {error}"))
}

fn get_approval_by_id(
    connection: &Connection,
    project_id: &str,
    approval_id: &str,
) -> Result<Option<ApprovalSummary>, String> {
    connection
        .query_row(
            "SELECT id, project_id, task_id, request_agent_id, target_service,
                operation_types, status, risk_level, reason, reject_reason, approved_at,
                rejected_at, created_at, updated_at
             FROM approvals
             WHERE id = ?1 AND project_id = ?2",
            params![approval_id, project_id],
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
        model_call_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
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
                "runner_request_write_files".to_string(),
                "runner_request_git_checkpoint".to_string(),
                assignment.operation_type.to_string(),
            ];
            let affected_files = assignment
                .affected_files
                .iter()
                .map(|file| file.to_string())
                .collect::<Vec<_>>();

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
    // 阶段 27：真实模型草案追加安全摘要（已脱敏截断，不再调模型）
    if draft.generated_by == "real_model_preview" {
        description.push_str("\n\n真实模型草案摘要：\n");
        description.push_str(&truncate_chars(&draft.summary, 1000));
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
    // 先查旧 agents 表（向后兼容）
    let old_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM agents WHERE id = ?1 AND project_id = ?2",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("database_error: check project plan agent failed: {error}"))?;

    if old_count == 1 {
        return Ok(());
    }

    // 再查 project_agents 表（新 P0 数据源；表不存在时视为 0）
    let pa_count = connection
        .query_row(
            "SELECT COUNT(*) FROM project_agents WHERE id = ?1 AND project_id = ?2 AND removed_at IS NULL",
            params![agent_id, project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if pa_count == 1 {
        return Ok(());
    }

    Err(format!(
        "not_found: project plan agent not found: {agent_id}"
    ))
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
        approve_project_plan, auto_generate_project_plan_tasks, create_project_plan_draft,
        delete_project_plan_draft, get_project_plan_execution_preview, list_project_plan_drafts,
        list_project_plan_task_templates, list_runner_requests, save_project_plan_model_draft,
        update_project_plan_task_template, ApproveProjectPlanInput, CreateProjectPlanDraftInput,
        DeleteProjectPlanDraftInput, SaveProjectPlanModelDraftInput,
        UpdateProjectPlanTaskTemplateInput,
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
            assert_eq!(response.planned_tasks.len(), 2);
            assert_eq!(response.planned_runner_requests.len(), 2);
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
    fn approve_project_plan_creates_default_tasks_requests_and_runtime_event() {
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
            assert_eq!(response.created_task_ids.len(), 2);
            assert_eq!(response.created_runner_request_ids.len(), 2);
            assert_eq!(count_rows(&connection, "tasks"), before_tasks + 2);
            assert_eq!(count_rows(&connection, "runner_requests"), 2);
            assert_eq!(count_rows(&connection, "runtime_events"), before_events + 1);
            assert!(response.side_effects.creates_tasks);
            assert!(response.side_effects.creates_runner_requests);

            let requests = list_runner_requests(&connection).expect("runner requests should read");
            assert_eq!(requests.len(), 2);
            assert!(requests.iter().all(|request| request
                .operation_types
                .contains(&"runner_request_write_files".to_string())));
            assert!(requests.iter().all(|request| request.checkpoint.is_none()));
            let frontend_request = requests
                .iter()
                .find(|request| request.task_id.ends_with("_frontend"))
                .expect("frontend request should exist");
            assert_eq!(
                frontend_request.affected_files,
                vec![
                    "virtual/index.html".to_string(),
                    "virtual/style.css".to_string(),
                    "virtual/main.js".to_string()
                ]
            );
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn auto_generate_project_plan_tasks_instantiates_without_ui_confirmation() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let before_tasks = count_rows(&connection, "tasks");
            let response = auto_generate_project_plan_tasks(
                &mut connection,
                CreateProjectPlanDraftInput {
                    idea: "做一个本地运行的轻量待办工具".to_string(),
                    constraints: Some("自动生成角色任务，不执行 Runner".to_string()),
                    requested_by: None,
                },
            )
            .expect("auto task generation should instantiate plan");

            assert_eq!(response.approval.status, "approved");
            assert_eq!(response.draft.status, "instantiated");
            assert_eq!(response.draft.requested_by, "swarm_auto");
            assert_eq!(response.created_task_ids.len(), 2);
            assert_eq!(response.created_runner_request_ids.len(), 2);
            assert_eq!(count_rows(&connection, "tasks"), before_tasks + 2);
            assert!(response.side_effects.creates_tasks);
            assert!(response.side_effects.creates_runner_requests);
            assert!(!response.side_effects.executes_runner);
            assert!(!response.side_effects.writes_project_files);
            assert!(!response.side_effects.modifies_git);
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
    fn delete_project_plan_draft_removes_draft_and_pending_approval() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let response = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            let deleted = delete_project_plan_draft(
                &mut connection,
                DeleteProjectPlanDraftInput {
                    draft_id: response.draft.id.clone(),
                    second_confirm: true,
                    confirm_text: "我确认删除项目计划草案".to_string(),
                },
            )
            .expect("draft should be deleted");

            assert_eq!(deleted.deleted_draft_id, response.draft.id);
            assert_eq!(deleted.deleted_approval_id, response.approval.id);
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
            assert_eq!(
                count_rows_where(&connection, "approvals", "target_service = 'project_plan'"),
                0
            );
            assert!(!deleted.side_effects.creates_tasks);
            assert!(!deleted.side_effects.creates_runner_requests);
            assert!(!deleted.side_effects.executes_runner);
            assert!(!deleted.side_effects.writes_project_files);
            assert!(!deleted.side_effects.modifies_git);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn delete_project_plan_draft_requires_second_confirmation() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let response = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            let error = delete_project_plan_draft(
                &mut connection,
                DeleteProjectPlanDraftInput {
                    draft_id: response.draft.id,
                    second_confirm: true,
                    confirm_text: "delete".to_string(),
                },
            )
            .expect_err("wrong confirm text should fail");

            assert!(error.contains("invalid_request"));
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 1);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn delete_project_plan_draft_rejects_instantiated_draft() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let response = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: response.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("project plan should instantiate");

            let error = delete_project_plan_draft(
                &mut connection,
                DeleteProjectPlanDraftInput {
                    draft_id: response.draft.id,
                    second_confirm: true,
                    confirm_text: "我确认删除项目计划草案".to_string(),
                },
            )
            .expect_err("instantiated draft should not delete");

            assert!(error.contains("invalid_state"));
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 1);
            assert_eq!(count_rows(&connection, "runner_requests"), 2);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn delete_project_plan_draft_rejects_unknown_draft() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let error = delete_project_plan_draft(
                &mut connection,
                DeleteProjectPlanDraftInput {
                    draft_id: "missing_draft".to_string(),
                    second_confirm: true,
                    confirm_text: "我确认删除项目计划草案".to_string(),
                },
            )
            .expect_err("missing draft should fail");

            assert!(error.contains("not_found"));
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

    #[test]
    fn cross_project_draft_not_found_from_another_project() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");

            // 在当前项目中创建一个 draft
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");

            // 记住当前项目 ID，然后插入另一个 project+审批+草案
            let pid = super::current_project_id(&connection).expect("current project id");

            connection
                .execute(
                    "INSERT INTO projects (id, name, status, phase, created_at, updated_at)
                     VALUES ('other_project', 'Other', 'planning', 'init', '2099-01-01', '2099-01-01')",
                    [],
                )
                .expect("other project insert should succeed");
            connection
                .execute(
                    "INSERT INTO approvals (
                        id, project_id, task_id, request_agent_id, target_service, operation_types,
                        status, risk_level, reason, created_at, updated_at
                    ) VALUES (
                        'approval_other', 'other_project', NULL, 'agent_architect', 'project_plan',
                        '[]', 'pending', 'medium', 'test', '2025-01-01', '2025-01-01'
                    )",
                    [],
                )
                .expect("other project approval insert should succeed");
            connection
                .execute(
                    "INSERT INTO project_plan_drafts (
                        id, project_id, approval_id, idea, summary, status,
                        generated_by, requested_by, created_at, updated_at
                    ) VALUES (
                        'project_plan_other_draft', 'other_project', 'approval_other',
                        'other idea', 'other summary', 'draft',
                        'local_deterministic_template', 'test', '2025-01-01', '2025-01-01'
                    )",
                    [],
                )
                .expect("other project draft insert should succeed");

            // 用当前 project_id 查另一个 project 的 draft —— 应返回 None
            let other_draft = super::get_draft_by_id(&connection, &pid, "project_plan_other_draft")
                .expect("query should succeed");
            assert!(
                other_draft.is_none(),
                "draft from another project should not be found"
            );

            // 自己的 draft 仍然可以用 project_id 查到
            let own_draft = super::get_draft_by_id(&connection, &pid, &draft.draft.id)
                .expect("query should succeed");
            assert!(own_draft.is_some(), "own draft should still be found");
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 阶段 26：save_project_plan_model_draft 测试
    // -------------------------------------------------------

    fn insert_test_model_call(connection: &Connection, audit_id: &str) {
        // 插入一条安全的 success model_call 用于保存
        connection
            .execute(
                "INSERT INTO model_calls (
                    id, project_id, purpose, provider, model, status,
                    structured_summary, error_category, redaction_applied,
                    token_usage, cost_estimate, created_at, updated_at
                ) VALUES (?1, 'project_agent_swarm', 'project_plan_generation', 'openai_compat',
                    'gpt-5.4-mini', 'success', ?2, NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                rusqlite::params![audit_id, "测试模型摘要"],
            )
            .expect("should insert test model_call");
    }

    fn valid_save_input(audit_id: &str) -> SaveProjectPlanModelDraftInput {
        SaveProjectPlanModelDraftInput {
            idea: "测试保存模型草案".to_string(),
            constraints: Some("无约束".to_string()),
            audit_record_id: audit_id.to_string(),
            second_confirm: true,
            confirm_text: "我确认保存真实模型草案".to_string(),
        }
    }

    #[test]
    fn save_success_creates_draft_and_approval_without_tasks_or_runner_requests() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_test_save_001";
            insert_test_model_call(&connection, audit_id);

            let before_tasks = count_rows(&connection, "tasks");
            let before_requests = count_rows(&connection, "runner_requests");
            let before_events = count_rows(&connection, "runtime_events");

            let response =
                save_project_plan_model_draft(&mut connection, valid_save_input(audit_id))
                    .expect("save should succeed");

            assert_eq!(response.approval.status, "pending");
            assert_eq!(response.approval.target_service, "project_plan");
            assert_eq!(response.draft.status, "draft");
            assert_eq!(response.draft.generated_by, "real_model_preview");
            assert_eq!(response.draft.summary, "测试模型摘要");
            assert_eq!(response.draft.model_call_id.as_deref(), Some(audit_id));
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 1);
            assert_eq!(count_rows(&connection, "approvals"), 4);
            assert!(!response.side_effects.creates_tasks);
            assert!(!response.side_effects.creates_runner_requests);

            // 未写入 tasks / runner_requests / runtime_events
            assert_eq!(count_rows(&connection, "tasks"), before_tasks);
            assert_eq!(count_rows(&connection, "runner_requests"), before_requests);
            assert_eq!(count_rows(&connection, "runtime_events"), before_events);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_requires_second_confirm() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            insert_test_model_call(&connection, "model_call_test_save_002");
            let before = count_rows(&connection, "approvals");

            let err = save_project_plan_model_draft(
                &mut connection,
                SaveProjectPlanModelDraftInput {
                    second_confirm: false,
                    confirm_text: "我确认保存真实模型草案".into(),
                    ..valid_save_input("model_call_test_save_002")
                },
            )
            .expect_err("second_confirm=false should fail");

            assert!(err.contains("invalid_input"));
            assert_eq!(count_rows(&connection, "approvals"), before);
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_requires_correct_confirm_text() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            insert_test_model_call(&connection, "model_call_test_save_003");

            let err = save_project_plan_model_draft(
                &mut connection,
                SaveProjectPlanModelDraftInput {
                    confirm_text: "随便".into(),
                    ..valid_save_input("model_call_test_save_003")
                },
            )
            .expect_err("wrong confirm_text should fail");

            assert!(err.contains("invalid_input"));
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_nonexistent_audit_record() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_nonexistent"),
            )
            .expect_err("nonexistent audit_record_id should fail");

            assert!(err.contains("not_found"));
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_failed_model_call() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 插入一条 failed 状态的 model_call
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_test_failed', 'project_agent_swarm',
                        'project_plan_generation', 'openai_compat', 'gpt-5.4-mini',
                        'failed', 'timeout', 0, '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert failed model_call");

            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_test_failed"),
            )
            .expect_err("failed model_call should not be saved");

            assert!(err.contains("must be success"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_model_call_with_null_summary() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_null_summary', 'project_agent_swarm',
                        'project_plan_generation', 'openai_compat', 'gpt-5.4-mini',
                        'success', NULL, NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert model_call with null summary");

            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_null_summary"),
            )
            .expect_err("null summary should be rejected");

            assert!(err.contains("no summary"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_cross_project_model_call() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 插入另一项目的 model_call
            connection
                .execute(
                    "INSERT INTO projects (id, name, status, created_at, updated_at)
                     VALUES ('other_proj', 'Other', 'planning', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert other project");
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_other_proj', 'other_proj',
                        'project_plan_generation', 'openai_compat', 'gpt-5.4-mini',
                        'success', '另一个项目的摘要', NULL, 1,
                        '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert cross-project model_call");

            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_other_proj"),
            )
            .expect_err("cross-project should fail");

            // 无论 LIMIT 1 返回哪个 project，跨项目 model_call 一定会被拒绝
            assert!(
                err.contains("does not belong") || err.contains("not_found"),
                "expected cross-project rejection, got: {err}"
            );
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_sensitive_patterns_in_idea() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            insert_test_model_call(&connection, "model_call_test_sensitive");

            let err = save_project_plan_model_draft(
                &mut connection,
                SaveProjectPlanModelDraftInput {
                    idea: "sk-abcdefghijklmnopqrstuvwxyz123456 的项目".into(),
                    ..valid_save_input("model_call_test_sensitive")
                },
            )
            .expect_err("sensitive input should be rejected");

            assert!(err.contains("API key"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_model_call_with_wrong_purpose() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 插入一条 purpose != project_plan_generation 的成功记录
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_wrong_purpose', 'project_agent_swarm',
                        'task_breakdown', 'openai_compat', 'gpt-5.4-mini',
                        'success', '摘要', NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert model_call with wrong purpose");

            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_wrong_purpose"),
            )
            .expect_err("wrong purpose should be rejected");

            assert!(err.contains("must be project_plan_generation"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_model_call_with_wrong_provider() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_wrong_provider', 'project_agent_swarm',
                        'project_plan_generation', 'deepseek', 'gpt-5.4-mini',
                        'success', '摘要', NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert model_call with wrong provider");

            let err = save_project_plan_model_draft(
                &mut connection,
                valid_save_input("model_call_wrong_provider"),
            )
            .expect_err("wrong provider should be rejected");

            assert!(err.contains("must be openai_compat"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_is_idempotent_for_same_model_call_id() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_test_idempotent";
            insert_test_model_call(&connection, audit_id);

            let first = save_project_plan_model_draft(&mut connection, valid_save_input(audit_id))
                .expect("first save should succeed");

            let approval_count = count_rows(&connection, "approvals");
            let draft_count = count_rows(&connection, "project_plan_drafts");

            let second = save_project_plan_model_draft(&mut connection, valid_save_input(audit_id))
                .expect("second save should be idempotent");

            assert_eq!(second.approval.id, first.approval.id);
            assert_eq!(second.draft.id, first.draft.id);
            assert_eq!(count_rows(&connection, "approvals"), approval_count);
            assert_eq!(count_rows(&connection, "project_plan_drafts"), draft_count);
            assert!(!second.side_effects.creates_tasks);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 阶段 35 返修：get_validated_summary 拒绝污染 model 字段
    // -------------------------------------------------------

    fn insert_model_call_with_model(connection: &Connection, audit_id: &str, model: &str) {
        connection
            .execute(
                "INSERT INTO model_calls (
                    id, project_id, purpose, provider, model, status,
                    structured_summary, error_category, redaction_applied,
                    token_usage, cost_estimate, created_at, updated_at
                ) VALUES (?1, 'project_agent_swarm', 'project_plan_generation', 'openai_compat',
                    ?2, 'success', ?3, NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                rusqlite::params![audit_id, model, "测试摘要"],
            )
            .expect("should insert test model_call");
    }

    #[test]
    fn save_rejects_model_call_with_url_model_name() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            insert_model_call_with_model(&connection, "model_call_url_model", "https://evil.com");

            let err = save_project_plan_model_draft(
                &mut connection,
                SaveProjectPlanModelDraftInput {
                    audit_record_id: "model_call_url_model".into(),
                    ..valid_save_input("model_call_url_model")
                },
            )
            .expect_err("model name 'https://evil.com' should be rejected");

            assert!(
                err.contains("model validation failed") || err.contains("invalid_input"),
                "should reject polluted model name, got: {err}"
            );
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn save_rejects_model_call_with_sk_prefix_model_name() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            insert_model_call_with_model(&connection, "model_call_sk_model", "sk-test-model");

            let err = save_project_plan_model_draft(
                &mut connection,
                SaveProjectPlanModelDraftInput {
                    audit_record_id: "model_call_sk_model".into(),
                    ..valid_save_input("model_call_sk_model")
                },
            )
            .expect_err("model name 'sk-test-model' should be rejected");

            assert!(
                err.contains("model validation failed") || err.contains("invalid_input"),
                "should reject polluted model name, got: {err}"
            );
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 阶段 27：approve_project_plan 真实模型草案审批测试
    // -------------------------------------------------------

    fn save_and_get_approval_id(connection: &mut rusqlite::Connection, audit_id: &str) -> String {
        let response = save_project_plan_model_draft(
            connection,
            SaveProjectPlanModelDraftInput {
                idea: "真实模型测试项目".to_string(),
                constraints: None,
                audit_record_id: audit_id.to_string(),
                second_confirm: true,
                confirm_text: "我确认保存真实模型草案".to_string(),
            },
        )
        .expect("save should succeed");
        response.approval.id
    }

    #[test]
    fn approve_real_model_preview_draft_creates_tasks_and_requests() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_real_approve_001";
            insert_test_model_call(&connection, audit_id);
            let approval_id = save_and_get_approval_id(&mut connection, audit_id);

            let before_tasks = count_rows(&connection, "tasks");
            let before_requests = count_rows(&connection, "runner_requests");
            let before_events = count_rows(&connection, "runtime_events");
            let before_model_calls = count_rows(&connection, "model_calls");

            let response = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: approval_id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve real_model_preview should succeed");

            assert_eq!(response.approval.status, "approved");
            assert_eq!(response.draft.status, "instantiated");
            assert_eq!(response.draft.generated_by, "real_model_preview");
            assert_eq!(response.created_task_ids.len(), 2);
            assert_eq!(response.created_runner_request_ids.len(), 2);
            assert_eq!(count_rows(&connection, "tasks"), before_tasks + 2);
            assert_eq!(
                count_rows(&connection, "runner_requests"),
                before_requests + 2
            );
            assert_eq!(count_rows(&connection, "runtime_events"), before_events + 1);
            assert_eq!(count_rows(&connection, "model_calls"), before_model_calls);
            assert!(response.side_effects.creates_tasks);
            assert!(response.side_effects.creates_runner_requests);
            assert!(!response.side_effects.calls_real_model);
            assert!(!response.side_effects.executes_runner);
            assert!(!response.side_effects.writes_project_files);
            assert!(!response.side_effects.modifies_git);

            // 验证任务描述包含模型摘要
            let tasks = list_tasks_for_approval(&connection, &approval_id);
            assert_eq!(tasks.len(), 2);
            assert!(tasks.iter().any(|t| t
                .description
                .as_deref()
                .unwrap_or("")
                .contains("真实模型草案摘要")));
            assert!(tasks.iter().any(|t| t
                .description
                .as_deref()
                .unwrap_or("")
                .contains("测试模型摘要")));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_preview_does_not_call_model() {
        // 证明 approve 不依赖 provider trait / env / model_calls 写入
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_no_reinvoke";
            insert_test_model_call(&connection, audit_id);
            let approval_id = save_and_get_approval_id(&mut connection, audit_id);
            let before = count_rows(&connection, "model_calls");

            // 无需设置任何 ENV
            approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve should work without model env");

            assert_eq!(count_rows(&connection, "model_calls"), before);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_draft_without_model_call_id_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 先正常保存，再 UPDATE 把 model_call_id 清掉
            let audit_id = "model_call_no_mc_test";
            insert_test_model_call(&connection, audit_id);
            let approval_id = save_and_get_approval_id(&mut connection, audit_id);
            connection
                .execute(
                    "UPDATE project_plan_drafts SET model_call_id = NULL WHERE approval_id = ?1",
                    rusqlite::params![approval_id.as_str()],
                )
                .expect("should update draft");
            let before_tasks = count_rows(&connection, "tasks");

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: approval_id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("missing model_call_id should be rejected");

            assert!(
                err.contains("must have model_call_id"),
                "unexpected error: {err}"
            );
            assert_eq!(count_rows(&connection, "tasks"), before_tasks);
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_draft_with_nonexistent_model_call_id_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 正常保存后，把 model_call_id 指向不存在的记录
            let audit_id = "model_call_real_for_bad";
            insert_test_model_call(&connection, audit_id);
            let approval_id = save_and_get_approval_id(&mut connection, audit_id);
            connection
                .execute(
                    "UPDATE project_plan_drafts SET model_call_id = 'model_call_nonexistent' WHERE approval_id = ?1",
                    rusqlite::params![approval_id.as_str()],
                )
                .expect("should update draft");

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: approval_id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("nonexistent model_call_id should be rejected");

            assert!(err.contains("not_found"));
            assert_eq!(count_rows(&connection, "tasks"), 4); // seed only
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_draft_with_failed_model_call_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 插入 failed model_call，然后手动插入对应的 approval + draft
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES ('model_call_failed_for_approve', 'project_agent_swarm',
                        'project_plan_generation', 'openai_compat', 'gpt-5.4-mini',
                        'failed', '摘要', 'timeout', 0, '{}', '{}', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert failed model_call");
            connection
                .execute(
                    "INSERT INTO approvals (
                        id, project_id, task_id, request_agent_id, target_service,
                        operation_types, status, risk_level, reason, reject_reason,
                        approved_at, rejected_at, created_at, updated_at
                    ) VALUES ('approval_draft_failed_mc', 'project_agent_swarm', NULL,
                        'agent_architect', 'project_plan', '[]', 'pending', 'medium',
                        'test', NULL, NULL, NULL, '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert approval");
            connection
                .execute(
                    "INSERT INTO project_plan_drafts (
                        id, project_id, approval_id, idea, constraints, summary, status,
                        generated_by, requested_by, model_call_id, created_at, updated_at
                    ) VALUES ('draft_failed_mc', 'project_agent_swarm',
                        'approval_draft_failed_mc', 'idea', NULL, 'summary', 'draft',
                        'real_model_preview', 'local_user',
                        'model_call_failed_for_approve',
                        '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert draft");

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: "approval_draft_failed_mc".to_string(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("failed model_call should be rejected");

            assert!(err.contains("must be success"), "unexpected error: {err}");
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_draft_with_sensitive_summary_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_sensitive_summary";
            // 手动插入 model_call + 含敏感内容的草案
            connection
                .execute(
                    "INSERT INTO model_calls (
                        id, project_id, purpose, provider, model, status,
                        structured_summary, error_category, redaction_applied,
                        token_usage, cost_estimate, created_at, updated_at
                    ) VALUES (?1, 'project_agent_swarm',
                        'project_plan_generation', 'openai_compat', 'gpt-5.4-mini',
                        'success', '包含 sk-abcdefghijklmnopqrstuvwxyz123456 的摘要',
                        NULL, 1, '{}', '{}', '2025-01-01', '2025-01-01')",
                    rusqlite::params![audit_id],
                )
                .expect("should insert");
            connection
                .execute(
                    "INSERT INTO approvals (
                        id, project_id, task_id, request_agent_id, target_service,
                        operation_types, status, risk_level, reason,
                        created_at, updated_at
                    ) VALUES ('approval_sensitive', 'project_agent_swarm', NULL,
                        'agent_architect', 'project_plan', '[]', 'pending', 'medium',
                        'test', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert approval");
            connection
                .execute(
                    "INSERT INTO project_plan_drafts (
                        id, project_id, approval_id, idea, summary, status,
                        generated_by, requested_by, model_call_id, created_at, updated_at
                    ) VALUES ('draft_sensitive', 'project_agent_swarm',
                        'approval_sensitive', 'idea',
                        '包含 sk-abcdefghijklmnopqrstuvwxyz123456 的摘要',
                        'draft', 'real_model_preview', 'local_user', ?1,
                        '2025-01-01', '2025-01-01')",
                    rusqlite::params![audit_id],
                )
                .expect("should insert draft");

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: "approval_sensitive".to_string(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("sensitive summary should be rejected");

            assert!(err.contains("API key"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_unknown_generated_by_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "INSERT INTO approvals (
                        id, project_id, task_id, request_agent_id, target_service,
                        operation_types, status, risk_level, reason,
                        created_at, updated_at
                    ) VALUES ('approval_unknown', 'project_agent_swarm', NULL,
                        'agent_architect', 'project_plan', '[]', 'pending', 'medium',
                        'test', '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert");
            connection
                .execute(
                    "INSERT INTO project_plan_drafts (
                        id, project_id, approval_id, idea, summary, status,
                        generated_by, requested_by, created_at, updated_at
                    ) VALUES ('draft_unknown', 'project_agent_swarm',
                        'approval_unknown', 'idea', 'summary', 'draft',
                        'unknown_ai_source', 'local_user',
                        '2025-01-01', '2025-01-01')",
                    [],
                )
                .expect("should insert unknown source draft");

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: "approval_unknown".to_string(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("unknown source should be rejected");

            assert!(err.contains("unsupported project plan draft source"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_real_model_draft_is_idempotent() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let audit_id = "model_call_idempotent_real";
            insert_test_model_call(&connection, audit_id);
            let approval_id = save_and_get_approval_id(&mut connection, audit_id);
            let input = ApproveProjectPlanInput {
                approval_id: approval_id.clone(),
                second_confirm: true,
                confirm_text: "确认生成任务".to_string(),
            };

            let first =
                approve_project_plan(&mut connection, input).expect("first approve should succeed");
            let task_count = count_rows(&connection, "tasks");
            let request_count = count_rows(&connection, "runner_requests");
            let event_count = count_rows(&connection, "runtime_events");

            let second = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: approval_id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("second approve should be idempotent");

            assert_eq!(second.created_task_ids, first.created_task_ids);
            assert_eq!(count_rows(&connection, "tasks"), task_count);
            assert_eq!(count_rows(&connection, "runner_requests"), request_count);
            assert_eq!(count_rows(&connection, "runtime_events"), event_count);
            assert!(!second.side_effects.creates_tasks);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn idempotent_returns_original_ids_after_template_change_enable() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            // 第一次审批：默认 5 个任务
            let first = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("first approve should succeed");
            assert_eq!(first.created_task_ids.len(), 2);
            assert_eq!(first.created_runner_request_ids.len(), 2);
            let task_count = count_rows(&connection, "tasks");
            let request_count = count_rows(&connection, "runner_requests");
            let event_count = count_rows(&connection, "runtime_events");

            // 启用 security，模板变为 6 个
            update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "security".to_string(),
                    enabled: true,
                },
            )
            .expect("enable security should succeed");

            // 再次审批同一个已实例化草案
            let second = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("second approve should be idempotent");

            // 必须仍返回原始 5 个，不新增
            assert_eq!(second.created_task_ids, first.created_task_ids);
            assert_eq!(second.created_task_ids.len(), 2);
            assert_eq!(second.created_runner_request_ids.len(), 2);
            assert_eq!(count_rows(&connection, "tasks"), task_count);
            assert_eq!(count_rows(&connection, "runner_requests"), request_count);
            assert_eq!(count_rows(&connection, "runtime_events"), event_count);
            assert!(!second.side_effects.creates_tasks);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn idempotent_returns_original_ids_after_all_templates_disabled() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            let first = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("first approve should succeed");
            assert_eq!(first.created_task_ids.len(), 2);
            let task_count = count_rows(&connection, "tasks");
            let request_count = count_rows(&connection, "runner_requests");
            let event_count = count_rows(&connection, "runtime_events");

            // 停用所有模板
            for role in &["frontend", "backend", "qa", "docs", "reviewer"] {
                update_project_plan_task_template(
                    &mut connection,
                    UpdateProjectPlanTaskTemplateInput {
                        role: role.to_string(),
                        enabled: false,
                    },
                )
                .expect("disable should succeed");
            }

            // 再次审批同一个已实例化草案 —— 仍应成功，返回原始 5 个
            let second = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("second approve should be idempotent even with all templates disabled");

            assert_eq!(second.created_task_ids, first.created_task_ids);
            assert_eq!(second.created_task_ids.len(), 2);
            assert_eq!(second.created_runner_request_ids.len(), 2);
            assert_eq!(count_rows(&connection, "tasks"), task_count);
            assert_eq!(count_rows(&connection, "runner_requests"), request_count);
            assert_eq!(count_rows(&connection, "runtime_events"), event_count);
            assert!(!second.side_effects.creates_tasks);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    // --- helpers for stage 27 tests ---

    fn list_tasks_for_approval(
        connection: &rusqlite::Connection,
        approval_id: &str,
    ) -> Vec<crate::services::tasks::TaskSummary> {
        let task_ids: Vec<String> = {
            let mut stmt = connection
                .prepare(
                    "SELECT task_id FROM runner_requests WHERE approval_id = ?1 ORDER BY task_id",
                )
                .expect("should prepare");
            stmt.query_map(rusqlite::params![approval_id], |row| row.get(0))
                .expect("should query")
                .filter_map(|r| r.ok())
                .collect()
        };
        let mut tasks = Vec::new();
        for tid in &task_ids {
            let mut stmt = connection
                .prepare("SELECT description FROM tasks WHERE id = ?1")
                .expect("should prepare");
            let desc: String = stmt
                .query_row(rusqlite::params![tid], |row| row.get(0))
                .unwrap_or_default();
            tasks.push(crate::services::tasks::TaskSummary {
                id: tid.clone(),
                project_id: "project_agent_swarm".into(),
                title: String::new(),
                description: Some(desc),
                status: "queued".into(),
                priority: "high".into(),
                assigned_agent_id: None,
                depends_on: vec![],
                risk_level: Some("medium".into()),
                created_at: String::new(),
                updated_at: String::new(),
            });
        }
        tasks
    }

    // -------------------------------------------------------
    // 阶段 28：可配置任务角色模板测试
    // -------------------------------------------------------

    #[test]
    fn migration_006_creates_table_with_nine_builtin_templates() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM project_plan_task_templates",
                    [],
                    |row| row.get(0),
                )
                .expect("count should work");
            assert_eq!(count, 9, "should have 9 built-in templates");

            let enabled_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM project_plan_task_templates WHERE enabled = 1",
                    [],
                    |row| row.get(0),
                )
                .expect("count should work");
            assert_eq!(enabled_count, 2, "should have 2 enabled by default");

            let disabled_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM project_plan_task_templates WHERE enabled = 0",
                    [],
                    |row| row.get(0),
                )
                .expect("count should work");
            assert_eq!(disabled_count, 7, "should have 7 disabled by default");
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn list_templates_returns_only_current_project_templates() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            let templates =
                list_project_plan_task_templates(&connection).expect("list should succeed");
            assert_eq!(templates.len(), 9);
            assert!(templates.first().unwrap().sort_order <= templates.last().unwrap().sort_order);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_template_enable_security() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let before = list_project_plan_task_templates(&connection).unwrap();
            let sec = before.iter().find(|t| t.role == "security").unwrap();
            assert!(!sec.enabled);

            let updated = update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "security".to_string(),
                    enabled: true,
                },
            )
            .expect("enable should succeed");

            let sec = updated.iter().find(|t| t.role == "security").unwrap();
            assert!(sec.enabled);
            assert_eq!(count_rows(&connection, "tasks"), 4); // seed only
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_template_disable_docs() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let updated = update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "docs".to_string(),
                    enabled: false,
                },
            )
            .expect("disable should succeed");
            let docs = updated.iter().find(|t| t.role == "docs").unwrap();
            assert!(!docs.enabled);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn update_template_rejects_unknown_role() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let err = update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "designer".to_string(),
                    enabled: true,
                },
            )
            .expect_err("unknown role should be rejected");
            assert!(err.contains("unknown role"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_with_security_enabled_creates_three_tasks() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 启用 security
            update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "security".to_string(),
                    enabled: true,
                },
            )
            .expect("enable security should succeed");

            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            let response = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve with 3 templates should succeed");

            assert_eq!(response.created_task_ids.len(), 3);
            assert_eq!(response.created_runner_request_ids.len(), 3);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_with_docs_disabled_creates_one_task() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "docs".to_string(),
                    enabled: false,
                },
            )
            .expect("disable docs should succeed");

            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            let response = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve with 1 template should succeed");

            assert_eq!(response.created_task_ids.len(), 1);
            assert_eq!(response.created_runner_request_ids.len(), 1);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_with_all_disabled_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 先创建草案（预览用 fallback 不受影响）
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            // 再停用所有模板
            for role in &["frontend", "backend", "qa", "docs", "reviewer"] {
                update_project_plan_task_template(
                    &mut connection,
                    UpdateProjectPlanTaskTemplateInput {
                        role: role.to_string(),
                        enabled: false,
                    },
                )
                .expect("disable should succeed");
            }

            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("all disabled should be rejected");

            assert!(err.contains("at least one"));
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn approve_with_polluted_affected_file_rejected() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            // 先创建草案（预览用 fallback 不受污染影响）
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            // 再手动污染模板
            connection
                .execute(
                    "UPDATE project_plan_task_templates SET affected_file = '../secret'
                     WHERE role = 'frontend'",
                    [],
                )
                .expect("should update");
            let err = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id,
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect_err("polluted affected_file should be rejected");
            assert!(err.contains("forbidden") || err.contains("invalid_input"));
            assert_eq!(count_rows(&connection, "runtime_events"), 0);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    // -------------------------------------------------------
    // 阶段 29：execution preview 测试
    // -------------------------------------------------------

    #[test]
    fn execution_preview_for_pending_draft_returns_empty_lists() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            assert_eq!(draft.draft.status, "draft");

            let preview =
                get_project_plan_execution_preview(&connection, draft.approval.id.clone())
                    .expect("preview should succeed");
            assert_eq!(preview.tasks.len(), 0);
            assert_eq!(preview.runner_requests.len(), 0);
            assert!(!preview.side_effects.creates_tasks);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execution_preview_after_approval_returns_persisted_tasks_and_requests() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            let approved = approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve should succeed");

            let preview =
                get_project_plan_execution_preview(&connection, draft.approval.id.clone())
                    .expect("preview should succeed");
            assert_eq!(preview.tasks.len(), 2);
            assert_eq!(preview.runner_requests.len(), 2);
            // 返回的 ID 与 approve response 一致（集合相等，分别排序后比较）
            let mut task_ids: Vec<String> = preview.tasks.iter().map(|t| t.id.clone()).collect();
            task_ids.sort();
            let mut approved_task_ids = approved.created_task_ids.clone();
            approved_task_ids.sort();
            assert_eq!(task_ids, approved_task_ids);
            let mut req_ids: Vec<String> = preview
                .runner_requests
                .iter()
                .map(|r| r.id.clone())
                .collect();
            req_ids.sort();
            let mut approved_req_ids = approved.created_runner_request_ids.clone();
            approved_req_ids.sort();
            assert_eq!(req_ids, approved_req_ids);
            // side_effects 全部 false
            let se = &preview.side_effects;
            assert!(!se.creates_tasks);
            assert!(!se.creates_runner_requests);
            assert!(!se.executes_runner);
            assert!(!se.writes_project_files);
            assert!(!se.modifies_git);
            assert!(!se.calls_real_model);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execution_preview_does_not_recompute_after_template_change() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve should succeed");

            // 启用 security
            update_project_plan_task_template(
                &mut connection,
                UpdateProjectPlanTaskTemplateInput {
                    role: "security".to_string(),
                    enabled: true,
                },
            )
            .expect("enable should succeed");

            let preview = get_project_plan_execution_preview(&connection, draft.approval.id)
                .expect("preview should succeed");
            // 仍然只有原始 5 个，不包含 security
            assert_eq!(preview.tasks.len(), 2);
            assert!(!preview.tasks.iter().any(|t| t.role == "security"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execution_preview_still_reads_tasks_after_template_rows_removed() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve should succeed");

            connection
                .execute(
                    "DELETE FROM project_plan_task_templates WHERE project_id = ?1",
                    rusqlite::params![draft.draft.project_id.as_str()],
                )
                .expect("template rows should be removable in test");

            let preview = get_project_plan_execution_preview(&connection, draft.approval.id)
                .expect("preview should not depend on current template rows");

            assert_eq!(preview.tasks.len(), 2);
            assert_eq!(preview.runner_requests.len(), 2);
            assert!(preview.tasks.iter().any(|task| task.role == "frontend"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execution_preview_rejects_non_project_plan_approval() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            // 使用 seed 中的普通 approval
            let err = get_project_plan_execution_preview(
                &connection,
                "approval_runner_permissions".to_string(),
            )
            .expect_err("non project_plan should be rejected");
            assert!(err.contains("not a project_plan"));
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execution_preview_has_no_side_effects() {
        let (state, test_dir) = test_db();
        {
            let mut connection = state.connection().expect("connection should be available");
            let draft = create_project_plan_draft(&mut connection, valid_draft_input())
                .expect("draft should be created");
            approve_project_plan(
                &mut connection,
                ApproveProjectPlanInput {
                    approval_id: draft.approval.id.clone(),
                    second_confirm: true,
                    confirm_text: "确认生成任务".to_string(),
                },
            )
            .expect("approve should succeed");

            let before_tasks = count_rows(&connection, "tasks");
            let before_requests = count_rows(&connection, "runner_requests");
            let before_events = count_rows(&connection, "runtime_events");
            let before_mc = count_rows(&connection, "model_calls");

            let preview = get_project_plan_execution_preview(&connection, draft.approval.id)
                .expect("preview should succeed");
            assert_eq!(preview.tasks.len(), 2);

            // 数量不变
            assert_eq!(count_rows(&connection, "tasks"), before_tasks);
            assert_eq!(count_rows(&connection, "runner_requests"), before_requests);
            assert_eq!(count_rows(&connection, "runtime_events"), before_events);
            assert_eq!(count_rows(&connection, "model_calls"), before_mc);
        }
        drop(state);
        let _ = std::fs::remove_dir_all(test_dir);
    }

    fn count_rows(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable")
    }

    fn count_rows_where(connection: &Connection, table: &str, where_clause: &str) -> i64 {
        connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {where_clause}"),
                [],
                |row| row.get(0),
            )
            .expect("table should be queryable")
    }
}
