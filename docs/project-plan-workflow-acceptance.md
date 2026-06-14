# 阶段 24 project_plan / Workflow 最小闭环验收

日期：2026-06-15

## 验收范围

阶段 24 已把旧 MVP-0.3 的项目计划闭环迁入新 Tauri/Rust 主线：

```text
项目想法
-> 本地确定性 project_plan 草案
-> pending 审批
-> approve_project_plan 二次确认
-> 5 个 queued 任务
-> 5 条只读 runner_requests
-> runtime_events 审计
-> ProjectPlanPage 展示
```

## 本阶段仍然不做

- 不调用真实模型。
- 不导入 provider SDK。
- 不发 provider HTTP 请求。
- 不读取 raw key。
- 不启用真实 Runner。
- 不执行命令。
- 不写用户项目文件。
- 不修改 Git。

## 交付物

- `data/migrations/004_add_project_plan_workflow.sql`
- `apps/desktop/src-tauri/src/services/project_plan.rs`
- `apps/desktop/src-tauri/src/commands/project_plan.rs`
- `packages/shared/src/types/project-plan.ts`
- `packages/agent-core/src/models/project-plan.ts`
- `packages/ui/src/pages/ProjectPlanPage.tsx`

## 验收结论

- 创建草案只写 `project_plan_drafts` 和 `approvals`。
- 创建草案不会写 `tasks` 或 `runner_requests`。
- 通用 `approve_approval` 会拒绝通过 `project_plan` 审批，不能绕过二次确认。
- `approve_project_plan` 必须二次确认。
- `approve_project_plan` 只对 `target_service=project_plan` 生效。
- 审批通过后确定性创建 5 个任务和 5 条只读 `runner_requests`。
- 二次批准保持幂等，不重复创建任务或队列记录。
- `runtime_events` 只记录本地审计事件。

## 验证命令

```powershell
cd apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test
```

```powershell
cd packages\ui
npm run typecheck
npm run build
```

```powershell
cd packages\shared
..\ui\node_modules\.bin\tsc.cmd -p tsconfig.json --noEmit

cd ..\agent-core
..\ui\node_modules\.bin\tsc.cmd -p tsconfig.json --noEmit
```

```powershell
git diff --check
```
