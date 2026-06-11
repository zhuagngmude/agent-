# agent蜂群 AI 开发维护手册

这份文档给后续 AI 用。目标是让修改更快，但不要把边界写丢。

## 当前项目状态

- 当前阶段：MVP-0.4 已验收，阶段 2 真实模型调用准入设计已开始
- 已完成闭环：项目计划审批 -> Agent 自动分工 -> 只读 Runner request queue -> execution request 生命周期 -> runtime events 审计
- 当前模式：Mock / SQLite / 本地 Web App
- 明确不做：真实 Runner、真实模型、云同步、完整权限系统
- 真正可用应用的后续路线见 `dev-docs/应用真正可用落地计划.md`
- 真实模型调用进入实现前必须先通过 `dev-docs/真实模型接入准入规格.md`
- 阶段 2 当前只完成 `model_calls` 结构草案；未建表、未新增 route、未导入 SDK、未发 provider 请求。

## 先读顺序

1. `AGENTS.md`
2. `docs/README.md`
3. `docs/api-draft.md`
4. `docs/data-model-draft.md`
5. `docs/demo-checklist.md`
6. `dev-docs/新窗口交接说明.md`
7. `dev-docs/应用真正可用落地计划.md`
8. `dev-docs/真实模型接入准入规格.md`

## 维护原则

- 小步提交，变更必须可验证。
- 改 API 就更新 `docs/api-draft.md`。
- 改数据结构就更新 `docs/data-model-draft.md`。
- 改验收流程就更新 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改路标或阶段状态就更新 `dev-docs/下一步开发路线.md`。
- 改当前约束或交接状态就更新 `dev-docs/新窗口交接说明.md`。
- 改真实模型调用相关设计时，必须同步 `dev-docs/真实模型接入准入规格.md`，并保持 `verify-model-gateway.ps1` 与 `verify-real-model-admission.ps1` 通过。

## 受保护范围

- 不要碰 `design/image2/`
- 不要提交 `data/mock/runtime-state.json`
- 不要碰 `data/local/`
- 不要碰 `logs/`
- 不要碰 `.playwright-cli/`
- 不要碰 `_internal/`

## 当前验证集

```powershell
node --check apps\web\app.js
node --check services\api\server.js
node --check services\api\mock-data.js
node --check services\api\project-plan.js
powershell -ExecutionPolicy Bypass -File scripts\check-encoding.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-project-plan-flow.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-config-safety-loop.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-model-gateway.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-real-model-admission.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-local-ui.ps1
git diff --check
```

## 记录方式

当一个功能、bug 修复、规格调整或验收脚本变化已经落地时：

1. 更新对应文档。
2. 跑相关验证。
3. 提交一个清晰 commit。

如果只是临时调试，不要把 debug 垃圾写进正式文档。
