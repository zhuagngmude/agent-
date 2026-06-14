# 阶段 20：Agent Run 记录视图只读迁移设计

日期：2026-06-14

本文是阶段 20 的设计文档，定义 Agent Run 只读记录视图的表结构、Rust command、共享类型和 UI 页面结构。本文只做设计，不写实现代码。实现由阶段 20 后续批次执行。

## 一、目标

把旧项目中已有的 Agent Run 本地记录链搬到新架构中展示。纯只读，不创建新 Agent Run、不触发真实 Agent、不调用模型、不写文件。

## 二、不做

- 不创建 `POST /agent-run-requests`（不新增写入 command）
- 不实现旧项目中的 `createAgentRunChain`
- 不实现失败注入（`simulateFailureRole`）
- 不触发真实 Agent 执行
- 不调用真实模型（不发 provider 请求）
- 不启用 Runner
- 不写用户文件
- 不操作 Git
- 不修改 SQLite 现有 4 张核心表

## 三、数据表设计

### 3.1 migration 002_add_agent_runs.sql

新增两表：`agent_runs` 和 `runtime_events`。字段定义参照旧 Python SQLite schema（`scripts/sqlite/sqlite_write.py`）和数据模型草案（`docs/data-model-draft.md`），旧 Python script 只作为字段设计参考，不作为新架构运行依赖。

```sql
-- 002_add_agent_runs.sql
-- Agent Run 记录链（只读展示，不在此阶段新增记录）

CREATE TABLE IF NOT EXISTS agent_runs (
  id              TEXT PRIMARY KEY,
  project_id      TEXT NOT NULL,
  chain_id        TEXT NOT NULL,
  root_run_id     TEXT NOT NULL,
  parent_run_id   TEXT,
  sequence        INTEGER NOT NULL,
  role            TEXT NOT NULL,
  agent_id        TEXT,
  agent_name      TEXT NOT NULL,
  model           TEXT NOT NULL,
  status          TEXT NOT NULL,
  input_summary   TEXT,
  output_summary  TEXT,
  token_usage     TEXT NOT NULL,
  cost_estimate   TEXT NOT NULL,
  error_category  TEXT,
  error_message   TEXT,
  requested_by    TEXT NOT NULL,
  chain_label     TEXT,
  created_at      TEXT NOT NULL,
  started_at      TEXT,
  completed_at    TEXT,
  failed_at       TEXT,
  updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_project_id ON agent_runs(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_chain_id ON agent_runs(chain_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status);

-- 运行时审计事件（纯只读，agent_run 详情中展示）

CREATE TABLE IF NOT EXISTS runtime_events (
  id            TEXT PRIMARY KEY,
  project_id    TEXT NOT NULL,
  entity_type   TEXT NOT NULL,
  entity_id     TEXT NOT NULL,
  event_type    TEXT NOT NULL,
  before_state  TEXT,
  after_state   TEXT,
  actor         TEXT,
  reason        TEXT,
  created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runtime_events_project_id ON runtime_events(project_id);
CREATE INDEX IF NOT EXISTS idx_runtime_events_entity_id ON runtime_events(entity_id);
```

**字段来源说明：**
- 旧 Python schema 定义了完整字段集合（`sqlite_write.py` 第454-489行）
- 旧 API 中 `agent-runs.js` 定义了角色目录、状态枚举和链生成逻辑
- `docs/data-model-draft.md` 第145-153行定义了 `agent_runs` 表的关键约束
- `token_usage` 和 `cost_estimate` 存 JSON 文本，前端展示时做格式化

### 3.2 种子数据

在 `db/mod.rs` 的 seed 流程中，从旧种子 JSON（`data/seed/project_agent_swarm.seed.json`）中读取 `agentRuns` 和 `runtimeEvents` 数组（如果存在），插入对应的表。

如果旧种子 JSON 不含这两个数组（当前是这样），则 `agent_runs` 和 `runtime_events` 表初始为空。种子数据补齐留到后续阶段。

### 3.3 状态枚举

**Agent Run 状态**（来自旧 `data.js` 第43-50行）：
```
queued    — 排队中
running   — 进行中
succeeded — 已完成
failed    — 已失败
blocked   — 已阻塞
```

注意：旧数据中的 `agentRun.status` 使用 `succeeded`（不是 `completed`）。新 `packages/agent-core` 的常量应同时包含两者，以兼容旧数据。

**Agent Run 角色**（来自旧 `agent-runs.js` 第1-68行）：
```
architect  — 架构师 Agent
scheduler  — Scheduler
frontend   — 前端 Agent
backend    — Backend Agent
qa         — QA Agent
docs       — 文档 Agent
reviewer   — Reviewer
```

## 四、Rust 层

### 4.1 db/ 层

在 `db/mod.rs` 中，`initialize()` 函数内按序执行 migration：
```rust
// 001_initial_sqlite.sql — 已有
// 002_add_agent_runs.sql — 新增
// 后续 migration 按序追加
```

### 4.2 services/ 层

新建 `services/agent_runs.rs`：

