# agent蜂群 API 草案

日期：2026-06-08

阶段：MVP-0.2 前端工程化之后，后端实现之前。

## 目标

这份文档定义前端控制台、Agent 调度、Runner 审批、知识库、Git 保存点和费用统计需要的第一批 API。

当前先写接口契约，不急着实现真实后端。

核心原则：

- Dashboard 使用聚合接口，避免前端一次请求十几个接口。
- Approval Service 和 Runner Service 必须分开。
- Runner 不能自己决定是否可以执行。
- 所有本地写文件、删文件、执行命令、网络请求、Git 操作都必须先创建 ApprovalRequest。
- API Key 不允许明文返回，不允许出现在日志里。
- Agent “全权限”必须按 `docs/agent-permission-contract.md` 拆成 planning / orchestration / request / approval / execution / secret access，不得用一个布尔值表示。

## 状态码约定

### ApprovalStatus

```text
draft
pending
approved
rejected
patch_only
executed
rolled_back
expired
```

### TaskStatus

```text
queued
running
blocked
waiting_user
completed
failed
cancelled
```

### AgentStatus

```text
running
idle
waiting
failed
disabled
```

## Agent Permission Contract

Agent 权限契约见：

```text
docs/agent-permission-contract.md
```

API 层必须区分以下能力：

```text
planning
orchestration
request
approval
execution
secret_access
```

MVP-0.2 code status:

- `services/api/agent-permissions.js` can expand and validate mock Agent permission profiles.
- `scripts/verify-agent-permissions.ps1` checks supported profiles, forbidden capability rejection, unknown capability rejection, `all=true` rejection, and all-false side effects.
- This is not runtime enforcement. No API route may treat the helper as permission authorization until a later explicit implementation adds route-level checks, Approval Service integration, persistence rules, and regression coverage.

后续即使支持 `architect_admin` 或 `all_agents_full_management`，也只能表示广义的规划、编排和申请权限。它不得自动包含：

```text
canApproveHighRisk
canApproveOwnRequest
canExecuteRunnerJob
canWriteFiles
canDeleteFiles
canExecuteCommands
canModifyGit
canMakeNetworkRequests
canAccessRawSecrets
```

不可绕过规则：

- Agent 可以发起 Approval Request，但不能批准自己的请求。
- Runner 只能执行已批准的 Runner job。
- 真实文件写入、命令执行、网络请求和 Git 操作仍必须走 Approval Service、二次确认和 Git checkpoint。
- Model Gateway 真实 provider 请求仍必须后端、手动、固定最小 ping，并等待单独 feature flag contract 改变。
- API 只能返回密钥是否配置等非敏感布尔值，不得返回 raw key、key suffix、masked fragment、authorization header、provider body、prompt 或 result。

## Dashboard

### GET /api/projects/:projectId/dashboard

用途：项目总览页聚合数据。

返回：

```json
{
  "project": {
    "id": "project_agent_swarm",
    "name": "agent蜂群 MVP",
    "status": "running",
    "phase": "MVP-0.2",
    "description": "多 AI 智能体协作调度系统"
  },
  "metrics": {
    "activeAgents": 18,
    "pendingApprovals": 7,
    "activeTasks": 32,
    "gitCheckpoints": 15,
    "tokenUsage": "1.23M",
    "modelCount": 6
  },
  "workflowSummary": {
    "totalAgents": 24,
    "totalTasks": 68,
    "completedTasks": 36,
    "successRate": 0.923,
    "averageResponseMs": 1200
  },
  "runnerStatus": {
    "connected": true,
    "runnerId": "local_runner_001",
    "version": "0.1.0",
    "workspacePath": "F:/projects/agent-swarm",
    "permissions": {
      "readFiles": true,
      "writeFiles": "approval_required",
      "executeCommands": "approval_required",
      "networkRequests": "approval_required"
    },
    "lastHeartbeatAt": "2026-06-08T12:00:00Z"
  },
  "pendingApprovals": [],
  "taskQueue": [],
  "agentStatus": [],
  "gitCheckpoints": [],
  "knowledgeUpdates": [],
  "usageSummary": {},
  "integrationHealth": []
}
```

## Agents

### GET /api/projects/:projectId/agents

用途：智能体管理页。

返回：

```json
{
  "agents": [
    {
      "id": "agent_architect",
      "name": "架构师 Agent",
      "role": "architect",
      "status": "running",
      "version": "v0.2.0",
      "model": "gpt-high-reasoning",
      "canSpawnSubAgents": true,
      "maxSubAgents": 3,
      "parentAgentId": "",
      "childAgentIds": ["agent_frontend", "agent_docs", "agent_reviewer"],
      "reportsToAgentId": "",
      "spawnDepth": 0,
      "permissions": ["read_project", "plan_tasks", "review_architecture"]
    }
  ]
}
```

### PATCH /api/agents/:agentId

用途：更新 Agent 配置。

当前 MVP-0.2 只展示配置规则草案，不开放真实修改。后续开放时必须遵守：

```text
可编辑字段：Agent 名称、使用模型、启用/禁用状态、权限列表、是否允许创建子 Agent、最大子 Agent 数
必须审批字段：权限列表、是否允许创建子 Agent、最大子 Agent 数、代码执行请求权限、API Key / 模型 Key 访问权限
暂时只读字段：Agent ID、角色类型、父 Agent、派生深度、汇总目标、创建来源
禁止子 Agent 修改：自己的权限、父 Agent、汇总目标、API Key、Runner 执行权限、其他 Agent 的配置
```

`permissions` 后续不得只保存 `all=true` 这类粗粒度值；必须保存明确能力或 profile，并按 `docs/agent-permission-contract.md` 展开。`architect_admin` 可以拥有最高规划、编排和申请权限，但仍不能自批、自执行或访问原始密钥。

当前前端仅实现变更请求预览，不会调用 PATCH 接口，也不会写入 Mock 状态。预览对象至少包含：

```json
{
  "agentId": "agent_frontend",
  "changeType": "permission",
  "riskLevel": "high",
  "requiresApproval": true,
  "changes": [
    { "field": "permissions", "before": ["read_project"], "after": ["read_project", "request_code_execution"] }
  ]
}
```

### POST /api/agents/:agentId/change-requests

用途：把 Agent 配置变更预览转换为审批申请。当前只创建 Approval Request，不修改 Agent 配置。

Permission changes are mock-validated before an approval is created:

- `changeType=permission` may include `permissionProfile`, `profile`, or explicit `capabilities`.
- Safe profiles such as `reviewer_agent` and `executor_agent` create a pending `agent_config` approval and return `permissionValidation`.
- The approval `changeRequest` stores the same `permissionValidation` for audit.
- Unsupported profiles, `all=true`, unknown capabilities, direct Runner/file/command/Git/network capabilities, high-risk approval, self-approval, and raw-secret access return `422 agent_permission_validation_failed`.
- A failed permission validation must not write SQLite, write runtime state, create an approval, create a Runner job, trigger Agents, execute Runner, call models, or read secrets.

