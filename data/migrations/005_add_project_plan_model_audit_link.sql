-- 005_add_project_plan_model_audit_link.sql
-- 阶段 26：project_plan_drafts 新增 model_call_id，关联 model_calls 安全审计记录
-- ALTER TABLE ADD COLUMN 由 Rust 迁移代码处理（SQLite 不支持 IF NOT EXISTS 的 ALTER TABLE）

CREATE INDEX IF NOT EXISTS idx_project_plan_drafts_model_call_id ON project_plan_drafts(model_call_id);