**`list_agent_runs(connection) -> Result<Vec<AgentRunSummary>, String>`**
- 查询 `SELECT * FROM agent_runs WHERE project_id = ? ORDER BY chain_id, sequence`
- 如果表为空则返回空 Vec（不报错）
- 字段映射到 Rust struct `AgentRunSummary`

**`list_runtime_events(connection, entity_type?, entity_id?) -> Result<Vec<RuntimeEventSummary>, String>`**
- 查询 `SELECT * FROM runtime_events WHERE project_id = ?` 加可选筛选
- 支持按 `entity_type = "agent_run"` 和指定 `entity_id` 筛选
- 如果表为空则返回空 Vec

**结构体定义：**
```rust
#[derive(Serialize)]
struct AgentRunSummary {
    id: String, project_id: String, chain_id: String,
    root_run_id: String, parent_run_id: Option<String>,
    sequence: i32, role: String, agent_id: Option<String>,
    agent_name: String, model: String, status: String,
    input_summary: Option<String>, output_summary: Option<String>,
    token_usage: String, cost_estimate: String,
    error_category: Option<String>, error_message: Option<String>,
    requested_by: String, chain_label: Option<String>,
    created_at: String, started_at: Option<String>,
    completed_at: Option<String>, failed_at: Option<String>,
    updated_at: String,
}

#[derive(Serialize)]
struct RuntimeEventSummary {
    id: String, project_id: String, entity_type: String,
    entity_id: String, event_type: String,
    before_state: Option<String>, after_state: Option<String>,
    actor: Option<String>, reason: Option<String>,
    created_at: String,
}
```

### 4.3 commands/ 层

新建 `commands/agent_runs.rs`：

**`#[tauri::command] list_agent_runs(state)`**
- 从 state 提取 db 连接，调用 `services::agent_runs::list_agent_runs`
- 只读，不做任何校验或状态变更
- 参数：无（project_id 从当前项目推导）

**`#[tauri::command] list_runtime_events(state, entity_id?: String)`**
- 调用 `services::agent_runs::list_runtime_events`，默认筛选 `entity_type = "agent_run"`
- 如果传了 `entity_id` 则进一步筛选

### 4.4 注册

在 `services/mod.rs` 添加 `pub mod agent_runs;`，在 `commands/mod.rs` 添加 `pub mod agent_runs;`，在 `lib.rs` 的 `generate_handler!` 中加入 `list_agent_runs` 和 `list_runtime_events`。

## 五、TypeScript 层

### 5.1 packages/shared

新增文件 `src/types/agent-run.ts`：

```ts
export type AgentRunStatus = "queued" | "running" | "succeeded" | "failed" | "blocked";

export type AgentRunSummary = {
  id: string;
  project_id: string;
  chain_id: string;
  root_run_id: string;
  parent_run_id: string | null;
  sequence: number;
  role: string;
  agent_id: string | null;
  agent_name: string;
  model: string;
  status: AgentRunStatus;
  input_summary: string | null;
  output_summary: string | null;
  token_usage: string;     // JSON text
  cost_estimate: string;   // JSON text
  error_category: string | null;
  error_message: string | null;
  requested_by: string;
  chain_label: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  failed_at: string | null;
  updated_at: string;
};

export type RuntimeEventSummary = {
  id: string;
  project_id: string;
  entity_type: string;
  entity_id: string;
  event_type: string;
  before_state: string | null;
  after_state: string | null;
  actor: string | null;
  reason: string | null;
  created_at: string;
};
```

在 `src/index.ts` 中重导出。

### 5.2 packages/agent-core

新增文件 `src/models/agent-run.ts`：

```ts
import type { AgentRunStatus } from "@agent-swarm/shared";

/** Agent Run 状态值列表（与 Rust normalize 对齐的允许值） */
export const AGENT_RUN_STATUS_VALUES: AgentRunStatus[] = [
  "queued", "running", "succeeded", "failed", "blocked",
];

/** Agent Run 角色值列表 */
export const AGENT_RUN_ROLE_VALUES = [
  "architect", "scheduler", "frontend", "backend", "qa", "docs", "reviewer",
] as const;

export type AgentRunRole = (typeof AGENT_RUN_ROLE_VALUES)[number];

/** 终态判断 */
export function isTerminalAgentRunStatus(status: AgentRunStatus): boolean {
  return status === "succeeded" || status === "failed";
}
```

在 `src/index.ts` 中重导出。

### 5.3 packages/ui

**新建 `packages/ui/src/pages/AgentRunsPage.tsx`**

组件 Props：
```ts
type AgentRunsPageProps = {
  agentRuns: AgentRunSummary[];
  runtimeEvents: RuntimeEventSummary[];
};
```

页面结构（使用 Ant Design 组件，不手写 CSS）：
```
Space (page-stack)
  PageHeading
    Title "运行记录"
    Text "Agent Run 链记录与运行时审计事件（只读）"
  Card
    Table
      columns: 链名、请求人、Agent数、状态汇总、创建时间
      expandable: 嵌套 Table 显示该链内各 run 的详细信息
        columns: 序号、角色、Agent、模型、状态(Badge)、输入摘要、输出摘要
        expandable: 关联的 runtime_events 列表
```

