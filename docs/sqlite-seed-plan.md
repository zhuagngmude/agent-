# SQLite 初始化与 Seed 方案

日期：2026-06-09（原稿），2026-06-13 更新
阶段：新架构最小落库阶段

本文设计本地 SQLite 初始化和 seed 方案。当前第一版只建 4 张核心表（projects / agents / tasks / approvals），其余表在后续 migration 中按需追加。

## 1. 目标

第一步数据库接入只解决一件事：把当前 `services/api/mock-data.js` 的初始数据和 `data/mock/runtime-state.json` 的运行态迁移到可重复初始化的本地 SQLite 结构中。

目标：

- 保持现有 API response 结构不变。
- 保持 Mock API 的状态流转语义不变。
- 让数据库可以一键重建和重新 seed。
- 为后续 Supabase PostgreSQL 迁移保留表名、字段名和索引命名习惯。

明确不做：

- 不接真实 Runner 执行。
- 不接真实模型 API。
- 不接云端 PostgreSQL。
- 不做登录、团队、多项目 UI 或完整权限系统。
- 不把本地 SQLite 数据库文件提交进 Git。

## 2. 产物位置

当前第一批实现使用以下位置：

```text
data/local/agent-swarm.sqlite
data/local/agent-swarm.sqlite-shm
data/local/agent-swarm.sqlite-wal
data/migrations/001_initial_sqlite.sql
scripts/init-sqlite.ps1
scripts/seed-sqlite.ps1
scripts/verify-sqlite-flows.ps1
scripts/sqlite/
services/api/db/
```

说明：

- `data/local/` 是本地数据库运行态目录，必须加入 `.gitignore`。
- `data/migrations/*.sql` 是可提交的 schema 迁移脚本。
- `scripts/init-sqlite.ps1` 负责创建数据库和应用迁移。
- `scripts/seed-sqlite.ps1` 负责从可提交的 seed 源写入初始数据。
- `scripts/sqlite/` 存放 SQLite Python 桥接脚本和 row mapper，避免在 PowerShell 或 Node.js 中维护大段内联 Python。
- `scripts/verify-sqlite-flows.ps1` 负责验证 SQLite 模式状态流转。
- `services/api/db/` 存放 Node.js 薄封装，负责调用 SQLite Python 桥接脚本。

## 3. 第一版表范围（2026-06-13 更新）

当前第一版最小落库只建 4 张核心表（对应 `001_initial_sqlite.sql`）：

```text
projects
agents
tasks
approvals
```

后续按功能阶段以 002、003… migration 追加：

```text
agent_relationships     -- Agent 父子关系（待 Agent 编排功能）
agent_config_applications -- Agent 配置变更申请（待审批链路完善）
agent_config_versions   -- Agent 配置版本历史（待审批链路完善）
runner_jobs             -- Runner 队列（待 Runner 启用）
workflows               -- 工作流定义（待工作流可视化）
knowledge_updates       -- 知识库更新（待知识库功能）
git_checkpoints         -- Git 保存点（待 Runner 启用）
runtime_events          -- 审计事件（待状态机完善）
runner_status           -- Runner 连接状态（待 Runner 启用）
```

暂不建（跨阶段）：

```text
users / teams / memberships
api_keys
model_calls
billing_records
cloud_sync_jobs
runner_execution_logs
```

## 4. Seed 数据来源

当前权威初始数据仍是：

```text
services/api/mock-data.js
```

当前 seed 先使用一个可提交的 JSON 快照，而不是让 SQL 脚本直接解析 JavaScript：

```text
data/seed/project_agent_swarm.seed.json
```

生成规则：

- 从 `mock-data.js` 导出的 `project`、`agents`、`tasks`、`approvals`、`workflows`、`runnerStatus`、`gitCheckpoints`、`knowledgeUpdates`、`usage`、`integrations`、`settings` 生成 seed 快照。
- `runnerJobs` 和 `agentConfigApplications` 初始为空数组。
- `status` 映射属于前端/展示层辅助数据，不进入第一版数据库表。
- `usage`、`integrations`、`settings` 第一版仍可继续留在 Mock 数据或 JSON 字段，不急着拆表。

## 5. 字段映射原则

JavaScript camelCase 到数据库 snake_case 的映射必须集中在 mapper 中，不要分散写在 API handler 里。

示例：

