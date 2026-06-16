-- Migration 013: 想法引导官（阶段 37）
-- 三张表：会话、追问、项目种子
-- 权限级别 L1（模型草案），不执行 Runner、不写文件、不改 Git
-- 所有模型调用必须走 Model Gateway + 受控模型目录 + model_calls 审计

-- 1. 想法引导会话
CREATE TABLE IF NOT EXISTS idea_guidance_sessions (
  id                 TEXT NOT NULL PRIMARY KEY,
  project_id         TEXT NOT NULL,
  status             TEXT NOT NULL DEFAULT 'draft'
                     CHECK (status IN ('draft','questions_ready','seed_ready','cancelled','failed')),
  source             TEXT NOT NULL DEFAULT 'model_guided'
                     CHECK (source IN ('manual','model_guided')),
  idea_summary       TEXT NOT NULL,
  constraints_summary TEXT,
  model_call_id      TEXT,
  created_by         TEXT NOT NULL DEFAULT 'local_user',
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (model_call_id) REFERENCES model_calls(id)
);

-- 2. 追问列表
CREATE TABLE IF NOT EXISTS idea_guidance_questions (
  id           TEXT NOT NULL PRIMARY KEY,
  project_id   TEXT NOT NULL,
  session_id   TEXT NOT NULL,
  sort_order   INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
  question     TEXT NOT NULL,
  answer       TEXT,
  status       TEXT NOT NULL DEFAULT 'pending'
               CHECK (status IN ('pending','answered','skipped')),
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (session_id) REFERENCES idea_guidance_sessions(id)
);

-- 3. 项目种子草案
CREATE TABLE IF NOT EXISTS project_seeds (
  id                      TEXT NOT NULL PRIMARY KEY,
  project_id              TEXT NOT NULL,
  session_id              TEXT NOT NULL,
  status                  TEXT NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft','ready','converted','cancelled')),
  product_goal            TEXT,
  target_users            TEXT,
  mvp_scope               TEXT,
  non_goals               TEXT,
  key_features            TEXT,
  pages_or_modules        TEXT,
  data_entities           TEXT,
  technical_constraints   TEXT,
  acceptance_criteria     TEXT,
  risk_points             TEXT,
  open_questions          TEXT,
  recommended_next_step   TEXT,
  model_call_id           TEXT,
  created_at              TEXT NOT NULL,
  updated_at              TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (session_id) REFERENCES idea_guidance_sessions(id),
  FOREIGN KEY (model_call_id) REFERENCES model_calls(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_idea_sessions_project
  ON idea_guidance_sessions(project_id);

CREATE INDEX IF NOT EXISTS idx_idea_sessions_status
  ON idea_guidance_sessions(project_id, status);

CREATE INDEX IF NOT EXISTS idx_idea_sessions_model_call
  ON idea_guidance_sessions(model_call_id);

CREATE INDEX IF NOT EXISTS idx_idea_questions_session
  ON idea_guidance_questions(session_id);

CREATE INDEX IF NOT EXISTS idx_project_seeds_project
  ON project_seeds(project_id);

CREATE INDEX IF NOT EXISTS idx_project_seeds_session
  ON project_seeds(session_id);

CREATE INDEX IF NOT EXISTS idx_project_seeds_model_call
  ON project_seeds(model_call_id);
