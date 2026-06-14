# agent蜂群 AI 开发维护手册

这份文档给后续 AI 用。目标是让修改更快，但不要把边界写丢。

## 当前项目状态

- 阶段 18 “共享类型与规则骨架”已收口。`packages/shared`（跨端类型、DTO、常量）和 `packages/agent-core`（状态转换、终态判断、领域规则纯函数）已建纯 TypeScript 骨架；`packages/ui` 通过 tsconfig paths 从两个新包导入，不再本地定义跨端类型和状态转换规则。未新增 npm 依赖，Runner/真实模型/Git 执行仍关闭。
- 当前阶段：MVP-0.4 已验收，阶段 2 真实模型调用准入设计已收口，阶段 3 Agent Run 记录链已收口为本地 Mock / SQLite 流程
- 已完成闭环：项目计划审批 -> Agent 自动分工 -> 只读 Runner request queue -> execution request 生命周期 -> runtime events 审计 -> Agent Run 本地记录链
- 当前旧模式：Mock / SQLite / 本地 Web App。旧前端和旧 API 是 MVP 验证原型，不作为后续正式工程架构继续扩展。
- 当前已定形态：单人自用、本地桌面工具、桌面端主入口，Web 仅辅助预览或后续扩展入口。
- 明确不做：真实 Runner、真实模型、云同步、完整权限系统
- 重新立项后的工程初始化期间明确不做：不继续手搓前端，不继续扩展 Node.js 原生 HTTP 后端，不直接删除旧目录，不把真实 Runner / 真实模型 / 云同步 / 完整权限系统混进当前主线。
- `packages/ui` 已完成最小可运行工程骨架：React + TypeScript + Vite + Ant Design，并已在 `OverviewPage` 接入项目、Agent、Task、Approval 的只读数据。
- `apps/desktop` 已完成 Tauri/Rust 最小宿主，SQLite 运行库写入 Tauri app data 目录，不写入 `data/local/`。
- Tauri/Rust + SQLite 只读骨架正式验收文档见 `docs/tauri-readonly-skeleton-acceptance.md`。
- 教程 #11 已裁剪为单机 Tauri 写入安全边界，见 `docs/write-commands-security-design.md`。
- 写入 commands 设计已完成，`create_task`、`update_task_status`、`create_approval`、`approve_approval`、`reject_approval`、`patch_only_approval` 均见 `docs/write-commands-design.md`。
- 写入 commands Rust 实现和正式验收已完成，6 个 command 均已接入 Tauri invoke，见 `docs/write-commands-acceptance.md`。
- 前端共享 UI 写入接入已完成，5 个写入函数已封装，OverviewPage 已接入任务创建、状态变更和审批操作，见 `docs/frontend-write-commands-design.md` 和 `docs/tauri-desktop-write-interaction-acceptance.md`。
- 旧原型归档方案已确认：`apps/web/`、`services/api/`、`design/index.html` 等冻结为参考资产，不再作为正式主线扩展。
- 真正可用应用的后续路线见 `dev-docs/应用真正可用落地计划.md`
- 真实模型调用进入实现前必须先通过 `dev-docs/真实模型接入准入规格.md`
- 阶段 2 当前已完成 `model_calls` 结构草案、`Model Gateway正式入口设计.md`、provider config resolver helper、redaction / response limiter helper、helper-only `model_calls` 写入 / 迁移草案和禁用态 route 草案；阶段 2 已收口，未建表、未写 `model_calls`、未导入 SDK、未读取或返回 raw key、未发 provider 请求。
- 阶段 3 已完成 `agent_runs` Mock / SQLite API、失败注入、runtime event 审计、Web UI 记录页和验证脚本；它仍然不触发真实 Agent、不调用 provider、不启用 Runner、不写项目文件。

## 先读顺序

1. `AGENTS.md`
2. `docs/Agent宪法.md`
3. `docs/AI开发细则.md`
4. `docs/README.md`
5. `docs/api-draft.md`
6. `docs/data-model-draft.md`
7. `docs/demo-checklist.md`
8. `dev-docs/新窗口交接说明.md`
9. `dev-docs/应用真正可用落地计划.md`
10. `dev-docs/真实模型接入准入规格.md`
11. `dev-docs/Model Gateway正式入口设计.md`
12. `dev-docs/旧原型归档方案.md`
13. `dev-docs/旧原型页面迁移清单.md`
14. `dev-docs/阶段16-独立页面拆分方案.md`
15. `dev-docs/阶段17-长期分层边界设计.md`

## 维护原则

