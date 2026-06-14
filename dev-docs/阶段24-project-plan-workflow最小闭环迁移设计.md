# 阶段 24：project_plan / Workflow 最小闭环迁移设计

日期：2026-06-15

本文定义如何把旧 MVP-0.3 的项目计划闭环迁到新 Tauri/Rust 架构。阶段 24 只做本地确定性模板、SQLite 记录和共享 UI，不接真实模型、不启用真实 Runner、不写用户项目文件。

## 一、目标

把旧项目已经验证过的闭环迁入新架构：

```text
用户输入项目想法
-> 本地确定性模板生成 project_plan 审批草案
-> 用户审批
-> 批量生成 5 个 queued 任务
-> 批量生成 5 条只读 Runner request 记录
-> 写入 runtime_events 审计
-> UI 展示计划、任务和只读队列
```

阶段 24 的价值是补齐旧 MVP 在新 Tauri/Rust 主线中的核心缺口，让应用在不依赖真实模型和 Runner 的情况下也能完整演示“从想法到任务拆解”的本地工作流。

## 二、明确不做

- 不调用真实模型。
- 不导入 provider SDK。
- 不发 provider HTTP 请求。
- 不读取 raw key。
- 不启用真实 Runner。
- 不执行命令。
- 不写用户项目文件。
- 不修改 Git。
- 不做云同步。
- 不做完整权限系统。
- 不复用旧 Node.js HTTP 服务作为正式实现。

## 三、旧项目可迁移资产

来源：`services/api/project-plan.js` 和 `docs/mvp-0.3-project-plan-flow-spec.md`。

可迁移内容：

- 5 个固定角色切片：`frontend`、`backend`、`qa`、`docs`、`reviewer`。
- 固定 Agent 分配：`agent_frontend`、`agent_backend`、`agent_qa`、`agent_docs`、`agent_reviewer`。
- 本地确定性模板：`generated_by=local_deterministic_template`。
- 审批目标：`target_service=project_plan`。
- 审批操作类型：`project_plan_approval`、`agent_task_assignment`、`runner_request_queue`。
- 只读 Runner operation type：`runner_request_readonly`。
- 审批前不得创建任务或 Runner request。
- 审批后不得重复实例化同一个 plan。

不迁移内容：

- 旧 Node.js route。
- 旧 Mock runtime state 写法。
- 旧 Web 原生 HTML/CSS/JS 页面。
- 旧 Python SQLite 脚本作为运行依赖。

## 四、数据模型

### 4.1 Migration 004

新增：

```text
data/migrations/004_add_project_plan_workflow.sql
```

建议新增两张表：

#### `project_plan_drafts`

保存项目计划草案本体，避免把完整草案硬塞进 `approvals.reason`。

字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PRIMARY KEY | `project_plan_{slug}` |
| `project_id` | TEXT NOT NULL | 当前项目 |
| `approval_id` | TEXT NOT NULL | 对应 `approvals.id` |
| `idea` | TEXT NOT NULL | 用户想法，限长 |
| `constraints` | TEXT | 约束，限长 |
| `summary` | TEXT NOT NULL | 本地摘要 |
| `status` | TEXT NOT NULL | `draft` / `approved` / `instantiated` / `cancelled` |
| `generated_by` | TEXT NOT NULL | 固定 `local_deterministic_template` |
| `requested_by` | TEXT NOT NULL | 默认 `local_user` |
| `created_at` | TEXT NOT NULL | 创建时间 |
| `updated_at` | TEXT NOT NULL | 更新时间 |

索引：

- `idx_project_plan_drafts_project_id`
- `idx_project_plan_drafts_approval_id`
- `idx_project_plan_drafts_status`

#### `runner_requests`

只读 Runner request 队列。使用 `runner_requests` 而不是 `runner_jobs`，是为了避免误解成可执行 Runner job。

