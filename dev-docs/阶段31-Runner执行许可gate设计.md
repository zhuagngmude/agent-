# 阶段 31：Runner 执行许可 gate 设计

目标：阶段 30 已经可以把只读 `runner_requests` 转为执行前审查记录 `runner_preflight_reviews`，并创建 `target_service = runner_preflight` 的审批。阶段 31 要做的是在“执行前审查已被人工审批通过”之后，生成一条结构化的 Runner 执行许可 gate。

这一阶段仍然不执行 Runner，不执行命令，不写用户项目文件，不改 Git，不创建 Git checkpoint，不发网络请求，不调用真实模型，不创建可执行 Runner job。

阶段 31 的“许可 gate”不是执行按钮，也不是执行队列。它只是把将来执行前必须满足的许可条件、阻断状态、文件范围、操作范围和撤销状态落库，给阶段 32+ 做更严格的执行准备。

## 一、宪法约束

阶段 31 必须遵守 `docs/Agent宪法.md` 和 `docs/AI开发细则.md`：

- 不越过当前阶段边界。
- 不绕过审批写用户项目文件。
- Runner 不得自动执行命令、写文件、删文件、发网络请求或修改 Git。
- 写入、变更、Runner、Git checkpoint、模型相关能力必须进入审批链，不得直接开放。
- 数据库结构变更必须走 migration。
- 新增依赖必须先说明理由并获批准；阶段 31 不需要新增依赖。

结论：阶段 31 不是 Runner 执行阶段。它只允许创建和撤销“执行许可 gate”记录，不允许执行任何动作。

## 二、阶段边界

允许做：

- 新增 SQLite migration，保存 Runner 执行许可 gate。
- 对一条已存在的 `runner_preflight_reviews` 创建执行许可 gate。
- 创建 gate 前必须校验：
  - preflight review 属于当前项目。
  - preflight review 关联的 `runner_request` 仍存在。
  - preflight review 关联的 `task` 仍存在。
  - preflight review 关联的审批 `target_service = runner_preflight`。
  - preflight review 关联的审批 `status = approved`。
  - runner_request 仍是 `queued`。
  - runner_request 仍是只读类型，包含 `runner_request_readonly`。
  - preflight review 和 runner_request 的操作范围、文件范围一致。
- gate 记录必须显式保存：
  - `can_execute = 0`
  - `stage_boundary_locked = 1`
  - `status = blocked_by_stage_boundary`
  - `blocked_reasons` 包含 `runner_execution_disabled_by_stage_boundary`
- 支持撤销 gate，撤销只改 gate 自身状态，不改 task / runner_request / preflight / approval。
- 前端展示 gate 状态、阻断原因、撤销入口。
- 增加测试证明没有 Runner、命令、文件、Git、模型、网络副作用。

禁止做：

- 不执行 Runner。
- 不执行命令。
- 不写用户项目文件。
- 不删除文件。
- 不改 Git。
- 不创建 Git checkpoint。
- 不发网络请求。
- 不调用真实模型。
- 不读取 raw key。
- 不创建可执行 Runner job。
- 不新增真实执行队列表。
- 不把 `runner_requests.status` 改为 `running` / `completed` / `approved`。
- 不把 `tasks.status` 改为 `running` / `completed`。
- 不改 `approve_approval` 让通用审批自动触发 Runner 行为。
- 不让 Agent 自己批准执行。
- 不展示 raw prompt / raw provider response / raw error / raw secret。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 三、为什么阶段 31 仍不执行

阶段 30 解决的是“执行前要审查什么”。阶段 31 解决的是“审查通过后，执行许可应该怎样被结构化保存、怎样被阻断、怎样被撤销”。

不能在阶段 31 直接执行，因为真实执行还缺这些能力：

- 命令白名单。
- 文件写入范围锁。
- Git checkpoint 创建和失败回滚。
- 执行日志脱敏。
- 超时、取消、重试。
- 执行产物 diff 预览。
- 真实 Runner 与审批链的强绑定。

所以阶段 31 只做 gate，不做 executor。换句话说：可以把门框装好，但门还是锁着的。

## 四、数据模型

新增 migration：

```text
008_add_runner_execution_gates.sql
```

新增表：

