# docs

这里放 `agent-swarm` 的正式技术文档、治理文档和验收记录。

## 当前真相

- 项目核心目标是本地多模型 Agent 调度系统：总控 Agent 根据项目目标调度固定员工和项目专家 Agent，各角色按职责边界工作，并通过模型网关、Runner、审批和运行记录安全产出项目成果。
- 桌面端是主入口：`apps/desktop` + `packages/ui`。
- 当前用户主链路：主控台输入目标 -> 全自动执行 -> 自动生成/推进任务 -> Runner 产出项目文件。
- 当前 UI 已包含 `AI 员工` 模块，用于承载全技术栈固定员工池、项目专家推荐、职责边界和执行器/模型选择。
- 任务页以“总任务”为入口，支持继续做、删除、打开结果文件夹。
- 输出目录：`F:\Projects\agent-swarm\workspace\generated`。
- 模型配置在桌面端“系统设置”里维护；API Key、Base URL、模型 ID 不得写入文档、SQLite 或日志。
- Runner 全自动链路允许在服务层受控执行；仍禁止自由 shell、Git push/commit、文件删除、保护路径写入和提交密钥。

## 优先阅读

- [Agent宪法.md](./Agent宪法.md)：AI、Agent、Model Gateway、Runner 和安全边界的最高规则。
- [AI开发细则.md](./AI开发细则.md)：前端、数据库、Tauri/Rust、Git 和文档同步细则。
- [api-draft.md](./api-draft.md)：历史 API 契约草案；当前实现以 Tauri commands 和 Rust service 为准。
- [data-model-draft.md](./data-model-draft.md)：数据模型草案；结构真相以 `data/migrations/` 为准。
- [project-expert-agent-system.md](./project-expert-agent-system.md)：核心员工 + 项目专家 Agent 的动态队伍设计。
- [user-facing-multi-model-agent-explainer.md](./user-facing-multi-model-agent-explainer.md)：面向用户解释多模型 Agent 调度如何实现。
- [demo-checklist.md](./demo-checklist.md)：演示和验收清单。
- [module-stability-map.md](./module-stability-map.md)：保护区、契约区和可重构区说明。

## 历史规格

以下文档记录早期只读、审批、Runner 安全和 MVP 阶段设计。它们是历史依据，不一定代表当前产品状态：

- `mvp-0.3-*`
- `mvp-0.4-*`
- `runner-safety-acceptance.md`
- `project-plan-workflow-acceptance.md`
- `tauri-readonly-skeleton-acceptance.md`
- `write-commands-*`
- `agent-config-*`
- `agent-permission-contract.md`

如果历史文档与当前入口文档冲突，以 `README.md`、`AGENTS.md`、`docs/Agent宪法.md`、`docs/AI开发细则.md` 和 `dev-docs/当前项目导航.md` 为准。

## 验证入口

常用检查：

```powershell
cd F:\Projects\agent-swarm\packages\ui
npm run typecheck

cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo check
```
