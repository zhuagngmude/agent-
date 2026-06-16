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
  /** 阶段 26：关联的 model_calls 审计记录 id；本地确定性草案时为 null */
  model_call_id: string | null;
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

export type AutoGenerateProjectPlanTasksInput = CreateProjectPlanDraftInput;

export type AutoRunSwarmIdeaInput = CreateProjectPlanDraftInput;

export type ApproveProjectPlanInput = {
  approval_id: string;
  second_confirm: boolean;
  confirm_text: string;
};

export type DeleteProjectPlanDraftInput = {
  draft_id: string;
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

export type DeleteProjectPlanDraftResponse = {
  deleted_draft_id: string;
  deleted_approval_id: string;
  side_effects: ProjectPlanSideEffects;
};

export type ProjectPlanModelDraftInput = {
  idea: string;
  constraints?: string | null;
  second_confirm: boolean;
  confirm_text?: string | null;
  /** 阶段 35：可选，来自 model_catalog 目录的 id */
  model_record_id?: string | null;
};

export type ProjectPlanModelDraftResponse = {
  status: string;
  error_category?: string | null;
  summary?: string | null;
  warnings: string[];
  /** 阶段 25.3：写入 model_calls 审计记录后返回 id；未进入 provider 阶段时为 null */
  audit_record_id?: string | null;
};

export type ProjectPlanTaskTemplateSummary = {
  id: string;
  project_id: string;
  role: string;
  agent_id: string;
  title: string;
  description: string;
  priority: string;
  risk_level: string;
  affected_file: string;
  operation_type: string;
  enabled: boolean;
  sort_order: number;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
};

export type CreateRunnerDryRunInput = {
  gate_id: string; second_confirm: boolean; confirm_text: string; requested_by?: string | null;
};
export type RevokeRunnerDryRunInput = {
  dry_run_id: string; second_confirm: boolean; confirm_text: string; revoked_reason?: string | null;
};
export type PlannedFileChangeSummary = { path: string; change_type: string; reason: string };
export type RunnerDryRunSummary = {
  id: string; project_id: string; gate_id: string; runner_request_id: string; task_id: string;
  status: string; risk_level: string; planned_operations: string[]; planned_commands: string[];
  planned_file_changes: PlannedFileChangeSummary[]; allowed_files: string[];
  blocked_reasons: string[]; safety_summary: string;
  can_execute: boolean; stage_boundary_locked: boolean;
  requires_git_checkpoint: boolean; requires_second_confirm: boolean;
  requested_by: string; revoked_reason: string | null;
  created_at: string; updated_at: string; revoked_at: string | null;
};
export type CreateRunnerDryRunResponse = { dry_run: RunnerDryRunSummary; side_effects: ProjectPlanSideEffects };
export type RevokeRunnerDryRunResponse = { dry_run: RunnerDryRunSummary; side_effects: ProjectPlanSideEffects };

export type CreateRunnerExecutionLockInput = { dry_run_id: string; second_confirm: boolean; confirm_text: string; requested_by?: string | null };
export type RevokeRunnerExecutionLockInput = { execution_lock_id: string; second_confirm: boolean; confirm_text: string; revoked_reason?: string | null };
export type RunnerExecutionLockSummary = {
  id: string; project_id: string; dry_run_id: string; gate_id: string; runner_request_id: string; task_id: string;
  status: string; allowed_files: string[]; denied_paths: string[];
  planned_commands: string[]; planned_file_changes: PlannedFileChangeSummary[];
  checkpoint_strategy: string; workspace_requirements: string; blocked_reasons: string[];
  can_execute: boolean; stage_boundary_locked: boolean; requires_git_checkpoint: boolean; requires_second_confirm: boolean;
  requested_by: string; revoked_reason: string | null; created_at: string; updated_at: string; revoked_at: string | null;
};
export type CreateRunnerExecutionLockResponse = { execution_lock: RunnerExecutionLockSummary; side_effects: ProjectPlanSideEffects };
export type RevokeRunnerExecutionLockResponse = { execution_lock: RunnerExecutionLockSummary; side_effects: ProjectPlanSideEffects };

export type CreateRunnerMinimalRunInput = { execution_lock_id: string; second_confirm: boolean; confirm_text: string; requested_by?: string | null };
export type RunnerCommandResultSummary = { command: string; status: string; exit_code: number | null; stdout_summary: string; stderr_summary: string };
export type RunnerMinimalRunSummary = {
  id: string; project_id: string; execution_lock_id: string; dry_run_id: string; gate_id: string; runner_request_id: string; task_id: string;
  status: string; allowed_files: string[]; written_files: string[]; command_plan: string[];
  command_results: RunnerCommandResultSummary[]; pre_git_status_summary: string; pre_git_diff_stat: string;
  post_git_status_summary: string | null; post_git_diff_stat: string | null;
  failure_category: string | null; failure_summary: string | null;
  side_effects: ProjectPlanSideEffects; requested_by: string;
  started_at: string | null; finished_at: string | null; created_at: string; updated_at: string;
};
export type CreateRunnerMinimalRunResponse = { run: RunnerMinimalRunSummary };

export type AutoRunSwarmTaskResult = {
  runner_request_id: string;
  preflight_review: RunnerPreflightReviewSummary | null;
  execution_gate: RunnerExecutionGateSummary | null;
  dry_run: RunnerDryRunSummary | null;
  execution_lock: RunnerExecutionLockSummary | null;
  minimal_run: RunnerMinimalRunSummary | null;
  status: string;
  message: string | null;
};

export type AutoRunSwarmIdeaResponse = {
  plan: ApproveProjectPlanResponse;
  task_results: AutoRunSwarmTaskResult[];
  status: string;
};

export type ProjectPlanTaskInstanceSummary = {
  id: string;
  project_id: string;
  role: string;
  title: string;
  description: string | null;
  status: string;
  priority: string;
  assigned_agent_id: string | null;
  depends_on: string[];
  risk_level: string | null;
  created_at: string;
  updated_at: string;
};

export type ProjectPlanExecutionPreview = {
  draft: ProjectPlanDraftSummary;
  approval: ApprovalSummary;
  tasks: ProjectPlanTaskInstanceSummary[];
  runner_requests: RunnerRequestSummary[];
  side_effects: ProjectPlanSideEffects;
};

export type CreateRunnerPreflightReviewInput = {
  runner_request_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RunnerPreflightReviewSummary = {
  id: string;
  project_id: string;
  runner_request_id: string;
  task_id: string;
  approval_id: string;
  status: string;
  risk_level: string;
  operation_types: string[];
  affected_files: string[];
  requires_git_checkpoint: boolean;
  requires_second_confirm: boolean;
  blocked_reasons: string[];
  safety_summary: string;
  requested_by: string;
  created_at: string;
  updated_at: string;
};

export type CreateRunnerPreflightReviewResponse = {
  review: RunnerPreflightReviewSummary;
  approval: ApprovalSummary;
  side_effects: ProjectPlanSideEffects;
};

export type CreateRunnerExecutionGateInput = {
  preflight_review_id: string;
  second_confirm: boolean;
  confirm_text: string;
  requested_by?: string | null;
};

export type RevokeRunnerExecutionGateInput = {
  gate_id: string;
  second_confirm: boolean;
  confirm_text: string;
  revoked_reason?: string | null;
};

export type RunnerExecutionGateSummary = {
  id: string;
  project_id: string;
  runner_request_id: string;
  task_id: string;
  preflight_review_id: string;
  preflight_approval_id: string;
  status: string;
  risk_level: string;
  operation_types: string[];
  affected_files: string[];
  blocked_reasons: string[];
  can_execute: boolean;
  stage_boundary_locked: boolean;
  requires_git_checkpoint: boolean;
  requires_second_confirm: boolean;
  revoked_reason: string | null;
  requested_by: string;
  created_at: string;
  updated_at: string;
  revoked_at: string | null;
};

export type CreateRunnerExecutionGateResponse = {
  gate: RunnerExecutionGateSummary;
  side_effects: ProjectPlanSideEffects;
};

export type RevokeRunnerExecutionGateResponse = {
  gate: RunnerExecutionGateSummary;
  side_effects: ProjectPlanSideEffects;
};

export type UpdateProjectPlanTaskTemplateInput = {
  role: string;
  enabled: boolean;
};

export type SaveProjectPlanModelDraftInput = {
  idea: string;
  constraints?: string | null;
  audit_record_id: string;
  second_confirm: boolean;
  confirm_text: string;
};

// ---------------------------------------------------------------------------
// 阶段 35：模型目录
// ---------------------------------------------------------------------------

export type ModelCatalogEntry = {
  id: string;
  project_id: string;
  provider: string;
  model_id: string;
  display_name: string;
  purpose: string;
  enabled: boolean;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
};

export type UpdateModelEnabledInput = {
  model_record_id: string;
  enabled: boolean;
  second_confirm: boolean;
  confirm_text: string;
};
