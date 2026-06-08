# agent蜂群 AI开发维护手册

## 1. 给后续 AI 的说明

你正在维护 agent蜂群。

这是一个多 AI 智能体协作开发平台，不是普通聊天应用，也不是单 Agent 代码生成器。

继续开发、修 bug、升级功能前，必须先理解：

- 本文件是给 AI 开发工具看的技术规格。
- [[人类说明书]] 是给用户和产品讨论看的说明。
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
[[人类说明书]]
```

插入位置：

- 使用方式变化：更新“核心用户流程”。
- 产品范围变化：更新“第一版必须做”或“第一版暂时不做”。
- 安全/权限变化：更新“权限系统”或“API Key 管理”。
- 重大决策：更新“变更记录”。

### 修改技术实现时

更新：

```text
[[AI开发维护手册]]
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
前端交互反推架构调整.md
下一步开发路线.md
docs/api-draft.md
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
