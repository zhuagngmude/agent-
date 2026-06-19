import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// 跨端类型从 packages/shared 导入，在此重导出以保持现有引用路径兼容
export type {
  AgentSummary,
  AgentRunSummary,
  ApprovalSummary,
  ApproveProjectPlanInput,
  ApproveProjectPlanResponse,
  AutoGenerateProjectPlanTasksInput,
  AutoRunSwarmIdeaInput,
  AutoRunSwarmIdeaResponse,
  AutoRunSwarmTaskResult,
  ContinueSwarmTasksInput,
  ContinueSwarmTasksResponse,
  CreateProjectPlanDraftInput,
  CreateProjectPlanDraftResponse,
  DeleteProjectPlanDraftInput,
  DeleteProjectPlanDraftResponse,
  DeleteTasksInput,
  DeleteTasksResponse,
  CreateTaskInput,
  OpenTaskOutputFolderInput,
  OpenTaskOutputFolderResponse,
  DesktopHostOverviewData,
  DesktopHostOverviewState,
  ProjectSummary,
  ProjectPlanDraftSummary,
  ProjectPlanModelDraftInput,
  ProjectPlanModelDraftResponse,
  CreateRunnerExecutionGateInput,
  CreateRunnerExecutionGateResponse,
  CreateRunnerPreflightReviewInput,
  CreateRunnerPreflightReviewResponse,
  ProjectPlanExecutionPreview,
  CreateRunnerDryRunInput, CreateRunnerDryRunResponse,
  CreateRunnerExecutionLockInput, CreateRunnerExecutionLockResponse,
  CreateRunnerMinimalRunInput, CreateRunnerMinimalRunResponse,
  RunnerCommandResultSummary, RunnerMinimalRunSummary,
  RunnerExecutionLockSummary,
  RevokeRunnerExecutionLockInput, RevokeRunnerExecutionLockResponse,
  PlannedFileChangeSummary, RunnerDryRunSummary,
  RevokeRunnerDryRunInput, RevokeRunnerDryRunResponse,
  RevokeRunnerExecutionGateInput,
  RevokeRunnerExecutionGateResponse,
  RunnerExecutionGateSummary,
  ProjectPlanTaskInstanceSummary,
  ProjectPlanTaskTemplateSummary,
  RunnerPreflightReviewSummary,
  SaveProjectPlanModelDraftInput,
  UpdateProjectPlanTaskTemplateInput,
  ModelCatalogEntry,
  RuntimeModelProviderStatus,
  TestRuntimeModelProviderResponse,
  UpdateModelEnabledInput,
  UpdateRuntimeModelProviderInput,
  RunnerRequestSummary,
  TaskStatus,
  TaskSummary,
  UpdateTaskStatusInput,
  CreateIdeaGuidanceQuestionsInput,
  CreateIdeaGuidanceQuestionsResponse,
  GenerateProjectSeedInput,
  GenerateProjectSeedResponse,
  IdeaGuidanceQuestion,
  IdeaGuidanceSession,
  ProjectSeed,
  SaveGuidanceAnswersInput,
  ClassifyProjectIntakeInput,
  ClassifyProjectIntakeResponse,
  ProjectIntakeSession,
  RuntimeEventSummary,
} from "@agent-swarm/shared";
import { userErrorLabel } from "./userError";

