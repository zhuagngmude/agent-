export type ProjectType =
  | "software_product"
  | "ai_automation"
  | "content_creation"
  | "business_plan"
  | "general_goal";

export type ProjectIntakeSession = {
  id: string;
  project_id: string;
  raw_idea: string;
  normalized_idea: string;
  project_type: ProjectType;
  project_type_label: string;
  confidence: number;
  reason: string;
  recommended_questions: string[];
  recommended_next_step: string;
  status: "classified" | "converted" | "cancelled";
  created_by: string;
  created_at: string;
  updated_at: string;
};

export type ProjectIntakeSideEffects = {
  calls_real_model: boolean;
  creates_tasks: boolean;
  creates_approvals: boolean;
  executes_runner: boolean;
  writes_project_files: boolean;
  modifies_git: boolean;
};

export type ClassifyProjectIntakeInput = {
  idea: string;
};

export type ClassifyProjectIntakeResponse = {
  session: ProjectIntakeSession;
  side_effects: ProjectIntakeSideEffects;
};
