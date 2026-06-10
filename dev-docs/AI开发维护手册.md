# agent蜂群 AI开发维护手册

## 1. 给后续 AI 的说明

你正在维护 agent蜂群。

这是一个多 AI 智能体协作开发平台，不是普通聊天应用，也不是单 Agent 代码生成器。

继续开发、修 bug、升级功能前，必须先理解：

- 本文件是给 AI 开发工具看的技术规格。
- `dev-docs/人类说明书.md` 是给用户和产品讨论看的说明。
- 修改功能后必须同步更新对应文档。

## 2. 产品目标

构建一个云端 Web App + 本地 Python Runner 的多模型智能体协作平台。

目标流程：

```text
用户创建项目
→ 输入项目想法
→ 架构师 Agent 生成总体方案
→ 架构师可创建受控子 Agent
→ 调度器 Agent 拆任务
→ 前端 / 后端 / 测试 / 文档 / 审查 Agent 执行
→ 审查 Agent 汇总
→ 主执行 Agent 生成代码修改计划
→ 用户确认
→ 本地 Python Runner 创建 Git checkpoint
→ Runner 修改代码和跑测试
→ 云端保存日志、输出、审查结果
```

## 3. 推荐技术栈

### 云端 Web

```text
Next.js
React
Tailwind CSS
Vercel
```

### 数据库和登录

```text
Supabase PostgreSQL
Supabase Auth
Supabase RLS
```

### 权限

```text
RBAC + ABAC 混合模型
```

RBAC 提供角色模板。

ABAC 支持项目、任务、Agent、模型 Key、代码执行等细粒度授权。

### 多模型调用

```text
OpenAI SDK
Anthropic SDK
Gemini API
```

### 本地 Runner

```text
Python CLI / Python 后台服务
```

Runner 负责本地文件读写、命令执行、测试、Git checkpoint。

## 4. 核心模块

### 4.1 Workspace

工作区，用于团队协作和权限边界。

### 4.2 Project

项目，包含项目想法、约束、Agent 配置、任务、日志和执行历史。

### 4.3 Agent

智能体角色，例如：

- Architect Agent
- Scheduler Agent
- Frontend Agent
- Backend Agent
- QA Agent
- Docs Agent
- Reviewer Agent
- Executor Agent

### 4.4 Sub Agent

由主 Agent 创建的受控子 Agent。

限制：

- depth 最大为 1。
- 每个主 Agent 最多 3 个子 Agent。
- 子 Agent 不允许继续创建子 Agent。
- 子 Agent 输出必须汇总回 parent_run。

### 4.5 Task

任务，包含状态、负责人、权限、依赖、可修改范围。

### 4.6 Agent Run

一次 Agent 执行记录。

保存：

- 输入
- 输出
- 模型
- token
- 成本
- 状态
- 错误
- parent_run_id
- 是否由子 Agent 产生

### 4.7 Local Runner

本地执行器。

职责：

- 绑定本地项目路径
- 拉取云端待执行任务
- 展示修改计划
- 等用户确认
- 创建 Git checkpoint
- 修改文件
- 跑测试
- 上传执行日志

## 5. 建议数据模型

### workspaces

```text
id
name
owner_user_id
created_at
updated_at
```

### workspace_members

```text
id
workspace_id
user_id
role_template
created_at
```

### projects

```text
id
workspace_id
name
idea
constraints
status
allow_sub_agents
allow_code_execution
created_by
created_at
updated_at
```

### agents

```text
id
project_id
name
agent_type
model_provider
model_name
system_prompt
max_sub_agents
can_spawn_sub_agents
can_execute_code
created_at
updated_at
```

### tasks

```text
id
project_id
parent_task_id
title
description
status
assigned_agent_id
priority
dependencies
allowed_file_scopes
requires_user_approval
created_at
updated_at
```

### agent_runs

```text
id
project_id
task_id
agent_id
parent_run_id
depth
spawn_reason
model_provider
model_name
input
output
status
token_input
token_output
cost_estimate
error_message
started_at
finished_at
created_at
```

### permissions

```text
id
workspace_id
project_id
subject_type
subject_id
resource_type
resource_id
action
effect
created_at
```

subject_type 示例：

```text
user
role
team
agent
```

resource_type 示例：

```text
workspace
project
task
agent
agent_run
model_key
export
code_execution
```

action 示例：

```text
view
create
edit
delete
run
approve
export
manage_permissions
manage_keys
spawn_agent
execute_code
```

### model_keys

```text
id
workspace_id
owner_user_id
provider
encrypted_key
scope
created_at
updated_at
```

注意：

- encrypted_key 必须加密。
- 不允许出现在日志、导出包、错误输出中。

### runner_connections

```text
id
project_id
runner_name
machine_fingerprint
local_project_path_alias
status
last_seen_at
created_at
```

不要把敏感本地路径无过滤地公开给其他用户。

### code_execution_requests

```text
id
project_id
task_id
agent_run_id
runner_connection_id
change_plan
status
requires_user_approval
approved_by
approved_at
git_checkpoint_ref
test_command
test_result
created_at
updated_at
```

## 6. Agent 流程约束

第一版必须按受控流程执行：

```text
Architect
→ Scheduler
→ Specialist Agents
→ Reviewer
→ Executor
```

Specialist Agents 包括：

- Frontend
- Backend
- QA
- Docs

子 Agent 规则：

```text
depth <= 1
parent_run_id 必填
每个 parent_run 最多 3 个子 Agent
子 Agent 不能 execute_code
子 Agent 不能 manage_permissions
子 Agent 不能 manage_keys
```

## 7. 代码执行安全约束

第一版只允许主执行 Agent 改代码。

必须满足：

```text
1. 用户显式确认。
2. 有 change_plan。
3. 有 allowed_file_scopes。
4. 执行前创建 Git checkpoint。
5. 执行后保存 diff 摘要。
6. 执行后跑测试或记录未跑原因。
7. 审查 Agent 检查结果。
```

禁止：

```text
多个 Agent 同时写同一项目
子 Agent 直接写文件
未经确认自动写文件
把 API Key 写入文件或日志
自动执行危险命令
```

后续升级到多 Agent 自动改代码时，必须加入：

```text
任务锁
文件所有权
Git 分支隔离
审查合并机制
冲突检测
回滚机制
```

## 8. API Key 安全规则

必须：

- 加密保存。
- 不写进项目日志。
- 不进入导出包。
- 错误信息中打码。
- 前端永远不接触完整 Key。

导出前必须扫描：

```text
sk-
AIza
token=
password=
Authorization: Bearer
api_key
secret
```

命中后打码或阻止导出。

## 8.1 本地内部资料目录

如果需要保存不适合进入公开仓库的开发计划、截图、临时想法、私密提示词或个人资料，统一放在：

```text
_internal/
```

规则：

- `_internal/` 已加入 `.gitignore`。
- AI 不要主动读取、修改或提交 `_internal/`，除非用户明确要求。
- 不要用 `_internal/` 存放真实 API Key、账号密码或客户敏感数据；`.gitignore` 只能防误提交，不等于加密保管。
- 如果发现内部资料已经被 Git 跟踪，先暂停并请用户确认，再考虑 `git rm --cached`。

## 9. MVP 范围

第一版必须做：

- 登录
- 邀请制团队使用
- Workspace
- Project
- Agent 配置
- 多模型自动流程
- 受控子 Agent
- 任务看板
- 完整历史记录
- 高级权限系统
- API Key 加密管理
- Python Runner
- 用户确认后自动改代码
- Git checkpoint
- 审查 Agent 汇总结果

第一版不做：

- 公开注册
- 计费系统
- 无限子 Agent
- 多个 Agent 同时改代码
- 完全无人确认的自动写文件
- 移动 App
- 插件市场

## 10. UI 设计维护规则

旧的 `ui-prototypes` 静态原型已删除，原因是用户反馈“不好看”。

后续 AI 不要恢复旧原型，不要沿用旧的视觉方向。

重新做 UI 前必须执行：

```text
1. 先询问或收集 design context。
2. 如果没有 design context，明确说明只能基于通用判断，并先给方向而不是直接大量写页面。
3. 优先使用成熟设计系统作为起点，例如 shadcn/ui、Radix Colors、Tailwind spacing scale。
4. 先做少量高质量关键页面，不要一次生成大量低质量页面。
5. 用 Microsoft Edge 打开预览并截图验证。
```

