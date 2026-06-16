# 阶段 34：最小真实 Runner 执行设计

目标：阶段 34 是第一次允许真实 Runner 在极小范围内执行。它只能在阶段 31 gate、阶段 32 dry-run、阶段 33 execution lock 全部通过后，由用户手动二次确认，针对一个 task / 一个 runner_request / 一个 execution lock 创建一条最小执行记录，并在严格白名单内执行。

阶段 34 不是完整 Runner，不是自动开发，不是批量任务执行，不是自由 shell。第一版只允许一个执行批次、一个任务、一个 request、一个锁定文件范围，并且只允许沙箱或测试沙箱路径。

阶段 34 写代码前必须再次获得用户明确确认，因为它会从“只读准备”进入“有限真实执行”。

## 一、进入条件

阶段 34 只有在以下条件全部满足后才允许实现：

- 阶段 31 已完成：`runner_execution_gates` 可创建，且历史 gate 审查无问题。
- 阶段 32 已完成：`runner_dry_runs` 可创建、查看、撤销，且仍无执行副作用。
- 阶段 33 已完成：`runner_execution_locks` 可创建、查看、撤销，且 dry-run 文件范围已经被锁定。
- 阶段 33 的 lock 未撤销。
- lock 显式要求 checkpoint。
- lock 文件范围只包含阶段 34 允许写入的沙箱路径。
- 用户明确说“开始实现阶段 34”或同等意思。

如果用户只是问“能不能落地”“下一步是什么”“写方案”，不得实现阶段 34 执行器。

## 二、宪法约束

阶段 34 虽然允许极小真实执行，但仍必须遵守：

- 不绕过审批链。
- 不自动执行下一条任务。
- 不批量执行。
- 不写保护路径。
- 不执行任意 shell。
- 不执行网络请求。
- 不调用真实模型。
- 不读取 raw key / raw prompt / raw response / raw provider error。
- 不自动 Git commit / push / reset / clean。
- 不删除文件。
- 不把执行结果自动提交。
- 不创建长期后台 Runner。
- 不新增第三方依赖，除非先说明理由并获批准。

## 三、第一版范围

第一版只允许：

```text
1 个 task
1 个 runner_request
1 个 runner_execution_lock
1 个 execution batch
1 次人工二次确认
只写 sandbox/ 或 tests/sandbox/ 明确白名单路径
只执行固定白名单命令
失败立即停止
执行后只展示 diff 摘要，不自动提交
```

第一版不允许：

```text
多个任务连续执行
自动挑选下一个 request
真实项目源码任意路径写入
删除文件
移动文件
改 Git 历史
git commit
git push
git reset
git clean
任意 shell
网络访问
模型调用
后台常驻 Runner
```

## 四、允许写入路径

阶段 34 第一版只能允许以下相对路径前缀：

```text
sandbox/
tests/sandbox/
tmp/runner-sandbox/
```

实现时建议先选更保守的单一路径：

```text
sandbox/runner-stage34/
```

写入路径规则：

