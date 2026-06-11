# scripts

项目脚本目录。

后续可以放：

- 本地开发启动脚本。
- mock API 启动脚本。
- 数据导出脚本。
- 日志归档脚本。

脚本文件名和路径尽量使用英文 ASCII。

## 当前脚本

```text
start-mock-api.ps1
start-dev.ps1
start-local.ps1
status-local.ps1
stop-local.ps1
verify-mock-flows.ps1
verify-sqlite-flows.ps1
verify-model-gateway.ps1
verify-agent-permissions.ps1
verify-agent-config-fields.ps1
verify-agent-config-dry-run.ps1
verify-agent-config-apply-gate.ps1
verify-agent-config-transaction-plan.ps1
verify-agent-config-rollback-request.ps1
verify-agent-config-version-history.ps1
verify-local-ui.ps1
init-sqlite.ps1
seed-sqlite.ps1
sqlite/
```

启动 `services/api/server.js`。

`start-dev.ps1` 会启动 mock API 并打开 `apps/web/index.html`。

`start-local.ps1` 会启动 SQLite 模式 API 和本地 Web 静态服务，用于人类本地试用。

`status-local.ps1` 会检查本地试用版 API、Web、SQLite 数据库和 pid 状态。

`stop-local.ps1` 会停止 `start-local.ps1` 启动的本地试用进程，并清理对应 pid 文件。

端口约定：

- `8787` 保留给人类本地试用和手动开发入口，例如 `start-local.ps1`、`start-dev.ps1`、前端默认 API、`verify-local-ui.ps1` 和 `verify-model-gateway.ps1` 对当前运行服务的检查。
- AI 自启动的自动验收脚本不得复用 `8787`，也不得连接一个已经存在的未知进程。
- `verify-sqlite-flows.ps1` 使用隔离端口 `8788`；`verify-mock-flows.ps1` 使用隔离端口 `8789`。如果对应端口启动前已经有 API 响应，脚本会直接失败，而不是误连旧服务。

`verify-mock-flows.ps1` 会在隔离端口 `8789` 启动 Mock API，验证 Mock API 的关键状态流转，并在结束后重置本地 runtime state。它还会检查：非法 Agent permission change request 在创建审批前被拒绝；已批准的 Agent config 变更只创建 `pending_apply` application 记录，不创建 Runner job，也不进行真实 Agent 配置写入；Agent config dry-run 保持 feature-disabled，且所有 sideEffects 为 false。

`verify-sqlite-flows.ps1` 会在隔离端口 `8788` 启动 SQLite 模式 API，验证 Dashboard、任务、审批、Runner job、Agent 配置应用/取消和 reset 状态重建。它还会检查：非法 Agent permission change request 在 SQLite 写入前被拒绝；已批准的 Agent config 变更只创建 `pending_apply` application 记录，不创建 Runner job，也不进行真实 Agent 配置写入；Agent config dry-run 保持 feature-disabled，且所有 sideEffects 为 false。

`verify-model-gateway.ps1` 会验证当前已运行 API 的 Model Gateway 禁用态、dry-run、connectivity-test disabled stub、preflight failure paths、disabled adapter registry、openai_compat relay interface、cheng.pink request builder、feature flag 边界和全 false sideEffects。该脚本不打开浏览器、不读取真实 key、不发真实 provider 请求，也不启动或停止本地服务。

