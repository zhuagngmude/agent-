-- 018_relax_tasks_assigned_agent_fk:
-- tasks.assigned_agent_id now accepts project_agents.id as the primary assignment source
-- while service-level validation keeps legacy agents.id compatible.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS tasks_new (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  priority TEXT NOT NULL,
  assigned_agent_id TEXT,
  depends_on TEXT,
  risk_level TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

INSERT OR IGNORE INTO tasks_new (
  id, project_id, title, description, status, priority, assigned_agent_id,
  depends_on, risk_level, created_at, updated_at
)
SELECT
  id, project_id, title, description, status, priority, assigned_agent_id,
  depends_on, risk_level, created_at, updated_at
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_agent_id ON tasks(assigned_agent_id);

CREATE TABLE IF NOT EXISTS agent_boundary_checks_new (
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

INSERT OR IGNORE INTO agent_boundary_checks_new (
  id, project_id, task_id, agent_id, requested_action, task_type,
  module_scope, target_path, decision, reason, created_at
)
SELECT
  id, project_id, task_id, agent_id, requested_action, task_type,
  module_scope, target_path, decision, reason, created_at
FROM agent_boundary_checks;

DROP TABLE agent_boundary_checks;
ALTER TABLE agent_boundary_checks_new RENAME TO agent_boundary_checks;

CREATE INDEX IF NOT EXISTS idx_agent_boundary_checks_project
  ON agent_boundary_checks (project_id, created_at);

CREATE INDEX IF NOT EXISTS idx_agent_boundary_checks_agent
  ON agent_boundary_checks (agent_id, decision);

PRAGMA foreign_keys = ON;
