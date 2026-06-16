# 阶段 32：Runner 执行沙箱和只读 dry-run 设计

目标：阶段 31 已经可以在 `runner_preflight` 审批通过后创建 Runner 执行许可 gate。阶段 32 要做的是基于 gate 生成只读 dry-run 预演记录，让用户看到“如果未来执行，Runner 会准备做什么”，包括计划操作、命令清单、影响文件清单、允许文件范围和阻断原因。

这一阶段仍然不执行 Runner，不执行命令，不写用户项目文件，不改 Git，不创建 Git checkpoint，不发网络请求，不调用真实模型，不创建可执行 Runner job。

阶段 32 的 dry-run 不是执行，不是队列，不是 checkpoint，也不是 diff。它只是结构化预演记录。

## 一、宪法约束

阶段 32 必须遵守 `docs/Agent宪法.md` 和 `docs/AI开发细则.md`：

- 不越过当前阶段边界。
- 不绕过审批写用户项目文件。
- Runner 不得自动执行命令、写文件、删文件、发网络请求或修改 Git。
- 写入、变更、Runner、Git checkpoint、模型相关能力必须进入审批链，不得直接开放。
- 数据库结构变更必须走 migration。
- 新增依赖必须先说明理由并获批准；阶段 32 不需要新增依赖。

结论：阶段 32 不是 Runner 执行阶段。它只允许创建和查看 dry-run 预演记录，不允许执行任何动作。

## 二、阶段边界

允许做：

- 新增 SQLite migration，保存 Runner dry-run 预演记录。
- 对一条已存在且未撤销的 `runner_execution_gates` 创建 dry-run。
- dry-run 记录保存：
  - 关联 gate。
  - 关联 runner_request。
  - 关联 task。
  - 计划操作。
  - 计划命令。
  - 计划文件变化。
  - 允许文件。
  - 阻断原因。
  - 安全说明。
  - `can_execute = 0`。
  - `stage_boundary_locked = 1`。
- 前端展示 dry-run 预演，不出现执行按钮。
- 增加测试证明没有 Runner、命令、文件、Git、模型、网络副作用。

禁止做：

- 不执行 Runner。
- 不执行命令。
- 不写用户项目文件。
- 不创建、修改、删除真实项目文件。
- 不创建 Git checkpoint。
- 不执行任何 Git 命令。
- 不运行测试命令。
- 不发网络请求。
- 不调用真实模型。
- 不读取 raw key。
- 不创建可执行 Runner job。
- 不新增真实执行队列表。
- 不把 `runner_requests.status` 改为 `running` / `completed` / `approved`。
- 不把 `tasks.status` 改为 `running` / `completed`。
- 不把 `runner_execution_gates.can_execute` 改成 true。
- 不把 dry-run 状态改成 `running` / `completed`。
- 不改 `approve_approval` 让通用审批触发 dry-run 或 Runner 行为。
- 不展示 raw prompt / raw provider response / raw error / raw secret。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 三、为什么 dry-run 仍然只读

阶段 31 的 gate 只是“可以进入执行准备”的结构化记录，但 gate 本身仍被边界锁定。阶段 32 要解决的是：

- 未来执行会涉及哪些操作。
- 未来可能需要哪些命令。
- 未来可能影响哪些文件。
- 哪些文件被允许进入后续锁定范围。
- 当前为什么仍不能执行。

这些信息必须先可见、可审查，再进入阶段 33 的 Git checkpoint 和文件范围锁。直接跳到写文件会同时打开命令执行、文件写入、Git 修改和回滚风险，容易越过宪法边界。

## 四、数据模型

新增 migration：

```text
009_add_runner_dry_runs.sql
```

新增表：

```sql
CREATE TABLE IF NOT EXISTS runner_dry_runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('blocked_by_stage_boundary', 'revoked')),
  risk_level TEXT NOT NULL,
  planned_operations TEXT NOT NULL,
  planned_commands TEXT NOT NULL,
  planned_file_changes TEXT NOT NULL,
  allowed_files TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  safety_summary TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 0 CHECK (can_execute = 0),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 1 CHECK (stage_boundary_locked = 1),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1,
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  requested_by TEXT NOT NULL,
  revoked_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_project_id
  ON runner_dry_runs(project_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_dry_runs_project_gate
  ON runner_dry_runs(project_id, gate_id);

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_runner_request
  ON runner_dry_runs(project_id, runner_request_id);

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_status
  ON runner_dry_runs(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_task_id
  ON runner_dry_runs(project_id, task_id);
```

