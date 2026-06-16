-- Migration 012: 模型目录（阶段 35）
-- 受控模型目录，前端只能从 enabled=true 的记录中选择模型。
-- 不存储 raw key、base URL、prompt 或 provider error。
-- 第一版 provider 固定 openai_compat，purpose 固定 project_plan_generation。

CREATE TABLE IF NOT EXISTS model_catalog (
  id           TEXT NOT NULL PRIMARY KEY,
  project_id   TEXT NOT NULL,
  provider     TEXT NOT NULL DEFAULT 'openai_compat',
  model_id     TEXT NOT NULL,
  display_name TEXT NOT NULL DEFAULT '',
  purpose      TEXT NOT NULL DEFAULT 'project_plan_generation',
  enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  is_builtin   INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

-- 唯一索引：同一 project 下 provider+model_id+purpose 不能重复
CREATE UNIQUE INDEX IF NOT EXISTS idx_model_catalog_unique
  ON model_catalog (project_id, provider, model_id, purpose);

CREATE INDEX IF NOT EXISTS idx_model_catalog_project_enabled
  ON model_catalog (project_id, enabled);

CREATE INDEX IF NOT EXISTS idx_model_catalog_purpose
  ON model_catalog (project_id, purpose);
