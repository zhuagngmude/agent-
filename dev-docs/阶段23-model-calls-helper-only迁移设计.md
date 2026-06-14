# 阶段 23：model_calls helper-only 迁移设计

日期：2026-06-15

本文是阶段 23 的设计与收口文档，定义 `model_calls` 审计表的建表策略、字段约束、helper-only 写入草案和验收方式。

收口状态（2026-06-15）：阶段 23 已实现并验收。已新增 `003_add_model_calls.sql`，接入 Rust migration runner，补齐 helper-only 草案和验收测试；`feature_disabled` 时不写 `model_calls`、不写 `runtime_events`，不调用真实 provider。

## 一、为什么现在需要建 model_calls

### 1.1 当前缺口

阶段 2 在旧 Node.js 架构中已完成了 `model_calls` 的结构草案和 helper-only scaffold（`model-gateway-model-calls.js`），但：

- 表未建（`001_initial_sqlite.sql` 不含 `model_calls`）
- 草案在旧 Node 代码中，新 Tauri/Rust 架构完全没有 `model_calls` 的表结构
- 后续一旦接入真实 provider 调用，必须有对应的审计表记录每次调用

### 1.2 建表时机

当前（阶段 22）`request_project_plan_model_draft` 已注册但返回 `feature_disabled`。阶段 23 建表后，后续真实模型阶段接入真实调用时：

- 成功调用 → 写入 `model_calls` 脱敏记录
- 失败/超时/被拒绝 → 写入 `model_calls` 错误记录
- Feature flag 关闭 → 不写入

如果等到真实调用阶段才建表，migration 和 command 修改会在同一个阶段引入，增加回归风险。**提前建表可以验证 migration 执行无误、表结构可查询，且不影响任何现有功能。**

## 二、表字段设计

### 2.1 完整字段清单

字段对齐旧 `model-gateway-model-calls.js` 中定义的 `modelCallColumns`（原 20+ 列），精简为 18 个字段（含 id）：

```sql
-- 003_add_model_calls.sql
-- Model Calls 审计记录表（当前阶段只建表，feature_disabled 时不写入）

CREATE TABLE IF NOT EXISTS model_calls (
  id                    TEXT PRIMARY KEY,
  project_id            TEXT NOT NULL,
  purpose               TEXT NOT NULL,          -- 当前只允许 "project_plan_generation"
  provider              TEXT NOT NULL,          -- 当前只允许 "openai_compat"
  model                 TEXT NOT NULL,          -- 当前只允许 "gpt-5.4-mini"
  status                TEXT NOT NULL,          -- blocked | pending | running | succeeded | failed
  request_hash          TEXT,                   -- 脱敏后的请求信封哈希，不可逆推原始输入
  structured_summary    TEXT,                   -- 结构化、脱敏、限长后的摘要
  token_usage           TEXT,                   -- JSON：{ prompt, completion, total }
  cost_estimate         TEXT,                   -- JSON：{ amount, currency }
  error_category        TEXT,                   -- 13 类，以阶段 21 第七节为准（feature_disabled | missing_key | missing_base_url | invalid_base_url | unsupported_provider | unsupported_model | invalid_purpose | forbidden_field | timeout | provider_error | response_too_large | redaction_failed | unknown）
  error_message         TEXT,                   -- 脱敏后的错误描述
  redaction_applied     INTEGER NOT NULL DEFAULT 0,  -- 0 | 1
  duration_ms           INTEGER,               -- 调用耗时（毫秒）
  related_approval_id   TEXT,                   -- 关联的审批（如有）
  runtime_event_id      TEXT,                   -- 关联的 runtime_event（如有）
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_calls_project_id ON model_calls(project_id);
CREATE INDEX IF NOT EXISTS idx_model_calls_status ON model_calls(status);
CREATE INDEX IF NOT EXISTS idx_model_calls_created_at ON model_calls(created_at);
```

### 2.2 字段来源说明

| 字段 | 来源 | 必填 |
|------|------|------|
| `id` | 生成（`model_call_{purpose}_{timestamp}_{uuid8}`） | 是 |
| `project_id` | 调用 `services::projects::get_current_project(connection)` 后取 `.id` | 是 |
| `purpose` | 固定 `"project_plan_generation"` | 是 |
| `provider` | `provider_config` 解析结果 | 是 |
| `model` | `provider_config` 解析结果 | 是 |
| `status` | `blocked`（禁用）/ `succeeded`（成功）/ `failed`（失败） | 是 |
| `request_hash` | 脱敏后的请求信封哈希 | 否（禁用态为空） |
| `structured_summary` | 脱敏+限长后的摘要 | 否 |
| `token_usage` | JSON，粗粒度 `{prompt, completion, total}` | 否（禁用态为 `{}`） |
| `cost_estimate` | JSON，粗粒度 `{amount, currency}` | 否（禁用态为 `{}`） |
| `error_category` | 错误分类（13 类，以阶段 21 第七节为权威源） | 否 |
| `error_message` | 脱敏后的错误描述 | 否 |
| `redaction_applied` | 是否执行了脱敏 | 是 |
| `duration_ms` | 调用耗时 | 否（禁用态为空） |
| `related_approval_id` | 关联审批 ID | 否 |
| `runtime_event_id` | 关联审计事件 ID | 否 |

### 2.3 绝不能存的内容

以下内容**任何情况下不得写入** `model_calls` 表的任何字段：

