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
const PROJECT_PLAN_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/004_add_project_plan_workflow.sql");
const PROJECT_PLAN_MODEL_AUDIT_LINK_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/005_add_project_plan_model_audit_link.sql");
const TASK_TEMPLATES_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/006_add_project_plan_task_templates.sql");
const RUNNER_PREFLIGHT_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/007_add_runner_preflight_reviews.sql");
const RUNNER_EXECUTION_GATES_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/008_add_runner_execution_gates.sql");
const RUNNER_DRY_RUNS_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/009_add_runner_dry_runs.sql");
const RUNNER_EXECUTION_LOCKS_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/010_add_runner_execution_locks.sql");
const RUNNER_MINIMAL_RUNS_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/011_add_runner_minimal_runs.sql");
const MODEL_CATALOG_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/012_add_model_catalog.sql");
const IDEA_GUIDANCE_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/013_add_idea_guidance.sql");
const PROJECT_INTAKE_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/014_add_project_intake.sql");
const AUTO_RUNNER_LOCKS_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/015_enable_auto_runner_execution_locks.sql");
const OPEN_RUNNER_FULL_AUTO_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/016_open_runner_full_auto.sql");
const AGENT_CONFIG_CORE_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/017_add_agent_config_core.sql");
const RELAX_TASK_AGENT_FK_MIGRATION_SQL: &str =
    include_str!("../../../../../data/migrations/018_relax_tasks_assigned_agent_fk.sql");
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
    run_project_plan_migration(&connection)?;
    run_project_plan_model_audit_link_migration(&connection)?;
    run_task_templates_migration(&connection)?;
    run_runner_preflight_migration(&connection)?;
    run_runner_execution_gates_migration(&connection)?;
    run_runner_dry_runs_migration(&connection)?;
    run_runner_execution_locks_migration(&connection)?;
    run_runner_minimal_runs_migration(&connection)?;
    run_model_catalog_migration(&connection)?;
    run_idea_guidance_migration(&connection)?;
    run_project_intake_migration(&connection)?;
    run_auto_runner_locks_migration(&connection)?;
    run_open_runner_full_auto_migration(&connection)?;
    run_agent_config_core_migration(&connection)?;
    run_relax_task_agent_fk_migration(&connection)?;
    seed_initial_data_if_needed(&mut connection)?;
    seed_builtin_task_templates(&connection)?;
    seed_builtin_models(&connection)?;
    seed_agent_config_core(&connection)?;

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

fn run_project_plan_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(PROJECT_PLAN_MIGRATION_SQL)?;
    Ok(())
}

fn run_project_plan_model_audit_link_migration(connection: &Connection) -> InitResult<()> {
    // SQLite 不支持 IF NOT EXISTS 的 ALTER TABLE，需要用 PRAGMA 先检查列是否存在
    let mut stmt = connection.prepare("PRAGMA table_info('project_plan_drafts')")?;
    let has_model_call_id = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("database_error: pragma table_info failed: {e}"))?
        .filter_map(|r| r.ok())
        .any(|col| col == "model_call_id");

    if !has_model_call_id {
        connection
            .execute_batch("ALTER TABLE project_plan_drafts ADD COLUMN model_call_id TEXT;")?;
    }

    connection.execute_batch(PROJECT_PLAN_MODEL_AUDIT_LINK_MIGRATION_SQL)?;
    Ok(())
}

fn run_task_templates_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(TASK_TEMPLATES_MIGRATION_SQL)?;
    Ok(())
}

fn run_runner_preflight_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RUNNER_PREFLIGHT_MIGRATION_SQL)?;
    Ok(())
}

fn run_runner_execution_gates_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RUNNER_EXECUTION_GATES_MIGRATION_SQL)?;
    Ok(())
}

fn run_runner_dry_runs_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RUNNER_DRY_RUNS_MIGRATION_SQL)?;
    Ok(())
}

fn run_runner_execution_locks_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RUNNER_EXECUTION_LOCKS_MIGRATION_SQL)?;
    Ok(())
}

fn run_runner_minimal_runs_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RUNNER_MINIMAL_RUNS_MIGRATION_SQL)?;
    Ok(())
}

fn run_model_catalog_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(MODEL_CATALOG_MIGRATION_SQL)?;
    Ok(())
}

fn run_idea_guidance_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(IDEA_GUIDANCE_MIGRATION_SQL)?;
    Ok(())
}

fn run_project_intake_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(PROJECT_INTAKE_MIGRATION_SQL)?;
    Ok(())
}

