# 本地 Demo 启动与验证清单

日期：2026-06-09

用途：给人类用户和后续 AI 一个可重复的本地验收入口。当前仍是 MVP-0.2 Mock 阶段，不接真实数据库、不调用真实模型、不让 Runner 执行本地命令。

## 1. 启动

在项目根目录执行：

```powershell
cd F:\projects\agent-swarm
powershell -ExecutionPolicy Bypass -File scripts\start-dev.ps1
```

脚本会做三件事：

1. 检查 `http://127.0.0.1:8787/api/health` 是否可用。
2. 如果 Mock API 未启动，则后台启动 `services/api/server.js`。
3. 打开 `apps/web/index.html`。

如果启动失败，先看日志：

```text
logs/mock-api.out.log
logs/mock-api.err.log
```

## 2. 快速健康检查

Mock API 默认地址：

```text
http://127.0.0.1:8787
```

浏览器或 PowerShell 可检查：

```powershell
Invoke-RestMethod http://127.0.0.1:8787/api/health
Invoke-RestMethod http://127.0.0.1:8787/api/projects/project_agent_swarm/dashboard
```

预期：

- `/api/health` 返回 `ok=true`。
- Dashboard 返回 `project`、`metrics`、`pendingApprovals`、`taskQueue`、`agentStatus`、`runnerStatus`。
- Web App 顶部显示 Mock API 已连接；如果 API 不可用，前端会回退到本地 `data.js`。

## 3. 页面验收点

推荐按这个顺序点一遍：

1. 首页：能看到项目阶段、指标卡、审批/任务摘要。
2. 审批页：能点选审批记录，查看风险、影响文件、diff 预览；按钮只改变 Mock 状态。
3. 任务页：能开始、完成、失败、取消任务；状态写入本地 runtime state。
4. 工作流页：只读展示工作流步骤、节点和依赖，不支持编辑或运行。
5. 运行与调度页：能查看 Runner job 队列、Runner 状态、权限边界和“不会执行本地命令”的安全说明。
6. 智能体页：能查看 Agent 详情、子 Agent 关系、配置变更预览、审批申请、待应用记录、Mock 应用/取消、应用审计和回滚前审查。
7. 设置页：能导出、重置、清理本地 Mock 运行态。

## 4. Mock 状态重置

运行态文件：

```text
data/mock/runtime-state.json
```

注意：

- 这个文件是本地运行文件，不进入 Git。
- 删除它或在设置页清理状态，会回到初始 Mock 数据。
- 不要把它提交。

也可以调用：

```powershell
Invoke-RestMethod -Method Post http://127.0.0.1:8787/api/runtime-state/reset
```

## 5. 自动验证 Mock 状态流转

可以运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
```

脚本会验证：

- Dashboard 聚合接口包含 Runner 状态。
- 任务可以 `start -> complete`。
- Runner 审批通过后只生成只读 Runner job。
- Agent 配置审批后可以走 Mock 应用状态流转。
- Agent 配置审批后可以走 Mock 取消状态流转。

脚本结束时会重置本地 runtime state，避免留下测试状态。

## 6. 当前安全边界

当前 Demo 允许：

- 读取 Mock API 数据。
- 把审批、任务、Agent 配置应用记录的状态写入本地 runtime state。
- 展示 Runner job、Runner 状态和 Agent 配置审查信息。

当前 Demo 不允许：

- 不会真实修改 Agent 配置。
- 不会让 Runner 写文件、删文件、执行命令、发起网络请求或修改 Git。
- 不会调用真实模型 API。
- 不会连接真实数据库或云同步。

如果后续要开放真实 Runner 或真实 Agent 配置写入，必须先补 Approval Service、二次确认、Git checkpoint 和回滚策略。
