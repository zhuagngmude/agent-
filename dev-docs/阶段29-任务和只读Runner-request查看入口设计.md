# 阶段 29：任务和只读 Runner request 查看入口

目标：阶段 27/28 已经能在人工审批 `project_plan` 后生成 `tasks` 和只读 `runner_requests`。阶段 29 只做“查看和核对入口”，让用户能在桌面端清楚看到某个项目计划草案审批后实际生成了哪些任务、每个任务对应哪条只读 Runner request，以及这些 request 为什么仍然不能执行。

这一阶段不开放 Runner 执行，不写用户项目文件，不改 Git，不调用真实模型，不新增可执行队列。

## 一、阶段边界

允许做：

- 在后端新增只读查询函数和 Tauri command，用于按 `approval_id` 查看某次项目计划审批生成的任务和只读 `runner_requests`。
- 在前端 `ProjectPlanPage` 增加“已生成任务 / 只读 Runner request 明细”区域。
- 让用户选中一个已实例化草案后，能看到该草案对应的真实落库任务和 request。
- 展示任务标题、角色、状态、优先级、风险、负责 Agent、依赖关系、只读操作类型、虚拟 affected files、安全说明。
- 保留现有全局 `list_runner_requests`，但新增更精确的“按审批读取”入口，避免页面只能看到混在一起的全部 request。
- 增加测试，确认这些查看入口不产生任何写入副作用。

禁止做：

- 不执行 Runner。
- 不新增 Runner 执行按钮。
- 不把 `runner_requests.status` 从 `queued` 改成 `running` / `completed`。
- 不执行命令。
- 不写用户项目文件。
- 不改 Git。
- 不调用真实模型。
- 不新增 `model_calls`。
- 不写 `runtime_events`。
- 不创建新的 `tasks` / `approvals` / `runner_requests`。
- 不展示 raw key、raw base URL、raw prompt、raw provider response、provider error 原文。
- 不展示 `model_call_id` / `audit_record_id` 明细。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 二、为什么阶段 29 要先做查看入口

现在链路已经能做到：

```text
项目想法
-> 真实模型草案预览
-> model_calls 安全审计
-> 保存为 project_plan_drafts + approvals
-> 人工审批
-> 生成 tasks + 只读 runner_requests
```

但用户还缺一个稳定入口来核对“审批之后到底生成了什么”。如果直接进入 Runner 执行，风险会跳得太快：用户无法先检查任务范围、虚拟 affected files、只读安全说明、任务依赖和 Agent 分配。

所以阶段 29 的正确目标是：把已经生成的任务队列展示清楚，让后续阶段再讨论“是否允许执行”。

## 三、是否新增 migration

不新增 migration。

阶段 29 复用已有表：

- `project_plan_drafts`
- `approvals`
- `tasks`
- `runner_requests`

不要改表结构。不要新增执行态表。不要新增 Runner job 表。

## 四、推荐后端设计

重点文件：

```text
apps/desktop/src-tauri/src/services/project_plan.rs
apps/desktop/src-tauri/src/commands/project_plan.rs
apps/desktop/src-tauri/src/lib.rs
packages/shared/src/types/project-plan.ts
packages/shared/src/index.ts
packages/ui/src/utils/desktopHost.ts
packages/ui/src/pages/ProjectPlanPage.tsx
```

### 4.1 新增响应类型

建议在 `services/project_plan.rs` 新增：

```rust
#[derive(Debug, Serialize)]
pub struct ProjectPlanExecutionPreview {
    pub draft: ProjectPlanDraftSummary,
    pub approval: ApprovalSummary,
    pub tasks: Vec<ProjectPlanTaskInstanceSummary>,
    pub runner_requests: Vec<RunnerRequestSummary>,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct ProjectPlanTaskInstanceSummary {
    pub id: String,
    pub project_id: String,
    pub role: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assigned_agent_id: Option<String>,
    pub depends_on: Vec<String>,
    pub risk_level: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

说明：

- `ProjectPlanTaskInstanceSummary` 可以复用 `tasks::TaskSummary`，但阶段 29 最好补一个 `role` 字段，方便前端按角色展示。
- `role` 第一版可以从 task id 解析，规则为 `task_{draft.id}_{role}`。解析不到时返回 `"unknown"`，但不要 panic。
- 如果不想新增 `ProjectPlanTaskInstanceSummary`，也可以直接返回 `TaskSummary`，前端用 task id 派生 role。更推荐后端派生，因为测试更好写。

### 4.2 新增 service 函数

建议新增：

```rust
pub fn get_project_plan_execution_preview(
    connection: &Connection,
    approval_id: String,
) -> Result<ProjectPlanExecutionPreview, String>
```

行为要求：

1. 获取当前项目 id。
2. 校验 `approval_id` 非空，长度限制建议沿用现有 `normalize_required_text(..., 1, 200, "approval_id")`。
3. 读取当前项目下的 `approval`。
4. 要求 `approval.target_service == "project_plan"`，否则返回：

```text
invalid_input: approval is not a project_plan approval
```

5. 读取当前项目下 `project_plan_drafts`。
6. 如果没有草案，返回：

```text
not_found: project plan draft not found
```

7. 如果 `draft.status != "instantiated"`，返回空 `tasks` 和空 `runner_requests`，不要报错。
8. 如果 `draft.status == "instantiated"`，按 `runner_requests.approval_id` 读取真实落库 request，再通过 `task_id` 读取对应 tasks。
9. 返回 `side_effects(false, false)`，确保：

```text
writes_project_files=false
modifies_git=false
executes_runner=false
calls_real_model=false
reads_raw_secrets=false
makes_network_requests=false
triggers_agents=false
creates_tasks=false
creates_runner_requests=false
```

### 4.3 查询顺序

必须从 `runner_requests` 反查实际落库的任务，不要按当前模板重新计算。

推荐 SQL：

```sql
SELECT
  rr.id,
  rr.project_id,
  rr.approval_id,
  rr.task_id,
  rr.status,
  rr.operation_types,
  rr.affected_files,
  rr.checkpoint,
  rr.safety_note,
  rr.created_at,
  rr.updated_at
