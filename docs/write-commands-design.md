# 写入 Commands 设计

日期：2026-06-14

本文用于承接 `docs/write-commands-security-design.md`，确认写入 commands 的参数、校验、状态流转、返回值和副作用边界。本文只完成设计，不代表已经进入 Rust 实现。

## 一、总边界

所有写入 commands 必须遵守：

- 前端不可信，Rust 层必须重新校验输入。
- commands 层只接收参数并调用 services。
- services 层负责输入校验、状态机和业务规则。
- db 层只做参数化 SQL 和事务。
- 不开放真实 Runner。
- 不调用真实模型。
- 不导入 provider SDK。
- 不读取或返回 raw key。
- 不写用户项目文件。
- 不修改 Git。
- 不写保护路径。

## 二、写入 Commands 状态

| command | 状态 | 说明 |
|---------|------|------|
| `create_task` | 已设计 | 单个普通任务创建入口。 |
| `update_task_status` | 已设计 | 任务状态机流转。 |
| `create_approval` | 已设计 | 手动创建审批记录。 |
| `approve_approval` | 已设计 | 审批进入批准终态。 |
| `reject_approval` | 已设计 | 审批进入拒绝终态。 |
| `patch_only_approval` | 已设计 | 审批进入仅补丁终态。 |

## 三、`create_task`

### 3.1 用途

`create_task` 是通用单任务创建入口，用于在当前本地项目下创建一条普通任务记录。

它不替代旧项目中的 `project_plan` 审批批量任务生成流程：

- 旧 `project_plan` 流程：审批通过后批量生成计划任务和只读 Runner request。
- 新 `create_task`：只创建单个普通 task，不创建 Runner request，不创建 approval。

### 3.2 前端调用形态

```ts
invoke("create_task", {
  input: {
    title: string,
    description?: string | null,
    priority: "low" | "medium" | "high",
    assigned_agent_id?: string | null,
    depends_on?: string[],
    risk_level?: "low" | "medium" | "high" | null
  }
});
```

### 3.3 前端不得传入

以下字段由 Rust 层生成或决定，前端不得传入：

- `id`
- `project_id`
- `status`
- `created_at`
- `updated_at`

Rust 层规则：

- `id`：Rust 层生成，格式后续实现时固定。
- `project_id`：读取当前本地项目。
- `status`：固定为 `queued`。
- `created_at` / `updated_at`：Rust 层生成当前时间。

### 3.4 输入校验

| 字段 | 规则 |
|------|------|
| `title` | 必填，trim 后长度为 1 到 120 字符。 |
| `description` | 可为空；非空时 trim 后不超过 2000 字符。 |
| `priority` | 必填，只能是 `low`、`medium`、`high`。 |
| `assigned_agent_id` | 可为空；非空时必须存在于当前项目的 `agents` 表。 |
| `depends_on` | 可为空数组；每个 task id 必须存在于当前项目；禁止重复。 |
| `risk_level` | 可为空；非空时只能是 `low`、`medium`、`high`。 |

补充规则：

- `depends_on` 第一版不允许引用将来才会创建的任务。
- `depends_on` 第一版不需要做复杂环检测，因为新任务 id 由 Rust 生成，前端无法让新任务依赖自己。
- 字符串字段统一 trim 后再入库。

### 3.5 关联检查

- 当前项目必须存在。
- `assigned_agent_id` 非空时，目标 Agent 必须属于当前项目。
- `depends_on` 非空时，所有依赖任务必须属于当前项目。

### 3.6 返回值

第一版返回：

```ts
{
  task: TaskSummary
}
```

`TaskSummary` 与只读 `list_tasks` 当前返回结构保持一致。

### 3.7 错误语义

| 场景 | 错误语义 |
|------|----------|
| `title` 为空或超长 | `invalid_input` |
| `description` 超长 | `invalid_input` |
| `priority` 非法 | `invalid_input` |
| `risk_level` 非法 | `invalid_input` |
| `assigned_agent_id` 不存在 | `not_found` |
| `depends_on` 存在重复项 | `invalid_input` |
| `depends_on` 引用不存在任务 | `not_found` |
| 当前项目不存在 | `not_found` |
| SQLite 写入失败 | `database_error` |

### 3.8 副作用边界

`create_task` 当前阶段只允许：

- 写入 SQLite `tasks` 表。

`create_task` 当前阶段禁止：

- 创建 approval。
- 创建 Runner request。
- 创建 Runner job。
- 触发 Agent。
- 调用真实模型。
- 写用户项目文件。
- 修改 Git。
- 写保护路径。

### 3.9 与审批的关系

`create_task` 第一版不要求审批，因为它只是创建任务记录，不执行任务、不写文件、不改 Git。

