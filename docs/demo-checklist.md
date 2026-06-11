# 本地 Demo 启动与验证清单

当前用途：给人类和后续 AI 一个可重复的本地验收入口。当前仍是 Mock / SQLite 优先阶段，不调用真实模型，不执行真实 Runner。

## 启动

```powershell
cd F:\projects\agent-swarm
powershell -ExecutionPolicy Bypass -File scripts\start-local.ps1
```

- API 默认在 `http://127.0.0.1:8787`
- Web 默认在 `http://127.0.0.1:5175`

## 停止与状态

```powershell
powershell -ExecutionPolicy Bypass -File scripts\status-local.ps1
powershell -ExecutionPolicy Bypass -File scripts\stop-local.ps1
```

## 验证顺序

1. `verify-project-plan-flow.ps1` 先确认项目计划审批与只读 Runner 队列的 helper 约束。
2. `verify-mock-flows.ps1` 再检查 Mock 流程和安全边界。
3. `verify-sqlite-flows.ps1` 再检查 SQLite 流程和持久化边界。
4. `verify-agent-config-safety-loop.ps1` 再确认 Agent 配置 apply / version / rollback 闭环。
5. `verify-model-gateway.ps1` 再确认 Model Gateway 仍是禁用态和只读边界。
6. `verify-real-model-admission.ps1` 再确认阶段 2 真实模型准入预备层仍不调用 provider、不写模型记录、不创建审批 / 任务 / Runner request。
7. `verify-local-ui.ps1` 最后做浏览器烟测。

## 现场检查点

- Dashboard 能看到项目、任务、审批、Runner 状态和本地试用状态。
- 项目计划审批通过后，只生成五个 queued 任务和五条只读 Runner request 记录。
- Agent config 审批通过后，默认只会进入 `pending_apply`，不会直接改配置，不会生成 Runner job。
- rollback 预检只返回只读 diff，不创建审批，不写入运行时状态。
- `project-plan-model-requests` 目前只返回禁用态草案响应，不调用真实模型，不写 `model_calls`，不创建审批、任务或 Runner request。
- 页面控制台保持 0 errors / 0 warnings。

## 不做的事

- 不开放真实 Runner 执行。
- 不开放真实模型调用。
- 不做云同步。
- 不启用完整权限系统。
