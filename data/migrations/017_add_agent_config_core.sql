-- Migration 017: P0 AI 员工 / 执行器 / Skill / 边界检查核心配置
-- 只保存非敏感配置；API Key、Token、私钥、raw prompt、raw response 不进入这些表。
-- model_catalog.executor_key 由 Rust 迁移代码用 PRAGMA 检查后追加，避免 SQLite ALTER TABLE 重复执行失败。

CREATE TABLE IF NOT EXISTS executor_configs (
  id               TEXT NOT NULL PRIMARY KEY,
  key              TEXT NOT NULL UNIQUE,
  label            TEXT NOT NULL,
  kind             TEXT NOT NULL CHECK (kind IN ('model_gateway', 'external_executor', 'local_tool')),
  provider         TEXT,
  base_url_status  TEXT NOT NULL DEFAULT 'not_configured',
  executable_path  TEXT,
  status           TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'error')),
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_executor_configs_key
  ON executor_configs (key);

CREATE INDEX IF NOT EXISTS idx_executor_configs_status
  ON executor_configs (status);

CREATE TABLE IF NOT EXISTS agent_templates (
  id                   TEXT NOT NULL PRIMARY KEY,
  name                 TEXT NOT NULL,
  role                 TEXT NOT NULL,
  category             TEXT NOT NULL CHECK (category IN ('core', 'expert')),
  specialty            TEXT,
  stack                TEXT,
  module_scope         TEXT NOT NULL,
  allowed_task_types   TEXT NOT NULL DEFAULT '[]',
  allowed_paths        TEXT NOT NULL DEFAULT '[]',
  forbidden_actions    TEXT NOT NULL DEFAULT '[]',
  default_executor_key TEXT NOT NULL,
  default_model_id     TEXT,
  enabled              INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL,
  FOREIGN KEY (default_executor_key) REFERENCES executor_configs(key)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_templates_role_category
  ON agent_templates (role, category);

CREATE INDEX IF NOT EXISTS idx_agent_templates_scope
  ON agent_templates (module_scope);

CREATE INDEX IF NOT EXISTS idx_agent_templates_enabled
  ON agent_templates (enabled);

CREATE TABLE IF NOT EXISTS project_agents (
  id                TEXT NOT NULL PRIMARY KEY,
  project_id        TEXT NOT NULL,
  agent_template_id TEXT NOT NULL,
  name              TEXT NOT NULL,
  role              TEXT NOT NULL,
  source            TEXT NOT NULL CHECK (source IN ('core', 'recommended', 'manual')),
  executor_key      TEXT NOT NULL,
  model_id          TEXT,
  module_scope      TEXT NOT NULL,
  status            TEXT NOT NULL CHECK (status IN ('active', 'idle', 'disabled', 'removed')),
  joined_at         TEXT NOT NULL,
  removed_at        TEXT,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (agent_template_id) REFERENCES agent_templates(id),
  FOREIGN KEY (executor_key) REFERENCES executor_configs(key)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_project_agents_project_template_active
  ON project_agents (project_id, agent_template_id)
  WHERE removed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_project_agents_project_status
  ON project_agents (project_id, status);

CREATE INDEX IF NOT EXISTS idx_project_agents_executor
  ON project_agents (executor_key);

CREATE TABLE IF NOT EXISTS executor_skills (
  id                TEXT NOT NULL PRIMARY KEY,
  executor_key      TEXT NOT NULL,
  agent_template_id TEXT,
  skill_name        TEXT NOT NULL,
  skill_scope       TEXT NOT NULL,
  enabled           INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  FOREIGN KEY (executor_key) REFERENCES executor_configs(key),
  FOREIGN KEY (agent_template_id) REFERENCES agent_templates(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_executor_skills_unique
  ON executor_skills (executor_key, agent_template_id, skill_name);

CREATE INDEX IF NOT EXISTS idx_executor_skills_enabled
  ON executor_skills (executor_key, enabled);

CREATE TABLE IF NOT EXISTS agent_boundary_checks (
  id               TEXT NOT NULL PRIMARY KEY,
  project_id       TEXT NOT NULL,
  task_id          TEXT,
  agent_id         TEXT NOT NULL,
  requested_action TEXT NOT NULL,
  task_type        TEXT,
  module_scope     TEXT NOT NULL,
  target_path      TEXT,
  decision         TEXT NOT NULL CHECK (decision IN ('allowed', 'denied', 'needs_approval')),
  reason           TEXT NOT NULL,
  created_at       TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_agent_boundary_checks_project
  ON agent_boundary_checks (project_id, created_at);

CREATE INDEX IF NOT EXISTS idx_agent_boundary_checks_agent
  ON agent_boundary_checks (agent_id, decision);
