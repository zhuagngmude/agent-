# 阶段 30：Runner 执行前审查闸门

目标：阶段 29 已经能查看 `project_plan` 审批后真实落库的 `tasks` 和只读 `runner_requests`。阶段 30 要做的是“执行前审查闸门”，让用户可以针对某条只读 `runner_request` 生成一份可审查的执行申请和安全预检结果。

这一阶段仍然不执行 Runner，不执行命令，不写用户项目文件，不改 Git，不发网络请求，不调用真实模型。它只把“将来如果要执行，需要审查什么”落成结构化记录和 UI。

## 一、宪法约束

阶段 30 必须遵守 `docs/Agent宪法.md` 和 `docs/AI开发细则.md`：

- 不越过当前阶段边界。
- 不绕过审批写用户项目文件。
- Runner 不得自动执行命令、写文件、删文件、发网络请求或修改 Git。
- 写入、变更、Runner、Git checkpoint、模型相关能力必须进入审批链，不得直接开放。
- 任何数据库结构变更必须走 migration。
- 新增依赖必须先说明理由并获批准；阶段 30 不需要新增依赖。

结论：阶段 30 不是 Runner 执行阶段。它只允许创建“执行前审查申请”，不允许创建可执行 Runner job。

## 二、阶段边界

允许做：

- 新增 SQLite migration，保存执行前审查记录。
- 为一条已存在的只读 `runner_requests` 创建执行前审查申请。
- 同时创建一条 `approvals` 记录，`target_service = runner_preflight`。
- 在执行前审查记录中保存结构化预检结果：
  - 关联 task。
  - 关联 runner_request。
  - 操作类型。
  - 虚拟 affected files。
  - 风险等级。
  - 是否需要 Git checkpoint。
  - 是否需要二次确认。
  - 阻断原因。
  - 安全说明。
- 在前端展示“申请执行前审查”按钮和“执行前审查记录”列表。
- 用户可以查看审查记录、阻断原因和审批状态。
- 增加测试证明它没有 Runner、文件、Git、模型、网络副作用。

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
- 不把 `runner_requests.status` 改为 `running` / `completed`。
- 不把 `tasks.status` 改为 `running` / `completed`。
- 不改 `approve_approval`，不能让通用审批通过后自动执行任何 Runner 行为。
- 不让 Agent 自己批准执行。
- 不展示 raw prompt / raw provider response / raw error / raw secret。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 三、为什么先做 preflight

现在系统已经能生成任务和只读 Runner request，但这些 request 还只是“计划意图”，不是执行许可。

如果直接开放执行，会同时引入：

- 文件范围锁定。
- 命令白名单。
- Git checkpoint。
- 执行日志。
- 失败回滚。
- 权限模型。
- 审计追责。

这一步太大，容易破坏宪法边界。阶段 30 只做执行前审查，把真正执行前必须检查的东西先结构化落库，下一阶段再决定是否允许把审查通过的记录转成更接近执行的 gate。

## 四、数据模型

新增 migration：

```text
007_add_runner_preflight_reviews.sql
```

新增表：

```sql
CREATE TABLE IF NOT EXISTS runner_preflight_reviews (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  approval_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT NOT NULL,
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1,
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  blocked_reasons TEXT NOT NULL,
  safety_summary TEXT NOT NULL,
  requested_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_project_id
  ON runner_preflight_reviews(project_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_runner_request_id
  ON runner_preflight_reviews(runner_request_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_preflight_reviews_project_runner_request
  ON runner_preflight_reviews(project_id, runner_request_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_approval_id
  ON runner_preflight_reviews(approval_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_status
  ON runner_preflight_reviews(project_id, status, created_at);
```

说明：

- `runner_request_id` 唯一，避免同一只读 request 重复申请多份 preflight。
- `status` 第一版只允许：

```text
pending_review
blocked
approved_for_future_execution
cancelled
```