fn run_auto_runner_locks_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(AUTO_RUNNER_LOCKS_MIGRATION_SQL)?;
    Ok(())
}

fn run_open_runner_full_auto_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(OPEN_RUNNER_FULL_AUTO_MIGRATION_SQL)?;
    Ok(())
}

fn run_agent_config_core_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(AGENT_CONFIG_CORE_MIGRATION_SQL)?;

    let mut stmt = connection.prepare("PRAGMA table_info('model_catalog')")?;
    let has_executor_key = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("database_error: pragma table_info failed: {e}"))?
        .filter_map(|r| r.ok())
        .any(|col| col == "executor_key");

    if !has_executor_key {
        connection.execute_batch(
            "ALTER TABLE model_catalog ADD COLUMN executor_key TEXT DEFAULT 'model_gateway_default';",
        )?;
    }

    connection.execute_batch(
        "UPDATE model_catalog
         SET executor_key = 'model_gateway_default'
         WHERE executor_key IS NULL OR TRIM(executor_key) = '';

         DROP INDEX IF EXISTS idx_model_catalog_unique;

         CREATE UNIQUE INDEX IF NOT EXISTS idx_model_catalog_unique
           ON model_catalog (project_id, executor_key, provider, model_id, purpose);

         CREATE INDEX IF NOT EXISTS idx_model_catalog_executor
           ON model_catalog (project_id, executor_key, enabled);",
    )?;
    Ok(())
}

fn run_relax_task_agent_fk_migration(connection: &Connection) -> InitResult<()> {
    connection.execute_batch(RELAX_TASK_AGENT_FK_MIGRATION_SQL)?;
    Ok(())
}

pub(crate) fn seed_builtin_task_templates(connection: &Connection) -> InitResult<()> {
    seed_builtin_task_templates_inner(connection)
}

pub(crate) fn seed_builtin_models(connection: &Connection) -> InitResult<()> {
    crate::services::model_catalog::seed_builtin_models(connection)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn Error>)
}

pub(crate) fn seed_agent_config_core(connection: &Connection) -> InitResult<()> {
    seed_agent_config_core_inner(connection)
}