FROM runner_requests rr
WHERE rr.project_id = ?1 AND rr.approval_id = ?2
ORDER BY rr.created_at, rr.id;
```

读取 tasks 时：

```sql
SELECT
  id, project_id, title, description, status, priority, assigned_agent_id,
  COALESCE(depends_on, '[]'), risk_level, created_at, updated_at
FROM tasks
WHERE project_id = ?1 AND id = ?2;
```

注意：

- 不要 JOIN 当前 `project_plan_task_templates` 作为必要条件。
- 模板可能在审批后被启停，查看入口必须展示“当时已经落库的真实结果”。
- 如果某条 `runner_requests.task_id` 找不到 task，应返回粗粒度数据库一致性错误，不要静默吞掉：

```text
database_error: runner request task is missing
```

### 4.4 新增 command

在 `commands/project_plan.rs` 新增：

```rust
#[tauri::command]
pub fn get_project_plan_execution_preview(
    state: tauri::State<'_, DbState>,
    approval_id: String,
) -> Result<ProjectPlanExecutionPreview, String>
```

并在 `lib.rs` 注册。

不要把它做成写入 command。它只读数据库。

## 五、前端设计

### 5.1 共享类型

在 `packages/shared/src/types/project-plan.ts` 增加：

```ts
export type ProjectPlanTaskInstanceSummary = {
  id: string;
  project_id: string;
  role: string;
  title: string;
  description: string | null;
  status: string;
  priority: string;
  assigned_agent_id: string | null;
  depends_on: string[];
  risk_level: string | null;
  created_at: string;
  updated_at: string;
};

