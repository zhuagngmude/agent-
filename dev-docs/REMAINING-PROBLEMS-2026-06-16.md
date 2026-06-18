# Agent Swarm - 剩余问题报告（历史）

> 检查时间：2026-06-16  
> 检查人：WorkBuddy  
> 项目路径：`F:\Projects\agent-swarm`

> 2026-06-17 更新：本文是历史问题单。部分问题已经修复或产品方向已改变；当前以 `dev-docs/当前项目导航.md` 和 `dev-docs/下一步开发路线.md` 为准。

---

## 一、严重问题（会阻塞自动流程）

### 1. `auto_swarm.rs` 自动流程中 gate/dry-run/lock 状态可能不是 "approved"（已验证，不是实际阻塞点）

**文件：** `apps/desktop/src-tauri/src/services/auto_swarm.rs`（第 124-170 行）

**问题描述：**
`run_one_runner_request` 函数依次调用：
1. `auto_create_runner_execution_gate`（第 124 行）→ 创建 gate
2. `auto_create_runner_dry_run`（第 139 行）→ 创建 dry run  
3. `auto_create_runner_execution_lock`（第 151 行）→ 创建 lock
4. `auto_create_runner_minimal_run`（第 166 行）→ 创建 minimal run 并执行

**问题：** `auto_create_runner_execution_gate` 内部调用 `create_runner_execution_gate`，但后者创建的 gate 记录 status 是什么？如果 status 是 `"created"` 或 `"pending"` 而不是 `"approved"`，那么下一步 `auto_create_runner_dry_run` 读取 gate 时会因为 status 不对而报错失败。

**需要验证：** 读取 `create_runner_execution_gate` 函数的 INSERT 语句，确认 gate 创建后的 status 值。如果非 `"approved"`，需要在 `auto_create_runner_execution_gate` 内追加自动审批逻辑（类似 `auto_swarm.rs` 第 113 行对 preflight 的处理方式）。

**2026-06-16 更新：** 已验证。gate/dry-run 的正常状态就是 `blocked_by_stage_boundary`，lock 的正常状态是 `locked`；后续 `create_runner_dry_run`、`create_runner_execution_lock`、`create_runner_minimal_run` 也正是按这些状态校验，不要求 `"approved"`。该项不是当前自动流程阻塞点。

**同样问题也存在于：**
- `auto_create_runner_dry_run` → dry run 的 status
- `auto_create_runner_execution_lock` → lock 的 status

---

### 2. `runner_minimal_run.rs` 同一份 AI 内容写入所有沙箱文件（已修复）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 579-604 行）

**问题描述：**
```rust
// 第 570 行：AI 生成一份内容
let sandbox_content = match call_ai_model_for_task(c, &lk.task_id) { ... };

// 第 579-604 行：同一份内容写入 ALL 沙箱文件
for sf in &sandbox_files {
    std::fs::write(sf_path, &sandbox_content)  // ← 所有文件内容一样！
}
```

**后果：** 如果任务允许写入 3 个文件（如 `frontend/index.html`, `frontend/app.js`, `styles.css`），AI 生成的同一份内容会被写入这 3 个文件，显然是错误的。

**应该改为：** AI 应按文件分别生成内容，或者 `call_ai_model_for_task` 需要知道要生成哪些文件、每个文件应该是什么内容。

**2026-06-16 更新：** 已改为把允许文件列表传给 AI，并要求返回 JSON 数组：`{"path":"...","content":"..."}`。写入前会校验路径必须完全匹配允许文件、不能多、不能少、不能重复、内容不能为空。

---

### 3. `runner_minimal_run.rs` 假设 Git 仓库已存在，新项目会失败（已修复）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 507-531 行）

**问题描述：**
```rust
// 第 507 行：直接执行 git status，假设 git 仓库存在
let pre_status = run_cmd_checked("git", &["status", "--short"], &repo_root_str);
let pre_diff = run_cmd_checked("git", &["diff", "--stat"], &repo_root_str);
// 如果 git 命令失败（仓库不存在），整个 minimal run 直接失败（第 509-531 行）
if pre_status.is_none() || pre_diff.is_none() {
    // → 直接返回 failed
}
```