**行数据转换：**
- 链列表：按 `chain_id` 分组，计算每组的状态汇总（成功数/失败数/阻塞数）
- 展开行：按 `sequence` 排序展示该 chain 的全部 run
- Runtime events：按 `entity_type = "agent_run"` 且 `entity_id` 匹配筛选

**数据获取：**
- 不调用 `useDesktopHostOverview()`（该 hook 不包含 agent_runs）
- 在 `AgentRunsPage` 内部通过 Tauri invoke 直接调用 `list_agent_runs` 和 `list_runtime_events`
- 浏览器 fallback：返回空数组

**导航集成：**

`mainNavItems.ts` 新增：
```ts
{ key: "agentRuns", label: "运行记录", icon: Activity }  // from lucide-react
```

`PageKey` 类型扩展为：
```ts
export type PageKey =
  | "overview" | "tasks" | "agents" | "approvals" | "settings"
  | "agentRuns";
```

`App.tsx` 中 `renderPage()` 的 switch 新增：
```ts
case "agentRuns":
  return <AgentRunsPage agentRuns={...} runtimeEvents={...} />;
```

AgentRunsPage 的数据获取方式：因为当前 `useDesktopHostOverview` hook 不包含 agent_runs，阶段 20 有两种方案：
- **方案 A（保守）：** AgentRunsPage 内部通过 `invoke("list_agent_runs")` 和 `invoke("list_runtime_events")` 独立获取数据，用 `useState` + `useEffect` 管理。
- **方案 B（统一）：** 扩展 `useDesktopHostOverview` hook 加入 agent_runs，在 App.tsx 统一获取后通过 props 传入。

建议用方案 A（保守），不修改现有 hook 签名，AgentRunsPage 自管理数据加载。

## 六、文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `data/migrations/002_add_agent_runs.sql` | 新增 | agent_runs + runtime_events 表 |
| `apps/desktop/src-tauri/src/db/mod.rs` | 修改 | 执行 002 migration |
| `apps/desktop/src-tauri/src/services/agent_runs.rs` | 新增 | list_agent_runs、list_runtime_events |
| `apps/desktop/src-tauri/src/services/mod.rs` | 修改 | 注册 agent_runs 模块 |
| `apps/desktop/src-tauri/src/commands/agent_runs.rs` | 新增 | Tauri command 薄封装 |
| `apps/desktop/src-tauri/src/commands/mod.rs` | 修改 | 注册 agent_runs 模块 |
| `apps/desktop/src-tauri/src/lib.rs` | 修改 | 注册 2 个新 command |
| `packages/shared/src/types/agent-run.ts` | 新增 | AgentRunSummary、RuntimeEventSummary |
| `packages/shared/src/index.ts` | 修改 | 重导出新类型 |
| `packages/agent-core/src/models/agent-run.ts` | 新增 | 状态常量、角色列表、终态判断 |
| `packages/agent-core/src/index.ts` | 修改 | 重导出新增内容 |
| `packages/ui/src/pages/AgentRunsPage.tsx` | 新增 | 运行记录页面 |
| `packages/ui/src/routes/mainNavItems.ts` | 修改 | 新增 agentRuns 导航项 |
| `packages/ui/src/app/App.tsx` | 修改 | 新增 agentRuns 路由分支 |

不改动：
- `desktopHost.ts` — 不扩展 hook
- `OverviewPage.tsx` / `TasksPage.tsx` 等已有页面
- Rust services 层 `tasks.rs`、`approvals.rs` 等
- SQLite 现有 4 张核心表

## 七、验证

```powershell
cd packages/ui; npm run typecheck; npm run build
cd ..\..\apps\desktop\src-tauri; cargo check; cargo test
git diff --check
```

额外验证点：
- 新 migration SQL 语法正确（SQLite 兼容）
- `agent_runs` 和 `runtime_events` 表在首次启动后存在（`sqlite3 .tables`）
- Migration 可重复执行（`CREATE TABLE IF NOT EXISTS`）
- AgentRunsPage 在浏览器模式下不崩溃（空数组 fallback）

## 八、边界约束

- 不新增 npm 依赖
- 不新增 Rust crate 依赖
- 不修改 `packages/ui/package.json`
- 不创建根目录 workspace
- 不新增写入 Tauri command
- 不修改现有 4 张核心表
- 不接真实模型、Runner、Git
- 不使用旧 Python SQLite 脚本

## 九、提交计划

```
commit 1: docs: 设计阶段20 Agent Run 只读迁移
commit 2: feat: Agent Run 记录视图只读迁移
```

## 十、参考

- [阶段19-冻结模块解冻评估](./阶段19-冻结模块解冻评估.md) — 选型依据
- [阶段17-长期分层边界设计](./阶段17-长期分层边界设计.md) — 分层边界和部分解冻条件
- [data-model-draft](../docs/data-model-draft.md) — agent_runs 表设计约束
- `scripts/sqlite/sqlite_write.py` — 旧 Python schema（字段参考）
- `services/api/agent-runs.js` — 旧业务逻辑（语义参考）
