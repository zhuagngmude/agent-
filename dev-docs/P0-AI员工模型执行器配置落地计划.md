# P0-AI员工模型执行器配置落地计划

日期：2026-06-18

本文是下一阶段施工计划。目标不是继续堆 UI，而是把现在已经能看的 `AI 员工 / 模型 / 执行器 / skill` 配置，落到真实 Tauri + Rust + SQLite 链路里，并为后续总控 Agent 分配任务打基础。

## 一句话目标

```text
系统设置配置模型服务
-> 模型网关维护可用模型目录
-> AI 员工页只从已配置模型和执行器中选择
-> 总控按项目目标选择员工和专家
-> 每个任务绑定具体 Agent / 执行器 / 模型 / 职责边界
-> Runner 执行前由后端强校验是否越界
```

## 当前状态

- 前端已经有 `AI 员工` 页、系统设置页、执行器配置树、模型目录、员工绑定和 skill 配置预览。
- `packages/ui` 是真实 UI 源，同时服务浏览器预览和 Tauri 桌面端。
- 浏览器预览只适合看界面，不代表真实读写 SQLite。
- 真实能力必须走 `packages/ui -> desktopHost.ts -> Tauri invoke -> Rust command -> service -> SQLite`。
- 部分 AI 员工配置目前仍是前端临时状态或 `localStorage`，不能作为长期真实配置。

## 核心原则

1. API Key、Token、私钥不进 SQLite、不进日志、不进文档、不回显给前端。
2. 前端只负责展示和发起请求，业务真相必须在 Rust service 和 SQLite。
3. Agent 职责边界不能只靠 prompt 或 UI 文案，必须由后端和 Runner 强校验。
4. 浏览器预览可以保留 fallback 数据，但不得伪装成真实保存。
5. 每一步都要能单独验证，避免一次性大改。

## P0 数据对象

### executor_configs

保存执行器的非敏感配置。

建议字段：

```text
id
key
label
kind: model_gateway | external_executor | local_tool
provider
base_url_status
executable_path
status: active | disabled | error
created_at
updated_at
```

说明：

- 不保存真实 API Key。
- `base_url_status` 只保存是否已配置、是否格式有效，不保存完整敏感连接信息。
- 如果后续需要保存可恢复的 key，必须单独设计安全存储，不混进普通业务表。

### executor_models

保存某个执行器可用的模型目录。

建议字段：

```text
id
executor_key
model_id
label
purpose
enabled
is_default
created_at
updated_at
```

说明：

- AI 员工页只能从这里选模型。
- 不允许用户在 Agent 绑定处随手输入任意模型 ID 后直接执行。

### agent_templates

保存固定员工池和可复用专家模板。

建议字段：

```text
id
name
role
category: core | expert
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

说明：

- 固定员工池按技术栈补全，但不要为了数量好看默认激活一堆专家。
- 专家模板可以存在，但项目中是否启用由总控推荐或用户确认。

### project_agents

保存当前项目真正参与工作的 Agent。

建议字段：

```text
id
project_id
agent_template_id
name
role
source: core | recommended | manual
executor_key
model_id
module_scope
status
joined_at
removed_at
created_at
updated_at
```

说明：

- 任务分配必须绑定到 `project_agents.id`，不能只写一个泛泛的 role。
- `removed_at` 用于软删除，避免运行记录断链。

### executor_skills

保存执行器或 Agent 能使用的 skill。

建议字段：

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

说明：

- skill 不是自由权限，必须和职责边界一起校验。
- 后续如果 skill 会触发写文件、命令、网络请求，必须进入 Runner 或审批链。

### agent_boundary_checks

记录后端对 Agent 越界行为的判定。

建议字段：

```text
id
project_id
task_id
agent_id
requested_action
task_type
module_scope
target_path
decision: allowed | denied | needs_approval
reason
created_at
```

说明：

- 这是总控调度能否可信的关键。
- 如果前端 Agent 想改数据库 migration，但它的 scope 是 frontend，后端必须拒绝。

## 后端命令计划

第一批 Tauri commands 只做配置读写，不做自动执行：

```text
list_executor_configs
upsert_executor_config
delete_executor_config

list_executor_models
upsert_executor_model
delete_executor_model

list_agent_templates
upsert_agent_template
delete_agent_template

list_project_agents
upsert_project_agent
remove_project_agent

