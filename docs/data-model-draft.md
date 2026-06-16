# agent蜂群数据模型草案

日期：2026-06-11

这是一份当前态的数据模型草案，只描述 MVP-0.2 到 MVP-0.4 已经用到的核心表，以及阶段 2 真实模型调用准入设计中需要先固定的记录结构。本文不等于数据库迁移已经实现。

## 设计目标

- 让 Mock 结构平滑迁移到 SQLite，再迁移到后续数据库。
- 让审批、任务、Runner 请求、Agent 配置变化和回滚都能追溯。
- 保持 `project_id` 贯穿核心对象，先单项目实现，后续再扩展多项目。
- 任何敏感内容都不要进入快照、日志或版本表。
- 模型调用记录必须先固定脱敏、成本、错误和审计边界，再允许任何真实 provider 请求进入实现。

## 命名规范

- 表名和字段名统一用 `snake_case`。
- 主键统一叫 `id`。
- 外键统一用 `<entity>_id`。
- 时间字段统一用 ISO datetime。
- 可变结构优先用 JSON 字段承载。

## 第一版最小落库（当前阶段执行）

以下四张表是 MVP 骨架阶段确认要建的，其他表暂缓。

### `projects`

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 如 `project_agent_swarm` |
| name | TEXT | NOT NULL | 项目名称 |
| status | TEXT | NOT NULL | 项目状态 |
| phase | TEXT | — | 当前阶段 |
| description | TEXT | — | 项目描述 |
| workspace_path | TEXT | — | 本地工作区路径 |
| created_at | TEXT | NOT NULL | ISO 时间 |
| updated_at | TEXT | NOT NULL | ISO 时间 |

### `agents`

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 如 `agent_architect` |
| project_id | TEXT | NOT NULL | 所属项目 |
| name | TEXT | NOT NULL | 名称 |
| role | TEXT | NOT NULL | 角色 |
| status | TEXT | NOT NULL | running / idle / stopped |
| model | TEXT | — | 使用的模型标识 |
| permissions | TEXT | — | JSON 数组，如 `["read_project","plan_tasks"]` |
| created_at | TEXT | NOT NULL | ISO 时间 |
| updated_at | TEXT | NOT NULL | ISO 时间 |

暂不建 `agent_relationships` 表。父子关系第一版不落地。

### `tasks`

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 如 `task_001` |
| project_id | TEXT | NOT NULL | 所属项目 |
| title | TEXT | NOT NULL | 标题 |
| description | TEXT | — | 描述 |
| status | TEXT | NOT NULL | queued / running / completed / blocked |
| priority | TEXT | NOT NULL | low / medium / high |
| assigned_agent_id | TEXT | — | 分配给哪个 Agent |
| depends_on | TEXT | — | JSON 数组，如 `["task_001"]` |
| risk_level | TEXT | — | low / medium / high |
| created_at | TEXT | NOT NULL | ISO 时间 |
| updated_at | TEXT | NOT NULL | ISO 时间 |

### `approvals`

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 如 `approval_001` |
| project_id | TEXT | NOT NULL | 所属项目 |
| task_id | TEXT | — | 关联的 Task |
| request_agent_id | TEXT | NOT NULL | 发起审批的 Agent |
| target_service | TEXT | NOT NULL | 审批目标：runner / agent_config / project_plan |
| operation_types | TEXT | NOT NULL | JSON 数组，如 `["file_write","git_checkpoint"]` |
| status | TEXT | NOT NULL | pending / approved / rejected / patch_only |
| risk_level | TEXT | NOT NULL | low / medium / high |
| reason | TEXT | — | 审批原因 |
| reject_reason | TEXT | — | 拒绝原因 |
| approved_at | TEXT | — | 批准时间 |
| rejected_at | TEXT | — | 拒绝时间 |
| created_at | TEXT | NOT NULL | ISO 时间 |
| updated_at | TEXT | NOT NULL | ISO 时间 |

关键约束：
- `target_service = project_plan` 时不生成可执行 Runner job。
- `target_service = agent_config` 时只进入 `pending_apply`，不直接改 Agent 当前态。

---

## 核心表（后续阶段）