请求：

```json
{
  "changeType": "permission",
  "riskLevel": "high",
  "permissionProfile": "reviewer_agent",
  "reason": "新增代码执行请求权限会影响 Runner 安全边界，必须二次确认。",
  "changes": [
    { "field": "permissions", "before": "read_project / review_risk / review_diff", "after": "reviewer_agent" }
  ]
}
```

返回：

```json
{
  "approval": {
    "id": "approval_agent_agent_frontend_permission",
    "status": "pending",
    "targetService": "agent_config",
    "changeRequest": {
      "permissionProfile": "reviewer_agent",
      "permissionValidation": {
        "ok": true
      }
    }
  },
  "permissionValidation": {
    "ok": true,
    "profile": "reviewer_agent",
    "sideEffects": {
      "writesSqlite": false,
      "createsApprovals": false,
      "executesRunner": false,
      "callsRealModel": false,
      "readsRawSecrets": false
    }
  },
  "message": "Agent change request created. Agent config was not modified."
}
```

注意：`targetService = agent_config` 的审批通过后不生成 Runner job。

请求：

```json
{
  "model": "claude-ui",
  "status": "disabled",
  "permissions": ["read_project", "write_docs"]
}
```

### GET /api/projects/:projectId/agent-config-applications

用途：查看已审批通过但尚未应用到 Agent 配置的变更记录。当前只读展示，不提供应用按钮，也不会生成 Runner job。

前端审查视图会把每条记录和来源审批关联展示：
- 目标 Agent。
- 来源审批 ID 和审批状态。
- 变更字段 before / after。
- 应用前检查项：审批是否已批准、目标服务是否为 `agent_config`、是否没有 Runner job、当前是否仍为 `pending_apply`。
- 应用审计记录：`appliedAt`、`appliedBy`、`applyConfirmText`、是否未生成 Runner job、是否未写 Agent 配置。
- 取消审计记录：`cancelledAt`、`cancelledBy`、`cancelReason`。
- 回滚前审查：仅对 `applied` 记录展示是否具备回滚审查条件，当前不提供真实回滚接口。

注意：当前接口只返回待审查记录，不提供应用配置接口；真正写入 Agent 配置前还需要单独的人工应用确认流程。

返回：
```json
{
  "applications": [
    {
      "id": "agent_config_application_approval_agent_agent_frontend_permission",
      "approvalId": "approval_agent_agent_frontend_permission",
      "agentId": "agent_frontend",
      "agentName": "前端 Agent",
      "changeType": "permission",
      "status": "pending_apply",
      "changes": [
        { "field": "permissions", "before": "read_project", "after": "read_project / request_code_execution" }
      ],
      "appliedAt": "",
      "appliedBy": "",
      "applyConfirmText": "",
      "cancelledAt": "",
      "cancelledBy": "",
      "cancelReason": "",
      "createdAt": "2026-06-09T12:00:00Z",
      "updatedAt": "2026-06-09T12:00:00Z"
    }
  ]
}
```

字段说明：
- `appliedAt`：Mock 应用状态流转完成时间，未应用时为空字符串。
- `appliedBy`：触发 Mock 应用状态流转的本地用户标识。
- `applyConfirmText`：用户提交二次确认时的确认文本。
- `cancelledAt`：Mock 取消状态流转完成时间，未取消时为空字符串。
- `cancelledBy`：触发 Mock 取消状态流转的本地用户标识。
- `cancelReason`：取消待应用记录的原因。
- 这些字段只记录状态流转审计信息，不代表 Agent 配置已经真实写入。
- 回滚前审查只基于来源审批、应用审计和字段差异推导；真正回滚必须重新创建审批申请，不能绕过 Approval Service。

### POST /api/agent-config-applications/:applicationId/apply

用途：人工确认后模拟应用已审批通过的 Agent 配置变更。

当前状态：Mock 状态流转已实现。MVP-0.2 只会把 `agentConfigApplications.status` 从 `pending_apply` 改为 `applied`，记录确认信息，不会修改 Agent 配置，也不会生成 Runner job。

真实写入前置规格见 `docs/agent-config-apply-dry-run-spec.md`。后续必须先实现 dry-run，证明待应用记录、来源审批、权限变更、版本写入计划和回滚计划都满足验收，才能在单独提交里开放真实写入。当前 `apply` 仍不得写入 `agents` 或 `agent_config_versions`。

必须满足：
- 来源审批状态必须是 `approved`。
- 来源审批 `targetService` 必须是 `agent_config`。
- 来源审批不得关联 Runner job。
- 应用记录状态必须是 `pending_apply`。
- 请求体必须包含二次确认字段。

请求草案：
```json
{
  "secondConfirm": true,
  "confirmText": "我确认仅执行 Agent 配置 Mock 应用状态流转",
  "appliedBy": "local_user"
}
```

返回：
```json
{
  "application": {
    "id": "agent_config_application_approval_agent_agent_frontend_permission",
    "status": "applied",
    "appliedAt": "2026-06-09T12:30:00Z",
    "appliedBy": "local_user",
    "applyConfirmText": "我确认仅执行 Agent 配置 Mock 应用状态流转"
  },
  "message": "Mock application status changed to applied. Agent config was not modified."
}
```

### POST /api/agent-config-applications/:applicationId/cancel

用途：在真正应用前取消已审批但尚未应用的 Agent 配置变更。

当前状态：Mock 状态流转已实现。MVP-0.2 只会把 `agentConfigApplications.status` 从 `pending_apply` 改为 `cancelled`，记录取消原因，不会修改 Agent 配置，也不会生成 Runner job。

必须满足：
- 应用记录状态必须是 `pending_apply`。
- 请求体必须包含取消原因。

请求：
```json
{
  "reason": "用户在控制台取消待应用 Agent 配置变更",
  "cancelledBy": "local_user"
}
```

返回：
```json
{
  "application": {
    "id": "agent_config_application_approval_agent_agent_frontend_permission",
    "status": "cancelled",
    "cancelledAt": "2026-06-09T12:40:00Z",
    "cancelledBy": "local_user",
    "cancelReason": "用户在控制台取消待应用 Agent 配置变更"
  },
  "message": "Mock application status changed to cancelled. Agent config was not modified."
}
```

### POST /api/agent-config-applications/:applicationId/dry-run

用途：真实 Agent 配置写入前 dry-run 的禁用态接口。当前已实现为 blocked / feature-disabled，只返回 write plan 和 rollback plan 预览，不改变状态。规格见 `docs/agent-config-apply-dry-run-spec.md`。

MVP-0.2 约束：

