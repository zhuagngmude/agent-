# agent蜂群数据模型草案

日期：2026-06-09

阶段：MVP-0.2 数据库接入前设计。

本文只定义真实数据库接入前的数据模型草案，不实现 SQLite/PostgreSQL，不修改 Mock API 运行逻辑。

## 1. 设计目标

当前系统已经有 Web App、Mock API、本地 runtime state 和关键状态流转。下一步接真实数据库前，必须先固定核心实体、字段命名和关系，避免边写边改表。

目标：

- 让 Mock API 数据结构可以平滑迁移到数据库。
- 让审批、Runner job、Agent 配置变更和回滚审查可追溯。
- 让任务、工作流、知识库和 Git checkpoint 可以统一关联到项目。
- 先覆盖 MVP-0.2 必需模型，不提前设计计费、团队权限、真实模型调用明细。

## 2. 命名规范

- 表名使用复数 snake_case，例如 `projects`、`agent_config_applications`。
- 字段名使用 snake_case，例如 `created_at`、`agent_id`。
- 主键统一使用字符串 ID，字段名为 `id`。
- 外键使用 `<entity>_id`，例如 `project_id`、`approval_id`。
- 状态字段统一命名为 `status`。
- 时间字段统一使用 ISO datetime，字段名为 `created_at`、`updated_at`、`approved_at`。
- 可变结构先用 JSON 字段承接，例如 `permissions`、`changes`、`diff_preview`。
- Agent 权限 JSON 必须遵守 `docs/agent-permission-contract.md`，不能用单个 `all=true` 表示规划、审批、执行和密钥访问。

暂定数据库类型映射：

```text
id/string       TEXT
datetime        TEXT
boolean         BOOLEAN
number          INTEGER / REAL
json            JSON 或 TEXT(JSON)
status enum     TEXT + 应用层校验
```

## 3. 核心关系概览

```text
projects
  ├─ agents
  │   ├─ agent_relationships
  │   ├─ agent_config_versions
  │   └─ agent_config_applications
  ├─ tasks
  ├─ approvals
  │   ├─ runner_jobs
  │   └─ agent_config_applications
  ├─ workflows
  │   └─ workflow_steps
  ├─ knowledge_updates
  ├─ git_checkpoints
  └─ runtime_events
```

## 4. 表草案

### projects

用途：项目主表。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 项目 ID，例如 `project_agent_swarm` |
| name | TEXT | 是 | 项目名称 |
| status | TEXT | 是 | `running`、`paused`、`archived` |
| phase | TEXT | 否 | 当前阶段，例如 `MVP-0.2` |
| description | TEXT | 否 | 项目说明 |
| workspace_path | TEXT | 否 | 本地工作区路径 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

### agents

用途：Agent 配置当前态。注意：后续真实修改必须经过审批和应用流程。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | Agent ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| name | TEXT | 是 | Agent 名称 |
| role | TEXT | 是 | `architect`、`frontend`、`docs`、`reviewer` 等 |
| status | TEXT | 是 | `running`、`idle`、`waiting`、`failed`、`disabled` |
| version | TEXT | 否 | Agent 版本 |
| model | TEXT | 否 | 当前模型标识 |
| can_spawn_sub_agents | BOOLEAN | 是 | 是否允许创建子 Agent |
| max_sub_agents | INTEGER | 是 | 最大子 Agent 数 |
| permissions | JSON | 是 | 权限列表或 profile 展开结果，必须区分 planning / orchestration / request / approval / execution / secret access |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

索引：

- `idx_agents_project_id`
- `idx_agents_role`
- `idx_agents_status`

权限字段约束：

- `permissions` 可以包含 `architect_admin`、`executor_agent`、`reviewer_agent` 等 profile 名称，但应用层必须按 `docs/agent-permission-contract.md` 展开成明确能力。
- `architect_admin` 只能代表最高规划、编排和申请权限，不能隐含 `canApproveHighRisk`、`canApproveOwnRequest`、`canExecuteRunnerJob`、`canAccessRawSecrets`。
- 如果后续新增 `all_agents_full_management`，它也只能表示广义管理权限，不能代表自批、自执行、写文件、跑命令、改 Git、发网络请求或读取原始密钥。
- 任何包含审批、执行或密钥相关能力的变更都必须走 Approval Service，并写入 `approvals` / `agent_config_applications` / `runtime_events`。
- 当前 `services/api/agent-permissions.js` 只做 Mock profile 展开和禁止能力校验；它不是数据库约束、路由授权层、真实 RBAC/ABAC 系统或运行时权限执行路径。

### agent_relationships

