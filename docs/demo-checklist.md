# 本地 Demo 启动与验证清单

日期：2026-06-09

用途：给人类用户和后续 AI 一个可重复的本地验收入口。当前仍是 MVP-0.2 本地试用阶段，不调用真实模型、不让 Runner 执行本地命令。

## 1. 本地试用启动

推荐先用本地 SQLite 试用版：

```powershell
cd F:\projects\agent-swarm
powershell -ExecutionPolicy Bypass -File scripts\start-local.ps1
```

脚本会做四件事：

1. 如果 `data/local/agent-swarm.sqlite` 不存在，则从 seed 创建本地 SQLite 数据库。
2. 以 SQLite 模式启动 API：`http://127.0.0.1:8787`。
3. 启动 Web 静态服务：`http://127.0.0.1:5175/index.html`。
4. 打开浏览器访问本地 Web App。

查看状态：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\status-local.ps1
```

停止试用版：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\stop-local.ps1
```

如果后续要删除试用数据，删除 `data/local/` 即可；该目录不会进入 Git。

## 2. 开发 Mock 启动

在项目根目录执行：

```powershell
cd F:\projects\agent-swarm
powershell -ExecutionPolicy Bypass -File scripts\start-dev.ps1
```

脚本会做三件事：

1. 检查 `http://127.0.0.1:8787/api/health` 是否可用。
2. 如果 Mock API 未启动，则后台启动 `services/api/server.js`。
3. 打开 `apps/web/index.html`。

如果启动失败，先看日志：

```text
logs/mock-api.out.log
logs/mock-api.err.log
```

## 3. 快速健康检查

本地 API 默认地址：

```text
http://127.0.0.1:8787
```

端口约定：

- `8787` 是人类本地试用和手动开发默认入口，保留给 `start-local.ps1` / `start-dev.ps1` / 前端默认 API 使用。
- AI 自启动的自动验收脚本不能复用 `8787`，避免连到人类正在查看的旧服务或污染本地试用状态。
- `verify-mock-flows.ps1` 自启 Mock API 时使用隔离端口 `8789`；`verify-sqlite-flows.ps1` 自启 SQLite API 时使用隔离端口 `8788`。
- `verify-local-ui.ps1` 和 `verify-model-gateway.ps1` 是“当前本地试用服务检查”，期望 `start-local.ps1` 已经在 `8787` 运行；它们不启动或停止服务。

浏览器或 PowerShell 可检查：

```powershell
Invoke-RestMethod http://127.0.0.1:8787/api/health
Invoke-RestMethod http://127.0.0.1:8787/api/projects/project_agent_swarm/dashboard
```

预期：

- `/api/health` 返回 `ok=true`。
- Dashboard 返回 `project`、`metrics`、`pendingApprovals`、`taskQueue`、`agentStatus`、`runnerStatus`。
- Web App 顶部显示本地 API 已连接；本地试用版的数据实际由 SQLite 持久化。
- 如果 API 不可用，前端会回退到本地 `data.js`。

## 4. 页面验收点

推荐按这个顺序点一遍：

1. 首页：能看到项目阶段、指标卡、审批/任务摘要。
2. 审批页：能点选审批记录，查看风险、影响文件、diff 预览；按钮只改变 Mock 状态。
3. 任务页：能开始、完成、失败、取消任务；状态写入本地 runtime state。
4. 工作流页：只读展示工作流步骤、节点和依赖，不支持编辑或运行。
5. 运行与调度页：能查看 Runner job 队列、Runner 状态、权限边界和“不会执行本地命令”的安全说明。
6. 智能体页：能查看 Agent 详情、子 Agent 关系、配置变更预览、审批申请、待应用记录、Mock 应用/取消、应用审计和回滚前审查。
7. 设置页：能看到本地试用状态、SQLite/Mock 模式、状态文件位置、查看/停止命令；能导出、重置、清理本地状态。

本地试用体验小修后的补充验收：