- 当前接口必须保持 feature-disabled / blocked。
- dry-run 可以返回 write plan 和 rollback plan，但不得写 `agents`、`agent_config_versions`、SQLite/runtime state、审批、Runner job 或 runtime event。
- dry-run 不得执行 Runner、调用真实模型、读取 raw secret、接受前端传入的任意 Agent config JSON 或 `all=true` 权限。
- 所有真实写入必须等 dry-run 验收和回滚审批策略通过后，再由单独提交打开。

请求：

```json
{
  "secondConfirm": true,
  "confirmText": "我确认这只是 Agent 配置 dry-run",
  "requestedBy": "local_user"
}
```

返回：

```json
{
  "ok": false,
  "dryRun": true,
  "applicationId": "agent_config_application_approval_agent_agent_reviewer_permission",
  "approvalId": "approval_agent_agent_reviewer_permission",
  "agentId": "agent_reviewer",
  "canApply": false,
  "blockedReasons": ["feature_disabled"],
  "validationErrors": [],
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

### Agent config real apply gate helper

Current status: helper-only, no HTTP route, no real write.

`services/api/server.js` exports `buildAgentConfigRealApplyGate(...)` for future real-apply contract verification. This helper checks the dry-run proof, source approval, target Agent, second confirmation, requestedBy, Git checkpoint, rollback acceptance, and all-false side effects.

Even when all preconditions are present, MVP-0.2 must return `preconditionsReady=true` but keep `ok=false`, `gateReady=false`, `canApply=false`, `blockedReasons=["feature_disabled"]`, and all side effects false. A later real apply implementation must be a separate feature-flagged commit and must not reuse this helper as permission to write `agents` or `agent_config_versions` by itself.

### Agent config change-plan field whitelist helper

Current status: helper-only, no HTTP route, no real write.

`services/api/agent-config-fields.js` exports `validateAgentConfigChangePlan(...)`. The current allowed future-write fields are `permissions`, `model`, `status`, `maxSubAgents`, and `canSpawnSubAgents`. The helper rejects unsupported fields, API keys, raw secrets, tokens, authorization/provider headers, provider responses, prompts, local private paths, Runner/tool/command/file/Git/network/workspace fields, parent/reporting relationship fields, forbidden Agent capabilities, and `all=true`.

`POST /api/agent-config-applications/:applicationId/dry-run` and `buildAgentConfigRealApplyGate(...)` include the helper result as `changePlanValidation`. This only tightens the disabled safety boundary; it does not enable Agent config writes.

### Agent config real-write transaction plan helper

Current status: helper-only, no HTTP route, no SQLite write.

`services/api/agent-config-transaction-plan.js` exports `buildAgentConfigApplyTransactionPlan(...)`. The helper previews the future real-write transaction: update `agents`, insert `agent_config_versions`, mark `agent_config_applications` applied, and insert `runtime_events` in one transaction.

Even when the plan is valid, MVP-0.2 must return `ok=false`, `canWrite=false`, `blockedReasons=["feature_disabled"]`, and all side effects false. The helper must not call SQLite, write runtime state, create Runner jobs, execute Runner, call models, read raw secrets, modify files, or modify Git.

### Agent config rollback request helper

Current status: disabled HTTP preview route plus helper, no approval creation, no SQLite write.

`services/api/agent-config-rollback-request.js` exports `buildAgentConfigRollbackRequest(...)`. The helper validates a future rollback request against an applied original application, an approved `agent_config` source approval without Runner job, the target Agent, current and restore versions that belong to that Agent, restore-version ordering, second confirmation, requester, reason, and changed fields.

`POST /api/agent-config-applications/:applicationId/rollback-request` is a disabled preview route for the currently selected applied application. In MVP-0.2 the route reads application, source approval, and target Agent only. Because real `agent_config_versions` are not written yet, normal Mock/SQLite route calls must return `requestReady=false` with missing current/restore version validation errors while still reporting `feature_disabled`.

Even when a direct helper test uses valid version inputs, MVP-0.2 must return `ok=false`, `requestReady=true`, `canCreateApproval=false`, `blockedReasons=["feature_disabled"]`, draft-only approval/application objects, rollback rules, and all side effects false. The helper and route must not create approvals, create applications, write `agents`, write `agent_config_versions`, call SQLite writes, write runtime state, create Runner jobs, execute Runner, call models, read raw secrets, modify files, or modify Git.

## Tasks

### GET /api/projects/:projectId/tasks

用途：任务管理页。

查询参数：

```text
status
agentId
riskLevel
keyword
```

返回：

```json
{
  "tasks": [
    {
      "id": "task_runner_approval",
      "title": "打磨 Runner 审批确认页",
      "status": "running",
      "priority": "high",
      "assignedAgentId": "agent_frontend",
      "riskLevel": "high",
      "relatedFiles": ["frontend/index.html", "frontend/app.js"],
      "requiresApproval": true,
      "dependsOn": []
    }
  ]
}
```

### POST /api/projects/:projectId/tasks

用途：创建任务。

请求：

```json
{
  "title": "实现 ApprovalStatus 状态机",
  "description": "让审批页从状态 code 渲染中文状态和动作",
  "priority": "high",
  "assignedAgentId": "agent_frontend"
}
```

### PATCH /api/tasks/:taskId/status

用途：更新任务状态。

请求：

```json
{
  "status": "blocked",
  "reason": "等待 Runner 审批接口"
}
```

### POST /api/tasks/:taskId/start

用途：开始任务，将任务状态切换为 `running`。

返回：

```json
{
  "task": {
    "id": "task_task_state_api",
    "status": "running",
    "startedAt": "2026-06-08T13:00:00Z"
  }
}
```

### POST /api/tasks/:taskId/complete

用途：标记任务完成。当前 Mock API 只允许 `running` 任务完成。

### POST /api/tasks/:taskId/fail

用途：标记任务失败。

请求：

```json
{
  "reason": "测试未通过"
}
```

### POST /api/tasks/:taskId/cancel

用途：取消任务。

## Workflows

### GET /api/projects/:projectId/workflows

用途：工作流编排页。

返回：

```json
{
  "workflows": [
    {
      "id": "workflow_runner_safe_execute",
      "name": "Runner 安全执行流程",
      "nodes": [
        { "id": "node_plan", "type": "agent", "label": "生成执行计划" },
        { "id": "node_approval", "type": "approval", "label": "用户审批" },
        { "id": "node_runner", "type": "runner", "label": "本地 Runner 执行" }
      ],
      "edges": [
        { "from": "node_plan", "to": "node_approval" },
        { "from": "node_approval", "to": "node_runner" }
      ]
    }
  ]
}
```

### GET /api/workflow-runs/:runId

用途：执行流水线回放。

返回：

```json
{
  "id": "run_001",
  "workflowId": "workflow_runner_safe_execute",
  "status": "waiting_user",
  "steps": [
    {
      "id": "step_approval",
      "type": "approval",
      "status": "pending",
      "startedAt": "2026-06-08T12:00:00Z",
      "outputSummary": "等待用户确认 Runner 写入权限"
    }
  ]
}
```

## Approvals

### GET /api/projects/:projectId/approvals

用途：审批与确认页。

查询参数：

```text
status
riskLevel
agentId
operationType
```

返回：

```json
{
  "approvals": [
    {
      "id": "approval_runner_permissions",
      "status": "pending",
      "riskLevel": "high",
      "requestAgentId": "agent_backend",
      "operationTypes": ["file_write", "git_checkpoint", "audit_log_update"],
      "reason": "新增 Runner 写入审批状态机",
      "checkpoint": {
        "required": true,
        "created": true,
        "commit": "a5d3f2c"
      },
      "affectedFiles": [
        "runner/permissions.py",
        "server/audit_log.go",
        "docs/ai-maintenance.md"
      ],
      "diffPreview": [
        "- return runner.execute(command)",
        "+ approval = require_user_approval(command, changed_files)"
      ],
      "requiresSecondConfirm": true,
      "createdAt": "2026-06-08T12:00:00Z"
    }
  ]
}
```

### GET /api/approvals/:approvalId

用途：查看单个审批详情。

返回：同上单个 approval 对象。

### POST /api/approvals/:approvalId/reject

用途：拒绝执行。

请求：

```json
{
  "reason": "风险说明不完整，需要补充影响范围"
}
```

返回：

```json
{
  "id": "approval_runner_permissions",
  "status": "rejected"
}
```

### POST /api/approvals/:approvalId/patch-only

用途：只生成补丁，不写入工作区。

请求：

```json
{
  "reason": "先保留 AI 产物，稍后人工审查"
}
```

返回：

```json
{
  "id": "approval_runner_permissions",
  "status": "patch_only",
  "patchArtifactId": "patch_001"
}
```

### POST /api/approvals/:approvalId/approve

用途：批准执行。

高风险审批必须传入二次确认字段。

请求：

```json
{
  "secondConfirm": true,
  "confirmText": "我确认允许 Runner 修改列出的文件",
  "allowedOperations": ["file_write", "audit_log_update"]
}
```

返回：

```json
{
  "id": "approval_runner_permissions",
  "status": "approved",
  "runnerJobId": "runner_job_001"
}
```

## Runner

### GET /api/projects/:projectId/runner/status

用途：查看本地 Runner 连接状态。

返回：

```json
{
  "connected": true,
  "runnerId": "local_runner_001",
  "version": "0.1.0",
  "workspacePath": "F:/projects/agent-swarm",
  "permissions": {
    "readFiles": true,
    "writeFiles": "approval_required",
    "executeCommands": "approval_required",
    "networkRequests": "approval_required"
  },
  "lastHeartbeatAt": "2026-06-08T12:00:00Z"
}
```

### GET /api/projects/:projectId/runner/jobs

用途：查看已批准审批生成的 Runner job 队列。

当前 MVP-0.2 只读展示，不会真的执行本地命令。

返回：
```json
{
  "jobs": [
    {
      "id": "runner_job_approval_runner_permissions",
      "approvalId": "approval_runner_permissions",
      "status": "queued",
      "operationTypes": ["file_write", "git_checkpoint"],
      "affectedFiles": ["runner/permissions.py"],
      "checkpoint": "a5d3f2c",
      "createdAt": "2026-06-08T14:30:00Z"
    }
  ]
}
```

### POST /api/runner-jobs/:runnerJobId/start

用途：启动已批准的 Runner job。

限制：

- 只有 `ApprovalStatus = approved` 才允许启动。
- 如果 checkpoint 未创建，不允许启动。

返回：

```json
{
  "runnerJobId": "runner_job_001",
  "status": "running"
}
```

## Git

### GET /api/projects/:projectId/git/checkpoints

用途：Git 保存点列表。

返回：

```json
{
  "checkpoints": [
    {
      "commit": "620d44d",
      "message": "Start frontend MVP engineering cleanup",
      "type": "feature",
      "relatedTaskId": "task_frontend_cleanup",
      "createdAt": "2026-06-08T12:00:00Z"
    }
  ]
}
```

### POST /api/projects/:projectId/git/checkpoints

用途：创建保存点。

请求：

```json
{
  "message": "Before Runner modifies permissions",
  "type": "before_major_change",
  "relatedApprovalId": "approval_runner_permissions"
}
```

## Knowledge

### GET /api/projects/:projectId/knowledge/updates

用途：知识库更新列表。

返回：

```json
{
  "updates": [
    {
      "id": "knowledge_update_001",
      "document": "dev-docs/下一步开发路线.md",
      "section": "核心状态机",
      "status": "synced",
      "relatedFeature": "ApprovalStatus",
      "updatedAt": "2026-06-08T12:00:00Z"
    }
  ]
}
```

### POST /api/projects/:projectId/knowledge/sync

用途：同步人类说明书和 AI 开发维护手册。

请求：

```json
{
  "featureId": "runner_approval",
  "humanDocSection": "Runner 执行确认",
  "aiDocSection": "Approval 与 Runner 安全网关"
}
```

## Usage

### GET /api/projects/:projectId/usage

用途：费用与用量页。

返回：

```json
{
  "tokenUsage": {
    "total": 1230000,
    "today": 82000
  },
  "estimatedCost": {
    "currency": "CNY",
    "today": 128.4,
    "month": 245.6
  },
  "byModel": [
    { "provider": "openai", "model": "gpt", "tokens": 500000 },
    { "provider": "anthropic", "model": "claude", "tokens": 400000 }
  ]
}
```

## Settings

### GET /api/runtime-state

用途：设置页本地状态管理和本地试用状态面板。

当前实现状态：

- Mock 模式返回 `mode=mock`，状态保存到 `data/mock/runtime-state.json`。
- SQLite 本地试用模式返回 `mode=sqlite`，状态保存到 `data/local/agent-swarm.sqlite`。
- 返回值包含 `localTrial` 元信息，供前端展示 API 地址、Web 地址、状态文件位置、启动/停止/查看状态命令和安全边界。

返回示例：

```json
{
  "mode": "sqlite",
  "localTrial": {
    "mode": "sqlite",
    "persistence": "sqlite",
    "apiUrl": "http://127.0.0.1:8787",
    "webUrl": "http://127.0.0.1:5175/index.html",
    "projectRoot": "F:/projects/agent-swarm",
    "sqliteDbFile": "F:/projects/agent-swarm/data/local/agent-swarm.sqlite",
    "runtimeStateFile": "F:/projects/agent-swarm/data/mock/runtime-state.json",
    "commands": {
      "start": "powershell -ExecutionPolicy Bypass -File scripts\\start-local.ps1",
      "status": "powershell -ExecutionPolicy Bypass -File scripts\\status-local.ps1",
      "stop": "powershell -ExecutionPolicy Bypass -File scripts\\stop-local.ps1",
      "reset": "Invoke-RestMethod -Method Post http://127.0.0.1:8787/api/runtime-state/reset"
    },
    "safety": {
      "runnerExecutesCommands": false,
      "runnerWritesFiles": false,
      "realModelCalls": false,
      "cloudSync": false
    }
  }
}
```

注意：网页只展示停止命令，不提供停止本地进程按钮；停止必须由用户在终端执行 `scripts\stop-local.ps1`。

### GET /api/projects/:projectId/settings

用途：系统设置页。

返回时禁止返回完整 API Key。

```json
{
  "models": [
    { "role": "architect", "provider": "openai", "model": "gpt-high-reasoning" },
    { "role": "frontend", "provider": "anthropic", "model": "claude-ui" }
  ],
  "apiKeys": [
    { "provider": "openai", "configured": true, "display": "已加密保存" },
    { "provider": "anthropic", "configured": true, "display": "已加密保存" }
  ],
  "security": {
    "logRedaction": true,
    "syncSecretsToCloud": false,
    "runnerWriteRequiresApproval": true
  }
}
```

### PATCH /api/projects/:projectId/settings

用途：更新设置。

请求：

```json
{
  "security": {
    "runnerWriteRequiresApproval": true,
    "syncSecretsToCloud": false
  }
}
```

## 第一版实现顺序

1. 先用 `frontend/data.js` 对齐这些 response 结构。
2. 再实现本地 mock API。
3. 再接 SQLite 或 PostgreSQL。
4. 最后接真实模型调用和本地 Runner。

不要跳过 Approval Service 直接让 Runner 执行。

### GET /api/model-gateway/status

Purpose: read the current Model Gateway boundary for the local trial.

Current MVP-0.2 implementation is status-only:

- Real model calls are disabled.
- Provider SDKs are not loaded.
- Provider network requests are not made.
- API keys are never returned to the frontend.
- The endpoint only checks whether expected server-side environment variables exist.
- The endpoint does not write SQLite, runtime state, tasks, approvals, Runner jobs, logs, prompts, or responses.

Response example:

```json
{
  "enabled": false,
  "realModelCallsAllowed": false,
  "gatewayMode": "disabled",
  "serviceBoundary": "server_only",
  "featureFlags": {
    "manualConnectivityTestEnvVar": "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST",
    "manualConnectivityTestRequested": false,
    "manualConnectivityTestActive": false,
    "realProviderRequestsAllowed": false
  },
  "providers": [
    {
      "id": "openai",
      "label": "OpenAI",
      "keyEnvVar": "AGENT_SWARM_OPENAI_API_KEY",
      "configured": false,
      "providerAdapterId": "openai_disabled_connectivity_adapter",
      "providerAdapterMode": "disabled",
      "keyExposedToFrontend": false,
      "canRunConnectivityTest": false
    }
  ],
  "safety": {
    "storesApiKeys": false,
    "exposesApiKeysToFrontend": false,
    "writesDatabase": false,
    "createsTasks": false,
    "createsApprovals": false,
    "createsRunnerJobs": false,
    "runnerExecutesCommands": false,
    "logsPromptsOrResponses": false,
    "makesNetworkRequests": false
  },
  "blockedReasons": [
    "Real model calls are disabled in MVP-0.2.",
    "Approval, logging, cost tracking, and key-safety rules are not ready.",
    "This endpoint only reports provider configuration boundaries."
  ]
}
```

### POST /api/model-gateway/dry-run

Purpose: validate the shape and safety boundary of a future model connectivity test without calling a real model provider.

Current MVP-0.2 implementation is disabled and read-only. The dry-run phase is intentionally narrower than the long-term product. Its "must not" rules apply only to dry-run; they do not mean the final product will never write model logs, create tasks, trigger Agents, or call real models.

The settings and integrations pages may display this dry-run result as a read-only preview. The frontend must use a fixed connectivity-check request and must not collect, store, or display API keys or free-form prompts.

Request draft:

```json
{
  "provider": "openai",
  "model": "gpt-4.1-mini",
  "purpose": "connectivity_check",
  "promptPreview": "optional short user-visible test prompt label",
  "requestedBy": "local_user"
}
```

Response draft:

```json
{
  "ok": false,
  "dryRun": true,
  "provider": "openai",
  "requestValid": true,
  "validationErrors": [],
  "providerSupported": true,
  "keyEnvVar": "AGENT_SWARM_OPENAI_API_KEY",
  "keyConfigured": false,
  "featureFlags": {
    "manualConnectivityTestEnvVar": "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST",
    "manualConnectivityTestRequested": false,
    "manualConnectivityTestActive": false,
    "realProviderRequestsAllowed": false
  },
  "realModelCallsAllowed": false,
  "wouldCallProvider": false,
  "blockedReasons": [
    "Dry-run does not call real providers.",
    "Real model calls are disabled in MVP-0.2.",
    "Approval, logging, cost tracking, and key-safety rules are not ready."
  ],
  "sideEffects": {
    "writesSqlite": false,
    "writesRuntimeState": false,
    "createsTasks": false,
    "createsApprovals": false,
    "createsRunnerJobs": false,
    "triggersAgents": false,
    "callsRealModel": false,
    "logsPromptOrResult": false
  }
}
```

Dry-run acceptance rules:

- It must not write SQLite or `data/mock/runtime-state.json`.
- It must not create tasks, approvals, Runner jobs, workflow runs, runtime events, or model call records.
- It must not trigger any Agent.
- It must not call OpenAI, Anthropic, Google, or any real provider network API.
- It must not log prompts, prompt previews, model outputs, API keys, provider responses, or error bodies.
- It may only validate request shape, supported provider ids, expected server-side env var names, and current safety switches.
- It must return key presence as a boolean only; it must never return raw keys, key suffixes, or masked key fragments.
- It must stay backend-only; the frontend must not send, store, or display real API keys.

Later phases, intentionally not part of dry-run:

1. Manual connectivity test: user-triggered, real provider ping with minimal response and no Agent/Runner side effects.
2. Logged model call: redacted audit trail, token/cost/error tracking, and retention rules.
3. Agent orchestration: model output can create proposed tasks or approvals through controlled services.
4. Runner integration: only approved work may create Runner jobs after Runner safety acceptance passes.

### POST /api/model-gateway/connectivity-test

Purpose: planned manual real-provider connectivity test after dry-run is stable.

Current MVP-0.2 implementation is a disabled backend stub. It validates the request shape and routes through the disabled provider adapter stub, which returns `result=blocked`, `errorCategory=feature_disabled`, and all side effects false. It does not add provider SDKs, does not make OpenAI/Anthropic/Gemini requests, and is not exposed as an active frontend control.

Implementation boundary: `services/api/model-gateway.js` owns provider metadata, env var presence checks, dry-run validation, and the disabled connectivity-test stub. `services/api/server.js` should only wire HTTP routes to that module.

This phase is narrower than general model calling. It only proves that a server-side provider key can reach the provider with a minimal, fixed connectivity check. It is not an Agent run, not a chat/completion feature, not a Runner capability, and not a logged model-call pipeline.

Planned request draft:

```json
{
  "provider": "openai",
  "model": "gpt-4.1-mini",
  "purpose": "manual_connectivity_test",
  "secondConfirm": true,
  "confirmText": "I understand this will make one real provider connectivity request.",
  "requestedBy": "local_user"
}
```

Planned response draft:

```json
{
  "ok": false,
  "provider": "openai",
  "model": "gpt-4.1-mini",
  "purpose": "manual_connectivity_test",
  "requestValid": true,
  "providerSupported": true,
  "keyEnvVar": "AGENT_SWARM_OPENAI_API_KEY",
  "keyConfigured": false,
  "featureFlags": {
    "manualConnectivityTestEnvVar": "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST",
    "manualConnectivityTestRequested": false,
    "manualConnectivityTestActive": false,
    "realProviderRequestsAllowed": false
  },
  "preflight": {
    "ok": false,
    "result": "blocked",
    "errorCategory": "missing_key",
    "requestValid": true,
    "providerSupported": true,
    "modelSupported": true,
    "keyConfigured": false,
    "realProviderRequestAttempted": false,
    "blockingCategories": ["missing_key", "feature_disabled"],
    "checks": {
      "providerSupported": true,
      "modelPresent": true,
      "modelSupported": true,
      "purposeValid": true,
      "secondConfirmPresent": true,
      "confirmTextPresent": true,
      "featureEnabled": false,
      "realProviderRequestsAllowed": false,
      "keyConfigured": false,
      "timeoutWithinLimit": true,
      "responseBodyLimitWithinLimit": true
    }
  },
  "adapter": "disabled_provider_connectivity_adapter",
  "providerAdapterId": "openai_disabled_connectivity_adapter",
  "providerAdapterMode": "disabled",
  "realProviderRequestAttempted": false,
  "result": "blocked",
  "errorCategory": "feature_disabled",
  "providerResponseStored": false,
  "durationMs": 0,
  "redactionApplied": true,
  "sideEffects": {
    "writesSqlite": false,
    "writesRuntimeState": false,
    "createsTasks": false,
    "createsApprovals": false,
    "createsRunnerJobs": false,
    "triggersAgents": false,
    "executesRunner": false,
    "logsPromptOrResult": false,
    "storesProviderResponse": false
  }
}
```

Manual connectivity acceptance rules:

- It must be user-triggered and require an explicit confirmation field; Agents, page load, background jobs, and Runner jobs must not trigger it.
- It must run only on the backend; the frontend must never send, store, or display API keys.
- It must use server-side environment variables only and return key presence as booleans; it must never return raw keys, key suffixes, masked key fragments, or authorization headers.
- It must use a fixed provider-specific minimal ping. It must not accept free-form prompts, system prompts, user content, files, tool calls, function calls, or Agent context.
- It must not write SQLite or `data/mock/runtime-state.json`.
- It must not create tasks, approvals, Runner jobs, workflow runs, runtime events, model-call records, or billing records.
- It must not trigger any Agent and must not execute Runner code.
- It must not log prompts, model outputs, provider response bodies, request headers, API keys, or raw error bodies.
- It may return only coarse result fields such as `ok`, `provider`, `model`, `result`, `errorCategory`, and timestamp metadata.
- It must have a timeout and a small response/body limit before any real provider request is allowed.
- It must stay disabled by default until verification covers blocked, missing-key, unsupported-provider, timeout, and provider-error cases.
- MVP-0.2 may report `featureFlags.manualConnectivityTestRequested=true` when `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST=true` is present on the API process, but `manualConnectivityTestActive` and `realProviderRequestsAllowed` must still remain `false`.

Preflight gate implementation:

- `services/api/model-gateway.js` exports `modelGatewayConnectivityPreflight(...)` for backend and regression verification.
- `POST /api/model-gateway/connectivity-test` includes a `preflight` object, but the top-level response still routes through the disabled provider adapter and remains `blocked / feature_disabled`.
- The preflight gate validates provider support, provider-specific fixed connectivity model, `purpose=manual_connectivity_test`, second confirmation, feature flag status, server-side key presence, timeout, and response body limit.
- The preflight helper has acceptance-only options for deterministic regression checks such as missing key, timeout, and provider error. These options are not part of the HTTP API request body and must not become frontend controls.
- Preflight failure-path verification must keep `realProviderRequestAttempted=false` and all side effects false for feature disabled, missing key, unsupported provider, unsupported model, invalid purpose, timeout, and provider error.

Implementation order before enabling real provider requests:

1. Keep the disabled backend stub returning `blocked / feature_disabled` through the disabled provider adapter, with all side effects false.
2. Keep regression checks proving the stub cannot call providers.
3. Keep the manual feature flag boundary visible while forcing it inactive in MVP-0.2.
4. Only then consider adding isolated provider adapters, one provider at a time, with no SDK leakage into UI, Agent, or Runner code.

### Model Gateway provider adapter draft

Purpose: define the future backend-only adapter boundary before any real provider SDK or network request is added.

Current MVP-0.2 status: disabled stub only. `services/api/model-gateway-adapters.js` provides a disabled provider connectivity adapter; it must not import OpenAI, Anthropic, Google Gemini, or other provider SDKs. No code path should send real provider requests yet.
The current disabled registry may expose provider-specific disabled adapter ids for OpenAI, Anthropic, and Google Gemini, but those ids are metadata only and must still map to blocked, no-request behavior.

Adapter ownership:

- Provider adapters must live behind `services/api/model-gateway.js` or a later `services/api/model-gateway/` submodule.
- UI, Agent orchestration, Runner, SQLite mappers, and generic route handlers must not call provider SDKs directly.
- `services/api/server.js` must continue to only wire HTTP routes.

Adapter input draft:

```json
{
  "provider": "openai",
  "model": "gpt-4.1-mini",
  "purpose": "manual_connectivity_test",
  "requestId": "model_gateway_connectivity_001",
  "timeoutMs": 5000,
  "responseBodyLimitBytes": 4096,
  "serverKeyEnvVar": "AGENT_SWARM_OPENAI_API_KEY"
}
```

Adapter input rules:

- `purpose` must be `manual_connectivity_test`.
- `provider` must be one known provider id.
- `model` must be a non-empty provider-specific model id.
- `requestId` must be generated by the backend and must not contain prompts, API keys, or user secrets.
- `timeoutMs` and `responseBodyLimitBytes` must be enforced before any real provider request is allowed.
- The adapter must read only the configured server-side env var for the selected provider.
- The adapter must not accept free-form prompts, system prompts, user content, files, tool calls, function calls, Agent context, Runner job ids, or arbitrary HTTP options.

Adapter output draft:

```json
{
  "ok": false,
  "provider": "openai",
  "model": "gpt-4.1-mini",
  "adapter": "disabled_provider_connectivity_adapter",
  "providerAdapterId": "openai_disabled_connectivity_adapter",
  "providerAdapterMode": "disabled",
  "result": "blocked",
  "errorCategory": "feature_disabled",
  "realProviderRequestAttempted": false,
  "providerResponseStored": false,
  "durationMs": 0,
  "redactionApplied": true
}
```

Allowed `result` values:

```text
blocked
missing_key
unsupported_provider
unsupported_model
timeout
provider_error
network_error
success
```

Allowed `errorCategory` values:

```text
feature_disabled
missing_key
invalid_request
unsupported_provider
unsupported_model
timeout
provider_unavailable
network_error
unknown
```

Adapter output rules:

- It must never return raw API keys, key suffixes, masked key fragments, authorization headers, request headers, provider response bodies, model text, token usage, cost, or raw error bodies.
- It may return only coarse status fields, coarse error category, provider id, model id, duration, and booleans that prove whether a provider request was attempted or stored.
- `providerResponseStored` must remain `false` for manual connectivity checks.
- `realProviderRequestAttempted` must remain `false` while `manualConnectivityTestActive=false` or `realProviderRequestsAllowed=false`.

Adapter acceptance before any implementation:

- A disabled adapter stub must return `feature_disabled` with `realProviderRequestAttempted=false`.
- Missing-key, unsupported-provider, unsupported-model, timeout, and provider-error paths must be testable without writing SQLite/runtime state.
- Regression checks must prove no adapter path creates tasks, approvals, Runner jobs, Agent runs, runtime events, model-call records, billing records, or prompt/result logs.
- Each provider must be implemented and verified one at a time.
- Adding a provider SDK must be a separate commit from this draft and must not happen until the feature flag boundary is changed intentionally and reviewed.
- The disabled stub response may still include `provider`, `model`, `adapter`, `durationMs`, and redaction booleans, but it must never expose raw keys, provider responses, or prompt/result bodies.
- The disabled stub may also return `providerAdapterId` and `providerAdapterMode=disabled` so the registry can be verified provider by provider, but that metadata must never imply a real request path.

Real-provider phase gate:

- Do not start the first real provider adapter until the disabled registry verifies OpenAI, Anthropic, and Google Gemini metadata separately.
- The first real provider adapter must be a separate commit from the disabled registry work.
- The feature flag contract must change explicitly before `manualConnectivityTestActive` or `realProviderRequestsAllowed` can become `true`.
- The first real provider adapter must include tests for blocked, missing-key, unsupported-provider, timeout, provider-error, and no-side-effect paths before it can send a provider request.
- The first real provider adapter must keep the request fixed and minimal; it must not accept free-form prompt, Agent context, files, tool calls, Runner job ids, arbitrary headers, or arbitrary HTTP options.

First real-provider manual connectivity spec freeze:

- Start with exactly one provider in the first real-provider commit; do not enable OpenAI, Anthropic, and Google Gemini together.
- The request body shape must stay the same as the current connectivity-test stub.
- The backend must select the provider-specific adapter from the registry; the route handler must not import SDKs or construct provider requests.
- The adapter may read only the selected provider's server-side env var and may return only boolean key presence plus coarse result fields.
- The adapter must enforce `timeoutMs <= 5000` and `responseBodyLimitBytes <= 4096` before the feature flag can allow a request.
- A real request can be attempted only when the provider is supported, the model is non-empty, `purpose=manual_connectivity_test`, `secondConfirm=true`, `confirmText` is non-empty, `manualConnectivityTestActive=true`, and `realProviderRequestsAllowed=true`.
- Even on success, the response must not include provider response body, model text, token usage, cost, request headers, authorization headers, raw error body, raw key, key suffix, or masked key fragment.
- The verification script for the first real provider must be able to run without real credentials and still pass blocked/missing-key/no-side-effect cases.

OpenAI-compatible relay first-provider candidate plan:

- The first real-provider candidate is an OpenAI-compatible relay only, not official OpenAI. Anthropic, Google Gemini, and official OpenAI must stay on disabled adapters during this step.
- This is still a plan, not an implementation. Do not import the OpenAI SDK, do not call OpenAI or the relay, and do not change `manualConnectivityTestActive` or `realProviderRequestsAllowed` yet.
- The candidate provider id should be distinct from official OpenAI, for example `openai_compat`, so official OpenAI and relay credentials cannot be mixed.
- The candidate adapter id should be introduced separately from the current disabled adapter, for example `openai_compat_manual_connectivity_adapter`, while preserving disabled adapters for default behavior.
- The adapter may read only backend env vars dedicated to the relay: `AGENT_SWARM_OPENAI_COMPAT_API_KEY` and `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`. It must never accept an API key or base URL from the frontend or request body.
- The implementation must validate the relay base URL against an operator-provided allowlist or exact env var. It must not accept arbitrary URLs, redirects to non-HTTPS endpoints, localhost, private network targets, or request-time base URL overrides.
- The fixed connectivity model must be configured as relay-specific metadata, because relay model names may not match official OpenAI names. Do not assume `gpt-4.1-mini` works for the relay.
- The future real request must be a fixed minimal connectivity ping. It must not accept free-form prompt text, system prompt text, Agent context, files, tools, Runner job ids, arbitrary headers, arbitrary URLs, or arbitrary HTTP options.
- The feature flag change must be explicit and reviewed in the same commit as the first relay adapter or in a preceding dedicated commit. Until then, preflight may report the flag as requested but must keep real provider requests blocked.
- The first relay implementation commit must keep all no-side-effect guarantees: no SQLite/runtime-state writes, no task/approval/Runner job/Agent creation, no model-call records, no billing records, no stored provider response, and no prompt/result logging.
- The first relay verification must pass without real credentials by covering `feature_disabled`, `missing_key`, `missing_base_url`, `unsupported_provider`, `unsupported_model`, `timeout`, `provider_error`, invalid base URL, and no-side-effect cases.
- A real credential run, if ever performed manually, must be a separate operator action with the server env vars set outside Git and with logs checked for absence of key, base URL secrets, prompt, result, provider body, headers, token usage, and cost.

OpenAI-compatible relay disabled preflight implementation:

- `openai_compat` is now a Model Gateway provider metadata entry, but it is still disabled and cannot make network requests.
- `GET /api/model-gateway/status` may expose `openai_compat` with `keyEnvVar=AGENT_SWARM_OPENAI_COMPAT_API_KEY`, `baseUrlEnvVar=AGENT_SWARM_OPENAI_COMPAT_BASE_URL`, and boolean key/base-url presence only.
- `services/api/model-gateway-adapters.js` includes `openai_compat_disabled_connectivity_adapter`; this is metadata only and must still return blocked behavior.
- `modelGatewayConnectivityPreflight(...)` checks relay base URL presence and safe shape without returning the URL value.
- A relay base URL is considered unsafe when it is missing, not parseable, not `https:`, points to localhost, or points to common private IPv4 ranges.
- The HTTP API must not accept relay base URL overrides. Test-only base URL injection is limited to direct backend helper calls from regression scripts.
- `scripts/verify-model-gateway.ps1` must cover `missing_base_url`, `invalid_base_url`, safe-shape-but-feature-disabled, and all-false side effects for `openai_compat`.
- `scripts/verify-local-ui.ps1` remains the browser smoke entry and may keep overlapping assertions until the UI script is narrowed in a later cleanup batch.

OpenAI-compatible relay adapter interface checkpoint:

- `services/api/model-gateway-adapters.js` now exposes `openai_compat_manual_connectivity_adapter` as future adapter metadata only, with current mode `interface_disabled`.
- The relay interface checkpoint is not wired to real HTTP, provider SDKs, or real relay requests. It only documents and verifies the future adapter input/output boundary.
- `docs/relay-provider-info-checklist.md` records the non-secret relay documentation facts required before real request implementation.
- The operator has provided non-secret cheng.pink relay facts: base URL shape `https://api.cheng.pink/v1`, Chat Completions endpoint `/v1/chat/completions`, optional Responses endpoint `/v1/responses`, and minimal test model `gpt-5.4-mini`.
- Future implementation must normalize the optional `/v1` base URL suffix and endpoint path to avoid duplicated paths such as `/v1/v1/chat/completions`.
- The interface only accepts backend-shaped manual connectivity inputs: provider id, fixed relay model id, purpose, preflight result, timeout limit, and response body limit.
- The future adapter must read the relay key and base URL only from server env. It must not accept API keys, base URLs, free-form prompts, Agent context, files, tool calls, Runner jobs, arbitrary headers, arbitrary URLs, or arbitrary HTTP options from the request body.
- The interface returns only coarse blocked status, coarse `errorCategory`, redaction booleans, duration metadata, and a request-shape contract. It must not return key values, base URL values, request headers, provider bodies, model text, token usage, cost, or raw errors.
- `scripts/verify-model-gateway.ps1` directly calls the backend helper and verifies relay interface failure paths for missing key, missing base URL, invalid base URL, unsupported provider, unsupported model, timeout, provider error, and feature disabled.
- All relay interface cases must keep `realProviderRequestAttempted=false`, `providerResponseStored=false`, and all side effects false.