### `projects`

项目主表。核心字段：`id`, `name`, `status`, `phase`, `description`, `workspace_path`, `created_at`, `updated_at`。

### `agents`

Agent 当前态。核心字段：`id`, `project_id`, `name`, `role`, `status`, `version`, `model`, `can_spawn_sub_agents`, `max_sub_agents`, `permissions`, `created_at`, `updated_at`。

约束：
- `permissions` 必须展开成明确能力，不允许只用 `all=true` 糊过去。
- `architect_admin`、`all_agents_full_management` 只能表示规划/编排/申请级别能力，不能自动带出 Runner 执行、文件写入、Git 修改、网络请求或原始密钥访问。

### `agent_relationships`

父子 Agent 关系。核心字段：`id`, `project_id`, `parent_agent_id`, `child_agent_id`, `reports_to_agent_id`, `spawn_depth`, `created_at`, `updated_at`。

### `agent_config_versions`

Agent 配置版本历史。核心字段：`id`, `project_id`, `agent_id`, `version`, `approval_id`, `application_id`, `config_snapshot`, `changes`, `applied_by`, `applied_at`, `created_at`。

约束：
- 只记录已应用版本。
- `config_snapshot` 不能包含 API key、模型 key 明文、prompt、raw secret 或本地敏感路径。
- 回滚不能直接覆盖旧版本，只能起新审批和新版本。

### `agent_config_applications`

Agent 配置审批后的待应用 / 已应用 / 已取消记录。核心字段：`id`, `project_id`, `approval_id`, `agent_id`, `agent_name`, `change_type`, `changes`, `status`, `applied_at`, `applied_by`, `apply_confirm_text`, `cancelled_at`, `cancelled_by`, `cancel_reason`, `created_at`, `updated_at`。

状态只看：
- `pending_apply`
- `applied`
- `cancelled`

### `tasks`

任务队列和任务状态。核心字段：`id`, `project_id`, `title`, `description`, `status`, `priority`, `assigned_agent_id`, `risk_level`, `related_files`, `requires_approval`, `depends_on`, `started_at`, `completed_at`, `failed_at`, `cancelled_at`, `failure_reason`, `created_at`, `updated_at`。

### `approvals`

审批主表。核心字段：`id`, `project_id`, `status`, `risk_level`, `request_agent_id`, `request_agent_name`, `target_service`, `operation_types`, `reason`, `checkpoint_required`, `checkpoint_created`, `checkpoint_commit`, `affected_files`, `diff_summary`, `diff_preview`, `requires_second_confirm`, `change_request`, `runner_job_id`, `patch_artifact_id`, `reject_reason`, `approved_at`, `rejected_at`, `patch_only_at`, `created_at`, `updated_at`。

关键约束：
- `target_service = project_plan` 时，审批通过后只生成任务和只读 Runner request 记录，不生成可执行 Runner job。
- `target_service = agent_config` 时，审批通过后默认只进入 `pending_apply`，不直接改 Agent 当前态。

### `agent_runs`

Agent Run 记录链。核心字段：`id`, `project_id`, `chain_id`, `root_run_id`, `parent_run_id`, `sequence`, `role`, `agent_id`, `agent_name`, `model`, `status`, `input_summary`, `output_summary`, `token_usage`, `cost_estimate`, `error_category`, `error_message`, `requested_by`, `chain_label`, `created_at`, `started_at`, `completed_at`, `failed_at`, `updated_at`。

关键约束：
- 只记录本地 Agent Run 链，不记录原始 prompt、provider payload、文件内容或 Runner job 上下文。
- `input_summary` / `output_summary` 只能是结构化摘要，不是完整对话。
- `token_usage` 和 `cost_estimate` 只能保存粗粒度结果，不能保存账单凭据或原始 provider 响应。
- 失败注入只能停留在本地记录链，不得扩权，不得改写任务或 Runner 请求。

### `project_plan_drafts`

阶段：阶段 24 已完成，migration 004 已实现。

项目计划草案表。核心字段：`id`, `project_id`, `approval_id`, `idea`, `constraints`, `summary`, `status`, `generated_by`, `requested_by`, `model_call_id`（阶段 26 新增，可为 NULL），`created_at`, `updated_at`。