字段说明：

- `gate_id`：阶段 31 的执行许可 gate。
- `status` 第一版只允许：

```text
blocked_by_stage_boundary
revoked
```

- 阶段 32 不允许出现：

```text
ready
approved_to_execute
running
completed
```

- `planned_operations`、`planned_commands`、`planned_file_changes`、`allowed_files`、`blocked_reasons` 使用 JSON 数组字符串。
- `can_execute` 第一版必须恒为 `0`。
- `stage_boundary_locked` 第一版必须恒为 `1`。
- `blocked_reasons` 第一版必须包含：

```text
runner_execution_disabled_by_stage_boundary
```

## 五、dry-run 内容定义

### 5.1 planned_operations

来自 gate / runner_request 的 `operation_types`，但必须继续过滤真实执行操作。

禁止值：

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

允许承接只读/计划类操作，例如：

```text
runner_request_readonly
frontend_plan
backend_plan
qa_plan
docs_plan
review_plan
security_review_plan
devops_plan
ux_plan
data_plan
```

以代码中阶段 28 模板实际白名单为准，但不能加入真实执行类。

### 5.2 planned_commands

阶段 32 只保存命令草案，不执行。建议按角色生成固定预演命令：

```text
frontend_plan -> npm run typecheck, npm run build
backend_plan -> cargo fmt --check, cargo check, cargo test
qa_plan -> cargo test, npm run typecheck
docs_plan -> git diff --check
review_plan -> git diff --check, cargo test
security_review_plan -> rg 安全扫描命令
devops_plan -> git status --short
ux_plan -> npm run build
data_plan -> cargo test
```

注意：

- 这些命令只是字符串计划。
- 不调用 shell。
- 不启动进程。
- 不读真实命令输出。
- 不把状态写成执行成功或失败。

命令字符串必须来自后端固定映射，不允许前端传入自定义命令。

### 5.3 planned_file_changes

阶段 32 不生成真实 diff，只生成文件影响草案。

建议结构：

```json
[
  {
    "path": "virtual/frontend-plan.md",
    "change_type": "planned_review",
    "reason": "来自只读 Runner request 的 affected_files"
  }
]
```

规则：

- `path` 必须来自 gate / runner_request 的 `affected_files`。
- `path` 必须仍是 `virtual/...`。
- `change_type` 第一版只允许：

```text
planned_review
planned_validation
planned_documentation
```

- 不能出现真实项目路径。
- 不能出现 `../`、`\`、`:`、`~`。

### 5.4 allowed_files

阶段 32 的 `allowed_files` 仍是虚拟路径集合，不是真实写入白名单。

规则：

- 等于或小于 gate 的 `affected_files`。
- 不允许为空。
- 不允许新增 gate 范围外文件。
- 不允许保护路径。
- 不允许真实绝对路径。

### 5.5 blocked_reasons

第一版必须包含：

```text
runner_execution_disabled_by_stage_boundary
dry_run_only_no_command_execution
dry_run_only_no_file_write
```

如果 gate 已撤销，创建 dry-run 应拒绝，不应写入 blocked 记录。

## 六、后端模块

建议新增：

```text
apps/desktop/src-tauri/src/services/runner_dry_run.rs
apps/desktop/src-tauri/src/commands/runner_dry_run.rs
```

需要注册：

```text
apps/desktop/src-tauri/src/services/mod.rs
apps/desktop/src-tauri/src/commands/mod.rs
apps/desktop/src-tauri/src/lib.rs
```

不要新增第三方依赖。

## 七、后端类型

### 7.1 创建 dry-run 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerDryRunInput {
    pub gate_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
```

确认文本固定：

```text
我确认只生成dry-run预演，不执行Runner
```

### 7.2 撤销 dry-run 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerDryRunInput {
    pub dry_run_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}
