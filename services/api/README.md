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
model-gateway.js
model-gateway-adapters.js
```

`agent-permissions.js` owns the mock Agent permission profile boundary. `POST /api/agents/:agentId/change-requests` validates `changeType=permission` before creating an approval. Safe profiles create an Agent config approval with `permissionValidation` recorded in `changeRequest`; forbidden capabilities, unknown capabilities, unsupported profiles, and `all=true` return `422 agent_permission_validation_failed` without writing runtime state or SQLite.

`POST /api/agent-config-applications/:applicationId/dry-run` is implemented as a disabled Agent config apply preview. It reads the current application, source approval, and target Agent, then returns `dryRun=true`, `canApply=false`, `blockedReasons=["feature_disabled"]`, write/rollback plans, and all-false side effects. It must not write `agents`, `agent_config_versions`, SQLite/runtime state, approvals, Runner jobs, runtime events, call models, execute Runner, or read raw secrets.

`model-gateway.js` owns the disabled Model Gateway boundary: provider metadata, env var presence checks, dry-run validation, feature flag metadata, and the disabled connectivity-test stub. `model-gateway-adapters.js` owns the disabled provider adapter registry and stub for OpenAI, Anthropic, and Google Gemini. These modules must not import provider SDKs, make OpenAI/Anthropic/Gemini requests, write SQLite/runtime state, create tasks/approvals/Runner jobs, trigger Agents, or log prompts/results.

`AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` is currently a visible request flag only. Even when that environment variable is `true`, MVP-0.2 must keep `manualConnectivityTestActive=false` and `realProviderRequestsAllowed=false`.

Provider adapter work currently stops at the disabled adapter registry and stub. Future real adapters must stay behind the Model Gateway service boundary, enforce timeout and response-size limits, return only coarse redacted status fields, and be implemented one provider at a time. Do not add provider SDK imports or real provider network calls in this stage.

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
