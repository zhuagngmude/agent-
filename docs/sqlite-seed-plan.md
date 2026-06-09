# SQLite 初始化与 Seed 方案

日期：2026-06-09

阶段：MVP-0.2 数据库实现前设计。

本文只设计本地 SQLite 初始化和 seed 方案，不创建数据库文件，不实现真实后端，不修改 Mock API 运行逻辑。

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
services/api/db/
```

说明：

- `data/local/` 是本地数据库运行态目录，必须加入 `.gitignore`。
- `data/migrations/*.sql` 是可提交的 schema 迁移脚本。
- `scripts/init-sqlite.ps1` 负责创建数据库和应用迁移。
- `scripts/seed-sqlite.ps1` 负责从可提交的 seed 源写入初始数据。
- `services/api/db/` 后续放数据库连接、查询和 row mapper，当前尚未创建。

## 3. 第一版表范围

第一版初始化只建当前 Mock API 必需表：

```text
projects
agents
agent_relationships
tasks
approvals
runner_jobs
agent_config_applications
workflows
knowledge_updates
git_checkpoints
runtime_events
runner_status
```

暂不建：

```text
users
teams
memberships
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

## 6. 初始化流程

建议后续实现顺序：

1. 已新增 `.gitignore` 规则，忽略 `data/local/` 和 SQLite wal/shm 文件。
2. 已新增 `data/migrations/001_initial_sqlite.sql`，创建第一版表和索引。
3. 已新增 `scripts/init-sqlite.ps1`，创建 `data/local/agent-swarm.sqlite` 并执行 migration。
4. 已新增 `data/seed/project_agent_swarm.seed.json`，保存从 Mock 数据导出的初始快照。
5. 已新增 `scripts/seed-sqlite.ps1`，清空第一版表并按依赖顺序写入 seed。
6. 已新增只读数据库查询层，可通过 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 让 Dashboard、Agents、Tasks、Approvals、Workflows、Runner status/jobs、Agent config applications、Git checkpoints 和 Knowledge updates 从 SQLite 读取。
7. 下一步迁移任务、审批、Agent 配置应用/取消等状态流转。

## 7. Seed 写入顺序

建议按以下顺序写入，避免外键和引用顺序混乱：

1. `projects`
2. `agents`
3. `agent_relationships`
4. `tasks`
5. `approvals`
6. `runner_jobs`
7. `agent_config_applications`
8. `workflows`
9. `runner_status`
10. `knowledge_updates`
11. `git_checkpoints`
12. `runtime_events`

第一版 seed 可以不写 `runtime_events` 历史，只写一条 `seed_completed` 事件，后续状态流转再开始完整记录 before/after。

## 8. Runtime State 迁移规则

当前运行态文件：

```text
data/mock/runtime-state.json
```

迁移到 SQLite 后，语义对应如下：

- Approval 状态变化写入 `approvals`，并新增 `runtime_events`。
- Task 状态变化写入 `tasks`，并新增 `runtime_events`。
- Runner job 队列写入 `runner_jobs`，但仍只读，不执行。
- Agent 配置应用/取消写入 `agent_config_applications`，并新增 `runtime_events`。

`POST /api/runtime-state/reset` 后续应改为：

1. 停止使用当前 SQLite 连接的写事务。
2. 清空第一版业务表。
3. 重新执行 seed。
4. 写入 `runtime_events(seed_reset)`。
5. 返回与当前 Mock reset 相同语义的结果。

## 9. API 切换策略

第一版不要一次性把所有 API 都切到数据库。

建议顺序：

1. `GET /api/projects/:projectId/dashboard`
2. `GET /api/projects/:projectId/agents`
3. `GET /api/projects/:projectId/tasks`
4. `GET /api/projects/:projectId/approvals`
5. `GET /api/projects/:projectId/workflows`
6. 任务 action API
7. 审批 action API
8. Agent 配置应用/取消 API

`GET /api/projects/:projectId/dashboard`、`/agents`、`/tasks`、`/approvals`、`/workflows`、`/agent-config-applications`、`/runner/status`、`/runner/jobs`、`/git/checkpoints`、`/knowledge/updates` 已支持环境变量开关的 SQLite 只读读取，默认仍使用 Mock 数据。

每迁移一组 API，都必须保持 response shape 与 `docs/api-draft.md` 和当前前端预期一致。

## 10. 验证标准

后续实现 SQLite 初始化和 seed 后，至少要通过：

```powershell
node --check apps\web\app.js
node --check services\api\server.js
node --check services\api\mock-data.js
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
git status --short
```

新增数据库脚本后还应补充：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\init-sqlite.ps1
powershell -ExecutionPolicy Bypass -File scripts\seed-sqlite.ps1
```

验收点：

- 重复执行初始化和 seed 不报错。
- Dashboard 数据与当前 Mock 初始数据一致。
- 任务状态流转、审批状态流转、Runner job 只读队列、Agent 配置应用/取消仍通过回归脚本。
- `data/local/`、SQLite 文件、runtime state 和日志不进入 Git。

## 11. 风险与约束

- SQLite 第一版只能作为本地开发和单机运行态，不代表云端多用户并发方案。
- 不要在 API 契约里暴露 SQLite 特有行为，例如 rowid、PRAGMA、文件路径。
- 不要把 `data/mock/runtime-state.json` 和 SQLite 数据库同时作为可写权威源。
- 不要让 seed 脚本读取 `_internal/`、`design/image2/`、日志或密钥文件。
- 不要在 seed 中写入真实 API Key、真实本地用户凭据或客户数据。
