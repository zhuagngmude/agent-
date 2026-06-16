-- 011_add_runner_minimal_runs.sql
-- 阶段 34：最小真实 Runner 执行。只允许沙箱路径，固定白名单命令。

CREATE TABLE IF NOT EXISTS runner_minimal_runs (
  id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
  execution_lock_id TEXT NOT NULL, dry_run_id TEXT NOT NULL,
  gate_id TEXT NOT NULL, runner_request_id TEXT NOT NULL, task_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('created','running','succeeded','failed','failed_scope_violation','aborted')),
  allowed_files TEXT NOT NULL, written_files TEXT NOT NULL,
  command_plan TEXT NOT NULL, command_results TEXT NOT NULL,
  pre_git_status_summary TEXT NOT NULL, pre_git_diff_stat TEXT NOT NULL,
  post_git_status_summary TEXT, post_git_diff_stat TEXT,
  failure_category TEXT, failure_summary TEXT,
  side_effects TEXT NOT NULL,
  second_confirmed INTEGER NOT NULL DEFAULT 1 CHECK (second_confirmed = 1),
  requested_by TEXT NOT NULL,
  started_at TEXT, finished_at TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (execution_lock_id) REFERENCES runner_execution_locks(id),
  FOREIGN KEY (dry_run_id) REFERENCES runner_dry_runs(id),
  FOREIGN KEY (gate_id) REFERENCES runner_execution_gates(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);
CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_project_id ON runner_minimal_runs(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_minimal_runs_project_lock ON runner_minimal_runs(project_id, execution_lock_id);
CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_runner_request ON runner_minimal_runs(project_id, runner_request_id);
CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_status ON runner_minimal_runs(project_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_runner_minimal_runs_task_id ON runner_minimal_runs(project_id, task_id);