用途：记录父子 Agent、汇总目标和派生深度。单独成表是为了避免 `agents` 表内数组字段难查。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 关系 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| parent_agent_id | TEXT | 否 | 父 Agent |
| child_agent_id | TEXT | 是 | 子 Agent |
| reports_to_agent_id | TEXT | 否 | 汇总目标 Agent |
| spawn_depth | INTEGER | 是 | 派生深度，MVP 约束 `<= 1` |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

约束：

- `child_agent_id` 应唯一，避免一个子 Agent 同时属于多个父 Agent。
- 子 Agent 不允许自行扩权，权限变化必须走审批。

### agent_config_versions

用途：记录 Agent 配置真实写入后的版本历史。第一版真实应用配置时，必须在同一事务内更新 `agents` 当前态并写入版本记录。

真实写入前必须先通过 `docs/agent-config-apply-dry-run-spec.md` 中的 dry-run 验收。MVP-0.2 当前不会写入该表；`agent_config_applications.status = applied` 仍只代表 Mock 状态流转。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 版本记录 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| agent_id | TEXT | 是 | 关联 `agents.id` |
| version | INTEGER | 是 | Agent 配置版本号，从 1 递增 |
| approval_id | TEXT | 是 | 来源审批 |
| application_id | TEXT | 是 | 来源应用记录 |
| config_snapshot | JSON | 是 | 应用后的 Agent 配置快照 |
| changes | JSON | 是 | 本次字段变更 before/after |
| applied_by | TEXT | 是 | 触发应用的人 |
| applied_at | TEXT | 是 | 应用时间 |
| created_at | TEXT | 是 | 创建时间 |

关键约束：

- 只能由 `agent_config_applications.status = applied` 的记录生成。
- `agent_id + version` 应唯一。
- `config_snapshot` 不得包含 API Key、模型 Key 明文或本地敏感路径。
- 回滚不得直接删除版本记录；必须重新创建审批和新的版本。
- 真实写入必须和 `agents` 当前态更新处于同一事务；失败时不得产生半写入版本。
- `services/api/agent-config-version-history.js` 当前只是只读 helper，用于规范化已加载的版本行并选择回滚来源；它不直接读取 SQLite、不暴露 HTTP 路由、不写该表。

### tasks

用途：任务队列和任务当前态。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 任务 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| title | TEXT | 是 | 任务标题 |
| description | TEXT | 否 | 任务说明 |
| status | TEXT | 是 | `queued`、`running`、`blocked`、`waiting_user`、`completed`、`failed`、`cancelled` |
| priority | TEXT | 否 | `high`、`medium`、`low` |
| assigned_agent_id | TEXT | 否 | 负责人 Agent |
| risk_level | TEXT | 否 | `high`、`medium`、`low` |
| related_files | JSON | 否 | 关联文件列表 |
| requires_approval | BOOLEAN | 是 | 是否需要审批 |
| depends_on | JSON | 否 | 依赖任务 ID 列表 |
| started_at | TEXT | 否 | 开始时间 |
| completed_at | TEXT | 否 | 完成时间 |
| failed_at | TEXT | 否 | 失败时间 |
| cancelled_at | TEXT | 否 | 取消时间 |
| failure_reason | TEXT | 否 | 失败原因 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

后续可拆：

- 如果依赖关系复杂，再拆 `task_dependencies`。
- 如果需要完整状态历史，再由 `runtime_events` 或 `task_events` 承接。

### approvals

用途：Approval Service 主表。Runner 和 Agent 配置变更都必须先进入这里。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 审批 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| status | TEXT | 是 | `draft`、`pending`、`approved`、`rejected`、`patch_only`、`executed`、`rolled_back`、`expired` |
| risk_level | TEXT | 是 | `high`、`medium`、`low` |
| request_agent_id | TEXT | 否 | 发起 Agent |
| request_agent_name | TEXT | 否 | 发起 Agent 名称快照 |
| target_service | TEXT | 是 | `runner`、`agent_config` 等 |
| operation_types | JSON | 是 | 操作类型列表 |
| reason | TEXT | 否 | 申请原因 |
| checkpoint_required | BOOLEAN | 是 | 是否需要 Git checkpoint |
| checkpoint_created | BOOLEAN | 是 | checkpoint 是否已创建 |
| checkpoint_commit | TEXT | 否 | checkpoint commit |
| affected_files | JSON | 否 | 影响文件列表 |
| diff_summary | TEXT | 否 | diff 摘要 |
| diff_preview | JSON | 否 | diff 预览 |
| requires_second_confirm | BOOLEAN | 是 | 是否需要二次确认 |
| change_request | JSON | 否 | Agent 配置变更草案或其它申请上下文 |
| runner_job_id | TEXT | 否 | 关联 Runner job。`target_service=agent_config` 时必须为空 |
| patch_artifact_id | TEXT | 否 | 只生成补丁时的产物 ID |
| reject_reason | TEXT | 否 | 拒绝原因 |
| approved_at | TEXT | 否 | 批准时间 |
| rejected_at | TEXT | 否 | 拒绝时间 |
| patch_only_at | TEXT | 否 | 只生成补丁时间 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

