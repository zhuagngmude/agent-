# Agent 配置 Apply Dry-Run 与回滚规格

日期：2026-06-10

状态：已实现禁用态 dry-run endpoint。本文档不启用真实 Agent 配置写入、Runner 执行、模型调用、云同步或完整运行时权限系统。

## 目的

在 Agent 配置变更可以写入真实 `agents` 当前态之前，项目必须先增加 dry-run 闸门，用来证明已批准的变更是有效的、可回滚的、可审计的，并且仍在当前安全边界内。

当前 `POST /api/agent-config-applications/:applicationId/apply` endpoint 仍然只是 Mock 状态流转：它会把 application 标记为 `applied`，记录确认元数据，但不得修改 Agent 配置。

## 当前边界

当前允许：

- 通过 Agent change-request flow 创建 `agent_config` 审批。
- 在创建审批前验证 permission profile 变更。
- 批准 `agent_config` 审批，但不创建 Runner job。
- 创建 `pending_apply` 状态的 `agent_config_applications` 记录。
- 对这些记录做 Mock apply 或 cancel，用于本地状态流转验证。

当前禁止：

- 修改 `agents` 当前配置。
- 写入 `agent_config_versions`。
- 为 `agent_config` 审批创建 Runner job。
- 让 Agent 自批或自行应用配置。
- 调用真实模型 provider。
- 读取 raw secret 或 API key。
- 不经过新审批就应用回滚。

## 禁用态 Dry-Run Endpoint

当前 endpoint：

```text
POST /api/agent-config-applications/:applicationId/dry-run
```

用途：验证一个 pending Agent 配置 application，并在不改变状态的前提下预览准确写入计划。MVP-0.2 始终保持 blocked，`blockedReasons=["feature_disabled"]`。

请求格式：

```json
{
  "secondConfirm": true,
  "confirmText": "I understand this is a dry-run only.",
  "requestedBy": "local_user"
}
```

响应格式：

```json
{
  "ok": false,
  "dryRun": true,
  "applicationId": "agent_config_application_approval_agent_agent_reviewer_permission",
  "approvalId": "approval_agent_agent_reviewer_permission",
  "agentId": "agent_reviewer",
  "canApply": false,
  "blockedReasons": ["feature_disabled"],
  "writePlan": {
    "wouldUpdateAgent": false,
    "wouldCreateVersion": false,
    "wouldWriteRuntimeEvent": false,
    "targetVersion": 2,
    "changedFields": ["permissions"]
  },
  "rollbackPlan": {
    "rollbackRequiresNewApproval": true,
    "wouldRestoreVersion": 1,
    "rollbackAction": "create_new_agent_config_application"
  },
  "sideEffects": {
    "writesAgents": false,
    "writesAgentConfigVersions": false,
    "writesRuntimeEvents": false,
    "writesSqlite": false,
    "writesRuntimeState": false,
    "createsApprovals": false,
    "createsRunnerJobs": false,
    "executesRunner": false,
    "callsRealModel": false,
    "readsRawSecrets": false
  }
}
```

MVP-0.2 返回 blocked / feature-disabled 行为。dry-run 会从已经批准的本地状态计算预览，但在后续实现提交明确改变 feature gate 前，所有 sideEffects 都必须保持 false。

## Dry-Run 验证规则

只要下面任一条件不成立，dry-run 就必须拒绝或阻断：

- Application 存在。
- Application 状态是 `pending_apply`。
- 来源审批存在。
- 来源审批状态是 `approved`。
- 来源审批 `targetService` 是 `agent_config`。
- 来源审批的 `runnerJobId` 为空。
- 请求包含 `secondConfirm=true`。
- 请求包含非空 `confirmText`。
- 目标 Agent 存在。
- 变更字段属于当前阶段支持范围。
- Permission 变更通过 `services/api/agent-permissions.js` 验证。
- Change plan 不包含未知字段。
- Change plan 不包含 raw secret、API key、provider header、prompt、provider response、本地私有路径，或未经检查的 tool/Runner 字段。
- 操作不需要 Runner 执行。

## Change Plan 字段白名单

当前可执行 helper：

```text
validateAgentConfigChangePlan(...)
```

当前状态：helper-only、no-write。它在 dry-run/apply-gate 继续之前验证未来写入计划，但不会修改 Agent 配置。

当前阶段允许字段：

- `permissions`
- `model`
- `status`
- `maxSubAgents`
- `canSpawnSubAgents`

禁止的 change-plan 内容：

- API key、raw secret、token、authorization header、provider header、provider response、prompt、本地私有路径。
- Runner、tool、command、file、Git、network、workspace、parent Agent、reporting relationship 字段。
- 客户端传入的任意完整 Agent config JSON。
- `all=true` 权限授予。
- 禁止 Agent capability，例如直接执行 Runner、本地文件写入/删除、命令执行、Git 修改、网络请求、高风险/自审批、raw secret 访问。

