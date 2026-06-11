# services/api

API 服务预留目录。

第一版接口契约见：

```text
../../docs/api-draft.md
```

后续可以先实现 mock API，再接 SQLite / PostgreSQL。

SQLite 初始化和 seed 方案见：

```text
../../docs/sqlite-seed-plan.md
```

## 本地 mock API

当前已提供纯 Node.js mock API：

```text
server.js
mock-data.js
agent-permissions.js
agent-config-fields.js
agent-config-transaction-plan.js
agent-config-rollback-request.js
agent-config-version-history.js
model-gateway.js
model-gateway-adapters.js
project-plan.js
```

`agent-permissions.js` 负责 mock Agent 权限 profile 边界。`POST /api/agents/:agentId/change-requests` 在创建审批前验证 `changeType=permission`。安全 profile 会创建 Agent 配置审批，并把 `permissionValidation` 记录到 `changeRequest`；禁止 capability、未知 capability、未支持 profile 和 `all=true` 会返回 `422 agent_permission_validation_failed`，不写 runtime state 或 SQLite。

`agent-config-fields.js` 负责 helper-only 的 Agent 配置 change-plan 白名单。当前允许的未来写入字段只有 `permissions`、`model`、`status`、`maxSubAgents`、`canSpawnSubAgents`。它会拒绝未支持字段、secret/API key/provider/prompt/local-path 内容、Runner/tool/command/file/Git/network/workspace 字段、父子/汇报关系字段、禁止 Agent capability 和 `all=true`。它不写 Agent 配置，也不持久化任何内容。

`POST /api/agent-config-applications/:applicationId/dry-run` 当前是禁用态的 Agent 配置 apply 预览。它读取当前 application、来源审批和目标 Agent，然后返回 `dryRun=true`、`canApply=false`、`blockedReasons=["feature_disabled"]`、write/rollback plan 和全 false sideEffects。它不得写 `agents`、`agent_config_versions`、SQLite/runtime state、审批、Runner job、runtime event，不得调用模型、执行 Runner 或读取原始密钥。

`buildAgentConfigRealApplyGate(...)` 是 helper-only 的未来真实 apply 闸门。它可以证明 dry-run proof、来源审批、目标 Agent、二次确认、requestedBy、Git checkpoint 和 rollback acceptance 都存在，但 MVP-0.2 仍必须返回 `ok=false`、`gateReady=false`、`canApply=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。在后续明确 feature flag 的提交前，不得把它接到真实 Agent 配置写入。

`agent-config-transaction-plan.js` 负责 helper-only 的未来真实写入事务计划。它可以预览后续实现必须在一个事务内更新 `agents`、插入 `agent_config_versions`、标记 application applied、插入 `runtime_events`；但 MVP-0.2 仍必须保持 `canWrite=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。它不得直接调用 SQLite 或写 runtime state。