import type {
  AgentSummary,
  AgentRunSummary,
  ApprovalSummary,
  ApproveProjectPlanInput,
  ApproveProjectPlanResponse,
  AutoGenerateProjectPlanTasksInput,
  AutoRunSwarmIdeaInput,
  AutoRunSwarmIdeaResponse,
  AutoRunSwarmTaskResult,
  ContinueSwarmTasksInput,
  ContinueSwarmTasksResponse,
  CreateProjectPlanDraftInput,
  CreateProjectPlanDraftResponse,
  DeleteProjectPlanDraftInput,
  DeleteProjectPlanDraftResponse,
  DeleteTasksInput,
  DeleteTasksResponse,
  CreateTaskInput,
  OpenTaskOutputFolderInput,
  OpenTaskOutputFolderResponse,
  DesktopHostOverviewData,
  DesktopHostOverviewState,
  ProjectSummary,
  ProjectPlanDraftSummary,
  ProjectPlanModelDraftInput,
  ProjectPlanModelDraftResponse,
  CreateRunnerExecutionGateInput,
  CreateRunnerExecutionGateResponse,
  CreateRunnerPreflightReviewInput,
  CreateRunnerPreflightReviewResponse,
  ProjectPlanExecutionPreview,
  CreateRunnerDryRunInput, CreateRunnerDryRunResponse,
  CreateRunnerExecutionLockInput, CreateRunnerExecutionLockResponse,
  CreateRunnerMinimalRunInput, CreateRunnerMinimalRunResponse,
  RunnerCommandResultSummary, RunnerMinimalRunSummary,
  RunnerExecutionLockSummary,
  RevokeRunnerExecutionLockInput, RevokeRunnerExecutionLockResponse,
  PlannedFileChangeSummary, RunnerDryRunSummary,
  RevokeRunnerDryRunInput, RevokeRunnerDryRunResponse,
  RevokeRunnerExecutionGateInput,
  RevokeRunnerExecutionGateResponse,
  RunnerExecutionGateSummary,
  ProjectPlanTaskInstanceSummary,
  ProjectPlanTaskTemplateSummary,
  RunnerPreflightReviewSummary,
  SaveProjectPlanModelDraftInput,
  UpdateProjectPlanTaskTemplateInput,
  ModelCatalogEntry,
  RuntimeModelProviderStatus,
  TestRuntimeModelProviderResponse,
  UpdateModelEnabledInput,
  UpdateRuntimeModelProviderInput,
  RunnerRequestSummary,
  TaskSummary,
  UpdateTaskStatusInput,
  CreateIdeaGuidanceQuestionsInput,
  CreateIdeaGuidanceQuestionsResponse,
  GenerateProjectSeedInput,
  GenerateProjectSeedResponse,
  IdeaGuidanceQuestion,
  IdeaGuidanceSession,
  ProjectSeed,
  SaveGuidanceAnswersInput,
  ClassifyProjectIntakeInput,
  ClassifyProjectIntakeResponse,
  ProjectIntakeSession,
  RuntimeEventSummary,
} from "@agent-swarm/shared";

// ---------------------------------------------------------------------------
// Tauri 环境检测
// ---------------------------------------------------------------------------

