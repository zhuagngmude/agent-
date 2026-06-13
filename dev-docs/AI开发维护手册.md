# agent蜂群 AI 开发维护手册

这份文档给后续 AI 用。目标是让修改更快，但不要把边界写丢。

## 当前项目状态

- 当前进入“重新立项讨论中”。在技术栈、目录架构和共享 UI 方案确认前，不继续写业务代码。
- 当前阶段：MVP-0.4 已验收，阶段 2 真实模型调用准入设计已收口，阶段 3 Agent Run 记录链已收口为本地 Mock / SQLite 流程
- 已完成闭环：项目计划审批 -> Agent 自动分工 -> 只读 Runner request queue -> execution request 生命周期 -> runtime events 审计 -> Agent Run 本地记录链
- 当前旧模式：Mock / SQLite / 本地 Web App。旧前端和旧 API 是 MVP 验证原型，不作为后续正式工程架构继续扩展。
- 当前已定形态：单人自用、本地桌面工具、桌面端主入口，Web 仅辅助预览或后续扩展入口。
- 明确不做：真实 Runner、真实模型、云同步、完整权限系统
- 重新立项期间明确不做：不继续手搓前端，不继续扩展 Node.js 原生 HTTP 后端，不直接接 Tauri，不直接删除旧目录。
- `packages/ui` 已提前完成最小可运行工程骨架：React + TypeScript + Vite + Ant Design，只作为共享 UI 起点，不接真实业务能力。
- 真正可用应用的后续路线见 `dev-docs/应用真正可用落地计划.md`
- 真实模型调用进入实现前必须先通过 `dev-docs/真实模型接入准入规格.md`
- 阶段 2 当前已完成 `model_calls` 结构草案、`Model Gateway正式入口设计.md`、provider config resolver helper、redaction / response limiter helper、helper-only `model_calls` 写入 / 迁移草案和禁用态 route 草案；阶段 2 已收口，未建表、未写 `model_calls`、未导入 SDK、未读取或返回 raw key、未发 provider 请求。
- 阶段 3 已完成 `agent_runs` Mock / SQLite API、失败注入、runtime event 审计、Web UI 记录页和验证脚本；它仍然不触发真实 Agent、不调用 provider、不启用 Runner、不写项目文件。

## 先读顺序

1. `AGENTS.md`
2. `docs/Agent宪法.md`
3. `docs/README.md`
4. `docs/api-draft.md`
5. `docs/data-model-draft.md`
6. `docs/demo-checklist.md`
7. `dev-docs/新窗口交接说明.md`
8. `dev-docs/应用真正可用落地计划.md`
9. `dev-docs/真实模型接入准入规格.md`
10. `dev-docs/Model Gateway正式入口设计.md`

## 维护原则

- 小步提交，变更必须可验证。
- 重新立项讨论期间，只改路线、架构、技术栈和交接文档；不要写业务代码。
- 不要继续在 `apps/web` 的原生 HTML / CSS / JavaScript 上叠新功能。
- 不要继续扩展 `services/api/server.js` 的 Node.js 原生 HTTP 路由作为正式后端方案。
- 新工程初始化已先从 `packages/ui` 最小骨架开始；桌面宿主、数据库和旧原型迁移仍必须等对应方案确认后再做。
- 治理文档、交接文档和阶段路线默认使用中文；英文只保留在代码标识、API、命令、环境变量和路径中。
- 改 AI 操作边界时，先同步 `docs/Agent宪法.md`，再同步本手册和交接说明。
- 改 API 就更新 `docs/api-draft.md`。
- 改数据结构就更新 `docs/data-model-draft.md`。
- 改验收流程就更新 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改路标或阶段状态就更新 `dev-docs/下一步开发路线.md`。
- 改当前约束或交接状态就更新 `dev-docs/新窗口交接说明.md`。
- 改真实模型调用相关设计时，必须同步 `dev-docs/真实模型接入准入规格.md`，并保持 `verify-model-gateway.ps1` 与 `verify-real-model-admission.ps1` 通过。
- 当前阶段 2 和阶段 3 已收口；后续不直接进入旧路线的阶段 4，而是先完成重新立项讨论。不要把真实 provider 调用和 Runner 执行混进当前主线。
- 当前阶段 2 已收口，技术栈、目录架构和共享 UI 方案已确认；`packages/ui` 最小工程骨架已建立；下一步进入 Tauri/Rust 桌面宿主方案确认。

## 重新立项阶段列表

0. 重新立项讨论：进行中。
1. 中文治理文档：已完成第一版，后续按新架构继续修订。
2. 技术栈确认：已完成。
3. 新项目目录架构确认：已完成。
4. 共享 UI 方案确认：已完成，并已建立 `packages/ui` 最小工程骨架。
5. Tauri/Rust 桌面宿主方案确认：下一步。
6. 宿主本地能力层和数据库方案确认：未开始。
7. 旧原型归档方案确认：未开始。
8. 新工程初始化：已提前启动 `packages/ui` 骨架，桌面宿主和数据库工程暂不开始。

## 候选技术栈口径

```text
桌面主入口：Tauri 2 + Rust
前端 UI：React + TypeScript + Vite + Ant Design
本地数据库：SQLite + rusqlite
后续云端数据库：PostgreSQL / Supabase
共享 UI：packages/ui 作为唯一 UI 源码，apps/desktop 作为主入口，apps/web 作为辅助预览入口
```

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
