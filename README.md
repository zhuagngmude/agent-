# agent蜂群

多 AI 智能体协作控制台，当前以本地 Mock / SQLite 试用为主。

## 当前状态

- 当前阶段：MVP-0.3
- 当前模式：Web App + Mock/SQLite 状态机 + 项目计划审批原型
- 明确不做：真实 Runner 执行、真实模型调用、云同步、完整权限系统

## 先读这些

- [AGENTS.md](./AGENTS.md)
- [docs/README.md](./docs/README.md)
- [dev-docs/README.md](./dev-docs/README.md)

## 当前 MVP-0.3

用户输入项目想法后，工作流页会生成本地确定性 `project_plan` 审批草案。审批通过后，系统会自动拆成五个 queued 任务，分别分配给 `agent_frontend`、`agent_backend`、`agent_qa`、`agent_docs`、`agent_reviewer`，并生成五条只读 Runner request queue 记录。

这条链路仍然不调用真实模型，不执行真实 Runner，不写本地项目文件，不改 Git。

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
```

更多验收入口见 [docs/README.md](./docs/README.md) 和 [scripts/README.md](./scripts/README.md)。
