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

请求：

```json
{
  "model": "claude-ui",
  "status": "disabled",
  "permissions": ["read_project", "write_docs"]
}
```

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
      "document": "下一步开发路线.md",
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

## 2026-06-08 实现备注：工作流只读接口

当前 Mock API 已实现工作流只读数据：

- `GET /api/projects/:projectId/workflows` 返回 `workflows` 数组。
- `GET /api/projects/:projectId/dashboard` 同时返回 `workflows`，供首页工作流总览和工作流编排页复用。
- 当前只支持展示流程、节点和依赖连线，不支持编辑、运行和保存编排。