下一轮 UI 要避免：

```text
1. 通用 dashboard 卡片堆。
2. 假 metrics 和假 quote 装饰。
3. 装饰性 icon 滥用。
4. 紫蓝渐变、霓虹科技感、AI 味背景。
5. 无真实流程的展示页。
6. 为了填满页面而加入无用内容。
```

推荐 UI 信息架构：

```text
主工作台：项目、Agent 状态、任务、成本、权限风险。
Agent 流程页：架构师、调度器、专家 Agent、审查 Agent 的运行链路。
Runner 执行页：修改计划、文件范围、Git checkpoint、测试结果、用户确认。
权限页：成员、角色模板、资源级权限、高风险权限。
日志页：Agent Run、模型输出、成本、错误、审查记录。
```

UI 原型重新创建时，应放在：

```text
design/
```

不要再使用已删除的：

```text
ui-prototypes/
```

当前设计方向稿：

```text
design/index.html
```

包含 8 个方向：

```text
Linear / Notion / Raycast / GitHub / Vercel / Cursor / Feishu / IDE
```

注意：

- 这些方向只用于探索信息架构和交互气质。
- 不允许复制第三方品牌视觉资产。
- 用户选择方向后，再基于选中方向做正式页面。
- 后续正式 UI 仍应优先使用成熟组件系统，例如 shadcn/ui + Radix Colors + Tailwind。

## 11. 文档同步规则

后续 AI 修改项目时，必须同步更新文档。

### 修改用户可见功能时

更新：

```text
dev-docs/人类说明书.md
```

插入位置：

- 使用方式变化：更新“核心用户流程”。
- 产品范围变化：更新“第一版必须做”或“第一版暂时不做”。
- 安全/权限变化：更新“权限系统”或“API Key 管理”。
- 重大决策：更新“变更记录”。

### 修改技术实现时

更新：

```text
dev-docs/AI开发维护手册.md
```

插入位置：

- 数据库变化：更新“建议数据模型”。
- Agent 流程变化：更新“Agent 流程约束”。
- Runner 变化：更新“Local Runner”或“代码执行安全约束”。
- 权限变化：更新“权限”相关模型。
- 安全变化：更新“API Key 安全规则”。
- 新增限制：更新“MVP 范围”或“代码执行安全约束”。

### 每次改动必须追加记录

在本文件“开发变更记录”追加：

```text
日期
改了什么
为什么改
影响哪些模块
是否需要同步人类说明书
```

## 12. 给 AI 的 Build Prompt

```text
你是一个资深全栈产品工程师和 AI Agent 系统架构师。

请开发 agent蜂群：一个云端 Web App + 本地 Python Runner 的多 AI 智能体协作开发平台。

产品目标：
用户输入项目想法后，系统自动调用多个不同模型扮演架构师、调度器、前端、后端、测试、文档、审查等 Agent。主 Agent 可以创建受控子 Agent。系统保存完整历史、任务状态、模型输出、成本和审查记录。用户确认后，本地 Python Runner 可以安全修改代码、创建 Git checkpoint、运行测试并上传结果。

技术栈：
- 前端：Next.js + React + Tailwind CSS
- 云端部署：Vercel
- 数据库：Supabase PostgreSQL
- 登录：Supabase Auth
- 权限：Supabase RLS + RBAC/ABAC
- AI 调用：OpenAI SDK + Anthropic SDK + Gemini API
- 本地执行：Python Runner

第一版必须实现：
1. 登录和邀请制团队使用。
2. Workspace 和 Project。
3. Agent 配置，包括模型、角色、权限、是否允许子 Agent。
4. 自动 Agent 流程：Architect → Scheduler → Specialist Agents → Reviewer。
5. 受控子 Agent：最多 2 层，每个主 Agent 最多 3 个子 Agent。
6. 任务看板和任务状态。
7. 完整历史记录：Agent Run、模型输出、token、成本、错误。
8. 高级权限系统：项目、任务、Agent、API Key、代码执行细粒度权限。
9. API Key 加密保存，不进日志，不进导出包。
10. Python Runner：绑定本地项目、拉取执行请求、展示修改计划、用户确认、Git checkpoint、修改代码、跑测试、上传日志。
11. 审查 Agent 汇总执行结果。

第一版明确不做：
- 公开注册
- 计费系统
- 无限子 Agent
- 多个 Agent 同时改代码
- 完全无人确认的自动写文件
- 移动 App
- 插件市场

安全要求：
- 子 Agent 不能写代码。
- 只有主执行 Agent 能请求代码执行。
- 代码执行前必须有用户确认。
- 执行前必须创建 Git checkpoint。
- API Key 必须加密保存并打码。
- 导出前必须扫描敏感信息。

请优先实现清晰、可维护、可验证的 MVP，不要过度设计。
```

## 13. 开发变更记录

### 2026-06-07

- 创建 agent蜂群 AI 开发维护手册。
- 确定云端 Web App + 本地 Python Runner 架构。
- 确定多模型自动 Agent 流程。
- 确定受控子 Agent 规则。
- 确定高级权限系统方向。
- 确定第一版允许主执行 Agent 在用户确认后通过 Runner 改代码。
- 建立文档同步规则，要求后续改动同时更新人类说明书和 AI 开发维护手册。
- 删除旧 UI 静态原型；新增 UI 设计维护规则，要求后续先收集 design context，避免通用 AI 味界面。
- 新增 `design/index.html`，创建 8 个 UI 方向稿用于选择，不沿用已删除的旧原型。
## 2026-06-08 重要补充：前端交互反推架构

后续 AI 接手开发前，除了阅读本文档，还必须阅读：

```text
dev-docs/前端交互反推架构调整.md
dev-docs/下一步开发路线.md
docs/api-draft.md
docs/data-model-draft.md
docs/runner-safety-acceptance.md
```

原因：`frontend/index.html` 已经把产品从最初的多 Agent 任务调度原型，扩展成 12 个模块的 AI 项目控制台。后端架构需要按照前端交互重新确认模块边界，尤其是：

- Dashboard 聚合接口
- Agent 资源池
- Task 状态机
- Workflow 编排
- Approval 与 Runner 安全网关
- Knowledge 文档片段
- Usage/Cost 统计
- Integration 插件边界

关键原则：Runner Service 不能自己决定自己是否可以执行。所有本地写文件、删文件、执行命令、网络请求、Git 操作，都必须经过 Approval Service，并在高风险场景中要求二次确认和 Git checkpoint。

应用形态原则：当前阶段是电脑端 Web App，不是安装版桌面软件。先把 Web 控制台、Mock 数据、状态机和 Runner 审批流程做稳；随后接本地 Runner；最后再考虑用 Tauri 或 Electron 封装成 `agent蜂群.exe`。

工程骨架原则：正式前端入口是 `apps/web/`。旧 `frontend/` 只保留兼容跳转入口，后续不要在 `frontend/` 新增业务代码。API 服务放 `services/api/`，本地 Runner 放 `services/runner/`，Agent 调度放 `services/worker/`，共享状态和类型放 `packages/shared/`。

## 2026-06-08 变更记录：任务管理 Mock 状态机

- 改了什么：为 `services/api` 增加任务运行时状态保存和 `POST /api/tasks/:taskId/start|complete|fail|cancel`；为 `apps/web` 任务页增加任务详情和状态操作。
- 为什么改：让任务管理从静态展示进入可操作状态机，并让 Dashboard 的活跃任务数跟随任务状态变化。
- 影响模块：`services/api/mock-data.js`、`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；这是 MVP-0.2 内部工程能力增强，用户核心流程没有变化。

## 2026-06-08 变更记录：工作流编排只读 Mock API

- 改了什么：为 `services/api` 增加 `GET /api/projects/:projectId/workflows`，并让 Dashboard 聚合接口返回 `workflows`；为 `apps/web` 工作流编排页增加只读流程、节点和依赖连线渲染。
- 为什么改：让工作流编排页从静态占位进入可接数据状态，为后续流程运行记录、Runner job 队列和编排编辑做准备。
- 影响模块：`services/api/mock-data.js`、`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前只读展示不改变用户核心操作流程。

## 2026-06-08 变更记录：Runner job 只读队列

