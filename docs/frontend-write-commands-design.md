# 前端写入 Commands 接入设计

日期：2026-06-14

本文是第 13 步的设计文档，定义前端如何最小化接入已验收的写入 commands，不重写整套页面。

## 一、总边界

- 所有写入调用封装在 `packages/ui/src/utils/desktopHost.ts`，页面不直接写 `invoke("create_task", ...)`。
- 浏览器预览模式下写入功能降级为 `message.warning` 提示，不崩溃。
- 所有 UI 使用 Ant Design 组件，不手搓。
- 写入成功后自动刷新数据。
- 第一版不实现创建审批的 UI（审批由 Rust 层或后续业务触发），只做任务创建、状态变更、审批操作。

## 二、封装函数（desktopHost.ts 新增）

所有函数签名返回 `Promise<T>`，调用方用 try/catch + message 处理错误。

第一版前端只封装并暴露 5 个 UI 会使用的写入函数：`createTask`、`updateTaskStatus`、`approveApproval`、`rejectApproval`、`patchOnlyApproval`。Rust 侧已验收的 `create_approval` 暂不暴露 UI，由后续业务流程触发时再接入。

### 2.1 createTask

```ts
import { invoke } from "@tauri-apps/api/core";

export type CreateTaskInput = {
  title: string;
  description?: string | null;
  priority: "low" | "medium" | "high";
  assigned_agent_id?: string | null;
  depends_on?: string[];
  risk_level?: "low" | "medium" | "high" | null;
};

export async function createTask(input: CreateTaskInput): Promise<{ task: TaskSummary }> {
  return invoke("create_task", { input });
}
```

### 2.2 updateTaskStatus

```ts
export type UpdateTaskStatusInput = {
  id: string;
  status: TaskStatus;
};

export async function updateTaskStatus(input: UpdateTaskStatusInput): Promise<{ task: TaskSummary }> {
  return invoke("update_task_status", { input });
}
```

### 2.3 approveApproval / rejectApproval / patchOnlyApproval

```ts
export async function approveApproval(id: string): Promise<{ approval: ApprovalSummary }> {
  return invoke("approve_approval", { input: { id } });
}

export async function rejectApproval(id: string, rejectReason?: string | null): Promise<{ approval: ApprovalSummary }> {
  return invoke("reject_approval", { input: { id, reject_reason: rejectReason ?? null } });
}

export async function patchOnlyApproval(id: string): Promise<{ approval: ApprovalSummary }> {
  return invoke("patch_only_approval", { input: { id } });
}
```

### 2.4 isTauriHost 守卫

每个函数内部先检查 `isTauriHost()`，浏览器模式下直接 throw 一个可展示的错误：

```ts
function requireTauri(): void {
  if (!isTauriHost()) {
    throw new Error("当前运行在浏览器预览模式，写入操作不可用。请启动 Tauri 桌面宿主。");
  }
}
```

### 2.5 刷新机制

`useDesktopHostOverview` 新增 `refresh` 函数，写入成功后调用刷新数据：

```ts
export function useDesktopHostOverview(): DesktopHostOverviewState & { refresh: () => void } {
  // 现有逻辑不变
  // 新增 refresh 函数，重新执行 Promise.all 查询
}
```

## 三、UI 交互设计

### 3.1 任务区：新增任务按钮 → create_task

位置：任务 Card 标题栏右侧，加一个 `+ 新建任务` 按钮。

交互：
1. 点击按钮 → 打开 Modal
2. Modal 内含 Form：
   - `title`：Input，必填，maxLength 120
   - `description`：Input.TextArea，选填，maxLength 2000
   - `priority`：Radio.Group（低/中/高），默认 "medium"
   - `assigned_agent_id`：Select，选填，选项来自当前 agents 列表
   - `risk_level`：Radio.Group（低/中/高），选填
   - `depends_on`：第一版不做 UI（选填，后续再加）
3. 提交时调用 `createTask(input)`：
   - 成功 → `message.success("任务已创建")` → 关闭 Modal → `refresh()`
   - 失败 → `message.error(errorMessage)`

组件：Ant Design `Modal` + `Form` + `Input` + `Radio.Group` + `Select` + `message`

### 3.2 任务状态：状态操作按钮 → update_task_status

位置：任务表格每行末尾新增 "操作" 列。

交互：
1. 每行根据当前 status 显示可用的状态转换按钮：
   - `queued`：显示 "开始"、"取消"
   - `running`：显示 "完成"、"阻塞"、"等待用户"、"失败"、"取消"
   - `blocked`：显示 "恢复"
   - `waiting_user`：显示 "恢复"、"取消"
   - `completed` / `failed` / `cancelled`：不显示操作（终态）
2. 每个按钮用 `Popconfirm` 包裹，防止误操作
3. 点击确认后调用 `updateTaskStatus({ id, status })`：
   - 成功 → `message.success("状态已更新")` → `refresh()`
   - 失败 → `message.error(errorMessage)`

组件：Ant Design `Button` + `Popconfirm` + `message`

状态映射表（中文展示）：