```

确认文本固定：

```text
我确认撤销dry-run预演
```

### 7.3 输出类型

```rust
#[derive(Debug, Serialize)]
pub struct PlannedFileChangeSummary {
    pub path: String,
    pub change_type: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct RunnerDryRunSummary {
    pub id: String,
    pub project_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub risk_level: String,
    pub planned_operations: Vec<String>,
    pub planned_commands: Vec<String>,
    pub planned_file_changes: Vec<PlannedFileChangeSummary>,
    pub allowed_files: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub safety_summary: String,
    pub can_execute: bool,
    pub stage_boundary_locked: bool,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub requested_by: String,
    pub revoked_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerDryRunResponse {
    pub dry_run: RunnerDryRunSummary,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct RevokeRunnerDryRunResponse {
    pub dry_run: RunnerDryRunSummary,
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

## 八、后端服务行为

### 8.1 新增函数

```rust
pub fn create_runner_dry_run(
    connection: &mut Connection,
    input: CreateRunnerDryRunInput,
) -> Result<CreateRunnerDryRunResponse, String>

pub fn list_runner_dry_runs(
    connection: &Connection,
) -> Result<Vec<RunnerDryRunSummary>, String>

pub fn revoke_runner_dry_run(
    connection: &mut Connection,
    input: RevokeRunnerDryRunInput,
) -> Result<RevokeRunnerDryRunResponse, String>
```

### 8.2 创建 dry-run 步骤

1. 获取当前项目 id。
2. 校验 `gate_id` 非空，长度 1..200。
3. 校验二次确认：

```text
second_confirm == true
confirm_text == "我确认只生成dry-run预演，不执行Runner"
```

4. 校验 `requested_by`：
   - 缺省为 `local_user`。
   - trim 后长度 1..120。
   - 不能包含 `sk-...`、`Authorization: Bearer ...`、`api_key=`、`token=`、`password=`。
5. 读取当前项目下的 `runner_execution_gates`。
6. 如果不存在，返回：

```text
not_found: execution gate not found
```

7. 校验 gate：
   - `project_id` 必须等于当前项目。
   - `status == "blocked_by_stage_boundary"`。
   - `can_execute == false`。
   - `stage_boundary_locked == true`。
   - `blocked_reasons` 必须包含 `runner_execution_disabled_by_stage_boundary`。
   - `operation_types` 不得包含真实执行操作。
   - `affected_files` 全部必须是安全虚拟路径。
8. 如果 gate 已 revoked，拒绝创建 dry-run。
9. 读取 gate 关联 runner_request：
   - 必须存在。
   - 必须属于当前项目。
   - `status == "queued"`。
   - `task_id` 必须等于 gate.task_id。
   - `operation_types` 必须包含 `runner_request_readonly`。
   - `operation_types` 必须与 gate 记录一致。
   - `affected_files` 必须与 gate 记录一致。
10. 读取关联 task：
   - 必须存在。
   - 必须属于当前项目。
   - 不修改 task 状态。
11. 幂等检查：
   - 如果当前项目下同一 `gate_id` 已有 dry-run，返回已有 dry-run。
   - 不重复插入。
12. 生成 `planned_operations`：
   - 从 gate.operation_types 复制。
   - 拒绝真实执行类型。
13. 生成 `planned_commands`：
   - 只用后端固定映射。
   - 不能使用前端传入命令。
14. 生成 `planned_file_changes`：
   - 基于 gate.affected_files。
   - 每个文件生成 `planned_review` 或按 role 生成固定 `change_type`。
15. 生成 `allowed_files`：
   - 等于 gate.affected_files。
   - 必须非空。
16. 插入 `runner_dry_runs`：
   - `id = runner_dry_run_{gate_id}`，必要时做安全字符归一。
   - `status = blocked_by_stage_boundary`
   - `can_execute = 0`
   - `stage_boundary_locked = 1`
   - `blocked_reasons = ["runner_execution_disabled_by_stage_boundary","dry_run_only_no_command_execution","dry_run_only_no_file_write"]`
   - `requires_git_checkpoint = 1`
   - `requires_second_confirm = 1`
17. 返回 dry-run + 全 false side_effects。

### 8.3 撤销 dry-run 步骤

1. 获取当前项目 id。
2. 校验 `dry_run_id` 非空，长度 1..200。
3. 校验二次确认：

```text
second_confirm == true
confirm_text == "我确认撤销dry-run预演"
```

4. 校验 `revoked_reason`：
   - 可为空。
   - 非空 trim 后长度 1..500。
   - 不能包含敏感值。
5. 读取当前项目下 dry-run。
6. 如果不存在，返回：

```text
not_found: dry-run not found
```

7. 如果 `status == "revoked"`，幂等返回已有 dry-run。
8. 更新 dry-run：
   - `status = revoked`
   - `revoked_reason = normalized_reason`
   - `revoked_at = now`
   - `updated_at = now`
   - `can_execute` 仍为 `0`
   - `stage_boundary_locked` 仍为 `1`
9. 不修改 gate、runner_request、task、approval、preflight。
10. 返回 dry-run + 全 false side_effects。

### 8.4 读取 dry-run 的安全校验

`list_runner_dry_runs` 和返回单条 dry-run 前，都必须检查：

- `can_execute == false`
- `stage_boundary_locked == true`
- status 只允许 `blocked_by_stage_boundary` 或 `revoked`
- blocked_reasons 包含 `runner_execution_disabled_by_stage_boundary`
- planned_commands 不能包含真实危险命令：

```text
git push
git reset
git clean
rm
del
Remove-Item
curl
wget
ssh
```

如果数据库被污染，返回 `invalid_state`，不要把污染数据当正常记录给前端。

### 8.5 不要改这些路径

- 不改 `approve_approval` 的语义。
- 不改 `approve_project_plan` 的语义。
- 不改 `runner_execution_gates`。
- 不改 `runner_preflight_reviews`。
- 不改 `runner_requests`。
- 不改 `tasks`。
- 不写 `runtime_events`。
- 不写 `model_calls`。
- 不创建新的 runner request。

## 九、Tauri commands

新增：

```rust
#[tauri::command]
pub fn create_runner_dry_run(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerDryRunInput,
) -> Result<CreateRunnerDryRunResponse, String>

#[tauri::command]
pub fn list_runner_dry_runs(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerDryRunSummary>, String>

#[tauri::command]
pub fn revoke_runner_dry_run(
    state: tauri::State<'_, DbState>,
    input: RevokeRunnerDryRunInput,
) -> Result<RevokeRunnerDryRunResponse, String>
```

注册到 `lib.rs`。

命名必须使用 `dry_run` 或 `preview`，不要使用 `execute`、`run`、`start`、`launch` 作为动作名。`dry_run` 是对象名，可以保留。

## 十、前端类型和 host 封装

### 10.1 shared 类型

在 `packages/shared/src/types/project-plan.ts` 增加：

```ts
export type CreateRunnerDryRunInput = {
  gate_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RevokeRunnerDryRunInput = {
  dry_run_id: string;
  second_confirm: boolean;
  confirm_text: string;
  revoked_reason?: string | null;
};

export type PlannedFileChangeSummary = {
  path: string;
  change_type: string;
  reason: string;
};

export type RunnerDryRunSummary = {
  id: string;
  project_id: string;
  gate_id: string;
  runner_request_id: string;
  task_id: string;
  status: string;
  risk_level: string;
  planned_operations: string[];
  planned_commands: string[];
  planned_file_changes: PlannedFileChangeSummary[];
  allowed_files: string[];
  blocked_reasons: string[];
  safety_summary: string;
  can_execute: boolean;
  stage_boundary_locked: boolean;
  requires_git_checkpoint: boolean;
  requires_second_confirm: boolean;
  requested_by: string;
  revoked_reason: string | null;
  created_at: string;
  updated_at: string;
  revoked_at: string | null;
};

export type CreateRunnerDryRunResponse = {
  dry_run: RunnerDryRunSummary;
  side_effects: ProjectPlanSideEffects;
};

export type RevokeRunnerDryRunResponse = {
  dry_run: RunnerDryRunSummary;
  side_effects: ProjectPlanSideEffects;
};
```

从 `packages/shared/src/index.ts` 导出。

### 10.2 desktopHost

新增：

```ts
export async function createRunnerDryRun(
  input: CreateRunnerDryRunInput,
): Promise<CreateRunnerDryRunResponse> {
  requireTauri();
  return invoke("create_runner_dry_run", { input });
}

export async function listRunnerDryRuns(): Promise<RunnerDryRunSummary[]> {
  requireTauri();
  return invoke("list_runner_dry_runs");
}

export async function revokeRunnerDryRun(
  input: RevokeRunnerDryRunInput,
): Promise<RevokeRunnerDryRunResponse> {
  requireTauri();
  return invoke("revoke_runner_dry_run", { input });
}
```

## 十一、前端 UI

建议在 `ProjectPlanPage` 的“执行许可 gate”卡片下方增加：

```text
只读 dry-run 预演
```

展示规则：

- 只显示当前选中草案 / 当前 execution preview 对应 runner request 的 dry-run。
- 不显示其他草案的 dry-run。
- 对每条 gate：
  - 如果 gate 已 revoked，显示“gate 已撤销，不能生成 dry-run”。
  - 如果 gate 存在且没有 dry-run，显示“生成 dry-run 预演”按钮。
  - 如果已有 dry-run，显示 dry-run 状态、计划命令、影响文件、允许文件、阻断原因、`can_execute=false`、`stage_boundary_locked=true`。
  - 如果 dry-run 未撤销，显示“撤销 dry-run 预演”按钮。
- 创建 dry-run 和撤销 dry-run 都必须二次确认。

创建 dry-run 确认文本：

```text
我确认只生成dry-run预演，不执行Runner
```

撤销 dry-run 确认文本：

```text
我确认撤销dry-run预演
```

前端文案必须明确：

```text
dry-run 预演只生成计划、命令清单和影响文件清单；不会执行 Runner，不会执行命令，不会写文件，不会改 Git。
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
生成 dry-run 预演
撤销 dry-run 预演
```

错误提示使用粗粒度文案：

```text
生成 dry-run 预演失败
撤销 dry-run 预演失败
```

不要展示后端 raw error、SQL 细节、堆栈或敏感内容。

## 十二、测试要求

### 12.1 Rust 测试

至少新增：

1. `create_dry_run_requires_second_confirmation`
   - 未勾选二次确认拒绝。
   - 确认文本错误拒绝。

2. `create_dry_run_rejects_unknown_gate`
   - 不存在的 gate 拒绝。

3. `create_dry_run_rejects_revoked_gate`
   - gate revoked 时拒绝。

4. `create_dry_run_requires_locked_non_executable_gate`
   - gate.can_execute 被污染时拒绝或 schema 拒绝污染。
   - gate.stage_boundary_locked 被污染时拒绝或 schema 拒绝污染。

5. `create_dry_run_creates_blocked_preview_without_execution_side_effects`
   - 先走 project_plan -> preflight -> preflight approval -> gate。
   - 创建 dry-run。
   - dry_run.status 为 `blocked_by_stage_boundary`。
   - dry_run.can_execute 为 false。
   - dry_run.stage_boundary_locked 为 true。
   - blocked_reasons 包含三个固定阻断原因。
   - planned_commands 非空。
   - planned_file_changes 非空。
   - allowed_files 非空。
   - 不新增 tasks。
   - 不新增 runner_requests。
   - 不写 runtime_events。
   - 不写 model_calls。
   - 不改变 task.status。
   - 不改变 runner_request.status。
   - 不改变 gate.status。

6. `create_dry_run_is_idempotent_for_same_gate`
   - 重复创建只返回已有 dry-run。
   - dry-run 不重复。

7. `create_dry_run_rejects_polluted_gate_affected_files`
   - 手动污染 gate affected_files 为 `["../secret"]`。
   - 创建 dry-run 被拒绝。

8. `create_dry_run_rejects_forbidden_operation_type`
   - 手动污染 operation_types 包含 `file_write` 或 `command_execute`。
   - 创建 dry-run 被拒绝。

9. `create_dry_run_rejects_changed_runner_request_scope`
   - gate 创建后手动修改 runner_request operation_types 或 affected_files。
   - 创建 dry-run 被拒绝。

10. `dry_run_uses_backend_command_mapping_only`
    - 前端输入没有 command 字段。
    - command 只由后端映射产生。

11. `list_dry_runs_filters_current_project`
    - 不返回其他 project 的 dry-run。

12. `revoke_dry_run_requires_second_confirmation`
    - 缺二次确认拒绝。
    - 确认文本错误拒绝。

13. `revoke_dry_run_marks_only_dry_run_revoked`
    - dry-run 状态变为 revoked。
    - can_execute 仍为 false。
    - stage_boundary_locked 仍为 true。
    - 不修改 task / runner_request / gate / preflight / approval。

14. `revoke_dry_run_is_idempotent`
    - 重复撤销返回已有 revoked dry-run。

15. `dry_run_inputs_reject_unknown_fields`
    - 创建输入拒绝未知字段。
    - 撤销输入拒绝未知字段。

16. `dry_run_rejects_sensitive_requested_by_or_revoked_reason`
    - `requested_by` 或 `revoked_reason` 包含 `sk-...` / `api_key=` 等敏感模式时拒绝。

17. `dry_run_schema_rejects_executable_or_unlocked_pollution`
    - 数据库 CHECK 拒绝 `can_execute = 1`。
    - 数据库 CHECK 拒绝 `stage_boundary_locked = 0`。

18. `list_dry_runs_rejects_polluted_dangerous_command`
    - 手动污染 planned_commands 包含 `git push` 或 `Remove-Item`。
    - list 返回 invalid_state。

### 12.2 前端检查

- `ProjectPlanPage` 不出现执行按钮。
- 生成 dry-run 后刷新 dry-run 列表。
- 撤销 dry-run 后刷新 dry-run 列表。
- 只展示当前选中草案相关 dry-run，不串其他草案。
- gate revoked 时不能创建 dry-run。
- `useCallback` / `useEffect` 依赖完整。
- 未连接 Tauri 时不崩溃。
- 错误提示不展示 raw error。

### 12.3 验证命令

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
rg -n "execute_runner|runner_execute|command_execute|file_write|git_commit|git_push|Remove-Item|rm -rf|curl|wget|ssh" apps/desktop/src-tauri/src/services/runner_dry_run.rs packages/ui/src/pages/ProjectPlanPage.tsx
```

最后一条如果命中，必须逐条解释：只允许命中禁止列表、测试污染值、安全扫描命令或说明文案，不能出现真实执行路径。

## 十三、文档同步

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
阶段 32 完成：已可基于 Runner 执行许可 gate 创建只读 dry-run 预演；
dry-run 只保存计划操作、计划命令、影响文件、允许文件和阻断原因；
dry-run 显式 can_execute=false、stage_boundary_locked=true；
仍不执行 Runner，不执行命令，不写文件，不改 Git，不调用模型，不创建可执行 Runner job。
```

## 十四、不要做的事

- 不新增真实 Runner。
- 不新增 Runner job 表。
- 不新增命令执行器。
- 不新增文件写入器。
- 不新增 Git checkpoint 执行。
- 不新增网络请求执行。
- 不新增模型调用。
- 不把 dry-run 自动变成可执行。
- 不让 `approve_approval` 触发 dry-run 创建。
- 不让 `approve_approval` 触发 Runner 行为。
- 不在 UI 上出现“执行/运行/开始/提交/写入”这类动作按钮。
- 不把真实文件路径写入 `allowed_files` 或 `planned_file_changes`。
- 不把 raw error 展示到前端。

## 十五、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 32 完成：Runner 执行沙箱和只读 dry-run

改了哪些文件：
- ...

是否新增 migration：
- 是，009_add_runner_dry_runs.sql

核心行为：
- 可基于未撤销的 runner_execution_gate 创建只读 dry-run 预演。
- dry_run.status = blocked_by_stage_boundary。
- dry_run.can_execute = false。
- dry_run.stage_boundary_locked = true。
- blocked_reasons 包含 runner_execution_disabled_by_stage_boundary、dry_run_only_no_command_execution、dry_run_only_no_file_write。
- planned_commands 由后端固定映射生成，不来自前端。
- planned_file_changes 和 allowed_files 只使用 virtual/... 路径。
- 同一 gate 幂等，不重复创建。
- 支持撤销 dry-run，撤销不影响 task / runner_request / gate / preflight / approval。
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

最终阶段 32 口径：

```text
阶段 32 完成后：系统具备 Runner 执行沙箱和只读 dry-run 预演，可以把 gate 转成可审查的执行计划草案；
但 dry-run 仍被阶段边界锁定，can_execute=false，不执行命令，不写用户项目文件，不改 Git，不调用模型，不创建可执行 Runner job。
```