- 改了什么：审批批准时创建 `runner_job_*` 只读队列项；新增 `GET /api/projects/:projectId/runner/jobs`；运行与调度页展示 Runner job 数量、等待执行数量、失败数量和队列表格。
- 为什么改：补齐“审批通过之后会发生什么”的可追踪链路，但暂不让 Runner 真实执行本地命令。
- 影响模块：`services/api/mock-data.js`、`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部只读状态追踪。

## 2026-06-08 变更记录：Runner job 只读详情

- 改了什么：运行与调度页的 Runner job 队列表格支持点选；新增只读详情面板，展示 Job ID、来源审批、关联任务、操作类型、影响文件、Git checkpoint、创建/更新时间和安全说明。
- 为什么改：让用户在批准审批后能看懂后续待执行项的来源和影响范围，同时继续明确当前不会执行本地命令。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；这是 MVP-0.2 运行页可读性增强，不改变核心用户流程。

## 2026-06-08 变更记录：智能体管理页只读 Mock API

- 改了什么：智能体管理页从静态占位改为渲染 Mock API 聚合数据，展示 Agent 名称、角色、状态、模型、权限摘要和子 Agent 创建限制；模型分配区同步展示各 Agent 的模型和状态。
- 为什么改：让系统核心对象“多智能体”进入可读、可接数据的页面状态，为后续 Agent 详情、子 Agent 关系和配置编辑打基础。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部只读展示增强。

## 2026-06-08 变更记录：智能体详情只读面板

- 改了什么：智能体资源池卡片支持点选；新增智能体详情面板，展示 Agent ID、角色、模型、版本、状态、权限和子 Agent 创建限制。
- 为什么改：让用户可以从资源池进入单个 Agent 的配置视图，但当前仍不允许修改配置。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；这是 MVP-0.2 内部只读展示增强。

## 2026-06-09 变更记录：子 Agent 关系只读展示

- 改了什么：Mock Agent 数据新增 `parentAgentId`、`childAgentIds`、`reportsToAgentId`、`spawnDepth`；智能体管理页新增子 Agent 关系面板，并在 Agent 详情里展示父 Agent、汇总目标、派生深度和当前子 Agent 数。
- 为什么改：把“主 Agent 可以派生受控子 Agent，子 Agent 输出必须汇总回父 Agent”的约束做成可见数据，为后续配置编辑和运行记录打基础。
- 影响模块：`services/api/mock-data.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部只读展示增强。

## 2026-06-09 变更记录：Agent 配置规则草案

- 改了什么：智能体管理页新增配置规则草案面板，展示可编辑字段、必须审批字段、暂时只读字段和禁止子 Agent 修改字段；API 草案补充 `PATCH /api/agents/:agentId` 的权限边界说明。
- 为什么改：在开放任何 Agent 配置编辑前，先把可修改范围和审批边界固定下来，避免后续子 Agent 自行扩权或绕过审批。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部规则展示，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置变更请求预览

- 改了什么：智能体管理页新增变更请求预览面板，可基于当前选中 Agent 预览模型切换、子 Agent 权限调整和权限升级，展示字段差异、风险等级和是否需要审批。
- 为什么改：先把 Agent 配置修改变成可审查的申请预览，后续再接 Approval Request，避免直接保存高风险配置。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前不保存配置，也不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置变更申请接口

- 改了什么：新增 `POST /api/agents/:agentId/change-requests`，可把 Agent 配置变更预览转换为 `targetService=agent_config` 的 Approval Request；前端新增“生成审批申请”按钮并刷新审批列表。
- 为什么改：让 Agent 配置修改先进入审批流，而不是直接写配置；`agent_config` 审批通过后不会生成 Runner job。
- 影响模块：`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前只创建审批申请，不直接改变 Agent 配置。

## 2026-06-09 变更记录：Agent 配置待应用状态

- 改了什么：`targetService=agent_config` 的审批通过后，会创建 `agentConfigApplications` 只读记录，状态为 `pending_apply`；智能体管理页新增“待应用配置变更”面板。
- 为什么改：在“审批通过”和“真正修改 Agent 配置”之间增加缓冲状态，避免审批一通过就自动改权限、模型或子 Agent 能力边界。
- 影响模块：`services/api/mock-data.js`、`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部只读状态追踪，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置待应用审查视图

- 改了什么：智能体管理页的“待应用配置变更”面板从简单列表升级为只读审查视图，可选中待应用记录，查看目标 Agent、来源审批、审批状态、字段变更和应用前检查项。
- 为什么改：在开放真正应用配置之前，先让用户和后续 AI 能看清每条 `pending_apply` 记录是否来自 `agent_config` 审批、是否没有 Runner job、是否仍等待人工应用。
- 影响模块：`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部只读审查增强，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置人工应用确认草案

- 改了什么：API 草案新增 `POST /api/agent-config-applications/:applicationId/apply` 的未实现接口边界；智能体管理页待应用审查视图新增“人工应用确认条件”，说明二次确认、状态流转和只能应用已审批字段。
- 为什么改：在真正允许写入 Agent 配置前，先固定人工应用的安全前置条件，避免 `pending_apply` 记录被自动应用或绕过审批。
- 影响模块：`apps/web/app.js`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只是内部流程草案和只读展示，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置人工应用 Mock 状态流转

- 改了什么：`services/api` 新增 `POST /api/agent-config-applications/:applicationId/apply`，在满足来源审批已批准、目标服务为 `agent_config`、无 Runner job、状态为 `pending_apply` 且带二次确认时，把应用记录标记为 `applied`；前端新增“模拟应用状态”按钮并刷新状态。
- 为什么改：先验证 Agent 配置应用流程的状态闭环，同时继续阻止真实 Agent 配置写入和 Runner job 生成。
- 影响模块：`services/api/server.js`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 Mock 状态流转，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置应用审计记录展示

- 改了什么：智能体管理页待应用详情新增“应用审计记录”，展示 `appliedAt`、`appliedBy`、`applyConfirmText`，并明确 Agent 配置未真实写入、Runner job 未生成、当前仅为 Mock 状态流转。
- 为什么改：让用户和后续 AI 能追踪一次 Agent 配置 Mock 应用是谁确认的、何时确认的、确认文本是什么，避免“已应用”状态缺少审计上下文。
- 影响模块：`apps/web/app.js`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 内部审计展示，不改变用户核心流程。

## 2026-06-09 变更记录：Agent 配置待应用 Mock 取消流程

- 改了什么：`services/api` 新增 `POST /api/agent-config-applications/:applicationId/cancel`，只允许把 `pending_apply` 记录标记为 `cancelled` 并记录取消原因；智能体管理页新增“模拟取消应用”按钮和取消审计字段展示。
- 为什么改：让已审批但尚未应用的 Agent 配置变更可以安全作废，避免审批流只能向前推进。
- 影响模块：`services/api/server.js`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 Mock 状态流转，不改变用户核心流程。

## 2026-06-09 变更记录：Runner 状态只读面板

- 改了什么：Mock Dashboard 聚合接口新增 `runnerStatus`；`GET /api/projects/:projectId/runner/status` 复用同一份 Runner 状态数据；运行与调度页新增 Runner 连接状态、版本、工作区、权限边界和最后心跳展示。
- 为什么改：让用户能确认本地 Runner 的安全边界和连接状态，同时继续保持只读，不开放任何本地执行能力。
- 影响模块：`services/api/mock-data.js`、`services/api/server.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是 MVP-0.2 只读状态展示，不改变真实 Runner 执行边界。

## 2026-06-09 变更记录：Agent 配置回滚前审查展示

- 改了什么：智能体管理页待应用详情新增“回滚前审查”区块，展示回滚入口边界、应用状态、应用审计、来源审批、Runner job、字段差异和当前结论。
- 为什么改：在真正开放回滚前，先让用户和后续 AI 看清一次已应用记录是否具备回滚审查条件，并明确真正回滚必须重新走 Approval Service。
- 影响模块：`apps/web/app.js`、`docs/api-draft.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是只读审查展示，不执行真实回滚、不修改 Agent 配置。

## 2026-06-09 变更记录：本地 Demo 验证清单

- 改了什么：新增 `docs/demo-checklist.md`，汇总本地启动、健康检查、页面验收点、Mock 状态重置和当前安全边界；README 和 `scripts/README.md` 增加入口。
- 为什么改：减少人类用户和后续 AI 接手时的试错成本，让 MVP-0.2 的可用范围和不可做事项更清楚。
- 影响模块：`docs/demo-checklist.md`、`README.md`、`scripts/README.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是本地开发/验收说明，不改变产品功能。

