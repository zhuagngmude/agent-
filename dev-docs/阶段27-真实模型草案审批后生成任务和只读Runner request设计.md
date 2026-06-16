# 阶段 27：审批通过后由真实模型草案生成任务和只读 Runner request

目标：阶段 26 已经允许把真实模型草案保存为待审批 `project_plan_drafts`。阶段 27 要把这类 `generated_by=real_model_preview` 的草案接入现有 `approve_project_plan` 实例化链路：用户二次确认审批通过后，生成 5 个 `tasks` 和 5 条只读 `runner_requests`。

这一阶段仍然不执行 Runner、不执行命令、不写用户项目文件、不改 Git、不再次调用模型。

## 一、阶段边界

允许做：

- 用户对真实模型草案对应的 `project_plan` approval 执行 `approve_project_plan`。
- 后端识别 `project_plan_drafts.generated_by = real_model_preview`。
- 审批通过后，在同一个 SQLite transaction 内：
  - 更新 `approvals.status = approved`。
  - 更新 `project_plan_drafts.status = instantiated`。
  - 创建 5 个 `tasks`。
  - 创建 5 条只读 `runner_requests`。
  - 写入 1 条本地 `runtime_events` 审计事件。
- 任务描述可以包含真实模型草案的脱敏摘要 `project_plan_drafts.summary`，但必须是已落库的安全摘要，不能重新调用模型或读取 raw response。
- UI 可以明确展示：真实模型草案审批通过后已生成任务和只读队列，但 Runner 没有执行。

禁止做：

- 不调用真实模型。
- 不读取 raw key / raw base URL。
- 不读取、保存或展示 raw prompt / raw provider request / raw provider response / raw provider error。
- 不创建可执行 Runner job。
- 不执行 Runner。
- 不执行命令。
- 不写用户项目文件。
- 不改 Git。
- 不新增前端 key / base URL / provider / model / prompt / headers / body 输入。
- 不让 Agent 自动触发审批。
- 不绕过 `approve_project_plan`，不改通用 `approve_approval` 让它能批准 project_plan。
- 不触碰保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。

## 二、当前基础

阶段 24 已经有本地确定性草案审批实例化链路：

```text
project_plan_drafts(draft)
-> approve_project_plan 二次确认
-> 5 tasks
-> 5 readonly runner_requests
-> runtime_events(project_plan_instantiated)
```

阶段 26 已经有真实模型草案保存链路：

```text
request_project_plan_model_draft
-> model_calls 安全审计记录
-> save_project_plan_model_draft
-> project_plan_drafts(generated_by=real_model_preview, model_call_id=...)
-> approvals(pending)
```

阶段 27 不应新建一条平行审批链。正确方向是收口现有 `approve_project_plan`，让它明确支持 `real_model_preview` 草案，并把真实模型安全摘要纳入任务描述或审计展示。

## 三、推荐实现策略

优先小改，不新增 migration。

重点文件：

```text
apps/desktop/src-tauri/src/services/project_plan.rs
packages/ui/src/pages/ProjectPlanPage.tsx
dev-docs/阶段27-真实模型草案审批后生成任务和只读Runner request设计.md
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
docs/data-model-draft.md
```

如果实现过程中发现现有字段已经足够，不要新增表、字段或 command。

## 四、后端行为要求

### 4.1 approve_project_plan 必须继续是唯一入口

真实模型草案实例化只能走：

```rust
approve_project_plan(connection, ApproveProjectPlanInput)
```

不要新增 `approve_real_model_project_plan`，除非现有函数无法保持清晰边界。当前看不需要新增 command。

通用审批函数 `approve_approval` 仍然必须拒绝 `target_service=project_plan`，避免绕过二次确认。

### 4.2 草案来源校验

在实例化前，后端应明确允许且只允许以下 `generated_by`：

```text
local_deterministic_template
real_model_preview
```

如果出现其他值，应返回：

```text
invalid_input: unsupported project plan draft source
```

不要静默接受未知来源，防止后续低级智能体塞入新来源。

### 4.3 真实模型草案安全校验

当 `draft.generated_by == real_model_preview` 时，必须校验：