- 阶段 30 第一版创建后状态建议固定为 `pending_review` 或 `blocked`。
- 如果预检发现阻断原因，状态为 `blocked`，仍可写入记录，但不创建可执行 job。
- `approval_id` 指向新增的审批记录；审批记录也只是审查审批，不是执行许可。
- `operation_types`、`affected_files`、`blocked_reasons` 使用 JSON 数组字符串。
- `affected_files` 仍只能是 `virtual/...`，不能是真实文件路径。

## 五、后端类型

在 `apps/desktop/src-tauri/src/services/project_plan.rs` 或新模块 `services/runner_preflight.rs` 中新增类型。推荐新模块，避免 `project_plan.rs` 继续膨胀。

建议文件：

```text
apps/desktop/src-tauri/src/services/runner_preflight.rs
apps/desktop/src-tauri/src/commands/runner_preflight.rs
```

如果新增模块，需要在 `services/mod.rs`、`commands/mod.rs`、`lib.rs` 注册。

### 5.1 输入类型

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRunnerPreflightReviewInput {
    pub runner_request_id: String,
    pub second_confirm: bool,
    pub confirm_text: String,
    #[serde(default)]
    pub requested_by: Option<String>,
}
```

确认文本固定：

```text
我确认只创建执行前审查，不执行Runner
```

注意：

- 文本必须完全匹配或至少包含 `执行前审查` 和 `不执行Runner`。推荐完全匹配，避免歧义。
- `requested_by` 缺省为 `local_user`。

### 5.2 输出类型

```rust
#[derive(Debug, Serialize)]
pub struct RunnerPreflightReviewSummary {
    pub id: String,
    pub project_id: String,
    pub runner_request_id: String,
    pub task_id: String,
    pub approval_id: String,
    pub status: String,
    pub risk_level: String,
    pub operation_types: Vec<String>,
    pub affected_files: Vec<String>,
    pub requires_git_checkpoint: bool,
    pub requires_second_confirm: bool,
    pub blocked_reasons: Vec<String>,
    pub safety_summary: String,
    pub requested_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerPreflightReviewResponse {
    pub review: RunnerPreflightReviewSummary,
    pub approval: ApprovalSummary,
    pub side_effects: ProjectPlanSideEffects,
}
```

可以复用 `ProjectPlanSideEffects`，但所有字段必须 false：

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

如果后端认为创建 approval 是副作用，不要复用 `creates_tasks/creates_runner_requests` 表示；第一版只在文档里说明“会创建 approval + preflight review，不创建任务或 runner request”。

## 六、后端服务行为

### 6.1 新增函数

```rust
pub fn create_runner_preflight_review(
    connection: &mut Connection,
    input: CreateRunnerPreflightReviewInput,
) -> Result<CreateRunnerPreflightReviewResponse, String>

pub fn list_runner_preflight_reviews(
    connection: &Connection,
) -> Result<Vec<RunnerPreflightReviewSummary>, String>
```

可选增强：

```rust
pub fn get_runner_preflight_review_by_runner_request(
    connection: &Connection,
    runner_request_id: String,
) -> Result<Option<RunnerPreflightReviewSummary>, String>
```

### 6.2 创建 preflight 的步骤

1. 获取当前项目 id。
2. 校验 `runner_request_id` 非空，长度 1..200。
3. 校验二次确认：

```text
second_confirm == true
confirm_text == "我确认只创建执行前审查，不执行Runner"
```

4. 读取当前项目下的 `runner_requests`。
5. 如果不存在，返回：

```text
not_found: runner request not found
```

6. 校验 `runner_requests.status == "queued"`。
   - 第一版只允许 queued 的只读 request 创建 preflight。
7. 校验 `operation_types` 必须包含 `runner_request_readonly`。
8. 校验 `operation_types` 中不包含真实执行操作：

禁止值建议：

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

9. 校验 `affected_files` 全部满足：

```text
以 virtual/ 开头
不包含 ..
不包含 \
不包含 :
不包含 ~
```

10. 读取关联 task，必须存在且属于当前项目。
11. 计算风险：
   - 如果 task.risk_level 是 `high`，preflight risk 为 `high`。
   - 如果 operation_types 中有 `security_review_plan`，preflight risk 为 `high`。
   - 否则使用 task.risk_level，缺省 `medium`。
12. 计算阻断原因。

第一版建议永远加入这个阻断原因：

```text
runner_execution_disabled_by_stage_boundary
```

这样即使 preflight 创建成功，也不会被误解为可以执行。

13. 创建 approval：

```text
target_service = runner_preflight
operation_types = ["runner_preflight_review"]
status = pending
risk_level = high 或 medium
reason = "申请对只读 Runner request 创建执行前审查：{runner_request_id}"
task_id = 关联 task_id
request_agent_id = agent_architect 或 requested_by 映射不到时使用 agent_architect
```

14. 创建 `runner_preflight_reviews`：
   - 如果有阻断原因，`status = blocked`。
   - 如果没有阻断原因，`status = pending_review`。
   - 阶段 30 因为 Runner 仍关闭，默认会有阻断原因，所以通常是 `blocked`。
15. 整个创建过程放在一个事务中。
16. 返回 review + approval + 全 false side_effects。

### 6.3 幂等规则

同一个 `runner_request_id` 重复创建 preflight：

- 不重复创建 approval。
- 不重复创建 review。
- 返回已有 review 和已有 approval。
- side_effects 仍全 false。

这点必须测试。

### 6.4 不要改这些路径

- 不改 `approve_approval` 的语义。
- 不改 `approve_project_plan` 的语义。
- 不改 `runner_requests` 状态。
- 不改 `tasks` 状态。
- 不写 `runtime_events`。

阶段 30 写入的是“审查申请记录”，不是执行状态变化；暂不写 runtime events，避免让人误判为真实运行。

## 七、Tauri commands

新增：

```rust
#[tauri::command]
pub fn create_runner_preflight_review(
    state: tauri::State<'_, DbState>,
    input: CreateRunnerPreflightReviewInput,
) -> Result<CreateRunnerPreflightReviewResponse, String>

#[tauri::command]
pub fn list_runner_preflight_reviews(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<RunnerPreflightReviewSummary>, String>
```

注册到 `lib.rs`。

命名不要用 `execute`、`run`、`start`。必须叫 `preflight` 或 `review`。

## 八、前端类型和 host 封装

### 8.1 shared 类型

在 `packages/shared/src/types/project-plan.ts` 或新文件中增加：

```ts
export type CreateRunnerPreflightReviewInput = {
  runner_request_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RunnerPreflightReviewSummary = {
  id: string;
  project_id: string;
  runner_request_id: string;
  task_id: string;
  approval_id: string;
  status: string;
  risk_level: string;
  operation_types: string[];
  affected_files: string[];
  requires_git_checkpoint: boolean;
  requires_second_confirm: boolean;
  blocked_reasons: string[];
  safety_summary: string;
  requested_by: string;
  created_at: string;
  updated_at: string;
};

export type CreateRunnerPreflightReviewResponse = {
  review: RunnerPreflightReviewSummary;
  approval: ApprovalSummary;
  side_effects: ProjectPlanSideEffects;
};
```

从 `packages/shared/src/index.ts` 导出。

### 8.2 desktopHost

新增：

```ts
export async function createRunnerPreflightReview(
  input: CreateRunnerPreflightReviewInput,
): Promise<CreateRunnerPreflightReviewResponse> {
  requireTauri();
  return invoke("create_runner_preflight_review", { input });
}

export async function listRunnerPreflightReviews(): Promise<RunnerPreflightReviewSummary[]> {
  requireTauri();
  return invoke("list_runner_preflight_reviews");
}
```

## 九、前端 UI

建议在 `ProjectPlanPage` 的“已生成任务和只读 Runner request”卡片下方增加：

```text
执行前审查
```

显示内容：

- 当前选中草案的 runner requests。
- 每条 request 一个“创建执行前审查”按钮。
- 按钮必须有二次确认，确认文本固定：

```text
我确认只创建执行前审查，不执行Runner
```

- 创建后显示审查记录：
  - 状态。
  - 风险。
  - 阻断原因。
  - 操作类型。
  - affected files。
  - 审批 id。
  - 安全说明。

文案必须明确：

```text
执行前审查不会执行 Runner，不会执行命令，不会写文件，不会改 Git。
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
创建执行前审查
```

## 十、测试要求

### 10.1 Rust 测试

至少新增：

1. `create_preflight_requires_second_confirmation`
   - 未勾选二次确认拒绝。
   - 确认文本错误拒绝。

2. `create_preflight_rejects_unknown_runner_request`
   - 不存在的 request 拒绝。

3. `create_preflight_creates_review_and_pending_approval`
   - 先走 project_plan 审批生成 runner_requests。
   - 对其中一条创建 preflight。
   - 写入 1 条 `runner_preflight_reviews`。
   - 写入 1 条 `approvals`，target_service 为 `runner_preflight`。
   - 不写 tasks。
   - 不写 runner_requests。
   - 不写 runtime_events。
   - 不写 model_calls。

4. `create_preflight_is_idempotent_for_same_runner_request`
   - 重复调用只返回已有 review。
   - approval 不重复。
   - review 不重复。

5. `create_preflight_blocks_real_execution_by_stage_boundary`
   - 返回 blocked_reasons 包含 `runner_execution_disabled_by_stage_boundary`。
   - 不创建可执行 job。

6. `create_preflight_rejects_polluted_affected_files`
   - 手动污染 `runner_requests.affected_files = ["../secret"]`。
   - 创建 preflight 被拒绝。

7. `create_preflight_rejects_forbidden_operation_type`
   - 手动污染 operation_types 包含 `file_write` 或 `command_execute`。
   - 创建 preflight 被拒绝。

8. `list_preflight_reviews_filters_current_project`
   - 不返回其他 project 的 review。

9. `preflight_does_not_change_task_or_runner_request_status`
   - 调用前后 task.status、runner_request.status 不变。

### 10.2 前端检查

- `ProjectPlanPage` 不出现执行按钮。
- 创建 preflight 后刷新列表。
- `useCallback` / `useEffect` 依赖完整。
- 未连接 Tauri 时不崩溃。
- 错误提示使用粗粒度文案，不展示后端堆栈或敏感内容。

### 10.3 验证命令

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

## 十一、文档同步

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
阶段 30 完成：只读 runner_request 已可创建执行前审查记录和 runner_preflight 审批；
该审查仍被 stage boundary 阻断，不执行 Runner，不执行命令，不写文件，不改 Git，不调用模型，不创建可执行 Runner job。
```

## 十二、不要做的事

- 不新增真实 Runner。
- 不新增 Runner job 表。
- 不新增命令执行器。
- 不新增文件写入器。
- 不新增 Git checkpoint 执行。
- 不新增网络请求执行。
- 不新增模型调用。
- 不把 blocked preflight 自动变成 approved。
- 不让 `approve_approval` 触发 preflight 后续动作。
- 不在 UI 上出现“执行/运行/开始/提交/写入”这类动作按钮。

## 十三、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 30 完成：Runner 执行前审查闸门

改了哪些文件：
- ...

是否新增 migration：
- 是，007_add_runner_preflight_reviews.sql

核心行为：
- 可对只读 runner_request 创建执行前审查记录。
- 同时创建 pending runner_preflight 审批。
- 重复创建同一 runner_request 的 preflight 幂等返回已有记录。
- 审查记录包含 blocked_reasons，其中包含 runner_execution_disabled_by_stage_boundary。
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

遗留风险：
- 无 / ...
```

最终阶段 30 口径：

```text
阶段 30 完成后：系统具备 Runner 执行前审查闸门，可以把只读 runner_request 转为可审查的 preflight review 和 runner_preflight 审批；
但真实 Runner 仍关闭，不执行命令，不写用户项目文件，不改 Git，不调用模型，不创建可执行 Runner job。
```