fn seed_agent_config_core_inner(connection: &Connection) -> InitResult<()> {
    let project_id: String = connection
        .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
        .unwrap_or_else(|_| "project_agent_swarm".into());
    let now = "2026-06-19T00:00:00Z";

    connection.execute(
        "INSERT INTO executor_configs (
            id, key, label, kind, provider, base_url_status, executable_path,
            status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?8)
        ON CONFLICT(key) DO NOTHING",
        params![
            "executor_model_gateway_default",
            "model_gateway_default",
            "默认模型网关",
            "model_gateway",
            "openai_compat",
            "configured_by_system_settings",
            "active",
            now,
        ],
    )?;

    let templates: &[BuiltinAgentTemplate] = &[
        BuiltinAgentTemplate {
            id: "agent_template_controller",
            name: "总控 Agent",
            role: "controller",
            category: "core",
            specialty: "理解目标、拆解任务、选择员工、控制风险",
            stack: "orchestration",
            module_scope: "controller",
            allowed_task_types: &[
                "project_intake",
                "task_breakdown",
                "agent_assignment",
                "risk_routing",
            ],
            allowed_paths: &["workspace/generated/**"],
            forbidden_actions: &[
                "free_shell",
                "git_push",
                "delete_protected_path",
                "read_raw_secret",
            ],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_frontend",
            name: "前端工程 Agent",
            role: "frontend",
            category: "core",
            specialty: "页面、组件、交互、样式和前端构建",
            stack: "typescript/react/css",
            module_scope: "frontend",
            allowed_task_types: &["frontend_impl", "ux_plan", "ui_refine"],
            allowed_paths: &["packages/ui/**", "workspace/generated/**"],
            forbidden_actions: &["database_migration", "backend_secret_config", "git_push"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_backend",
            name: "后端工程 Agent",
            role: "backend",
            category: "core",
            specialty: "接口、服务、业务逻辑和模型调用封装",
            stack: "rust/tauri/sqlite",
            module_scope: "backend",
            allowed_task_types: &["backend_impl", "tauri_command", "service_logic"],
            allowed_paths: &["apps/desktop/src-tauri/src/**", "workspace/generated/**"],
            forbidden_actions: &["frontend_visual_redesign", "git_push", "read_raw_secret"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_database",
            name: "数据库 Agent",
            role: "database",
            category: "core",
            specialty: "SQLite schema、migration、索引和数据边界",
            stack: "sqlite/rusqlite",
            module_scope: "database",
            allowed_task_types: &["database_migration", "schema_review", "seed_data"],
            allowed_paths: &["data/migrations/**", "apps/desktop/src-tauri/src/db/**"],
            forbidden_actions: &["frontend_style_edit", "free_shell", "read_raw_secret"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_desktop",
            name: "桌面端 Agent",
            role: "desktop",
            category: "core",
            specialty: "Tauri 桌面能力、系统凭据、本机路径和宿主集成",
            stack: "tauri/windows",
            module_scope: "desktop",
            allowed_task_types: &["desktop_integration", "credential_status", "host_bridge"],
            allowed_paths: &["apps/desktop/**", "packages/ui/src/utils/desktopHost.ts"],
            forbidden_actions: &["store_api_key_in_db", "log_secret", "git_push"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_qa",
            name: "测试 Agent",
            role: "qa",
            category: "core",
            specialty: "测试、验收、回归和构建检查",
            stack: "cargo/npm/playwright",
            module_scope: "qa",
            allowed_task_types: &["qa_test", "regression_check", "acceptance_check"],
            allowed_paths: &["apps/desktop/src-tauri/src/**", "packages/ui/src/**"],
            forbidden_actions: &["product_behavior_change", "delete_files", "git_push"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_security",
            name: "安全 Agent",
            role: "security",
            category: "core",
            specialty: "密钥边界、危险动作、保护路径和越界审查",
            stack: "security-review",
            module_scope: "security",
            allowed_task_types: &["security_review_plan", "boundary_check", "secret_scan"],
            allowed_paths: &["docs/**", "dev-docs/**", "apps/desktop/src-tauri/src/**"],
            forbidden_actions: &["approve_high_risk_action", "read_raw_secret", "git_push"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_docs",
            name: "文档 Agent",
            role: "docs",
            category: "core",
            specialty: "用户说明、开发文档和交接记录",
            stack: "markdown",
            module_scope: "docs",
            allowed_task_types: &["docs_write", "handover_update", "api_doc"],
            allowed_paths: &["docs/**", "dev-docs/**", "README.md"],
            forbidden_actions: &["change_product_behavior", "write_secret", "git_push"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_devops",
            name: "DevOps Agent",
            role: "devops",
            category: "core",
            specialty: "脚本、运行命令、构建和本地验证流程",
            stack: "powershell/npm/cargo",
            module_scope: "devops",
            allowed_task_types: &["devops_plan", "build_check", "script_review"],
            allowed_paths: &["scripts/**", "apps/desktop/src-tauri/**", "packages/ui/**"],
            forbidden_actions: &["unapproved_shell", "git_push", "delete_protected_path"],
            enabled: true,
        },
        BuiltinAgentTemplate {
            id: "agent_template_ux",
            name: "UX Agent",
            role: "ux",
            category: "core",
            specialty: "用户流程、交互状态、可用性和界面信息架构",
            stack: "ux/product-ui",
            module_scope: "ux",
            allowed_task_types: &["ux_plan", "interaction_review", "copy_refine"],
            allowed_paths: &[
                "packages/ui/src/pages/**",
                "packages/ui/src/theme/**",
                "dev-docs/**",
            ],
            forbidden_actions: &["database_migration", "backend_secret_config", "git_push"],
            enabled: true,
        },
    ];

    for template in templates {
        connection.execute(
            "INSERT INTO agent_templates (
                id, name, role, category, specialty, stack, module_scope,
                allowed_task_types, allowed_paths, forbidden_actions,
                default_executor_key, default_model_id, enabled, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                'model_gateway_default', NULL, ?11, ?12, ?12)
            ON CONFLICT(id) DO NOTHING",
            params![
                template.id,
                template.name,
                template.role,
                template.category,
                template.specialty,
                template.stack,
                template.module_scope,
                json_list(template.allowed_task_types)?,
                json_list(template.allowed_paths)?,
                json_list(template.forbidden_actions)?,
                template.enabled as i64,
                now,
            ],
        )?;
    }

    let project_agents: &[BuiltinProjectAgent] = &[
        BuiltinProjectAgent {
            id: "project_agent_controller",
            template_id: "agent_template_controller",
            name: "项目总控",
            role: "controller",
            module_scope: "controller",
            status: "active",
        },
        BuiltinProjectAgent {
            id: "project_agent_frontend",
            template_id: "agent_template_frontend",
            name: "前端执行员",
            role: "frontend",
            module_scope: "frontend",
            status: "idle",
        },
        BuiltinProjectAgent {
            id: "project_agent_backend",
            template_id: "agent_template_backend",
            name: "后端执行员",
            role: "backend",
            module_scope: "backend",
            status: "idle",
        },
        BuiltinProjectAgent {
            id: "project_agent_docs",
            template_id: "agent_template_docs",
            name: "文档记录员",
            role: "docs",
            module_scope: "docs",
            status: "idle",
        },
    ];

    for agent in project_agents {
        connection.execute(
            "INSERT INTO project_agents (
                id, project_id, agent_template_id, name, role, source,
                executor_key, model_id, module_scope, status,
                joined_at, removed_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'core',
                'model_gateway_default', NULL, ?6, ?7, ?8, NULL, ?8, ?8)
            ON CONFLICT(id) DO NOTHING",
            params![
                agent.id,
                project_id.as_str(),
                agent.template_id,
                agent.name,
                agent.role,
                agent.module_scope,
                agent.status,
                now,
            ],
        )?;
    }

    let skills: &[BuiltinExecutorSkill] = &[
        BuiltinExecutorSkill {
            id: "skill_controller_task_breakdown",
            template_id: "agent_template_controller",
            name: "task_breakdown",
            scope: "controller",
        },
        BuiltinExecutorSkill {
            id: "skill_controller_agent_assignment",
            template_id: "agent_template_controller",
            name: "agent_assignment",
            scope: "controller",
        },
        BuiltinExecutorSkill {
            id: "skill_frontend_ui_impl",
            template_id: "agent_template_frontend",
            name: "frontend_ui_impl",
            scope: "frontend",
        },
        BuiltinExecutorSkill {
            id: "skill_backend_tauri_command",
            template_id: "agent_template_backend",
            name: "tauri_command_impl",
            scope: "backend",
        },
        BuiltinExecutorSkill {
            id: "skill_database_schema_review",
            template_id: "agent_template_database",
            name: "schema_review",
            scope: "database",
        },
        BuiltinExecutorSkill {
            id: "skill_security_boundary_check",
            template_id: "agent_template_security",
            name: "boundary_check",
            scope: "security",
        },
        BuiltinExecutorSkill {
            id: "skill_docs_handover",
            template_id: "agent_template_docs",
            name: "handover_update",
            scope: "docs",
        },
        BuiltinExecutorSkill {
            id: "skill_qa_regression",
            template_id: "agent_template_qa",
            name: "regression_check",
            scope: "qa",
        },
    ];

    for skill in skills {
        connection.execute(
            "INSERT INTO executor_skills (
                id, executor_key, agent_template_id, skill_name, skill_scope,
                enabled, created_at, updated_at
            ) VALUES (?1, 'model_gateway_default', ?2, ?3, ?4, 1, ?5, ?5)
            ON CONFLICT(id) DO NOTHING",
            params![skill.id, skill.template_id, skill.name, skill.scope, now],
        )?;
    }

    Ok(())
}

fn json_list(items: &[&str]) -> InitResult<String> {
    serde_json::to_string(items).map_err(|error| Box::new(error) as Box<dyn Error>)
}

struct BuiltinAgentTemplate {
    id: &'static str,
    name: &'static str,
    role: &'static str,
    category: &'static str,
    specialty: &'static str,
    stack: &'static str,
    module_scope: &'static str,
    allowed_task_types: &'static [&'static str],
    allowed_paths: &'static [&'static str],
    forbidden_actions: &'static [&'static str],
    enabled: bool,
}