字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | TEXT PRIMARY KEY | `runner_request_{task_id}` |
| `project_id` | TEXT NOT NULL | 当前项目 |
| `approval_id` | TEXT NOT NULL | 来源 project_plan 审批 |
| `task_id` | TEXT NOT NULL | 对应任务 |
| `status` | TEXT NOT NULL | 第一版固定 `queued` |
| `operation_types` | TEXT NOT NULL | JSON 字符串数组，必须包含 `runner_request_readonly` |
| `affected_files` | TEXT NOT NULL | JSON 字符串数组，只允许虚拟路径 |
| `checkpoint` | TEXT | 第一版为空 |
| `safety_note` | TEXT NOT NULL | 明确不执行命令、不写文件、不改 Git |
| `created_at` | TEXT NOT NULL | 创建时间 |
| `updated_at` | TEXT NOT NULL | 更新时间 |

索引：

- `idx_runner_requests_project_id`
- `idx_runner_requests_approval_id`
- `idx_runner_requests_task_id`
- `idx_runner_requests_status`

### 4.2 不新增 Workflow 表

阶段 24 不单独建 `workflows` 表。原因：

- 旧 MVP 的 workflow 是展示性编排，不是当前最小闭环必需。
- 现在最小目标是“项目计划审批 -> 任务 -> 只读 Runner request”。
- Workflow 页可以先由 `project_plan_drafts + tasks + runner_requests` 组合展示。

如后续要做可视化 workflow，再单独设计 `workflows` / `workflow_nodes`。

## 五、Rust 分层设计

### 5.1 新增 service

新增：

```text
apps/desktop/src-tauri/src/services/project_plan.rs
apps/desktop/src-tauri/src/commands/project_plan.rs
```

### 5.2 Commands

#### `create_project_plan_draft`

输入：

```rust
CreateProjectPlanDraftInput {
  idea: String,
  constraints: Option<String>,
  requested_by: Option<String>
}
```

行为：

- 校验 `idea` 非空，最长 500 字符。
- 校验 `constraints` 最长 2000 字符。
- 生成或复用一个 pending `project_plan` approval。
- 写入 `project_plan_drafts`。
- 不创建 task。
- 不创建 runner request。
- 不写用户项目文件。
- 不调用模型。
- 不触发 Runner。

实现注意：

- 现有通用 `create_approval` 的 `target_service` 白名单当前只服务普通审批写入。阶段 24 实现时必须显式允许 `project_plan`，或在 `project_plan` service 内使用专用写入逻辑；无论采用哪种方式，都不能让 `runner` / `agent_config` 的既有语义被改坏。

返回：

```text
ProjectPlanDraftResponse {
  draft,
  approval,
  planned_tasks,
  planned_runner_requests,
  side_effects
}
```

`planned_tasks` 和 `planned_runner_requests` 是内存预览，不落入 `tasks` / `runner_requests` 表，直到审批通过。

#### `approve_project_plan`

输入：

```rust
ApproveProjectPlanInput {
  approval_id: String,
  second_confirm: bool,
  confirm_text: String
}
```

行为：

- 必须要求 `second_confirm=true`。
- `confirm_text` 必须明确包含 `生成任务` 或 `确认生成任务`。
- 审批必须存在，且 `target_service=project_plan`。
- 审批必须是 `pending`。
- 找到对应 `project_plan_drafts`。
- 在同一个 SQLite transaction 内：
  - 把 approval 改成 `approved`。
  - 把 draft 改成 `instantiated`。
  - 插入 5 个 task。
  - 插入 5 条 `runner_requests`。
  - 插入 runtime event。
- 如果同一 approval 已经实例化，必须幂等返回已有 task/request，不重复插入。

返回：

```text
ApproveProjectPlanResponse {
  approval,
  draft,
  created_task_ids,
  created_runner_request_ids,
  side_effects
}
```

#### `list_project_plan_drafts`

只读列出计划草案。

#### `list_runner_requests`

只读列出 Runner request 队列。

### 5.3 不改现有 `approve_approval`

阶段 24 不把通用 `approve_approval` 偷偷改成会实例化 project plan。

原因：

