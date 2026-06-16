# 阶段 37：Skill 目录与角色 Skill 绑定设计

## 目标

为不同角色准备不同 skill，让系统知道每个角色在生成、审查、执行前应该遵守哪套能力说明。

示例：

```text
frontend -> React / UI / Ant Design skill
backend -> Rust / Tauri / SQLite skill
qa -> 测试回归 skill
docs -> 文档一致性 skill
reviewer -> 代码审查 skill
security -> 安全边界 skill
```

阶段 37 只做目录和绑定，不把 skill 内容直接拼进真实模型 prompt。真正把 skill 参与模型调用，应放到后续阶段单独审查。

## 当前边界

仍然禁止：

- 前端自由输入 prompt。
- 前端上传任意系统提示词。
- 直接把 skill 原文塞进 provider 请求。
- 让 skill 触发命令执行、文件写入或 Git 操作。
- 自动执行 Runner。
- 写用户项目文件。
- 改 Git。
- 把 raw prompt / raw response 落库。

## 数据模型

新增 migration：

```text
data/migrations/014_add_skill_catalog.sql
```

新增表建议：

```sql
CREATE TABLE IF NOT EXISTS skill_catalog (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  skill_key TEXT NOT NULL,
  display_name TEXT NOT NULL,
  description TEXT NOT NULL,
  scope TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_skill_catalog_project_skill_key
  ON skill_catalog(project_id, skill_key);

CREATE TABLE IF NOT EXISTS role_skill_bindings (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  role TEXT NOT NULL,
  purpose TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (skill_id) REFERENCES skill_catalog(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_role_skill_bindings_project_role_purpose_skill
  ON role_skill_bindings(project_id, role, purpose, skill_id);
```

## Skill 内容策略

第一版不要把完整 skill 文本落库。只落目录元数据：

- `skill_key`
- `display_name`
- `description`
- `scope`

如果后续要让模型使用 skill 内容，应单独设计：

- 内容来源。
- 内容长度限制。
- 脱敏策略。
- prompt 注入防护。
- model_calls 审计字段。

阶段 37 不做这一步。

## 内置 skill 建议

第一版 seed 这些目录项：

```text
frontend_react_ui
backend_rust_tauri_sqlite
qa_regression
docs_consistency
reviewer_code_review
security_boundary
devops_local_runtime
ux_product_flow
data_sqlite_schema
```

这些只是目录项，不等于已经执行 skill。

## 后端服务

新增模块建议：

```text
apps/desktop/src-tauri/src/services/skill_catalog.rs
apps/desktop/src-tauri/src/commands/skill_catalog.rs
```

新增 command：

```text
list_skill_catalog
list_role_skill_bindings
update_role_skill_binding
```

`list_skill_catalog`：

- 只返回当前项目。
- 不返回 prompt 原文。
- 不返回文件路径。
- 不读取本地 skill 文件。

`update_role_skill_binding`：

- 需要二次确认。
- 校验 role 白名单。
- 校验 purpose 第一版只允许 `project_plan_generation`。
- 校验 skill 当前项目且 enabled=1。
- 不调用模型。
- 不写 model_calls。
- 不写 runtime_events。
- 不创建 tasks / approvals / runner_requests。

## 前端

新增“角色 Skill 绑定”卡片：

- 展示 role。
- 展示当前绑定 skill。
- 支持为角色选择一个或多个 enabled skill。
- 修改需要二次确认。
- 只展示 skill 名称、描述、scope。
- 不展示 prompt 原文。
- 不允许自由输入 prompt。

## 与模型调用的关系

阶段 37 不修改真实模型调用 prompt。

阶段 37 完成后，只能说：

```text
系统已经知道角色绑定了哪些 skill。
```

不能说：

```text
模型已经按 skill 执行。
```

把 skill 注入模型上下文应作为后续阶段，例如阶段 39。

## 测试要求

必须补 Rust 测试：

- migration 创建 skill_catalog 和 role_skill_bindings。
- seed 幂等。
- list skill 只返回当前项目。
- list binding 只返回当前项目。
- update 需要二次确认。
- update 拒绝未知 role。
- update 拒绝未知 skill。
- update 拒绝 disabled skill。
- update 拒绝跨项目 skill。
- update 不调用 provider。
- update 不写 model_calls。
- update 不写 runtime_events。
- update 不创建 tasks / approvals / runner_requests。
- skill_key 污染值拒绝：
  - `../secret`
  - `https://evil.com`
  - `sk-xxx`
  - `prompt\ninject`
  - `{json}`

前端验证：

- 类型导出正确。
- command 封装正确。
- UI typecheck 通过。

## 验证命令

```powershell
cd F:\projects\agent-swarm\apps\desktop\src-tauri
cargo fmt --check
cargo check
cargo test

cd F:\projects\agent-swarm\packages\ui
npm run typecheck
npm run build

cd F:\projects\agent-swarm
git diff --check
```

## 完成口径

阶段 37 完成后，系统具备 Skill 目录和角色 Skill 绑定能力，但 Skill 还不参与真实模型 prompt，不执行 Runner，不写用户项目文件，不改 Git。