如果 `risk_level = high`：

- 第一版只保存风险等级。
- 不自动创建 approval。
- 不自动触发 Runner。

后续如果要把高风险任务创建纳入审批，必须先更新本文档和 `docs/write-commands-security-design.md`。

### 3.10 测试要求

实现 `create_task` 时至少新增 Rust 测试：

- 正常创建任务，返回 `status = queued`。
- `title` 为空被拒绝。
- `title` 超过 120 字符被拒绝。
- `description` 超过 2000 字符被拒绝。
- 非法 `priority` 被拒绝。
- 非法 `risk_level` 被拒绝。
- 不存在的 `assigned_agent_id` 被拒绝。
- 不存在的 `depends_on` task id 被拒绝。
- 重复的 `depends_on` task id 被拒绝。
- 创建任务不会创建 approval。
- 创建任务不会创建 Runner request / Runner job。

## 四、`update_task_status`

### 4.1 用途

`update_task_status` 用于变更单个任务状态，并强制遵守任务状态机。

### 4.2 前端调用形态

```ts
invoke("update_task_status", {
  input: {
    id: string,
    status: "queued" | "running" | "blocked" | "waiting_user" | "completed" | "failed" | "cancelled"
  }
});
```

### 4.3 前端不得传入

- `project_id`
- `updated_at`
- 任务其他字段

Rust 层规则：

- 根据当前本地项目查找任务。
- `updated_at` 由 Rust 层生成。
- 第一版只改 `status` 和 `updated_at`。

### 4.4 输入校验

| 字段 | 规则 |
|------|------|
| `id` | 必填，必须指向当前项目中的任务。 |
| `status` | 必填，只能是 7 个合法 TaskStatus。 |

第一版不接收 `reason`。如果后续要为 `failed`、`blocked` 或 `waiting_user` 增加原因字段，必须先更新本文档和数据模型。

### 4.5 状态机

第一版允许：

```text
queued -> running
queued -> cancelled
running -> completed
running -> blocked
running -> waiting_user
running -> failed
running -> cancelled
blocked -> running
waiting_user -> running
waiting_user -> cancelled
```

禁止：

- `completed` -> 任意状态
- `failed` -> 任意状态
- `cancelled` -> 任意状态
- 任意状态 -> 非法状态
- 不存在任务 -> 任意状态

### 4.6 返回值

```ts
{
  task: TaskSummary
}
```

### 4.7 错误语义

| 场景 | 错误语义 |
|------|----------|
| `id` 为空 | `invalid_input` |
| `status` 非法 | `invalid_input` |
| 任务不存在 | `not_found` |
| 任务不属于当前项目 | `not_found` |
| 状态流转非法 | `invalid_transition` |
| SQLite 写入失败 | `database_error` |

### 4.8 副作用边界

`update_task_status` 只允许更新 SQLite `tasks` 表中的任务状态。

禁止：

- 创建 approval。
- 创建 Runner request / Runner job。
- 触发 Agent。
- 调用真实模型。
- 写用户项目文件。
- 修改 Git。

### 4.9 测试要求

- `queued -> running` 通过。
- `running -> completed` 通过。
- `running -> blocked` 通过。
- `blocked -> running` 通过。
- `completed -> running` 被拒绝。
- 非法 `status` 被拒绝。
- 不存在任务被拒绝。
- 更新状态不会创建 approval。
- 更新状态不会创建 Runner request / Runner job。

## 五、`create_approval`

### 5.1 用途

`create_approval` 用于手动创建一条审批记录，初始状态固定为 `pending`。

第一版只创建审批记录，不绑定自动拦截逻辑。

### 5.2 前端调用形态

```ts
invoke("create_approval", {
  input: {
    task_id?: string | null,
    request_agent_id: string,
    target_service: "task" | "approval" | "runner" | "agent_config" | "model_gateway",
    operation_types: string[],
    risk_level: "low" | "medium" | "high",
    reason?: string | null
  }
});
```

### 5.3 前端不得传入

- `id`
- `project_id`
- `status`
- `approved_at`
- `rejected_at`
- `created_at`
- `updated_at`

Rust 层规则：

- `id` 由 Rust 层生成。
- `project_id` 读取当前本地项目。
- `status` 固定为 `pending`。
- `created_at` / `updated_at` 由 Rust 层生成。
- `approved_at` / `rejected_at` 固定为空。

### 5.4 输入校验

