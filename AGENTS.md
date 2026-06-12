# agent蜂群 AI 操作入口

这是 `agent-swarm` 仓库的根级操作说明。后续 AI 进入项目时，必须先读本文件，再读中文治理文档和交接文档。

## 当前阶段

- 工作目录：`F:\projects\agent-swarm`
- 当前状态：MVP-0.4 已验收；阶段 2 真实模型调用准入设计已收口；阶段 3 Agent Run 记录链已收口为本地 Mock / SQLite 流程。
- 阶段 2 只保留 helper-only scaffold：不建真实表、不写 `model_calls`、不导入 provider SDK、不读取 raw key、不发 provider 请求。
- 阶段 3 只保留本地记录链和审计视图：不触发真实 Agent、不调用真实模型、不启用 Runner。
- Web App 优先，桌面端后续再接入；Mock / SQLite 优先，真实数据库后续再接入。

## 先读顺序

```text
docs/Agent宪法.md
docs/README.md
dev-docs/README.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
dev-docs/下一步开发路线.md
dev-docs/应用真正可用落地计划.md
dev-docs/真实模型接入准入规格.md
```

本地演示或验收相关工作，还要读：

```text
docs/demo-checklist.md
scripts/README.md
```

## 受保护路径

以下路径不要修改、不要提交、不要作为自动化写入目标：

- `design/image2/`
- `data/mock/runtime-state.json`
- `data/local/`
- `logs/`
- `.playwright-cli/`
- `_internal/`

公开计划、交接说明和阶段记录不要放在项目根目录，统一放到 `dev-docs/`。

## Runner 安全边界

- Runner 不得自动执行命令、写文件、删文件、发网络请求或修改 Git。
- MVP-0.3 项目计划审批只能创建只读 Runner request queue 记录。
- `targetService=agent_config` 的审批不得直接修改 Agent 配置，也不得创建 Runner job。
- 高风险动作必须先有二次确认和 Git checkpoint。
- 在真实模型链、Agent Run 记录链和执行计划审查链稳定前，不开放真实 Runner 执行。

## 真实模型边界

- 真实模型调用默认关闭。
- 禁止 UI、Agent、Runner 或任务状态机绕过 Model Gateway 直接调用 provider。
- 禁止导入 provider SDK、读取 raw key、返回 key fragment、记录 raw prompt 或 raw provider response。
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 不能复用为真实业务模型调用开关。
- 后续如果进入真实模型实现，必须先更新准入规格、验证脚本、数据模型和脱敏策略。

## 开发工作流

- 优先小步、可验证修改。
- 搜索文件和内容优先用 `rg` / `rg --files`。
- 手工编辑用 `apply_patch`。
- 改 API 同步 `docs/api-draft.md`。
- 改数据结构同步 `docs/data-model-draft.md`。
- 改演示或验收流程同步 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改阶段状态同步 `dev-docs/下一步开发路线.md`。
- 改 AI 操作边界同步 `docs/Agent宪法.md`、`dev-docs/AI开发维护手册.md` 和 `dev-docs/新窗口交接说明.md`。

## 常用验证

```powershell
node --check apps\web\app.js
node --check services\api\server.js
node --check services\api\mock-data.js
powershell -ExecutionPolicy Bypass -File scripts\check-encoding.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-project-plan-flow.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
git diff --check
```

## 本地运行

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-local.ps1
powershell -ExecutionPolicy Bypass -File scripts\status-local.ps1
powershell -ExecutionPolicy Bypass -File scripts\stop-local.ps1
```

默认 Mock API：

```text
http://127.0.0.1:8787
```
