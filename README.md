# agent蜂群

单人自用的本地多模型 Agent 调度系统。当前主入口是 Tauri 桌面端，前端 UI 来自 `packages/ui`，本地状态使用 SQLite。

## 核心目标

`agent-swarm` 的长期目标是实现一个本地多模型 Agent 调度系统：

- 用户只对总控 Agent 描述项目目标。
- 总控 Agent 判断项目类型、技术栈、风险和当前阶段。
- 系统从固定员工池和项目专家 Agent 中选择合适角色。
- 每个 Agent 只在自己的职责模块内工作，跨模块任务必须交回总控拆分或转派。
- 每个 Agent 可以绑定不同模型或执行器，例如 Codex、Claude、DeepSeek、Gemini、Cursor、OpenCode。
- 模型调用统一经过模型网关，写代码和高风险动作统一经过 Runner、审批和运行记录。
- 产物按项目归档到 `workspace/generated`。

后续开发不能偏离这个目标，也不要绕远路做与该目标无关的功能。

## 当前状态

- 已打通主链路：主控台输入目标 -> 自动生成任务 -> Runner 推进执行 -> 产物写入 `workspace/generated`。
- 当前 UI 已扩展为多模块入口：`主控台`、`任务拆解`、`流程蓝图`、`运行输出`、`AI 员工`、`审批与安全`、`系统设置`。
- `AI 员工` 页已开始承载全技术栈固定员工池、项目专家推荐、职责边界和执行器/模型选择。
- 模型服务可在系统设置里配置 API Key、Base URL 和模型 ID；密钥只写入当前桌面进程环境变量，不写入文档或数据库。
- `apps/web`、`services/api` 和旧设计稿只作为历史参考，不再作为正式主线扩展。

## 先读这些

- [AGENTS.md](./AGENTS.md)
- [docs/README.md](./docs/README.md)
- [docs/project-expert-agent-system.md](./docs/project-expert-agent-system.md)
- [docs/user-facing-multi-model-agent-explainer.md](./docs/user-facing-multi-model-agent-explainer.md)
- [dev-docs/README.md](./dev-docs/README.md)
- [dev-docs/当前项目导航.md](./dev-docs/当前项目导航.md)
- [dev-docs/新窗口交接说明.md](./dev-docs/新窗口交接说明.md)

## 本地运行

桌面端开发启动：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo tauri dev
```

前端类型检查：

```powershell
cd F:\Projects\agent-swarm\packages\ui
npm run typecheck
```

Rust 相关快速检查：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo test project_plan --lib
cargo test runner_minimal_run --lib
cargo test auto_swarm --lib
```

## 产物位置

用户项目输出统一在：

```text
F:\Projects\agent-swarm\workspace\generated
```

任务页的“打开文件夹”按钮会打开对应项目/任务输出目录。

## 文档口径

历史阶段文档保留当时设计，不一定代表当前能力边界。当前事实优先级：

1. 当前源码、测试、运行结果
2. `AGENTS.md`、`docs/Agent宪法.md`、`docs/AI开发细则.md`
3. `dev-docs/当前项目导航.md`、`dev-docs/新窗口交接说明.md`
4. 其他 `docs/` 和 `dev-docs/` 历史文档