关键约束：

- `target_service = agent_config` 时，不得生成 Runner job。
- 高风险审批必须 `requires_second_confirm = true`。
- Runner 执行前必须有 Approval 记录。

### runner_jobs

用途：已批准 Runner 操作的只读执行队列。MVP-0.2 不真实执行。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | Runner job ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| approval_id | TEXT | 是 | 来源审批 |
| task_id | TEXT | 否 | 关联任务 |
| status | TEXT | 是 | `queued`、`running`、`succeeded`、`failed`、`cancelled` |
| operation_types | JSON | 是 | 操作类型 |
| affected_files | JSON | 否 | 影响文件 |
| checkpoint_commit | TEXT | 否 | Git checkpoint |
| safety_note | TEXT | 否 | 安全说明 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

关键约束：

- 只有 `approvals.status = approved` 的 Runner 审批可以生成。
- `agent_config` 审批不得生成 Runner job。
- 后续真实执行前必须补充执行日志和回滚策略。

### agent_config_applications

用途：Agent 配置审批通过后的待应用/已应用/已取消记录。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 应用记录 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| approval_id | TEXT | 是 | 来源审批 |
| agent_id | TEXT | 是 | 目标 Agent |
| agent_name | TEXT | 否 | Agent 名称快照 |
| change_type | TEXT | 是 | `model`、`spawn`、`permission` 等 |
| changes | JSON | 是 | 字段变更 before/after |
| status | TEXT | 是 | `pending_apply`、`applied`、`cancelled` |
| applied_at | TEXT | 否 | Mock 应用状态流转时间 |
| applied_by | TEXT | 否 | 触发应用的人 |
| apply_confirm_text | TEXT | 否 | 二次确认文本 |
| cancelled_at | TEXT | 否 | 取消时间 |
| cancelled_by | TEXT | 否 | 取消人 |
| cancel_reason | TEXT | 否 | 取消原因 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

关键约束：

- 只能由 `target_service=agent_config` 且已批准的审批生成。
- 当前 `applied` 只表示 Mock 状态流转，不代表 Agent 配置真实写入。
- 真正回滚必须重新创建审批申请，不能直接改 Agent 配置。
- 真实写入前必须先通过 Agent config apply dry-run；dry-run 规格见 `docs/agent-config-apply-dry-run-spec.md`。

### workflows

用途：工作流主表。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 工作流 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| name | TEXT | 是 | 工作流名称 |
| status | TEXT | 是 | `active`、`draft`、`disabled` |
| description | TEXT | 否 | 说明 |
| steps | JSON | 否 | 当前前端流程步骤展示数据 |
| stats | JSON | 否 | 当前前端流程统计展示数据 |
| nodes | JSON | 否 | 当前 MVP 可先保留节点 JSON |
| edges | JSON | 否 | 当前 MVP 可先保留连线 JSON |
| updated_at | TEXT | 是 | 更新时间 |
| created_at | TEXT | 是 | 创建时间 |

### workflow_steps

用途：工作流步骤。若 `nodes/edges` 后续需要更强查询能力，再拆分更多表。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 步骤 ID |
| workflow_id | TEXT | 是 | 关联 `workflows.id` |
| project_id | TEXT | 是 | 冗余项目 ID，便于查询 |
| name | TEXT | 是 | 步骤名称 |
| detail | TEXT | 否 | 步骤说明 |
| progress | TEXT | 否 | 当前展示为百分比字符串 |
| tone | TEXT | 否 | 前端展示 tone |
| sort_order | INTEGER | 是 | 排序 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

### knowledge_updates

用途：知识库/文档同步记录。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 更新记录 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| document | TEXT | 是 | 文档路径 |
| section | TEXT | 否 | 文档章节 |
| status | TEXT | 是 | `synced`、`pending`、`failed` |
| related_feature | TEXT | 否 | 关联功能 |
| updated_at | TEXT | 是 | 更新时间 |
| created_at | TEXT | 是 | 创建时间 |

### runner_status