dry-run 响应包含 `changePlanValidation`。未来真实 apply 必须先要求这个结果 `ok=true`，才可以考虑任何写入。

## 第一次真实 Apply 闸门

第一次真实 Agent 配置 apply 必须在 dry-run 稳定后，通过单独提交实现。

当前可执行 gate helper：

```text
buildAgentConfigRealApplyGate(...)
```

当前状态：helper-only、feature-disabled。它可以用 `preconditionsReady=true` 证明未来前置条件齐全，但仍必须返回 `ok=false`、`gateReady=false`、`canApply=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。

未来考虑任何真实写入前，必须具备：

- 同一 application、approval、目标 Agent 的匹配 dry-run 结果。
- dry-run 结果仍是当前 feature-disabled 预览：`dryRun=true`、`ok=false`、`canApply=false`，且 `blockedReasons` 包含 `feature_disabled`。
- dry-run 结果没有 validation errors。
- dry-run 结果有 `changePlanValidation.ok=true`。
- dry-run 结果所有 sideEffects 都是 false。
- Application 存在并仍是 `pending_apply`。
- 来源审批存在，状态是 `approved`，目标是 `agent_config`，且没有 Runner job。
- 目标 Agent 存在。
- 请求包含 `secondConfirm=true`、非空 `confirmText`、非空 `requestedBy`。
- 请求包含 `gitCheckpoint.created=true` 和 checkpoint commit id。
- 请求包含 `rollbackPlanAccepted=true`。

## 事务计划

当前可执行 helper：

```text
buildAgentConfigApplyTransactionPlan(...)
```

当前状态：helper-only、feature-disabled。有效计划可以返回 `planReady=true`，但仍必须返回 `ok=false`、`canWrite=false`、`blockedReasons=["feature_disabled"]` 和全 false sideEffects。

未来真实写入集必须是一个事务：

1. 更新 `agents` 当前态。
2. 插入一条 `agent_config_versions` 记录。
3. 将 `agent_config_applications` 记录标记为 applied。
4. 插入一条 `runtime_events` 审计记录。

事务保护规则：

- 写入时 application 必须仍是 `pending_apply`。
- 来源审批必须仍是 `approved`，目标为 `agent_config`，且没有 Runner job。
- 目标 Agent 行必须存在。
- 目标版本必须等于当前 Agent 配置版本 + 1。
- `agent_id + version` 不得已存在于 `agent_config_versions`。
- 所有写入必须一起 commit 或一起 rollback。
- runtime event 插入必须属于同一个事务。
- transaction plan 不得创建 Runner job、执行 Runner、调用模型、读取 raw secret、修改文件或修改 Git。

必须做到：

- 增加独立 feature flag，不复用 Model Gateway 或 Runner flag。
- 保持 `targetService=agent_config` 与 Runner job 创建隔离。
- 每次真实 apply 前都重新运行同样的 dry-run 验证。
- 在一个事务内更新 `agents` 当前配置并插入 `agent_config_versions`。
- 写入 runtime event 或等价审计记录。
- 存储 before/after 字段摘要，不存 secret。
- 拒绝半写入。
- 保留旧配置版本，用于回滚规划。
- UI 文案必须清楚说明这是 Agent config apply，不是 Runner execution。

不得做：

- 执行 Runner。
- 修改文件或 Git。
- 调用模型 provider。
- 读取 raw API key。
- 允许 Agent 自批或自行 apply。
- 接收客户端传入的任意 config JSON。
- 允许宽泛 `all=true` 权限授予。

## 回滚规则

回滚是一次新的已批准配置变更，不是数据库删除，也不是直接 revert。

当前可执行 helper：

```text
buildAgentConfigRollbackRequest(...)
buildAgentConfigVersionHistory(...)
```

当前状态：禁用态 HTTP 预览路由加 helper。直接 helper 调用在版本输入有效时可以返回 `requestReady=true`，但仍必须返回 `ok=false`、`canCreateApproval=false`、`blockedReasons=["feature_disabled"]`、只作为草稿的 approval/application 对象，以及全 false sideEffects。

当前路由：

```text
POST /api/agent-config-applications/:applicationId/rollback-request
```

该路由只读取 application、来源审批和目标 Agent。由于 MVP-0.2 还没有在 app flow 中写入或读取真实 `agent_config_versions`，普通 Mock/SQLite 路由调用会返回 `requestReady=false`，带 current/restore version 验证错误，同时仍保持所有 sideEffects 为 false。

helper 覆盖脚本：

```text
scripts/verify-agent-config-rollback-request.ps1
scripts/verify-agent-config-version-history.ps1
```

`buildAgentConfigVersionHistory(...)` 是回滚来源选择的只读 source helper。`GET /api/agents/:agentId/config-version-history` 是对应只读 HTTP route：Mock 模式返回空历史，SQLite 模式从 snapshot 只读 `agent_config_versions` 后交给 helper 规范化。它支持 camelCase 和 SQLite 风格 snake_case 字段，解析 JSON snapshot/change，按目标 Agent 过滤，按版本倒序排序，选择 current version 和 restore candidates，并保持全 false sideEffects。它不创建回滚请求，不写 SQLite，不写版本。

回滚必须：

- 引用原 application ID。
- 引用要从哪个版本回滚。
- 引用要恢复到哪个版本。
- 生成新的 `agent_config` 审批。
- 要求人类二次确认。
- 要求原 application 是 `applied`。
- 要求来源审批是 `approved`，目标为 `agent_config`，且没有 Runner job。
- 要求 current 和 restore 版本都属于目标 Agent。
- 要求 restore 版本早于 current 版本。
- 要求至少有一个回滚字段发生变化。
- 通过 dry-run 验证。
- 如果被 apply，则插入新的 `agent_config_versions` 行。
- 保留旧版本历史不变。

回滚不得：

- 删除 `agent_config_versions`。
- 覆盖 `agent_config_versions`。
- 不经审批直接编辑 `agents`。
- 在 feature disabled 的 request-helper 阶段创建审批/application。
- 运行 Git 或 Runner 命令。
- 触碰 `_internal/`、`design/image2/`、`data/local/`、logs、runtime-state 或 secrets。

## 验收清单

启用任何真实 apply endpoint 前，必须满足：

- Dry-run endpoint 已存在，并由 Mock 和 SQLite 验证覆盖。
- Helper 级回归覆盖普通 HTTP flow 无法构造的前置条件，包括未批准来源审批、来源审批带 Runner job。
- 真实 apply gate helper 已存在，并由 `scripts/verify-agent-config-apply-gate.ps1` 覆盖。
- 真实 apply gate 可以对有效输入报告 `preconditionsReady=true`，同时仍保持 `gateReady=false`、`canApply=false` 和 `feature_disabled`。
- 真实 apply gate 会拒绝缺少 requestedBy、缺少 Git checkpoint、缺少 rollback acceptance、缺少或不匹配 dry-run proof、dry-run validation errors、dry-run sideEffects、来源审批带 Runner job。
- 字段白名单 helper 已存在，并由 `scripts/verify-agent-config-fields.ps1` 覆盖。
- Dry-run 和真实 apply gate 都会拒绝未支持字段、禁止字段、禁止值、`all=true` 和禁止 Agent capability。
- 事务计划 helper 已存在，并由 `scripts/verify-agent-config-transaction-plan.ps1` 覆盖。
- 事务计划保持 `canWrite=false` / `feature_disabled`，同时证明未来写入集和 rollback-on-failure 保护规则。
- 回滚请求 helper 已存在，并由 `scripts/verify-agent-config-rollback-request.ps1` 覆盖。
- 版本历史来源 helper 已存在，并由 `scripts/verify-agent-config-version-history.ps1` 覆盖。
- 版本历史只读路由已存在，并由 Mock 和 SQLite flow 覆盖。
- 禁用态回滚请求路由已存在，并由 Mock 和 SQLite 验证覆盖。
- 回滚请求可以对有效输入报告 `requestReady=true`，同时仍保持 `ok=false`、`canCreateApproval=false` 和 `feature_disabled`。
- Mock/SQLite 回滚请求路由在真实版本历史存在前保持 `requestReady=false`。
- 回滚请求会拒绝非 applied 原 application、未批准来源审批、来源审批带 Runner job、错误 target service、错误 Agent 版本归属、restore version 不早于 current version、缺少 confirmation/requester/reason、没有变更字段。
- 回滚请求只起草新的审批、新 application 和未来新版本，不实际创建。
- 版本历史来源 helper 会拒绝 wrong-Agent 版本、重复或缺失版本号、禁止 snapshot 字段、禁止 snapshot 值、非法 restore target，并且没有 sideEffects。
- Dry-run blocked 状态保持所有 sideEffects 为 false。
- 无效 application ID 返回安全错误。
- 非 `pending_apply` application 被拒绝。
- 缺少来源审批会被拒绝。
- 非 `approved` 来源审批会被拒绝。
- 来源审批带 `runnerJobId` 会被拒绝。
- 未支持变更字段会被拒绝。
- 无效 permission capability 会被拒绝。
- Dry-run 证明目标 Agent 配置保持不变。
- 第一次真实 apply 具备 `agents` 和 `agent_config_versions` 的事务覆盖。
- 回滚创建新审批，而不是修改历史。

在每一项通过之前，项目都保持 Mock / dry-run only 模式。
