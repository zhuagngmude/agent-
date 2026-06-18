-- 015_enable_auto_runner_execution_locks.sql
-- Open the auto Runner path by allowing execution locks to carry can_execute=1.
PRAGMA foreign_keys=OFF;

CREATE TABLE IF NOT EXISTS runner_execution_locks_v2 (
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

INSERT OR IGNORE INTO runner_execution_locks_v2 (
  id, project_id, dry_run_id, gate_id, runner_request_id, task_id,
  status, allowed_files, denied_paths, planned_commands, planned_file_changes,
  checkpoint_strategy, workspace_requirements, blocked_reasons, can_execute,
  stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
  requested_by, revoked_reason, created_at, updated_at, revoked_at
)
SELECT
  id, project_id, dry_run_id, gate_id, runner_request_id, task_id,
  status, allowed_files, denied_paths, planned_commands, planned_file_changes,
  checkpoint_strategy, workspace_requirements, blocked_reasons, can_execute,
  stage_boundary_locked, requires_git_checkpoint, requires_second_confirm,
  requested_by, revoked_reason, created_at, updated_at, revoked_at
FROM runner_execution_locks;

DROP TABLE runner_execution_locks;
ALTER TABLE runner_execution_locks_v2 RENAME TO runner_execution_locks;

CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_project_id ON runner_execution_locks(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_locks_project_dry_run ON runner_execution_locks(project_id, dry_run_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_runner_request ON runner_execution_locks(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_status ON runner_execution_locks(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_task_id ON runner_execution_locks(project_id, task_id);

PRAGMA foreign_keys=ON;
