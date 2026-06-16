// 阶段 37：想法引导官类型定义
// 权限级别 L1（模型草案），不执行 Runner、不写文件、不改 Git

// ── Session ──
export type IdeaGuidanceSession = {
  id: string;
  project_id: string;
  status: "draft" | "questions_ready" | "seed_ready" | "cancelled" | "failed";
  source: "manual" | "model_guided";
  idea_summary: string;
  constraints_summary: string | null;
  model_call_id: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
};

// ── Question ──
export type IdeaGuidanceQuestion = {
  id: string;
  project_id: string;
  session_id: string;
  sort_order: number;
  question: string;
  answer: string | null;
  status: "pending" | "answered" | "skipped";
  created_at: string;
  updated_at: string;
};

// ── Seed ──
export type ProjectSeed = {
  id: string;
  project_id: string;
  session_id: string;
  status: "draft" | "ready" | "converted" | "cancelled";
  product_goal: string | null;
  target_users: string | null;
  mvp_scope: string | null;
  non_goals: string | null;
  key_features: string | null;
  pages_or_modules: string | null;
  data_entities: string | null;
  technical_constraints: string | null;
  acceptance_criteria: string | null;
  risk_points: string | null;
  open_questions: string | null;
  recommended_next_step: string | null;
  model_call_id: string | null;
  created_at: string;
  updated_at: string;
};

// ── Command Inputs ──
export type CreateIdeaGuidanceQuestionsInput = {
  idea: string;
  constraints?: string | null;
  model_record_id?: string | null;
  second_confirm: boolean;
  confirm_text?: string | null;
};

export type GenerateProjectSeedInput = {
  session_id: string;
  model_record_id?: string | null;
  second_confirm: boolean;
  confirm_text?: string | null;
};

export type SaveGuidanceAnswersInput = {
  session_id: string;
  answers: { question_id: string; answer: string }[];
};

// ── Command Responses ──
export type CreateIdeaGuidanceQuestionsResponse = {
  session: IdeaGuidanceSession;
  questions: IdeaGuidanceQuestion[];
  audit_record_id: string | null;
  warnings: string[];
};

export type GenerateProjectSeedResponse = {
  seed: ProjectSeed;
  session: IdeaGuidanceSession;
  audit_record_id: string | null;
  warnings: string[];
};
