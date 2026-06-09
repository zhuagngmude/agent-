# agent蜂群

这是一个多 AI 智能体协作开发平台的项目文档入口。

## 文档入口

- `dev-docs/人类说明书.md`：给人看的项目说明、使用方式、产品计划和决策记录。
- `dev-docs/AI开发维护手册.md`：给 AI 编程工具看的开发规格、架构约束、升级路线和变更记录规则。
- `dev-docs/README.md`：开发过程资料目录说明。

## 当前定位

agent蜂群的目标是做一个“云端 Web 平台 + 本地 Python Runner”的多模型智能体协作系统。

用户输入项目想法后，系统自动调用不同模型，让它们扮演架构师、调度器、前端、后端、测试、文档、审查等 Agent。主 Agent 可以创建受控子 Agent。系统保存完整历史，并在用户确认后让本地 Runner 安全修改代码、跑测试、记录结果。

## 使用建议

人先读 `dev-docs/人类说明书.md`，理解这个项目要做什么、怎么使用、第一版边界是什么。

AI 后续接手开发、修 bug、升级功能时，必须先读 `AGENTS.md` 和 `dev-docs/AI开发维护手册.md`，再根据里面的“变更记录规则”更新文档。

AI/IDE 工具的快速规则入口：

```text
AGENTS.md
```

## 设计状态

旧的 4 套 UI 静态原型已删除，因为用户反馈“不好看”。

下一轮 UI 设计必须重新做，不沿用旧原型。重新设计前，应先参考合适的设计/前端相关 skill 和成熟产品界面，再给出更高质量的方案。

当前新的 UI 方向稿入口：

```text
design/index.html
```

预览服务启动后可访问：

```text
http://127.0.0.1:5174/index.html
```

这版包含 8 个方向：Linear、Notion、Raycast、GitHub、Vercel、Cursor、飞书、IDE。它们只借鉴信息结构和交互气质，不复刻第三方品牌。

## 文档维护规则

每次做了功能改动、架构调整、修 bug 或变更产品范围，都要同步更新：

1. `dev-docs/人类说明书.md` 的“变更记录”或相关说明。
2. `dev-docs/AI开发维护手册.md` 的“开发变更记录”或相关技术约束。

如果改动影响用户怎么使用，优先更新 `dev-docs/人类说明书.md`。

如果改动影响代码结构、数据模型、接口、权限、安全、Agent 流程，优先更新 `dev-docs/AI开发维护手册.md`。

## Git 保存规则

为了避免大项目被一次错误改动毁掉，本项目必须坚持小步提交：

- 每次重要讨论后改文档，要 `git commit`。
- 每次实现一个功能，要 `git commit`。
- 每次修一个 bug，要 `git commit`。
- 每次大改前，先 `git commit` 当前状态。

推荐提交前先运行：

```powershell
git status
```

推荐保存流程：

```powershell
git add .
git commit -m "说明这次改了什么"
```

## 本地内部资料

如果有不适合进入公开仓库的开发计划、截图、临时想法、私密提示词或个人资料，统一放到：

```text
_internal/
```

这个目录已被 `.gitignore` 忽略。AI 和人类协作者都不要提交它；如果资料包含真实密钥、账号或客户数据，最好放在项目目录之外，`.gitignore` 只用于防误提交，不是保密保险柜。

公开可提交的开发过程资料统一放在 `dev-docs/`，不要继续把计划、复盘、调研草案堆在根目录。

## 当前下一步

当前前端已经扩展为 12 个模块控制台。继续开发前请先阅读：

- `dev-docs/前端交互反推架构调整.md`
- `dev-docs/下一步开发路线.md`
- `docs/api-draft.md`
- `docs/data-model-draft.md`
- `docs/runner-safety-acceptance.md`

下一阶段目标是 `MVP-0.2：前端工程化 + Mock 状态机 + Runner 审批原型`。优先修复前端中文编码、抽出 mock 数据模型，并重点打磨审批与确认页面。

## 工程骨架

当前已建立正式工程骨架：

```text
apps/
  web/          电脑端 Web App，当前主要前端
  desktop/      后续桌面应用封装
services/
  api/          后端 API 服务
  runner/       本地 Runner
  worker/       Agent 调度与异步任务
packages/
  shared/       共享状态码、类型、工具函数
  ui/           通用 UI 组件
  agent-core/   Agent 编排核心逻辑
data/
  mock/         mock 数据
  migrations/   数据库迁移
scripts/        项目脚本
docs/           API、架构和决策文档
```

正式前端入口：

```text
apps/web/index.html
```

旧入口 `frontend/index.html` 只保留为兼容跳转页，后续不要在 `frontend/` 继续新增业务代码。

## 一键启动

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-dev.ps1
```

这个脚本会启动本地 mock API，并打开电脑端 Web App。

本地验收步骤见：

```text
docs/demo-checklist.md
```
