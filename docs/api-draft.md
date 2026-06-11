# agent蜂群 API 草案

日期：2026-06-11

这是一份当前态 API 草案，只保留现在真的在用、或者已经作为 helper/禁用态契约存在的路由和边界。实现细节和更完整的 route 说明放在 [services/api/README.md](../services/api/README.md)。

## 当前边界

- 仍然是 Mock / SQLite 优先。
- 不开放真实 Runner 执行。
- 不开放真实模型调用。
- 不做云同步。
- 不做完整权限系统。
- 所有本地写入、审批、Runner 请求、Agent 配置变化都必须先经过 Approval Service。

## 共享状态码

### ApprovalStatus

`draft`, `pending`, `approved`, `rejected`, `patch_only`, `executed`, `rolled_back`, `expired`

### TaskStatus

`queued`, `running`, `blocked`, `waiting_user`, `completed`, `failed`, `cancelled`

### AgentStatus

`running`, `idle`, `waiting`, `failed`, `disabled`

## 目前的核心契约

### 项目计划闭环

- `POST /api/projects/:projectId/project-plan-requests`
- `POST /api/approvals/:approvalId/approve`

行为要点：

- 只能由本地确定性模板生成 `project_plan` 草案。
- 草案阶段不能创建任务，也不能创建 Runner request queue 记录。
- `targetService=project_plan` 的审批通过后，只会生成 5 个 queued 任务和 5 条只读 Runner request 记录。
- 这些 Runner request 记录必须是 `runner_request_readonly`，不能执行命令、写文件、改 Git、发网络请求、调用模型或触发 Agent。

### Agent 配置安全闭环

- `POST /api/agents/:agentId/change-requests`
- `GET /api/projects/:projectId/agent-config-applications`
- `POST /api/agent-config-applications/:applicationId/dry-run`
- `POST /api/agent-config-applications/:applicationId/apply`
- `POST /api/agent-config-applications/:applicationId/cancel`
- `POST /api/agent-config-applications/:applicationId/rollback-request`
- `GET /api/agents/:agentId/config-version-history`

行为要点：

- `changeType=permission` 先走 `services/api/agent-permissions.js` 的 mock profile 校验。
- 安全 profile 可以创建 pending `agent_config` 审批；禁止 capability、未知 capability、`all=true` 都要拒绝。
- `dry-run` 默认仍是禁用态预览，只返回 write plan / rollback plan / `feature_disabled`，不写配置、不写版本、不生成 Runner job。
- `apply` 默认仍是状态流转，不直接改 Agent 当前态；真实写入只有在 feature flag + dry-run proof + 二次确认 + Git checkpoint + rollback acceptance 都满足时才可能开启。
- `rollback-request` 和 `config-version-history` 都是只读预览，不得直接创建审批或覆盖版本。

### Model Gateway 禁用边界

- `GET /api/model-gateway/status`
- `POST /api/model-gateway/dry-run`
- `POST /api/model-gateway/connectivity-test`

行为要点：

- 仍然只做禁用态信息展示、预演和预检，不调用真实 provider。
- `connectivity-test` 必须保持 `blocked / feature_disabled / realProviderRequestAttempted=false`。
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 目前只是一项被报告的请求标志，不会把真实 provider 请求打开。
- `openai_compat` 只能作为禁用态 relay 元数据出现。
- `services/api/model-gateway-project-plan.js` 只是 helper-only 的 `project_plan_generation` 准入构造器，不接 route，不写状态，不调用 provider。

阶段 2 未来入口草案：

- 候选 route：`POST /api/projects/:projectId/project-plan-model-requests`
- 当前状态：未实现。
- 未来行为：只能通过后端 Model Gateway 生成 `project_plan_generation` 模型调用，并且只把结构化结果写入 `project_plan` 审批草案。
- 禁止：客户端 API key、base URL、headers、provider body、prompt template、system prompt、stream、tools、files、Runner job id。
- Provider 配置只允许后端配置来源；状态响应不得返回 key、key suffix、masked key fragment 或 base URL 原文。

## 常规只读路由

当前实现里还保留这些读取/展示路由：

- `GET /api/projects/:projectId/dashboard`
- `GET /api/projects/:projectId/agents`
- `GET /api/projects/:projectId/tasks`
- `GET /api/projects/:projectId/approvals`
- `GET /api/projects/:projectId/workflows`
- `GET /api/projects/:projectId/runner/status`
- `GET /api/projects/:projectId/runner/jobs`
- `GET /api/projects/:projectId/execution-requests`
- `GET /api/projects/:projectId/runtime-events`
- `GET /api/projects/:projectId/git/checkpoints`
- `GET /api/projects/:projectId/knowledge/updates`
- `GET /api/projects/:projectId/usage`
- `GET /api/runtime-state`
- `GET /api/projects/:projectId/settings`
- `GET /api/health`

以及基础变更路由：

- `PATCH /api/agents/:agentId`
- `PATCH /api/tasks/:taskId/status`
- `POST /api/tasks/:taskId/start`
- `POST /api/tasks/:taskId/complete`
- `POST /api/tasks/:taskId/fail`
- `POST /api/tasks/:taskId/cancel`
- `POST /api/runner/jobs/:jobId/review`
- `POST /api/runner/jobs/:jobId/start`
- `POST /api/runner/jobs/:jobId/pause`
- `POST /api/runner/jobs/:jobId/complete`
- `POST /api/runner/jobs/:jobId/fail`
- `POST /api/runner/jobs/:jobId/cancel`
- `POST /api/runner/jobs/:jobId/block`
- `POST /api/approvals/:approvalId/reject`
- `POST /api/approvals/:approvalId/patch-only`
- `PATCH /api/projects/:projectId/settings`
- `POST /api/runtime-state/reset`
- `DELETE /api/runtime-state`

Runner request records are still read-only with respect to real execution, but the mock / SQLite API now allows lifecycle state changes through the `review`, `start`, `pause`, `complete`, `fail`, `cancel`, and `block` actions above. Every transition must be audited through `runtime_events`; none of these routes may execute local commands, write project files, or modify Git.

## Helper-only 模块

- `services/api/agent-permissions.js`
- `services/api/agent-config-fields.js`
- `services/api/agent-config-transaction-plan.js`
- `services/api/agent-config-rollback-request.js`
- `services/api/agent-config-version-history.js`
- `services/api/model-gateway.js`
- `services/api/model-gateway-adapters.js`
- `services/api/model-gateway-project-plan.js`
- `services/api/project-plan.js`

这些 helper 都必须维持全 false sideEffects，不能偷偷写 SQLite、写 runtime state、创建审批、创建 Runner job、执行 Runner、调用真实模型或读取原始密钥。

## 相关验收

- `scripts/verify-project-plan-flow.ps1`
- `scripts/verify-mock-flows.ps1`
- `scripts/verify-sqlite-flows.ps1`
- `scripts/verify-model-gateway.ps1`
- `scripts/verify-real-model-admission.ps1`
- `scripts/verify-agent-permissions.ps1`
- `scripts/verify-agent-config-safety-loop.ps1`

## 现阶段不做的事

- 真正的 Runner 执行。
- 真正的模型调用。
- 真正的回滚写入。
- 完整权限系统。
- 云同步。