```text
assignedAgentId      -> assigned_agent_id
riskLevel            -> risk_level
relatedFiles         -> related_files
requiresApproval     -> requires_approval
canSpawnSubAgents    -> can_spawn_sub_agents
maxSubAgents         -> max_sub_agents
parentAgentId        -> parent_agent_id
childAgentIds        -> agent_relationships
reportsToAgentId     -> reports_to_agent_id
spawnDepth           -> spawn_depth
operationTypes       -> operation_types
affectedFiles        -> affected_files
diffPreview          -> diff_preview
requiresSecondConfirm -> requires_second_confirm
```

JSON 字段第一版以 JSON 字符串保存，读取时在 mapper 中还原为数组或对象。

## 6. 初始化流程（2026-06-13 更新）

当前状态：

1. `.gitignore` 已忽略 `data/local/` 和 SQLite wal/shm 文件。
2. `data/migrations/001_initial_sqlite.sql` 已更新为最小落库版本（4 张表）。
3. `data/seed/project_agent_swarm.seed.json` 保留旧 Mock 快照作为参考。
4. `scripts/init-sqlite.ps1`、`scripts/seed-sqlite.ps1`、`scripts/verify-sqlite-flows.ps1` 待新架构 Tauri/Rust 后端搭建后重写。
5. `scripts/sqlite/` 旧 Python 桥接脚本待评估是否需要在新架构中保留。

后续实现顺序（新架构）：

1. Tauri + Rust 后端搭建后，通过 `rusqlite` 执行 `001_initial_sqlite.sql`。
2. 从 `data/seed/project_agent_swarm.seed.json` 提取 4 张核心表的初始数据写入。
3. 实现 Rust 侧读写层，仅覆盖 projects / agents / tasks / approvals。
4. 后续追加 migration 时同步扩展 seed 和读写层。

## 7. Seed 写入顺序

建议按以下顺序写入，避免外键和引用顺序混乱：

1. `projects`
2. `agents`
3. `tasks`
4. `approvals`

后续追加表时按依赖关系插入合适位置。

第一版不写 `runtime_events`，`seed_completed` 等 `runtime_events` 表追加后再补。

## 8. Runtime State 迁移规则（2026-06-13 更新）

当前运行态文件 `data/mock/runtime-state.json` 仍为 Mock 模式下使用。

新架构下迁移到 SQLite 后：

- Task 状态变化写入 `tasks`。
- Approval 状态变化写入 `approvals`。
- `runtime_events` 表在当前第一版暂不建，待状态机完善后以独立 migration 追加。
- Runner job、Agent 配置应用等功能当前不实现，对应表待后续 migration。

旧 Node.js API（`POST /api/runtime-state/reset` 等）随旧原型冻结，新架构下由 Tauri commands 接管。

## 9. 新架构接入策略

1. Tauri + Rust 后端搭建后，通过 `rusqlite` 直接执行 migration。
2. 先覆盖 projects / agents / tasks / approvals 四张表的读写。
3. 前端通过 Tauri invoke 调用，不直接访问 SQLite。
4. 旧 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 环境变量开关不再适用，由 Tauri 宿主统一管理。
5. 后续追加表时同步扩展 Tauri commands 和前端类型。

## 10. 验证标准

新架构下第一版验证：

```powershell
# Rust 编译检查（后续 Tauri 项目搭建后启用）
cargo check

# 前端类型检查
cd packages/ui && npx tsc -b

# Migration 语法检查
sqlite3 :memory: < data/migrations/001_initial_sqlite.sql
```

验收点（当前第一版 4 表最小落库）：

- Migration 在 SQLite 内存库执行成功，projects / agents / tasks / approvals 四张表和索引均创建，外键正常。
- `data/local/`、SQLite 文件、runtime state 和日志不进入 Git。

旧 Node.js Mock/SQLite 回归（`verify-mock-flows.ps1`、`verify-sqlite-flows.ps1`）属于旧原型冻结验证，不作为新架构第一版数据库验收。

## 11. 风险与约束

- SQLite 第一版只能作为本地开发和单机运行态，不代表云端多用户并发方案。
- 不要在 API 契约里暴露 SQLite 特有行为，例如 rowid、PRAGMA、文件路径。
- 不要把 `data/mock/runtime-state.json` 和 SQLite 数据库同时作为可写权威源。
- 不要让 seed 脚本读取 `_internal/`、`design/image2/`、日志或密钥文件。
- 不要在 seed 中写入真实 API Key、真实本地用户凭据或客户数据。