关键约束：
- 只保存本地确定性模板草案或真实模型脱敏草案，不保存真实模型 raw prompt 或 raw response。
- `generated_by` 为 `local_deterministic_template`（本地）或 `real_model_preview`（真实模型）。
- `model_call_id` 关联 `model_calls` 审计记录，只存 ID 不存模型原文。本地确定性草案时为 NULL。
- 创建草案时只写 `project_plan_drafts` 和 pending `project_plan` approval，不创建任务或 Runner request。
- 审批通过后状态进入 `instantiated`，且同一 approval 不能重复实例化。

### `runner_requests`

阶段：阶段 24 已完成，migration 004 已实现。旧文档和旧原型里有时称为 `runner_jobs`，新 Tauri/Rust 主线第一版使用 `runner_requests`，避免误解成可执行 Runner job。

只读 Runner request 队列记录。核心字段：`id`, `project_id`, `approval_id`, `task_id`, `status`, `operation_types`, `affected_files`, `checkpoint`, `safety_note`, `created_at`, `updated_at`。

关键约束：
- 目前只允许承载 `runner_request_readonly` 之类的队列语义。
- 不代表真实 Runner 执行许可。
- `project_plan` 生成的记录必须关联任务，但 `checkpoint` 为空。
- 只允许虚拟 affected files，不代表真实文件写入范围。

### `workflows` / `workflow_steps`

工作流展示数据。第一版可以先保留 `nodes` / `edges` JSON，不急着拆成更细的表。

### `knowledge_updates`

文档同步和知识更新记录。核心字段：`id`, `project_id`, `document`, `section`, `status`, `related_feature`, `updated_at`, `created_at`。

### `git_checkpoints`

Git 保存点记录。核心字段：`id`, `project_id`, `commit_hash`, `message`, `type`, `related_task_id`, `created_at`。

### `runtime_events`

运行时审计事件。核心字段：`id`, `project_id`, `entity_type`, `entity_id`, `event_type`, `before_state`, `after_state`, `actor`, `reason`, `created_at`。

用途：
- 记录任务、审批、Runner 请求、Agent 配置应用等状态变化。
- 以后如果要追责或回放，本表是第一入口。

### `model_calls`

阶段：阶段 23 已完成 helper-only SQLite 迁移。`003_add_model_calls.sql` 已建立 18 字段审计表和 3 个索引；阶段 25.3 已实现真实调用审计落库：成功/失败进入 provider 阶段的调用写入脱敏 `model_calls`。

模型调用记录表。当前 SQLite 字段：`id`, `project_id`, `purpose`, `provider`, `model`, `status`, `request_hash`, `structured_summary`, `token_usage`, `cost_estimate`, `error_category`, `error_message`, `redaction_applied`, `duration_ms`, `related_approval_id`, `runtime_event_id`, `created_at`, `updated_at`。

第一条允许进入设计的 `purpose` 只有：

- `project_plan_generation`

后续 `task_breakdown`、`review_summary` 或 Agent Run 相关用途必须另写准入规格和验收脚本，不能复用第一条链路偷偷放开。

状态草案：

- `blocked`
- `pending`
- `running`
- `succeeded`
- `failed`

关键约束：

- `model_calls` 只记录后端 Model Gateway 固定形态请求，不记录 UI 自由 prompt。
- `provider` 和 `model` 必须来自后端白名单或后端配置，不能来自前端自由提交。
- `request_hash` 只能是脱敏后、固定请求摘要的哈希，不能还原 prompt、headers、key 或 provider body。
- `token_usage` 只能保存 provider 返回且已安全解析后的粗粒度 JSON，例如 `prompt_tokens`、`completion_tokens`、`total_tokens`。
- `cost_estimate` 只能保存本地估算值和币种，不保存账单凭据。
- `structured_summary` 只能保存结构化、脱敏、限长后的业务摘要；第一条链路只允许保存 project plan 摘要。
- `error_category` 必须使用粗粒度分类，例如 `feature_disabled`、`missing_key`、`invalid_request`、`unsupported_provider`、`unsupported_model`、`timeout`、`provider_unavailable`、`network_error`、`response_too_large`、`redaction_failed`、`unknown`。`audit_write_failed` 为接口响应专用，不入库（审计写入失败意味着记录未成功写入）。
- `runtime_event_id` 可关联模型调用状态变化审计，但 runtime event 也只能保存脱敏前后状态。