- `draft.model_call_id IS NOT NULL`。
- `draft.summary.trim()` 非空。
- `draft.summary` 继续通过敏感值拦截，不能包含 `sk-...`、`Authorization: Bearer ...`、`api_key=`、`token=`、`password=`。
- 可选但推荐：查询 `model_calls`，确认：
  - `id = draft.model_call_id`。
  - `project_id` 等于当前项目。
  - `purpose = project_plan_generation`。
  - `provider = openai_compat`。
  - `model = gpt-5.4-mini`。
  - `status = success`。
  - `error_category IS NULL`。
  - `structured_summary` 与 `draft.summary` 一致，或至少非空。

如果复用阶段 26 的 `get_validated_summary()`，不要把 `model_calls` 原始字段返回给前端。

### 4.4 任务生成规则

阶段 27 仍然生成固定 5 个任务，不让模型动态决定任务数量、Agent、权限或文件范围。

继续使用现有 5 个角色：

```text
frontend
backend
qa
docs
reviewer
```

继续使用现有固定 `operation_types` 和虚拟 `affected_files`。

真实模型摘要的使用方式建议：

- 本地草案保持当前任务描述。
- 真实模型草案的任务描述在现有模板基础上追加：

```text
真实模型草案摘要：
{draft.summary 的安全截断版本}
```

建议截断到 1000 字以内，避免任务描述过长。不要保存 raw prompt 或 raw response。这里的 `draft.summary` 已经是阶段 25.3 脱敏、截断后的安全摘要。

不要让模型摘要覆盖固定任务标题、固定 Agent 分配、固定 runner request 安全说明。

### 4.5 runner_requests 仍然只读

生成的 `runner_requests` 必须保持：

- `status = queued`
- `checkpoint = NULL`
- `safety_note = Read-only Runner request...`
- `operation_types` 包含 `runner_request_readonly`

这些记录仍只是队列/审查记录，不是可执行 Runner job。

### 4.6 runtime_events 只写安全摘要

`runtime_events.reason` 可以继续写 `draft.summary`，但必须确认它来自安全摘要。

不要写：

- raw prompt
- raw provider request
- raw provider response
- raw provider error
- key / base URL
- 模型原始 reasoning

### 4.7 幂等

如果草案已经是 `instantiated`，二次调用 `approve_project_plan` 应继续幂等返回已有任务和只读 request ID，不重复插入。

真实模型草案和本地草案都必须满足这个规则。

## 五、前端行为要求

`ProjectPlanPage.tsx` 可以做小幅 UI 文案收口。

建议：

- 草案列表增加或复用 `generated_by` 展示。
- `generated_by=real_model_preview` 显示为“真实模型草案”。
- 批准区域文案改为更准确：

```text
批准后会创建 5 个 queued 任务和 5 条只读 runner_requests。不会执行 Runner、调用模型、写文件或修改 Git。
```

- 真实模型草案审批通过后，成功提示可以是：

```text
真实模型草案已批准，任务和只读队列已生成
```

如果不方便区分来源，也可以保持通用提示，但必须不误导用户以为 Runner 已执行。

前端禁止：

- 不展示 `model_call_id` 明细。
- 不展示 `audit_record_id` 明细。
- 不展示 raw 错误。
- 不新增模型配置输入。
- 不新增 Runner 执行按钮。

## 六、测试要求

Rust 必须新增或补强测试。

### 6.1 真实模型草案审批成功

准备：

- 插入安全 `model_calls` success 记录。
- 调用 `save_project_plan_model_draft` 保存为草案。
- 再调用 `approve_project_plan`。

断言：

- approval 变为 `approved`。
- draft 变为 `instantiated`。
- 创建 5 个 `tasks`。
- 创建 5 条 `runner_requests`。
- 创建 1 条 `runtime_events`。
- `tasks.description` 包含 `generated_by=real_model_preview` 或能明确识别真实模型来源。
- `tasks.description` 包含安全模型摘要。
- `runner_requests.safety_note` 仍声明只读，不执行命令/文件/Git。
- `side_effects.creates_tasks = true`。
- `side_effects.creates_runner_requests = true`。
- `side_effects.executes_runner = false`。
- `side_effects.writes_project_files = false`。
- `side_effects.modifies_git = false`。
- `side_effects.calls_real_model = false`。