struct BuiltinProjectAgent {
    id: &'static str,
    template_id: &'static str,
    name: &'static str,
    role: &'static str,
    module_scope: &'static str,
    status: &'static str,
}

struct BuiltinExecutorSkill {
    id: &'static str,
    template_id: &'static str,
    name: &'static str,
    scope: &'static str,
}

fn seed_builtin_task_templates_inner(connection: &Connection) -> InitResult<()> {
    let project_id: String = connection
        .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
        .unwrap_or_else(|_| "project_agent_swarm".into());
    let now = "2025-01-01T00:00:00Z";

    let builtins: &[BuiltinTemplate] = &[
        BuiltinTemplate {
            role: "frontend",
            agent_id: "agent_frontend",
            title: "生成可打开页面",
            description:
                "生成 index.html、style.css 和 main.js，做出用户能直接打开查看的第一版页面。",
            priority: "high",
            risk_level: "medium",
            affected_files: &["virtual/index.html", "virtual/style.css", "virtual/main.js"],
            operation_type: "frontend_impl",
            enabled: true,
            sort_order: 0,
        },
        BuiltinTemplate {
            role: "backend",
            agent_id: "agent_backend",
            title: "后端接口实现",
            description: "实现服务器端接口、业务逻辑和数据持久化。",
            priority: "high",
            risk_level: "medium",
            affected_files: &["virtual/server.js"],
            operation_type: "backend_impl",
            enabled: false,
            sort_order: 1,
        },
        BuiltinTemplate {
            role: "qa",
            agent_id: "agent_qa",
            title: "功能测试",
            description: "编写测试用例，验证功能是否正常工作。",
            priority: "medium",
            risk_level: "low",
            affected_files: &["virtual/test.js"],
            operation_type: "qa_test",
            enabled: false,
            sort_order: 2,
        },
        BuiltinTemplate {
            role: "docs",
            agent_id: "agent_docs",
            title: "项目文档",
            description: "编写用户文档和开发文档。",
            priority: "low",
            risk_level: "low",
            affected_files: &["virtual/README.md"],
            operation_type: "docs_write",
            enabled: true,
            sort_order: 3,
        },
        BuiltinTemplate {
            role: "reviewer",
            agent_id: "agent_reviewer",
            title: "代码审查",
            description: "审查代码质量、安全性和最佳实践。",
            priority: "medium",
            risk_level: "low",
            affected_files: &["virtual/review-comments.md"],
            operation_type: "code_review",
            enabled: false,
            sort_order: 4,
        },
        BuiltinTemplate {
            role: "security",
            agent_id: "agent_reviewer",
            title: "安全审查切片",
            description: "审查安全边界、敏感数据路径和保护路径合规。",
            priority: "high",
            risk_level: "high",
            affected_files: &["virtual/security-plan.md"],
            operation_type: "security_review_plan",
            enabled: false,
            sort_order: 5,
        },
        BuiltinTemplate {
            role: "devops",
            agent_id: "agent_backend",
            title: "DevOps 切片",
            description: "整理本地运行、脚本和验证命令。",
            priority: "medium",
            risk_level: "medium",
            affected_files: &["virtual/devops-plan.md"],
            operation_type: "devops_plan",
            enabled: false,
            sort_order: 6,
        },
        BuiltinTemplate {
            role: "ux",
            agent_id: "agent_frontend",
            title: "UX 设计切片",
            description: "整理用户交互流程和界面状态。",
            priority: "medium",
            risk_level: "low",
            affected_files: &["virtual/ux-plan.md"],
            operation_type: "ux_plan",
            enabled: false,
            sort_order: 7,
        },
        BuiltinTemplate {
            role: "data",
            agent_id: "agent_backend",
            title: "数据建模切片",
            description: "整理数据模型、迁移策略和查询边界。",
            priority: "medium",
            risk_level: "medium",
            affected_files: &["virtual/data-plan.md"],
            operation_type: "data_plan",
            enabled: false,
            sort_order: 8,
        },
    ];

    for t in builtins {
        let affected_file = t.affected_files.join("\n");
        connection.execute(
            "INSERT INTO project_plan_task_templates (
                id, project_id, role, agent_id, title, description,
                priority, risk_level, affected_file, operation_type,
                enabled, sort_order, is_builtin, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?13)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                priority = excluded.priority,
                risk_level = excluded.risk_level,
                affected_file = excluded.affected_file,
                operation_type = excluded.operation_type,
                enabled = excluded.enabled,
                sort_order = excluded.sort_order,
                is_builtin = 1,
                updated_at = excluded.updated_at",
            rusqlite::params![
                format!("template_{}_{}", project_id, t.role),
                project_id.as_str(),
                t.role,
                t.agent_id,
                t.title,
                t.description,
                t.priority,
                t.risk_level,
                affected_file.as_str(),
                t.operation_type,
                t.enabled as i64,
                t.sort_order,
                now,
            ],
        )?;
    }
    Ok(())
}

