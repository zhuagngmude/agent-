import { useEffect, useState } from "react";
import { Spin } from "antd";
import {
  AlertTriangle,
} from "lucide-react";

import type {
  AgentSummary,
  ApprovalSummary,
  ProjectSummary,
  TaskSummary,
  ProjectPlanDraftSummary,
  ProjectPlanExecutionPreview,
  RunnerPreflightReviewSummary,
  RunnerExecutionGateSummary,
  RunnerDryRunSummary,
  RunnerExecutionLockSummary,
  RunnerMinimalRunSummary,
  RunnerRequestSummary,
  ProjectSeed,
} from "@agent-swarm/shared";

import type { WorkflowNodeData } from "../components/WorkflowNodeCard";
import { WorkflowStageRail } from "../components/WorkflowStageRail";
import { WorkflowCanvas } from "../components/WorkflowCanvas";
import { AutomationRulesPanel } from "../components/AutomationRulesPanel";
import {
  autoGenerateProjectPlanTasks,
  autoRunSwarmIdea,
  isTauriHost,
  listProjectPlanDrafts,
  listRunnerPreflightReviews,
  listRunnerExecutionGates,
  listRunnerDryRuns,
  listRunnerExecutionLocks,
  listRunnerMinimalRuns,
  listRunnerRequests,
  listProjectSeeds,
  getProjectPlanExecutionPreview,
} from "../utils/desktopHost";
import { userErrorLabel } from "../utils/userError";
import { agentNameLabel, roleLabel } from "../utils/labels";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

type WorkflowPageProps = {
  project: ProjectSummary;
  agents: AgentSummary[];
  tasks: TaskSummary[];
  approvals: ApprovalSummary[];
  connectionStatus: "loading" | "browser" | "connected" | "error";
  message?: string;
  onRefresh?: () => void;
};

// ---------------------------------------------------------------------------
// 阶段定义
// ---------------------------------------------------------------------------

const STAGE_DEFS: { key: string; label: string }[] = [
  { key: "idea_clarify", label: "想法澄清" },
  { key: "project_seed", label: "项目种子" },
  { key: "req_breakdown", label: "需求拆解" },
  { key: "plan_draft", label: "计划草案" },
  { key: "agent_assign", label: "智能体分配" },
  { key: "preflight_review", label: "预检" },
  { key: "execution_gate", label: "放行" },
  { key: "dry_run", label: "试跑" },
  { key: "human_confirm", label: "人工确认" },
  { key: "minimal_run", label: "真干" },
  { key: "review", label: "验收复盘" },
  { key: "next_round", label: "下一轮计划" },
];

// ---------------------------------------------------------------------------
// 数据 → 工作流节点 映射逻辑
// ---------------------------------------------------------------------------

interface WorkflowDataSource {
  tasks: TaskSummary[];
  approvals: ApprovalSummary[];
  agents: AgentSummary[];
  drafts: ProjectPlanDraftSummary[];
  preflights: RunnerPreflightReviewSummary[];
  gates: RunnerExecutionGateSummary[];
  dryRuns: RunnerDryRunSummary[];
  locks: RunnerExecutionLockSummary[];
  minimalRuns: RunnerMinimalRunSummary[];
  runnerRequests: RunnerRequestSummary[];
  seeds: ProjectSeed[];
  executionPreviews: ProjectPlanExecutionPreview[];
}

function isNonEmptyString(value: string | null | undefined): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function latestExecutionPreview(previews: ProjectPlanExecutionPreview[]): ProjectPlanExecutionPreview | null {
  return [...previews].sort((a, b) => b.draft.updated_at.localeCompare(a.draft.updated_at))[0] ?? null;
}

function compactList(values: string[], maxItems = 5): string[] {
  const clean = values.map((value) => value.trim()).filter(Boolean);
  if (clean.length <= maxItems) return clean;
  return [...clean.slice(0, maxItems), `还有 ${clean.length - maxItems} 项`];
}

