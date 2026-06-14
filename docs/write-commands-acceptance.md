# 写入 Commands 正式验收

日期：2026-06-14

本文验收 `docs/write-commands-security-design.md` 和 `docs/write-commands-design.md` 中定义的 6 个 Tauri/Rust 写入 commands。验收范围只包含本地 SQLite 记录写入和状态流转，不开放真实 Runner、真实模型调用、文件写入、Git 修改或云同步。

## 一、验收范围

| command | 实现位置 | 验收结论 |
|---------|----------|----------|
| `create_task` | `apps/desktop/src-tauri/src/services/tasks.rs`、`commands/tasks.rs` | 通过 |
| `update_task_status` | `apps/desktop/src-tauri/src/services/tasks.rs`、`commands/tasks.rs` | 通过 |
| `create_approval` | `apps/desktop/src-tauri/src/services/approvals.rs`、`commands/approvals.rs` | 通过 |
| `approve_approval` | `apps/desktop/src-tauri/src/services/approvals.rs`、`commands/approvals.rs` | 通过 |
| `reject_approval` | `apps/desktop/src-tauri/src/services/approvals.rs`、`commands/approvals.rs` | 通过 |
| `patch_only_approval` | `apps/desktop/src-tauri/src/services/approvals.rs`、`commands/approvals.rs` | 通过 |

相关提交：

```text
db44275 feat: 实现 create_task 写入命令
9368e2a feat: 实现 update_task_status 写入命令
f76e45a feat: 实现 create_approval 写入命令
f69f122 feat: 实现审批终态写入命令
```

## 二、分层验收

| 层级 | 要求 | 验收结果 |
|------|------|----------|
| `commands/` | 只作为 Tauri invoke 入口，接收参数并调用 service | 通过 |
| `services/` | 负责输入校验、关联检查、状态机和业务边界 | 通过 |
| SQLite 写入 | 使用 rusqlite 参数化 SQL 和事务，不拼接用户输入 | 通过 |
| 数据库结构 | 本轮不新增表、不修改 migration | 通过 |
| UI | 本轮不接前端按钮，只验收后端写入能力 | 通过 |

## 三、输入校验验收

已通过 Rust 单元测试覆盖：

- `create_task`
  - 正常创建任务，返回 `status = queued`。
  - 空 `title` 被拒绝。
  - 超长 `title` / `description` 被拒绝。
  - 非法 `priority` / `risk_level` 被拒绝。
  - 不存在的 `assigned_agent_id` / `depends_on` 被拒绝。
  - 重复 `depends_on` 被拒绝。
- `update_task_status`
  - 合法状态流转通过。
  - 旧原型遗留的非法状态 `review` 被拒绝。
  - 不存在任务被拒绝。
  - 终态任务不能回到运行态。
- `create_approval`
  - 正常创建审批，返回 `status = pending`。
  - 非法 `target_service` / `operation_types` / `risk_level` 被拒绝。
  - 空或重复 `operation_types` 被拒绝。
  - 超长 `reason` 被拒绝。
  - 不存在的 `request_agent_id` / `task_id` 被拒绝。
- 审批终态 commands
  - `pending -> approved` 通过。
  - `pending -> rejected` 通过。
  - `pending -> patch_only` 通过。
  - `approved` / `rejected` / `patch_only` 不能再次变更。
  - 不存在审批被拒绝。
  - 超长 `reject_reason` 被拒绝。

## 四、状态机验收

### TaskStatus

已实现并测试的合法流转：

```text
queued -> running
queued -> cancelled
running -> completed
running -> blocked
running -> waiting_user
running -> failed
running -> cancelled
blocked -> running
waiting_user -> running
waiting_user -> cancelled
```

已验证 `completed`、`failed`、`cancelled` 等终态不会再次进入运行态。

### ApprovalStatus

已实现并测试的合法流转：

```text
pending -> approved
pending -> rejected
pending -> patch_only
```

已验证 `approved`、`rejected`、`patch_only` 不能再次变更。

## 五、副作用边界验收

本轮 commands 只允许写入 SQLite 中的 `tasks` / `approvals` 记录。已确认仍然禁止：

- 不创建 Runner request 或 Runner job。
- 不执行本地命令。
- 不写用户项目文件。
- 不修改 Git。
- 不触发真实 Agent。
- 不调用真实模型。
- 不导入 provider SDK。
- 不读取、返回或记录 raw key。
- 不写入 `data/local/`、`logs/`、`data/mock/runtime-state.json`、`design/image2/`、`_internal/`、`.playwright-cli/`。

特别说明：

- `approve_approval` 即使面对 `target_service = runner` 的审批，也只更新审批记录状态，不创建 Runner job。
- `patch_only_approval` 不接收 patch 内容，不生成 patch 文件，不写用户项目文件。
- `create_task` 和 `update_task_status` 不会自动创建 approval。

## 六、旧项目关系验收

本轮只迁移旧项目中的通用业务语义：

- 任务状态流转语义。
- 审批状态 `pending` / `approved` / `rejected` / `patch_only`。
- Agent、Task、Approval 属于当前项目的关联约束。

本轮明确不迁移：

- `project_plan` 审批后批量生成 5 个任务。
- `project_plan` 审批后生成只读 Runner request。
- Runner job 生命周期。
- Agent Run 真实链路。
- Git checkpoint 执行。
- 文件写入和 patch 落盘。

这些能力后续必须作为独立迁移项重新设计和验收。

## 七、验证命令与结果

执行位置：`F:\projects\agent-swarm\apps\desktop\src-tauri`

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo check
```

结果：

```text
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

执行位置：`F:\projects\agent-swarm\apps\desktop\src-tauri`

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo test
```

结果：

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

执行位置：`F:\projects\agent-swarm`

```powershell
git diff --check
git status --short
```

结果：

```text
git diff --check: 通过，无输出
git status --short: 验收前为干净工作区
```

## 八、结论

写入 commands 后端实现通过正式验收，可以进入下一步：前端共享 UI 接入写入 invoke。

进入下一步前仍必须保持以下边界：

- 不开放真实 Runner。
- 不调用真实模型。
- 不写用户项目文件。
- 不修改 Git 执行能力。
- 不把旧 Node.js HTTP API 继续作为正式后端扩展。
- 不继续在旧手搓前端上叠新功能。
