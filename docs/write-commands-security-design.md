# 写入 Commands 安全边界设计

日期：2026-06-14

本文是教程 #11《后端安全技术规范及设计流程》在 `agent蜂群` 新架构中的裁剪版。它只服务于下一阶段的写入 commands 设计，不开放真实 Runner、真实模型调用或用户项目文件写入。

## 一、当前项目场景

`agent蜂群` 当前是单人本地桌面工具：

- 主入口：Tauri 2 + Rust 桌面宿主。
- UI：`packages/ui` 的 React + TypeScript + Vite + Ant Design。
- 数据库：SQLite + rusqlite，运行库位于 Tauri app data 目录。
- 通信方式：前端通过 Tauri `invoke` 调用 Rust commands。
- 不暴露 HTTP API，不做公网服务。
- 不做登录系统、不做多用户角色系统。

因此教程 #11 不能按 Web / SaaS 后端完整照搬，必须裁剪成单机 Tauri 写入边界。

## 二、教程 #11 裁剪表

| 防线 | 当前适用性 | 项目落地方式 |
|------|------------|--------------|
| 身份认证 | 第一版不适用 | 单人本地桌面工具，不做登录；后续如接云同步再重新设计。 |
| 权限控制 | 部分适用 | 不做账号角色权限；但必须保留审批边界、Agent 能力边界、Runner 禁用边界。 |
| 输入校验 | 强适用 | 所有写入 command 必须在 Rust 层校验类型、枚举、长度、必填字段和状态流转。 |
| 数据归属 | 部分适用 | 不做多用户归属；但必须校验 `task_id`、`agent_id`、`approval_id` 属于当前 `project_id`。 |
| 注入防护 | 适用 | SQLite 写入必须使用 rusqlite 参数化；禁止把用户输入拼进 SQL、命令或文件路径。 |
| 密码与敏感数据 | 当前不适用 | 当前无密码；真实模型仍禁止读取 raw key、返回 key fragment 或导入 provider SDK。 |
| 防止过度防御 | 适用 | 不把单机工具写成企业权限系统；不添加无法触发的死代码和重复校验。 |

## 三、写入 Commands 范围

下一阶段只允许设计以下写入 commands：

- `create_task`
- `update_task_status`
- `create_approval`
- `approve_approval`
- `reject_approval`
- `patch_only_approval`

暂不开放：

- `create_agent`
- `update_agent_status`
- `assign_task_agent`
- Runner job 创建或执行
- Git checkpoint 执行
- 文件写入
- 真实模型请求
- provider 配置读取

## 四、必须校验的输入

### 通用字段

- `project_id`：必须存在，且只能指向当前本地项目。
- `id`：由 Rust 层生成或严格校验格式；禁止前端传入任意覆盖已有记录。
- `created_at` / `updated_at`：由 Rust 层生成，前端不得直接指定。

### Task 输入

- `title`：必填，trim 后长度为 1 到 120 字符。
- `description`：可为空，非空时 trim 后不超过 2000 字符。
- `status`：只能使用 `queued`、`running`、`blocked`、`waiting_user`、`completed`、`failed`、`cancelled`。
- `priority`：只能使用 `low`、`medium`、`high`。
- `assigned_agent_id`：可为空；非空时必须存在于当前项目的 `agents` 表。
- `depends_on`：必须是字符串数组；每个 task id 必须存在于当前项目；禁止任务依赖自己。
- `risk_level`：可为空；非空时只能使用 `low`、`medium`、`high`。

### Approval 输入

- `task_id`：可为空；非空时必须存在于当前项目的 `tasks` 表。
- `request_agent_id`：必填，必须存在于当前项目的 `agents` 表。
- `target_service`：只能使用已登记服务名，第一版允许 `task`、`approval`、`runner`、`agent_config`、`model_gateway`。
- `operation_types`：必须是非空字符串数组，每一项必须在允许列表内。
- `risk_level`：只能使用 `low`、`medium`、`high`。
- `reason`：可为空；非空时 trim 后不超过 2000 字符。
- `reject_reason`：可为空；非空时 trim 后不超过 2000 字符。