list_executor_skills
upsert_executor_skill
delete_executor_skill
```

第二批再接调度和边界：

```text
recommend_project_agents
assign_project_agents_to_task
check_agent_boundary
list_agent_boundary_checks
```

## 前端改造计划

### AI 员工页

- 去掉真实配置对 `localStorage` 的依赖。
- 浏览器预览保留示例数据，但明确只读。
- 桌面端从 Tauri commands 读取执行器、模型、Agent 模板、项目成员和 skill。
- 新增、删除、编辑都走后端服务。
- API Key 输入不保存，保存后清空，只显示“已配置 / 未配置 / 测试失败”等状态。

### 系统设置页

- 保留系统默认模型服务配置。
- 系统默认模型可以作为总控默认大脑。
- 执行器级别配置可以覆盖系统默认。
- 模型连接测试只显示粗粒度错误：未配置 key、Base URL 错误、模型 ID 不可用、网络失败、余额或权限问题。

### 任务拆解页

- 每个任务必须显示分配的 AI 员工。
- 每个员工显示当前状态、职责、模型、执行器和产物路径。
- 不能只显示泛泛的“frontend/backend/qa”角色。

## 实施顺序

### 阶段 1：盘点现有 schema 和命令

目标：

- 确认现有 `model_catalog` 能否复用，哪些字段需要新表补齐。
- 确认 `agents` 表是否只适合当前项目成员，还是能承载模板。
- 确认 `tasks` 和 Runner records 是否已有足够字段绑定 `agent_id / executor_key / model_id`。

验收：

- 输出一份字段对照结论。
- 不改业务逻辑。

### 阶段 2：新增 SQLite migration

目标：

- 新增或扩展 P0 表。
- 为唯一键、project_id、executor_key、agent_id 建索引。
- 保证 seed 可重复运行，不污染用户已有配置。

验收：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo test db::tests --lib
cargo test model_catalog --lib
```

### 阶段 3：Rust service 和 Tauri commands

目标：

- 增加配置 CRUD service。
- 所有输入做长度、枚举、重复值、敏感内容校验。
- API Key 不参与普通 CRUD。
- 错误返回中文粗粒度原因，不泄露 raw provider error。

验收：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test
```

### 阶段 4：前端接真实读写

目标：

- `AgentsPage.tsx` 桌面端改为 Tauri 读写。
- `desktopHost.ts` 增加统一封装，不在页面里直接 `invoke`。
- 浏览器预览继续可看，但不假装能保存真实配置。

验收：

```powershell
cd F:\Projects\agent-swarm\packages\ui
npm run typecheck
npm run build
```

### 阶段 5：任务绑定 Agent

目标：

- 任务创建、继续执行、Runner 输出记录开始绑定具体 `project_agent_id`。
- 任务页按“项目任务 -> 分配的 AI 员工 -> 子任务/状态/产物”展示真实数据。

验收：

- 新建或自动生成任务后，可以看到具体 AI 员工。
- 打开运行输出能追踪该员工使用的模型和执行器。
- 刷新或重启桌面端后数据仍存在。

### 阶段 6：总控调度和越界校验

目标：

- 总控根据项目类型、技术栈、风险、阶段选择 Agent。
- 跨模块任务必须拆分或转派。
- Runner 执行前调用后端边界检查。

验收：

- frontend Agent 请求改 migration，被拒绝。
- database Agent 请求改 CSS，被拒绝。
- docs Agent 请求改产品行为，被拒绝或进入审批。
- 合法范围内的任务可以继续进入 Runner。

## 不做事项

- 不重写前端框架。
- 不把浏览器预览当真实桌面端。
- 不把 API Key 写进 SQLite、localStorage、日志或文档。
- 不开放自由 shell、Git commit/push、删除文件、保护路径写入。
- 不让 UI 直接决定 Agent 权限。
- 不为了好看默认激活所有专家 Agent。

## 需要同步的文档

实现阶段如果有变化，需要同步：

- `dev-docs/当前项目导航.md`
- `dev-docs/下一步开发路线.md`
- `docs/api-draft.md`
- `docs/data-model-draft.md`
- `docs/Agent宪法.md`
- `docs/AI开发细则.md`

## 完成定义

P0 完成不等于总控已经完全聪明。P0 完成的标准是：

```text
AI 员工 / 执行器 / 模型 / skill 的非敏感配置
已经从前端临时状态迁移到 Tauri + Rust + SQLite，
任务能绑定具体项目 Agent，
Runner 能根据 Agent 边界做后端强校验。
```

到这一步，后续再做真正的总控智能推荐、专家 Agent 动态加入、多模型执行器路由，才不会变成 UI 假入口。