禁止保存或返回：

- raw API key、key suffix、masked key fragment。
- raw request headers、raw response headers。
- raw provider request body、raw provider response body。
- raw prompt、完整 prompt template、system prompt。
- raw provider error、原始堆栈、provider 内部错误体。
- model reasoning text。
- 文件内容、本地敏感路径、Runner job 上下文。

副作用边界：

- 创建或更新 `model_calls` 不得直接创建任务。
- 不得直接创建 Runner job。
- 不得触发 Agent。
- 不得写项目文件。
- 不得修改 Git。
- 不得执行 Runner。

### `runner_status`

本地 Runner 连接状态的只读展示。核心字段：`id`, `project_id`, `connected`, `runner_id`, `version`, `workspace_path`, `permissions`, `last_heartbeat_at`, `created_at`, `updated_at`。

### `model_catalog`（阶段 35）

受控模型目录表。核心字段：`id`, `project_id`, `provider`, `model_id`, `display_name`, `purpose`, `enabled`, `is_builtin`, `created_at`, `updated_at`。

唯一索引：`(project_id, provider, model_id, purpose)`。

约束：
- 第一版 `provider` 固定 `openai_compat`，`purpose` 固定 `project_plan_generation`。
- 前端只能从 `enabled=true` 的记录中选择模型。
- 模型名必须是后端校验后的合法格式。
- 不存储 raw key、base URL、prompt 或 provider error。

### `auto_mode_sessions`（阶段 36 草案，未落库）

全自动模式会话表。草案字段：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 会话 ID |
| project_id | TEXT | NOT NULL | 所属项目 |
| mode | TEXT | NOT NULL | `manual` 或 `full_auto` |
| status | TEXT | NOT NULL | `enabled` / `paused` / `stopping` / `stopped` / `failed` |
| controller_agent_id | TEXT | NOT NULL | 总控智能体 ID |
| started_by | TEXT | NOT NULL | 启动者 |
| stop_requested_by | TEXT | — | 停止请求者 |
| stop_reason | TEXT | — | 停止原因 |
| policy_version | TEXT | NOT NULL | 策略版本号 |
| created_at | TEXT | NOT NULL | ISO 时间 |
| updated_at | TEXT | NOT NULL | ISO 时间 |

### `auto_authorization_records`（阶段 36 草案，未落库）

自动授权记录表。草案字段：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 记录 ID |
| project_id | TEXT | NOT NULL | 所属项目 |
| auto_mode_session_id | TEXT | NOT NULL | 关联的 auto_mode_session |
| granted_by_agent_id | TEXT | NOT NULL | 授权方（总控智能体） |
| granted_to_agent_id | TEXT | NOT NULL | 被授权方（角色智能体） |
| permission_level | TEXT | NOT NULL | 权限级别：L0 / L1 / L2 / L3 / L4 |
| permission_scope | TEXT | NOT NULL | 授权范围描述 |
| reason_summary | TEXT | NOT NULL | 授权原因脱敏摘要 |
| status | TEXT | NOT NULL | `active` / `revoked` / `expired` / `denied` |
| created_at | TEXT | NOT NULL | ISO 时间 |
| revoked_at | TEXT | — | 撤销时间 |

关键约束：
- `permission_level = L4` 在阶段 36 不得自动生成，必须人工确认。
- `reason_summary` 必须脱敏，不得包含 raw prompt、raw key 或文件完整内容。

### `controller_decisions`（阶段 36 草案，未落库）

