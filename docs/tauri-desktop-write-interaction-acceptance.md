# Tauri 桌面写入交互验收

日期：2026-06-14

本文验收第 13 步"前端共享 UI 接入写入 commands"的完整链路：React UI → Tauri invoke → Rust command → service → db → SQLite。验收范围只包含本地 SQLite 记录写入和状态流转，不开放真实 Runner、真实模型调用、文件写入、Git 修改或云同步。

## 一、验收范围

| 交互 | 前端位置 | Rust command | 验收结论 |
|------|----------|-------------|----------|
| 新建任务 | `CreateTaskModal.tsx` → `desktopHost.createTask()` | `create_task` | 通过 |
| 任务状态变更 | `OverviewPage` 操作列 → `desktopHost.updateTaskStatus()` | `update_task_status` | 通过 |
| 审批通过 | `OverviewPage` 操作列 → `desktopHost.approveApproval()` | `approve_approval` | 通过 |
| 审批拒绝 | `OverviewPage` 操作列 → `desktopHost.rejectApproval()` | `reject_approval` | 通过 |
| 审批仅补丁 | `OverviewPage` 操作列 → `desktopHost.patchOnlyApproval()` | `patch_only_approval` | 通过 |

相关提交：

```text
e0b92ef docs: 设计前端写入 commands 接入
e7b77bb feat: 封装前端写入 commands 调用
7cd9b02 feat: 接入 Overview 写入交互
```

## 二、分层验收

| 层级 | 要求 | 验收结果 |
|------|------|----------|
| `packages/ui/src/utils/desktopHost.ts` | 封装写入函数，页面不直接调 invoke | 通过 |
| `packages/ui/src/components/CreateTaskModal.tsx` | Modal + Form，含必填校验和浏览器降级 | 通过 |
| `packages/ui/src/pages/OverviewPage.tsx` | 操作列、Popconfirm 防误触、浏览器降级 | 通过 |
| `packages/ui/src/main.tsx` | 包裹 `<AntdApp>` 支持 `useApp()` 上下文 | 通过 |
| `commands/` | 只接收参数，调用 service | 通过（前一轮已验收） |
| `services/` | 输入校验、状态机、关联检查 | 通过（前一轮已验收） |
| `db/` | 参数化 SQL + 事务 | 通过（前一轮已验收） |

## 三、交互链路验收

### 3.1 新建任务

```
OverviewPage "+ 新建任务" 按钮
  → CreateTaskModal (Modal + Form)
  → desktopHost.createTask(input)
  → invoke("create_task", { input })
  → commands::tasks::create_task
  → services::tasks::create_task
  → INSERT INTO tasks (status = queued)
  → 返回 TaskSummary
  → message.success("任务已创建")
  → refresh() 刷新列表
```

验收点：
- Modal 表单含 title（必填）、description（选填）、priority（Radio）、assigned_agent_id（Select）、risk_level（Radio）
- Form 内置 maxLength 校验（title 120、description 2000）
- 提交失败时 `message.error` 展示 Rust 错误消息
- 提交成功后关闭 Modal、清空表单、刷新数据

### 3.2 任务状态变更

```
OverviewPage 任务操作列
  → Popconfirm 确认
  → desktopHost.updateTaskStatus({ id, status })
  → invoke("update_task_status", { input })
  → services::tasks::update_task_status
    → 校验状态机
    → UPDATE tasks SET status, updated_at
  → 返回 TaskSummary
  → message.success("任务状态已更新")
  → refresh() 刷新列表
```

验收点：
- `queued` 行显示 "开始"、"取消"
- `running` 行显示 "完成"、"阻塞"、"等待用户"、"失败"、"取消"
- `blocked` 行显示 "恢复"
- `waiting_user` 行显示 "恢复"、"取消"
- 终态行（completed/failed/cancelled）不显示操作按钮
- 每个按钮有 Popconfirm 防误触
- 失败/取消按钮标红（danger）

### 3.3 审批操作

```
OverviewPage 审批操作列
  → Popconfirm 确认
  → desktopHost.approveApproval(id) / rejectApproval(id) / patchOnlyApproval(id)
  → invoke("approve_approval", ...) / invoke("reject_approval", ...) / invoke("patch_only_approval", ...)
  → services::approvals::transition_approval
  → UPDATE approvals SET status, ...
  → 返回 ApprovalSummary
  → message.success(...)
  → refresh() 刷新列表
```