| 禁止存储 | 原因 |
|----------|------|
| raw API key | 密钥泄露风险 |
| key fragment / suffix | 旁路泄露 |
| raw prompt（用户输入原文） | 隐私 + 安全 |
| raw provider response body | 可能包含注入内容 |
| raw HTTP 请求头 | 包含 Authorization header |
| provider base URL 原文 | 配置信息泄露 |
| 完整对话内容 | `structured_summary` 只允许 4096 字节以内的结构化摘要 |
| 账单凭据 | `cost_estimate` 只允许粗粒度 `{amount, currency}` |
| 原始 token usage 详情 | 只允许 `{prompt, completion, total}` 三个数字 |

## 三、feature_disabled 时不写入的原因

阶段 23 的 `request_project_plan_model_draft` 在 feature flag 关闭时：

1. 不写入 `model_calls`（不落盘）
2. 不写入 `runtime_events`（不创建审计事件）

**原因：**

- `feature_disabled` 表示整个调用链路未被激活——没有请求、没有响应、没有错误。写入一条 `status=blocked` 的记录会产生无意义的审计噪音
- 与旧准入规格一致：旧 `model-gateway-model-calls.js` 的 `canWrite=false`，`feature_disabled` 时不落盘
- 后续真实调用开启后：成功/失败/超时/错误**才**写入 `model_calls` + `runtime_events`，此时每条记录都有实际的事件可审计

`feature_disabled` 时的行为：直接返回内存中的 `ProjectPlanModelDraftResponse`，不碰任何持久化存储。

## 四、Rust 文件变更

| 文件 | 变更 | 说明 |
|------|------|------|
| `data/migrations/003_add_model_calls.sql` | 新增 | 建表 + 3 索引 |
| `apps/desktop/src-tauri/src/db/mod.rs` | 修改 | 新增 `MODEL_CALLS_MIGRATION_SQL` 常量 + `run_model_calls_migration()` |
| `apps/desktop/src-tauri/src/services/model_gateway/mod.rs` | 修改 | 暴露 `model_calls` helper 模块；`create_project_plan_draft` 仍保持 `feature_disabled` 逻辑 |
| `apps/desktop/src-tauri/src/services/model_gateway/model_calls.rs` | 新增 | `build_model_call_draft()` helper（`canWrite=false` 时返回草案，不落盘） |

**不修改：**
- `commands/model_gateway.rs` — 无功能变更，继续返回 `feature_disabled`
- `lib.rs` — 不新增 command
- 现有 4 张核心表
- `packages/ui` — 不做前端接入
- `packages/shared` — 暂不新增 model_calls 类型

## 五、测试

### 5.1 新增测试

| 测试 | 说明 |
|------|------|
| `initialize_creates_minimal_tables_and_seed_data_once` | 验证初始化后 `model_calls` 表可查询且初始 0 条 |
| `model_calls_table_has_expected_columns` | 验证 18 个字段齐全 |
| `model_calls_indexes_exist` | 验证 3 个索引存在 |
| `feature_disabled_does_not_write_model_calls` | 调用 `create_project_plan_draft` 后返回 `feature_disabled`，且 `SELECT COUNT(*) FROM model_calls` 仍为 0 |
| `feature_disabled_does_not_create_runtime_events` | 同上，`runtime_events` 表无新增 |
| `draft_can_write_is_false` | helper 返回草案但 `can_write=false` |
| `draft_does_not_contain_raw_secrets` | helper 结构不包含 raw key / raw prompt / raw response 字段 |
| `draft_status_is_blocked_when_feature_disabled` | 禁用态草案状态为 `blocked`，错误分类为 `feature_disabled` |
| `draft_has_all_expected_fields` | helper 草案字段齐全，默认 token / cost 为空 JSON |
| `error_category_as_str_covers_all_13_variants` | 13 类错误分类均有固定字符串 |

### 5.2 回归验证

```powershell
cd apps/desktop/src-tauri; cargo check; cargo test    # 全量通过
cd packages/ui; npm run typecheck; npm run build        # pass
git diff --check                                          # clean
```

## 六、回滚方案

`003_add_model_calls.sql` 使用 `CREATE TABLE IF NOT EXISTS`，不修改现有表。回滚策略：

1. **未生产数据时回滚：** 删除 migration 003 的 include_str 和执行调用，重新编译
2. **已有生产数据时回滚：** `DROP TABLE IF EXISTS model_calls` + 删除 migration 调用
3. **向前兼容：** migration 003 失败不影响 001/002 已建表的功能

## 七、不做

- 不写入 `model_calls`（`feature_disabled` 时不落盘）
- 不写入 `runtime_events`（不创建审计事件）
- 不导入 `reqwest` 或任何 HTTP 客户端
- 不导入 provider SDK
- 不发 HTTP 请求
- 不读取 raw key
- 不返回 key fragment
- 不保存 raw prompt / raw provider response
- 不创建 task / approval
- 不触发 Agent Run
- 不启用 Runner
- 不写用户项目文件

## 八、参考

- [阶段21-真实模型接入新架构适配设计](./阶段21-真实模型接入新架构适配设计.md) — 五阶段规划 + model_calls 字段约束
- [data-model-draft](../docs/data-model-draft.md) — model_calls 表设计约束
- `services/api/model-gateway-model-calls.js` — 旧 helper-only 草案（语义参考）
- `services/api/model-gateway-redaction.js` — 旧脱敏规则（语义参考）
- [真实模型接入准入规格](./真实模型接入准入规格.md) — 17 项准入检查项
