# Tauri 只读骨架正式验收

日期：2026-06-14

本文是教材 #10 的验收存档，用于确认新架构的只读骨架已经合格，可以在后续进入写入 commands 和审批状态机设计。

## 一、验收范围

本次只验收以下链路：

```text
packages/ui
  -> @tauri-apps/api invoke
  -> apps/desktop/src-tauri/src/commands
  -> apps/desktop/src-tauri/src/services
  -> apps/desktop/src-tauri/src/db
  -> SQLite app data 数据库
```

已纳入验收的 command：

- `get_project`
- `list_agents`
- `list_tasks`
- `list_approvals`

本次不验收、也不开放：

- 真实 Runner 执行
- 真实模型调用
- provider SDK
- raw key 读取或返回
- 云同步
- 完整权限系统
- 绕过审批的用户项目文件写入

## 二、规则审计

已对照 `docs/Agent宪法.md` 和 `docs/AI开发细则.md` 检查：

- 没有修改保护路径：`design/image2/`、`_internal/`、`data/mock/runtime-state.json`、`data/local/`、`logs/`、`.playwright-cli/`。
- SQLite 运行库写入 Tauri app data 目录，不写入仓库内 `data/local/`。
- 数据库结构来自 `data/migrations/001_initial_sqlite.sql`，不是手动直改运行库。
- 初始数据来自 `data/seed/project_agent_swarm.seed.json`，旧 mock 只作为冻结参考。
- 当前前端只读数据通过 Tauri invoke 读取，不直接访问 SQLite。
- 没有新增真实写入 command。
- 没有新增 Runner、模型调用或网络 provider 请求。

验收结论：通过。

## 三、目录责任验收

Rust/Tauri 侧目录责任：

```text
apps/desktop/src-tauri/src/
  commands/   Tauri command 入口，只接收 Tauri State 并调用 services
  services/   业务读取与序列化边界
  db/         SQLite 初始化、migration、seed、连接状态
```

前端侧目录责任：

```text
packages/ui/src/
  utils/desktopHost.ts       Tauri invoke 和浏览器 fallback
  pages/OverviewPage.tsx     Overview 数据展示
  components/StatusBadge.tsx 状态展示组件
```

旧原型责任：

```text
apps/web/        冻结为旧 Web 原型参考
services/api/    冻结为旧 Node.js API / 规则参考
design/index.html 冻结为设计原型参考
```

验收结论：目录责任清楚，新主线没有继续扩展旧手搓前端或旧 Node.js HTTP 后端。

## 四、最小模块演练

### get_project

```text
OverviewPage
  -> useDesktopHostOverview()
  -> invoke("get_project")
  -> commands::projects::get_project()
  -> services::projects::get_current_project()
  -> SELECT projects
```

### list_agents

```text
OverviewPage
  -> useDesktopHostOverview()
  -> invoke("list_agents")
  -> commands::agents::list_agents()
  -> services::agents::list_agents()
  -> SELECT agents
```

### list_tasks

```text
OverviewPage
  -> useDesktopHostOverview()
  -> invoke("list_tasks")
  -> commands::tasks::list_tasks()
  -> services::tasks::list_tasks()
  -> SELECT tasks
```

### list_approvals

```text
OverviewPage
  -> useDesktopHostOverview()
  -> invoke("list_approvals")
  -> commands::approvals::list_approvals()
  -> services::approvals::list_approvals()
  -> SELECT approvals
```

验收结论：最小模块演练通过，链路完整。

## 五、接口返回示例

当前 seed 数据规模由 Rust 测试固定验证：

| 数据域 | 预期数量 | 来源 |
|--------|----------|------|
| projects | 1 | `data/seed/project_agent_swarm.seed.json` |
| agents | 6 | `data/seed/project_agent_swarm.seed.json` |
| tasks | 4 | `data/seed/project_agent_swarm.seed.json` |
| approvals | 3 | `data/seed/project_agent_swarm.seed.json` |

返回对象形态：

```text
ProjectSummary: id, name, status, phase
AgentSummary: id, project_id, name, role, status, model, permissions, created_at, updated_at
TaskSummary: id, project_id, title, description, status, priority, assigned_agent_id, depends_on, risk_level, created_at, updated_at
ApprovalSummary: id, project_id, task_id, request_agent_id, target_service, operation_types, status, risk_level, reason, reject_reason, approved_at, rejected_at, created_at, updated_at
```

`TaskStatus` 已和 `docs/api-draft.md` / `docs/backend-design.md` 对齐，不再保留旧前端假数据里的 `review` 状态。

验收结论：接口返回对象满足当前 Overview 只读展示需要。

## 六、框架复用边界

- 前端使用 React + TypeScript + Vite + Ant Design。
- 前端只通过 `@tauri-apps/api/core` 的 `invoke` 调用桌面宿主。
- 桌面宿主使用 Tauri 2 + Rust。
- SQLite 访问使用 `rusqlite`，当前不引入 ORM 或额外 migration 框架。
- commands 层不写业务规则，不直接拼 UI 数据。
- services 层负责只读查询的业务返回对象。
- db 层负责数据库初始化、migration、seed 和连接状态。

验收结论：当前复用边界清楚，没有把前端、宿主、数据库责任混在一起。

## 七、运行证据包

本次验收已运行并通过以下命令：

```powershell
cd F:\projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\projects\agent-swarm\apps\desktop\src-tauri
cargo check
cargo test

cd F:\projects\agent-swarm
git diff --check
git status --short --ignored
```

验收关注点：

- `npm run typecheck` 证明共享 UI 类型正确。
- `npm run build` 证明共享 UI 可以生产构建。
- `cargo check` 证明 Rust/Tauri 后端可编译。
- `cargo test` 证明 SQLite 初始化、seed 幂等和只读查询可运行。
- `git diff --check` 证明无空白错误。
- `git status --short --ignored` 用于确认未提交的生成物、受保护路径仍保持 ignored，不进入提交。

本次结果：

| 命令 | 结果 |
|------|------|
| `npm run typecheck` | 通过 |
| `npm run build` | 通过 |
| `cargo check` | 通过 |
| `cargo test` | 通过，4 个测试全部通过 |
| `git diff --check` | 通过 |
| `git status --short --ignored` | 仅显示本次文档变更和 ignored 生成物 / 保护路径 |

环境备注：当前 PowerShell 会话未自动带上 Rust 的 cargo 路径，验收时临时加入 `C:\Users\zmd\.cargo\bin` 后执行 Rust 命令。

## 八、未完成项

以下内容本次不做，需后续单独设计和验收：

- `create_task`
- `update_task_status`
- `create_approval`
- `approve_approval`
- `reject_approval`
- `patch_only_approval`
- 写入状态机校验
- 审批规则和二次确认
- 写入审计记录
- Agent Run 新架构迁移
- KnowledgeDoc / GitCheckpoint / Settings / PipelineRun 迁移

## 九、验收结论

只读骨架通过正式验收，可以进入写入 commands 和审批状态机的设计阶段。

进入下一阶段时仍必须遵守：

- 不开放真实 Runner。
- 不开放真实模型调用。
- 不导入 provider SDK。
- 不读取或返回 raw key。
- 不绕过审批写用户项目文件。
- 数据库结构变更必须走 migration。
