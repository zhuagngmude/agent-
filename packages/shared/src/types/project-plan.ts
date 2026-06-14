import type { ApprovalSummary } from "./approval";

export type ProjectPlanDraftSummary = {
  id: string;
  project_id: string;
  approval_id: string;
  idea: string;
  constraints: string | null;
  summary: string;
  status: string;
  generated_by: string;
  requested_by: string;
  created_at: string;
  updated_at: string;
};

export type PlannedTaskSummary = {
  id: string;
  role: string;
  title: string;
  description: string;
  status: string;
  priority: string;
  assigned_agent_id: string;
  depends_on: string[];
  risk_level: string;
  operation_types: string[];
  affected_files: string[];
};

export type RunnerRequestSummary = {
  id: string;
  project_id: string;
  approval_id: string;
  task_id: string;
  status: string;
  operation_types: string[];
  affected_files: string[];
  checkpoint: string | null;
  safety_note: string;
  created_at: string;
  updated_at: string;
};

export type ProjectPlanSideEffects = {
  writes_project_files: boolean;
  modifies_git: boolean;
  executes_runner: boolean;
  calls_real_model: boolean;
  reads_raw_secrets: boolean;
  makes_network_requests: boolean;
  triggers_agents: boolean;
  creates_tasks: boolean;
  creates_runner_requests: boolean;
};

export type CreateProjectPlanDraftInput = {
  idea: string;
  constraints?: string | null;
  requested_by?: string | null;
};

export type ApproveProjectPlanInput = {
  approval_id: string;
  second_confirm: boolean;
  confirm_text: string;
};

export type CreateProjectPlanDraftResponse = {
  draft: ProjectPlanDraftSummary;
  approval: ApprovalSummary;
  planned_tasks: PlannedTaskSummary[];
  planned_runner_requests: RunnerRequestSummary[];
  side_effects: ProjectPlanSideEffects;
};

export type ApproveProjectPlanResponse = {
  approval: ApprovalSummary;
  draft: ProjectPlanDraftSummary;
  created_task_ids: string[];
  created_runner_request_ids: string[];
  side_effects: ProjectPlanSideEffects;
};