export type ProjectPlanExecutionPreview = {
  draft: ProjectPlanDraftSummary;
  approval: ApprovalSummary;
  tasks: ProjectPlanTaskInstanceSummary[];
  runner_requests: RunnerRequestSummary[];
  side_effects: ProjectPlanSideEffects;
};
```

并从 `packages/shared/src/index.ts` 导出。

### 5.2 desktopHost 封装

在 `packages/ui/src/utils/desktopHost.ts` 增加：

```ts
export async function getProjectPlanExecutionPreview(
  approvalId: string,
): Promise<ProjectPlanExecutionPreview> {
  requireTauri();
  return invoke("get_project_plan_execution_preview", { approvalId });
}
```

注意 Tauri 参数名要和 command 函数参数一致。若 command 参数是 `approval_id`，前端 invoke 需要确认 Tauri 的 casing 行为，建议保持 Rust 参数名 `approval_id`，前端传 `{ approvalId }` 前要以实际项目既有写法为准。不要猜，写完用 typecheck/build 和桌面手测确认。

### 5.3 UI 入口

在 `ProjectPlanPage.tsx` 增加一个区域，建议放在“草案列表”和“批准生成任务”之间，或者放在当前“只读 Runner request 队列”之前。

标题建议：

```text
已生成任务和只读 Runner request
```

显示逻辑：

- 未选中草案：显示空态。
- 选中草案但 `draft.status !== "instantiated"`：提示“草案尚未审批实例化，暂无已生成任务”。
- 选中已实例化草案：调用 `getProjectPlanExecutionPreview(selectedApprovalId)`。
- 展示两个表：
  - 已生成任务
  - 对应只读 Runner request

任务表字段建议：

- 角色
- 任务标题
- 状态
- 负责 Agent
- 优先级
- 风险
- 依赖

Runner request 表字段建议：

- 队列记录 id
- task_id
- 状态
- operation_types
- affected_files
- safety_note

页面文案要克制，不要写大段说明。可以用一个 `Alert`：

```text
这里只展示审批后生成的任务和只读 Runner request，不执行 Runner，不写文件，不改 Git。
```

### 5.4 前端禁止项

- 不新增“执行”“运行”“开始”“应用补丁”“写入文件”“提交 Git”按钮。
- 不让用户编辑 `runner_requests`。
- 不让用户编辑 `tasks.description`。
- 不展示 `model_call_id`、`audit_record_id`。
- 不展示真实模型 prompt 或 response。
- 不新增网络请求配置、模型配置、key/base URL 输入。

## 六、测试要求

### 6.1 Rust service 测试

至少新增这些测试：

1. `execution_preview_for_pending_draft_returns_empty_lists`
   - 创建草案但不审批。
   - 调用 `get_project_plan_execution_preview`。
   - 返回 draft/approval。
   - `tasks.len() == 0`。
   - `runner_requests.len() == 0`。
   - 不写 runtime_events。

2. `execution_preview_after_approval_returns_persisted_tasks_and_requests`
   - 创建草案。
   - 审批生成任务。
   - 调用 preview。
   - 返回 5 个 tasks 和 5 条 runner_requests。
   - task ids 与 approve response 的 `created_task_ids` 一致。
   - request ids 与 approve response 的 `created_runner_request_ids` 一致。

3. `execution_preview_does_not_recompute_after_template_change`
   - 默认审批生成 5 个。
   - 启用 `security`。
   - 调用 preview。
   - 仍返回原始 5 个，不返回 security。

4. `execution_preview_rejects_cross_project_approval`
   - 插入另一个项目的 approval/draft/request。
   - 当前项目调用该 approval id。
   - 返回 `not_found`。

5. `execution_preview_rejects_non_project_plan_approval`
   - 使用普通 approval。
   - 返回 `invalid_input`。

6. `execution_preview_has_no_side_effects`
   - 调用前后统计：
     - `tasks`
     - `runner_requests`
     - `runtime_events`
     - `model_calls`
   - 数量不变。
   - side_effects 全 false。

### 6.2 前端验证

至少保证：

- `npm run typecheck` 通过。
- `npm run build` 通过。
- `ProjectPlanPage` 没有 `useCallback` 依赖遗漏。
- 未连接 Tauri 时不崩溃。
- 选中不同草案时会刷新明细，不显示上一个草案的旧结果。

### 6.3 全局验证命令

低级智能体完成后必须跑：

```powershell
cd F:\projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test

cd F:\projects\agent-swarm
git diff --check
rg -n ('response' + '_body_limit') dev-docs docs packages apps/desktop/src-tauri/src
rg -n "sk-[A-Za-z0-9]{20,}|Authorization: Bearer [A-Za-z0-9._-]+|api_key=|token=|password=" apps/desktop/src-tauri/src packages docs dev-docs scripts
```

旧错误名不应作为业务错误枚举出现。敏感扫描如果命中测试假值、规则文档、验证脚本样例，需要在交付说明里明确说明，不要混入真实 key。

## 七、文档同步

完成实现后同步：

```text
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
docs/data-model-draft.md
```

同步口径：

```text
阶段 29 完成：项目计划审批后生成的 tasks 和只读 runner_requests 已有桌面端查看入口；
该入口只读，不执行 Runner，不执行命令，不写文件，不改 Git，不调用模型；
显示内容来自已落库的 tasks/runner_requests，不按当前模板重新计算。
```

## 八、不要做的事

- 不新增 migration。
- 不新增 Runner 执行状态机。
- 不新增执行日志。
- 不新增文件写入预览。
- 不新增补丁生成。
- 不新增 Git checkpoint。
- 不新增 Agent 自动派工。
- 不改 `approve_approval` 让它能实例化 project plan。
- 不把只读 `runner_requests` 变成可执行 job。
- 不让前端显示任何真实密钥、真实 provider 配置或模型原文。

## 九、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 29 完成：任务和只读 Runner request 查看入口

改了哪些文件：
- ...

是否新增 migration：
- 否

核心行为：
- 选中未实例化草案时显示空任务/request 明细。
- 选中已实例化草案时，从已落库 tasks + runner_requests 读取真实生成结果。
- 模板变更后，查看入口仍显示审批当时已经生成的任务，不重新按当前模板计算。
- 只读展示，不执行 Runner、不写文件、不改 Git、不调用模型。

新增测试：
- ...

验证结果：
- npm run typecheck
- npm run build
- cargo fmt --check
- cargo check
- cargo test
- git diff --check
- 旧错误名扫描
- 敏感信息扫描

遗留风险：
- 无 / ...
```

最终阶段 29 口径：

```text
阶段 29 完成后：用户可以在桌面端查看 project_plan 审批实例化后真实落库的 tasks 和只读 runner_requests；
查看入口只读，不重新计算模板，不执行 Runner，不执行命令，不写用户项目文件，不改 Git，不调用真实模型。
```