| 字段 | 规则 |
|------|------|
| `task_id` | 可为空；非空时必须存在于当前项目的 `tasks` 表。 |
| `request_agent_id` | 必填，必须存在于当前项目的 `agents` 表。 |
| `target_service` | 必填，只能是 `task`、`approval`、`runner`、`agent_config`、`model_gateway`。 |
| `operation_types` | 必填，必须是非空数组；每一项必须在允许列表内；必须去重。 |
| `risk_level` | 必填，只能是 `low`、`medium`、`high`。 |
| `reason` | 可为空；非空时 trim 后不超过 2000 字符。 |

第一版允许的 `operation_types`：

```text
task_create
task_status_update
approval_create
approval_approve
approval_reject
approval_patch_only
runner_request_readonly
agent_config_review
model_gateway_review
```

### 5.5 关联检查

- 当前项目必须存在。
- `request_agent_id` 必须属于当前项目。
- `task_id` 非空时，任务必须属于当前项目。

### 5.6 返回值

```ts
{
  approval: ApprovalSummary
}
```

返回的 `ApprovalSummary.status` 必须是 `pending`。

### 5.7 错误语义

| 场景 | 错误语义 |
|------|----------|
| `request_agent_id` 为空或不存在 | `not_found` |
| `task_id` 不存在 | `not_found` |
| `target_service` 非法 | `invalid_input` |
| `operation_types` 为空、重复或包含非法值 | `invalid_input` |
| `risk_level` 非法 | `invalid_input` |
| `reason` 超长 | `invalid_input` |
| SQLite 写入失败 | `database_error` |

### 5.8 副作用边界

`create_approval` 只允许写入 SQLite `approvals` 表。

禁止：

- 自动批准。
- 自动拒绝。
- 创建 Runner request / Runner job。
- 触发 Agent。
- 调用真实模型。
- 写用户项目文件。
- 修改 Git。

### 5.9 测试要求

- 正常创建审批，返回 `status = pending`。
- 非法 `target_service` 被拒绝。
- 空 `operation_types` 被拒绝。
- 重复 `operation_types` 被拒绝。
- 非法 `operation_types` 被拒绝。
- 非法 `risk_level` 被拒绝。
- 超长 `reason` 被拒绝。
- 不存在 `request_agent_id` 被拒绝。
- 不存在 `task_id` 被拒绝。
- 创建审批不会创建 Runner request / Runner job。

## 六、`approve_approval`

### 6.1 用途

`approve_approval` 用于把一条 `pending` 审批标记为 `approved`。

这是最敏感的写入 command。第一版只更新 SQLite 审批记录，不执行审批所代表的真实动作。

### 6.2 前端调用形态

```ts
invoke("approve_approval", {
  input: {
    id: string
  }
});
```

### 6.3 前端不得传入

- `status`
- `approved_at`
- `updated_at`
- 任何 Runner、文件、Git 或模型执行参数

Rust 层规则：

- 只能执行 `pending -> approved`。
- `approved_at` / `updated_at` 由 Rust 层生成。

### 6.4 返回值

```ts
{
  approval: ApprovalSummary
}
```

### 6.5 错误语义

| 场景 | 错误语义 |
|------|----------|
| `id` 为空 | `invalid_input` |
| 审批不存在 | `not_found` |
| 审批不属于当前项目 | `not_found` |
| 审批不是 `pending` | `invalid_transition` |
| SQLite 写入失败 | `database_error` |

### 6.6 副作用边界

即使 `target_service = runner`，第一版也只更新审批状态。

禁止：

- 创建 Runner request / Runner job。
- 执行命令。
- 写用户项目文件。
- 修改 Git。
- 触发 Agent。
- 调用真实模型。
- 读取 raw key。

### 6.7 测试要求

- `pending -> approved` 通过。
- `approved -> approved` 被拒绝。
- `rejected -> approved` 被拒绝。
- `patch_only -> approved` 被拒绝。
- 不存在审批被拒绝。
- `target_service = runner` 的审批通过后不创建 Runner request / Runner job。

## 七、`reject_approval`

### 7.1 用途

`reject_approval` 用于把一条 `pending` 审批标记为 `rejected`。

### 7.2 前端调用形态

```ts
invoke("reject_approval", {
  input: {
    id: string,
    reject_reason?: string | null
  }
});
```

### 7.3 前端不得传入

- `status`
- `rejected_at`
- `updated_at`

Rust 层规则：

- 只能执行 `pending -> rejected`。
- `rejected_at` / `updated_at` 由 Rust 层生成。
- `reject_reason` 可为空；非空时 trim 后入库。

### 7.4 输入校验

| 字段 | 规则 |
|------|------|
| `id` | 必填，必须指向当前项目中的审批。 |
| `reject_reason` | 可为空；非空时 trim 后不超过 2000 字符。 |

### 7.5 返回值

```ts
{
  approval: ApprovalSummary
}
```

### 7.6 错误语义