`verify-agent-permissions.ps1` 验证本地 Agent 权限 profile helper：展开 mock profile，拒绝 `all=true`、未知 capability 和禁止 capability，并确认所有验证 sideEffects 都是 false。它不会启动本地服务、修改 Agent 配置、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`verify-agent-config-fields.ps1` 验证未来 Agent 配置真实写入前的字段白名单 helper。覆盖允许字段、未支持字段、禁止字段名、secret/prompt/provider/local-path 等禁止值、非法字段形状、禁止权限 capability、`all=true`，以及 dry-run/apply-gate 集成。它不会写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`verify-agent-config-dry-run.ps1` 验证本地 Agent 配置 dry-run helper。覆盖禁用态预览、缺少二次确认、缺少确认文本、非 `pending_apply` application、未批准来源审批、来源审批带 Runner job、错误 target service、缺少目标 Agent，以及全 false sideEffects。它不会写 Agent 配置、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`verify-agent-config-apply-gate.ps1` 验证未来真实应用 Agent 配置前的闸门 helper。它证明真实 apply 的前置条件可以被检查，但功能闸门仍关闭：即使输入有效，也必须保持 `ok=false`、`gateReady=false`、`canApply=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。它不会写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`verify-agent-config-transaction-plan.ps1` 验证未来真实写入的事务计划 helper。它证明计划写入集将来必须在一个事务内更新 `agents`、插入 `agent_config_versions`、标记 application applied、插入 `runtime_events`，但当前仍保持 `canWrite=false` 和全 false sideEffects。它不会写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`verify-agent-config-rollback-request.ps1` 验证直接 helper 级别的 Agent 配置回滚请求契约。它证明有效输入可以起草未来的审批/application/版本，但当前仍必须保持 `ok=false`、`canCreateApproval=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。Mock 和 SQLite flow 脚本会覆盖禁用态 HTTP 路由；在真实版本历史接入前，该路由保持 `requestReady=false`。

`verify-agent-config-version-history.ps1` 验证只读的 Agent 配置版本历史来源 helper。它只规范化已经加载好的版本行，检查按目标 Agent 过滤、按版本排序、当前版本/恢复来源选择、snapshot 字段白名单和禁止字段/值。它不会直接读 SQLite、暴露 HTTP 路由、写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型或读取密钥。

`init-sqlite.ps1` 会创建本地 SQLite 数据库并应用 `data/migrations/001_initial_sqlite.sql`。

`seed-sqlite.ps1` 会从 `data/seed/project_agent_swarm.seed.json` 重建第一版 SQLite 初始数据。

`sqlite/` 存放 SQLite Python 桥接脚本和 row mapper；PowerShell 和 Node.js 只负责传入路径、命令和参数。

SQLite 数据库文件位于 `data/local/`，该目录是本地运行态，不提交。

本地 Demo 验收步骤见：

```text
../docs/demo-checklist.md
```
## verify-local-ui.ps1

`verify-local-ui.ps1` validates the currently running local SQLite trial:

- API health, runtime state, and local trial safety flags.
- Microsoft Edge + Playwright CLI smoke coverage for overview, tasks, approval, runtime, settings, and integrations pages, including the Model Gateway dry-run preview.
- Browser console must report 0 errors and 0 warnings.

The script expects `scripts/start-local.ps1` to already be running. It does not start or stop the local trial service, does not call real model providers, and does not execute Runner jobs.

Future real Model Gateway manual connectivity testing must remain behind an explicit disabled-by-default gate. The current `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` env var may only be reported as requested; it must not make real provider requests active. No script in this directory should require real provider API keys, install provider SDKs, call OpenAI/Anthropic/Gemini, store provider responses, or turn disabled stub checks into real connectivity checks.

Provider adapter acceptance, request/response shape checks, redaction rules, cheng.pink request builder checks, feature flag boundary checks, and no-side-effect guarantees live in `verify-model-gateway.ps1`. `verify-local-ui.ps1` should stay focused on browser-rendered UI smoke coverage and must not perform real connectivity tests or depend on real provider credentials.

Model Gateway backend logic lives in `../services/api/model-gateway.js`; `server.js` should only wire routes to that module.

## verify-model-gateway.ps1

`verify-model-gateway.ps1` is the dedicated Model Gateway acceptance entry for the currently running local API:

- `GET /api/model-gateway/status` must stay disabled and expose only env var names / booleans, provider adapter metadata, and safety flags.
- `POST /api/model-gateway/dry-run` must remain a no-provider-call dry-run with all side effects false.
- `POST /api/model-gateway/connectivity-test` must remain blocked through the disabled adapter stub with `realProviderRequestAttempted=false`.
- `modelGatewayConnectivityPreflight(...)` failure paths are checked through direct backend helper calls without real credentials.
- Disabled adapter registry entries for OpenAI, Anthropic, Google Gemini, and `openai_compat` are checked as metadata only.
- The `openai_compat` relay interface remains `interface_disabled`.
- The cheng.pink request builder checks URL normalization, fixed `gpt-5.4-mini` body shape, unsafe URL rejection, unsupported model rejection, client input rejection, and all-false side effects.
- Setting `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST=true` in the script process must still keep `manualConnectivityTestActive=false` and `realProviderRequestsAllowed=false`.

This script is acceptance verification, not a real connectivity test. It must not require API keys, import provider SDKs, call OpenAI/Anthropic/Gemini/DeepSeek/cheng.pink, read `data/local/` directly, write runtime state, create tasks/approvals/Runner jobs, or log prompt/result/provider body content.

## verify-agent-permissions.ps1

`verify-agent-permissions.ps1` is the dedicated Agent permission profile contract check.

- `services/api/agent-permissions.js` remains a local helper only; it is not wired into API routes or runtime authorization.
- Built-in profiles must expand to explicit capabilities and avoid all forbidden Agent capabilities.
- `architect_admin` and `all_agents_full_management` may include broad planning/orchestration/request authority, but must not include self-approval, high-risk approval, direct Runner execution, direct file/command/Git/network operations, or raw secret access.
- Invalid contracts such as unsupported profiles, `all=true`, unknown capabilities, direct execution capabilities, and raw-secret capabilities must be rejected.
- Every validation result must keep side effects false: no SQLite/runtime-state writes, no tasks, no approvals, no Runner jobs, no Agent triggers, no Runner execution, no real model calls, and no raw-secret reads.

## verify-agent-config-dry-run.ps1

`verify-agent-config-dry-run.ps1` 是 Agent 配置 dry-run helper 的专用契约检查。

- `services/api/server.js` 导出 `buildAgentConfigApplyDryRun(...)`，用于不启动 API server 的本地 helper 验证。
- 有效的 pending application 仍必须返回 `ok=false`、`canApply=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。
- 无效 dry-run 输入必须在无 sideEffects 的情况下报告：缺少二次确认、缺少确认文本、非 pending application、未批准审批、审批带 Runner job、错误 target service、缺少目标 Agent。
- 这个 helper 检查不会启动本地服务、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型、读取 raw secret 或修改 Agent 配置。

## verify-agent-config-fields.ps1

`verify-agent-config-fields.ps1` 是 Agent 配置 change-plan 字段白名单的专用检查。

- `services/api/agent-config-fields.js` 负责未来真实写入允许字段：`permissions`、`model`、`status`、`maxSubAgents`、`canSpawnSubAgents`。
- 禁止字段包括 secret/API key、provider header/response、prompt、Runner/tool/command/file/Git/network 字段、workspace 路径、父子/汇报关系字段，以及宽泛 raw-secret token。
- Permission 变更仍使用 `services/api/agent-permissions.js`，因此 `canExecuteRunnerJob`、raw secret access、`all=true` 等禁止 capability 会被拒绝。
- `buildAgentConfigApplyDryRun(...)` 和 `buildAgentConfigRealApplyGate(...)` 会包含字段验证结果，但真实写入 sideEffects 仍保持 false。
- 这个 helper 检查不会启动本地服务、写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型、读取 raw secret 或修改 Agent 配置。

## verify-agent-config-apply-gate.ps1

`verify-agent-config-apply-gate.ps1` 是未来真实 apply 闸门的专用契约检查。

- `services/api/server.js` 导出 `buildAgentConfigRealApplyGate(...)`，用于不启动 API server 的本地 helper 验证。
- 有效前置条件可以返回 `preconditionsReady=true`，但仍必须返回 `ok=false`、`gateReady=false`、`canApply=false` 和 `blockedReasons=["feature_disabled"]`。
- 闸门要求匹配且无 sideEffects 的 dry-run 结果、已批准的 `agent_config` 来源审批、无 Runner job、目标 Agent、二次确认、requestedBy、Git checkpoint 和 rollback-plan acceptance。
- 无效 gate 输入必须在无 sideEffects 的情况下报告：缺少 requestedBy、缺少 Git checkpoint、缺少 rollback acceptance、缺少 dry-run proof、dry-run proof 不匹配、dry-run validation errors、dry-run sideEffects、来源审批带 Runner job。
- 这个 helper 检查不会启动本地服务、写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型、读取 raw secret 或修改 Agent 配置。

## verify-agent-config-transaction-plan.ps1

`verify-agent-config-transaction-plan.ps1` 是未来真实写入事务计划的专用契约检查。

- `services/api/agent-config-transaction-plan.js` 负责后续真实 apply 实现前的 helper-only 事务计划。
- 有效计划可以返回 `planReady=true`，但仍必须返回 `ok=false`、`canWrite=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。
- 计划写入集必须是一个事务：更新 `agents`、插入 `agent_config_versions`、标记 `agent_config_applications` applied、插入 `runtime_events`。
- 计划必须要求版本号严格 +1、`agent_id + version` 唯一、任意失败即 rollback、写入时 application 仍是 pending、来源 `agent_config` 审批已批准且没有 Runner job。
- 这个 helper 检查不会启动本地服务、写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型、读取 raw secret 或修改 Agent 配置。