DeepSeek provider information checkpoint:

- `docs/deepseek-provider-info-checklist.md` records non-secret facts from the official DeepSeek API docs before any real DeepSeek request implementation.
- DeepSeek is a distinct provider candidate from official OpenAI, Anthropic, Google Gemini, and unknown OpenAI-compatible relays.
- Current status remains documentation-only: no DeepSeek SDK import, no DeepSeek HTTP request, no feature flag activation, no real model call.
- The future provider id should be distinct, such as `deepseek`, and the future key env var should be server-only, such as `AGENT_SWARM_DEEPSEEK_API_KEY`.
- The first DeepSeek manual ping must remain backend-only, fixed, non-streaming, timeout-limited, response-size-limited, redacted, and manually triggered.
- It must not accept free-form prompts, Agent context, files, tools, Runner job ids, arbitrary headers, arbitrary URLs, or client-provided API keys.

Cheng relay fixed manual ping spec:

- `docs/cheng-relay-manual-ping-spec.md` freezes the cheng.pink `openai_compat` manual ping before implementation.
- The fixed model is `gpt-5.4-mini`; the fixed provider request is a non-stream Chat Completions ping and must not accept client prompt text.
- The future adapter must normalize `AGENT_SWARM_OPENAI_COMPAT_BASE_URL` whether it is configured as `https://api.cheng.pink` or `https://api.cheng.pink/v1`, and must produce only `/v1/chat/completions`.
- Acceptance checks must cover feature disabled, missing key, missing base URL, invalid base URL, supported base URLs with and without `/v1`, unsupported provider, unsupported model, timeout, provider error, and all-false side effects without real credentials.
- Current status remains documentation-only: no relay SDK, no relay HTTP request, no feature flag activation, no real model call.

