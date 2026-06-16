-- 008_add_runner_execution_gates.sql
-- 阶段 31：Runner 执行许可 gate。gate 仍被阶段边界锁定，不执行 Runner。

CREATE TABLE IF NOT EXISTS runner_execution_gates (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  preflight_review_id TEXT NOT NULL,
  preflight_approval_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('blocked_by_stage_boundary', 'revoked')),
  risk_level TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 0 CHECK (can_execute = 0),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 1 CHECK (stage_boundary_locked = 1),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1,
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  revoked_reason TEXT,
  requested_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id),
  FOREIGN KEY (preflight_review_id) REFERENCES runner_preflight_reviews(id),
  FOREIGN KEY (preflight_approval_id) REFERENCES approvals(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_project_id
  ON runner_execution_gates(project_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_gates_project_runner_request
  ON runner_execution_gates(project_id, runner_request_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_gates_project_preflight_review
  ON runner_execution_gates(project_id, preflight_review_id);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_status
  ON runner_execution_gates(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_runner_execution_gates_preflight_approval
  ON runner_execution_gates(preflight_approval_id);
