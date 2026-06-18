-- 009_add_runner_dry_runs.sql
-- 阶段 32：Runner dry-run 预演。只保存预演计划，不执行。

CREATE TABLE IF NOT EXISTS runner_dry_runs (
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

CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_project_id ON runner_dry_runs(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_dry_runs_project_gate ON runner_dry_runs(project_id, gate_id);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_runner_request ON runner_dry_runs(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_status ON runner_dry_runs(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_dry_runs_task_id ON runner_dry_runs(project_id, task_id);
