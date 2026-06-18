# P0-阶段1-SQLite schema 和命令盘点结论

日期：2026-06-18

## 本阶段结论

本阶段只做盘点，不改业务逻辑。结论是：当前项目已经有可用的 SQLite / Tauri / Rust 主链路，能够支撑“一个执行器、一个模型、一条真实 Runner 链路”的 MVP；但还不能完整支撑“总控按项目选择员工、员工绑定执行器/模型/skill、后端强校验职责边界”的长期目标。

下一步不应该继续堆 UI，也不应该直接铺满很多模型和员工。应该先补齐非敏感配置表，再让 AI 员工页从真实后端读写。

## 已有结构能复用什么

### projects

来源：`data/migrations/001_initial_sqlite.sql`

已有字段：

```text
id, name, status, phase, description, workspace_path, created_at, updated_at
```

结论：可以继续作为单项目 MVP 的项目根表。`workspace_path` 已经被任务输出目录和 Runner 相关逻辑复用。

### agents

来源：`data/migrations/001_initial_sqlite.sql`、`apps/desktop/src-tauri/src/services/agents.rs`

已有字段：

```text
id, project_id, name, role, status, model, permissions, created_at, updated_at
```

结论：现在的 `agents` 更像“当前项目里的简化 Agent 状态”，不适合作为全局固定员工池或专家模板表。

缺口：

- 没有 `category` 区分固定员工和项目专家。
- 没有 `module_scope`、`allowed_task_types`、`allowed_paths`、`forbidden_actions`。
- 没有 `default_executor_key`。
- `model` 只是自由文本语义，不能表达受控模型目录绑定。
- 没有软删除或加入/移除项目的历史语义。

建议：保留 `agents` 作为历史兼容或当前项目简化视图，新增 `agent_templates` 和 `project_agents` 承接真实员工配置。

### tasks

来源：`data/migrations/001_initial_sqlite.sql`、`apps/desktop/src-tauri/src/services/tasks.rs`

已有字段：

```text
id, project_id, title, description, status, priority, assigned_agent_id,
depends_on, risk_level, created_at, updated_at
```

结论：`tasks.assigned_agent_id` 已经能做最小任务到 Agent 的绑定，但目前只绑定到 `agents.id`，还不能记录执行器、模型、模块边界和产物目录。

缺口：

- 没有 `project_agent_id` 或明确的项目成员绑定。
- 没有 `executor_key`、`model_id`。
- 没有 `module_scope`、`task_type`。
- 没有任务级 `output_folder` 字段；当前输出目录主要通过 Runner 或任务标题推导。

建议：后续可先新增字段或建立任务分派表。不要一次重写 `tasks`。

### approvals

来源：`data/migrations/001_initial_sqlite.sql`

结论：审批表能继续承载高风险动作的入口，但 P0 员工配置 CRUD 第一阶段不应该绕到审批里自动执行。涉及写文件、执行命令、删除、Git、保护路径、跨模块修改时，再进入审批和 Runner gate。

### model_catalog

来源：`data/migrations/012_add_model_catalog.sql`、`apps/desktop/src-tauri/src/services/model_catalog.rs`

已有字段：

```text
id, project_id, provider, model_id, display_name, purpose,
enabled, is_builtin, created_at, updated_at
```

结论：这是目前最能复用的表。它已经具备受控模型目录、启停、purpose、模型 ID 校验和默认模型 fallback。

缺口：

- 没有 `executor_key`，不能表达“某个执行器下面有哪些模型”。
- `provider` 当前固定偏向 `openai_compat`，还没有多 provider / 多执行器形态。
- 没有用户自定义显示分组，例如总控默认模型、员工默认模型、专家模型。

建议：不要废掉 `model_catalog`。下一步应新增 `executor_configs`，再给模型目录补 `executor_key`，或者新建 `executor_models` 做兼容映射。

### runner_requests / runner_preflight_reviews / runner_execution_gates / runner_dry_runs / runner_execution_locks / runner_minimal_runs

来源：

```text
data/migrations/004_add_project_plan_workflow.sql
data/migrations/007_add_runner_preflight_reviews.sql
data/migrations/008_add_runner_execution_gates.sql
data/migrations/009_add_runner_dry_runs.sql
data/migrations/010_add_runner_execution_locks.sql
data/migrations/011_add_runner_minimal_runs.sql
data/migrations/015_enable_auto_runner_execution_locks.sql
data/migrations/016_open_runner_full_auto.sql
```

结论：Runner 主链路已经存在，可以继续作为所有真实写入和执行的受控出口。

缺口：