- 小步提交，变更必须可验证。
- 重新立项后的工程初始化期间，只做已确认架构内的最小闭环和只读链路；不要写完整真实业务功能。
- 不要继续在 `apps/web` 的原生 HTML / CSS / JavaScript 上叠新功能。
- 不要继续扩展 `services/api/server.js` 的 Node.js 原生 HTTP 路由作为正式后端方案。
- 新工程初始化已完成 `packages/ui`、`apps/desktop`、SQLite 最小读链路和 Overview 只读数据接入；旧原型迁移按归档方案逐步做。
- 治理文档、交接文档和阶段路线默认使用中文；英文只保留在代码标识、API、命令、环境变量和路径中。
- 改 AI 操作边界时，先同步 `docs/Agent宪法.md` 和 `docs/AI开发细则.md`，再同步本手册和交接说明。
- 改 API 就更新 `docs/api-draft.md`。
- 改数据结构就更新 `docs/data-model-draft.md`。
- 改验收流程就更新 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改路标或阶段状态就更新 `dev-docs/下一步开发路线.md`。
- 改当前约束或交接状态就更新 `dev-docs/新窗口交接说明.md`。
- 改真实模型调用相关设计时，必须同步 `dev-docs/真实模型接入准入规格.md`，并保持 `verify-model-gateway.ps1` 与 `verify-real-model-admission.ps1` 通过。
- 当前阶段 2 和阶段 3 已收口；后续不直接进入旧路线的阶段 4，而是先完成重新立项讨论。不要把真实 provider 调用和 Runner 执行混进当前主线。
- 当前阶段 18 已收口，技术栈、目录架构、共享 UI 方案、Tauri/Rust 桌面宿主、SQLite 只读 commands、写入安全边界裁剪文档、写入 commands 设计、Rust 实现、正式验收、前端共享 UI 写入接入、Tauri 桌面写入交互验收、旧原型归档方案、旧原型页面迁移清单、阶段 16 独立页面拆分、阶段 17 长期分层边界设计和阶段 18 共享类型与规则骨架已确认；下一步进入冻结模块解冻条件评估和后续实现阶段范围确认，但必须继续保持 Runner、真实模型和文件写入关闭。

## 任务触发口径（简版）

这部分只管工作流，不新增边界；如果和 `docs/Agent宪法.md` 冲突，以宪法为准。

- 需求不清、先别写代码：用需求澄清 / 方案讨论流程。
- 需求已明确、要拆步骤：用计划拆解流程。
- 报错、异常、性能问题：用系统化排错流程。
- 准备交付、检查是否完成：用验收自证流程。
- 要看页面效果：用浏览器验证流程。
- 要判断提交风险：用代码审查流程。
- 有多个互不依赖任务：再考虑并行调度。
- 需要端到端做完一个功能：用完整工程流，顺序保持“设计 → 计划 → 实现 → 自查 → 审查 → 验收”。

## 重新立项阶段列表

0. 重新立项讨论：进行中。
1. 中文治理文档：已完成第一版，后续按新架构继续修订。
2. 技术栈确认：已完成。
3. 新项目目录架构确认：已完成。
4. 共享 UI 方案确认：已完成，并已建立 `packages/ui` 最小工程骨架。
5. Tauri/Rust 桌面宿主方案确认：已完成，并已初始化最小 Tauri 宿主。
6. 宿主本地能力层和数据库方案确认：已完成，SQLite 最小闭环已接入 `get_project`。
7. 旧原型归档方案确认：已完成，见 `dev-docs/旧原型归档方案.md`。
8. 新工程初始化：进行中，已完成 `packages/ui`、`apps/desktop`、SQLite 初始化、seed 读链路、`projects / agents / tasks / approvals` 只读 commands、`OverviewPage` 只读数据接入和 Tauri 只读骨架正式验收文档。
9. 写入 commands 安全边界裁剪：已完成，见 `docs/write-commands-security-design.md`。
10. 写入 commands 设计：已完成，见 `docs/write-commands-design.md`。
11. 写入 commands Rust 实现：已完成，6 个 command 均已接入 Tauri invoke 并通过 Rust 测试。
12. 写入 commands 正式验收：已完成，见 `docs/write-commands-acceptance.md`。
13. 前端共享 UI 写入接入：已完成，见 `docs/frontend-write-commands-design.md`。
14. Tauri 桌面写入交互验收：已完成，见 `docs/tauri-desktop-write-interaction-acceptance.md`。
15. 旧原型页面迁移清单：已完成，见 `dev-docs/旧原型页面迁移清单.md`。
16. 独立页面拆分：已完成，见 `dev-docs/阶段16-独立页面拆分方案.md`。
17. 长期分层边界设计：已完成，见 `dev-docs/阶段17-长期分层边界设计.md`。
18. 共享类型与规则骨架：已完成。`packages/shared` 和 `packages/agent-core` 已建纯 TypeScript 骨架，UI 已接入。

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
cd packages\ui; npm run typecheck; npm run build
cd ..\..\apps\desktop\src-tauri; cargo check; cargo test
git diff --check
```

## 记录方式

当一个功能、bug 修复、规格调整或验收脚本变化已经落地时：

1. 更新对应文档。
2. 跑相关验证。
3. 提交一个清晰 commit。

如果只是临时调试，不要把 debug 垃圾写进正式文档。