## verify-agent-config-rollback-request.ps1

`verify-agent-config-rollback-request.ps1` 是 Agent 配置回滚请求的专用契约检查。

- `services/api/agent-config-rollback-request.js` 负责后续回滚 flow 的禁用态回滚请求草稿。
- 有效请求可以返回 `requestReady=true`，但仍必须返回 `ok=false`、`canCreateApproval=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。
- helper 要求原 application 已 applied、来源 `agent_config` 审批已批准且没有 Runner job、目标 Agent 存在、current/restore 版本属于目标 Agent、restore 版本早于 current 版本、二次确认、requester、reason，以及至少一个变更字段。
- `POST /api/agent-config-applications/:applicationId/rollback-request` 是禁用态预览路由，由 `verify-mock-flows.ps1` 和 `verify-sqlite-flows.ps1` 覆盖；真实版本历史存在前，普通路由调用保持 `requestReady=false`。
- 回滚必须起草新的审批、新 application 和未来新版本。它不得删除或覆盖版本历史、直接更新 `agents`、创建审批、创建 application、创建 Runner job、执行 Runner、调用模型、写 SQLite/runtime state 或读取 raw secret。

## verify-agent-config-version-history.ps1

`verify-agent-config-version-history.ps1` 是 Agent 配置版本历史只读来源的专用检查。

- `services/api/agent-config-version-history.js` 负责后续版本历史/回滚来源读取前的 helper-only 规范化。
- helper 接收已经加载好的版本行，支持 camelCase 和 SQLite 风格 snake_case 字段，解析 JSON snapshot/change，按目标 Agent 过滤，按版本倒序排序，并默认选择最新旧版本作为 restore source。
- snapshot 输出只允许 `permissions`、`model`、`status`、`maxSubAgents`、`canSpawnSubAgents`；secret/prompt/provider/local-path/Runner/tool/command/file/Git/network/workspace 等禁止字段和值必须被拒绝。
- 这个 helper 检查不会启动本地服务、直接读 SQLite、暴露 HTTP 路由、写 Agent 配置、写版本、写 SQLite/runtime state、创建审批或 Runner job、执行 Runner、调用模型、读取 raw secret 或修改 Agent 配置。