总控智能体决策记录表。草案字段：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | TEXT | PK | 决策 ID |
| project_id | TEXT | NOT NULL | 所属项目 |
| auto_mode_session_id | TEXT | NOT NULL | 关联的 auto_mode_session |
| decision_type | TEXT | NOT NULL | 决策类型：workflow_select / agent_assign / model_select / skill_bind / permission_grant / stop / rollback |
| input_summary | TEXT | NOT NULL | 输入脱敏摘要 |
| output_summary | TEXT | NOT NULL | 输出脱敏摘要 |
| risk_level | TEXT | NOT NULL | low / medium / high |
| status | TEXT | NOT NULL | 决策状态 |
| created_at | TEXT | NOT NULL | ISO 时间 |

关键约束：
- `input_summary` 和 `output_summary` 必须是脱敏摘要，不保存 raw prompt、raw response 或 raw provider error。
- 真实模型调用细节仍以 `model_calls` 为准，本表只记录决策级别的摘要。

## 暂缓设计的表

- `users` / `teams` / `memberships`
- `api_keys`
- `token_usage_events`
- `billing_records`
- `cloud_sync_jobs`
- `runner_execution_logs`

说明：

- `model_calls` SQLite 审计表已由 migration 003 建立，阶段 25.3 已实现安全审计落库（成功/失败调用写入脱敏记录），`runtime_events` 和 token usage 独立事件表仍暂缓。
- `api_keys` 仍暂缓；阶段 2 第一版只允许 server env 读取 API key，不在 SQLite、Mock runtime state、前端 storage 或日志中保存 key。

## `model_calls` helper-only 当前状态

- 阶段 23 建立 SQLite 审计表和 Rust helper 草案。
- 阶段 25.3 已开放真实写入路径：`insert_safe_model_call()` 写入脱敏 model_calls 记录。
- `request_project_plan_model_draft` 在 feature flag 关闭时返回 `feature_disabled`，不写入 `model_calls`。
- 写入入口只接受后端固定的请求信封，不接受前端自由 prompt、headers、key 或 provider body。
- `request_hash` 基于安全归一化摘要生成（purpose/provider/model/idea长度/constraints是否存在）。
- `structured_summary` 只保存 redact_secrets + truncate_summary 后的摘要。
- 仍不写 `runtime_events`（25.3 不碰 runtime_events）。
- 仍禁止 raw key / raw base URL / raw prompt / raw provider response。

## 迁移顺序

1. `001_initial_sqlite` — 第一版最小落库：`projects` + `agents` + `tasks` + `approvals`（当前阶段）。
2. `002_add_agent_runs` — Agent Run 记录链 + `runtime_events`。
3. `003_add_model_calls` — `model_calls` helper-only 审计表。
4. `004_add_project_plan_workflow` — `project_plan_drafts` + `runner_requests`，用于迁移旧 MVP-0.3 的项目计划审批闭环（阶段 24 已完成）。
5. `005_add_project_plan_model_audit_link` — `project_plan_drafts.model_call_id` 列 + 索引，关联 `model_calls` 安全审计记录（阶段 26 已完成）。
6. `006_add_project_plan_task_templates` — 可配置任务角色模板表，内置 9 个角色（阶段 28 已完成）。
7. `007_add_runner_preflight_reviews` — Runner 执行前审查闸门表（阶段 30 已完成）。
8. `008_add_runner_execution_gates` — Runner 执行许可 gate 表（阶段 31 已完成）。
9. `009_add_runner_dry_runs` — Runner dry-run 预演表（阶段 32 已完成）。
10. `010_add_runner_execution_locks` — Runner 执行范围锁表（阶段 33 已完成）。
11. `011_add_runner_minimal_runs` — 最小真实 Runner 执行表（阶段 34 已完成）。
12. `012_add_model_catalog` — 受控模型目录表（阶段 35 已完成）。
13. 后续按需追加：`agent_relationships`、`agent_config_versions`、`agent_config_applications`、`workflows`。
14. 辅助表按需：`knowledge_updates`、`git_checkpoints`、`runner_status`。

## 已定稿的几条规则

- 第一版只做单项目，但所有核心表都保留 `project_id`。
- `runtime_events` 要优先记录状态变化，不要等到最后补日志。
- Agent 配置真实写入时，当前态和版本表必须在同一事务里更新。
- `docs/agent-permission-contract.md` 仍然是权限语义的上位约束。