```sql
CREATE TABLE IF NOT EXISTS runner_execution_gates (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  preflight_review_id TEXT NOT NULL,
  preflight_approval_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 0,
  stage_boundary_locked INTEGER NOT NULL DEFAULT 1,
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1,
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  revoked_reason TEXT,
  requested_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id),
  FOREIGN KEY (preflight_review_id) REFERENCES runner_preflight_reviews(id),
  FOREIGN KEY (preflight_approval_id) REFERENCES approvals(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_project_id
  ON runner_execution_gates(project_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_gates_project_runner_request
  ON runner_execution_gates(project_id, runner_request_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_gates_project_preflight_review
  ON runner_execution_gates(project_id, preflight_review_id);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_status
  ON runner_execution_gates(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_preflight_approval
  ON runner_execution_gates(preflight_approval_id);
```

字段说明：

- `runner_request_id`：沿用阶段 27/29 生成的只读 request。
- `preflight_review_id`：阶段 30 的审查记录。
- `preflight_approval_id`：阶段 30 创建的 `runner_preflight` 审批。
- `status` 第一版只允许：

```text
blocked_by_stage_boundary
revoked
```

- 阶段 31 不允许出现 `ready`、`approved_to_execute`、`running`、`completed`。
- `can_execute` 第一版必须恒为 `0`。
- `stage_boundary_locked` 第一版必须恒为 `1`。
- `blocked_reasons` 第一版必须包含：

```text
runner_execution_disabled_by_stage_boundary
```

- `operation_types`、`affected_files`、`blocked_reasons` 使用 JSON 数组字符串。
- `affected_files` 仍只能是 `virtual/...`，不能是真实文件路径。

## 五、后端模块

建议新增模块，避免继续膨胀 `project_plan.rs`：

```text
apps/desktop/src-tauri/src/services/runner_execution_gate.rs
apps/desktop/src-tauri/src/commands/runner_execution_gate.rs
```

需要注册：

```text
apps/desktop/src-tauri/src/services/mod.rs
apps/desktop/src-tauri/src/commands/mod.rs
apps/desktop/src-tauri/src/lib.rs
```

不要新增第三方依赖。

## 六、后端类型

### 6.1 创建 gate 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerExecutionGateInput {
    pub preflight_review_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
```

确认文本固定：

```text
我确认只创建执行许可记录，不执行Runner
```

### 6.2 撤销 gate 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerExecutionGateInput {
    pub gate_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}
```

确认文本固定：

```text
我确认撤销执行许可记录
```

### 6.3 输出类型

```rust
#[derive(Debug, Serialize)]
pub struct RunnerExecutionGateSummary {
    pub id: String,
    pub project_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub preflight_review_id: String,
    pub preflight_approval_id: String,
    pub status: String,
    pub risk_level: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub can_execute: bool,
    pub stage_boundary_locked: bool,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub revoked_reason: Option<String>,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerExecutionGateResponse {
    pub gate: RunnerExecutionGateSummary,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct RevokeRunnerExecutionGateResponse {
    pub gate: RunnerExecutionGateSummary,
    pub side_effects: ProjectPlanSideEffects,
}
```

`side_effects` 必须全部为 false：

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

说明：创建 gate 会写 `runner_execution_gates` 表，但不创建 task / runner_request / runtime_event / model_call。

## 七、后端服务行为

### 7.1 新增函数

```rust
pub fn create_runner_execution_gate(
    connection: &mut Connection,
    input: CreateRunnerExecutionGateInput,
) -> Result<CreateRunnerExecutionGateResponse, String>

pub fn list_runner_execution_gates(
    connection: &Connection,
) -> Result<Vec<RunnerExecutionGateSummary>, String>

pub fn revoke_runner_execution_gate(
    connection: &mut Connection,
    input: RevokeRunnerExecutionGateInput,
) -> Result<RevokeRunnerExecutionGateResponse, String>
```

### 7.2 创建 gate 步骤

1. 获取当前项目 id。
2. 校验 `preflight_review_id` 非空，长度 1..200。
3. 校验二次确认：

```text
second_confirm == true
confirm_text == "我确认只创建执行许可记录，不执行Runner"
```

4. 校验 `requested_by`：
   - 缺省为 `local_user`。
   - trim 后长度 1..120。
   - 不能包含 `sk-...`、`Authorization: Bearer ...`、`api_key=`、`token=`、`password=`。
5. 读取当前项目下的 `runner_preflight_reviews`。
6. 如果不存在，返回：

```text
not_found: preflight review not found
```

7. 校验 preflight review：
   - `project_id` 必须等于当前项目。
   - `status` 必须是 `blocked`。
   - `blocked_reasons` 必须包含 `runner_execution_disabled_by_stage_boundary`。
   - `operation_types` 不得包含真实执行操作。
   - `affected_files` 全部必须是安全虚拟路径。
