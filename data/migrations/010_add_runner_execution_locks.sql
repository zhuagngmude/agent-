-- 010_add_runner_execution_locks.sql
-- 阶段 33：Runner 执行范围锁。锁定文件范围，不执行。

CREATE TABLE IF NOT EXISTS runner_execution_locks (
  id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
  dry_run_id TEXT NOT NULL, gate_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL, task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('locked','revoked')),
  allowed_files TEXT NOT NULL, denied_paths TEXT NOT NULL,
  planned_commands TEXT NOT NULL, planned_file_changes TEXT NOT NULL,
  checkpoint_strategy TEXT NOT NULL, workspace_requirements TEXT NOT NULL,
  blocked_reasons TEXT NOT NULL,
  can_execute INTEGER NOT NULL DEFAULT 0 CHECK (can_execute = 0),
  stage_boundary_locked INTEGER NOT NULL DEFAULT 1 CHECK (stage_boundary_locked = 1),
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1 CHECK (requires_git_checkpoint = 1),
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  requested_by TEXT NOT NULL, revoked_reason TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, revoked_at TEXT,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (dry_run_id) REFERENCES runner_dry_runs(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_project_id ON runner_execution_locks(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_execution_locks_project_dry_run ON runner_execution_locks(project_id, dry_run_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_runner_request ON runner_execution_locks(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_status ON runner_execution_locks(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_execution_locks_task_id ON runner_execution_locks(project_id, task_id);