- 必须是相对路径。
- 必须使用 `/`。
- 不允许 `..`。
- 不允许 `\`。
- 不允许 `:`。
- 不允许 `~`。
- 不允许绝对路径。
- 不允许空路径。
- 不允许目录本身作为文件写入目标。
- 不允许命中保护路径。
- 不允许写入 `.git/`、`node_modules/`、`target/`、`dist/`、`build/`、`.env`、`.env.*`。

阶段 34 不允许写入真实业务源码路径，例如：

```text
apps/
packages/
services/
docs/
dev-docs/
data/
scripts/
```

如果低级智能体想扩大范围，必须停下让用户确认，不能自行扩大。

## 五、允许命令

阶段 34 第一版允许命令必须来自后端固定白名单，不允许前端传入命令。

只允许以下只读或验证类命令：

```text
git status --short
git diff --stat
npm run typecheck
npm run build
cargo fmt --check
cargo check
cargo test
```

注意：

- `git status --short` 和 `git diff --stat` 只允许读取状态，不允许修改 Git。
- `npm run build` 可能写 `dist/`，因此第一版实现时可以先只允许它作为可选验证命令，默认不开；如果开启，必须确认构建产物不会被误认为 Runner 写入成果。
- `cargo test` 可能产生 `target/` 构建产物，第一版应只把它当验证副作用，不纳入用户项目变更结果。
- 不允许任何带参数的自由拼接命令。
- 不允许通过 `cmd /c`、`powershell -Command`、`bash -lc` 包装执行。

明确禁止：

```text
git commit
git push
git pull
git fetch
git reset
git clean
git checkout
git switch
git branch
git tag
git stash
rm
del
Remove-Item
mv
Move-Item
cp
Copy-Item
curl
wget
ssh
scp
python
node -e
npm install
cargo add
```

## 六、checkpoint 策略

阶段 34 必须有执行前回退策略。第一版建议采用“记录 Git 状态要求 + 执行前状态快照 + 执行后 diff 摘要”，不自动 commit。

实现要求：

- 执行前读取并保存 `git status --short` 输出摘要。
- 执行前读取并保存 `git diff --stat` 输出摘要。
- 如果工作区存在非允许路径的未提交改动，拒绝执行。
- 如果 lock 允许路径以外已有脏改动，拒绝执行。
- 执行后读取并保存 `git status --short` 和 `git diff --stat` 摘要。
- 执行后校验改动文件集合必须是 lock.allowed_files 的子集。
- 如果出现范围外改动，立即标记 `failed_scope_violation`，提示用户人工处理，不自动 reset。

不允许：

- 不自动 commit。
- 不自动 stash。
- 不自动 reset。
- 不自动 clean。
- 不自动回滚删除用户文件。

原因：自动回滚本身也是高风险 Git / 文件操作，第一版不要打开。

## 七、数据模型

新增 migration：

```text
011_add_runner_minimal_runs.sql
```

新增表：

```sql
CREATE TABLE IF NOT EXISTS runner_minimal_runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  execution_lock_id TEXT NOT NULL,
  dry_run_id TEXT NOT NULL,
  gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN (
    'created',
    'running',
    'succeeded',
    'failed',
    'failed_scope_violation',
    'aborted'
  )),
  allowed_files TEXT NOT NULL,
  written_files TEXT NOT NULL,
  command_plan TEXT NOT NULL,
  command_results TEXT NOT NULL,
  pre_git_status_summary TEXT NOT NULL,
  pre_git_diff_stat TEXT NOT NULL,
  post_git_status_summary TEXT,
  post_git_diff_stat TEXT,
  failure_category TEXT,
  failure_summary TEXT,
  side_effects TEXT NOT NULL,
  second_confirmed INTEGER NOT NULL DEFAULT 1 CHECK (second_confirmed = 1),
  requested_by TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (execution_lock_id) REFERENCES runner_execution_locks(id),
  FOREIGN KEY (dry_run_id) REFERENCES runner_dry_runs(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_project_id
  ON runner_minimal_runs(project_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_minimal_runs_project_lock
  ON runner_minimal_runs(project_id, execution_lock_id);

CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_runner_request
  ON runner_minimal_runs(project_id, runner_request_id);

CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_status
  ON runner_minimal_runs(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_task_id
  ON runner_minimal_runs(project_id, task_id);
```

字段说明：

- `execution_lock_id`：阶段 33 锁记录。
- `written_files`：实际写入文件列表，JSON 数组字符串。
- `command_plan`：后端白名单生成的命令计划。
- `command_results`：命令结果摘要，不能保存完整超长 stdout / stderr。
- `pre_git_status_summary` / `post_git_status_summary`：截断后的 Git 状态摘要。
- `side_effects`：必须明确记录本次确实可能写沙箱文件、可能执行白名单命令，但不改 Git、不调用模型、不发网络。

## 八、后端模块

建议新增：

```text
apps/desktop/src-tauri/src/services/runner_minimal_run.rs
apps/desktop/src-tauri/src/commands/runner_minimal_run.rs
```

注册：

```text
apps/desktop/src-tauri/src/services/mod.rs
apps/desktop/src-tauri/src/commands/mod.rs
apps/desktop/src-tauri/src/lib.rs
```

不要新增第三方依赖。命令执行如果必须实现，使用 Rust 标准库 `std::process::Command`，并且只能调用内部白名单映射。

## 九、后端类型

### 9.1 创建最小执行输入

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerMinimalRunInput {
    pub execution_lock_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
```

确认文本固定：

```text
我确认执行阶段34最小Runner，只允许沙箱范围
```

### 9.2 输出类型

```rust
#[derive(Debug, Serialize)]
pub struct RunnerCommandResultSummary {
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout_summary: String,
    pub stderr_summary: String,
}

#[derive(Debug, Serialize)]
pub struct RunnerMinimalRunSummary {
    pub id: String,
    pub project_id: String,
    pub execution_lock_id: String,
    pub dry_run_id: String,
    pub gate_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub status: String,
    pub allowed_files: Vec<String>,
    pub written_files: Vec<String>,
    pub command_plan: Vec<String>,
    pub command_results: Vec<RunnerCommandResultSummary>,
    pub pre_git_status_summary: String,
    pub pre_git_diff_stat: String,
    pub post_git_status_summary: Option<String>,
    pub post_git_diff_stat: Option<String>,
    pub failure_category: Option<String>,
    pub failure_summary: Option<String>,
    pub side_effects: ProjectPlanSideEffects,
    pub requested_by: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerMinimalRunResponse {
    pub run: RunnerMinimalRunSummary,
}
```

阶段 34 的 `side_effects` 不再全 false。必须准确表达：

```text
writes_project_files=true  // 但只能写沙箱白名单路径
modifies_git=false
executes_runner=true
calls_real_model=false
reads_raw_secrets=false
makes_network_requests=false
triggers_agents=false
creates_tasks=false
creates_runner_requests=false
```

## 十、后端服务行为

### 10.1 新增函数

```rust
pub fn create_runner_minimal_run(
    connection: &mut Connection,
    input: CreateRunnerMinimalRunInput,
) -> Result<CreateRunnerMinimalRunResponse, String>

pub fn list_runner_minimal_runs(
    connection: &Connection,
) -> Result<Vec<RunnerMinimalRunSummary>, String>
```

### 10.2 创建最小执行步骤

1. 获取当前 project。
2. 校验 `execution_lock_id` 非空，长度 1..200。
3. 校验二次确认：
   - `second_confirm == true`
   - `confirm_text == "我确认执行阶段34最小Runner，只允许沙箱范围"`
4. 校验 `requested_by`：
   - 缺省为 `local_user`
   - 长度 1..120
   - 不含敏感值模式
5. 读取当前项目下的 `runner_execution_locks`。
6. lock 必须存在。
7. lock 必须：
   - `status == "locked"`
   - `can_execute == false`
   - `stage_boundary_locked == true`
   - `requires_git_checkpoint == true`
   - `checkpoint_strategy == "manual_checkpoint_required_before_stage34"`
8. 读取关联 dry-run：
   - 必须存在
   - `status == "blocked_by_stage_boundary"`
   - `can_execute == false`
   - `stage_boundary_locked == true`
9. 读取 gate：
   - 必须存在
   - 未 revoked
   - `stage_boundary_locked == true`
10. 读取 runner_request：
   - 必须存在
   - `status == "queued"`
   - `operation_types` 仍只含允许值
11. 读取 task：
   - 必须存在
   - 不自动改状态，除非后续单独批准任务状态流转。
12. 校验 allowed_files：
   - 非空
   - 必须全部在阶段 34 沙箱前缀内
   - 必须是 lock.allowed_files 子集
13. 幂等检查：
   - 同一 project + execution_lock_id 如果已有 run，直接返回已有 run。
   - 不重复执行。
14. 执行前 Git 只读检查：
   - 只允许后端内部调用 `git status --short`
   - 只允许后端内部调用 `git diff --stat`
   - 输出截断后保存
   - 如果发现非允许路径脏改动，拒绝执行
15. 创建 run 记录，状态 `created`。
16. 将 run 状态更新为 `running`。
17. 执行最小写入动作：
   - 第一版建议只生成一个固定沙箱文件，例如 `sandbox/runner-stage34/{run_id}.md`
   - 内容只能来自后端固定模板和已脱敏摘要
   - 不能写 raw prompt / raw model response / raw error / secret
18. 执行白名单验证命令：
   - 第一版建议只执行 `git status --short` 和 `git diff --stat`
   - `npm` / `cargo` 命令可先不开放，或者只作为后续扩展
19. 每个命令都必须：
   - 固定 cwd
   - 固定参数数组
   - 超时
   - 截断 stdout / stderr
   - 失败立即停止
20. 执行后读取 `git status --short` 和 `git diff --stat`。
21. 计算实际变更文件。
22. 如果实际变更文件不是 allowed_files 子集：
   - 状态 `failed_scope_violation`
   - 不自动回滚
   - 返回粗粒度错误
23. 成功时：
   - 状态 `succeeded`
   - 保存 `written_files`
   - 保存命令摘要
24. 不自动修改 task / runner_request 状态。
25. 返回 run summary。

## 十一、Tauri commands

新增：

```rust
#[tauri::command]
pub fn create_runner_minimal_run(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerMinimalRunInput,
) -> Result<CreateRunnerMinimalRunResponse, String>

#[tauri::command]
pub fn list_runner_minimal_runs(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerMinimalRunSummary>, String>
```

命名必须包含 `minimal`，不要使用泛化的 `run_runner`、`execute_runner`、`start_runner`，避免后续误以为完整 Runner 已开放。

## 十二、前端类型和 host 封装

### 12.1 shared 类型

在 `packages/shared/src/types/project-plan.ts` 增加：

```ts
export type CreateRunnerMinimalRunInput = {
  execution_lock_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RunnerCommandResultSummary = {
  command: string;
  status: string;
  exit_code: number | null;
  stdout_summary: string;
  stderr_summary: string;
};

export type RunnerMinimalRunSummary = {
  id: string;
  project_id: string;
  execution_lock_id: string;
  dry_run_id: string;
  gate_id: string;
  runner_request_id: string;
  task_id: string;
  status: string;
  allowed_files: string[];
  written_files: string[];
  command_plan: string[];
  command_results: RunnerCommandResultSummary[];
  pre_git_status_summary: string;
  pre_git_diff_stat: string;
  post_git_status_summary: string | null;
  post_git_diff_stat: string | null;
  failure_category: string | null;
  failure_summary: string | null;
  side_effects: ProjectPlanSideEffects;
  requested_by: string;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
};

export type CreateRunnerMinimalRunResponse = {
  run: RunnerMinimalRunSummary;
};
```

同步从 `packages/shared/src/index.ts` 导出。

### 12.2 desktopHost

新增：

```ts
export async function createRunnerMinimalRun(
  input: CreateRunnerMinimalRunInput,
): Promise<CreateRunnerMinimalRunResponse> {
  requireTauri();
  return invoke("create_runner_minimal_run", { input });
}

export async function listRunnerMinimalRuns(): Promise<RunnerMinimalRunSummary[]> {
  requireTauri();
  return invoke("list_runner_minimal_runs");
}
```

## 十三、前端 UI

建议在 `ProjectPlanPage` 的阶段 33 lock 卡片下方新增：

```text
阶段 34 最小 Runner 执行
```

显示规则：

- 只有选中草案对应的 lock 存在且 `status=locked` 时才展示创建按钮。
- lock 未满足沙箱路径要求时，展示不可执行原因。
- 已有 run 时只展示 run 结果，不重复执行。
- 按钮文案必须是：

```text
执行阶段34最小Runner
```

二次确认文本：

```text
我确认执行阶段34最小Runner，只允许沙箱范围
```

必须展示：

- 关联 task。
- 关联 runner_request。
- 允许文件。
- 即将写入的沙箱文件。
- 允许命令。
- 不会自动提交 Git。
- 不会继续执行下一个任务。

错误提示必须粗粒度：

```text
阶段34最小执行失败
```

不要展示 raw stderr、raw stdout、SQL 错误、堆栈、敏感内容。

## 十四、测试要求

### 14.1 Rust 测试

至少新增：

1. `minimal_run_requires_second_confirmation`
   - 缺二次确认拒绝。
   - 确认文本错误拒绝。
2. `minimal_run_rejects_unknown_lock`
   - 不存在 lock 拒绝。
3. `minimal_run_rejects_revoked_lock`
   - lock revoked 拒绝。
4. `minimal_run_rejects_unlocked_or_polluted_lock`
   - lock 状态非 `locked` 拒绝。
   - lock 被污染为可执行状态时拒绝或 schema 拒绝。
5. `minimal_run_rejects_non_sandbox_allowed_files`
   - allowed_files 包含 `apps/...` 或 `packages/...` 拒绝。
6. `minimal_run_rejects_protected_paths`
   - allowed_files 命中保护路径拒绝。
7. `minimal_run_rejects_path_traversal`
   - `../secret`、反斜杠、绝对路径、盘符路径拒绝。
8. `minimal_run_rejects_forbidden_commands`
   - 污染 command_plan 包含 `git commit` / `Remove-Item` 拒绝。
9. `minimal_run_creates_single_run_for_single_lock`
   - 完整链路只创建 1 条 run。
   - 同一 lock 幂等，不重复执行。
10. `minimal_run_writes_only_allowed_sandbox_file`
    - 实际写入文件必须是 allowed_files 子集。
11. `minimal_run_stops_on_command_failure`
    - 白名单命令失败后停止后续命令。
12. `minimal_run_records_command_summaries_with_limits`
    - stdout / stderr 截断。
    - 不保存超长输出。
13. `minimal_run_does_not_commit_or_modify_git_history`
    - 不执行 commit / push / reset / clean。
14. `minimal_run_does_not_call_model_or_network`
    - 不新增 model_calls。
    - 不发网络。
15. `minimal_run_does_not_create_tasks_or_runner_requests`
    - 不新增 tasks。
    - 不新增 runner_requests。
16. `minimal_run_does_not_auto_advance_next_task`
    - 不自动执行第二个 request。
17. `list_minimal_runs_filters_current_project`
    - 项目隔离。
18. `minimal_run_inputs_reject_unknown_fields`
    - deny unknown fields。
19. `minimal_run_rejects_sensitive_requested_by`
    - `requested_by` 含 secret 模式拒绝。

### 14.2 前端检查

- 没有完整 Runner 执行入口。
- 只有阶段 34 最小执行入口。
- 二次确认文本完全匹配。
- 已有 run 不重复执行。
- 展示写入文件、命令摘要、Git 摘要。
- 错误提示不展示 raw error。
- Tauri 未连接时不崩溃。
- `useCallback` / `useEffect` 依赖完整。

### 14.3 验证命令

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
rg -n "git commit|git push|git reset|git clean|Remove-Item|rm -rf|curl|wget|ssh|npm install|cargo add" apps/desktop/src-tauri/src/services/runner_minimal_run.rs packages/ui/src/pages/ProjectPlanPage.tsx
```

最后一条如果命中，必须逐条解释：只允许命中禁止列表、测试污染值或安全扫描说明，不允许出现在真实执行路径。

## 十五、文档同步

实现完成后必须同步：

```text
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
docs/data-model-draft.md
docs/api-draft.md
docs/demo-checklist.md
scripts/README.md
```

同步口径：

```text
阶段 34 完成：系统已开放极小范围最小 Runner 执行。第一版只允许一个 task、一个 runner_request、一个 execution lock、一个执行批次；只允许沙箱白名单路径；只允许后端固定白名单命令；执行前需要二次确认和 checkpoint 要求；执行后只展示 diff / command 摘要，不自动提交 Git、不自动继续下一个任务、不调用模型、不发网络。
```

## 十六、不要做的事

- 不要实现完整 Runner。
- 不要批量执行。
- 不要自动执行下一条任务。
- 不要让前端传命令。
- 不要让模型生成命令。
- 不要执行任意 shell。
- 不要写真实业务源码路径。
- 不要删除文件。
- 不要自动 commit。
- 不要自动 rollback。
- 不要发网络。
- 不要调用真实模型。
- 不要读取 secret。
- 不要展示 raw stdout / stderr 全文。
- 不要把执行器做成后台常驻服务。

## 十七、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 34 完成：最小真实 Runner 执行

改了哪些文件：
- ...

是否新增 migration：
- 是，011_add_runner_minimal_runs.sql

核心行为：
- 只允许基于未撤销的 runner_execution_lock 创建一个最小执行 run。
- 只允许一个 task、一个 runner_request、一个 execution batch。
- 必须二次确认。
- 只写沙箱白名单路径。
- 命令来自后端固定白名单，前端不能传命令。
- 执行前保存 Git 状态摘要。
- 执行后保存 Git 状态和 diff 摘要。
- 失败立即停止。
- 不自动提交 Git。
- 不自动继续下一个任务。
- 不调用模型，不发网络。

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
- 禁止命令扫描

遗留风险：
- 无 / ...
```

最终阶段 34 口径：

```text
阶段 34 完成后：系统具备极小范围真实 Runner 执行能力，但只限一个任务、一个 request、一个 lock、一个执行批次和沙箱白名单路径；必须人工二次确认；不自动提交 Git，不自动执行下一项，不调用模型，不发网络，不开放任意 shell。
```