8. 读取 preflight 关联 approval：
   - 必须存在。
   - 必须属于当前项目。
   - `target_service == "runner_preflight"`。
   - `status == "approved"`。
   - `task_id` 必须等于 preflight review 的 `task_id`。
9. 读取 preflight 关联 runner_request：
   - 必须存在。
   - 必须属于当前项目。
   - `status == "queued"`。
   - `task_id` 必须等于 preflight review 的 `task_id`。
   - `operation_types` 必须包含 `runner_request_readonly`。
   - `operation_types` 必须与 preflight review 记录一致。
   - `affected_files` 必须与 preflight review 记录一致。
10. 读取关联 task：
   - 必须存在。
   - 必须属于当前项目。
   - 不修改 task 状态。
11. 幂等检查：
   - 如果当前项目下同一 `preflight_review_id` 已有 gate，返回已有 gate。
   - 如果当前项目下同一 `runner_request_id` 已有 gate，返回已有 gate。
   - 不重复插入。
12. 插入 `runner_execution_gates`：
   - `id = runner_gate_{runner_request_id}`，必要时做安全字符归一。
   - `status = blocked_by_stage_boundary`
   - `can_execute = 0`
   - `stage_boundary_locked = 1`
   - `blocked_reasons = ["runner_execution_disabled_by_stage_boundary"]`
   - `requires_git_checkpoint = 1`
   - `requires_second_confirm = 1`
13. 返回 gate + 全 false side_effects。

### 7.3 撤销 gate 步骤

1. 获取当前项目 id。
2. 校验 `gate_id` 非空，长度 1..200。
3. 校验二次确认：

```text
second_confirm == true
confirm_text == "我确认撤销执行许可记录"
```

4. 校验 `revoked_reason`：
   - 可为空。
   - 非空 trim 后长度 1..500。
   - 不能包含敏感值。
5. 读取当前项目下 gate。
6. 如果不存在，返回：

```text
not_found: execution gate not found
```

7. 如果 `status == "revoked"`，幂等返回已有 gate。
8. 更新 gate：
   - `status = revoked`
   - `revoked_reason = normalized_reason`
   - `revoked_at = now`
   - `updated_at = now`
   - `can_execute` 仍为 `0`
   - `stage_boundary_locked` 仍为 `1`
9. 不修改 approval、preflight review、runner_request、task。
10. 返回 gate + 全 false side_effects。

### 7.4 禁止操作类型

创建 gate 时必须拒绝以下 operation type：

```text
command_execute
file_write
file_delete
git_commit
git_push
network_request
model_call
runner_execute
```

只允许继续承接阶段 27/28 生成的安全只读类型，例如：

```text
runner_request_readonly
frontend_plan
backend_plan
qa_plan
docs_plan
review_plan
security_review_plan
```

如果现有代码里的模板 operation type 与上面不完全一致，以代码中阶段 28 seed 的实际白名单为准，但必须保持“只读/计划类”，不能加入真实执行类。

### 7.5 affected_files 校验

每个 affected file 必须满足：

```text
以 virtual/ 开头
不包含 ..
不包含 \
不包含 :
不包含 ~
长度 1..240
```

### 7.6 不要改这些路径

- 不改 `approve_approval` 的语义。
- 不改 `approve_project_plan` 的语义。
- 不改 `runner_preflight_reviews.status`。
- 不改 `runner_requests.status`。
- 不改 `tasks.status`。
- 不写 `runtime_events`。
- 不写 `model_calls`。
- 不创建新的 runner request。

阶段 31 写入的是“gate 记录”，不是执行事件；暂不写 runtime events，避免让人误判为真实运行。

## 八、Tauri commands

新增：

```rust
#[tauri::command]
pub fn create_runner_execution_gate(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerExecutionGateInput,
) -> Result<CreateRunnerExecutionGateResponse, String>

#[tauri::command]
pub fn list_runner_execution_gates(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerExecutionGateSummary>, String>

#[tauri::command]
pub fn revoke_runner_execution_gate(
    state: tauri::State<'_, DbState>,
    input: RevokeRunnerExecutionGateInput,
) -> Result<RevokeRunnerExecutionGateResponse, String>
```

注册到 `lib.rs`。

命名必须使用 `gate`，不要使用 `execute`、`run`、`start`、`launch`。函数名里出现 `execution_gate` 可以接受，因为它是对象名，不是动作。

## 九、前端类型和 host 封装

### 9.1 shared 类型

在 `packages/shared/src/types/project-plan.ts` 增加：