- 现有 `approve_approval` 的验收明确“不创建 Runner job / 不触发副作用”。
- 直接改它会破坏已验收语义。
- project_plan 是特殊高风险审批，应该走显式命令 `approve_project_plan`，带二次确认。

补充约束：

- `approve_approval` 遇到 `target_service=project_plan` 时必须拒绝通过，避免绕过二次确认把计划卡成已批准但未实例化。
- 真正实例化计划只能走 `approve_project_plan`。

## 六、模板规则

迁移旧模板的 5 个固定任务：

| role | agent_id | priority | risk_level |
|------|----------|----------|------------|
| frontend | `agent_frontend` | high | medium |
| backend | `agent_backend` | high | medium |
| qa | `agent_qa` | medium | low |
| docs | `agent_docs` | medium | low |
| reviewer | `agent_reviewer` | high | medium |

任务 ID：

```text
task_{plan_id}_{role}
```

Runner request ID：

```text
runner_request_{task_id}
```

依赖规则：

- `frontend` 无依赖。
- 其余任务依赖 `frontend`，与旧模板保持一致。

安全说明固定包含：

```text
Read-only Runner request. No command execution, file write, network request, or Git change occurs.
```

## 七、前端 UI 设计

### 7.1 新增页面

新增：

```text
packages/ui/src/pages/ProjectPlanPage.tsx
```

导航新增：

```text
项目计划
```

### 7.2 页面能力

页面包含：

- 输入项目想法。
- 输入约束。
- 生成计划草案按钮。
- 草案卡片：摘要、5 个计划任务、5 条只读 Runner request 预览。
- 待审批状态展示。
- 二次确认输入框。
- 批准生成任务按钮。
- 已生成任务 / Runner request 的只读列表。

UI 只调用 Tauri commands，不直接写数据。

浏览器预览模式：

- 没有 Tauri host 时按钮 disabled。
- 可显示占位说明，不崩溃。

## 八、共享类型

新增到 `packages/shared`：

- `ProjectPlanDraftSummary`
- `PlannedTaskSummary`
- `RunnerRequestSummary`
- `CreateProjectPlanDraftInput`
- `ApproveProjectPlanInput`

新增到 `packages/agent-core`：

- `PROJECT_PLAN_ROLES`
- `PROJECT_PLAN_AGENT_ASSIGNMENTS`
- `isProjectPlanApprovalTarget()`

注意：

- `agent-core` 只放纯规则和常量，不放 UI 文案。
- UI 文案仍放 `packages/ui`。

## 九、验收标准

Rust 测试至少覆盖：

1. 创建草案成功，approval 为 `pending`，target_service 为 `project_plan`。
2. 创建草案不写 tasks。
3. 创建草案不写 runner_requests。
4. 空 idea 被拒绝。
5. idea / constraints 超长被拒绝。
6. 未二次确认时 `approve_project_plan` 被拒绝。
7. 普通 approval 不能被 `approve_project_plan` 实例化。
8. project_plan 审批通过后创建 5 个 tasks。
9. project_plan 审批通过后创建 5 条 runner_requests。
10. runner_requests 全部包含 `runner_request_readonly`。
11. 二次 approve 幂等，不重复创建 tasks / runner_requests。
12. runtime_events 记录一次 project_plan instantiated。

前端验证：

```powershell
cd packages/ui
npm run typecheck
npm run build
```

Rust 验证：

```powershell
cd apps/desktop/src-tauri
cargo fmt --check
cargo check
cargo test
```

通用验证：

```powershell
git diff --check
```

## 十、阶段完成定义

阶段 24 完成必须满足：

- Migration 004 已建表。
- Rust commands 已注册。
- UI 页面已接入。
- 审批前不会创建 tasks / runner_requests。
- 审批后确定性创建 5 个 tasks 和 5 条只读 runner_requests。
- 不触发真实 Runner。
- 不调用真实模型。
- 不写用户项目文件。
- 不改 Git。
- 全量验证通过。
- 路线、维护手册、交接说明同步。
