# 阶段 33：Git checkpoint 和文件范围锁设计

目标：阶段 32 已经可以基于执行许可 gate 生成只读 dry-run 预演。阶段 33 要做的是在真实执行前增加一层“执行范围锁”，把 dry-run 中的计划文件、允许文件、禁止路径、checkpoint 要求和工作区要求落库，作为阶段 34 最小真实 Runner 执行的前置条件。

阶段 33 仍然不是执行阶段。它不执行 Runner，不执行命令，不写用户项目文件，不修改 Git，不创建真实 Git checkpoint，不运行测试，不发网络请求，不调用真实模型，不创建可执行 Runner job。

阶段 33 的产物是可审查的锁定记录：如果未来要执行，只能在这条锁记录允许的范围内执行；如果 dry-run 或 gate 已撤销、路径被污染、命令危险、文件范围为空或越界，就不能创建锁。

## 一、宪法约束

阶段 33 必须遵守：

- 不越过当前阶段边界。
- 不绕过审批写用户项目文件。
- 不执行 Runner。
- 不执行 shell / PowerShell / Git / npm / cargo 命令。
- 不创建 commit、stash、tag、branch、checkpoint 文件或任何真实 Git 状态变更。
- 不写真实项目文件。
- 不读取 raw key、raw prompt、raw response、raw provider error。
- 不调用真实模型。
- 不新增第三方依赖。
- 数据库结构变更必须走 migration。
- 所有新增输入必须 `#[serde(deny_unknown_fields)]`。

结论：阶段 33 只能创建、查看、撤销“执行锁记录”，不能产生真实执行副作用。

## 二、阶段边界

允许做：

- 新增 SQLite migration，保存 runner execution lock。
- 基于未撤销的 `runner_dry_runs` 创建文件范围锁。
- 锁定：
  - `dry_run_id`
  - `gate_id`
  - `runner_request_id`
  - `task_id`
  - `allowed_files`
  - `denied_paths`
  - `planned_commands`
  - `planned_file_changes`
  - `checkpoint_strategy`
  - `workspace_requirements`
  - `blocked_reasons`
  - `can_execute=false`
  - `stage_boundary_locked=true`
- 校验 dry-run 的计划文件全部在允许范围内。
- 校验 denied paths 覆盖保护路径和高危路径。
- 展示“执行前还缺什么”，而不是展示“开始执行”。
- 支持撤销 lock。

禁止做：

- 不执行 Runner。
- 不执行命令。
- 不执行 `git status`、`git diff`、`git commit`、`git stash`、`git reset`、`git clean`。
- 不创建真实 Git checkpoint。
- 不写、改、删任何用户项目文件。
- 不运行测试。
- 不改 `tasks.status`。
- 不改 `runner_requests.status`。
- 不改 `runner_execution_gates.can_execute`。
- 不把任何 lock 状态改成 `ready_to_execute`、`running`、`completed`。
- 不创建可执行 Runner job。
- 不写 `runtime_events`。
- 不写 `model_calls`。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 三、为什么阶段 33 不创建真实 checkpoint

Git checkpoint 本身会修改 Git 状态或至少依赖真实 Git 命令。当前阶段还没有开放命令执行器，也没有开放真实 Runner。阶段 33 如果直接创建 checkpoint，就等于提前打开了 Git 执行能力。

所以阶段 33 只记录 checkpoint 要求：

```text
requires_git_checkpoint = true
checkpoint_strategy = manual_checkpoint_required_before_stage34
workspace_requirements = clean_or_only_allowed_paths_dirty
```

真正创建 Git checkpoint 放到阶段 34 的执行前检查里，并且阶段 34 第一版也不自动 commit / stash / reset，只保存状态摘要和 diff 摘要。

## 四、数据模型

新增 migration：

```text
010_add_runner_execution_locks.sql
```

新增表：

```sql
CREATE TABLE IF NOT EXISTS runner_execution_locks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  dry_run_id TEXT NOT NULL,
  gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('locked', 'revoked')),
  allowed_files TEXT NOT NULL,
  denied_paths TEXT NOT NULL,
  planned_commands TEXT NOT NULL,
  planned_file_changes TEXT NOT NULL,
  checkpoint_strategy TEXT NOT NULL,
  workspace_requirements TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 0 CHECK (can_execute = 0),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 1 CHECK (stage_boundary_locked = 1),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1 CHECK (requires_git_checkpoint = 1),
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  requested_by TEXT NOT NULL,
  revoked_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (dry_run_id) REFERENCES runner_dry_runs(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_project_id
  ON runner_execution_locks(project_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_locks_project_dry_run
  ON runner_execution_locks(project_id, dry_run_id);

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_runner_request
  ON runner_execution_locks(project_id, runner_request_id);

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_status
  ON runner_execution_locks(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_task_id
  ON runner_execution_locks(project_id, task_id);
```

