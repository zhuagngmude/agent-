-- 016_open_runner_full_auto.sql
-- Runner full-auto mode: gates and dry-runs become approved records, while
-- locks keep the allowed-file scope without blocking execution.

PRAGMA foreign_keys=OFF;

CREATE TABLE IF NOT EXISTS runner_execution_gates_v3 (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  preflight_review_id TEXT NOT NULL,
  preflight_approval_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('approved', 'blocked_by_stage_boundary', 'revoked')),
  risk_level TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 1 CHECK (can_execute IN (0,1)),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 0 CHECK (stage_boundary_locked IN (0,1)),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 0 CHECK (requires_git_checkpoint IN (0,1)),
  requires_second_confirm INTEGER NOT NULL DEFAULT 0 CHECK (requires_second_confirm IN (0,1)),
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

INSERT OR IGNORE INTO runner_execution_gates_v3 (
  id, project_id, runner_request_id, task_id, preflight_review_id, preflight_approval_id,
  status, risk_level, operation_types, affected_files, blocked_reasons,
  can_execute, stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
  revoked_reason, requested_by, created_at, updated_at, revoked_at
)
SELECT
  id, project_id, runner_request_id, task_id, preflight_review_id, preflight_approval_id,
  CASE WHEN status = 'revoked' THEN 'revoked' ELSE 'approved' END,
  risk_level, operation_types, affected_files,
  CASE WHEN status = 'revoked' THEN blocked_reasons ELSE '["runner_execution_auto_approved"]' END,
  CASE WHEN status = 'revoked' THEN can_execute ELSE 1 END,
  CASE WHEN status = 'revoked' THEN stage_boundary_locked ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_git_checkpoint ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_second_confirm ELSE 0 END,
  revoked_reason, requested_by, created_at, updated_at, revoked_at
FROM runner_execution_gates;

DROP TABLE runner_execution_gates;
ALTER TABLE runner_execution_gates_v3 RENAME TO runner_execution_gates;

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

CREATE TABLE IF NOT EXISTS runner_dry_runs_v3 (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('approved', 'blocked_by_stage_boundary', 'revoked')),
  risk_level TEXT NOT NULL,
  planned_operations TEXT NOT NULL,
  planned_commands TEXT NOT NULL,
  planned_file_changes TEXT NOT NULL,
  allowed_files TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  safety_summary TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 1 CHECK (can_execute IN (0,1)),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 0 CHECK (stage_boundary_locked IN (0,1)),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 0 CHECK (requires_git_checkpoint IN (0,1)),
  requires_second_confirm INTEGER NOT NULL DEFAULT 0 CHECK (requires_second_confirm IN (0,1)),
  requested_by TEXT NOT NULL,
  revoked_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

INSERT OR IGNORE INTO runner_dry_runs_v3 (
  id, project_id, gate_id, runner_request_id, task_id, status, risk_level,
  planned_operations, planned_commands, planned_file_changes, allowed_files,
  blocked_reasons, safety_summary, can_execute, stage_boundary_locked,
  requires_git_checkpoint, requires_second_confirm, requested_by, revoked_reason,
  created_at, updated_at, revoked_at
)
SELECT
  id, project_id, gate_id, runner_request_id, task_id,
  CASE WHEN status = 'revoked' THEN 'revoked' ELSE 'approved' END,
  risk_level, planned_operations, planned_commands, planned_file_changes, allowed_files,
  CASE WHEN status = 'revoked' THEN blocked_reasons ELSE '[]' END,
  safety_summary,
  CASE WHEN status = 'revoked' THEN can_execute ELSE 1 END,
  CASE WHEN status = 'revoked' THEN stage_boundary_locked ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_git_checkpoint ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_second_confirm ELSE 0 END,
  requested_by, revoked_reason, created_at, updated_at, revoked_at
FROM runner_dry_runs;

DROP TABLE runner_dry_runs;
ALTER TABLE runner_dry_runs_v3 RENAME TO runner_dry_runs;

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_project_id ON runner_dry_runs(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_dry_runs_project_gate ON runner_dry_runs(project_id, gate_id);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_runner_request ON runner_dry_runs(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_status ON runner_dry_runs(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_task_id ON runner_dry_runs(project_id, task_id);

CREATE TABLE IF NOT EXISTS runner_execution_locks_v3 (
  id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
  dry_run_id TEXT NOT NULL, gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL, task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('locked','revoked')),
  allowed_files TEXT NOT NULL, denied_paths TEXT NOT NULL,
  planned_commands TEXT NOT NULL, planned_file_changes TEXT NOT NULL,
  checkpoint_strategy TEXT NOT NULL, workspace_requirements TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 1 CHECK (can_execute IN (0,1)),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 0 CHECK (stage_boundary_locked IN (0,1)),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 0 CHECK (requires_git_checkpoint IN (0,1)),
  requires_second_confirm INTEGER NOT NULL DEFAULT 0 CHECK (requires_second_confirm IN (0,1)),
  requested_by TEXT NOT NULL, revoked_reason TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (dry_run_id) REFERENCES runner_dry_runs(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

INSERT OR IGNORE INTO runner_execution_locks_v3 (
  id, project_id, dry_run_id, gate_id, runner_request_id, task_id,
  status, allowed_files, denied_paths, planned_commands, planned_file_changes,
  checkpoint_strategy, workspace_requirements, blocked_reasons, can_execute,
  stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
  requested_by, revoked_reason, created_at, updated_at, revoked_at
)
SELECT
  id, project_id, dry_run_id, gate_id, runner_request_id, task_id,
  status, allowed_files, denied_paths, planned_commands, planned_file_changes,
  checkpoint_strategy, workspace_requirements,
  CASE WHEN status = 'revoked' THEN blocked_reasons ELSE '["file_scope_locked_for_stage34"]' END,
  CASE WHEN status = 'revoked' THEN can_execute ELSE 1 END,
  CASE WHEN status = 'revoked' THEN stage_boundary_locked ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_git_checkpoint ELSE 0 END,
  CASE WHEN status = 'revoked' THEN requires_second_confirm ELSE 0 END,
  requested_by, revoked_reason, created_at, updated_at, revoked_at
FROM runner_execution_locks;

DROP TABLE runner_execution_locks;
ALTER TABLE runner_execution_locks_v3 RENAME TO runner_execution_locks;

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_project_id ON runner_execution_locks(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_locks_project_dry_run ON runner_execution_locks(project_id, dry_run_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_runner_request ON runner_execution_locks(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_status ON runner_execution_locks(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_task_id ON runner_execution_locks(project_id, task_id);

PRAGMA foreign_keys=ON;