- Runner 记录目前主要围绕 `task_id` 和 runner request 走。
- 还没有稳定记录 `project_agent_id`、`executor_key`、`model_id`、`module_scope`。
- 边界判断还没有形成通用 `agent_boundary_checks` 记录。

建议：先补配置和任务绑定，再把 Runner 入口接入边界检查。

### model_calls

来源：`data/migrations/003_add_model_calls.sql`

结论：可继续作为模型调用审计表。它应该记录脱敏摘要、状态、粗粒度错误和成本估算，不记录 raw prompt、raw response、provider body 或 key。

### project_plan_task_templates

来源：`data/migrations/006_add_project_plan_task_templates.sql`

结论：可作为历史任务模板参考，但不能替代新的 `agent_templates`。任务模板描述“要做什么任务”，员工模板描述“谁能做、能做哪里、用什么执行器/模型”。

## 当前命令和服务入口

已存在并可复用：

```text
list_project_plan_models
update_project_plan_model_enabled
get_runtime_model_provider_status
update_runtime_model_provider
test_runtime_model_provider
list_agents
list_tasks
create_task
update_task_status
delete_tasks
open_task_output_folder
create_runner_minimal_run
```

关键结论：

- 系统设置里的 API Key、Base URL、模型 ID 当前进入桌面进程环境变量，不进 SQLite。
- Runner 运行时模型优先级已经对齐为：系统设置模型 ID -> SQLite 模型目录默认模型 -> `deepseek-chat` fallback。
- AI 员工页后续不应直接 `localStorage` 当真实配置源；桌面端应走 Tauri command。

## P0 缺的表

下一轮 migration 建议最小补齐：

```text
executor_configs
agent_templates
project_agents
executor_skills
agent_boundary_checks
```

`model_catalog` 可以二选一处理：

```text
方案 A：给 model_catalog 增加 executor_key
方案 B：新增 executor_models，保留 model_catalog 兼容旧模型目录
```

建议优先方案 A：改动少，能复用已有模型启停和默认模型逻辑。若后续多 provider / 多执行器关系变复杂，再拆 `executor_models`。

## 推荐字段

### executor_configs

```text
id
key
label
kind
provider
base_url_status
executable_path
status
created_at
updated_at
```

说明：不保存 API Key。`base_url_status` 只保存是否配置、格式是否有效、最后测试状态这类非敏感信息。

### agent_templates

```text
id
name
role
category
specialty
stack
module_scope
allowed_task_types
allowed_paths
forbidden_actions
default_executor_key
default_model_id
enabled
created_at
updated_at
```

说明：这是固定员工池和专家模板的真实来源。不要为了好看默认启用一堆专家。

### project_agents

```text
id
project_id
agent_template_id
name
role
source
executor_key
model_id
module_scope
status
joined_at
removed_at
created_at
updated_at
```

说明：任务最终必须绑定到项目成员，而不是泛泛的角色名。

### executor_skills

```text
id
executor_key
agent_template_id
skill_name
skill_scope
enabled
created_at
updated_at
```

说明：skill 是能力目录，不是自由权限。会触发写文件、命令、网络、Git 的 skill 必须继续走 Runner / 审批。

### agent_boundary_checks

```text
id
project_id
task_id
agent_id
requested_action
task_type
module_scope
target_path
decision
reason
created_at
```

说明：这是“总控能调度，但员工不会越界”的关键证据表。

## 下一步落地顺序

1. 新增 migration：先建 `executor_configs`、`agent_templates`、`project_agents`、`executor_skills`、`agent_boundary_checks`。
2. 给 `model_catalog` 增加 `executor_key`，默认填 `model_gateway_default`。
3. seed 一个默认执行器 `model_gateway_default`，再 seed 一个总控 Agent 和一个开发 Agent。
4. 写 Rust service / command：先做非敏感配置 CRUD，不触发 Runner。
5. 改 `desktopHost.ts`：统一封装新命令，页面不直接 `invoke`。
6. 改 AI 员工页：桌面端读写 SQLite，浏览器预览只显示示例或只读状态。
7. 再做任务绑定：任务必须显示分配的 AI 员工、执行器、模型、产物目录。
8. 最后接边界检查：Runner 执行前按 `agent_id`、`task_type`、`module_scope`、`target_path`、`forbidden_actions` 判断。

## 不做

- 不把 API Key、Token、私钥写进 SQLite、localStorage、日志或文档。
- 不让前端自由输入模型 ID 后直接执行。
- 不让 UI 决定权限，权限必须后端强校验。
- 不开放自由 shell、Git push、删除文件、保护路径写入。
- 不一次性重写现有任务和 Runner 链路。