| 场景 | 错误语义 |
|------|----------|
| `id` 为空 | `invalid_input` |
| `reject_reason` 超长 | `invalid_input` |
| 审批不存在 | `not_found` |
| 审批不属于当前项目 | `not_found` |
| 审批不是 `pending` | `invalid_transition` |
| SQLite 写入失败 | `database_error` |

### 7.7 副作用边界

`reject_approval` 只更新 SQLite 审批记录。

禁止：

- 创建 Runner request / Runner job。
- 执行命令。
- 写用户项目文件。
- 修改 Git。
- 触发 Agent。
- 调用真实模型。

### 7.8 测试要求

- `pending -> rejected` 通过。
- 超长 `reject_reason` 被拒绝。
- `approved -> rejected` 被拒绝。
- `rejected -> rejected` 被拒绝。
- `patch_only -> rejected` 被拒绝。
- 不存在审批被拒绝。

## 八、`patch_only_approval`

### 8.1 用途

`patch_only_approval` 用于把一条 `pending` 审批标记为 `patch_only`，表示仅保留补丁或审查意图，不执行真实动作。

第一版不接收 patch 内容，也不写用户项目文件。

### 8.2 前端调用形态

```ts
invoke("patch_only_approval", {
  input: {
    id: string
  }
});
```

### 8.3 前端不得传入

- `status`
- `updated_at`
- patch 内容
- 文件路径
- Git 参数
- Runner 参数

Rust 层规则：

- 只能执行 `pending -> patch_only`。
- `updated_at` 由 Rust 层生成。
- 第一版不设置 `approved_at` 或 `rejected_at`。

### 8.4 返回值

```ts
{
  approval: ApprovalSummary
}
```

### 8.5 错误语义

| 场景 | 错误语义 |
|------|----------|
| `id` 为空 | `invalid_input` |
| 审批不存在 | `not_found` |
| 审批不属于当前项目 | `not_found` |
| 审批不是 `pending` | `invalid_transition` |
| SQLite 写入失败 | `database_error` |

### 8.6 副作用边界

`patch_only_approval` 只更新 SQLite 审批记录。

禁止：

- 写用户项目文件。
- 生成真实 patch 文件。
- 创建 Runner request / Runner job。
- 执行命令。
- 修改 Git。
- 触发 Agent。
- 调用真实模型。

### 8.7 测试要求

- `pending -> patch_only` 通过。
- `approved -> patch_only` 被拒绝。
- `rejected -> patch_only` 被拒绝。
- `patch_only -> patch_only` 被拒绝。
- 不存在审批被拒绝。
- 调用后不写用户项目文件。
- 调用后不创建 Runner request / Runner job。

## 九、与旧项目的关系

旧项目保留为业务语义参考，不直接搬迁 Node.js 代码。

| 新写入 command | 旧项目参考 | 新架构处理 |
|----------------|------------|------------|
| `create_task` | 旧 `project_plan` 审批后会批量生成任务。 | 新 `create_task` 只创建单个普通任务，不替代 `project_plan` 批量生成流程。 |
| `update_task_status` | 旧 `services/api/server.js` 中的 `transitionTask`。 | 复用状态语义，重写为 Rust services 状态机。 |
| `create_approval` | 旧 `project-plan.js` 会构造审批对象。 | 复用字段语义，重写为通用 Rust 审批创建。 |
| `approve_approval` | 旧审批通过可能触发 `project_plan` 实例化。 | 第一版只改审批状态，不触发批量任务、Runner request 或执行动作。 |
| `reject_approval` | 旧审批状态包含 `rejected`。 | 复用状态语义，新增 `reject_reason` 长度校验。 |
| `patch_only_approval` | 旧审批状态包含 `patch_only`。 | 复用状态语义，但不写文件、不生成真实补丁。 |

暂不迁移：

- `project_plan` 审批后批量生成 5 个任务。
- `project_plan` 审批后生成只读 Runner request。
- Runner job 生命周期。
- Agent Run 真实链路。
- Git checkpoint 执行。
- 文件写入和 patch 落盘。

这些能力后续必须作为独立迁移项设计，不得混进本批通用写入 commands。

## 十、实现顺序建议

设计完成后，Rust 实现建议按以下顺序小步提交：

1. `create_task`
2. `update_task_status`
3. `create_approval`
4. `approve_approval` / `reject_approval` / `patch_only_approval`

说明：

- `create_task` 最基础，可以先验证输入校验、关联检查和参数化写入。
- `update_task_status` 可以先验证状态机。
- `create_approval` 可以验证审批输入校验和关联检查。
- 三个审批终态 command 可以共享终态校验。
- 不建议一次性实现全部写入 command；每一组实现后都应补 Rust 测试并提交。