- 页面中不应存在 `href="#"` 这类空跳转入口；能跳转到现有模块的入口应切换到对应页。
- 暂未接入的入口应显示为禁用状态，例如多项目切换、拓扑/依赖视图、Git 保存点详情、任务拆解、代码索引和审计日志。
- 任务页默认应选中第一条可操作任务，避免默认落在已完成任务导致动作按钮全部禁用。
- 审批页的批准按钮应明确表示“批准并生成只读 Runner job”，不得暗示真实 Runner 执行已经开放。
- 运行与调度页的 Runner job 应标记为只读队列；审批通过只生成排队记录，不执行命令、不写文件、不修改 Git。
- 设置页的恢复/清理按钮应说明只恢复 seed 状态；SQLite 模式不会删除数据库文件，不会停止本地服务，也不会执行 Runner。

## 5. 状态重置

本地 SQLite 试用版状态文件：

```text
data/local/agent-swarm.sqlite
```

注意：

- 这个文件是本地运行文件，不进入 Git。
- 试用时的任务、审批、Agent 配置应用/取消状态会保存在这里。
- 可以调用 reset 接口恢复 seed 初始状态。
- 设置页的“恢复 Seed 数据”和“清理运行态并恢复 Seed”在 SQLite 模式下都会重新写入 seed；不会删除 `data/local/agent-swarm.sqlite` 文件，也不会停止本地服务。

运行态文件：

```text
data/mock/runtime-state.json
```

注意：

- 这个文件是本地运行文件，不进入 Git。
- 删除它或在设置页清理状态，会回到初始 Mock 数据。
- 不要把它提交。

也可以调用：

```powershell
Invoke-RestMethod -Method Post http://127.0.0.1:8787/api/runtime-state/reset
```

## 6. 自动验证状态流转

