-- 007_add_runner_preflight_reviews.sql
-- 阶段 30：Runner 执行前审查闸门。只保存审查记录，不执行 Runner。

CREATE TABLE IF NOT EXISTS runner_preflight_reviews (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  runner_request_id TEXT NOT NULL,
  task_id TEXT NOT NULL,
  approval_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT NOT NULL,
  requires_git_checkpoint INTEGER NOT NULL DEFAULT 1,
  requires_second_confirm INTEGER NOT NULL DEFAULT 1,
  blocked_reasons TEXT NOT NULL,
  safety_summary TEXT NOT NULL,
  requested_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (runner_request_id) REFERENCES runner_requests(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_project_id
  ON runner_preflight_reviews(project_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_runner_request_id
  ON runner_preflight_reviews(runner_request_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_runner_preflight_reviews_project_runner_request
  ON runner_preflight_reviews(project_id, runner_request_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_approval_id
  ON runner_preflight_reviews(approval_id);

CREATE INDEX IF NOT EXISTS idx_runner_preflight_reviews_status
  ON runner_preflight_reviews(project_id, status, created_at);