**后果：** 新项目第一次执行任务时，如果 `generated/` 目录还不是一个 git 仓库，`git status` 会失败，导致 minimal run 直接失败，整个自动流程中断。

**应该改为：** 在执行 git 命令前，先检查/初始化 git 仓库（如果不存在则 `git init`）。

**2026-06-16 更新：** 已在 `workspace/generated` 沙箱根目录检查并初始化 Git 仓库；首次运行如果没有 `.git` 会执行 `git init`，后续 checkpoint/commit 都在生成沙箱仓库内完成。

---

## 二、主要问题（导致输出质量差或体验不好）

### 4. AI 模型硬编码为 `"deepseek-chat"`（需复查）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 909 行）

**问题描述：**
```rust
let request = ModelRequest {
    model_id: "deepseek-chat".to_string(), // ← 硬编码
    ...
};
```

**后果：** 所有任务都用同一个模型（deepseek-chat），无法根据任务类型/智能体选择合适的模型。比如后端任务可能适合用 deepseek-coder，文档任务可能适合用其他模型。

**当前方向：** 系统设置已经有模型服务配置入口。后续应确认 Runner 是否读取当前配置，而不是继续硬编码。

---

### 5. `call_ai_model_for_task` 的提示词没有告诉 AI 要生成哪些文件（已修复）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 900-905 行）

**问题描述：**
```rust
let user_message = format!(
    "任务信息：\n- 任务 ID: {}\n- 任务标题: {}\n- 任务描述: {}\n\n请生成该任务的执行结果内容...",
    task_id, task_title, task_description
);
```

提示词只说了"生成任务结果内容"，但没有说：
- 要生成哪些文件？
- 每个文件的路径和用途是什么？
- 每个文件应该包含什么内容？

**后果：** AI 生成的内容格式不确定，可能是一整块文本，无法正确拆分成多个文件。

**应该改为：** 提示词中应包含 `allowed_files`（允许写入的文件列表），并让 AI 按文件分别输出内容（比如用特殊分隔符或 JSON 格式）。

**2026-06-16 更新：** 提示词已包含允许写入的文件路径 JSON，并明确要求每个文件输出独立内容，且只返回 JSON。

---

### 6. `"0 models ready"` 提示体验仍需改善（需复查）

**文件：** `packages/ui/src/pages/ProjectPlanPage.tsx`（第 528 行附近，据交接文档）

**问题描述：** 交接文档说"已部分修复"，但"模型配置 UI 太简陋"仍标记为"部分"。用户可能在模型目录为空时看到警告，但还不知道**怎么添加模型**。

**当前方向：** 模型配置主入口在系统设置。后续应让错误提示直接指向系统设置，并区分 key 过期、余额不足、Base URL 错误和模型 ID 错误。

---

## 三、中等问题（应修复但不阻塞主流程）

### 7. `AgentExecutorTest.tsx` 不在仓库中

**文件：** `packages/ui/src/components/AgentExecutorTest.tsx`（据交接文档第 133 行，但 `git status` 未显示此文件）

**问题描述：** 交接文档提到之前会话创建了 `AgentExecutorTest.tsx`，但当前仓库中找不到这个文件（`git diff` 和 `git status` 均未显示）。可能是：
- 未提交（在 `.gitignore` 或未被 add）
- 被删除了
- 实际路径不同

**影响：** 测试执行器页面无法访问，之前做的 DeepSeek API 测试工作可能无法使用。

---

### 8. 数据库不删除无法看到新模板

**文件：** `data/local/agent-swarm.sqlite`（数据库文件）

**问题描述：** 模板 seed 用的是 `INSERT OR IGNORE`，如果数据库已存在旧模板记录，删除后再建库也不会更新（因为 `OR IGNORE` 跳过已存在的记录）。

**等等——** 实际上 `ensure_templates_seeded` 检查 `COUNT(*) == 0` 才 seed，所以如果数据库已有模板，不会重新 seed。这意味着：
- 如果数据库是从旧版本创建的（有旧的模板记录），即使代码改了模板内容，数据库里的模板也不会更新。
- **必须删除数据库重建**才能看到新模板。