`agent-config-rollback-request.js` 负责禁用态的未来回滚请求草稿。`POST /api/agent-config-applications/:applicationId/rollback-request` 读取 application、来源审批、目标 Agent 和只读版本历史，然后返回 blocked 预览。无版本历史时路由保持 `requestReady=false` 并返回缺少版本的验证错误；feature-gated SQLite real apply 产生至少两个版本后，路由可返回 `requestReady=true`、current/restore 版本和 read-only restore diff。路由和 helper 必须保持 `ok=false`、`canCreateApproval=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。它们不得创建审批/application、写 Agent 配置、写版本、调用 SQLite 写入、写 runtime state、创建 Runner job、执行 Runner、调用模型或读取原始密钥。

`agent-config-version-history.js` 负责 Agent 配置版本历史只读来源。`GET /api/agents/:agentId/config-version-history` 在 Mock 模式返回空历史，在 SQLite 模式从 snapshot 只读 `agent_config_versions` 后交给 helper 规范化。它支持 camelCase 和 SQLite 风格 snake_case 字段，按目标 Agent 过滤，按版本排序，选择当前版本和可恢复版本，并只暴露允许的 config snapshot/change 字段。禁止字段和值会产生 validation error；禁止值必须 redacted。它不得写 Agent 配置、写版本、调用 SQLite 写入、写 runtime state、创建审批/application、创建 Runner job、执行 Runner、调用模型或读取原始密钥。

Feature-gated SQLite real apply is wired only through `POST /api/agent-config-applications/:applicationId/apply` when the API process has `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` and `AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY=true`. The request must include dry-run proof, `secondConfirm=true`, confirm text, `requestedBy`, Git checkpoint acknowledgement, and `rollbackPlanAccepted=true`. When the flag is absent, the route keeps the previous status-only apply behavior and must not write `agents` or `agent_config_versions`. The real apply command updates `agents`, inserts `agent_config_versions`, marks the application applied, and writes one runtime event in one SQLite transaction; it must still not create Runner jobs, execute Runner, call models, create approvals, or read raw secrets.

`model-gateway.js` owns the disabled Model Gateway boundary: provider metadata, env var presence checks, dry-run validation, feature flag metadata, and the disabled connectivity-test stub. `model-gateway-adapters.js` owns the disabled provider adapter registry and stub for OpenAI, Anthropic, and Google Gemini. These modules must not import provider SDKs, make OpenAI/Anthropic/Gemini requests, write SQLite/runtime state, create tasks/approvals/Runner jobs, trigger Agents, or log prompts/results.

`AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` is currently a visible request flag only. Even when that environment variable is `true`, MVP-0.2 must keep `manualConnectivityTestActive=false` and `realProviderRequestsAllowed=false`.

Provider adapter work currently stops at the disabled adapter registry and stub. Future real adapters must stay behind the Model Gateway service boundary, enforce timeout and response-size limits, return only coarse redacted status fields, and be implemented one provider at a time. Do not add provider SDK imports or real provider network calls in this stage.

`project-plan.js` owns the MVP-0.3 local project planning loop. `POST /api/projects/:projectId/project-plan-requests` builds a deterministic local `project_plan` approval from a user idea and constraints. The draft may persist an approval in Mock runtime state or SQLite, but it must not create tasks or Runner request records before approval. Approving that `project_plan` approval creates five queued tasks for `agent_frontend`, `agent_backend`, `agent_qa`, `agent_docs`, and `agent_reviewer`, plus five read-only Runner request queue records with `runner_request_readonly`. These queue records are not executable Runner jobs: they must not execute commands, write files, make network requests, modify Git, call models, trigger Agents, or read raw secrets. Mock and SQLite approval paths both validate that plan tasks and Runner requests have IDs, no duplicate IDs, and that each Runner request references a planned task and stays read-only.

启动：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-mock-api.ps1
```

默认地址：

```text
http://127.0.0.1:8787
```

健康检查：

```text
GET /api/health
```

## SQLite Dashboard 读取

当前默认仍从 Mock 内存数据读取。可通过环境变量只读试用 SQLite Dashboard 和第一批列表接口：

```powershell
$env:AGENT_SWARM_DASHBOARD_SOURCE="sqlite"
powershell -ExecutionPolicy Bypass -File scripts/start-mock-api.ps1
```

如果 `data/local/agent-swarm.sqlite` 不存在或查询失败，API 会回退到 Mock Dashboard。

当前 SQLite 只读开关覆盖：

```text
GET /api/projects/:projectId/dashboard
GET /api/projects/:projectId/agents
GET /api/projects/:projectId/tasks
GET /api/projects/:projectId/approvals
GET /api/projects/:projectId/workflows
GET /api/projects/:projectId/agent-config-applications
GET /api/projects/:projectId/runner/status
GET /api/projects/:projectId/runner/jobs
GET /api/projects/:projectId/git/checkpoints
GET /api/projects/:projectId/knowledge/updates
```

任务、审批和 Agent 配置申请/应用/取消写操作在 SQLite 模式下会写入 SQLite，并记录 `runtime_events`；默认 Mock 模式仍使用 `data/mock/runtime-state.json`。

在 SQLite 模式下：

- `GET /api/runtime-state` 会返回 `localTrial` 元信息，供设置页显示当前模式、API/Web 地址、状态文件位置、启动/停止/查看状态命令和安全边界。
- `POST /api/runtime-state/reset` 会重新执行 seed，重建 SQLite 状态。
- `DELETE /api/runtime-state` 不删除 SQLite 文件，只重置 seed 状态。
- 默认 Mock 模式仍使用 `data/mock/runtime-state.json`。

SQLite 模式回归验证：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify-sqlite-flows.ps1
```