## 2026-06-09 变更记录：Mock API 状态流转验证脚本

- 改了什么：新增 `scripts/verify-mock-flows.ps1`，可自动检查 Mock API 健康状态、Dashboard 聚合、任务 `start -> complete`、Runner 审批生成只读 job、Agent 配置 Mock 应用和 Mock 取消流程；脚本结束后会重置 runtime state。
- 为什么改：给后续开发一个可重复回归入口，避免状态机改动后悄悄破坏审批、任务或 Agent 配置应用流程。
- 影响模块：`scripts/verify-mock-flows.ps1`、`scripts/README.md`、`docs/demo-checklist.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是开发验证工具，不改变产品功能。

## 2026-06-09 变更记录：本地内部资料目录规则

- 改了什么：`.gitignore` 新增 `_internal/`；README、新窗口交接说明和 AI 维护手册增加说明，明确 `_internal/` 用于本地内部资料，AI 不要提交。
- 为什么改：降低开发计划、截图、临时想法、私密提示词等内部资料被误提交到公开仓库的风险。
- 影响模块：`.gitignore`、`README.md`、`新窗口交接说明.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是仓库安全规则，不改变产品功能。

## 2026-06-09 变更记录：AI/IDE 快速规则入口

- 改了什么：新增 `AGENTS.md`，汇总项目阶段、必读文档、禁止提交目录、Runner 安全边界、开发流程、验证命令和本地启动方式；README 增加入口。
- 为什么改：让 Codex、Cursor、Windsurf、Copilot 等 AI/IDE 工具接手时能先读一页高优先级规则，减少漏读长文档导致的误提交或越权执行。
- 影响模块：`AGENTS.md`、`README.md`、`下一步开发路线.md`、`AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是开发协作规则入口，不改变产品功能。

## 2026-06-09 变更记录：开发过程资料目录整理

- 改了什么：新增 `dev-docs/`，把人类说明书、AI 维护手册、开发路线、新窗口交接说明、前端交互反推和 UI 验收建议从根目录迁入；README、AGENTS、Mock 数据和 API 示例同步新路径。
- 为什么改：根目录只保留入口/宪法类文件，避免后续几十份计划、复盘和调研草案堆在根目录；同时保留 `docs/` 作为正式 API/Demo 文档目录，`_internal/` 作为不提交的本地内部资料目录。
- 影响模块：`dev-docs/`、`README.md`、`AGENTS.md`、`apps/web/index.html`、`services/api/mock-data.js`、`docs/api-draft.md`。
- 是否需要同步人类说明书：暂不需要；这是仓库文档结构整理，不改变产品功能。

## 2026-06-09 变更记录：数据库模型草案

- 改了什么：新增 `docs/data-model-draft.md`，定义真实数据库接入前的核心表、字段、关系、状态约束、迁移顺序和待确认问题。
- 为什么改：数据库是后续真实后端的地基，先把项目、Agent、任务、审批、Runner job、Agent 配置应用、工作流、知识更新、Git checkpoint 和运行事件的模型固定下来，避免边写边改表。
- 影响模块：`docs/data-model-draft.md`、`README.md`、`AGENTS.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只做数据库设计草案，不改变产品功能或运行逻辑。

## 2026-06-09 变更记录：Runner 安全验收标准

- 改了什么：新增 `docs/runner-safety-acceptance.md`，定义真实 Runner 执行前必须满足的审批、二次确认、Git checkpoint、文件范围锁定、命令白名单、执行审计、失败处理和回滚前置条件。
- 为什么改：Runner 一旦能真实写文件或执行命令，风险显著高于 Mock 阶段；必须先把放行条件写成可验收标准，避免凭感觉开放本地执行能力。
- 影响模块：`docs/runner-safety-acceptance.md`、`README.md`、`AGENTS.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只做安全验收标准，不实现 Runner 执行代码。

## 2026-06-09 变更记录：数据库模型待确认问题定稿

- 改了什么：更新 `docs/data-model-draft.md`，把数据库第一版决策定为本地 SQLite、单项目优先但保留 `project_id`、Agent 配置采用 `agents` 当前态加 `agent_config_versions`、状态流转写完整 `runtime_events`、工作流节点和连线暂保留 JSON。
- 为什么改：真实数据库初始化和 seed 方案需要先固定边界，否则后续会在 SQLite/PostgreSQL、多项目、Agent 配置审计和工作流拆表之间反复摇摆。
- 影响模块：`docs/data-model-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只定稿数据库设计决策，不改变产品功能或运行逻辑。

## 2026-06-09 变更记录：SQLite 初始化与 Seed 方案

- 改了什么：新增 `docs/sqlite-seed-plan.md`，定义第一版本地 SQLite 数据库文件位置、migration/seed 产物、表范围、Mock 数据映射、runtime state 迁移、API 切换顺序和验证标准；README、AGENTS、data、migrations、API README 和开发路线同步入口。
- 为什么改：在实现真实数据库代码前，先把初始化和 seed 边界写清楚，避免直接从 Mock API 跳到不可重复、不可重建的本地数据库状态。
- 影响模块：`docs/sqlite-seed-plan.md`、`README.md`、`AGENTS.md`、`data/README.md`、`data/migrations/README.md`、`services/api/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只新增数据库实现前设计文档，不改变产品功能或运行逻辑。

## 2026-06-09 变更记录：SQLite 初始化与 Seed 脚本

- 改了什么：新增 `.gitignore` 本地数据库忽略规则、`data/migrations/001_initial_sqlite.sql`、`data/seed/project_agent_swarm.seed.json`、`scripts/init-sqlite.ps1` 和 `scripts/seed-sqlite.ps1`，可用 Python 标准库创建并重建第一版本地 SQLite 数据库；同步更新脚本 README、数据模型和开发路线。
- 为什么改：让数据库接入从文档进入可验证的本地初始化阶段，同时仍不改变 Mock API 运行逻辑，避免一次性切换 API 带来状态机回归风险。
- 影响模块：`.gitignore`、`data/migrations/001_initial_sqlite.sql`、`data/seed/project_agent_swarm.seed.json`、`scripts/init-sqlite.ps1`、`scripts/seed-sqlite.ps1`、`scripts/README.md`、`docs/sqlite-seed-plan.md`、`docs/data-model-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前是本地数据库初始化能力，不改变用户可见产品流程。

## 2026-06-09 变更记录：SQLite Dashboard 只读查询

- 改了什么：新增 `services/api/db/sqlite-read.js`，可通过 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 让 Dashboard 聚合接口从本地 SQLite 只读查询；默认仍使用 Mock 内存数据，SQLite 查询失败会回退 Mock Dashboard。
- 为什么改：先验证数据库 row 到现有 Dashboard response shape 的映射，降低后续一次性切换所有 API 的风险。
- 影响模块：`services/api/db/sqlite-read.js`、`services/api/server.js`、`services/api/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前是开发期开关能力，不改变默认用户流程。

## 2026-06-09 变更记录：SQLite 第一批只读接口

- 改了什么：扩展 `services/api/db/sqlite-read.js`，将 SQLite 查询结果拆为项目快照，并让 `GET /agents`、`GET /tasks`、`GET /approvals`、`GET /workflows` 在 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 下读取 SQLite；默认仍使用 Mock 数据，写操作仍走 Mock runtime state。
- 为什么改：让第一批核心只读页面可以共享同一套 SQLite row mapper，同时避免过早迁移任务和审批写入状态机。
- 影响模块：`services/api/db/sqlite-read.js`、`services/api/server.js`、`services/api/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是开发期开关能力，不改变默认用户流程。

## 2026-06-09 变更记录：SQLite 剩余只读接口