字段说明：

- `dry_run_id`：阶段 32 只读预演记录。
- `status` 第一版只允许 `locked` 和 `revoked`。
- `allowed_files`：从 dry-run 的 `allowed_files` 复制并再次校验。
- `denied_paths`：后端固定生成，必须包含保护路径和危险路径。
- `planned_commands`：从 dry-run 复制，只能是计划字符串，不执行。
- `planned_file_changes`：从 dry-run 复制并校验路径。
- `checkpoint_strategy` 第一版固定为 `manual_checkpoint_required_before_stage34`。
- `workspace_requirements` 第一版固定为 `clean_or_only_allowed_paths_dirty`。
- `blocked_reasons` 必须包含：

```text
runner_execution_disabled_by_stage_boundary
git_checkpoint_not_created_in_stage33
file_scope_locked_for_stage34
```

- `can_execute` 必须恒为 `0`。
- `stage_boundary_locked` 必须恒为 `1`。
- `requires_git_checkpoint` 必须恒为 `1`。

## 五、路径规则

阶段 33 必须把路径校验写死，不允许前端传入任意路径。

### 5.1 allowed_files

规则：

- 必须来自 dry-run 的 `allowed_files`。
- 必须非空。
- 必须是相对路径。
- 必须使用 `/`。
- 不允许 `..`。
- 不允许 `\`。
- 不允许 `:`。
- 不允许 `~`。
- 不允许绝对路径。
- 不允许空白路径。
- 不允许保护路径。
- 不允许 `.git/`。
- 不允许 `node_modules/`。
- 不允许 `target/`、`dist/`、`build/` 等构建产物目录。

阶段 33 仍然可以沿用阶段 32 的 `virtual/...` 路径作为准备锁。如果实现者想在阶段 33 就引入真实沙箱路径，只能允许：

```text
sandbox/
tests/sandbox/
tmp/runner-sandbox/
```

但建议阶段 33 继续保持 `virtual/...`，把真实沙箱路径映射留给阶段 34。

### 5.2 denied_paths

后端固定生成，至少包含：

```text
design/image2/
_internal/
data/mock/runtime-state.json
data/local/
logs/
.playwright-cli/
.git/
node_modules/
target/
dist/
build/
.env
.env.*
```

`denied_paths` 不允许前端传入，也不允许从模型摘要生成。

## 六、后端模块

建议新增：

```text
apps/desktop/src-tauri/src/services/runner_execution_lock.rs
apps/desktop/src-tauri/src/commands/runner_execution_lock.rs
```

注册：

```text
apps/desktop/src-tauri/src/services/mod.rs
apps/desktop/src-tauri/src/commands/mod.rs
apps/desktop/src-tauri/src/lib.rs
```

不要新增第三方依赖。

## 七、后端类型

### 7.1 创建 lock 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerExecutionLockInput {
    pub dry_run_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
```

确认文本固定：

```text
我确认锁定执行范围，不创建Git checkpoint
```

