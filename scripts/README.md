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
verify-agent-config-dry-run.ps1
verify-agent-config-apply-gate.ps1
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

`verify-mock-flows.ps1` 会在隔离端口 `8789` 启动 Mock API，验证 Mock API 的关键状态流转，并在结束后重置本地 runtime state。It also checks that invalid Agent permission change requests are rejected before approval creation, that approved Agent config changes only create `pending_apply` application records without Runner jobs or real Agent config writes, and that Agent config dry-run stays feature-disabled with all side effects false.

`verify-sqlite-flows.ps1` 会在隔离端口 `8788` 启动 SQLite 模式 API，验证 Dashboard、任务、审批、Runner job、Agent 配置应用/取消和 reset 状态重建。It also checks that invalid Agent permission change requests are rejected before SQLite writes, that approved Agent config changes only create `pending_apply` application records without Runner jobs or real Agent config writes, and that Agent config dry-run stays feature-disabled with all side effects false.

`verify-model-gateway.ps1` 会验证当前已运行 API 的 Model Gateway 禁用态、dry-run、connectivity-test disabled stub、preflight failure paths、disabled adapter registry、openai_compat relay interface、cheng.pink request builder、feature flag 边界和全 false sideEffects。该脚本不打开浏览器、不读取真实 key、不发真实 provider 请求，也不启动或停止本地服务。

`verify-agent-permissions.ps1` validates the local Agent permission profile helper. It expands mock profiles, rejects `all=true`, rejects unknown capabilities, rejects forbidden Agent capabilities, and checks all validation side effects stay false. It does not start local services, change Agent config, write SQLite/runtime state, create approvals/Runner jobs, execute Runner, call models, or read secrets.

`verify-agent-config-dry-run.ps1` validates the local Agent config dry-run helper without starting services. It covers blocked preview, missing second confirmation, missing confirm text, non-`pending_apply` application, unapproved source approval, source approval with a Runner job, wrong target service, missing target Agent, and all-false side effects. It does not write Agent config, write SQLite/runtime state, create approvals/Runner jobs, execute Runner, call models, or read secrets.

`verify-agent-config-apply-gate.ps1` validates the future real Agent config apply gate helper without starting services. It proves that all real-apply preconditions can be checked while the feature gate remains closed: even valid inputs keep `ok=false`, `gateReady=false`, `canApply=false`, `blockedReasons=["feature_disabled"]`, and all-false side effects. It does not write Agent config, write versions, write SQLite/runtime state, create approvals/Runner jobs, execute Runner, call models, or read secrets.

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

`verify-agent-config-dry-run.ps1` is the dedicated Agent config dry-run helper contract check.

- `services/api/server.js` exports `buildAgentConfigApplyDryRun(...)` for local helper verification without starting the API server.
- Valid pending applications must still return `ok=false`, `canApply=false`, `blockedReasons=["feature_disabled"]`, and all-false side effects.
- Invalid dry-run inputs must be reported without side effects: missing second confirmation, missing confirm text, non-pending application, unapproved approval, approval with Runner job, wrong target service, and missing target Agent.
- The helper check does not start local services, write SQLite/runtime state, create approvals/Runner jobs, execute Runner, call models, read raw secrets, or mutate Agent config.

## verify-agent-config-apply-gate.ps1

`verify-agent-config-apply-gate.ps1` is the dedicated future real-apply gate contract check.

- `services/api/server.js` exports `buildAgentConfigRealApplyGate(...)` for local helper verification without starting the API server.
- Valid preconditions may return `preconditionsReady=true`, but must still return `ok=false`, `gateReady=false`, `canApply=false`, and `blockedReasons=["feature_disabled"]`.
- The gate requires a matching no-side-effect dry-run result, approved `agent_config` source approval, no Runner job, target Agent, second confirmation, requestedBy, Git checkpoint, and rollback-plan acceptance.
- Invalid gate inputs must be reported without side effects: missing requestedBy, missing Git checkpoint, missing rollback acceptance, missing dry-run proof, mismatched dry-run proof, dry-run validation errors, dry-run side effects, and source approval with Runner job.
- The helper check does not start local services, write Agent config, write versions, write SQLite/runtime state, create approvals/Runner jobs, execute Runner, call models, read raw secrets, or mutate Agent config.