Cheng relay request builder checkpoint:

- `services/api/model-gateway-adapters.js` now exports pure local helpers for the cheng.pink relay manual ping request shape and URL normalization.
- `buildChengRelayManualPingRequest(...)` only builds a deterministic endpoint/body preview. It does not read API keys, does not read env vars, does not make HTTP requests, and reports `realProviderRequestAttempted=false`.
- The builder accepts only base URL and fixed model inputs for local verification, normalizes `https://api.cheng.pink`, `https://api.cheng.pink/v1`, and `https://api.cheng.pink/v1/` to `https://api.cheng.pink/v1/chat/completions`, and rejects unsafe URLs or unsupported models.
- `scripts/verify-model-gateway.ps1` verifies the builder through direct backend helper calls without real credentials or provider network calls.

Model Gateway verification script checkpoint:

- `scripts/verify-model-gateway.ps1` is the dedicated non-browser acceptance entry for Model Gateway.
- The script verifies status, dry-run, connectivity-test disabled stub, preflight failure paths, disabled adapter registry metadata, `openai_compat` interface-disabled relay metadata, cheng.pink request builder shape, and feature flag boundary.
- The script must not open a browser, read real API keys, require provider SDKs, call real providers, write SQLite/runtime state, create tasks/approvals/Runner jobs, trigger Agents, execute Runner, store provider responses, or log prompt/result/provider body content.

## 2026-06-08 实现备注：工作流只读接口

当前 Mock API 已实现工作流只读数据：

- `GET /api/projects/:projectId/workflows` 返回 `workflows` 数组。
- `GET /api/projects/:projectId/dashboard` 同时返回 `workflows`，供首页工作流总览和工作流编排页复用。
- 当前只支持展示流程、节点和依赖连线，不支持编辑、运行和保存编排。

## 2026-06-09 实现备注：Runner 状态只读展示

当前 Mock API 已把 Runner 状态纳入运行与调度页：

- `GET /api/projects/:projectId/runner/status` 返回本地 Runner 连接状态、版本、工作区、权限边界和最后心跳。
- `GET /api/projects/:projectId/dashboard` 同时返回 `runnerStatus`，供前端运行与调度页一次聚合渲染。
- 当前只读展示 Runner 状态和权限，不执行本地命令、不写文件、不发起网络请求、不修改 Git。