### 7.2 撤销 lock 输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeRunnerExecutionLockInput {
    pub execution_lock_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub revoked_reason: Option<String>,
}
```

确认文本固定：

```text
我确认撤销执行范围锁
```

### 7.3 输出类型

```rust
#[derive(Debug, Serialize)]
pub struct RunnerExecutionLockSummary {
    pub id: String,
    pub project_id: String,
    pub dry_run_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub allowed_files: Vec<String>,
    pub denied_paths: Vec<String>,
    pub planned_commands: Vec<String>,
    pub planned_file_changes: Vec<PlannedFileChangeSummary>,
    pub checkpoint_strategy: String,
    pub workspace_requirements: String,
    pub blocked_reasons: Vec<String>,
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
pub struct CreateRunnerExecutionLockResponse {
    pub execution_lock: RunnerExecutionLockSummary,
    pub side_effects: ProjectPlanSideEffects,
}

#[derive(Debug, Serialize)]
pub struct RevokeRunnerExecutionLockResponse {
    pub execution_lock: RunnerExecutionLockSummary,
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
pub fn create_runner_execution_lock(
    connection: &mut Connection,
    input: CreateRunnerExecutionLockInput,
) -> Result<CreateRunnerExecutionLockResponse, String>

pub fn list_runner_execution_locks(
    connection: &Connection,
) -> Result<Vec<RunnerExecutionLockSummary>, String>

pub fn revoke_runner_execution_lock(
    connection: &mut Connection,
    input: RevokeRunnerExecutionLockInput,
) -> Result<RevokeRunnerExecutionLockResponse, String>
```

### 8.2 创建 lock 步骤

1. 获取当前 project。
2. 校验 `dry_run_id` 非空，长度 1..200。
3. 校验二次确认：
   - `second_confirm == true`
   - `confirm_text == "我确认锁定执行范围，不创建Git checkpoint"`
4. 校验 `requested_by`：
   - 缺省为 `local_user`
   - trim 后长度 1..120
   - 不含 `sk-...`、`Authorization: Bearer ...`、`api_key=`、`token=`、`password=`
5. 读取当前项目下的 dry-run。
6. dry-run 必须存在。
7. dry-run 必须：
   - `status == "blocked_by_stage_boundary"`
   - `can_execute == false`
   - `stage_boundary_locked == true`
   - `requires_git_checkpoint == true`
   - `blocked_reasons` 包含 `runner_execution_disabled_by_stage_boundary`
8. 读取关联 gate：
   - 必须存在
   - 当前项目
   - `status == "blocked_by_stage_boundary"`
   - `can_execute == false`
   - `stage_boundary_locked == true`
9. 读取关联 runner_request：
   - 必须存在
   - 当前项目
   - `status == "queued"`
   - 仍是只读 request
10. 读取关联 task：
   - 必须存在
   - 当前项目
   - 不修改状态
11. 校验 dry-run 的 `planned_commands`：
   - 只能来自阶段 32 后端映射
   - 不得含 `git commit`、`git push`、`git reset`、`git clean`、`Remove-Item`、`rm -rf`、`curl`、`wget`、`ssh`
12. 校验 dry-run 的 `planned_file_changes`：
   - 路径全部在 `allowed_files` 内
   - 不含保护路径
   - 不含路径穿越
13. 生成 `denied_paths`：
   - 使用后端固定列表
   - 不接受前端输入
14. 幂等检查：
   - 同一 project + dry_run_id 已有 lock，则返回已有 lock。
   - 不重复插入。
15. 插入 `runner_execution_locks`：
   - `status = locked`
   - `checkpoint_strategy = manual_checkpoint_required_before_stage34`
   - `workspace_requirements = clean_or_only_allowed_paths_dirty`
   - `can_execute = 0`
   - `stage_boundary_locked = 1`
   - `requires_git_checkpoint = 1`
16. 不修改 dry-run、gate、runner_request、task、approval。
17. 不写 runtime_events。
18. 返回 lock + 全 false side_effects。

### 8.3 撤销 lock 步骤

1. 获取当前 project。
2. 校验 `execution_lock_id`。
3. 校验二次确认：
   - `second_confirm == true`
   - `confirm_text == "我确认撤销执行范围锁"`
4. 校验 `revoked_reason`：
   - 可为空
   - 非空 trim 后长度 1..500
   - 不含敏感值
5. 读取当前项目下的 lock。
6. 如果不存在，返回 `not_found: execution lock not found`。
7. 如果已 revoked，幂等返回已有记录。
8. 更新：
   - `status = revoked`
   - `revoked_reason`
   - `revoked_at`
   - `updated_at`
9. `can_execute` 仍为 false。
10. `stage_boundary_locked` 仍为 true。
11. 不修改 dry-run、gate、runner_request、task、approval。

### 8.4 读取 lock 的安全校验

`list_runner_execution_locks` 返回前必须校验：

- `can_execute == false`
- `stage_boundary_locked == true`
- `requires_git_checkpoint == true`
- status 只能是 `locked` 或 `revoked`
- `allowed_files` 非空且安全
- `denied_paths` 包含保护路径
- `planned_commands` 不含危险命令

如果数据库被污染，返回 `invalid_state`，不要把污染数据展示到前端。

## 九、Tauri commands

新增：

```rust
#[tauri::command]
pub fn create_runner_execution_lock(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerExecutionLockInput,
) -> Result<CreateRunnerExecutionLockResponse, String>

#[tauri::command]
pub fn list_runner_execution_locks(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerExecutionLockSummary>, String>

#[tauri::command]
pub fn revoke_runner_execution_lock(
    state: tauri::State<'_, DbState>,
    input: RevokeRunnerExecutionLockInput,
) -> Result<RevokeRunnerExecutionLockResponse, String>
```

命名可以使用 `lock`，不要使用 `execute`、`run`、`start`、`launch` 作为动作名。

## 十、前端类型和 host 封装

### 10.1 shared 类型

在 `packages/shared/src/types/project-plan.ts` 增加：

```ts
export type CreateRunnerExecutionLockInput = {
  dry_run_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RevokeRunnerExecutionLockInput = {
  execution_lock_id: string;
  second_confirm: boolean;
  confirm_text: string;
  revoked_reason?: string | null;
};

export type RunnerExecutionLockSummary = {
  id: string;
  project_id: string;
  dry_run_id: string;
  gate_id: string;
  runner_request_id: string;
  task_id: string;
  status: string;
  allowed_files: string[];
  denied_paths: string[];
  planned_commands: string[];
  planned_file_changes: PlannedFileChangeSummary[];
  checkpoint_strategy: string;
  workspace_requirements: string;
  blocked_reasons: string[];
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

export type CreateRunnerExecutionLockResponse = {
  execution_lock: RunnerExecutionLockSummary;
  side_effects: ProjectPlanSideEffects;
};

export type RevokeRunnerExecutionLockResponse = {
  execution_lock: RunnerExecutionLockSummary;
  side_effects: ProjectPlanSideEffects;
};
```

同步从 `packages/shared/src/index.ts` 导出。

### 10.2 desktopHost

新增：

```ts
export async function createRunnerExecutionLock(
  input: CreateRunnerExecutionLockInput,
): Promise<CreateRunnerExecutionLockResponse> {
  requireTauri();
  return invoke("create_runner_execution_lock", { input });
}

export async function listRunnerExecutionLocks(): Promise<RunnerExecutionLockSummary[]> {
  requireTauri();
  return invoke("list_runner_execution_locks");
}

export async function revokeRunnerExecutionLock(
  input: RevokeRunnerExecutionLockInput,
): Promise<RevokeRunnerExecutionLockResponse> {
  requireTauri();
  return invoke("revoke_runner_execution_lock", { input });
}
```

## 十一、前端 UI

建议在 `ProjectPlanPage` 的 dry-run 卡片下方新增：

```text
执行范围锁
```

展示规则：

- 只展示当前选中草案 / 当前 runner request 对应的 lock。
- dry-run 不存在时不显示创建按钮。
- dry-run 已 revoked 时不显示创建按钮。
- lock 不存在时显示“锁定执行范围”按钮。
- lock 已存在时展示：
  - 状态
  - 允许文件
  - 禁止路径
  - checkpoint 策略
  - 工作区要求
  - 阻断原因
  - `can_execute=false`
  - `stage_boundary_locked=true`
- lock 未撤销时显示“撤销执行范围锁”按钮。

创建确认文本：

```text
我确认锁定执行范围，不创建Git checkpoint
```

撤销确认文本：

```text
我确认撤销执行范围锁
```

按钮文案不要出现：

```text
执行
运行
开始
写入
提交
创建 checkpoint
```

推荐按钮文案：

```text
锁定执行范围
撤销执行范围锁
```

错误提示使用粗粒度文案：

```text
锁定执行范围失败
撤销执行范围锁失败
```

不要展示 raw error、SQL 细节、堆栈或敏感内容。

## 十二、测试要求

### 12.1 Rust 测试

至少新增：

1. `create_execution_lock_requires_second_confirmation`
   - 未勾选二次确认拒绝。
   - 确认文本错误拒绝。
2. `create_execution_lock_rejects_unknown_dry_run`
   - 不存在 dry-run 拒绝。
3. `create_execution_lock_rejects_revoked_dry_run`
   - dry-run revoked 拒绝。
4. `create_execution_lock_requires_locked_non_executable_dry_run`
   - dry-run.can_execute 被污染拒绝或 schema 拒绝。
   - dry-run.stage_boundary_locked 被污染拒绝或 schema 拒绝。
5. `create_execution_lock_creates_scope_lock_without_execution_side_effects`
   - 完整链路 preflight -> gate -> dry-run -> lock。
   - lock.status = locked。
   - can_execute = false。
   - stage_boundary_locked = true。
   - requires_git_checkpoint = true。
   - side_effects 全 false。
   - 不新增 tasks。
   - 不新增 runner_requests。
   - 不写 runtime_events。
   - 不改 task / runner_request / gate / dry-run。
6. `create_execution_lock_is_idempotent_for_same_dry_run`
   - 重复创建返回已有 lock。
7. `create_execution_lock_rejects_empty_allowed_files`
   - dry-run allowed_files 为空拒绝。
8. `create_execution_lock_rejects_polluted_allowed_files`
   - `../secret`、反斜杠、绝对路径、盘符路径拒绝。
9. `create_execution_lock_rejects_protected_paths`
   - 命中保护路径拒绝。
10. `create_execution_lock_rejects_dangerous_planned_commands`
    - `git commit`、`git push`、`Remove-Item`、`curl` 拒绝。
11. `execution_lock_denied_paths_contains_protected_paths`
    - denied_paths 包含全部保护路径。
12. `list_execution_locks_filters_current_project`
    - 项目隔离。
13. `revoke_execution_lock_requires_second_confirmation`
    - 二次确认缺失或文本错误拒绝。
14. `revoke_execution_lock_marks_only_lock_revoked`
    - 只改 lock。
    - 不改 dry-run / gate / runner_request / task / approval。
15. `revoke_execution_lock_is_idempotent`
    - 重复撤销返回已有 revoked lock。
16. `execution_lock_inputs_reject_unknown_fields`
    - 创建和撤销输入拒绝未知字段。
17. `execution_lock_rejects_sensitive_requested_by_or_revoked_reason`
    - 输入含 secret 模式拒绝。
18. `execution_lock_schema_rejects_executable_or_unlocked_pollution`
    - CHECK 拒绝 `can_execute=1`。
    - CHECK 拒绝 `stage_boundary_locked=0`。

### 12.2 前端检查

- 没有执行按钮。
- 没有创建 checkpoint 按钮。
- 只显示锁定范围和撤销范围锁。
- dry-run 不存在时不能创建 lock。
- lock created 后刷新列表。
- lock revoked 后刷新列表。
- 只展示当前选中草案相关 lock。
- Tauri 未连接时不崩溃。
- `useCallback` / `useEffect` 依赖完整。
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
rg -n "git commit|git push|git reset|git clean|Remove-Item|rm -rf|curl|wget|ssh|execute_runner|runner_execute|command_execute|file_write" apps/desktop/src-tauri/src/services/runner_execution_lock.rs packages/ui/src/pages/ProjectPlanPage.tsx
```

最后一条如果命中，必须逐条解释：只允许命中禁止列表、测试污染值或安全扫描说明，不允许出现在真实执行路径。

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
阶段 33 完成：系统已可基于只读 dry-run 创建执行范围锁，锁定 allowed_files、denied_paths、planned_commands、planned_file_changes、checkpoint_strategy 和 workspace_requirements；lock 显式 can_execute=false、stage_boundary_locked=true、requires_git_checkpoint=true。阶段 33 仍不执行 Runner，不执行命令，不写文件，不改 Git，不创建真实 Git checkpoint。
```

## 十四、不要做的事

- 不要创建真实 Git checkpoint。
- 不要执行任何 Git 命令。
- 不要执行 npm / cargo 命令。
- 不要写真实文件。
- 不要把 `can_execute` 改成 true。
- 不要把 `stage_boundary_locked` 改成 false。
- 不要新增完整 Runner。
- 不要新增命令执行器。
- 不要新增文件写入器。
- 不要写 runtime_events。
- 不要改 task / runner_request 状态。
- 不要把前端按钮写成“执行”“运行”“开始”。

## 十五、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 33 完成：Git checkpoint 要求和文件范围锁

改了哪些文件：
- ...

是否新增 migration：
- 是，010_add_runner_execution_locks.sql

核心行为：
- 可基于未撤销 dry-run 创建 runner_execution_lock。
- allowed_files 来自 dry-run，并再次校验。
- denied_paths 由后端固定生成，包含保护路径。
- checkpoint_strategy = manual_checkpoint_required_before_stage34。
- workspace_requirements = clean_or_only_allowed_paths_dirty。
- can_execute = false。
- stage_boundary_locked = true。
- requires_git_checkpoint = true。
- 支持撤销 lock。
- 不执行 Runner、不执行命令、不写文件、不改 Git、不创建真实 checkpoint。

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
- 执行类关键词扫描

遗留风险：
- 无 / ...
```

最终阶段 33 口径：

```text
阶段 33 完成后：系统具备真实执行前的范围锁和 checkpoint 要求记录，但仍不执行 Runner、不执行命令、不写文件、不改 Git、不创建真实 checkpoint。下一步阶段 34 才能在人工确认后开放极小范围真实执行。
```