验收点：
- 只有 `pending` 行显示 "通过"、"拒绝"、"仅补丁" 按钮
- 终态行（approved/rejected/patch_only）不显示操作按钮
- 每个按钮有 Popconfirm 防误触
- 通过按钮绿色（type="primary"）、拒绝按钮红色（danger）

### 3.4 数据刷新

验收点：
- `useDesktopHostOverview` 返回 `refresh` 函数
- 每次写入成功后自动调用 `refresh()`
- `refresh` 使用 `refreshKey` 计数器触发 `useEffect` 重新查询
- 统计卡片自动随数据更新

## 四、浏览器预览降级验收

| 场景 | 预期行为 | 验收结果 |
|------|----------|----------|
| 新建任务按钮 | 显示但 disabled + tooltip | 通过 |
| CreateTaskModal 打开 | 触发 `message.warning("浏览器预览模式不支持写入操作")` | 通过 |
| 任务操作列 | 不渲染（`showWriteUI = false`） | 通过 |
| 审批操作列 | 不渲染（`showWriteUI = false`） | 通过 |
| 只读数据 | fallback 数据正常展示 | 通过（前一轮已验收） |
| 页面崩溃 | 不崩溃、不白屏 | 通过 |

实现方式：
- `isTauriHost()` 从 `desktopHost.ts` 导出
- OverviewPage 通过 `showWriteUI` 控制是否渲染写入交互
- 写入函数内置 `requireTauri()` 守卫，浏览器模式下抛出明确错误

## 五、副作用边界验收

前端写入接入不引入新的副作用。已确认：

- 前端只通过 `desktopHost.ts` 中的封装函数调用 Tauri invoke
- 不绕过封装直接写 `invoke(...)` 
- 不访问 SQLite 文件系统
- 不执行本地命令
- 不写用户项目文件
- 不修改 Git
- 不调用真实模型
- 不读取或返回 raw key

前端侧特有的安全边界：
- 表单校验优先在前端执行（maxLength、required），Rust 层再二次校验
- 状态转换按钮基于当前状态动态渲染，不提供非法转换入口
- Popconfirm 防止误操作
- 错误消息直接展示 Rust 返回的稳定错误语义，不额外解析或暴露内部信息

## 六、文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `docs/frontend-write-commands-design.md` | 新增 | 前端接入设计文档 |
| `packages/ui/src/utils/desktopHost.ts` | 修改 | 导出 isTauriHost、新增 5 个写入函数、refresh |
| `packages/ui/src/components/CreateTaskModal.tsx` | 新增 | 新建任务 Modal |
| `packages/ui/src/pages/OverviewPage.tsx` | 修改 | 操作列、新建按钮、浏览器降级 |
| `packages/ui/src/main.tsx` | 修改 | 包裹 `<AntdApp>` |

未修改：
- Rust 侧无变更（写入命令在前一轮已验收）
- `StatusBadge.tsx` 无变更
- Agent 编排表格无变更

## 七、验证命令与结果

执行位置：`F:\projects\agent-swarm\packages\ui`

```powershell
npm run typecheck
```

结果：

```text
tsc -b --pretty false
通过，无错误输出
```

执行位置：`F:\projects\agent-swarm\packages\ui`

```powershell
npm run build
```

结果：

```text
tsc -b && vite build
✓ built in 752ms
```

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
```

结果：

```text
通过，无输出
```

## 八、结论

前端共享 UI 写入交互接入通过验收。6 个写入 command 中 5 个已接入 UI（`create_approval` 暂不暴露），完整链路从 React UI → Tauri invoke → Rust command → SQLite 均可验证。

可以进入下一阶段：旧原型页面迁移或独立页面拆分。

进入下一阶段前仍必须保持以下边界：

- 不开放真实 Runner。
- 不调用真实模型。
- 不写用户项目文件。
- 不修改 Git。
- 不把旧 Node.js HTTP API 继续作为正式后端扩展。
- 不继续在旧手搓前端上叠新功能。
- 数据库结构变更必须走 migration。
