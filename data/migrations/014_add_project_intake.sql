-- Migration 014: 项目类型分流与通用想法入口（阶段 38）
-- 只记录用户首页一句话入口、规则分类结果和推荐问题。
-- 不调用模型、不创建任务、不创建审批、不执行 Runner、不写用户项目文件、不改 Git。

CREATE TABLE IF NOT EXISTS project_intake_sessions (
  id                    TEXT NOT NULL PRIMARY KEY,
  project_id            TEXT NOT NULL,
  raw_idea              TEXT NOT NULL,
  normalized_idea       TEXT NOT NULL,
  project_type          TEXT NOT NULL
                        CHECK (project_type IN ('software_product','ai_automation','content_creation','business_plan','general_goal')),
  project_type_label    TEXT NOT NULL,
  confidence            INTEGER NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 100),
  reason                TEXT NOT NULL,
  recommended_questions TEXT NOT NULL,
  recommended_next_step TEXT NOT NULL,
  status                TEXT NOT NULL DEFAULT 'classified'
                        CHECK (status IN ('classified','converted','cancelled')),
  created_by            TEXT NOT NULL DEFAULT 'local_user',
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_project_intake_project
  ON project_intake_sessions(project_id);

CREATE INDEX IF NOT EXISTS idx_project_intake_type
  ON project_intake_sessions(project_id, project_type);

CREATE INDEX IF NOT EXISTS idx_project_intake_created
  ON project_intake_sessions(project_id, created_at);
