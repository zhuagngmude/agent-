# agent蜂群 API 草案

日期：2026-06-11

> 2026-06-17 更新：本文是旧 API / 状态契约草案，仍可作为字段和历史迁移参考，但不再完整代表当前产品状态。当前主线以 Tauri/Rust commands、Rust services、`data/migrations/`、`docs/Agent宪法.md`、`docs/AI开发细则.md` 和 `dev-docs/当前项目导航.md` 为准。真实模型调用和 Runner 主链路已经在受控入口中开放。

这是一份当前态 API 草案，只保留现在真的在用、或者已经作为 helper/禁用态契约存在的路由和边界。实现细节和更完整的 route 说明放在 [services/api/README.md](../services/api/README.md)。

## 当前边界

- 桌面端 Tauri/Rust 是当前主入口，旧 Node.js HTTP route 仅作历史参考。
- Runner 主链路允许在应用受控服务层内全自动执行，产物写入 `workspace/generated`。
- 真实模型调用允许经 Model Gateway 和系统设置中的模型配置发起。
- 不做云同步，不做完整权限系统。
- 禁止自由 shell、Git commit/push、文件删除、保护路径写入和提交密钥。
- 密钥、raw prompt、raw response、raw provider error 不得写入文档、SQLite 或日志。

## 共享状态码

### ApprovalStatus

`draft`, `pending`, `approved`, `rejected`, `patch_only`, `executed`, `rolled_back`, `expired`

### TaskStatus

`queued`, `running`, `blocked`, `waiting_user`, `completed`, `failed`, `cancelled`

### AgentStatus

`running`, `idle`, `waiting`, `failed`, `disabled`

## 目前的核心契约

### 新 Tauri/Rust project_plan commands（阶段 24 已实现）

阶段 24 在新架构中不继续扩展旧 Node.js HTTP route，而是通过 Tauri invoke 暴露明确 command：

- `create_project_plan_draft`
- `approve_project_plan`
- `list_project_plan_drafts`
- `list_runner_requests`

### 新 Tauri/Rust agent_config commands（P0 阶段 3 已实现）

AI 员工、执行器、模型目录和 Skill 配置通过 Tauri invoke 暴露，不走旧 Node.js HTTP route：

- `list_executor_configs`
- `upsert_executor_config`
- `delete_executor_config`
- `list_executor_models`
- `upsert_executor_model`
- `delete_executor_model`
- `list_agent_templates`
- `upsert_agent_template`
- `delete_agent_template`
- `list_project_agents`
- `upsert_project_agent`
- `remove_project_agent`
- `list_executor_skills`
- `upsert_executor_skill`
- `delete_executor_skill`
- `list_agent_boundary_checks`

行为要点：

- 普通 CRUD 只保存非敏感配置，API Key、Token、raw prompt、raw response 不进入 SQLite。
- 内置执行器和内置模型不能删除；被项目 Agent 或其他配置引用的记录会被依赖保护挡住。
- 项目 Agent 删除是软移除，保留历史链路，不直接断掉运行记录。
- `list_agent_boundary_checks` 只读，用来展示后端越界判断记录；真正的边界校验会在后续 Runner 调度阶段继续接入。

行为要点：

- 早期 `create_project_plan_draft` / `approve_project_plan` 的审批链路保留为历史参考；当前主控台全自动路径会自动生成/推进任务。
- 当前任务数量、模板和 Runner request 结构以 Rust service 与数据库迁移为准，不再以“固定 5 个任务 + 5 条只读 request”为当前真相。
- 通用 `approve_approval` 不得自动实例化 project plan，也不得通过 `project_plan` 审批；真正批准只能走 `approve_project_plan`。
- 历史 `runner_requests` 曾是只读队列；当前 Runner 主链路已经能进入受控执行。

### 项目计划闭环

- `POST /api/projects/:projectId/project-plan-requests`
- `POST /api/approvals/:approvalId/approve`

行为要点：

- 本节是旧 Node.js / Mock 原型契约，保留为迁移参考，不作为新 Tauri/Rust 主线继续扩展。
- 只能由本地确定性模板生成 `project_plan` 草案。
- 草案阶段不能创建任务，也不能创建 Runner request queue 记录。
- `targetService=project_plan` 的审批通过后，只会生成 5 个 queued 任务和 5 条只读 Runner request 记录。
- 这些 Runner request 记录必须是 `runner_request_readonly`，不能执行命令、写文件、改 Git、发网络请求、调用模型或触发 Agent。

### Agent Run 记录链

- `POST /api/projects/:projectId/agent-run-requests`
- `GET /api/projects/:projectId/agent-runs`

行为要点：

- 只生成本地可追踪的 Agent Run 记录链，不直接写项目文件、不改 Git、不创建任务、不创建 Runner job。
- 每条记录只保存摘要、模型、token、成本、错误和 parent chain 关系，不保存原始 prompt、provider payload 或密钥。
- `simulateFailureRole` 只用于本地失败注入，不会扩大权限，也不会触发真实 Runner。
- `GET /api/projects/:projectId/agent-runs` 只提供链路列表、选中链路和 runtime event 审计视图。

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
- `POST /api/projects/:projectId/project-plan-model-requests`

行为要点：

- 仍然只做禁用态信息展示、预演和预检，不调用真实 provider。
- `connectivity-test` 必须保持 `blocked / feature_disabled / realProviderRequestAttempted=false`。
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 目前只是一项被报告的请求标志，不会把真实 provider 请求打开。
- `openai_compat` 只能作为禁用态 relay 元数据出现。
- `services/api/model-gateway-project-plan.js` 是 helper-only 的 `project_plan_generation` 准入构造器，不写状态，不调用 provider。
- `services/api/model-gateway-provider-config.js` 只解析后端 provider 配置的粗粒度状态，不返回 raw key、key suffix、masked fragment、base URL 原文或 endpoint URL。
- `services/api/model-gateway-redaction.js` 只提供脱敏、限长和安全 `model_calls` 记录草稿；当前 `modelCallRecordReady=false`、`canWrite=false`。
- `services/api/model-gateway-model-calls.js` 只维护未来 `model_calls` 写入 / 迁移草案；当前 `modelCallRecordReady=false`、`canWrite=false`，不写 SQLite / runtime state / provider。

阶段 2 禁用态入口草案：

- route：`POST /api/projects/:projectId/project-plan-model-requests`
- 当前状态：已接入禁用态草案，只返回 `blocked / feature_disabled`。
- 当前行为：验证固定 `project_plan_generation` 请求形态，报告后端 provider 配置粗粒度状态，但不创建审批、不创建任务、不创建 Runner request、不写 runtime event、不写 `model_calls`、不调用 provider。
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
- `GET /api/projects/:projectId/agent-runs`
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
- `services/api/model-gateway-provider-config.js`
- `services/api/model-gateway-project-plan.js`
- `services/api/model-gateway-redaction.js`
- `services/api/model-gateway-model-calls.js`
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