### 6.2 真实模型草案审批不再次调用模型

可以通过 service 层结构证明：`approve_project_plan` 不依赖 provider trait、不读取 env、不调用 `request_project_plan_model_draft`。

测试可断言：

- 审批前后 `model_calls` 数量不变。
- 不新增新的 `model_calls`。
- 不需要设置 `AGENT_SWARM_OPENAI_COMPAT_API_KEY` 或 base URL，审批仍可通过。

### 6.3 缺失 model_call_id 的真实模型草案必须拒绝

构造：

- 手动插入 `generated_by=real_model_preview` 但 `model_call_id=NULL` 的草案。
- 调用 `approve_project_plan`。

断言：

- 返回 `invalid_input`。
- 不创建 tasks。
- 不创建 runner_requests。
- 不写 runtime_events。

### 6.4 model_calls 不匹配必须拒绝

至少覆盖：

- `model_call_id` 不存在。
- `model_calls.project_id` 不等于当前项目。
- `model_calls.status != success`。
- `model_calls.purpose != project_plan_generation`。

断言都相同：

- 不创建 tasks。
- 不创建 runner_requests。
- 不写 runtime_events。

### 6.5 摘要敏感值拦截

构造：

- 手动插入 `project_plan_drafts.summary` 含 `sk-...` 或 `Authorization: Bearer ...` 的真实模型草案。

断言：

- `approve_project_plan` 拒绝。
- 不创建任务/队列/事件。

### 6.6 幂等

同一个真实模型草案：

- 第一次 `approve_project_plan` 创建 5 个任务和 5 条只读队列。
- 第二次调用返回同一批 ID。
- 数量不增加。
- 不重复写 runtime_events。

如果现有本地草案幂等测试已经覆盖大部分逻辑，也要新增一个 `real_model_preview` 来源的专门测试。

## 七、不要做的重构

- 不要改任务数量。
- 不要引入模型输出 JSON parser。
- 不要让模型控制 Agent 分配。
- 不要让模型控制 `operation_types`。
- 不要让模型控制真实文件路径。
- 不要新增 Runner 执行状态机。
- 不要新增 Git checkpoint。
- 不要新增 migration，除非现有字段无法满足测试。
- 不要把 `approve_approval` 改成能实例化 project plan。
- 不要把阶段 25/26 的真实模型调用入口扩大到其他 purpose。

## 八、建议改动文件

预计代码文件：

```text
apps/desktop/src-tauri/src/services/project_plan.rs
packages/ui/src/pages/ProjectPlanPage.tsx
```

如果共享类型已经够用，不要改 `packages/shared`。

预计文档文件：

```text
dev-docs/阶段27-真实模型草案审批后生成任务和只读Runner request设计.md
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
docs/data-model-draft.md
```

不要提交 commit。

## 九、验证命令

修完后运行：

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
rg -n "sk-[A-Za-z0-9]{20,}|Authorization: Bearer [A-Za-z0-9._-]+|api_key=|token=|password=" apps/desktop/src-tauri/src packages docs dev-docs
```

说明：

- 旧错误名不应作为错误枚举出现。
- 密钥扫描命中测试假值或脱敏规则说明可以接受，但不得出现真实 key。

## 十、交付回复模板

低级智能体完成后按这个格式回复：

```text
阶段 27 完成：审批通过后由真实模型草案生成任务和只读 Runner request

改了哪些文件：
- ...

核心行为：
- real_model_preview 草案可通过 approve_project_plan 审批实例化。
- 审批通过后创建 5 个 tasks 和 5 条只读 runner_requests。
- 不再次调用模型。
- 不执行 Runner。
- 不写用户项目文件。
- 不改 Git。

新增测试：
- ...

验证结果：
- npm run typecheck
- npm run build
- cargo fmt --check
- cargo check
- cargo test
- git diff --check

遗留风险：
- 无 / ...
```

最终阶段 27 口径：

```text
阶段 27 完成后：真实模型草案保存为 project_plan 草案后，可以经人工二次确认审批生成 5 个任务和 5 条只读 Runner request；
审批动作不再次调用模型；
Runner request 仍只读；
仍不执行 Runner；
仍不执行命令；
仍不写用户项目文件；
仍不改 Git。
```