- 改了什么：继续扩展 SQLite 快照读取，让 `GET /agent-config-applications`、`GET /runner/status`、`GET /runner/jobs`、`GET /git/checkpoints` 和 `GET /knowledge/updates` 在 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 下读取 SQLite。
- 为什么改：补齐 MVP-0.2 当前已实现的只读接口，确保后续迁移写入状态机前，读路径已经完整可验证。
- 影响模块：`services/api/db/sqlite-read.js`、`services/api/server.js`、`services/api/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是开发期开关能力，不改变默认用户流程。

## 2026-06-09 变更记录：SQLite 状态流转写入

- 改了什么：新增 `services/api/db/sqlite-write.js`，在 `AGENT_SWARM_DASHBOARD_SOURCE=sqlite` 下支持任务状态流转、审批 approve/reject/patch-only、Agent 配置变更申请、Agent 配置应用/取消写入 SQLite，并为状态变化写入 `runtime_events`；SQLite 模式下 `/api/runtime-state/reset` 通过 seed 重建状态。
- 为什么改：让 SQLite 模式从只读展示进入可验证状态机，同时保持默认 Mock runtime-state 路径不变，避免影响现有演示。
- 影响模块：`services/api/db/sqlite-write.js`、`services/api/server.js`、`services/api/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍是开发期开关能力，不改变默认用户流程。

## 2026-06-09 变更记录：技术栈适配性记录

- 改了什么：新增 `docs/tech-stack-notes.md`，记录当前实际技术栈、SQLite 本地持久化路径、未来生产栈方向，以及暂时不迁移的边界；README、AGENTS 和开发路线同步入口。
- 为什么改：明确当前 HTML/CSS/JavaScript、Node mock API、PowerShell、Python SQLite bridge、Markdown 和 SQLite 适合 MVP 验证，但不等同于最终商业化最优栈，避免过早迁移框架或数据库。
- 影响模块：`docs/tech-stack-notes.md`、`README.md`、`AGENTS.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是技术决策记录，不改变用户可见产品流程。
## 2026-06-09 变更记录：SQLite Python 桥接层与回归验证

- 改了什么：新增 `scripts/sqlite/init_sqlite.py`、`scripts/sqlite/seed_sqlite.py` 和 `scripts/sqlite/sqlite_write.py`，让 `init-sqlite.ps1`、`seed-sqlite.ps1` 和 `services/api/db/sqlite-write.js` 不再维护大段内联 Python；新增 `scripts/verify-sqlite-flows.ps1`，在独立端口验证 SQLite 模式 Dashboard、任务、审批、Runner job、Agent 配置应用/取消和 reset。
- 为什么改：SQLite 已经从只读进入状态流转写入阶段，桥接层必须更可维护、可回归，避免后续改 schema 或状态机时同时修改 PowerShell、Node 内联字符串和 Python 逻辑。
- 影响模块：`scripts/init-sqlite.ps1`、`scripts/seed-sqlite.ps1`、`scripts/sqlite/`、`scripts/verify-sqlite-flows.ps1`、`services/api/db/sqlite-write.js`、`scripts/README.md`、`services/api/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是开发验证和数据库桥接层整理，不改变用户可见产品流程。
## 2026-06-09 变更记录：SQLite 读取 mapper 整理

- 改了什么：新增 `scripts/sqlite/sqlite_read.py`，将 SQLite 读取查询、Dashboard 快照组装和 row-to-API 字段转换集中到独立 Python mapper；`services/api/db/sqlite-read.js` 改为只负责调用脚本并解析 JSON。
- 为什么改：数据库使用 snake_case，API/前端使用 camelCase，字段转换必须保留但应集中管理，避免转换逻辑散在查询流程中，后续改 schema 或 response shape 时难以维护。
- 影响模块：`scripts/sqlite/sqlite_read.py`、`services/api/db/sqlite-read.js`、`scripts/README.md`、`docs/sqlite-seed-plan.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是数据库读取层内部整理，不改变用户可见产品流程。
## 2026-06-09 变更记录：前端旧占位文案收敛

- 改了什么：清理 `apps/web/index.html` 和 `apps/web/data.js` 中加载前静态固定数值、离线兜底旧审批/任务/commit 示例、真实模型密钥“已配置”、真实集成“已连接”、假监控和假费用文案；`apps/web/app.js` 补充首页空态并停止显示假 Token 消耗。
- 为什么改：数据库和 API 链路已经稳定，前端加载前或离线兜底不能继续展示容易误导验收的旧 demo 数据，否则后续会误判真实能力已经接入。
- 影响模块：`apps/web/index.html`、`apps/web/data.js`、`apps/web/app.js`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是前端原型文案和兜底数据收敛，不改变核心用户流程。

## 2026-06-09 变更记录：前端 Demo 数据对齐与页面验收

- 改了什么：将 Mock/SQLite seed 中的审批示例对齐到当前项目文件，移除旧 Runner 文件、旧 commit、假 diff 数字、假 Token、假费用和假密钥状态；修复浏览器 favicon 404；前端待审批列表只展示 `pending` 审批，避免把 `patch_only` 历史项误算为待审批。
- 为什么改：Demo 验收必须反映当前 MVP-0.2 的真实边界，不能让页面看起来像已经接入真实 Runner、真实模型调用、真实费用统计或真实密钥配置。
- 影响模块：`apps/web/index.html`、`apps/web/app.js`、`services/api/mock-data.js`、`services/api/server.js`、`data/seed/project_agent_swarm.seed.json`、`scripts/sqlite/sqlite_read.py`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是 Demo 数据和验收展示修正，不改变核心用户流程，也不开放真实 Runner 执行。

## 2026-06-09 变更记录：本地 SQLite 试用版启停入口

- 改了什么：新增 `scripts/start-local.ps1`、`scripts/status-local.ps1` 和 `scripts/stop-local.ps1`，用于启动 SQLite 模式 API、本地 Web 静态服务、查看状态和停止本项目本地试用进程；README、AGENTS、scripts README、Demo checklist 和开发路线同步入口。
- 为什么改：让用户可以先试用“本地端”形态，状态持久化到 `data/local/agent-swarm.sqlite`，并且能明确停止和删除本地状态，降低试用成本。
- 影响模块：`scripts/start-local.ps1`、`scripts/status-local.ps1`、`scripts/stop-local.ps1`、`README.md`、`AGENTS.md`、`scripts/README.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是本地试用入口和文档说明，不开放真实 Runner 执行、真实模型调用或云同步。

## 2026-06-09 变更记录：设置页本地试用状态面板

- 改了什么：`GET /api/runtime-state` 新增 `localTrial` 元信息，设置页新增本地试用状态面板，展示 SQLite/Mock 模式、状态保存位置、API/Web 地址、查看/停止/重置命令和安全边界。
- 为什么改：用户需要在界面内确认当前连的是本地 SQLite 试用版还是 Mock 状态，知道数据存在哪里，以及如何停止或重置，避免把本地试用状态和真实 Runner/真实模型能力混淆。
- 影响模块：`services/api/server.js`、`services/api/db/sqlite-read.js`、`apps/web/index.html`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`docs/demo-checklist.md`、`services/api/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是本地试用可见性增强，不开放真实 Runner 执行、真实模型调用或云同步。

## 2026-06-08 变更记录：迁移到英文路径

- 改了什么：复制项目到 `F:\projects\agent-swarm`，并更新交接说明、API 草案、Mock API `workspacePath` 和设计参考页中的旧中文路径。
- 为什么改：减少中文路径和括号对 Node、Python、Git、Runner 和外部工具造成的编码问题。
- 影响模块：`新窗口交接说明.md`、`services/api/server.js`、`docs/api-draft.md`、`design/index.html`。
- 是否需要同步人类说明书：暂不需要；这是本地开发路径迁移，不改变产品功能。

## 2026-06-10 变更记录：Model Gateway 只读状态骨架

- 改了什么：新增 `GET /api/model-gateway/status`，返回 Model Gateway 是否启用、真实模型调用是否允许、OpenAI / Anthropic / Google Gemini 服务端环境变量是否存在，以及当前安全边界；设置页和集成页动态展示该状态。
- 为什么改：把“以后接大模型”先落成一个统一、可验收、默认禁用的服务边界，避免未来前端、Agent 或 Runner 直接接触 provider SDK 和 API Key。
- 影响模块：`services/api/server.js`、`apps/web/app.js`、`apps/web/styles.css`、`docs/api-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只显示禁用状态，不调用真实模型、不保存或暴露 API Key、不写数据库、不创建任务/审批/Runner job。

## 2026-06-08 变更记录：清理前端旧占位数据

