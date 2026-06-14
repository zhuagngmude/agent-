-- 003_add_model_calls.sql
-- Model Calls 审计记录表（当前阶段只建表，feature_disabled 时不写入）

CREATE TABLE IF NOT EXISTS model_calls (
  id                    TEXT PRIMARY KEY,
  project_id            TEXT NOT NULL,
  purpose               TEXT NOT NULL,
  provider              TEXT NOT NULL,
  model                 TEXT NOT NULL,
  status                TEXT NOT NULL,
  request_hash          TEXT,
  structured_summary    TEXT,
  token_usage           TEXT NOT NULL DEFAULT '{}',
  cost_estimate         TEXT NOT NULL DEFAULT '{}',
  error_category        TEXT,
  error_message         TEXT,
  redaction_applied     INTEGER NOT NULL DEFAULT 0,
  duration_ms           INTEGER,
  related_approval_id   TEXT,
  runtime_event_id      TEXT,
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_calls_project_id ON model_calls(project_id);
CREATE INDEX IF NOT EXISTS idx_model_calls_status ON model_calls(status);
CREATE INDEX IF NOT EXISTS idx_model_calls_created_at ON model_calls(created_at);