```ts
export type CreateRunnerExecutionGateInput = {
  preflight_review_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RevokeRunnerExecutionGateInput = {
  gate_id: string;
  second_confirm: boolean;
  confirm_text: string;
  revoked_reason?: string | null;
};

export type RunnerExecutionGateSummary = {
  id: string;
  project_id: string;
  runner_request_id: string;
  task_id: string;
  preflight_review_id: string;
  preflight_approval_id: string;
  status: string;
  risk_level: string;
  operation_types: string[];
  affected_files: string[];
  blocked_reasons: string[];
  can_execute: boolean;
  stage_boundary_locked: boolean;
  requires_git_checkpoint: boolean;
  requires_second_confirm: boolean;
  revoked_reason: string | null;
  requested_by: string;
  created_at: string;
  updated_at: string;
  revoked_at: string | null;
};

export type CreateRunnerExecutionGateResponse = {
  gate: RunnerExecutionGateSummary;
  side_effects: ProjectPlanSideEffects;
};

export type RevokeRunnerExecutionGateResponse = {
  gate: RunnerExecutionGateSummary;
  side_effects: ProjectPlanSideEffects;
};
```

从 `packages/shared/src/index.ts` 导出。

### 9.2 desktopHost

新增：

```ts
export async function createRunnerExecutionGate(
  input: CreateRunnerExecutionGateInput,
): Promise<CreateRunnerExecutionGateResponse> {
  requireTauri();
  return invoke("create_runner_execution_gate", { input });
}

export async function listRunnerExecutionGates(): Promise<RunnerExecutionGateSummary[]> {
  requireTauri();
  return invoke("list_runner_execution_gates");
}

export async function revokeRunnerExecutionGate(
  input: RevokeRunnerExecutionGateInput,
): Promise<RevokeRunnerExecutionGateResponse> {
  requireTauri();
  return invoke("revoke_runner_execution_gate", { input });
}
```

## 十、前端 UI

建议在 `ProjectPlanPage` 的“执行前审查”卡片下方增加：

```text
执行许可 gate
```

展示规则：

- 只显示当前选中草案 / 当前 execution preview 对应 runner request 的 gate。
- 不显示其他草案的 gate。
- 对每条 preflight review：
  - 如果对应 `runner_preflight` 审批未 approved，显示“需先批准执行前审查”。
  - 如果已 approved 且没有 gate，显示“创建执行许可记录”按钮。
  - 如果已有 gate，显示 gate 状态、阻断原因、`can_execute=false`、`stage_boundary_locked=true`。
  - 如果 gate 未撤销，显示“撤销许可记录”按钮。
- 创建 gate 和撤销 gate 都必须二次确认。

创建 gate 确认文本：

```text
我确认只创建执行许可记录，不执行Runner
```

撤销 gate 确认文本：

```text
我确认撤销执行许可记录
```

前端文案必须明确：

```text
执行许可 gate 仍不会执行 Runner，不会执行命令，不会写文件，不会改 Git。当前 Runner 仍被系统边界锁定。
```

按钮文案不要叫：

```text
执行
运行
开始
应用
写入
提交
```

推荐按钮文案：

```text
创建执行许可记录
撤销许可记录
```

错误提示使用粗粒度文案：

```text
创建执行许可记录失败
撤销执行许可记录失败
```

不要展示后端 raw error、SQL 细节、堆栈或敏感内容。

## 十一、测试要求

### 11.1 Rust 测试

至少新增：

1. `create_gate_requires_second_confirmation`
   - 未勾选二次确认拒绝。
   - 确认文本错误拒绝。

2. `create_gate_rejects_unknown_preflight_review`
   - 不存在的 preflight review 拒绝。

3. `create_gate_requires_approved_preflight_approval`
   - preflight approval 仍是 pending 时拒绝。
   - preflight approval rejected 时拒绝。

4. `create_gate_rejects_non_runner_preflight_approval`
   - approval.target_service 不是 `runner_preflight` 时拒绝。

5. `create_gate_creates_blocked_gate_without_execution_side_effects`
   - 先走 project_plan 审批生成 runner_requests。
   - 创建 preflight review。
   - 批准 preflight approval。
   - 创建 gate。
   - gate.status 为 `blocked_by_stage_boundary`。
   - gate.can_execute 为 false。
   - gate.stage_boundary_locked 为 true。
   - blocked_reasons 包含 `runner_execution_disabled_by_stage_boundary`。
   - 不新增 tasks。
   - 不新增 runner_requests。
   - 不写 runtime_events。
   - 不写 model_calls。
   - 不改变 task.status。
   - 不改变 runner_request.status。

6. `create_gate_is_idempotent_for_same_preflight_review`
   - 重复创建只返回已有 gate。
   - gate 不重复。