- 改了什么：清理 `apps/web/index.html` 中的旧静态占位数据、通用示例文件、假 API Key 展示和不属于 agent蜂群 的示例文案。
- 为什么改：减少页面加载前的误导信息，让控制台更像当前项目真实状态。
- 影响模块：`apps/web/index.html`。
- 是否需要同步人类说明书：暂不需要；这是前端原型文案收敛。
## 2026-06-10 变更记录：本地 UI 自动冒烟脚本

- 改了什么：新增 `scripts/verify-local-ui.ps1`，用于检查当前运行中的本地 SQLite 试用版 API、runtime safety、Model Gateway 禁用状态、Runner 安全边界，并通过 Microsoft Edge + Playwright CLI 冒烟首页、任务、审批、运行、设置和集成页。
- 为什么改：把前面手工浏览器验收固化成一条可重复命令，减少后续每次小修时漏看控制台错误、假按钮或安全边界文案的风险。
- 影响模块：`scripts/verify-local-ui.ps1`、`scripts/README.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是开发验收脚本，不新增真实模型调用、Runner 执行、云同步或权限能力。

## 2026-06-10 变更记录：Model Gateway dry-run 规格

- 改了什么：`docs/api-draft.md` 新增计划中的 `POST /api/model-gateway/dry-run` 草案，定义请求体、响应体、sideEffects 和 dry-run 验收边界。
- 为什么改：把“接真实模型前先做什么”拆成更小的后端验证步骤；dry-run 只验证 provider、env var 和安全开关，不发真实模型请求。
- 影响模块：`docs/api-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是未实现的后端规格，不改变当前本地试用能力。注意：dry-run 禁止写 SQLite、创建任务/审批/Runner job、触发 Agent、调用真实模型、记录 prompt/result，只是 dry-run 阶段限制，不代表长期产品不做这些能力。

## 2026-06-10 变更记录：Model Gateway dry-run 只读接口

- 改了什么：`services/api/server.js` 新增 `POST /api/model-gateway/dry-run`，返回 provider 是否支持、服务端 env var 是否存在、请求体校验结果和 sideEffects；`scripts/verify-local-ui.ps1` 增加 dry-run 验收断言。
- 为什么改：把真实模型接入前的第一步落成可验证后端接口，确认服务边界和禁用态行为稳定，而不是直接连接 provider SDK。
- 影响模块：`services/api/server.js`、`scripts/verify-local-ui.ps1`、`scripts/README.md`、`docs/api-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不调用真实模型、不写 SQLite、不创建任务/审批/Runner job、不触发 Agent、不记录 prompt/result。

## 2026-06-10 变更记录：Model Gateway dry-run 前端预览

- 改了什么：设置页和集成页新增 Model Gateway dry-run 只读预览，展示 provider、env var、provider call 边界和 sideEffects；`verify-local-ui.ps1` 增加页面级断言。
- 为什么改：让用户能在界面上看到“离真实模型还差什么”，同时仍保持禁用态和无副作用边界。
- 影响模块：`apps/web/app.js`、`apps/web/styles.css`、`scripts/verify-local-ui.ps1`、`scripts/README.md`、`docs/api-draft.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前只是只读预览，不新增真实模型调用、API Key 输入、任务/审批/Runner job 创建或 Agent 触发。

## 2026-06-10 变更记录：Model Gateway manual connectivity test 规格

- 改了什么：`docs/api-draft.md` 新增计划中的 `POST /api/model-gateway/connectivity-test` 规格，定义人工真实 provider 连通性测试的请求、响应、sideEffects 和启用前验收条件；`docs/demo-checklist.md`、`scripts/README.md` 和开发路线同步说明当前仍不得真实调用 provider。
- 为什么改：dry-run 已经能验证请求形状和安全边界，下一阶段需要先把“人工真实连通性测试”写成可验收规格，避免直接接 SDK、发真实请求或把它误当成 Agent/Runner 能力。
- 影响模块：`docs/api-draft.md`、`docs/demo-checklist.md`、`scripts/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是后续实现前的技术规格，不改变当前本地试用能力。当前仍禁止接真实 provider SDK、发真实 OpenAI/Anthropic/Gemini 请求、保存 prompt/result、创建任务/审批/Runner job 或触发 Agent。

## 2026-06-10 变更记录：Model Gateway connectivity-test 禁用态 stub

- 改了什么：`POST /api/model-gateway/connectivity-test` 禁用态后端 stub 只校验 provider、model、purpose、secondConfirm 和 confirmText，经由 disabled provider adapter stub 返回 `blocked / feature_disabled`、`realProviderRequestAttempted=false` 和全 false sideEffects；`scripts/verify-local-ui.ps1` 增加正向 blocked 和反向用例验收。
- 为什么改：把上一轮规格落成可回归的安全边界，先证明“接口存在但不会真实连通”，再考虑后续 provider adapter 和 feature flag。
- 影响模块：`services/api/server.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`docs/demo-checklist.md`、`scripts/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不调用真实模型、不接 provider SDK、不写 SQLite/runtime state、不创建任务/审批/Runner job、不触发 Agent、不保存 prompt/result 或 provider response。

## 2026-06-10 变更记录：Model Gateway 后端隔离层

- 改了什么：新增 `services/api/model-gateway.js`，集中管理 provider metadata、env var presence、dry-run 校验和 connectivity-test 禁用态 stub；`services/api/server.js` 改为只做 HTTP route wiring。
- 为什么改：先把 Model Gateway 边界从通用 API server 中拆出，避免后续 feature flag 或 provider adapter 设计散落在路由层，同时保持当前所有接口行为不变。
- 影响模块：`services/api/model-gateway.js`、`services/api/server.js`、`services/api/README.md`、`docs/api-draft.md`、`scripts/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是后端结构整理，不新增真实 provider SDK、真实模型请求、API Key 输入/保存、Agent 触发或 Runner job 创建。

## 2026-06-10 变更记录：Model Gateway feature flag 边界

- 改了什么：`services/api/model-gateway.js` 在 status、dry-run 和 connectivity-test 响应中返回 `featureFlags`，公开 `AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST` 的请求状态；`scripts/verify-local-ui.ps1` 抽出 feature flag 验收 helper，并覆盖所有 Model Gateway 正反用例。
- 为什么改：在进入 provider adapter 设计前，先把“手动连通性测试即使被请求开启也不能真实生效”的边界固化成 API 字段和回归断言。
- 影响模块：`services/api/model-gateway.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`services/api/README.md`、`scripts/README.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不调用真实模型、不接 provider SDK、不写 SQLite/runtime state、不创建任务/审批/Runner job、不触发 Agent、不保存 prompt/result 或 provider response。

## 2026-06-10 变更记录：Model Gateway provider adapter 草案

- 改了什么：`docs/api-draft.md` 新增 Model Gateway provider adapter 草案，定义后端隔离层归属、输入/输出形状、result/errorCategory 枚举、脱敏规则、超时和响应大小限制，以及实现前验收条件。
- 为什么改：在任何真实 SDK 或网络请求进入仓库前，先固定 adapter 的安全契约，避免 UI、Agent、Runner 或通用路由层直接接触 provider SDK、API Key、prompt 或 provider response。
- 影响模块：`docs/api-draft.md`、`services/api/README.md`、`scripts/README.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是技术草案，不新增真实 provider SDK、真实模型请求、API Key 输入/保存、Agent 触发、Runner job 创建、prompt/result/provider response 存储。

## 2026-06-10 变更记录：Model Gateway disabled adapter stub

- 改了什么：新增 `services/api/model-gateway-adapters.js`，提供 `disabled_provider_connectivity_adapter`；`POST /api/model-gateway/connectivity-test` 改为经由该 disabled adapter 返回 `blocked / feature_disabled`、`realProviderRequestAttempted=false`、`providerResponseStored=false` 和 `redactionApplied=true`；`scripts/verify-local-ui.ps1` 增加对应断言。
- 为什么改：把上一轮 provider adapter 草案落成可回归的禁用态服务边界，先证明 adapter 插槽存在但仍不会真实连通 provider。
- 影响模块：`services/api/model-gateway-adapters.js`、`services/api/model-gateway.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`services/api/README.md`、`scripts/README.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不调用真实模型、不接 provider SDK、不读取或保存 API Key、不写 SQLite/runtime state、不创建任务/审批/Runner job、不触发 Agent、不保存 prompt/result 或 provider response。

## 2026-06-10 变更记录：Model Gateway provider-specific disabled registry

- 改了什么：扩展 `services/api/model-gateway-adapters.js`，为 OpenAI、Anthropic 和 Google Gemini 增加 provider-specific disabled adapter registry；`GET /api/model-gateway/status` 和 `POST /api/model-gateway/connectivity-test` 可返回 `providerAdapterId` 与 `providerAdapterMode=disabled`；`scripts/verify-local-ui.ps1` 覆盖三家 provider 的禁用 adapter 断言。
- 为什么改：在进入任何真实 provider SDK 或网络请求前，先把 provider 级 adapter 边界固定为可回归元数据，证明每个 provider 都只能走禁用态路径。
- 影响模块：`services/api/model-gateway-adapters.js`、`services/api/model-gateway.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`docs/demo-checklist.md`、`scripts/README.md`、`services/api/README.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是后端禁用 adapter 元数据和验收补强，不新增真实模型调用、provider SDK、API Key 读取/保存、SQLite/runtime state 写入、任务/审批/Runner job 创建、Agent 触发或 prompt/result/provider response 存储。

## 2026-06-10 变更记录：Model Gateway real-provider phase gate

- 改了什么：在 `docs/api-draft.md` 和开发路线中补充真实 provider 阶段准入条件，要求真实 adapter 必须独立提交、显式改变 feature flag 边界、保持后端手动固定最小 ping、覆盖 blocked/missing-key/unsupported/timeout/provider-error 和 no-side-effect 验收。
- 为什么改：用户已确认继续推进到下一阶段前置工作，但当前仍不能直接开放真实模型调用；需要先把进入真实连通性测试的门槛写成可执行清单。
- 影响模块：`docs/api-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；这是技术准入条件，不改变当前本地试用能力，仍禁止真实 OpenAI/Anthropic/Gemini 请求。

