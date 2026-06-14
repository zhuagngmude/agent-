-- 002_add_agent_runs.sql
-- Agent Run 记录链 + 运行时审计事件（纯只读展示，此阶段不新增记录）

CREATE TABLE IF NOT EXISTS agent_runs (
  id              TEXT PRIMARY KEY,
  project_id      TEXT NOT NULL,
  chain_id        TEXT NOT NULL,
  root_run_id     TEXT NOT NULL,
  parent_run_id   TEXT,
  sequence        INTEGER NOT NULL,
  role            TEXT NOT NULL,
  agent_id        TEXT,
  agent_name      TEXT NOT NULL,
  model           TEXT NOT NULL,
  status          TEXT NOT NULL,
  input_summary   TEXT,
  output_summary  TEXT,
  token_usage     TEXT NOT NULL,
  cost_estimate   TEXT NOT NULL,
  error_category  TEXT,
  error_message   TEXT,
  requested_by    TEXT NOT NULL,
  chain_label     TEXT,
  created_at      TEXT NOT NULL,
  started_at      TEXT,
  completed_at    TEXT,
  failed_at       TEXT,
  updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_project_id ON agent_runs(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_chain_id ON agent_runs(chain_id);
CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status);
CREATE INDEX IF NOT EXISTS idx_agent_runs_created_at ON agent_runs(created_at);

CREATE TABLE IF NOT EXISTS runtime_events (
  id            TEXT PRIMARY KEY,
  project_id    TEXT NOT NULL,
  entity_type   TEXT NOT NULL,
  entity_id     TEXT NOT NULL,
  event_type    TEXT NOT NULL,
  before_state  TEXT,
  after_state   TEXT,
  actor         TEXT,
  reason        TEXT,
  created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runtime_events_project_id ON runtime_events(project_id);
CREATE INDEX IF NOT EXISTS idx_runtime_events_entity_id ON runtime_events(entity_id);
