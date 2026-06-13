# 写入 Commands 设计

日期：2026-06-14

本文用于承接 `docs/write-commands-security-design.md`，逐个确认写入 commands 的参数、校验、状态流转、返回值和副作用边界。当前只确认 `create_task`，其余 command 继续保持待设计状态。

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
| `update_task_status` | 待设计 | 任务状态机流转。 |
| `create_approval` | 待设计 | 审批记录创建。 |
| `approve_approval` | 待设计 | 审批进入批准终态。 |
| `reject_approval` | 待设计 | 审批进入拒绝终态。 |
| `patch_only_approval` | 待设计 | 审批进入仅补丁终态。 |

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

## 四、后续待设计

下一步按以下顺序继续讨论：

1. `update_task_status`
2. `create_approval`
3. `approve_approval`
4. `reject_approval`
5. `patch_only_approval`