## 2026-06-10 变更记录：Model Gateway first real-provider spec freeze

- 改了什么：在 `docs/api-draft.md` 固定第一家真实 provider 手动连通性测试规格：一次只做一家 provider、请求体沿用当前 stub、route 不接 SDK、adapter 只读所选 provider 服务端 env var、先验证超时和响应大小限制、无真实凭据时仍可通过 blocked/missing-key/no-side-effect 验收。
- 为什么改：把“下一阶段第一家真实 provider 怎么开始”限定到最小、可验收、可回滚的范围，避免一次性接入多家 provider 或把连通性测试扩展成通用聊天/Agent/Runner 能力。
- 影响模块：`docs/api-draft.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍未实现真实 provider 请求，只冻结后续实现规格和验收条件。

## 2026-06-10 变更记录：Model Gateway manual connectivity preflight gate

- 改了什么：`services/api/model-gateway.js` 新增后端 `modelGatewayConnectivityPreflight(...)` 闸门函数；`connectivity-test` 响应包含 `preflight` 对象但顶层仍经 disabled adapter 返回 blocked；`verify-local-ui.ps1` 直接调用 helper 覆盖 feature disabled、missing key、unsupported provider、unsupported model、invalid purpose、timeout、provider error 和 no-side-effect 断言。
- 为什么改：在第一家真实 provider 前先把所有保险丝和失败路径固定下来，确保 feature flag、二次确认、server-side key、timeout/body limit 与无副作用边界都可回归。
- 影响模块：`services/api/model-gateway.js`、`services/api/model-gateway-adapters.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不接 provider SDK、不发 OpenAI/Anthropic/Gemini 请求、不读取或保存 API Key、不写 SQLite/runtime state、不创建任务/审批/Runner job、不触发 Agent。

## 2026-06-10 变更记录：Model Gateway OpenAI-compatible relay first-provider candidate

- 改了什么：把第一家真实 provider 候选从官方 OpenAI 修正为 OpenAI-compatible relay，并在 `docs/api-draft.md`、`docs/demo-checklist.md` 和开发路线中固定 relay 专用 env var 草案：`AGENT_SWARM_OPENAI_COMPAT_API_KEY` 与 `AGENT_SWARM_OPENAI_COMPAT_BASE_URL`。
- 为什么改：用户说明没有官方 OpenAI key，只有中转 key；为了避免官方 OpenAI 与中转 provider 混用，必须把 provider id、key env var 和 base URL 边界拆开记录。
- 影响模块：`docs/api-draft.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍未实现真实 relay 或官方 OpenAI 请求，官方 OpenAI、Anthropic 和 Google Gemini 也继续保持 disabled adapter。

## 2026-06-10 变更记录：Model Gateway OpenAI-compatible relay disabled preflight

- 改了什么：新增 `openai_compat` 禁用 provider 元数据和 `openai_compat_disabled_connectivity_adapter`；`modelGatewayConnectivityPreflight(...)` 增加 relay 专用 key/base URL 存在性与安全形状检查；`verify-local-ui.ps1` 覆盖 `missing_base_url`、`invalid_base_url`、安全 URL 但 feature disabled、disabled adapter 和 no-side-effect 断言。
- 为什么改：在接中转真实请求前，先固定 relay 与官方 OpenAI 的隔离边界，确认 base URL 只能来自服务端 env，且不会被前端或请求体覆盖。
- 影响模块：`services/api/model-gateway.js`、`services/api/model-gateway-adapters.js`、`scripts/verify-local-ui.ps1`、`docs/api-draft.md`、`docs/demo-checklist.md`、`dev-docs/下一步开发路线.md`、`dev-docs/AI开发维护手册.md`。
- 是否需要同步人类说明书：暂不需要；当前仍不接 relay SDK/HTTP 请求，不读取或保存真实 key/base URL 值，不写 SQLite/runtime state，不创建任务/审批/Runner job，不触发 Agent，不保存 prompt/result/provider body。

## 2026-06-10 Change log: Model Gateway OpenAI-compatible relay adapter interface checkpoint

- What changed: added an interface-disabled future relay adapter boundary in `services/api/model-gateway-adapters.js`, exposed `futureProviderAdapterId=openai_compat_manual_connectivity_adapter` as metadata, and extended `scripts/verify-local-ui.ps1` to verify relay interface failure paths without real credentials or network calls.
- Why: before any real OpenAI-compatible relay request is implemented, the adapter contract must prove that keys and base URLs remain server-side, prompts and Agent/Runner context are rejected, failure categories stay coarse, and all side effects remain false.
- Impacted modules: `services/api/model-gateway-adapters.js`, `services/api/model-gateway.js`, `scripts/verify-local-ui.ps1`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a backend safety checkpoint and does not enable real relay calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway relay provider information checklist

- What changed: added `docs/relay-provider-info-checklist.md` as a safe, non-secret checklist for confirming the first OpenAI-compatible relay before implementation.
- Why: the operator does not yet know the relay endpoint family, model id, or request shape. Capturing only documentation facts first prevents mixing official OpenAI behavior with relay behavior and avoids accidental key, prompt, or provider-body leakage.
- Impacted modules: `docs/relay-provider-info-checklist.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a development checklist and does not enable real relay calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway DeepSeek provider information checklist

- What changed: added `docs/deepseek-provider-info-checklist.md` as a documentation-only checklist for the official DeepSeek API provider candidate.
- Why: the unknown relay candidate lacks public endpoint and model details, while DeepSeek has public docs for OpenAI-compatible Chat Completions. Recording only non-secret facts lets the project compare provider candidates without handling API keys or making real requests.
- Impacted modules: `docs/deepseek-provider-info-checklist.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a development checklist and does not enable real DeepSeek calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway cheng.pink relay facts captured

- What changed: updated `docs/relay-provider-info-checklist.md` with operator-provided cheng.pink OpenAI-compatible relay facts, including base URL shape, endpoint paths, available model ids, minimal test model, Bearer auth, and streaming support.
- Why: the relay administrator provided enough non-secret information to draft a fixed minimal ping later, while still keeping API keys and account data outside Git, logs, SQLite, and frontend code.
- Impacted modules: `docs/relay-provider-info-checklist.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a documentation checkpoint and does not enable real relay calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway cheng.pink fixed manual ping spec

