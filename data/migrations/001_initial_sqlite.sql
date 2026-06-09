PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  phase TEXT,
  description TEXT,
  workspace_path TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  status TEXT NOT NULL,
  version TEXT,
  model TEXT,
  can_spawn_sub_agents INTEGER NOT NULL,
  max_sub_agents INTEGER NOT NULL,
  permissions TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_agents_project_id ON agents(project_id);
CREATE INDEX IF NOT EXISTS idx_agents_role ON agents(role);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);

CREATE TABLE IF NOT EXISTS agent_relationships (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  parent_agent_id TEXT,
  child_agent_id TEXT NOT NULL UNIQUE,
  reports_to_agent_id TEXT,
  spawn_depth INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (parent_agent_id) REFERENCES agents(id),
  FOREIGN KEY (child_agent_id) REFERENCES agents(id),
  FOREIGN KEY (reports_to_agent_id) REFERENCES agents(id)
);

CREATE INDEX IF NOT EXISTS idx_agent_relationships_project_id ON agent_relationships(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_relationships_parent_agent_id ON agent_relationships(parent_agent_id);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  priority TEXT,
  assigned_agent_id TEXT,
  risk_level TEXT,
  related_files TEXT,
  requires_approval INTEGER NOT NULL,
  depends_on TEXT,
  started_at TEXT,
  completed_at TEXT,
  failed_at TEXT,
  cancelled_at TEXT,
  failure_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (assigned_agent_id) REFERENCES agents(id)
);

CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_agent_id ON tasks(assigned_agent_id);

CREATE TABLE IF NOT EXISTS approvals (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  risk_tone TEXT,
  request_agent_id TEXT,
  request_agent_name TEXT,
  target_service TEXT,
  operation_types TEXT NOT NULL,
  reason TEXT,
  checkpoint_required INTEGER NOT NULL,
  checkpoint_created INTEGER NOT NULL,
  checkpoint_commit TEXT,
  affected_files TEXT,
  diff_summary TEXT,
  diff_preview TEXT,
  requires_second_confirm INTEGER NOT NULL,
  change_request TEXT,
  runner_job_id TEXT,
  patch_artifact_id TEXT,
  reject_reason TEXT,
  approved_at TEXT,
  rejected_at TEXT,
  patch_only_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (request_agent_id) REFERENCES agents(id)
);

CREATE INDEX IF NOT EXISTS idx_approvals_project_id ON approvals(project_id);
CREATE INDEX IF NOT EXISTS idx_approvals_status ON approvals(status);
CREATE INDEX IF NOT EXISTS idx_approvals_target_service ON approvals(target_service);

CREATE TABLE IF NOT EXISTS runner_jobs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  approval_id TEXT NOT NULL,
  task_id TEXT,
  status TEXT NOT NULL,
  operation_types TEXT NOT NULL,
  affected_files TEXT,
  checkpoint_commit TEXT,
  safety_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id),
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_jobs_project_id ON runner_jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_runner_jobs_approval_id ON runner_jobs(approval_id);
CREATE INDEX IF NOT EXISTS idx_runner_jobs_status ON runner_jobs(status);

CREATE TABLE IF NOT EXISTS agent_config_applications (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  approval_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  agent_name TEXT,
  change_type TEXT NOT NULL,
  changes TEXT NOT NULL,
  status TEXT NOT NULL,
  applied_at TEXT,
  applied_by TEXT,
  apply_confirm_text TEXT,
  cancelled_at TEXT,
  cancelled_by TEXT,
  cancel_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id),
  FOREIGN KEY (agent_id) REFERENCES agents(id)
);

CREATE INDEX IF NOT EXISTS idx_agent_config_applications_project_id ON agent_config_applications(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_config_applications_approval_id ON agent_config_applications(approval_id);
CREATE INDEX IF NOT EXISTS idx_agent_config_applications_status ON agent_config_applications(status);

CREATE TABLE IF NOT EXISTS agent_config_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  approval_id TEXT NOT NULL,
  application_id TEXT NOT NULL,
  config_snapshot TEXT NOT NULL,
  changes TEXT NOT NULL,
  applied_by TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE (agent_id, version),
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (agent_id) REFERENCES agents(id),
  FOREIGN KEY (approval_id) REFERENCES approvals(id),
  FOREIGN KEY (application_id) REFERENCES agent_config_applications(id)
);

CREATE INDEX IF NOT EXISTS idx_agent_config_versions_project_id ON agent_config_versions(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_config_versions_agent_id ON agent_config_versions(agent_id);

CREATE TABLE IF NOT EXISTS workflows (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  description TEXT,
  steps TEXT,
  stats TEXT,
  nodes TEXT,
  edges TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_workflows_project_id ON workflows(project_id);
CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status);

CREATE TABLE IF NOT EXISTS runner_status (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  connected INTEGER NOT NULL,
  runner_id TEXT NOT NULL,
  version TEXT NOT NULL,
  workspace_path TEXT,
  permissions TEXT NOT NULL,
  last_heartbeat_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_runner_status_project_id ON runner_status(project_id);

CREATE TABLE IF NOT EXISTS knowledge_updates (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  document TEXT NOT NULL,
  section TEXT,
  status TEXT NOT NULL,
  related_feature TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_updates_project_id ON knowledge_updates(project_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_updates_status ON knowledge_updates(status);

CREATE TABLE IF NOT EXISTS git_checkpoints (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  commit_hash TEXT NOT NULL,
  message TEXT NOT NULL,
  type TEXT,
  related_task_id TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (related_task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_git_checkpoints_project_id ON git_checkpoints(project_id);
CREATE INDEX IF NOT EXISTS idx_git_checkpoints_commit_hash ON git_checkpoints(commit_hash);

CREATE TABLE IF NOT EXISTS runtime_events (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  before_state TEXT,
  after_state TEXT,
  actor TEXT,
  reason TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_runtime_events_project_id ON runtime_events(project_id);
CREATE INDEX IF NOT EXISTS idx_runtime_events_entity ON runtime_events(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_runtime_events_created_at ON runtime_events(created_at);