export function isTauriHost(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function requireTauri(): void {
  if (!isTauriHost()) {
    throw new Error("当前运行在浏览器预览模式，写入操作不可用。请启动 Tauri 桌面宿主。");
  }
}

// ---------------------------------------------------------------------------
// 写入 commands 封装
// ---------------------------------------------------------------------------

export async function createTask(input: CreateTaskInput): Promise<{ task: TaskSummary }> {
  requireTauri();
  return invoke("create_task", { input });
}

export async function updateTaskStatus(input: UpdateTaskStatusInput): Promise<{ task: TaskSummary }> {
  requireTauri();
  return invoke("update_task_status", { input });
}

export async function deleteTasks(input: DeleteTasksInput): Promise<DeleteTasksResponse> {
  requireTauri();
  return invoke("delete_tasks", { input });
}

export async function openTaskOutputFolder(
  input: OpenTaskOutputFolderInput,
): Promise<OpenTaskOutputFolderResponse> {
  requireTauri();
  return invoke("open_task_output_folder", { input });
}

export async function approveApproval(id: string): Promise<{ approval: ApprovalSummary }> {
  requireTauri();
  return invoke("approve_approval", { input: { id } });
}

export async function rejectApproval(
  id: string,
  rejectReason?: string | null,
): Promise<{ approval: ApprovalSummary }> {
  requireTauri();
  return invoke("reject_approval", { input: { id, reject_reason: rejectReason ?? null } });
}

export async function patchOnlyApproval(id: string): Promise<{ approval: ApprovalSummary }> {
  requireTauri();
  return invoke("patch_only_approval", { input: { id } });
}

export async function createProjectPlanDraft(
  input: CreateProjectPlanDraftInput,
): Promise<CreateProjectPlanDraftResponse> {
  requireTauri();
  return invoke("create_project_plan_draft", { input });
}

export async function approveProjectPlan(
  input: ApproveProjectPlanInput,
): Promise<ApproveProjectPlanResponse> {
  requireTauri();
  return invoke("approve_project_plan", { input });
}

export async function autoGenerateProjectPlanTasks(
  input: AutoGenerateProjectPlanTasksInput,
): Promise<ApproveProjectPlanResponse> {
  requireTauri();
  return invoke("auto_generate_project_plan_tasks", { input });
}

export async function autoRunSwarmIdea(
  input: AutoRunSwarmIdeaInput,
): Promise<AutoRunSwarmIdeaResponse> {
  requireTauri();
  return invoke("auto_run_swarm_idea", { input });
}

export async function continueSwarmTasks(
  input: ContinueSwarmTasksInput,
): Promise<ContinueSwarmTasksResponse> {
  requireTauri();
  return invoke("continue_swarm_tasks", { input });
}

export async function listProjectPlanDrafts(): Promise<ProjectPlanDraftSummary[]> {
  requireTauri();
  return invoke("list_project_plan_drafts");
}

export async function deleteProjectPlanDraft(
  input: DeleteProjectPlanDraftInput,
): Promise<DeleteProjectPlanDraftResponse> {
  requireTauri();
  return invoke("delete_project_plan_draft", { input });
}

export async function listRunnerRequests(): Promise<RunnerRequestSummary[]> {
  requireTauri();
  return invoke("list_runner_requests");
}

export async function listAgentRuns(): Promise<AgentRunSummary[]> {
  requireTauri();
  return invoke("list_agent_runs");
}

export async function listRuntimeEvents(): Promise<RuntimeEventSummary[]> {
  requireTauri();
  return invoke("list_runtime_events");
}

export type TaskAgentInfo = {
  task_id: string;
  project_agent_id: string | null;
  agent_name: string | null;
  agent_role: string | null;
  executor_key: string | null;
  model_id: string | null;
  module_scope: string | null;
  agent_status: string | null;
};

export type AssignProjectAgentsToTaskInput = {
  task_id: string;
  project_agent_id: string;
};

export type AssignProjectAgentsToTaskResponse = {
  task: TaskSummary;
  agent: TaskAgentInfo;
};

export async function getTaskAgentInfo(taskId: string): Promise<TaskAgentInfo | null> {
  requireTauri();
  return invoke("get_task_agent_info", { taskId });
}

export async function assignProjectAgentsToTask(
  input: AssignProjectAgentsToTaskInput,
): Promise<AssignProjectAgentsToTaskResponse> {
  requireTauri();
  return invoke("assign_project_agents_to_task", { input });
}

export async function requestProjectPlanModelDraft(
  input: ProjectPlanModelDraftInput,
): Promise<ProjectPlanModelDraftResponse> {
  requireTauri();
  return invoke("request_project_plan_model_draft", { input });
}

export async function saveProjectPlanModelDraft(
  input: SaveProjectPlanModelDraftInput,
): Promise<CreateProjectPlanDraftResponse> {
  requireTauri();
  return invoke("save_project_plan_model_draft", { input });
}

export async function listProjectPlanTaskTemplates(): Promise<
  ProjectPlanTaskTemplateSummary[]
> {
  requireTauri();
  return invoke("list_project_plan_task_templates");
}

export async function updateProjectPlanTaskTemplate(
  input: UpdateProjectPlanTaskTemplateInput,
): Promise<ProjectPlanTaskTemplateSummary[]> {
  requireTauri();
  return invoke("update_project_plan_task_template", { input });
}

export async function getProjectPlanExecutionPreview(
  approvalId: string,
): Promise<ProjectPlanExecutionPreview> {
  requireTauri();
  return invoke("get_project_plan_execution_preview", { approvalId });
}

export async function createRunnerPreflightReview(
  input: CreateRunnerPreflightReviewInput,
): Promise<CreateRunnerPreflightReviewResponse> {
  requireTauri();
  return invoke("create_runner_preflight_review", { input });
}

export async function listRunnerPreflightReviews(): Promise<
  RunnerPreflightReviewSummary[]
> {
  requireTauri();
  return invoke("list_runner_preflight_reviews");
}

export async function createRunnerExecutionGate(
  input: CreateRunnerExecutionGateInput,
): Promise<CreateRunnerExecutionGateResponse> {
  requireTauri();
  return invoke("create_runner_execution_gate", { input });
}

export async function listRunnerExecutionGates(): Promise<
  RunnerExecutionGateSummary[]
> {
  requireTauri();
  return invoke("list_runner_execution_gates");
}

export async function revokeRunnerExecutionGate(
  input: RevokeRunnerExecutionGateInput,
): Promise<RevokeRunnerExecutionGateResponse> {
  requireTauri();
  return invoke("revoke_runner_execution_gate", { input });
}

export async function createRunnerDryRun(
  input: CreateRunnerDryRunInput,
): Promise<CreateRunnerDryRunResponse> {
  requireTauri();
  return invoke("create_runner_dry_run", { input });
}
export async function listRunnerDryRuns(): Promise<RunnerDryRunSummary[]> {
  requireTauri();
  return invoke("list_runner_dry_runs");
}
export async function revokeRunnerDryRun(
  input: RevokeRunnerDryRunInput,
): Promise<RevokeRunnerDryRunResponse> {
  requireTauri();
  return invoke("revoke_runner_dry_run", { input });
}

export async function createRunnerExecutionLock(
  input: CreateRunnerExecutionLockInput,
): Promise<CreateRunnerExecutionLockResponse> {
  requireTauri();
  return invoke("create_runner_execution_lock", { input });
}
export async function listRunnerExecutionLocks(): Promise<RunnerExecutionLockSummary[]> {
  requireTauri();
  return invoke("list_runner_execution_locks");
}
export async function revokeRunnerExecutionLock(
  input: RevokeRunnerExecutionLockInput,
): Promise<RevokeRunnerExecutionLockResponse> {
  requireTauri();
  return invoke("revoke_runner_execution_lock", { input });
}

export async function createRunnerMinimalRun(
  input: CreateRunnerMinimalRunInput,
): Promise<CreateRunnerMinimalRunResponse> {
  requireTauri();
  return invoke("create_runner_minimal_run", { input });
}
export async function listRunnerMinimalRuns(): Promise<RunnerMinimalRunSummary[]> {
  requireTauri();
  return invoke("list_runner_minimal_runs");
}

// ---------------------------------------------------------------------------
// 阶段 35：模型目录
// ---------------------------------------------------------------------------

export async function listProjectPlanModels(): Promise<ModelCatalogEntry[]> {
  requireTauri();
  return invoke("list_project_plan_models");
}

export async function updateProjectPlanModelEnabled(
  input: UpdateModelEnabledInput,
): Promise<ModelCatalogEntry[]> {
  requireTauri();
  return invoke("update_project_plan_model_enabled", { input });
}

export async function getRuntimeModelProviderStatus(): Promise<RuntimeModelProviderStatus> {
  requireTauri();
  return invoke("get_runtime_model_provider_status");
}

export async function updateRuntimeModelProvider(
  input: UpdateRuntimeModelProviderInput,
): Promise<RuntimeModelProviderStatus> {
  requireTauri();
  return invoke("update_runtime_model_provider", { input });
}

export async function testRuntimeModelProvider(): Promise<TestRuntimeModelProviderResponse> {
  requireTauri();
  return invoke("test_runtime_model_provider");
}

export type ChatWithControllerInput = {
  message: string;
};

export type ChatWithControllerResponse = {
  reply: string;
  model_id: string;
};

export async function chatWithController(
  input: ChatWithControllerInput,
): Promise<ChatWithControllerResponse> {
  requireTauri();
  return invoke("chat_with_controller", { input });
}

// ---------------------------------------------------------------------------
// P0：AI 员工 / 执行器 / 模型 / Skill 配置
// ---------------------------------------------------------------------------

export type ExecutorConfigSummary = {
  id: string;
  key: string;
  label: string;
  kind: "model_gateway" | "external_executor" | "local_tool";
  provider: string | null;
  base_url_status: string;
  executable_path: string | null;
  status: "active" | "disabled" | "error";
  created_at: string;
  updated_at: string;
};

export type ExecutorModelSummary = {
  id: string;
  project_id: string;
  executor_key: string;
  provider: string;
  model_id: string;
  display_name: string;
  purpose: string;
  enabled: boolean;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
};

export type AgentTemplateSummary = {
  id: string;
  name: string;
  role: string;
  category: "core" | "expert";
  specialty: string | null;
  stack: string | null;
  module_scope: string;
  allowed_task_types: string[];
  allowed_paths: string[];
  forbidden_actions: string[];
  default_executor_key: string;
  default_model_id: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type ProjectAgentSummary = {
  id: string;
  project_id: string;
  agent_template_id: string;
  name: string;
  role: string;
  source: "core" | "recommended" | "manual";
  executor_key: string;
  model_id: string | null;
  module_scope: string;
  status: "active" | "idle" | "disabled" | "removed";
  joined_at: string;
  removed_at: string | null;
  created_at: string;
  updated_at: string;
};

export type ExecutorSkillSummary = {
  id: string;
  executor_key: string;
  agent_template_id: string | null;
  skill_name: string;
  skill_scope: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type AgentBoundaryCheckSummary = {
  id: string;
  project_id: string;
  task_id: string | null;
  agent_id: string;
  requested_action: string;
  task_type: string | null;
  module_scope: string;
  target_path: string | null;
  decision: "allowed" | "denied" | "needs_approval";
  reason: string;
  created_at: string;
};

export type UpsertProjectAgentInput = {
  project_id?: string | null;
  agent_template_id: string;
  name: string;
  role: string;
  source: "core" | "recommended" | "manual";
  executor_key: string;
  model_id?: string | null;
  module_scope: string;
  status: "active" | "idle" | "disabled" | "removed";
};

export type UpsertExecutorModelInput = {
  project_id?: string | null;
  executor_key: string;
  provider: "openai_compat";
  model_id: string;
  display_name: string;
  purpose: string;
  enabled: boolean;
};

export type UpsertExecutorSkillInput = {
  executor_key: string;
  agent_template_id?: string | null;
  skill_name: string;
  skill_scope: string;
  enabled: boolean;
};

export async function listExecutorConfigs(): Promise<ExecutorConfigSummary[]> {
  requireTauri();
  return invoke("list_executor_configs");
}

export async function listExecutorModels(input: {
  project_id?: string | null;
  executor_key?: string | null;
  purpose?: string | null;
} = {}): Promise<ExecutorModelSummary[]> {
  requireTauri();
  return invoke("list_executor_models", { input });
}

export async function upsertExecutorModel(input: UpsertExecutorModelInput): Promise<ExecutorModelSummary> {
  requireTauri();
  return invoke("upsert_executor_model", { input });
}

export async function deleteExecutorModel(modelRecordId: string): Promise<void> {
  requireTauri();
  return invoke("delete_executor_model", { input: { model_record_id: modelRecordId } });
}

export async function listAgentTemplates(): Promise<AgentTemplateSummary[]> {
  requireTauri();
  return invoke("list_agent_templates");
}

export async function listProjectAgents(): Promise<ProjectAgentSummary[]> {
  requireTauri();
  return invoke("list_project_agents");
}

export async function upsertProjectAgent(input: UpsertProjectAgentInput): Promise<ProjectAgentSummary> {
  requireTauri();
  return invoke("upsert_project_agent", { input });
}

export async function removeProjectAgent(projectAgentId: string): Promise<void> {
  requireTauri();
  return invoke("remove_project_agent", { input: { project_agent_id: projectAgentId } });
}

export async function listExecutorSkills(): Promise<ExecutorSkillSummary[]> {
  requireTauri();
  return invoke("list_executor_skills");
}

export async function upsertExecutorSkill(input: UpsertExecutorSkillInput): Promise<ExecutorSkillSummary> {
  requireTauri();
  return invoke("upsert_executor_skill", { input });
}

export async function deleteExecutorSkill(skillId: string): Promise<void> {
  requireTauri();
  return invoke("delete_executor_skill", { input: { skill_id: skillId } });
}

export async function listAgentBoundaryChecks(input: {
  project_id?: string | null;
  limit?: number | null;
} = {}): Promise<AgentBoundaryCheckSummary[]> {
  requireTauri();
  return invoke("list_agent_boundary_checks", { input });
}

// ---------------------------------------------------------------------------
// CP3：总控确定性分派规则
// ---------------------------------------------------------------------------

export type RecommendProjectAgentsInput = {
  project_id?: string | null;
  project_type: string;
  tech_stack: string[];
  risk_level: "low" | "medium" | "high";
  phase: string;
};

export type RecommendProjectAgentsOutput = {
  project_id: string;
  recommended_core_agents: ProjectAgentSummary[];
  recommended_expert_agents: ProjectAgentSummary[];
  reason: string;
  warnings: string[];
};

export async function recommendProjectAgents(
  input: RecommendProjectAgentsInput,
): Promise<RecommendProjectAgentsOutput> {
  requireTauri();
  return invoke("recommend_project_agents", { input });
}

// ---------------------------------------------------------------------------
// CP4：Runner 执行前边界强校验
// ---------------------------------------------------------------------------

export type CheckAgentBoundaryInput = {
  agent_id: string;
  task_id?: string | null;
  task_type: string;
  module_scope: string;
  target_path?: string | null;
  forbidden_actions: string[];
  requested_action: string;
};

export type CheckAgentBoundaryOutput = {
  check_id: string;
  decision: "allowed" | "denied" | "needs_approval";
  reason: string;
};

export async function checkAgentBoundary(
  input: CheckAgentBoundaryInput,
): Promise<CheckAgentBoundaryOutput> {
  requireTauri();
  return invoke("check_agent_boundary", { input });
}

// ---------------------------------------------------------------------------
// 阶段 37：想法引导官
// ---------------------------------------------------------------------------

export async function createIdeaGuidanceQuestions(
  input: CreateIdeaGuidanceQuestionsInput,
): Promise<CreateIdeaGuidanceQuestionsResponse> {
  requireTauri();
  return invoke("create_idea_guidance_questions", { input });
}

export async function generateProjectSeed(
  input: GenerateProjectSeedInput,
): Promise<GenerateProjectSeedResponse> {
  requireTauri();
  return invoke("generate_project_seed", { input });
}

export async function saveGuidanceAnswers(
  input: SaveGuidanceAnswersInput,
): Promise<IdeaGuidanceSession> {
  requireTauri();
  return invoke("save_guidance_answers", { input });
}

export async function listProjectSeeds(): Promise<ProjectSeed[]> {
  requireTauri();
  return invoke("list_project_seeds");
}

// ---------------------------------------------------------------------------
// 阶段 38：项目类型分流与通用想法入口
// ---------------------------------------------------------------------------

export async function classifyProjectIntake(
  input: ClassifyProjectIntakeInput,
): Promise<ClassifyProjectIntakeResponse> {
  requireTauri();
  return invoke("classify_project_intake", { input });
}

export async function listProjectIntakes(): Promise<ProjectIntakeSession[]> {
  requireTauri();
  return invoke("list_project_intakes");
}

// ---------------------------------------------------------------------------
// Fallback 数据（浏览器预览模式）
// ---------------------------------------------------------------------------

const fallbackOverviewData: DesktopHostOverviewData = {
  project: {
    id: "browser_preview",
    name: "示例项目：agent蜂群预览",
    status: "preview",
    phase: "浏览器预览 · 只读示例数据",
  },
  agents: [
    {
      id: "agent_product_planning",
      project_id: "browser_preview",
      name: "示例 Agent：产品规划",
      role: "architect",
      status: "idle",
      model: "从受控模型目录选择",
      permissions: ["read_project"],
      created_at: "",
      updated_at: "",
    },
    {
      id: "agent_frontend_impl",
      project_id: "browser_preview",
      name: "示例 Agent：前端实现",
      role: "frontend",
      status: "idle",
      model: "从受控模型目录选择",
      permissions: ["read_project"],
      created_at: "",
      updated_at: "",
    },
  ],
  tasks: [
    {
      id: "task_organize_ideas",
      project_id: "browser_preview",
      title: "示例任务：整理项目想法",
      description: "这是浏览器预览示例数据，不表示真实任务。桌面模式下从本地 SQLite 读取真实任务。",
      status: "queued",
      priority: "medium",
      assigned_agent_id: "agent_product_planning",
      depends_on: [],
      risk_level: "low",
      created_at: "",
      updated_at: "",
    },
    {
      id: "task_frontend_preview",
      project_id: "browser_preview",
      title: "示例任务：生成前端预览",
      description: "这是浏览器预览示例数据，不表示真实任务。桌面模式下从本地 SQLite 读取真实任务。",
      status: "queued",
      priority: "medium",
      assigned_agent_id: "agent_frontend_impl",
      depends_on: ["task_organize_ideas"],
      risk_level: "medium",
      created_at: "",
      updated_at: "",
    },
  ],
  approvals: [
    {
      id: "approval_preview_example",
      project_id: "browser_preview",
      task_id: null,
      request_agent_id: "agent_product_planning",
      target_service: "runner",
      operation_types: ["read_only"],
      status: "pending",
      risk_level: "medium",
      reason: "这是浏览器预览示例审批，不表示真实审批记录。桌面模式下从本地 SQLite 读取真实审批。",
      reject_reason: null,
      approved_at: null,
      rejected_at: null,
      created_at: "",
      updated_at: "",
    },
  ],
};

// ---------------------------------------------------------------------------
// 数据读取 Hook
// ---------------------------------------------------------------------------

export function useDesktopHostOverview(): DesktopHostOverviewState & { refresh: () => void } {
  const [refreshKey, setRefreshKey] = useState(0);
  const refresh = useCallback(() => setRefreshKey((key) => key + 1), []);

  const [state, setState] = useState<DesktopHostOverviewState>(() => {
    if (!isTauriHost()) {
      return { status: "browser", ...fallbackOverviewData };
    }

    return { status: "loading" };
  });

  useEffect(() => {
    if (!isTauriHost()) {
      return;
    }

    let mounted = true;

    Promise.all([
      invoke<ProjectSummary>("get_project"),
      invoke<AgentSummary[]>("list_agents"),
      invoke<TaskSummary[]>("list_tasks"),
      invoke<ApprovalSummary[]>("list_approvals"),
    ])
      .then(([project, agents, tasks, approvals]) => {
        if (mounted) {
          setState({ status: "connected", project, agents, tasks, approvals });
        }
      })
      .catch((error: unknown) => {
        if (mounted) {
          setState({
            status: "error",
            message: userErrorLabel(error, "桌面宿主连接失败，请检查应用是否已启动"),
            ...fallbackOverviewData,
          });
        }
      });

    return () => {
      mounted = false;
    };
  }, [refreshKey]);

  return { ...state, refresh };
}