struct BuiltinTemplate {
    role: &'static str,
    agent_id: &'static str,
    title: &'static str,
    description: &'static str,
    priority: &'static str,
    risk_level: &'static str,
    affected_files: &'static [&'static str],
    operation_type: &'static str,
    enabled: bool,
    sort_order: i64,
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
        agents::list_agents, approvals::list_approvals,
        model_gateway::create_project_plan_draft_core, tasks::list_tasks,
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
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
            assert_eq!(count_rows(&connection, "executor_configs"), 1);
            assert_eq!(count_rows(&connection, "agent_templates"), 10);
            assert_eq!(count_rows(&connection, "project_agents"), 4);
            assert_eq!(count_rows(&connection, "executor_skills"), 8);
            assert_eq!(count_rows(&connection, "agent_boundary_checks"), 0);

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

            let executor_key: String = connection
                .query_row("SELECT key FROM executor_configs LIMIT 1", [], |row| {
                    row.get(0)
                })
                .expect("default executor should exist");
            assert_eq!(executor_key, "model_gateway_default");

            let controller_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM project_agents
                     WHERE project_id = 'project_agent_swarm'
                       AND role = 'controller'
                       AND executor_key = 'model_gateway_default'
                       AND removed_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .expect("controller project agent should be queryable");
            assert_eq!(controller_count, 1);
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
            assert_eq!(count_rows(&connection, "project_plan_drafts"), 0);
            assert_eq!(count_rows(&connection, "runner_requests"), 0);
            assert_eq!(count_rows(&connection, "executor_configs"), 1);
            assert_eq!(count_rows(&connection, "agent_templates"), 10);
            assert_eq!(count_rows(&connection, "project_agents"), 4);
            assert_eq!(count_rows(&connection, "executor_skills"), 8);
        }
        drop(state);

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn agent_config_core_tables_and_model_executor_key_exist() {
        let (state, test_dir) = test_db();
        {
            let connection = state.connection().expect("connection should be available");
            for table in [
                "executor_configs",
                "agent_templates",
                "project_agents",
                "executor_skills",
                "agent_boundary_checks",
            ] {
                assert_eq!(table_exists(&connection, table), 1, "{table} should exist");
            }

            let model_columns = table_columns(&connection, "model_catalog");
            assert!(
                model_columns.contains(&"executor_key".to_string()),
                "model_catalog.executor_key should exist"
            );

            let enabled_model_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM model_catalog
                     WHERE executor_key = 'model_gateway_default' AND enabled = 1",
                    [],
                    |row| row.get(0),
                )
                .expect("model catalog should be queryable");
            assert!(enabled_model_count >= 1);
        }
        drop(state);
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn agent_config_seed_preserves_user_state_on_reinitialize() {
        let test_dir = std::env::temp_dir().join(format!(
            "agent-swarm-sqlite-agent-config-seed-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let state = initialize(test_dir.clone()).expect("sqlite should initialize");
        {
            let connection = state.connection().expect("connection should be available");
            connection
                .execute(
                    "UPDATE project_agents
                     SET status = 'removed',
                         removed_at = '2026-06-19T01:00:00Z',
                         updated_at = '2026-06-19T01:00:00Z'
                     WHERE id = 'project_agent_frontend'",
                    [],
                )
                .expect("project agent should update");
            connection
                .execute(
                    "UPDATE agent_templates
                     SET enabled = 0, updated_at = '2026-06-19T01:00:00Z'
                     WHERE id = 'agent_template_frontend'",
                    [],
                )
                .expect("agent template should update");
        }
        drop(state);

        let state = initialize(test_dir.clone()).expect("sqlite should reinitialize");
        {
            let connection = state.connection().expect("connection should be available");
            let removed_at: Option<String> = connection
                .query_row(
                    "SELECT removed_at FROM project_agents WHERE id = 'project_agent_frontend'",
                    [],
                    |row| row.get(0),
                )
                .expect("project agent should exist");
            assert_eq!(removed_at.as_deref(), Some("2026-06-19T01:00:00Z"));

            let enabled: i64 = connection
                .query_row(
                    "SELECT enabled FROM agent_templates WHERE id = 'agent_template_frontend'",
                    [],
                    |row| row.get(0),
                )
                .expect("agent template should exist");
            assert_eq!(enabled, 0);
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

            let response = create_project_plan_draft_core(
                "测试想法",
                &None,
                false,
                &None,
                "false",
                None,
                None,
                None,
                None,
                None,
            )
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

            let response = create_project_plan_draft_core(
                "测试想法",
                &None,
                false,
                &None,
                "false",
                None,
                None,
                None,
                None,
                None,
            )
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

    fn table_exists(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("sqlite_master should be queryable")
    }

    fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut stmt = connection
            .prepare(&format!("PRAGMA table_info('{table}')"))
            .expect("table_info should be queryable");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("columns should map")
            .filter_map(|row| row.ok())
            .collect()
    }
}
