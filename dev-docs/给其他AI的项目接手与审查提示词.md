# 给其他 AI 的项目接手与审查提示词

用途：把下面整段提示词复制给另一个 AI，让它快速理解 `agent-swarm` 当前状态，并优先做审查、清理或小步增强。

适用范围：2026-06-19 之后。P0 “AI 员工 -> 任务分派 -> Runner 边界检查 -> 任务页 Agent 展示”闭环已经完成，历史阶段文档只作背景。

---

## 可复制提示词

你现在接手一个本地多模型 AI Agent 编排桌面项目。请先审查当前工作区，不要急着大范围重构。

项目路径：

```text
F:\Projects\agent-swarm
```

当前真实主线：

```text
主控台输入目标
-> 总控确定性推荐项目 Agent
-> 任务绑定 project_agents.id
-> Runner preflight 调用 check_agent_boundary
-> denied 阻断 preflight 并写入 agent_boundary_checks
-> 任务页展示 Agent / 角色 / 执行器 / 模型 / 模块范围
-> 产物进入 workspace/generated
```

先阅读这些文件：

```text
AGENTS.md
docs/Agent宪法.md
docs/AI开发细则.md
docs/api-draft.md
docs/data-model-draft.md
dev-docs/当前项目导航.md
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/P0-剩余6步一体化推进计划.md
apps/desktop/src-tauri/src/services/agent_config.rs
apps/desktop/src-tauri/src/services/tasks.rs
apps/desktop/src-tauri/src/services/runner_preflight.rs
packages/ui/src/pages/AgentsPage.tsx
packages/ui/src/pages/TasksPage.tsx
packages/ui/src/utils/desktopHost.ts
```

开始前先运行：

```powershell
cd F:\Projects\agent-swarm
git status --short
git diff --check
```

已知工作区注意事项：

```text
workspace/generated 下可能有运行产物删除记录。
这些不是 P0 主线源码改动，不要恢复、修改或提交，除非用户明确要求。
```

当前已经具备的关键接口：

```text
recommend_project_agents
assign_project_agents_to_task
check_agent_boundary
list_agent_boundary_checks
get_task_agent_info
upsert_project_agent
remove_project_agent
upsert_executor_config
```

硬边界：

```text
不要新增密钥表。
不要把 API Key、Token、私钥、raw prompt、raw response、raw provider error 写入 SQLite、localStorage、日志或文档。
不要开放自由 shell、Git commit/push、文件删除、保护路径写入或未受控网络请求。
不要把浏览器预览假数据伪装成真实保存。
不要绕开总控、职责边界、模型网关、Runner preflight、审批/闸门和运行记录。
不要触碰 design/image2/、data/local/、logs/、_internal/。
```

建议下一步优先级：

1. 审查当前未提交 diff，确认 P0 闭环是否仍通过验证。
2. 清理入口文档、过时提示词和误导性“待实现”文案。
3. 增强 AgentRunsPage，把历史运行记录补充到项目 Agent、执行器、模型和产物上下文。
4. 审查并扩充专家模板 seed。
5. 清理假按钮或无真实后端能力的 UI 入口。

必须运行的验证：

```powershell
cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test agent_config --lib
cargo test tasks --lib
cargo test runner_preflight --lib

cd F:\Projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\Projects\agent-swarm
git diff --check
rg -n "sk-[A-Za-z0-9_-]{16,}|Authorization:\s*Bearer\s+[^<\s]+|api_key=|token=|password=" -g "*.md" -g "*.rs" -g "*.ts" -g "*.tsx" -g "!node_modules/**" -g "!target/**" -g "!dist/**"
```

请输出：

```text
1. Findings：按严重程度列出问题，带文件路径和原因。
2. 已确认可清理项：明确哪些文件/入口/文案可以删或改。
3. 已修改内容：如果动了代码或文档，列出文件。
4. 验证结果。
5. 剩余风险或需要用户确认的点。
```

不要 stage、commit、push，除非用户明确要求。