## 五、状态机规则

### TaskStatus

允许的状态：

```text
queued
running
blocked
waiting_user
completed
failed
cancelled
```

第一版允许的状态流转：

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

禁止：

- `completed`、`failed`、`cancelled` 再回到运行态。
- 直接把不存在的任务改成任意状态。
- 前端传入未定义状态，例如旧原型遗留的 `review`。

### ApprovalStatus

第一版写入只允许从 `pending` 流转到：

```text
approved
rejected
patch_only
```

禁止：

- `approved`、`rejected`、`patch_only` 再次变更。
- 审批通过后自动触发 Runner。
- `patch_only` 写入用户项目文件。

## 六、禁止副作用

所有写入 commands 在当前阶段必须满足：

- 不执行本地命令。
- 不写用户项目文件。
- 不删除文件。
- 不修改 Git。
- 不创建真实 Runner job。
- 不调用真实模型。
- 不导入 provider SDK。
- 不读取、返回或记录 raw key。
- 不写入 `data/local/`、`logs/`、`data/mock/runtime-state.json`、`design/image2/`、`_internal/`、`.playwright-cli/`。

审批类 command 的语义仅限于更新 SQLite 中的审批记录和必要的审计字段，不代表授权执行真实副作用。

## 七、SQLite 安全

- 所有 SQL 必须使用 rusqlite 参数化。
- 禁止把前端输入拼接进 SQL 字符串。
- 禁止动态表名、动态列名来自前端输入。
- 数据库结构变更必须通过 `data/migrations/` 下的新 migration 文件。
- 运行库只能位于 Tauri app data 目录。
- 写入操作必须优先使用事务，避免部分成功。

## 八、错误返回

Rust commands 可以先继续返回 `Result<T, String>`，但错误消息必须稳定表达语义。后续如需要再升级成结构化错误对象。

第一版建议错误语义：

| 错误语义 | 使用场景 |
|----------|----------|
| `invalid_input` | 参数缺失、空字符串、超长、枚举非法、数组格式错误 |
| `not_found` | task / agent / approval / project 不存在 |
| `invalid_transition` | 状态机流转非法 |
| `forbidden_by_stage_boundary` | 触碰当前阶段禁止能力，如 Runner、模型、文件写入 |
| `conflict` | 记录已终态、重复审批、重复创建 |
| `database_error` | SQLite 读写失败 |

错误返回必须避免泄露：

- 本机敏感路径。
- raw key。
- provider 原始响应。
- 未授权资源的完整内容。

## 九、后续验收

进入写入实现后，必须新增 Rust 测试覆盖：

- 正常创建任务。
- 空 title 被拒绝。
- 超长 title / description / reason / reject_reason 被拒绝。
- 非法 `status` 被拒绝。
- 非法 `risk_level` 被拒绝。
- 非法 `target_service` / `operation_types` 被拒绝。
- 不存在的 `agent_id` / `task_id` / `approval_id` 被拒绝。
- 跨 project 的 task / agent / approval 关系被拒绝。
- 合法 Task 状态流转通过。
- 非法 Task 状态流转被拒绝。
- `pending` Approval 可以进入 `approved` / `rejected` / `patch_only`。
- 终态 Approval 不能再次变更。
- 审批通过不会创建 Runner job。
- `patch_only` 不会写用户项目文件。

验收命令至少包括：

```powershell
cd F:\projects\agent-swarm\apps\desktop\src-tauri
cargo check
cargo test

cd F:\projects\agent-swarm
git diff --check
```

## 十、结论

教程 #11 已按单机 Tauri 项目裁剪完成，可以进入写入 commands 设计；但仍不能直接开放 Runner、真实模型调用、provider SDK、raw key 读取或用户项目文件写入。