可以运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-model-gateway.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-permissions.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-fields.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-dry-run.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-apply-gate.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-transaction-plan.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-rollback-request.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-version-history.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-local-ui.ps1
```

脚本会验证：

- Dashboard 聚合接口包含 Runner 状态。
- 任务可以 `start -> complete`。
- Runner 审批通过后只生成只读 Runner job。
- Agent 配置审批后可以走 Mock 应用状态流转。
- Agent 配置审批后可以走 Mock 取消状态流转。
- Agent 配置审批通过后必须只生成 `pending_apply` 待应用记录，`runnerJobId` 为空，Runner job 队列不得出现对应记录，Mock 应用后也不得真实修改 Agent 配置。
- Agent 配置 dry-run 必须保持 feature-disabled / blocked，返回 write plan / rollback plan，但所有 sideEffects 为 false，且不得改变待应用状态或 Agent 配置。
- `verify-agent-config-fields.ps1` 会独立覆盖未来 Agent config 真实写入字段白名单：允许 `permissions`、`model`、`status`、`maxSubAgents`、`canSpawnSubAgents`，拒绝 API key、raw secret、provider header/response、prompt、本地私有路径、Runner/tool/command/file/Git/network 字段、父子/汇报关系越权字段、`all=true` 和 forbidden capabilities。
- `verify-agent-config-dry-run.ps1` 会独立覆盖 Agent config dry-run helper 的反向用例：缺少二次确认、缺少确认文本、非 `pending_apply`、未批准来源审批、来源审批带 Runner job、错误 target service、缺少目标 Agent 和全 false sideEffects。
- `verify-agent-config-apply-gate.ps1` 会独立覆盖未来真实 apply 前置闸门：即使 dry-run、二次确认、requestedBy、Git checkpoint 和 rollback acceptance 都满足，也只能返回 `preconditionsReady=true`，但 `gateReady=false`、`canApply=false`、`feature_disabled` 和全 false sideEffects 必须保持不变。
- `verify-agent-config-transaction-plan.ps1` 会独立覆盖未来真实写入事务计划：计划写入集必须是同一事务内更新 `agents`、插入 `agent_config_versions`、标记 application applied、插入 `runtime_events`，但当前仍只能 `canWrite=false`、`feature_disabled` 和全 false sideEffects。
- `verify-agent-config-rollback-request.ps1` 会独立覆盖直接 helper 级别的 Agent 配置回滚请求契约；Mock 和 SQLite flow 脚本也会覆盖禁用态 `POST /api/agent-config-applications/:applicationId/rollback-request` 路由。在真实版本历史存在前，该路由必须保持 `requestReady=false`。
- `verify-agent-config-version-history.ps1` 会独立覆盖只读 Agent 配置版本历史来源 helper：只规范化已经加载好的版本行，验证目标 Agent 过滤、版本排序、current/restore 选择、snapshot 字段白名单、禁止字段/值和全 false sideEffects；它不得直接读 SQLite、暴露路由、写版本、创建回滚审批、执行 Runner、调用模型或读取密钥。
- Agent 配置真实写入前必须先通过 `docs/agent-config-apply-dry-run-spec.md` 定义的 dry-run / rollback 验收；当前本地 Demo 不得写入 `agents` 或 `agent_config_versions`。
- Agent permission change request 必须在创建审批前先运行 mock profile 验证。安全 profile 可以创建 pending `agent_config` 审批；`canExecuteRunnerJob` 等禁止 capability 必须返回 422，并且不得创建审批、写 runtime 或写 SQLite。
- Model Gateway status 和 dry-run 必须保持禁用态，不调用真实 provider，不写状态，不触发 Agent 或 Runner。
- `verify-model-gateway.ps1` 会独立覆盖 Model Gateway status、dry-run、connectivity-test disabled stub、preflight failure paths、disabled adapter registry、openai_compat relay interface、cheng.pink request builder、feature flag 边界和全 false sideEffects。
- `verify-agent-permissions.ps1` 会独立覆盖 Agent permission profile 展开、`all=true` 拒绝、未知能力拒绝、禁止 Agent 能力拒绝和全 false sideEffects。
- 设置页和集成页必须展示 Model Gateway dry-run 只读预览，且浏览器控制台保持 0 errors / 0 warnings。

脚本结束时会重置本地 runtime state 或 SQLite seed 状态，避免留下测试状态。

注意：会自启动 API 的验收脚本必须使用隔离端口，并且启动前如果发现端口已有 API 响应，会直接失败，不会连接已有进程。`8787` 不删除，但只作为人工本地试用和当前运行服务检查入口。

Model Gateway manual connectivity test currently has only a disabled backend stub:

- No local demo step may call a real OpenAI, Anthropic, Google Gemini, or other provider API yet.
- No demo script may require real API keys or provider SDKs.
- `POST /api/model-gateway/connectivity-test` must return `result=blocked`, `errorCategory=feature_disabled`, `realProviderRequestAttempted=false`, and all side effects false.
- The connectivity-test response now includes a backend `preflight` object. It may report blocked categories such as `missing_key` or `feature_disabled`, but it must still report `realProviderRequestAttempted=false` and all side effects false.
- `scripts/verify-model-gateway.ps1` is the dedicated non-browser acceptance entry for Model Gateway; it calls the exported preflight helper directly to cover feature disabled, missing key, unsupported provider, unsupported model, invalid purpose, timeout, and provider-error paths without real keys or provider SDKs.
- `scripts/verify-local-ui.ps1` is the browser UI smoke entry; it only checks rendered Model Gateway UI status/copy and does not run backend helper or connectivity-test deep assertions.
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` is only a reported request flag in MVP-0.2; it must not make `manualConnectivityTestActive` or `realProviderRequestsAllowed` true.
- Provider adapter verification currently covers only the disabled adapter registry and stub; demo verification must not import provider SDKs, make provider requests, require real keys, store responses, or report raw provider errors.
- Future real manual connectivity checks must be user-triggered, backend-only, fixed-prompt/minimal-ping, and disabled by default.
- An OpenAI-compatible relay is the first planned real-provider candidate, but this checklist still forbids real relay or official OpenAI requests until a later explicit implementation commit changes the feature-flag boundary and passes blocked/missing-key/missing-base-url/invalid-base-url/no-side-effect verification.
- `openai_compat` may appear in Model Gateway status as disabled metadata only. The UI and scripts may show `AGENT_SWARM_OPENAI_COMPAT_API_KEY` and `AGENT_SWARM_OPENAI_COMPAT_BASE_URL` env var names, but must never show their values.
- Relay preflight may validate missing or unsafe base URL shape, but it must not make network requests and must keep all side effects false.