7. `create_gate_rejects_polluted_preflight_affected_files`
   - 手动污染 preflight affected_files 为 `["../secret"]`。
   - 创建 gate 被拒绝。

8. `create_gate_rejects_forbidden_operation_type`
   - 手动污染 operation_types 包含 `file_write` 或 `command_execute`。
   - 创建 gate 被拒绝。

9. `create_gate_rejects_changed_runner_request_scope`
   - preflight 创建后手动修改 runner_request operation_types 或 affected_files。
   - 创建 gate 被拒绝。

10. `list_gates_filters_current_project`
    - 不返回其他 project 的 gate。

11. `revoke_gate_requires_second_confirmation`
    - 缺二次确认拒绝。
    - 确认文本错误拒绝。

12. `revoke_gate_marks_only_gate_revoked`
    - gate 状态变为 revoked。
    - can_execute 仍为 false。
    - stage_boundary_locked 仍为 true。
    - 不修改 task / runner_request / preflight / approval。

13. `revoke_gate_is_idempotent`
    - 重复撤销返回已有 revoked gate。

14. `gate_inputs_reject_unknown_fields`
    - 创建输入拒绝未知字段。
    - 撤销输入拒绝未知字段。

15. `gate_rejects_sensitive_requested_by_or_revoked_reason`
    - `requested_by` 或 `revoked_reason` 包含 `sk-...` / `api_key=` 等敏感模式时拒绝。

### 11.2 前端检查

- `ProjectPlanPage` 不出现执行按钮。
- 创建 gate 后刷新 gate 列表。
- 撤销 gate 后刷新 gate 列表。
- 只展示当前选中草案相关 gate，不串其他草案。
- `useCallback` / `useEffect` 依赖完整。
- 未连接 Tauri 时不崩溃。
- 错误提示不展示 raw error。

### 11.3 验证命令

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
rg -n "execute_runner|runner_execute|command_execute|file_write|git_commit|git_push" apps/desktop/src-tauri/src/services/runner_execution_gate.rs packages/ui/src/pages/ProjectPlanPage.tsx
```

最后一条如果命中，必须逐条解释：只允许命中禁止列表、测试污染值或安全文案，不能出现真实执行路径。

## 十二、文档同步

实现完成后必须同步：

```text
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
docs/data-model-draft.md
docs/api-draft.md
```

如果新增验收脚本，再同步：

```text
scripts/README.md
docs/demo-checklist.md
```

同步口径：

```text
阶段 31 完成：已可在 runner_preflight 审批通过后创建 Runner 执行许可 gate；
gate 显式 can_execute=false、stage_boundary_locked=true，并继续被 runner_execution_disabled_by_stage_boundary 阻断；
仍不执行 Runner，不执行命令，不写文件，不改 Git，不调用模型，不创建可执行 Runner job。
```

## 十三、不要做的事

- 不新增真实 Runner。
- 不新增 Runner job 表。
- 不新增命令执行器。
- 不新增文件写入器。
- 不新增 Git checkpoint 执行。
- 不新增网络请求执行。
- 不新增模型调用。
- 不把 gate 自动变成可执行。
- 不让 `approve_approval` 触发 gate 创建。
- 不让 `approve_approval` 触发 Runner 行为。
- 不在 UI 上出现“执行/运行/开始/提交/写入”这类动作按钮。
- 不把真实文件路径写入 `affected_files`。
- 不把 raw error 展示到前端。

## 十四、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 31 完成：Runner 执行许可 gate

改了哪些文件：
- ...

是否新增 migration：
- 是，008_add_runner_execution_gates.sql

核心行为：
- runner_preflight 审批 approved 后，可创建 Runner 执行许可 gate。
- gate.status = blocked_by_stage_boundary。
- gate.can_execute = false。
- gate.stage_boundary_locked = true。
- blocked_reasons 包含 runner_execution_disabled_by_stage_boundary。
- 同一 preflight / runner_request 幂等，不重复创建。
- 支持撤销 gate，撤销不影响 task / runner_request / preflight / approval。
- 不执行 Runner、不执行命令、不写文件、不改 Git、不调用模型、不创建可执行 Runner job。

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
- 执行类关键字扫描

遗留风险：
- 无 / ...
```

最终阶段 31 口径：

```text
阶段 31 完成后：系统具备 Runner 执行许可 gate，可以把已批准的执行前审查转为结构化 gate；
但 gate 仍被阶段边界锁定，can_execute=false，不执行命令，不写用户项目文件，不改 Git，不调用模型，不创建可执行 Runner job。
```