- What changed: added `docs/cheng-relay-manual-ping-spec.md` to freeze the future cheng.pink non-stream manual ping request, URL normalization rules, coarse result contract, error mapping, and implementation-order checklist.
- Why: before adding any request-builder code or real relay adapter, the project needs a fixed reviewed target that prevents duplicated `/v1` paths, client prompt injection, API key exposure, provider body storage, and hidden side effects.
- Impacted modules: `docs/cheng-relay-manual-ping-spec.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a documentation checkpoint and does not enable real relay calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway cheng.pink request builder checkpoint

- What changed: added pure local cheng.pink request-builder helpers in `services/api/model-gateway-adapters.js` and extended `scripts/verify-local-ui.ps1` to verify URL normalization, fixed body shape, unsafe URL rejection, unsupported model rejection, and no-side-effect guarantees.
- Why: the project needs deterministic request-shape checks before any real relay adapter can exist. This catches duplicated `/v1` paths and client-controlled prompt/header/base URL mistakes while still making zero provider network requests.
- Impacted modules: `services/api/model-gateway-adapters.js`, `scripts/verify-local-ui.ps1`, `docs/api-draft.md`, `docs/demo-checklist.md`, `scripts/README.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is a backend verification checkpoint and does not enable real relay calls, real model calls, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Model Gateway dedicated verification script

- What changed: added `scripts/verify-model-gateway.ps1` as a dedicated non-browser Model Gateway acceptance script and documented it in `scripts/README.md`, `docs/demo-checklist.md`, `docs/api-draft.md`, and `dev-docs/下一步开发路线.md`.
- Why: Model Gateway deep checks had grown inside the browser UI smoke script. A dedicated entry keeps backend/API/helper acceptance easy to run without opening a browser or mixing it with UI smoke coverage.
- Impacted modules: `scripts/verify-model-gateway.ps1`, `scripts/README.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is an acceptance-script split and does not enable real relay calls, real model calls, provider SDKs, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Narrow local UI smoke script

- What changed: narrowed `scripts/verify-local-ui.ps1` to browser UI smoke coverage only, while keeping Model Gateway deep acceptance in `scripts/verify-model-gateway.ps1`; updated `scripts/README.md`, `docs/demo-checklist.md`, and `dev-docs/下一步开发路线.md`.
- Why: after the dedicated Model Gateway verification script existed, duplicating backend helper/preflight/adapter assertions inside the browser UI script made the UI smoke path too broad and slower to reason about.
- Impacted modules: `scripts/verify-local-ui.ps1`, `scripts/README.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`.
- Human docs sync: not required; this is test-script scope cleanup and does not enable real provider calls, provider SDKs, Runner execution, cloud sync, or permission changes.

## 2026-06-10 Change log: Agent permission contract

- What changed: added `docs/agent-permission-contract.md` and linked it from API, data model, Runner safety, demo, roadmap, and handoff docs.
- Why: future requests such as giving an architect Agent or all Agents "full permissions" need an explicit capability split so planning/orchestration/request rights cannot be confused with self-approval, direct execution, or raw secret access.
- Impacted modules: `docs/agent-permission-contract.md`, `docs/api-draft.md`, `docs/data-model-draft.md`, `docs/runner-safety-acceptance.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`, `dev-docs/新窗口交接说明.md`.
- Human docs sync: not required; this is a technical permission contract and does not enable real permissions, Runner execution, real provider calls, cloud sync, or secret access.

## 2026-06-10 Change log: Agent permission profile checks

- What changed: added `services/api/agent-permissions.js` and `scripts/verify-agent-permissions.ps1` to validate mock Agent permission profile expansion, forbidden capabilities, unknown capabilities, `all=true` rejection, and all-false side effects.
- Why: after documenting the Agent permission contract, the project needs a small executable check so future "full permission" profile changes cannot accidentally include self-approval, direct Runner execution, direct local operations, network requests, or raw-secret access.
- Impacted modules: `services/api/agent-permissions.js`, `scripts/verify-agent-permissions.ps1`, `docs/agent-permission-contract.md`, `docs/api-draft.md`, `docs/data-model-draft.md`, `docs/demo-checklist.md`, `docs/module-stability-map.md`, `scripts/README.md`, roadmap, maintenance, and handoff docs.
- Human docs sync: not required; this is mock/profile validation only and does not enable runtime authorization, Agent config writes, Runner execution, real model calls, cloud sync, or secret access.

## 2026-06-10 Change log: Agent permission change-request validation

- What changed: wired the Agent permission helper into `POST /api/agents/:agentId/change-requests` for `changeType=permission`, updated the web permission preview to submit safe profile names, stored successful `permissionValidation` in approval `changeRequest`, and expanded Mock/SQLite flow verification with invalid permission rejection cases.
- Why: permission profiles should be checked at the first write boundary so forbidden capabilities cannot create an approval, SQLite row, runtime-state entry, Runner job, Agent trigger, model call, or secret read.
- Impacted modules: `services/api/server.js`, `services/api/agent-permissions.js`, `scripts/sqlite/sqlite_write.py`, `apps/web/app.js`, `scripts/verify-mock-flows.ps1`, `scripts/verify-sqlite-flows.ps1`, `services/api/README.md`, `docs/api-draft.md`, `docs/demo-checklist.md`, `scripts/README.md`, roadmap, maintenance, and handoff docs.
- Human docs sync: not required; this keeps the current Mock/disabled safety boundary and does not enable real Agent config writes, runtime authorization, Runner execution, real model calls, cloud sync, or secret access.

## 2026-06-10 Change log: Local verification port isolation

- What changed: kept `8787` reserved for human local trial/manual development, documented the port policy, moved self-contained Mock flow verification to isolated `8789`, and added startup guards so Mock/SQLite flow scripts fail instead of attaching to an existing API on their isolated ports.
- Why: automated AI verification must not connect to or mutate the user's current `8787` local trial, and it must not silently validate against stale server code.
- Impacted modules: `AGENTS.md`, `scripts/verify-mock-flows.ps1`, `scripts/verify-sqlite-flows.ps1`, `scripts/README.md`, `docs/demo-checklist.md`, roadmap, maintenance, and handoff docs.
- Human docs sync: not required; this is verification hygiene and does not enable real Runner execution, real model calls, cloud sync, broad runtime permissions, or secret access.

## 2026-06-10 Change log: Agent config application safety regression

- What changed: expanded Mock and SQLite flow verification to assert that approved `agent_config` approvals only create `pending_apply` application records, keep `runnerJobId` empty, create no Runner queue item, and leave Agent permissions unchanged after approval and Mock apply.
- Why: the permission change-request boundary is only useful if the next approval/application step cannot accidentally become a real Agent config write or Runner job path.
- Impacted modules: `scripts/verify-mock-flows.ps1`, `scripts/verify-sqlite-flows.ps1`, `scripts/README.md`, `docs/demo-checklist.md`, roadmap, and maintenance docs.
- Human docs sync: not required; this is regression coverage only and does not enable real Agent config writes, Runner execution, real model calls, cloud sync, or broad runtime permissions.

## 2026-06-10 Change log: Agent config apply dry-run and rollback spec

- What changed: added `docs/agent-config-apply-dry-run-spec.md` and linked it from API, data model, Runner safety, demo, roadmap, maintenance, handoff, and module stability docs.
- Why: before real Agent config writes can exist, the project needs a reviewed dry-run gate, transaction rule, versioning rule, side-effect boundary, and rollback approval strategy.
- Impacted modules: `docs/agent-config-apply-dry-run-spec.md`, `docs/api-draft.md`, `docs/data-model-draft.md`, `docs/runner-safety-acceptance.md`, `docs/demo-checklist.md`, `docs/module-stability-map.md`, roadmap, maintenance, and handoff docs.
- Human docs sync: not required; this is documentation/specification only and does not implement a dry-run endpoint, real Agent config writes, Runner execution, real model calls, cloud sync, or broad runtime permissions.

## 2026-06-10 Change log: Module stability map

- What changed: added `docs/module-stability-map.md` and linked it from `README.md`, `docs/demo-checklist.md`, roadmap, maintenance, and handoff docs.
- Why: the project skeleton is stable enough for small verified iteration, but future deletion/refactor work needs a clear map of P0 anchors, contracts, refactorable areas, runtime state, and protected paths.
- Impacted modules: `docs/module-stability-map.md`, `README.md`, `docs/demo-checklist.md`, `dev-docs/下一步开发路线.md`, `dev-docs/AI开发维护手册.md`, `dev-docs/新窗口交接说明.md`.
- Human docs sync: not required; this is a maintenance map and does not change runtime behavior, enable Runner execution, enable real provider calls, or change permissions.
