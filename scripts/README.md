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

`verify-mock-flows.ps1` 会验证 Mock API 的关键状态流转，并在结束后重置本地 runtime state。

`verify-sqlite-flows.ps1` 会在独立端口启动 SQLite 模式 API，验证 Dashboard、任务、审批、Runner job、Agent 配置应用/取消和 reset 状态重建。

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

- API health, runtime state, Runner safety flags, `GET /api/model-gateway/status`, `POST /api/model-gateway/dry-run`, and the disabled `POST /api/model-gateway/connectivity-test` stub.
- Model Gateway must stay disabled and must not expose keys, write storage, create tasks, create approvals, create Runner jobs, trigger Agents, log prompts/results, or make provider network requests.
- Model Gateway feature flag metadata must expose `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST`, but `manualConnectivityTestActive` and `realProviderRequestsAllowed` must stay `false`.
- Microsoft Edge + Playwright CLI smoke coverage for overview, tasks, approval, runtime, settings, and integrations pages, including the Model Gateway dry-run preview.
- Browser console must report 0 errors and 0 warnings.

The script expects `scripts/start-local.ps1` to already be running. It does not start or stop the local trial service, does not call real model providers, and does not execute Runner jobs.

Future real Model Gateway manual connectivity testing must remain behind an explicit disabled-by-default gate. The current `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` env var may only be reported as requested; it must not make real provider requests active. No script in this directory should require real provider API keys, install provider SDKs, call OpenAI/Anthropic/Gemini, store provider responses, or turn disabled stub checks into real connectivity checks.

Provider adapter acceptance currently checks only the disabled adapter registry and stub. Verification scripts may check disabled adapter boundaries, request/response shapes, redaction rules, and no-side-effect guarantees, but they must not perform real connectivity tests or depend on real provider credentials.

`verify-local-ui.ps1` also checks the cheng.pink OpenAI-compatible relay request builder through direct backend helper calls. This builder check is local only: it verifies URL normalization, fixed non-stream ping body shape, unsupported model rejection, unsafe base URL rejection, and all-false side effects. It must not read real API keys, call the relay, store provider responses, or log prompt/result content.

Model Gateway backend logic lives in `../services/api/model-gateway.js`; `server.js` should only wire routes to that module.