用途：本地 Runner 连接状态只读展示。当前只展示安全边界，不执行本地能力。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | Runner 状态记录 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| connected | BOOLEAN | 是 | 是否连接 |
| runner_id | TEXT | 是 | Runner ID |
| version | TEXT | 是 | Runner 版本 |
| workspace_path | TEXT | 否 | 本地工作区路径 |
| permissions | JSON | 是 | 权限边界，例如写文件/执行命令是否需要审批 |
| last_heartbeat_at | TEXT | 否 | 最后心跳时间 |
| created_at | TEXT | 是 | 创建时间 |
| updated_at | TEXT | 是 | 更新时间 |

关键约束：

- 该表只表示状态，不代表 Runner 可以执行。
- `workspace_path` 只能用于本地展示，后续云端同步前必须脱敏或改为 alias。

### git_checkpoints

用途：Git 保存点记录。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | checkpoint ID，可等于 commit hash |
| project_id | TEXT | 是 | 关联 `projects.id` |
| commit_hash | TEXT | 是 | Git commit |
| message | TEXT | 是 | commit message |
| type | TEXT | 否 | `feature`、`docs`、`fix` 等 |
| related_task_id | TEXT | 否 | 关联任务 |
| created_at | TEXT | 是 | 创建时间 |

### runtime_events

用途：运行时事件审计。用于补足当前 runtime-state 只保存当前态的问题。

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| id | TEXT | 是 | 事件 ID |
| project_id | TEXT | 是 | 关联 `projects.id` |
| entity_type | TEXT | 是 | `task`、`approval`、`runner_job`、`agent_config_application` |
| entity_id | TEXT | 是 | 实体 ID |
| event_type | TEXT | 是 | `status_changed`、`created`、`applied`、`cancelled` 等 |
| before_state | JSON | 否 | 变更前快照 |
| after_state | JSON | 否 | 变更后快照 |
| actor | TEXT | 否 | 触发者，例如 `local_user`、Agent ID |
| reason | TEXT | 否 | 原因 |
| created_at | TEXT | 是 | 事件时间 |

说明：

- MVP-0.2 暂不要求所有接口写事件表。
- 接真实数据库时，审批、任务、Agent 配置应用等状态流转应优先写入事件。

## 5. 暂缓设计的表

以下模型暂缓，不进入第一版数据库落地：

- `users`、`teams`、`memberships`：当前没有登录/团队实现。
- `api_keys`：必须等安全方案确定后再设计，禁止明文保存。
- `model_calls`、`token_usage_events`：当前只有用量 mock 汇总，先不落真实调用明细。
- `billing_records`：计费系统不在 MVP-0.2。
- `cloud_sync_jobs`：云同步暂不做。
- `runner_execution_logs`：真实 Runner 执行前再细化。

## 6. 从 Mock 迁移到数据库的顺序

建议顺序：

1. 先建本地 SQLite 数据库和 seed 脚本，schema 保持 PostgreSQL 迁移友好。
2. 先建只读表：`projects`、`agents`、`agent_relationships`、`tasks`、`approvals`、`workflows`。
3. 把 `services/api/mock-data.js` 的初始数据导入数据库 seed。
4. Dashboard 聚合接口改为从数据库读取，但保持 response 结构不变。
5. 再迁移状态流转：任务 action、审批 action、Agent 配置应用/取消。
6. 状态流转迁移时同步写入 `runtime_events`，不要把事件审计放到最后补。
7. 最后再考虑 PostgreSQL / Supabase 迁移脚本，不在第一步接云端数据库。

明确不做：

- 不在第一步接真实 Runner 执行。
- 不在第一步接真实模型 API。
- 不在第一步设计完整权限系统。

## 7. 已定稿决策

以下决策用于指导第一版数据库初始化和 seed 方案：

1. 第一版使用 SQLite，本地落地 Mock API 的持久化和 seed 流程；字段类型、ID、JSON 字段和索引命名保持 PostgreSQL 迁移友好，后续云端再迁移到 Supabase PostgreSQL。
2. 第一版运行态先支持单项目 `project_agent_swarm`，但所有核心表继续保留 `project_id`，API 也继续使用 `:projectId`，避免以后补多项目时重改契约。
3. Agent 配置真实写入时，必须同时更新 `agents` 当前态并新增 `agent_config_versions` 版本记录；`agents` 负责快速读取当前配置，`agent_config_versions` 负责审计、追溯和回滚前审查。
4. `runtime_events` 对状态机实体必须记录完整 `before_state` / `after_state`，优先覆盖 `tasks`、`approvals`、`runner_jobs`、`agent_config_applications` 和真实 Agent 配置应用；大体积字段如完整 diff 可只保存摘要和产物 ID。
5. 工作流第一版继续在 `workflows.nodes` / `workflows.edges` 中保存 JSON；暂不拆 `workflow_nodes` / `workflow_edges`。只有当需要节点级查询、权限、运行统计或编辑冲突检测时再拆表。
