# agent蜂群

单人自用的本地桌面工具。桌面端主入口，Web 只做辅助预览。当前处于重新立项讨论中。

## 当前状态

- 当前阶段：MVP-0.4 已验收；阶段 2 真实模型调用准入设计已收口；阶段 3 Agent Run 记录链已收口为本地 Mock / SQLite 流程。
- 目录架构已确认：`packages/ui` 作为唯一 UI 源，`apps/desktop` 作为主入口，`apps/web` 作为辅助预览入口。
- 明确不做：真实 Runner 执行、真实模型调用、云同步、完整权限系统。

## 先读这些

- [AGENTS.md](./AGENTS.md)
- [docs/README.md](./docs/README.md)
- [dev-docs/README.md](./dev-docs/README.md)

## 当前 MVP-0.3 / MVP-0.4

用户输入项目想法后，工作流页会生成本地确定性 `project_plan` 审批草案。审批通过后，系统会自动拆成五个 queued 任务，分别分配给 `agent_frontend`、`agent_backend`、`agent_qa`、`agent_docs`、`agent_reviewer`，并生成五条只读 Runner request queue 记录。

在 MVP-0.4 中，execution request 审查视图、生命周期流转和 runtime events 审计闭环已经完成。Mock / SQLite flow 都覆盖该链路。

阶段 2 已完成准入设计和 helper-only 写入 / 迁移草案：`model_calls` 记录结构、Model Gateway 未来正式入口、provider config resolver、redaction helper 和禁用态 route 草案均已实现，但仍不建表、不写 `model_calls`、不接 provider、不调用真实模型。

这些链路仍然不调用真实模型，不执行真实 Runner，不写本地项目文件，不改 Git。

## 本地运行

SQLite 本地试用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-local.ps1
```

状态：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/status-local.ps1
```

停止：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/stop-local.ps1
```

开发 Mock：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-dev.ps1
```

## 验证

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify-project-plan-flow.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-sqlite-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-agent-config-safety-loop.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-model-gateway.ps1
powershell -ExecutionPolicy Bypass -File scripts/verify-real-model-admission.ps1
```

更多验收入口见 [docs/README.md](./docs/README.md) 和 [scripts/README.md](./scripts/README.md)。
