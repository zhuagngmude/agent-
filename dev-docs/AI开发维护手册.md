# AI 开发维护手册

日期：2026-06-18

这份手册给后续 AI 用。目标是减少重复摸索，避免把旧阶段边界当成当前事实。

## 当前项目状态

- 产品形态：本地桌面端 AI 项目总控台。
- 主入口：`apps/desktop`。
- UI 真源：`packages/ui`。
- 本地数据库：SQLite，经 Tauri/Rust service 访问。
- 产物目录：`workspace/generated`。
- 当前核心目标：稳定“输入目标 -> 总控调度 -> 选择 AI 员工/专家 -> 模型网关/执行器 -> Runner 产出项目文件”的链路。
- 当前准绳：`dev-docs/当前产品目标与落地路线.md`。

## 当前主页面

- 主控台：输入目标，发起全自动执行。
- 任务拆解：一个项目任务下展示分配的 AI 员工、状态、进度、产物目录，支持继续做、删除任务记录、打开产物文件夹。
- 流程蓝图：展示总控调度阶段、阶段涉及的 AI 员工、Runner/审批/模型网关关系。
- AI 员工：全局员工池、项目专家推荐、项目成员、执行器配置、模型目录、绑定智能体和 Skill 配置。
- 系统设置：配置模型服务和智能体大脑。

旧页面、旧审批入口、旧只读演示入口如果没有真实功能，不应继续出现在主导航。

## 当前下一步

暂停继续堆 UI，优先把 `AI 员工` 页相关配置从前端本地状态落到 Tauri/Rust/SQLite：

1. 执行器非敏感配置。
2. 模型目录。
3. Agent 模板。
4. 项目成员。
5. 专家推荐。
6. Executor Skill 配置。

API Key、Token、私钥不落普通数据库、不写日志、不显示给前端。

## Runner 和模型边界

允许：

- Runner 在应用受控服务层内全自动推进。
- Model Gateway 使用当前已配置模型发起请求。
- `workspace/generated` 内生成项目文件。
- 为了让个人本地流程跑通，网络模型请求本身不是问题。

禁止：

- 绕过 Model Gateway 直接调用 provider。
- 自由 shell、任意命令执行、Git commit/push。
- 文件删除、保护路径写入，除非用户明确要求并核验路径。
- 记录 raw key、raw prompt、raw response、raw provider error。
- 将密钥写进文档、代码、SQLite、日志或提交历史。

## 文档维护规则

- 改当前状态：同步 `dev-docs/当前项目导航.md` 和 `dev-docs/下一步开发路线.md`。
- 改交接口径：同步 `dev-docs/新窗口交接说明.md`。
- 改 AI 边界：同步 `docs/Agent宪法.md`、`docs/AI开发细则.md` 和本文件。
- 改 UI 主路径：同步 `README.md`。
- 历史阶段文档不逐篇重写，只在入口文档里说明它们是历史。

## 保护路径

不要碰：

- `design/image2/`
- `_internal/`
- `data/local/`
- `data/mock/runtime-state.json`
- `logs/`
- `.playwright-cli/`

## 常用检查

```powershell
cd F:\Projects\agent-swarm\packages\ui
npm run typecheck

cd F:\Projects\agent-swarm\apps\desktop\src-tauri
cargo check

cd F:\Projects\agent-swarm
rg -n "sk-[A-Za-z0-9_-]{16,}|Authorization:\s*Bearer\s+[^<\s]+|api_key=|token=|password=" -g "*.md" -g "!node_modules/**" -g "!target/**" -g "!dist/**"
```

## 做大体检时看什么

1. 页面是否有假入口、假按钮、点不动按钮。
2. 用户可见文案是否中文。
3. 任务页是否按总任务组织。
4. 已完成项目是否能打开产物文件夹。
5. 失败提示是否能让用户知道该换 key、改模型、继续做，还是重新执行。
6. 文档是否还写着旧的“只读、不执行、必须人工确认”并误导当前工作。
7. 是否有密钥或完整凭证残留。
