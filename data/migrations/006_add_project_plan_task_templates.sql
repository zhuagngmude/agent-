-- 006_add_project_plan_task_templates.sql
-- 阶段 28：可配置任务角色模板。内置 9 个角色，默认启用前 5 个。
-- 种子数据由 Rust migration helper 插入，本文件只负责建表和索引。

CREATE TABLE IF NOT EXISTS project_plan_task_templates (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  role TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  priority TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  affected_file TEXT NOT NULL,
  operation_type TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL,
  is_builtin INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (agent_id) REFERENCES agents(id)
);

CREATE INDEX IF NOT EXISTS idx_project_plan_task_templates_project_id
  ON project_plan_task_templates(project_id);

CREATE INDEX IF NOT EXISTS idx_project_plan_task_templates_enabled
  ON project_plan_task_templates(project_id, enabled, sort_order);

CREATE UNIQUE INDEX IF NOT EXISTS idx_project_plan_task_templates_project_role
  ON project_plan_task_templates(project_id, role);