## 7. 当前安全边界

当前 Demo 允许：

- 读取 Mock API 数据。
- 把审批、任务、Agent 配置应用记录的状态写入本地 runtime state 或 SQLite。
- 展示 Runner job、Runner 状态和 Agent 配置审查信息。

当前 Demo 不允许：

- 不会真实修改 Agent 配置。
- 不会让 Runner 写文件、删文件、执行命令、发起网络请求或修改 Git。
- 不会调用真实模型 API。
- 不会连接真实数据库或云同步。
- 不会因为 Agent 标记为 `architect_admin` 或“全权限”而绕过 Approval Service、Runner 安全检查、Model Gateway 禁用态或密钥边界。

如果后续要开放真实 Runner 或真实 Agent 配置写入，必须先补 Approval Service、二次确认、Git checkpoint 和回滚策略。

## Module Stability Map

- 模块稳定性、可删除/不可删除、可重构和受保护目录清单见 `docs/module-stability-map.md`。
- 后续删除或移动 tracked 文件前，应先用 `rg` 查引用，更新相关 docs/scripts，并跑对应验收脚本。
- `design/image2/`、`_internal/`、`data/local/`、`logs/`、`.playwright-cli/` 和 `data/mock/runtime-state.json` 仍属于受保护或运行态范围，不得误提交。
- P0 anchor 文件不能作为普通清理目标；如果确实要改，必须同步合同文档和验收脚本。

## Agent Permission Contract

- Agent 权限分层契约见 `docs/agent-permission-contract.md`。
- “全权限”只能表示广义规划、编排和申请权限；不得表示自批、自执行、写文件、跑命令、改 Git、发网络请求或访问原始密钥。
- `architect_admin` 可以作为未来最高管理型 Agent profile，但仍必须通过 Approval Service、Runner job、Model Gateway feature flag contract 和密钥服务边界。
- Demo 验证不应把权限 profile 当成真实执行授权；当前仍停留在 Mock / disabled / read-only 阶段。
- `services/api/agent-permissions.js` and `scripts/verify-agent-permissions.ps1` are mock/profile validation only. They do not change runtime authorization, Agent config, Runner execution, Model Gateway behavior, SQLite state, or secret access.

## Model Gateway Relay Interface Checkpoint

- The relay adapter interface checkpoint may expose future metadata such as `openai_compat_manual_connectivity_adapter` and `interface_disabled`, but it must still be tested only through backend helpers with simulated preflight failures.
- Relay interface verification must cover missing key, missing base URL, invalid base URL, unsupported provider, unsupported model, timeout, provider error, feature disabled, and all-false side effects without real relay credentials or network calls.
- `docs/relay-provider-info-checklist.md` is the safe place for non-secret relay documentation facts. It must not contain keys, auth headers, account ids, prompts, model outputs, provider bodies, token usage, cost, or raw errors.
- The cheng.pink relay facts are operator-provided and non-secret; demo scripts must still not call the relay, require a real key, store provider bodies, log prompt/result, or report token usage/cost.
- `docs/cheng-relay-manual-ping-spec.md` freezes the future cheng.pink non-stream ping shape and URL normalization rules. Demo scripts must still verify these only through disabled or simulated paths until a later implementation commit explicitly changes the feature flag boundary.
- `verify-model-gateway.ps1` verifies the cheng.pink request builder and URL normalization as pure local helper calls. These checks must keep `realProviderRequestAttempted=false`, must not read keys, and must not make network requests.
- `verify-local-ui.ps1` keeps only UI-level Model Gateway smoke checks and must not become a real connectivity test.

## Model Gateway DeepSeek Provider Checklist

- `docs/deepseek-provider-info-checklist.md` records only non-secret facts from official DeepSeek documentation.
- DeepSeek may be considered a first real-provider candidate later, but no local demo step may call DeepSeek yet.
- Demo verification must still pass without real DeepSeek credentials, SDK imports, network calls, stored provider responses, prompt/result logs, token usage, cost, or raw provider errors.