**应该改为：** 每次启动检查 `builtin_templates` 的 `is_builtin` 记录是否需要更新（比较 `title`/`description` 是否变化），如果有变化则 UPDATE。

---

### 9. `call_ai_model_for_task` AI 调用失败时没有重试（已部分修复）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 570-576 行）

**问题描述：**
```rust
let sandbox_content = match call_ai_model_for_task(c, &lk.task_id) {
    Ok(ai_content) => ai_content,
    Err(e) => {
        return fail_run(c, &pid, &id, &[], "ai_call_failed", &e);
        // ← 直接失败，没有重试
    }
};
```

**后果：** 网络抖动、API 限流等临时错误会导致整个 minimal run 失败，需要用户手动重新触发。

**应该改为：** 加入重试逻辑（比如最多重试 2 次），或至少把错误原因区分开（限流 vs 认证失败 vs 网络错误）。

**2026-06-16 更新：** 已加入默认 2 次重试，失败信息会说明重试次数和最后一次错误。可用 `AGENT_SWARM_RUNNER_AI_RETRIES` 调整，限制范围 1-5。

---

### 10. `AI_TIMEOUT_SECS` 硬编码为 45 秒（已部分修复）

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（需确认具体行）

**问题描述：** 超时时间硬编码在代码里，不同任务复杂度可能需要不同超时时间。简单任务 45 秒够，复杂任务可能不够。

**应该改为：** 超时时间应可配置（环境变量或数据库配置），或根据任务类型动态调整。

**2026-06-16 更新：** 默认仍是 45 秒，但可通过 `AGENT_SWARM_RUNNER_AI_TIMEOUT_SECS` 配置，限制范围 5-300 秒。

---

## 四、次要问题（优化项，不紧急）

### 11. 角色只有 6 个，用户想要 13 个

**文件：** `data/seed/project_agent_swarm.seed.json`（seed 数据）

**问题描述：** 当前 seed 数据只有 6 个智能体（frontend, backend, qa, docs, reviewer, architect），用户之前表达过想要 13 个角色。

**应新增的角色（据用户之前描述）：** 文档师、安全审查、DevOps、UX 设计、数据建模等。

---

### 12. 工作流图（前端页面）缺失（暂缓）

**文件：** `packages/ui/src/pages/*.tsx`（前端页面）

**当前方向：** 主导航已经收敛，不再优先做展示型工作流图。若恢复，必须接真实状态，不做假展示。

---

### 13. `runner_minimal_run.rs` 中 `side_effects_true()` 的 `triggers_agents` 为 `false`

**文件：** `apps/desktop/src-tauri/src/services/runner_minimal_run.rs`（第 877 行）

**问题描述：**
```rust
triggers_agents: false,  // ← 执行后不触发其他智能体
```

如果任务执行完成后需要触发其他智能体（比如前端完成后触发 QA 测试），这里应该为 `true` 或在执行完成后主动触发后续任务。

---

## 五、建议的修复顺序

1. **先验证问题 1**（auto_swarm.rs 流程是否能真正跑通）→ 在测试环境跑一个完整想法，看卡在哪一步
2. **修复问题 2 和 5**（AI 生成内容按文件拆分）→ 让执行结果真正有用
3. **修复问题 3**（Git 仓库初始化）→ 让新项目能跑通
4. **修复问题 4 和 10**（模型可配置 + 超时可配置）→ 提升灵活性
5. **修复问题 8**（模板自动更新）→ 避免用户手动删数据库
6. **修复问题 6**（模型配置 UI 优化）→ 改善体验
7. **问题 7、11、12**（次要优化）→ 后续迭代

---

## 六、快速验证建议

**最直接验证方法：** 删数据库 → 重启应用 → 输入"登录页面"想法 → 一路点确认 → 看是否到达执行阶段且成功。

如果卡住，看终端 Rust 日志（Tauri 控制台）找错误信息。

---

*检查完毕。以上问题供下一位智能体参考修复。*