```ts
const statusActionLabels: Record<string, Array<{ status: TaskStatus; label: string }>> = {
  queued: [
    { status: "running", label: "开始" },
    { status: "cancelled", label: "取消" },
  ],
  running: [
    { status: "completed", label: "完成" },
    { status: "blocked", label: "阻塞" },
    { status: "waiting_user", label: "等待用户" },
    { status: "failed", label: "失败" },
    { status: "cancelled", label: "取消" },
  ],
  blocked: [
    { status: "running", label: "恢复" },
  ],
  waiting_user: [
    { status: "running", label: "恢复" },
    { status: "cancelled", label: "取消" },
  ],
};
```

### 3.3 审批操作：通过/拒绝/仅补丁 → approve/reject/patch_only

位置：审批表格每行末尾新增 "操作" 列。

交互：
1. 只有 `status === "pending"` 的行显示操作按钮
2. 三个按钮：
   - "通过"（绿色）→ `Popconfirm` → `approveApproval(id)`
   - "拒绝"（红色）→ `Popconfirm` → 可填拒绝原因 → `rejectApproval(id, reason)`
   - "仅补丁"（默认色）→ `Popconfirm` → `patchOnlyApproval(id)`
3. 成功 → `message.success(...)` → `refresh()`
4. 失败 → `message.error(errorMessage)`

拒绝原因：`Popconfirm` 内部加一个小的 `Input` 让用户填写（选填，maxLength 2000）。如果 Popconfirm 放不下，改用点击后弹出简易 Modal。

第一版简化处理：拒绝按钮使用 `Popconfirm`，不要求填写原因（Rust 层 `reject_reason` 可为空）。

组件：Ant Design `Button` + `Popconfirm` + `message`

## 四、浏览器预览降级

浏览器预览模式下（`isTauriHost() === false`）：

- 新增任务按钮仍然显示，但点击后 `message.warning("浏览器预览模式不支持写入操作")`
- 状态操作和审批操作按钮不显示（或显示但 disabled + tooltip）
- 不崩溃，不白屏

实现方式：
- `desktopHost.ts` 导出 `isTauriHost()`（目前是私有函数，改为导出）
- OverviewPage 根据 `isTauriHost()` 决定是否渲染写入交互

## 五、OverviewPage 改动范围

| 区域 | 改动 |
|------|------|
| 任务 Card 标题栏 | 新增 `+ 新建任务` 按钮 |
| 任务表格 | 新增 `操作` 列，含状态转换按钮 |
| 审批表格 | 新增 `操作` 列，含通过/拒绝/仅补丁按钮 |
| 数据刷新 | `useDesktopHostOverview` 新增 `refresh` 返回 |
| 连接状态卡片 | 无改动 |

不改动的区域：
- Agent 编排表格（Agent 的创建/更新暂不开放）
- 统计卡片（自动随 refresh 更新）
- 连接状态卡片

## 六、错误展示统一策略

所有写入操作的错误通过 `message.error(message)` 展示：

```ts
function showWriteError(error: unknown): void {
  const messageText = error instanceof Error ? error.message : String(error);
  message.error(messageText);
}
```

Rust 层返回的错误消息已经稳定：`invalid_input: ...`、`not_found: ...`、`invalid_transition: ...`、`database_error: ...`。前端不额外解析，直接展示给用户即可。

## 七、类型共享

`CreateTaskInput` 和 `UpdateTaskStatusInput` 在前端定义一份，与 Rust 层的 `CreateTaskInput` / `UpdateTaskStatusInput` 结构保持一致。字段名使用 snake_case（Tauri invoke 的 JSON 序列化约定）。

后续如果共享类型增多，可以在 `packages/shared` 中统一维护 JSON Schema 或 TypeScript 类型源。第一版各自定义即可。

## 八、不做的

- 不新建独立页面
- 不实现 `create_approval` 的 UI（审批由后续业务触发）
- 不做 `depends_on` 选择器（字段保留但不暴露 UI）
- 不做批量操作
- 不做撤销
- 不做乐观更新（先写后刷新）
- 不修改 Agent 编排表格
- 浏览器预览模式下不写 fallback 假数据

## 九、提交计划

三个小提交，按顺序：

### 提交 1：docs: 设计前端写入 commands 接入

- 新增 `docs/frontend-write-commands-design.md`

### 提交 2：feat: 封装前端写入 commands 调用

- 修改 `packages/ui/src/utils/desktopHost.ts`
  - 导出 `isTauriHost`
  - 新增 5 个前端写入函数
  - `useDesktopHostOverview` 新增 `refresh`

### 提交 3：feat: 接入 Overview 写入交互

- 修改 `packages/ui/src/pages/OverviewPage.tsx`
  - 新建任务 Modal + Form
  - 任务状态操作列
  - 审批操作列
- 可能新增 `packages/ui/src/components/CreateTaskModal.tsx`
- 修改 `packages/ui/src/components/StatusBadge.tsx`（如需）

每个提交后验证：
```powershell
cd F:\Projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo check
cargo test
```
