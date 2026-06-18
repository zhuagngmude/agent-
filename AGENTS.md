# agent蜂群 AI 操作入口

这是 `agent-swarm` 仓库的根级操作说明。后续 AI 进入项目时，必须先读本文件，再读中文治理文档和交接文档。

## 当前阶段

- 工作目录：`F:\projects\agent-swarm`
- 当前状态：阶段 0-38 主线已落地到可用闭环。真实模型调用、任务模板、Runner 闸门链、最小执行、受控模型目录、运行时模型配置、主控台全自动入口、任务继续/删除/打开文件夹均已接入。
- 当前产品主线：实现本地多模型 Agent 调度系统。用户对总控 Agent 输入项目目标 -> 总控判断项目类型、技术栈、风险和阶段 -> 从固定员工池和项目专家 Agent 中选择角色 -> 按职责边界拆分任务 -> 通过模型网关调用 Codex/Claude/DeepSeek/Gemini/Cursor/OpenCode 等模型或执行器 -> 经过 Runner、审批和运行记录安全地产出代码与项目成果。
- 任何后续开发都必须服务这条主线：不要绕开总控、职责边界、模型网关、Runner、审批和运行记录，也不要扩展与该主线无关的功能。
- Runner 已按用户授权进入全自动开放口径：允许自动完成 preflight -> gate -> dry-run -> exec_lock -> minimal_run；仍禁止自由命令、Git commit/push、文件删除、保护路径写入和未受控网络请求。
- 模型配置当前通过系统设置写入运行时环境变量；不要把 API Key、Token、私钥或 `.env` 内容写入代码、文档、日志或提交。
- 桌面主入口优先，Web 只做辅助预览；SQLite + rusqlite 本地持久化优先，不做云同步。
- **历史边界**（阶段 2/3 已完成收口，此段仅描述阶段 2/3 当时的设计约束；截至阶段 35.1，真实模型已在阶段 25/35 边界内开放，Runner 已在阶段 34 边界内开放极小范围执行）：阶段 2 helper-only scaffold 当时落地为不建真实表、不写 `model_calls`、不导入 provider SDK；阶段 3 Agent Run 记录链当时收口为本地 Mock/SQLite 记录和审计视图（Agent Run 记录链至今仍保持 Mock/SQLite 定位，不触发真实 Agent 或 provider；真实模型调用走独立的 project_plan_generation 链路）。

## 先读顺序

```text
docs/Agent宪法.md
docs/AI开发细则.md
docs/README.md
docs/project-expert-agent-system.md
docs/user-facing-multi-model-agent-explainer.md
dev-docs/README.md
dev-docs/当前项目导航.md
dev-docs/AI开发维护手册.md
dev-docs/新窗口交接说明.md
dev-docs/下一步开发路线.md
dev-docs/应用真正可用落地计划.md
dev-docs/真实模型接入准入规格.md
```

## 真源优先级

当不同来源的信息冲突时，按以下顺序采信：

1. 当前源码、测试、schema、运行证据、git 状态
2. 本文件及 `docs/Agent宪法.md`、`docs/AI开发细则.md`
3. `dev-docs/` 下的内部真源索引和阶段设计文档
4. 当前有效的 `docs/`、README、验收文档
5. 历史文档和旧会话只能作为模式参考，不能覆盖当前代码事实

真源缺失或矛盾时，先报告冲突和推荐方向，等用户确认后再继续。

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

- Runner 当前允许在全自动主线中推进执行链并写入 `workspace/generated` 产物。
- 所有 Runner 执行仍必须经过 preflight→gate→dry-run→exec_lock→minimal_run 链路，不得绕过服务层直接写文件。
- `targetService=agent_config` 的审批不得直接修改 Agent 配置，也不得创建 Runner job。
- 不开放自由命令、Git commit/push、文件删除、保护路径写入、破坏性 Git 操作。
- 对用户项目目录以外的写入、删除、网络请求和凭据操作必须先单独设计并获用户明确确认。

## 真实模型边界

- 真实模型调用已开放给项目计划、Runner 执行和运行时模型连接测试等受控入口。
- 禁止 UI、Agent、Runner 或任务状态机绕过 Model Gateway 直接调用 provider。
- 运行时模型配置允许用户在系统设置输入 API Key、Base URL 和模型 ID；密钥只能留在当前桌面进程环境中，不得落库、写文档或写日志。
- 禁止导入 provider SDK、读取 raw key、返回 key fragment。
- 禁止把 raw key、raw base URL、raw prompt、raw response、raw provider error 写入前端、日志、SQLite。
- `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 不能复用为真实业务模型调用开关。
- 任何模型调用范围变更必须先更新准入规格、验证脚本、数据模型和脱敏策略。
- 阶段 35.1 前端中文化规则：用户可见文案默认中文；后端枚举值、数据库字段名、内部状态不得直接裸露给用户；英文只允许保留在模型 ID、命令、环境变量、Provider、路径、数据库 ID 等技术标识中。

## 开发工作流

- 优先小步、可验证修改。
- 搜索文件和内容优先用 `rg` / `rg --files`。
- 手工编辑用 `apply_patch`。
- 改 API 同步 `docs/api-draft.md`。
- 改数据结构同步 `docs/data-model-draft.md`。
- 改演示或验收流程同步 `docs/demo-checklist.md` 和 `scripts/README.md`。
- 改阶段状态同步 `dev-docs/当前项目导航.md` 和 `dev-docs/下一步开发路线.md`。
- 改 AI 操作边界同步 `docs/Agent宪法.md`、`docs/AI开发细则.md`、`dev-docs/AI开发维护手册.md` 和 `dev-docs/新窗口交接说明.md`。

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
