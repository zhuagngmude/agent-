-- 004_add_project_plan_workflow.sql
-- project_plan / Workflow 最小闭环：本地草案 + 只读 Runner request 队列

CREATE TABLE IF NOT EXISTS project_plan_drafts (
  id             TEXT PRIMARY KEY,
  project_id     TEXT NOT NULL,
  approval_id    TEXT NOT NULL,
  idea           TEXT NOT NULL,
  constraints    TEXT,
  summary        TEXT NOT NULL,
  status         TEXT NOT NULL,
  generated_by   TEXT NOT NULL,
  requested_by   TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id)
);

CREATE INDEX IF NOT EXISTS idx_project_plan_drafts_project_id ON project_plan_drafts(project_id);
CREATE INDEX IF NOT EXISTS idx_project_plan_drafts_approval_id ON project_plan_drafts(approval_id);
CREATE INDEX IF NOT EXISTS idx_project_plan_drafts_status ON project_plan_drafts(status);

CREATE TABLE IF NOT EXISTS runner_requests (
  id               TEXT PRIMARY KEY,
  project_id       TEXT NOT NULL,
  approval_id      TEXT NOT NULL,
  task_id          TEXT NOT NULL,
  status           TEXT NOT NULL,
  operation_types  TEXT NOT NULL,
  affected_files   TEXT NOT NULL,
  checkpoint       TEXT,
  safety_note      TEXT NOT NULL,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_requests_project_id ON runner_requests(project_id);
CREATE INDEX IF NOT EXISTS idx_runner_requests_approval_id ON runner_requests(approval_id);
CREATE INDEX IF NOT EXISTS idx_runner_requests_task_id ON runner_requests(task_id);
CREATE INDEX IF NOT EXISTS idx_runner_requests_status ON runner_requests(status);
