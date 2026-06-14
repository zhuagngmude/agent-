import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// 跨端类型从 packages/shared 导入，在此重导出以保持现有引用路径兼容
export type {
  AgentSummary,
  ApprovalSummary,
  ApproveProjectPlanInput,
  ApproveProjectPlanResponse,
  CreateProjectPlanDraftInput,
  CreateProjectPlanDraftResponse,
  CreateTaskInput,
  DesktopHostOverviewData,
  DesktopHostOverviewState,
  ProjectSummary,
  ProjectPlanDraftSummary,
  RunnerRequestSummary,
  TaskStatus,
  TaskSummary,
  UpdateTaskStatusInput,
} from "@agent-swarm/shared";

import type {
  AgentSummary,
  ApprovalSummary,
  ApproveProjectPlanInput,
  ApproveProjectPlanResponse,
  CreateProjectPlanDraftInput,
  CreateProjectPlanDraftResponse,
  CreateTaskInput,
  DesktopHostOverviewData,
  DesktopHostOverviewState,
  ProjectSummary,
  ProjectPlanDraftSummary,
  RunnerRequestSummary,
  TaskSummary,
  UpdateTaskStatusInput,
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

export async function listProjectPlanDrafts(): Promise<ProjectPlanDraftSummary[]> {
  requireTauri();
  return invoke("list_project_plan_drafts");
}

export async function listRunnerRequests(): Promise<RunnerRequestSummary[]> {
  requireTauri();
  return invoke("list_runner_requests");
}

// ---------------------------------------------------------------------------
// Fallback 数据（浏览器预览模式）
// ---------------------------------------------------------------------------

const fallbackOverviewData: DesktopHostOverviewData = {
  project: {
    id: "browser_preview",
    name: "agent蜂群",
    status: "preview",
    phase: "浏览器预览",
  },
  agents: [
    {
      id: "agent_architect",
      project_id: "browser_preview",
      name: "架构师 Agent",
      role: "architect",
      status: "running",
      model: "gpt-high-reasoning",
      permissions: ["read_project", "plan_tasks", "review_architecture"],
      created_at: "",
      updated_at: "",
    },
    {
      id: "agent_frontend",
      project_id: "browser_preview",
      name: "前端 Agent",
      role: "frontend",
      status: "running",
      model: "claude-ui",
      permissions: ["read_project", "write_frontend_patch"],
      created_at: "",
      updated_at: "",
    },
  ],
  tasks: [
    {
      id: "task_frontend_mock_data",
      project_id: "browser_preview",
      title: "抽出前端 mock 数据模型",
      description: "浏览器预览 fallback 数据。",
      status: "completed",
      priority: "high",
      assigned_agent_id: "agent_frontend",
      depends_on: [],
      risk_level: "low",
      created_at: "",
      updated_at: "",
    },
    {
      id: "task_runner_approval_page",
      project_id: "browser_preview",
      title: "打磨 Runner 审批确认页",
      description: "浏览器预览 fallback 数据。",
      status: "running",
      priority: "high",
      assigned_agent_id: "agent_frontend",
      depends_on: ["task_frontend_mock_data"],
      risk_level: "high",
      created_at: "",
      updated_at: "",
    },
  ],
  approvals: [
    {
      id: "approval_runner_permissions",
      project_id: "browser_preview",
      task_id: null,
      request_agent_id: "agent_architect",
      target_service: "runner",
      operation_types: ["file_write", "git_checkpoint", "audit_log_update"],
      status: "pending",
      risk_level: "high",
      reason: "浏览器预览 fallback 数据。",
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
          setState({ status: "error", message: String(error), ...fallbackOverviewData });
        }
      });

    return () => {
      mounted = false;
    };
  }, [refreshKey]);

  return { ...state, refresh };
}