function buildWorkflowNodes(data: WorkflowDataSource): WorkflowNodeData[] {
  const { tasks, approvals, agents, drafts, preflights, gates, dryRuns, locks, minimalRuns, runnerRequests, seeds, executionPreviews } = data;

  // 数据存在性判定
  const hasTasks = tasks.length > 0;
  const activePreview = latestExecutionPreview(executionPreviews);
  const roleTasks = activePreview?.tasks ?? [];
  const generatedRunnerRequests = activePreview?.runner_requests ?? [];
  const hasRoleTasks = roleTasks.length > 0;
  const hasSeeds = seeds.length > 0;
  const hasDrafts = drafts.length > 0;
  const hasRunnerRequests = runnerRequests.length > 0;
  const hasPreflights = preflights.length > 0;
  const hasGates = gates.length > 0;
  const hasDryRuns = dryRuns.length > 0;
  const hasLocks = locks.length > 0;
  const hasMinimalRuns = minimalRuns.length > 0;

  const hasPendingApproval = approvals.some((a) => a.status === "pending");
  const hasApproved = approvals.some((a) => a.status === "approved");

  // 找到活跃阶段：从后往前找最后一个有数据的阶段
  const activeIndex = (() => {
    if (hasMinimalRuns) return 9; // minimal_run
    if (hasLocks) return 9;       // 锁定但未执行 → human_confirm
    if (hasDryRuns) return 8;     // human_confirm
    if (hasGates) return 7;       // dry_run
    if (hasPreflights) return 6;  // execution_gate
    if (hasRunnerRequests || generatedRunnerRequests.length > 0) return 5; // preflight_review
    if (hasRoleTasks) return 4;      // agent_assign
    if (hasDrafts) return 4;      // agent_assign
    if (hasSeeds) return 2;       // req_breakdown
    if (hasTasks) return 1;       // project_seed
    return 0;                     // idea_clarify
  })();

  const getAgentName = (agentId?: string | null): string | null => {
    if (!agentId) return null;
    const agent = agents.find((a) => a.id === agentId);
    return agent ? agentNameLabel(agent.name) : agentNameLabel(agentId);
  };

  const assignedAgentNames = agents
    .filter((a) => tasks.some((t) => t.assigned_agent_id === a.id))
    .map((a) => agentNameLabel(a.name));
  const roleTaskAgentNames = Array.from(
    new Set(
      roleTasks
        .map((task) => task.assigned_agent_id)
        .filter(isNonEmptyString)
        .map((id) => agentNameLabel(id)),
    ),
  );

  return STAGE_DEFS.map((def, idx) => {
    const base: WorkflowNodeData = {
      stageKey: def.key,
      stageLabel: def.label,
      stageIndex: idx,
      status: "pending",
      statusLabel: "等待数据",
      agentName: null,
      assignedAgents: assignedAgentsForStage(def.key),
      canAutoAdvance: false,
      requiresApproval: false,
      riskLevel: "none",
      artifacts: [],
      sourceData: "无",
    };

    switch (def.key) {
      // 0 — 想法澄清
      case "idea_clarify": {
        const ideaTasks = tasks.filter((t) =>
          t.title.includes("想法") || t.title.includes("idea") || t.description?.includes("想法")
        );
        if (hasTasks && ideaTasks.length > 0) {
          return {
            ...base,
            status: "completed",
            statusLabel: "已完成梳理",
            agentName: getAgentName(ideaTasks[0]?.assigned_agent_id),
            artifacts: ideaTasks.map((t) => t.title),
            sourceData: `任务记录 × ${ideaTasks.length}`,
          };
        }
        if (hasTasks) {
          return {
            ...base,
            status: "in_progress",
            statusLabel: "已有任务数据",
            agentName: getAgentName(tasks[0]?.assigned_agent_id),
            artifacts: ["已有任务"],
            sourceData: `任务记录 × ${tasks.length}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "completed" : "pending", statusLabel: idx < activeIndex ? "已完成" : "等待开始" };
      }

      // 1 — 项目种子
      case "project_seed": {
        if (hasSeeds) {
          return {
            ...base,
            status: "completed",
            statusLabel: "已生成种子",
            artifacts: seeds.map((s) => s.product_goal ?? s.mvp_scope ?? "项目种子"),
            sourceData: `项目种子 × ${seeds.length}`,
          };
        }
        if (hasTasks) {
          return {
            ...base,
            status: "in_progress",
            statusLabel: "可从任务推导",
            artifacts: ["待明确种子形态"],
            sourceData: "从已有任务推导",
          };
        }
        return { ...base, status: idx < activeIndex ? "completed" : "pending", statusLabel: idx < activeIndex ? "已完成" : "等待数据" };
      }

      // 2 — 需求拆解
      case "req_breakdown": {
        if (hasDrafts || hasSeeds) {
          return {
            ...base,
            status: "completed",
            statusLabel: "已有计划框架",
            artifacts: drafts.map((d) => d.idea).filter(Boolean),
            sourceData: hasDrafts ? `计划草案 × ${drafts.length}` : "从项目种子推导",
          };
        }
        if (hasTasks) {
          return {
            ...base,
            status: "in_progress",
            statusLabel: "可从任务推导",
            artifacts: tasks.map((t) => t.title),
            sourceData: `任务记录 × ${tasks.length}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: idx < activeIndex ? "已跳过" : "等待数据" };
      }

      // 3 — 计划草案
      case "plan_draft": {
        if (hasDrafts) {
          const instantiatedDraft = drafts.find((d) => d.status === "instantiated");
          return {
            ...base,
            status: instantiatedDraft || hasRoleTasks ? "completed" : "in_progress",
            statusLabel: instantiatedDraft || hasRoleTasks ? "计划已生成角色任务" : "草案待审批",
            agentName: "项目计划官",
            requiresApproval: true,
            riskLevel: "low",
            artifacts: compactList(drafts.map((d) => d.summary ?? d.idea), 4),
            sourceData: `计划草案 × ${drafts.length}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未创建" };
      }

      // 4 — 智能体分配
      case "agent_assign": {
        if (hasRoleTasks) {
          return {
            ...base,
            status: "completed",
            statusLabel: `${roleTasks.length} 个角色任务已生成`,
            agentName: roleTaskAgentNames.length > 0 ? roleTaskAgentNames.join("、") : "总控智能体",
            artifacts: compactList(roleTasks.map((task) => `${roleLabel(task.role)}：${task.title}`), 6),
            sourceData: `项目计划角色任务 × ${roleTasks.length}`,
          };
        }
        const assignedCount = agents.filter((a) =>
          tasks.some((t) => t.assigned_agent_id === a.id)
        ).length;
        if (assignedCount > 0) {
          return {
            ...base,
            status: "completed",
            statusLabel: `${assignedCount} 个智能体已分配`,
            agentName: assignedAgentNames.length > 0 ? assignedAgentNames.join("、") : null,
            artifacts: assignedAgentNames,
            sourceData: `智能体分配记录 × ${assignedCount}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未分配" };
      }

      // 5 — 执行前审查
      case "preflight_review": {
        if (hasPreflights) {
          const lastPreflight = preflights[preflights.length - 1];
          const isDone = hasGates || hasDryRuns || hasLocks || hasMinimalRuns || lastPreflight.status === "completed" || lastPreflight.status === "approved";
          return {
            ...base,
            status: isDone ? "completed" : "in_progress",
            statusLabel: isDone ? "审查完成" : "审查中",
            agentName: "安全审查官",
            requiresApproval: true,
            riskLevel: (lastPreflight.risk_level as WorkflowNodeData["riskLevel"]) ?? "low",
            artifacts: [lastPreflight.safety_summary ?? "审查报告"],
            sourceData: `执行前审查 × ${preflights.length}`,
          };
        }
        if (hasRunnerRequests || generatedRunnerRequests.length > 0) {
          return {
            ...base,
            status: "pending",
            statusLabel: "可创建审查",
            requiresApproval: true,
            artifacts: generatedRunnerRequests.slice(0, 6).map((request) => {
              const task = roleTasks.find((item) => item.id === request.task_id);
              return task ? `${roleLabel(task.role)}：${task.title}` : "角色执行单";
            }),
            sourceData: `角色执行单 × ${generatedRunnerRequests.length || runnerRequests.length}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未创建" };
      }

      // 6 — 执行许可闸门
      case "execution_gate": {
        if (hasGates) {
          const lastGate = gates[gates.length - 1];
          const isLocked = lastGate.stage_boundary_locked;
          const canExec = lastGate.can_execute;
          const hasAdvanced = hasDryRuns || hasLocks || hasMinimalRuns;
          return {
            ...base,
            status: hasAdvanced || canExec ? "completed" : isLocked ? "locked" : "waiting",
            statusLabel: isLocked ? "闸门已锁定" : canExec ? "许可通过" : "等待许可",
            agentName: "安全审查官",
            requiresApproval: lastGate.requires_second_confirm,
            riskLevel: (lastGate.risk_level as WorkflowNodeData["riskLevel"]) ?? "medium",
            artifacts: lastGate.blocked_reasons?.length
              ? lastGate.blocked_reasons
              : ["许可通过"],
            sourceData: `执行许可闸门 × ${gates.length}`,
          };
        }
        if (hasPreflights) {
          return {
            ...base,
            status: "pending",
            statusLabel: "审查后可创建",
            requiresApproval: true,
            sourceData: "需先通过执行前审查",
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未创建" };
      }

      // 7 — 只读预演
      case "dry_run": {
        if (hasDryRuns) {
          const lastDryRun = dryRuns[dryRuns.length - 1];
          const hasAdvanced = hasLocks || hasMinimalRuns;
          return {
            ...base,
            status: hasAdvanced || lastDryRun.can_execute ? "completed" : "waiting",
            statusLabel: lastDryRun.can_execute ? "预演通过" : "预演未通过",
            agentName: "执行观察官",
            requiresApproval: lastDryRun.requires_second_confirm,
            riskLevel: (lastDryRun.risk_level as WorkflowNodeData["riskLevel"]) ?? "low",
            artifacts: [
              ...lastDryRun.planned_operations,
              ...lastDryRun.planned_file_changes.map((fc) => fc.path),
            ],
            sourceData: `只读预演 × ${dryRuns.length}`,
          };
        }
        if (hasGates) {
          return {
            ...base,
            status: "pending",
            statusLabel: "闸门通过后可创建",
            sourceData: "需先通过执行许可闸门",
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未创建" };
      }

      // 8 — 人工确认
      case "human_confirm": {
        if (hasMinimalRuns || hasLocks) {
          return {
            ...base,
            status: "completed",
            statusLabel: "自动通过",
            artifacts: ["全自动模式已接管中间确认"],
            sourceData: "自动推进记录",
          };
        }
        if (hasPendingApproval) {
          const pendingCount = approvals.filter((a) => a.status === "pending").length;
          return {
            ...base,
            status: "waiting",
            statusLabel: "等待你确认",
            requiresApproval: true,
            riskLevel: "medium",
            artifacts: [`${pendingCount} 个审批等待确认`],
            sourceData: `待审批记录 × ${pendingCount}`,
          };
        }
        if (hasApproved) {
          return {
            ...base,
            status: "completed",
            statusLabel: "已确认",
            artifacts: ["所有审批已通过"],
            sourceData: "审批记录已全部确认",
          };
        }
        return { ...base, status: idx < activeIndex ? "completed" : "pending", statusLabel: "无需确认" };
      }

      // 9 — 最小范围执行
      case "minimal_run": {
        if (hasMinimalRuns) {
          const lastRun = minimalRuns[minimalRuns.length - 1];
          const isDone = lastRun.status === "completed" || lastRun.status === "succeeded";
          return {
            ...base,
            status: isDone ? "completed" : "in_progress",
            statusLabel: isDone ? "沙箱执行完成" : "执行中",
            agentName: "执行观察官",
            riskLevel: "medium",
            canAutoAdvance: false,
            artifacts: lastRun.written_files?.length ? lastRun.written_files : lastRun.command_plan,
            sourceData: `最小范围执行 × ${minimalRuns.length}`,
          };
        }
        if (hasLocks) {
          return {
            ...base,
            status: "locked",
            statusLabel: "已锁定等待执行",
            riskLevel: "medium",
            sourceData: `执行锁定 × ${locks.length}`,
          };
        }
        return { ...base, status: idx < activeIndex ? "skipped" : "pending", statusLabel: "未执行" };
      }

      // 10 — 验收复盘
      case "review": {
        const completedTasks = tasks.filter((t) => t.status === "completed");
        if (completedTasks.length > 0) {
          return {
            ...base,
            status: "in_progress",
            statusLabel: "可复盘",
            agentName: "项目计划官",
            artifacts: completedTasks.map((t) => t.title),
            sourceData: `已完成任务 × ${completedTasks.length}`,
          };
        }
        return { ...base, status: "pending", statusLabel: "等待执行完成" };
      }

      // 11 — 下一轮计划
      case "next_round": {
        if (hasMinimalRuns) {
          return {
            ...base,
            status: "pending",
            statusLabel: "等待你开启",
            canAutoAdvance: false,
            sourceData: "上一轮执行完成后自动进入",
          };
        }
        return { ...base, status: "pending", statusLabel: "等待你开启" };
      }

      default:
        return base;
    }
  });
}

function assignedAgentsForStage(stageKey: string): WorkflowNodeData["assignedAgents"] {
  switch (stageKey) {
    case "idea_clarify":
      return [
        { name: "总控 Agent", responsibility: "理解用户目标，判断项目类型和风险" },
        { name: "产品规划员", responsibility: "补齐目标、用户、核心功能和边界" },
      ];
    case "project_seed":
      return [
        { name: "产品规划员", responsibility: "生成项目种子、MVP 范围和验收标准" },
        { name: "技术架构师", responsibility: "预判技术栈、模块边界和产物目录" },
      ];
    case "req_breakdown":
      return [
        { name: "总控 Agent", responsibility: "把大目标拆成可执行阶段" },
        { name: "产品规划员", responsibility: "把需求拆成用户流程和功能清单" },
      ];
    case "plan_draft":
      return [
        { name: "项目计划官", responsibility: "生成任务草案、依赖顺序和风险点" },
        { name: "技术架构师", responsibility: "检查任务是否跨模块、是否需要转派" },
      ];
    case "agent_assign":
      return [
        { name: "总控 Agent", responsibility: "按技术栈选择固定员工和项目专家" },
        { name: "前端工程师", responsibility: "接收页面、组件、交互任务" },
        { name: "后端工程师", responsibility: "接收 API、数据结构、服务任务" },
        { name: "测试验收员", responsibility: "接收验证、回归和验收任务" },
      ];
    case "preflight_review":
      return [
        { name: "安全审查官", responsibility: "检查文件写入、命令、网络和密钥风险" },
        { name: "总控 Agent", responsibility: "决定是否拆小任务或要求确认" },
      ];
    case "execution_gate":
      return [
        { name: "安全审查官", responsibility: "判断是否允许 Runner 执行" },
        { name: "执行器调度员", responsibility: "选择 Codex、Claude 或模型网关执行路径" },
      ];
    case "dry_run":
      return [
        { name: "执行观察官", responsibility: "只读预演文件变更和执行计划" },
        { name: "总控 Agent", responsibility: "根据预演结果决定继续、回滚或改计划" },
      ];
    case "human_confirm":
      return [
        { name: "审批助手", responsibility: "把高风险动作解释给用户确认" },
        { name: "总控 Agent", responsibility: "等待确认后继续调度" },
      ];
    case "minimal_run":
      return [
        { name: "执行器调度员", responsibility: "通过 Runner 执行受控写入" },
        { name: "对应模块 AI 员工", responsibility: "只在自己负责的模块内写代码或产物" },
      ];
    case "review":
      return [
        { name: "测试验收员", responsibility: "检查产物是否满足目标和任务要求" },
        { name: "文档员", responsibility: "整理运行结果、变更摘要和后续建议" },
      ];
    case "next_round":
      return [
        { name: "总控 Agent", responsibility: "根据验收结果生成下一轮任务" },
        { name: "项目计划官", responsibility: "把遗留问题重新排期" },
      ];
    default:
      return [];
  }
}

// ---------------------------------------------------------------------------
// 页面组件
// ---------------------------------------------------------------------------

export function WorkflowPage({
  project,
  agents,
  tasks,
  approvals,
  connectionStatus,
  message,
  onRefresh,
}: WorkflowPageProps) {
  const [loading, setLoading] = useState(true);
  const [partialFailures, setPartialFailures] = useState<string[]>([]);
  const [ideaInput, setIdeaInput] = useState("");
  const [generatingTasks, setGeneratingTasks] = useState(false);
  const [generationMessage, setGenerationMessage] = useState<string | null>(null);

  // 额外的 Runner 数据
  const [drafts, setDrafts] = useState<ProjectPlanDraftSummary[]>([]);
  const [preflights, setPreflights] = useState<RunnerPreflightReviewSummary[]>([]);
  const [gates, setGates] = useState<RunnerExecutionGateSummary[]>([]);
  const [dryRuns, setDryRuns] = useState<RunnerDryRunSummary[]>([]);
  const [locks, setLocks] = useState<RunnerExecutionLockSummary[]>([]);
  const [minimalRuns, setMinimalRuns] = useState<RunnerMinimalRunSummary[]>([]);
  const [runnerRequests, setRunnerRequests] = useState<RunnerRequestSummary[]>([]);
  const [seeds, setSeeds] = useState<ProjectSeed[]>([]);
  const [executionPreviews, setExecutionPreviews] = useState<ProjectPlanExecutionPreview[]>([]);

  const loadWorkflowData = async (mountedRef?: { current: boolean }) => {
    const isMounted = () => mountedRef?.current ?? true;

    if (!isTauriHost()) {
      // 浏览器模式：仅展示示例空工作流结构，不请求 Tauri 后端
      setDrafts([]);
      setPreflights([]);
      setGates([]);
      setDryRuns([]);
      setLocks([]);
      setMinimalRuns([]);
      setRunnerRequests([]);
      setSeeds([]);
      setExecutionPreviews([]);
      setPartialFailures([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    setPartialFailures([]);

    const failures: string[] = [];
    let pending = 8;

    const failed = (label: string, err: unknown) => {
      failures.push(`${label}：${userErrorLabel(err, "加载失败")}`);
    };

    function done() {
      pending--;
      if (pending === 0 && isMounted()) {
        setPartialFailures(failures);
        setLoading(false);
      }
    }

    listProjectPlanDrafts()
      .then(async (d) => {
        if (isMounted()) setDrafts(d);
        const instantiatedDrafts = d.filter((draft) => draft.status === "instantiated");
        const previews = await Promise.all(
          instantiatedDrafts.map((draft) =>
            getProjectPlanExecutionPreview(draft.approval_id).catch((err: unknown) => {
              failed(`角色任务 ${draft.id}`, err);
              return null;
            }),
          ),
        );
        if (isMounted()) {
          setExecutionPreviews(
            previews.filter((preview): preview is ProjectPlanExecutionPreview => preview !== null),
          );
        }
      })
      .catch((err: unknown) => { failed("计划草案", err); })
      .finally(done);

    listRunnerPreflightReviews()
      .then((d) => { if (isMounted()) setPreflights(d); })
      .catch((err: unknown) => { failed("执行前审查", err); })
      .finally(done);

    listRunnerExecutionGates()
      .then((d) => { if (isMounted()) setGates(d); })
      .catch((err: unknown) => { failed("执行许可闸门", err); })
      .finally(done);

    listRunnerDryRuns()
      .then((d) => { if (isMounted()) setDryRuns(d); })
      .catch((err: unknown) => { failed("只读预演", err); })
      .finally(done);

    listRunnerExecutionLocks()
      .then((d) => { if (isMounted()) setLocks(d); })
      .catch((err: unknown) => { failed("执行锁定", err); })
      .finally(done);

    listRunnerMinimalRuns()
      .then((d) => { if (isMounted()) setMinimalRuns(d); })
      .catch((err: unknown) => { failed("最小范围执行", err); })
      .finally(done);

    listRunnerRequests()
      .then((d) => { if (isMounted()) setRunnerRequests(d); })
      .catch((err: unknown) => { failed("执行请求", err); })
      .finally(done);

    await listProjectSeeds()
      .then((d) => { if (isMounted()) setSeeds(d); })
      .catch((err: unknown) => { failed("项目种子", err); })
      .finally(done);
  };

  useEffect(() => {
    const mountedRef = { current: true };
    void loadWorkflowData(mountedRef);

    return () => {
      mountedRef.current = false;
    };
  }, []);

  const handleAutoGenerateTasks = async () => {
    const idea = ideaInput.trim();
    if (!idea) {
      setGenerationMessage("先写一句想法，蜂群才能分析。");
      return;
    }
    if (!isTauriHost()) {
      setGenerationMessage("当前是浏览器预览模式。请打开桌面端后让蜂群自动生成角色任务。");
      return;
    }

    setGeneratingTasks(true);
    setGenerationMessage("蜂群正在分析想法、分配角色并自动推进执行链...");
    try {
      const input = {
        idea,
        constraints: "全自动生成角色任务、执行单，并自动推进到真实执行链路。",
        requested_by: "swarm_auto",
      };
      let result;
      try {
        result = await autoRunSwarmIdea(input);
      } catch (autoError) {
        const autoMessage = String(autoError);
        if (!autoMessage.includes("auto_run_swarm_idea") && !autoMessage.includes("Command")) {
          throw autoError;
        }
        const fallback = await autoGenerateProjectPlanTasks(input);
        setIdeaInput("");
        setGenerationMessage(
          `已生成 ${fallback.created_task_ids.length} 个角色任务和 ${fallback.created_runner_request_ids.length} 张执行单。`,
        );
        await loadWorkflowData();
        onRefresh?.();
        return;
      }
      setIdeaInput("");
      const succeededCount = result.task_results.filter((item) => item.status === "succeeded").length;
      setGenerationMessage(
        `蜂群已自动生成 ${result.plan.created_task_ids.length} 个角色任务，推进 ${succeededCount}/${result.task_results.length} 张执行单到最小执行记录。`,
      );
      await loadWorkflowData();
      onRefresh?.();
    } catch (error) {
      setGenerationMessage(userErrorLabel(error, "蜂群自动生成任务失败"));
    } finally {
      setGeneratingTasks(false);
    }
  };

  // 构建工作流节点
  const nodes = buildWorkflowNodes({
    tasks,
    approvals,
    agents,
    drafts,
    preflights,
    gates,
    dryRuns,
    locks,
    minimalRuns,
    runnerRequests,
    seeds,
    executionPreviews,
  });

  const completedCount = nodes.filter((n) => n.status === "completed").length;
  const activeNode = nodes.find((n) => n.status === "in_progress" || n.status === "waiting");
  const activeStageKey = activeNode?.stageKey;

  const summaryText = activeNode
    ? `当前阶段：${activeNode.stageLabel} — ${activeNode.statusLabel}`
    : "等待项目启动";

  // 加载中
  if (loading) {
    return (
      <div className="workflow-page">
        <Spin size="large" style={{ display: "block", marginTop: 120 }} />
      </div>
    );
  }

  return (
    <div className="workflow-page">
      {/* 错误提示 */}
      {connectionStatus === "error" ? (
        <div className="console-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <span>{message ?? "桌面宿主连接失败，当前展示只读示例数据。"}</span>
        </div>
      ) : null}

      {partialFailures.length > 0 ? (
        <div className="console-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>部分数据加载失败</strong>
            <ul style={{ margin: "6px 0 0", paddingLeft: 18, fontSize: 12 }}>
              {partialFailures.map((f, i) => (
                <li key={i}>{f}</li>
              ))}
            </ul>
            <span style={{ fontSize: 12 }}>
              已成功加载的数据仍正常展示，失败的数据源对应阶段显示为"等待数据"。
            </span>
          </div>
        </div>
      ) : null}

      {generationMessage ? (
        <div className="console-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <span>{generationMessage}</span>
        </div>
      ) : null}

      {/* 顶部状态栏 */}
      <section className="workflow-status-strip" aria-label="工作流态势">
        <article className="console-mission-card">
          <span>当前工作流</span>
          <strong>{project.name}</strong>
        </article>
        <article className="console-metric-card">
          <span>阶段进度</span>
          <strong>{completedCount} / {nodes.length}</strong>
        </article>
        <article className="console-metric-card">
          <span>活跃阶段</span>
          <strong>{activeNode?.stageLabel ?? "—"}</strong>
        </article>
        <article className="console-metric-card">
          <span>待确认</span>
          <strong>{approvals.filter((a) => a.status === "pending").length}</strong>
        </article>
        <article className="console-metric-card is-safe">
          <span>Runner</span>
          <strong>全自动</strong>
        </article>
      </section>

      {/* 三栏布局 */}
      <div className="workflow-grid">
        {/* 左侧：阶段轨道 */}
        <WorkflowStageRail
          nodes={nodes}
          activeStageKey={activeStageKey}
          completedCount={completedCount}
          totalCount={nodes.length}
        />

        {/* 中间：工作流画布 */}
        <WorkflowCanvas
          nodes={nodes}
          activeStageKey={activeStageKey}
          summaryText={summaryText}
          commandValue={ideaInput}
          commandDisabled={false}
          commandLoading={generatingTasks}
          onCommandChange={setIdeaInput}
          onCommandSubmit={() => void handleAutoGenerateTasks()}
        />

        {/* 右侧：自动化规则 */}
        <AutomationRulesPanel />
      </div>

    </div>
  );
}

export default WorkflowPage;
