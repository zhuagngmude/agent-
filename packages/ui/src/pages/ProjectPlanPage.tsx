import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  App as AntdApp,
  Button,
  Card,
  Checkbox,
  Collapse,
  Form,
  Input,
  Select,
  Space,
  Steps,
  Switch,
  Table,
  Tabs,
  Tag,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import { Network } from "lucide-react";

import { isProjectPlanApprovalTarget } from "@agent-swarm/agent-core";
import IdeaGuidancePanel from "../components/IdeaGuidancePanel";
import type {
  IdeaGuidanceHandoff,
  ProjectSeedDraftPayload,
} from "../components/IdeaGuidancePanel";
import {
  riskLabel,
  roleLabel,
  priorityLabel,
  operationTypeLabel,
  draftSourceLabel,
  agentNameLabel,
} from "../utils/labels";
import { userErrorLabel } from "../utils/userError";
import type {
  ApprovalSummary,
  CreateProjectPlanDraftResponse,
  CreateRunnerPreflightReviewResponse,
  ModelCatalogEntry,
  PlannedTaskSummary,
  ProjectPlanDraftSummary,
  ProjectPlanExecutionPreview,
  ProjectPlanModelDraftResponse,
  ProjectPlanTaskInstanceSummary,
  ProjectPlanTaskTemplateSummary,
  RunnerDryRunSummary,
  RunnerExecutionGateSummary,
  RunnerExecutionLockSummary,
  RunnerMinimalRunSummary,
  RunnerPreflightReviewSummary,
  RunnerRequestSummary,
} from "@agent-swarm/shared";
import {
  approveProjectPlan,
  createProjectPlanDraft,
  deleteProjectPlanDraft,
  createRunnerDryRun,
  createRunnerExecutionGate,
  createRunnerExecutionLock,
  createRunnerMinimalRun,
  createRunnerPreflightReview,
  getProjectPlanExecutionPreview,
  isTauriHost,
  listProjectPlanDrafts,
  listProjectPlanModels,
  listProjectPlanTaskTemplates,
  listRunnerDryRuns,
  listRunnerExecutionGates,
  listRunnerExecutionLocks,
  listRunnerMinimalRuns,
  listRunnerPreflightReviews,
  listRunnerRequests,
  requestProjectPlanModelDraft,
  revokeRunnerDryRun,
  revokeRunnerExecutionGate,
  revokeRunnerExecutionLock,
  saveProjectPlanModelDraft,
  updateProjectPlanModelEnabled,
  updateProjectPlanTaskTemplate,
} from "../utils/desktopHost";

type ProjectPlanPageProps = {
  approvals: ApprovalSummary[];
  refreshOverview: () => void;
  canWrite: boolean;
};

type DraftFormValues = {
  idea: string;
  constraints?: string;
};

type ConfirmFormValues = {
  confirmText: string;
  secondConfirm: boolean;
};

type DraftRow = ProjectPlanDraftSummary & {
  key: string;
  approval_status: string;
};

const DRY_RUN_CREATE_CONFIRM = "我确认只生成只读预演，不执行Runner";
const DRY_RUN_REVOKE_CONFIRM = "我确认撤销只读预演";
const LOCK_CREATE_CONFIRM = "我确认锁定执行范围，不创建Git检查点";
const LOCK_REVOKE_CONFIRM = "我确认撤销执行范围锁";
const DELETE_DRAFT_CONFIRM = "我确认删除项目计划草案";
const IDEA_GUIDANCE_HANDOFF_KEY = "agent_swarm_idea_guidance_handoff";

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    queued: "排队中",
    revoked: "已撤销",
    succeeded: "成功",
    success: "成功",
    failed: "失败",
    blocked: "已阻断",
    blocked_by_stage_boundary: "已被阶段边界阻断",
    locked: "已锁定",
    pending: "待处理",
    approved: "已批准",
    rejected: "已拒绝",
    running: "运行中",
    idle: "空闲",
    instantiated: "已实例化",
    draft: "草案",
    draft_ready: "草案已生成",
    pending_apply: "待应用",
    applied: "已应用",
    cancelled: "已取消",
    unknown: "未知",
  };
  return map[status] ?? "未识别状态";
}

function boolLabel(v: boolean): string {
  return v ? "是" : "否";
}

function reasonLabel(reason: string): string {
  const map: Record<string, string> = {
    runner_execution_disabled_by_stage_boundary: "当前阶段边界禁止执行",
    runner_execution_disabled_until_gate_approved: "执行许可未通过",
    runner_execution_disabled_until_dry_run_reviewed: "只读预演未确认",
    runner_execution_not_allowed_by_stage: "当前阶段不允许执行",
    manual_checkpoint_required_before_stage34: "阶段 34 前需要人工检查点",
    command_failed: "命令执行失败",
    scope_violation: "超出允许范围",
    audit_write_failed: "审计写入失败",
    provider_error: "模型服务返回错误",
    provider_config_error: "模型服务配置异常",
    response_too_large: "响应过大",
    invalid_request: "请求无效",
    invalid_state: "状态无效",
    feature_disabled: "功能开关未开启",
    git_unavailable: "版本管理不可用",
    timeout: "执行超时",
    file_write_not_allowed: "文件写入不被允许",
    file_delete_not_allowed: "文件删除不被允许",
    network_request_not_allowed: "网络请求不被允许",
  };
  return map[reason] ?? "未识别原因";
}

function checkpointStrategyLabel(strategy: string): string {
  const map: Record<string, string> = {
    manual_checkpoint_required_before_stage34: "阶段 34 前需要人工检查点",
    manual_checkpoint_required: "需要人工检查点",
    not_required_for_readonly_preview: "只读预演不需要检查点",
    no_checkpoint_required: "无需检查点",
    auto_checkpoint_before_execution: "执行前自动创建检查点",
  };
  return map[strategy] ?? strategy;
}

function reasonLabelMaybe(value: string): string | null {
  const map: Record<string, string> = {
    runner_execution_disabled_by_stage_boundary: "当前阶段边界禁止执行",
    runner_execution_disabled_until_gate_approved: "执行许可未通过",
    runner_execution_disabled_until_dry_run_reviewed: "只读预演未确认",
    runner_execution_not_allowed_by_stage: "当前阶段不允许执行",
    manual_checkpoint_required_before_stage34: "阶段 34 前需要人工检查点",
    command_failed: "命令执行失败",
    scope_violation: "超出允许范围",
    audit_write_failed: "审计写入失败",
    provider_error: "模型服务返回错误",
    provider_config_error: "模型服务配置异常",
    response_too_large: "响应过大",
    invalid_request: "请求无效",
    invalid_state: "状态无效",
    feature_disabled: "功能开关未开启",
    git_unavailable: "版本管理不可用",
    timeout: "执行超时",
    file_write_not_allowed: "文件写入不被允许",
    file_delete_not_allowed: "文件删除不被允许",
    network_request_not_allowed: "网络请求不被允许",
  };
  return map[value] ?? null;
}

function statusLabelMaybe(value: string): string | null {
  const map: Record<string, string> = {
    queued: "排队中",
    revoked: "已撤销",
    succeeded: "成功",
    success: "成功",
    failed: "失败",
    blocked: "已阻断",
    blocked_by_stage_boundary: "已被阶段边界阻断",
    locked: "已锁定",
    pending: "待处理",
    approved: "已批准",
    rejected: "已拒绝",
    running: "运行中",
    idle: "空闲",
    instantiated: "已实例化",
    draft: "草案",
    draft_ready: "草案已生成",
    pending_apply: "待应用",
    applied: "已应用",
    cancelled: "已取消",
    unknown: "未知",
  };
  return map[value] ?? null;
}

function optionalTextLabel(value: string | null): string {
  if (value === null || value === undefined) return "-";
  // 先尝试用 reasonLabel 映射
  const reasonMapped = reasonLabelMaybe(value);
  if (reasonMapped !== null) return reasonMapped;
  // 再尝试用 statusLabel 映射
  const statusMapped = statusLabelMaybe(value);
  if (statusMapped !== null) return statusMapped;
  // 都不匹配则原样展示后端文本
  return value;
}

const taskColumns: ColumnsType<PlannedTaskSummary> = [
  { title: "角色", dataIndex: "role", width: 100, render: (role: string) => <Tag>{roleLabel(role)}</Tag> },
  { title: "任务", dataIndex: "title" },
  { title: "负责智能体", dataIndex: "assigned_agent_id", render: (id: string) => <Tag>{agentNameLabel(id)}</Tag> },
  { title: "优先级", dataIndex: "priority", width: 90, render: (p: string) => priorityLabel(p) },
  { title: "风险", dataIndex: "risk_level", width: 90, render: (r: string) => riskLabel(r) },
];

const runnerRequestColumns: ColumnsType<RunnerRequestSummary> = [
  {
    title: "执行请求",
    dataIndex: "task_id",
    width: 120,
    render: (taskId: string) => <Tag>{roleLabel(parseRoleFromTaskId(taskId))}</Tag>,
  },
  {
    title: "将处理的任务",
    dataIndex: "task_id",
    render: (taskId: string) => `任务：${roleLabel(parseRoleFromTaskId(taskId))}`,
  },
  {
    title: "状态",
    dataIndex: "status",
    width: 90,
    render: (status: string) => <Tag>{statusLabel(status)}</Tag>,
  },
  {
    title: "操作类型",
    dataIndex: "operation_types",
    render: (operations: string[]) => (
      <Space size={4} wrap>
        {operations.map((operation) => (
          <Tag key={operation}>{operationTypeLabel(operation)}</Tag>
        ))}
      </Space>
    ),
  },
];

function parseRoleFromTaskId(taskId: string): string {
  return taskId.split("_").pop() || "unknown";
}

function shortRecordId(id: string): string {
  if (id.length <= 18) return id;
  return `${id.slice(0, 10)}...${id.slice(-6)}`;
}

function projectPlanStepIndex(params: {
  hasDraft: boolean;
  selectedDraftInstantiated: boolean;
  hasRunnerRequests: boolean;
  hasExecutionLock: boolean;
  hasMinimalRun: boolean;
}): number {
  if (params.hasMinimalRun || params.hasExecutionLock || params.hasRunnerRequests || params.selectedDraftInstantiated) {
    return 2;
  }
  if (params.hasDraft) return 1;
  return 0;
}

export function ProjectPlanPage({ approvals, refreshOverview, canWrite }: ProjectPlanPageProps) {
  const { message } = AntdApp.useApp();
  const [draftForm] = Form.useForm<DraftFormValues>();
  const [confirmForm] = Form.useForm<ConfirmFormValues>();
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [approving, setApproving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<ProjectPlanDraftSummary[]>([]);
  const [runnerRequests, setRunnerRequests] = useState<RunnerRequestSummary[]>([]);
  const [latestPreview, setLatestPreview] = useState<CreateProjectPlanDraftResponse | null>(null);
  const [selectedApprovalId, setSelectedApprovalId] = useState<string | null>(null);
  const [modelDraftLoading, setModelDraftLoading] = useState(false);
  const [modelDraftResult, setModelDraftResult] = useState<ProjectPlanModelDraftResponse | null>(null);
  const [modelDraftError, setModelDraftError] = useState<string | null>(null);
  const [modelDraftForm] = Form.useForm<{ secondConfirm: boolean; confirmText: string }>();
  const [savingModel, setSavingModel] = useState(false);
  const [saveForm] = Form.useForm<{ secondConfirm: boolean; confirmText: string }>();
  const [templates, setTemplates] = useState<ProjectPlanTaskTemplateSummary[]>([]);
  const [templatesLoading, setTemplatesLoading] = useState(false);
  const [togglingRole, setTogglingRole] = useState<string | null>(null);
  const [execPreview, setExecPreview] = useState<ProjectPlanExecutionPreview | null>(null);
  const [execPreviewLoading, setExecPreviewLoading] = useState(false);
  const [preflightReviews, setPreflightReviews] = useState<RunnerPreflightReviewSummary[]>([]);
  const [creatingPreflight, setCreatingPreflight] = useState<string | null>(null);
  const [gates, setGates] = useState<RunnerExecutionGateSummary[]>([]);
  const [creatingGate, setCreatingGate] = useState<string | null>(null);
  const [revokingGate, setRevokingGate] = useState<string | null>(null);
  const [dryRuns, setDryRuns] = useState<RunnerDryRunSummary[]>([]);
  const [creatingDryRun, setCreatingDryRun] = useState<string | null>(null);
  const [revokingDryRun, setRevokingDryRun] = useState<string | null>(null);
  const [execLocks, setExecLocks] = useState<RunnerExecutionLockSummary[]>([]);
  const [creatingLock, setCreatingLock] = useState<string | null>(null);
  const [revokingLock, setRevokingLock] = useState<string | null>(null);
  const [minimalRuns, setMinimalRuns] = useState<RunnerMinimalRunSummary[]>([]);
  const [creatingMinimalRun, setCreatingMinimalRun] = useState<string | null>(null);
  // 阶段 35：模型目录
  const [models, setModels] = useState<ModelCatalogEntry[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [activeTabKey, setActiveTabKey] = useState("draft");
  const [advancedPanelKeys, setAdvancedPanelKeys] = useState<string[]>([]);
  const [ideaGuidanceHandoff, setIdeaGuidanceHandoff] =
    useState<IdeaGuidanceHandoff | null>(null);
  const [deletingDraftId, setDeletingDraftId] = useState<string | null>(null);

  const projectPlanApprovalById = useMemo(() => {
    return new Map(
      approvals
        .filter((approval) => isProjectPlanApprovalTarget(approval.target_service))
        .map((approval) => [approval.id, approval]),
    );
  }, [approvals]);

  const selectedDraft = useMemo(() => {
    if (!selectedApprovalId) return null;
    return drafts.find((draft) => draft.approval_id === selectedApprovalId) ?? null;
  }, [drafts, selectedApprovalId]);

  const selectedApproval = selectedApprovalId
    ? projectPlanApprovalById.get(selectedApprovalId) ?? latestPreview?.approval ?? null
    : latestPreview?.approval ?? null;

  const plannedTasks = latestPreview?.planned_tasks ?? [];
  const plannedRunnerRequests = latestPreview?.planned_runner_requests ?? [];
  const selectedRunnerRequestIds = useMemo(
    () => new Set((execPreview?.runner_requests ?? []).map((request) => request.id)),
    [execPreview],
  );
  const selectedPreflightReviews = useMemo(
    () => preflightReviews.filter((review) => selectedRunnerRequestIds.has(review.runner_request_id)),
    [preflightReviews, selectedRunnerRequestIds],
  );
  const selectedGates = useMemo(
    () => gates.filter((gate) => selectedRunnerRequestIds.has(gate.runner_request_id)),
    [gates, selectedRunnerRequestIds],
  );
  const execTaskById = useMemo(
    () => new Map((execPreview?.tasks ?? []).map((task) => [task.id, task])),
    [execPreview],
  );
  const approvalById = useMemo(
    () => new Map(approvals.map((approval) => [approval.id, approval])),
    [approvals],
  );
  const canApproveSelected =
    canWrite &&
    Boolean(selectedApproval) &&
    selectedApproval?.status === "pending" &&
    selectedDraft?.status !== "instantiated";
  const selectedDraftInstantiated = selectedDraft?.status === "instantiated";
  const hasGeneratedRunnerRequests = (execPreview?.runner_requests.length ?? 0) > 0;
  const hasLockedExecutionScope = selectedRunnerRequestIds.size > 0
    ? execLocks.some((lock) => lock.status === "locked" && selectedRunnerRequestIds.has(lock.runner_request_id))
    : execLocks.some((lock) => lock.status === "locked");
  const hasMinimalRun = selectedRunnerRequestIds.size > 0
    ? minimalRuns.some((run) => selectedRunnerRequestIds.has(run.runner_request_id))
    : minimalRuns.length > 0;
  const simpleStepIndex = projectPlanStepIndex({
    hasDraft: drafts.length > 0,
    selectedDraftInstantiated,
    hasRunnerRequests: hasGeneratedRunnerRequests,
    hasExecutionLock: hasLockedExecutionScope,
    hasMinimalRun,
  });
  const enabledTemplateCount = templates.filter((template) => template.enabled).length;
  const openAdvancedTab = useCallback((tabKey: string) => {
    setAdvancedPanelKeys(["advanced"]);
    setActiveTabKey(tabKey);
    window.setTimeout(() => {
      document
        .querySelector(".project-plan-advanced")
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
    }, 0);
  }, []);

  const loadData = useCallback(async () => {
    if (!isTauriHost()) {
      setDrafts([]);
      setRunnerRequests([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    try {
      const [nextDrafts, nextRequests] = await Promise.all([
        listProjectPlanDrafts(),
        listRunnerRequests(),
      ]);
      setDrafts(nextDrafts);
      setRunnerRequests(nextRequests);
      setError(null);
      setSelectedApprovalId((current) => current ?? nextDrafts[0]?.approval_id ?? null);
    } catch (err) {
      setError(errorText(err));
      setDrafts([]);
      setRunnerRequests([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadTemplates = useCallback(async () => {
    if (!isTauriHost()) return;
    setTemplatesLoading(true);
    try {
      const list = await listProjectPlanTaskTemplates();
      setTemplates(list);
    } catch {
      // 读取失败静默处理，不阻塞页面
    } finally {
      setTemplatesLoading(false);
    }
  }, []);

  const loadPreflightReviews = useCallback(async () => {
    if (!isTauriHost()) return;
    try {
      const list = await listRunnerPreflightReviews();
      setPreflightReviews(list);
    } catch {
      // 静默处理
    }
  }, []);

  const loadGates = useCallback(async () => {
    if (!isTauriHost()) return;
    try {
      const list = await listRunnerExecutionGates();
      setGates(list);
    } catch { /* 静默 */ }
  }, []);

  const loadDryRuns = useCallback(async () => {
    if (!isTauriHost()) return;
    try { setDryRuns(await listRunnerDryRuns()); } catch { /* 静默 */ }
  }, []);

  const loadExecLocks = useCallback(async () => {
    if (!isTauriHost()) return;
    try { setExecLocks(await listRunnerExecutionLocks()); } catch { /* 静默 */ }
  }, []);

  const loadMinimalRuns = useCallback(async () => {
    if (!isTauriHost()) return;
    try { setMinimalRuns(await listRunnerMinimalRuns()); } catch { /* 静默 */ }
  }, []);

  const loadExecutionPreview = useCallback(
    async (approvalId: string) => {
      if (!canWrite) {
        setExecPreview(null);
        return;
      }
      setExecPreviewLoading(true);
      try {
        setExecPreview(await getProjectPlanExecutionPreview(approvalId));
      } catch {
        setExecPreview(null);
      } finally {
        setExecPreviewLoading(false);
      }
    },
    [canWrite],
  );

  const loadModels = useCallback(async () => {
    if (!isTauriHost()) return;
    setModelsLoading(true);
    setModelsError(null);
    try {
      const list = await listProjectPlanModels();
      setModels(list);
      // 初始选中第一个 enabled 模型
      setSelectedModelId((current) => {
        if (current && list.some((m) => m.id === current && m.enabled)) return current;
        const first = list.find((m) => m.enabled);
        return first?.id ?? null;
      });
    } catch {
      setModels([]);
      setSelectedModelId(null);
      setModelsError("模型目录读取失败，请检查桌面宿主是否已启动。");
    } finally {
      setModelsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    loadTemplates();
    loadPreflightReviews();
    loadGates();
    loadDryRuns();
    loadExecLocks();
    loadMinimalRuns();
    loadModels();
  }, [loadData, loadTemplates, loadPreflightReviews, loadGates, loadDryRuns, loadExecLocks, loadMinimalRuns, loadModels]);

  useEffect(() => {
    const raw = window.sessionStorage.getItem(IDEA_GUIDANCE_HANDOFF_KEY);
    if (!raw) return;
    window.sessionStorage.removeItem(IDEA_GUIDANCE_HANDOFF_KEY);
    try {
      const handoff = JSON.parse(raw) as IdeaGuidanceHandoff;
      if (handoff?.idea) {
        setIdeaGuidanceHandoff(handoff);
        setActiveTabKey("draft");
        draftForm.setFieldsValue({
          idea: handoff.idea,
          constraints: [
            handoff.projectTypeLabel ? `总控识别类型：${handoff.projectTypeLabel}` : null,
            handoff.reason ? `总控分流理由：${handoff.reason}` : null,
          ].filter(Boolean).join("\n"),
        });
        message.success("已接收总控分流结果，可继续澄清并创建草案");
      }
    } catch {
      window.sessionStorage.removeItem(IDEA_GUIDANCE_HANDOFF_KEY);
    }
  }, [draftForm, message]);

  useEffect(() => {
    if (!selectedApprovalId || !canWrite) {
      setExecPreview(null);
      return;
    }
    void loadExecutionPreview(selectedApprovalId);
  }, [selectedApprovalId, canWrite, loadExecutionPreview]);

  const handleCreate = useCallback(
    async (values: DraftFormValues) => {
      setCreating(true);
      try {
        const response = await createProjectPlanDraft({
          idea: values.idea,
          constraints: values.constraints ?? null,
          requested_by: "local_user",
        });
        setLatestPreview(response);
        setSelectedApprovalId(response.approval.id);
        setActiveTabKey("approval");
        message.success("项目计划草案已生成，请在审批步骤批准后生成任务");
        draftForm.resetFields();
        await loadData();
        refreshOverview();
      } catch (err) {
        message.error(errorText(err));
      } finally {
        setCreating(false);
      }
    },
    [draftForm, loadData, message, refreshOverview],
  );

  const handleSeedReadyForDraft = useCallback(
    async (payload: ProjectSeedDraftPayload) => {
      draftForm.setFieldsValue({
        idea: payload.idea,
        constraints: payload.constraints,
      });
      if (!canWrite) {
        message.warning("已填入项目计划草案表单；桌面宿主连接后可生成草案");
        return;
      }
      setCreating(true);
      try {
        const response = await createProjectPlanDraft({
          idea: payload.idea,
          constraints: payload.constraints || null,
          requested_by: "controller_agent",
        });
        setLatestPreview(response);
        setSelectedApprovalId(response.approval.id);
        setActiveTabKey("approval");
        message.success("已用项目种子创建项目计划草案，请批准后生成任务");
        await loadData();
        refreshOverview();
      } catch (err) {
        message.error(errorText(err));
      } finally {
        setCreating(false);
      }
    },
    [canWrite, draftForm, loadData, message, refreshOverview],
  );

  const handleApprove = useCallback(async () => {
    if (!selectedApprovalId) return;
    const values = await confirmForm.validateFields();
    setApproving(true);
    try {
      await approveProjectPlan({
        approval_id: selectedApprovalId,
        second_confirm: values.secondConfirm,
        confirm_text: values.confirmText,
      });
      const draftSource = selectedDraft?.generated_by;
      message.success(
        draftSource === "real_model_preview"
          ? "真实模型草案已批准，任务和只读队列已生成"
          : "项目计划已批准，任务和只读队列已生成",
      );
      confirmForm.resetFields();
      setLatestPreview(null);
      openAdvancedTab("generated");
      await loadData();
      await loadExecutionPreview(selectedApprovalId);
      refreshOverview();
    } catch (err) {
      message.error(errorText(err));
    } finally {
      setApproving(false);
    }
  }, [confirmForm, loadData, loadExecutionPreview, message, openAdvancedTab, refreshOverview, selectedApprovalId, selectedDraft]);

  const handleDeleteDraft = useCallback(
    async (draft: DraftRow) => {
      if (draft.status !== "draft") {
        message.warning("只能删除尚未实例化的草案");
        return;
      }
      const input = window.prompt(`删除草案需要输入确认文本：${DELETE_DRAFT_CONFIRM}`);
      if (input !== DELETE_DRAFT_CONFIRM) {
        message.warning("确认文本不匹配，已取消删除");
        return;
      }
      setDeletingDraftId(draft.id);
      try {
        await deleteProjectPlanDraft({
          draft_id: draft.id,
          second_confirm: true,
          confirm_text: input,
        });
        message.success("项目计划草案已删除");
        if (selectedApprovalId === draft.approval_id) {
          setSelectedApprovalId(null);
          setExecPreview(null);
        }
        await loadData();
        refreshOverview();
      } catch (err) {
        message.error(errorText(err));
      } finally {
        setDeletingDraftId(null);
      }
    },
    [loadData, message, refreshOverview, selectedApprovalId],
  );

  const handleModelDraft = useCallback(async () => {
    if (!canWrite) {
      message.warning("桌面宿主未连接或不可写，真实模型调用不可用");
      return;
    }
    const idea: string = draftForm.getFieldValue("idea") ?? "";
    if (!idea.trim()) {
      message.warning('请先在「生成项目计划草案」中填写项目想法');
      return;
    }

    const confirmValues = await modelDraftForm.validateFields();
    setModelDraftLoading(true);
    setModelDraftError(null);
    setModelDraftResult(null);

    try {
      const response = await requestProjectPlanModelDraft({
        idea: idea.trim(),
        constraints: (draftForm.getFieldValue("constraints") as string | undefined) ?? null,
        second_confirm: confirmValues.secondConfirm,
        confirm_text: confirmValues.confirmText,
        model_record_id: selectedModelId,
      });

      setModelDraftResult(response);

      if (response.status === "draft_ready") {
        const auditMsg =
          response.audit_record_id
            ? "真实模型草案已生成（仅预览），已写入安全审计记录"
            : "真实模型草案已生成（仅预览）";
        message.success(auditMsg);
      } else {
        const label =
          MODEL_DRAFT_ERROR_LABELS[response.error_category ?? ""] ??
          "真实模型调用失败";
        setModelDraftError(label);
      }
    } catch {
      // 不展示 raw error，避免内部细节泄露。调试日志后续另做安全日志。
      setModelDraftError("真实模型调用失败，请检查桌面宿主状态或模型配置。");
    } finally {
      setModelDraftLoading(false);
    }
  }, [canWrite, draftForm, modelDraftForm, message, selectedModelId]);

  const handleSaveModel = useCallback(async () => {
    const idea: string = draftForm.getFieldValue("idea") ?? "";
    if (!idea.trim()) {
      message.warning("请先在「生成项目计划草案」中填写项目想法");
      return;
    }
    if (!modelDraftResult?.summary || !modelDraftResult?.audit_record_id) {
      message.warning("没有可保存的真实模型草案");
      return;
    }
    if (modelDraftResult.status !== "draft_ready") {
      message.warning("当前模型草案状态不允许保存");
      return;
    }
    const saveValues = await saveForm.validateFields();
    setSavingModel(true);
    try {
      const response = await saveProjectPlanModelDraft({
        idea: idea.trim(),
        constraints: (draftForm.getFieldValue("constraints") as string | undefined) ?? null,
        audit_record_id: modelDraftResult.audit_record_id,
        second_confirm: saveValues.secondConfirm,
        confirm_text: saveValues.confirmText,
      });
      setLatestPreview(response);
      setSelectedApprovalId(response.approval.id);
      setActiveTabKey("approval");
      message.success("真实模型草案已保存为待审批项目计划草案，请批准后生成任务");
      saveForm.resetFields();
      await loadData();
      refreshOverview();
    } catch {
      message.error("保存真实模型草案失败，请重试");
    } finally {
      setSavingModel(false);
    }
  }, [draftForm, modelDraftResult, saveForm, message, loadData, refreshOverview]);

  const handleToggleTemplate = useCallback(
    async (role: string, enabled: boolean) => {
      setTogglingRole(role);
      try {
        const updated = await updateProjectPlanTaskTemplate({ role, enabled });
        setTemplates(updated);
        message.success(enabled ? `已启用 ${role}` : `已停用 ${role}`);
      } catch {
        message.error("模板更新失败");
      } finally {
        setTogglingRole(null);
      }
    },
    [message],
  );

  const handleCreatePreflight = useCallback(
    async (runnerRequestId: string) => {
      setCreatingPreflight(runnerRequestId);
      try {
        const response = await createRunnerPreflightReview({
          runner_request_id: runnerRequestId,
          second_confirm: true,
          confirm_text: "我确认只创建执行前审查，不执行 Runner",
        });
        message.success(
          response.review.status === "blocked"
            ? "执行前审查已创建（已被系统边界阻断）"
            : "执行前审查已创建",
        );
        await loadPreflightReviews();
      } catch {
        message.error("创建执行前审查失败");
      } finally {
        setCreatingPreflight(null);
      }
    },
    [message, loadPreflightReviews],
  );

  const handleCreateGate = useCallback(async (preflightReviewId: string) => {
    setCreatingGate(preflightReviewId);
    try {
      await createRunnerExecutionGate({
        preflight_review_id: preflightReviewId,
        second_confirm: true,
        confirm_text: "我确认只创建执行许可记录，不执行 Runner",
      });
      message.success("执行许可记录已创建（已被系统边界锁定）");
      await loadGates();
    } catch {
      message.error("创建执行许可记录失败");
    } finally { setCreatingGate(null); }
  }, [message, loadGates]);

  const handleRevokeGate = useCallback(async (gateId: string) => {
    setRevokingGate(gateId);
    try {
      await revokeRunnerExecutionGate({
        gate_id: gateId,
        second_confirm: true,
        confirm_text: "我确认撤销执行许可记录",
      });
      message.success("执行许可记录已撤销");
      await loadGates();
    } catch {
      message.error("撤销执行许可记录失败");
    } finally { setRevokingGate(null); }
  }, [message, loadGates]);

  const handleCreateDryRun = useCallback(async (gateId: string) => {
    const confirmed = window.prompt(`请输入确认文本：${DRY_RUN_CREATE_CONFIRM}`);
    if (confirmed !== DRY_RUN_CREATE_CONFIRM) {
      message.warning("确认文本不匹配，已取消生成只读预演");
      return;
    }
    setCreatingDryRun(gateId);
    try {
      await createRunnerDryRun({
        gate_id: gateId,
        second_confirm: true,
        confirm_text: DRY_RUN_CREATE_CONFIRM,
      });
      message.success("只读预演已生成");
      await loadDryRuns();
    } catch { message.error("生成只读预演失败"); }
    finally { setCreatingDryRun(null); }
  }, [message, loadDryRuns]);

  const handleRevokeDryRun = useCallback(async (dryRunId: string) => {
    const confirmed = window.prompt(`请输入确认文本：${DRY_RUN_REVOKE_CONFIRM}`);
    if (confirmed !== DRY_RUN_REVOKE_CONFIRM) {
      message.warning("确认文本不匹配，已取消撤销只读预演");
      return;
    }
    setRevokingDryRun(dryRunId);
    try {
      await revokeRunnerDryRun({
        dry_run_id: dryRunId,
        second_confirm: true,
        confirm_text: DRY_RUN_REVOKE_CONFIRM,
      });
      message.success("只读预演已撤销");
      await loadDryRuns();
    } catch { message.error("撤销只读预演失败"); }
    finally { setRevokingDryRun(null); }
  }, [message, loadDryRuns]);

  const handleCreateLock = useCallback(async (dryRunId: string) => {
    const confirmed = window.prompt(`请输入确认文本：${LOCK_CREATE_CONFIRM}`);
    if (confirmed !== LOCK_CREATE_CONFIRM) { message.warning("确认文本不正确"); return; }
    setCreatingLock(dryRunId);
    try {
      await createRunnerExecutionLock({ dry_run_id: dryRunId, second_confirm: true, confirm_text: LOCK_CREATE_CONFIRM });
      message.success("执行范围锁已创建");
      await loadExecLocks();
    } catch { message.error("锁定执行范围失败"); }
    finally { setCreatingLock(null); }
  }, [message, loadExecLocks]);

  const handleRevokeLock = useCallback(async (lockId: string) => {
    const confirmed = window.prompt(`请输入确认文本：${LOCK_REVOKE_CONFIRM}`);
    if (confirmed !== LOCK_REVOKE_CONFIRM) { message.warning("确认文本不正确"); return; }
    setRevokingLock(lockId);
    try {
      await revokeRunnerExecutionLock({ execution_lock_id: lockId, second_confirm: true, confirm_text: LOCK_REVOKE_CONFIRM });
      message.success("执行范围锁已撤销");
      await loadExecLocks();
    } catch { message.error("撤销执行范围锁失败"); }
    finally { setRevokingLock(null); }
  }, [message, loadExecLocks]);

  const handleCreateMinimalRun = useCallback(async (lockId: string) => {
    const confirmed = window.prompt("请输入确认文本：我确认执行阶段34最小Runner，只允许沙箱范围");
    if (confirmed !== "我确认执行阶段34最小Runner，只允许沙箱范围") { message.warning("确认文本不正确"); return; }
    setCreatingMinimalRun(lockId);
    try {
      await createRunnerMinimalRun({ execution_lock_id: lockId, second_confirm: true, confirm_text: "我确认执行阶段34最小Runner，只允许沙箱范围" });
      message.success("阶段 34 最小执行已完成");
      await loadMinimalRuns();
    } catch { message.error("阶段 34 最小执行失败"); }
    finally { setCreatingMinimalRun(null); }
  }, [message, loadMinimalRuns]);

  const canSaveModel =
    canWrite &&
    !savingModel &&
    modelDraftResult?.status === "draft_ready" &&
    Boolean(modelDraftResult?.summary) &&
    Boolean(modelDraftResult?.audit_record_id);

  const draftRows: DraftRow[] = drafts.map((draft) => ({
    ...draft,
    key: draft.id,
    approval_status: projectPlanApprovalById.get(draft.approval_id)?.status ?? "unknown",
  }));

  const draftColumns: ColumnsType<DraftRow> = [
    { title: "摘要", dataIndex: "summary" },
    {
      title: "来源",
      dataIndex: "generated_by",
      width: 120,
      render: (source: string) => (
        <Tag color={source === "real_model_preview" ? "blue" : "default"}>
          {draftSourceLabel(source)}
        </Tag>
      ),
    },
    {
      title: "草案状态",
      dataIndex: "status",
      width: 110,
      render: (status: string) => <Tag>{statusLabel(status)}</Tag>,
    },
    {
      title: "审批状态",
      dataIndex: "approval_status",
      width: 110,
      render: (status: string) => <Tag color={status === "pending" ? "processing" : "default"}>{statusLabel(status)}</Tag>,
    },
    { title: "请求人", dataIndex: "requested_by", width: 120 },
    {
      title: "操作",
      width: 90,
      render: (_: unknown, record: DraftRow) => (
        <Button
          size="small"
          danger
          loading={deletingDraftId === record.id}
          disabled={!canWrite || record.status !== "draft" || deletingDraftId !== null}
          onClick={() => void handleDeleteDraft(record)}
        >
          删除
        </Button>
      ),
    },
  ];

  return (
    <Space orientation="vertical" size={16} className="page-stack project-plan-workbench">
      <div className="page-heading">
        <Typography.Title level={2}>
          <Network size={22} style={{ marginRight: 8, verticalAlign: "middle" }} />
          项目计划
        </Typography.Title>
        <Typography.Text type="secondary">
          默认只走三步：说想法、看计划、确认开干。高级闸门和审计明细仍然保留。
        </Typography.Text>
      </div>

      {!canWrite && (
        <Alert
          type="info"
          showIcon
          title="桌面宿主未连接或不可写。生成草案、批准计划和真实模型调用需要 Tauri 桌面宿主已连接。"
        />
      )}

      {error && <Alert type="error" showIcon title="读取项目计划失败" description={error} />}

      <Card className="project-plan-simple-flow">
        <Space orientation="vertical" size={16} style={{ width: "100%" }}>
          <Steps
            current={simpleStepIndex}
            items={[
              {
                title: "说想法",
                description: drafts.length > 0 ? `已有 ${drafts.length} 份草案` : "输入目标，或让想法引导官先追问",
              },
              {
                title: "看计划",
                description: selectedDraftInstantiated
                  ? "计划已批准，任务已生成"
                  : canApproveSelected
                    ? "选中草案后批准生成任务"
                    : "等待可批准的草案",
              },
              {
                title: "开干",
                description: hasMinimalRun
                  ? "已有最小执行记录"
                  : hasLockedExecutionScope
                    ? "范围已锁定，可进入最小执行"
                    : hasGeneratedRunnerRequests
                      ? "任务单已生成，可继续安全准备"
                      : "审批后才会出现任务单",
              },
            ]}
          />

          <div className="project-plan-simple-grid">
            <Card size="small" title="1. 说想法" className="project-plan-simple-card">
              <Typography.Paragraph type="secondary">
                从一句话开始。需要追问时用想法引导官；已经想清楚时直接生成草案。
              </Typography.Paragraph>
              <Space wrap>
                <Button type="primary" onClick={() => openAdvancedTab("draft")}>
                  去生成草案
                </Button>
                <Tag>{enabledTemplateCount} 个任务角色已启用</Tag>
              </Space>
            </Card>

            <Card size="small" title="2. 看计划" className="project-plan-simple-card">
              <Typography.Paragraph type="secondary">
                草案不会立刻变成任务。批准后，会按当前启用的 {enabledTemplateCount} 个角色生成任务和只读任务单。
              </Typography.Paragraph>
              <Space wrap>
                <Button
                  type={canApproveSelected ? "primary" : "default"}
                  disabled={!canWrite || drafts.length === 0}
                  onClick={() => openAdvancedTab("approval")}
                >
                  {canApproveSelected ? "去批准计划" : "查看草案"}
                </Button>
                <Tag>{drafts.length} 份草案</Tag>
              </Space>
            </Card>

            <Card size="small" title="3. 开干" className="project-plan-simple-card">
              <Typography.Paragraph type="secondary">
                安全准备可以在高级明细中完成；真正执行仍只允许阶段 34 的沙箱最小范围。
              </Typography.Paragraph>
              <Space wrap>
                <Button
                  danger={hasLockedExecutionScope}
                  type={hasLockedExecutionScope ? "primary" : "default"}
                  disabled={!selectedDraftInstantiated && !hasGeneratedRunnerRequests}
                  onClick={() => openAdvancedTab(hasLockedExecutionScope ? "execute" : "review")}
                >
                  {hasLockedExecutionScope ? "去最小执行" : "查看安全准备"}
                </Button>
                <Tag>{hasGeneratedRunnerRequests ? `${execPreview?.runner_requests.length ?? 0} 张任务单` : "未生成任务单"}</Tag>
              </Space>
            </Card>
          </div>

          <Alert
            type="info"
            showIcon
            title="普通使用只看上面三步。下面的高级明细用于审计、返修和排查问题。"
          />
        </Space>
      </Card>

      <Collapse
        className="project-plan-advanced"
        activeKey={advancedPanelKeys}
        onChange={(keys) => setAdvancedPanelKeys(Array.isArray(keys) ? keys.map(String) : [String(keys)])}
        items={[
          {
            key: "advanced",
            label: "高级流程明细：草案、审批、任务单、安全准备、最小执行",
            children: (
      <Tabs
        activeKey={activeTabKey}
        onChange={setActiveTabKey}
        items={[
          {
            key: "draft",
            label: "1 草案",
            children: (
              <Space orientation="vertical" size={16} style={{ width: "100%" }}>
                <Alert
                  type="info"
                  showIcon
                  title="先把想法变成一份可审批草案。需要 AI 追问时用想法引导官，想法清楚时直接生成本地草案。"
                />

                <IdeaGuidancePanel
                  canWrite={canWrite}
                  handoff={ideaGuidanceHandoff}
                  onSeedReadyForDraft={handleSeedReadyForDraft}
                />

      <Card title="生成项目计划草案">
        <Form<DraftFormValues> form={draftForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item
            name="idea"
            label="项目想法"
            rules={[
              { required: true, message: "请输入项目想法" },
              { max: 500, message: "项目想法不能超过 500 字" },
            ]}
          >
            <Input.TextArea rows={4} maxLength={500} showCount />
          </Form.Item>
          <Form.Item
            name="constraints"
            label="约束"
            rules={[{ max: 2000, message: "约束不能超过 2000 字" }]}
          >
            <Input.TextArea rows={3} maxLength={2000} showCount />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={creating} disabled={!canWrite}>
            生成草案
          </Button>
        </Form>
      </Card>

      <Card
        title="真实模型草案预览"
        extra={
          !canWrite ? (
            <Tag color="default">桌面宿主未连接</Tag>
          ) : null
        }
      >
        <Space orientation="vertical" size={12} style={{ width: "100%" }}>
          <Alert
            type="info"
            showIcon
            title="真实模型草案只作为预览，不会创建任务、审批或执行请求。"
          />

          {!canWrite && (
            <Alert
              type="warning"
              showIcon
              title="桌面宿主未连接或不可写，真实模型调用不可用。请确认 Tauri 桌面宿主已启动并连接。"
            />
          )}

          {modelDraftError && (
            <Alert type="error" showIcon title={modelDraftError} closable onClose={() => setModelDraftError(null)} />
          )}

          {modelDraftResult?.status === "draft_ready" && (
            <>
              <Alert
                type="success"
                showIcon
                title={
                  modelDraftResult.audit_record_id
                    ? "真实模型草案已生成（仅预览），已写入安全审计记录"
                    : "真实模型草案已生成（仅预览）"
                }
              />
              {modelDraftResult.summary && (
                <Card size="small" title="模型摘要">
                  <Typography.Paragraph style={{ whiteSpace: "pre-wrap", marginBottom: 0 }}>
                    {modelDraftResult.summary}
                  </Typography.Paragraph>
                </Card>
              )}
              {modelDraftResult.warnings.length > 0 && (
                <Alert
                  type="warning"
                  showIcon
                  title="提示"
                  description={
                    <ul style={{ margin: 0, paddingLeft: 20 }}>
                      {modelDraftResult.warnings.map((w, i) => (
                        <li key={i}>{w}</li>
                      ))}
                    </ul>
                  }
                />
              )}
            </>
          )}

          <Form<{ secondConfirm: boolean; confirmText: string }>
            form={modelDraftForm}
            layout="vertical"
          >
            <Form.Item
              name="secondConfirm"
              valuePropName="checked"
              rules={[
                {
                  validator: (_, value: boolean | undefined) =>
                    value === true
                      ? Promise.resolve()
                      : Promise.reject(new Error("请勾选二次确认")),
                },
              ]}
            >
              <Checkbox>我确认发起一次真实模型调用</Checkbox>
            </Form.Item>

            <Form.Item
              name="confirmText"
              label="确认文本"
              rules={[
                { required: true, message: "请输入确认文本" },
                {
                  validator: (_, value: string | undefined) =>
                    value === "我确认发起真实模型调用"
                      ? Promise.resolve()
                      : Promise.reject(new Error("确认文本必须为：我确认发起真实模型调用")),
                },
              ]}
            >
              <Input placeholder="请输入：我确认发起真实模型调用" />
            </Form.Item>

            {/* 阶段 35：模型选择下拉框 */}
            <Form.Item label="选择模型">
              <Select
                value={selectedModelId}
                onChange={setSelectedModelId}
                loading={modelsLoading}
                placeholder="选择一个已启用模型"
                notFoundContent="暂无模型目录记录"
                style={{ maxWidth: 400 }}
                options={models.map((m) => ({
                  value: m.id,
                  label: `${m.display_name || m.model_id} (${m.model_id})`,
                  disabled: !m.enabled,
                }))}
              />
              {modelsError && (
                <Alert
                  type="warning"
                  showIcon
                  style={{ marginTop: 8, maxWidth: 640 }}
                  message={modelsError}
                />
              )}
              {!modelsLoading && !modelsError && models.length === 0 && (
                <Alert
                  type="info"
                  showIcon
                  style={{ marginTop: 8, maxWidth: 640 }}
                  message="还没有模型目录记录"
                  description="如需调用 DeepSeek/OpenAI 兼容模型，请用 scripts/start-desktop-real-model.ps1 启动桌面端，或确认数据库已初始化模型目录。"
                />
              )}
              {models.length > 0 && !models.some((m) => m.enabled) && (
                <Alert
                  type="warning"
                  showIcon
                  style={{ marginTop: 8, maxWidth: 640 }}
                  message="模型目录里没有已启用模型"
                  description="请在模型目录中启用一个项目计划模型；真实模型草案按钮会在选中已启用模型后可用。"
                />
              )}
            </Form.Item>

            <Button
              type="primary"
              loading={modelDraftLoading}
              disabled={!canWrite || modelDraftLoading || !selectedModelId || !models.some((m) => m.id === selectedModelId && m.enabled)}
              onClick={handleModelDraft}
            >
              生成真实模型草案
            </Button>
          </Form>

          {modelDraftResult?.status === "draft_ready" &&
            modelDraftResult.summary &&
            modelDraftResult.audit_record_id && (
              <Card size="small" title="保存为项目计划草案" style={{ marginTop: 16 }}>
                <Form<{ secondConfirm: boolean; confirmText: string }>
                  form={saveForm}
                  layout="vertical"
                >
                  <Form.Item
                    name="secondConfirm"
                    valuePropName="checked"
                    rules={[
                      {
                        validator: (_, value: boolean | undefined) =>
                          value === true
                            ? Promise.resolve()
                            : Promise.reject(new Error("请勾选二次确认")),
                      },
                    ]}
                  >
                    <Checkbox>我确认保存真实模型草案为待审批项目计划</Checkbox>
                  </Form.Item>
                  <Form.Item
                    name="confirmText"
                    label="确认文本"
                    rules={[
                      { required: true, message: "请输入确认文本" },
                      {
                        validator: (_, value: string | undefined) =>
                          value === "我确认保存真实模型草案"
                            ? Promise.resolve()
                            : Promise.reject(new Error("确认文本必须为：我确认保存真实模型草案")),
                      },
                    ]}
                  >
                    <Input placeholder="请输入：我确认保存真实模型草案" />
                  </Form.Item>
                  <Button
                    type="primary"
                    loading={savingModel}
                    disabled={!canSaveModel}
                    onClick={handleSaveModel}
                  >
                    保存为项目计划草案
                  </Button>
                </Form>
              </Card>
            )}
        </Space>
      </Card>

      <Card
        title={`任务角色模板（启用 ${templates.filter((t) => t.enabled).length} 个）`}
        extra={
          !canWrite ? <Tag color="default">桌面宿主未连接</Tag> : null
        }
      >
        <Alert
          type="info"
          showIcon
          title="模板只影响后续审批生成任务，不执行执行引擎、不调用模型、不写文件或修改版本。"
          style={{ marginBottom: 12 }}
        />
        <Table<ProjectPlanTaskTemplateSummary>
          loading={templatesLoading}
          dataSource={templates}
          rowKey="role"
          pagination={false}
          size="small"
          columns={[
            { title: "角色", dataIndex: "role", width: 100, render: (r: string) => roleLabel(r) },
            { title: "任务", dataIndex: "title" },
            { title: "智能体", dataIndex: "agent_id", width: 140, render: (id: string) => agentNameLabel(id) },
            { title: "风险", dataIndex: "risk_level", width: 70, render: (r: string) => riskLabel(r) },
            {
              title: "启用",
              dataIndex: "enabled",
              width: 70,
              render: (enabled: boolean, record: ProjectPlanTaskTemplateSummary) => (
                <Switch
                  checked={enabled}
                  loading={togglingRole === record.role}
                  disabled={!canWrite || togglingRole !== null}
                  onChange={(checked) => handleToggleTemplate(record.role, checked)}
                />
              ),
            },
          ]}
        />
      </Card>

              </Space>
            ),
          },
          {
            key: "approval",
            label: "2 审批",
            children: (
              <Space orientation="vertical" size={16} style={{ width: "100%" }}>
                <Alert
                  type="info"
                  showIcon
                  title="选择一份草案批准后，系统会拆成任务和任务单。任务单只进入队列，执行仍在后续步骤单独确认。"
                />

      <Card title="计划草案">
        <Table<DraftRow>
          loading={loading}
          columns={draftColumns}
          dataSource={draftRows}
          pagination={false}
          rowKey="approval_id"
          rowSelection={{
            type: "radio",
            selectedRowKeys: selectedApprovalId ? [selectedApprovalId] : [],
            onChange: (_, rows) => setSelectedApprovalId(rows[0]?.approval_id ?? null),
          }}
          locale={{ emptyText: "暂无项目计划草案" }}
          expandable={{
            expandedRowRender: (draft) => (
              <Space orientation="vertical" size={8}>
                <Typography.Text>{draft.idea}</Typography.Text>
                <Typography.Text type="secondary">{draft.constraints ?? "无额外约束"}</Typography.Text>
              </Space>
            ),
          }}
        />
      </Card>

      <Card title="批准生成任务">
        <Space orientation="vertical" size={12} style={{ width: "100%" }}>
          <Alert
            type="warning"
            showIcon
            title="批准后会创建排队任务和只读执行请求，不会启动执行引擎、调用模型、写文件或修改版本。"
          />
          <Form<ConfirmFormValues> form={confirmForm} layout="vertical">
            <Form.Item
              name="secondConfirm"
              valuePropName="checked"
              rules={[{ validator: (_, value) => (value ? Promise.resolve() : Promise.reject(new Error("请勾选二次确认"))) }]}
            >
              <Checkbox>我确认只生成任务和只读队列，不启动执行引擎</Checkbox>
            </Form.Item>
            <Form.Item
              name="confirmText"
              label="确认文本"
              rules={[
                { required: true, message: "请输入确认文本" },
                {
                  validator: (_, value: string | undefined) =>
                    value?.includes("生成任务")
                      ? Promise.resolve()
                      : Promise.reject(new Error('确认文本必须包含"生成任务"')),
                },
              ]}
            >
              <Input placeholder="请输入：确认生成任务" />
            </Form.Item>
            <Button
              type="primary"
              danger
              loading={approving}
              disabled={!canApproveSelected}
              onClick={handleApprove}
            >
              批准生成任务
            </Button>
          </Form>
        </Space>
      </Card>

              </Space>
            ),
          },
          {
            key: "generated",
            label: "3 已生成",
            children: (
              <Space orientation="vertical" size={16} style={{ width: "100%" }}>
                <Alert
                  type="info"
                  showIcon
                  title="已生成的任务和执行请求仅用于审查和预览，不会启动执行引擎、不会写文件、不会修改版本。"
                />

      {latestPreview && (
        <Card title="刚生成的内存预览">
          <Space orientation="vertical" size={12} style={{ width: "100%" }}>
            <Alert
              type="success"
              showIcon
              title="草案已创建，下面是审批前预览。任务和执行请求还没有落入对应表。"
            />
            <Table<PlannedTaskSummary>
              columns={taskColumns}
              dataSource={plannedTasks}
              pagination={false}
              rowKey="id"
              size="small"
            />
            <Table<RunnerRequestSummary>
              columns={runnerRequestColumns}
              dataSource={plannedRunnerRequests}
              pagination={false}
              rowKey="id"
              size="small"
            />
          </Space>
        </Card>
      )}

      <Card title="已生成任务和只读执行请求">
        <Alert
          type="info"
          showIcon
          title="只展示审批后生成的任务和只读执行请求，不启动执行引擎，不写文件，不修改版本。"
          style={{ marginBottom: 12 }}
        />
        {!selectedApprovalId ? (
          <Typography.Text type="secondary">请先选择一个项目计划草案。</Typography.Text>
        ) : execPreviewLoading ? (
          <Typography.Text type="secondary">加载中…</Typography.Text>
        ) : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={12} style={{ width: "100%" }}>
            <div className="project-plan-task-card-grid">
              {execPreview.tasks.map((task) => (
                <article key={task.id} className="project-plan-task-card">
                  <div className="project-plan-task-card__head">
                    <Tag color="blue">{roleLabel(task.role)}</Tag>
                    <Tag>{statusLabel(task.status)}</Tag>
                  </div>
                  <strong>{task.title}</strong>
                  <p>{task.description ?? "等待该角色补充交付说明。"}</p>
                  <div className="project-plan-task-card__meta">
                    <span>负责：{task.assigned_agent_id ? agentNameLabel(task.assigned_agent_id) : "未分配"}</span>
                    <span>优先级：{priorityLabel(task.priority)}</span>
                    <span>风险：{task.risk_level ? riskLabel(task.risk_level) : "未评估"}</span>
                  </div>
                </article>
              ))}
            </div>
            <Table<ProjectPlanTaskInstanceSummary>
              title={() => `角色任务明细（${execPreview.tasks.length} 个，来自当前审批时启用的角色模板）`}
              dataSource={execPreview.tasks}
              rowKey="id"
              pagination={false}
              size="small"
              columns={[
                { title: "角色", dataIndex: "role", width: 90, render: (r: string) => roleLabel(r) },
                { title: "任务名称", dataIndex: "title" },
                {
                  title: "交付目标",
                  dataIndex: "description",
                  ellipsis: true,
                  render: (description: string | null) => description ?? "等待补充",
                },
                { title: "状态", dataIndex: "status", width: 80, render: (s: string) => statusLabel(s) },
                { title: "负责智能体", dataIndex: "assigned_agent_id", width: 130, render: (id: string | null) => id ? agentNameLabel(id) : "未分配" },
                { title: "优先级", dataIndex: "priority", width: 70, render: (p: string) => priorityLabel(p) },
                { title: "风险", dataIndex: "risk_level", width: 70, render: (r: string | null) => r ? riskLabel(r) : "未评估" },
              ]}
            />
            <Table<RunnerRequestSummary>
              title={() => `只读执行请求（${execPreview.runner_requests.length} 条）`}
              dataSource={execPreview.runner_requests}
              rowKey="id"
              pagination={false}
              size="small"
              columns={[
                {
                  title: "角色",
                  dataIndex: "task_id",
                  width: 100,
                  render: (taskId: string) => {
                    const task = execTaskById.get(taskId);
                    return <Tag>{roleLabel(task?.role ?? parseRoleFromTaskId(taskId))}</Tag>;
                  },
                },
                {
                  title: "关联任务",
                  dataIndex: "task_id",
                  render: (taskId: string) => execTaskById.get(taskId)?.title ?? "未知任务",
                },
                { title: "状态", dataIndex: "status", width: 80, render: (s: string) => statusLabel(s) },
                {
                  title: "操作类型",
                  dataIndex: "operation_types",
                  render: (ops: string[]) => ops.map(operationTypeLabel).join(", "),
                },
                {
                  title: "安全说明",
                  dataIndex: "safety_note",
                  ellipsis: true,
                },
              ]}
              expandable={{
                expandedRowRender: (request) => (
                  <Space orientation="vertical" size={4}>
                    <Typography.Text type="secondary">执行请求编号：{shortRecordId(request.id)}</Typography.Text>
                    <Typography.Text type="secondary">任务编号：{shortRecordId(request.task_id)}</Typography.Text>
                  </Space>
                ),
              }}
            />
          </Space>
        ) : (
          <Space orientation="vertical" size={8}>
            <Typography.Text type="secondary">
              {execPreview
                ? "草案尚未审批，暂无已生成任务。"
                : "还没有拿到已生成任务。如果刚刚批准成功，请刷新这一块。"}
            </Typography.Text>
            {selectedApprovalId && (
              <Button
                size="small"
                onClick={() => void loadExecutionPreview(selectedApprovalId)}
              >
                刷新已生成任务
              </Button>
            )}
          </Space>
        )}
      </Card>

              </Space>
            ),
          },
          {
            key: "review",
            label: "4 审查与锁定",
            children: (
              <Space orientation="vertical" size={16} style={{ width: "100%" }}>
                <Alert
                  type="info"
                  showIcon
                  title="这里把任务单依次做预检、放行、试跑和范围锁定。默认只生成记录和清单，真正执行放在最后一步。"
                />

      <Card title="执行前审查">
        <Alert
          type="warning"
          showIcon
          title="预检会为任务单创建一条审查记录，标记风险和阻断原因，供后续放行使用。"
          style={{ marginBottom: 12 }}
        />
        {!canWrite ? (
          <Typography.Text type="secondary">桌面宿主未连接。</Typography.Text>
        ) : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={8} style={{ width: "100%" }}>
            <Table<RunnerRequestSummary>
              dataSource={execPreview.runner_requests}
              rowKey="id"
              pagination={false}
              size="small"
              columns={[
                {
                  title: "角色",
                  dataIndex: "task_id",
                  width: 100,
                  render: (taskId: string) => {
                    const task = execTaskById.get(taskId);
                    return <Tag>{roleLabel(task?.role ?? parseRoleFromTaskId(taskId))}</Tag>;
                  },
                },
                {
                  title: "关联任务",
                  dataIndex: "task_id",
                  render: (taskId: string) => execTaskById.get(taskId)?.title ?? "未知任务",
                },
                {
                  title: "操作",
                  width: 160,
                  render: (_: unknown, record: RunnerRequestSummary) => {
                    const existing = selectedPreflightReviews.find(
                      (r) => r.runner_request_id === record.id,
                    );
                    if (existing) {
                      return (
                        <Tag color={existing.status === "blocked" ? "orange" : "blue"}>
                          {statusLabel(existing.status)}
                        </Tag>
                      );
                    }
                    return (
                      <Button
                        size="small"
                        loading={creatingPreflight === record.id}
                        disabled={creatingPreflight !== null}
                        onClick={() => handleCreatePreflight(record.id)}
                      >
                        创建执行前审查
                      </Button>
                    );
                  },
                },
              ]}
              expandable={{
                expandedRowRender: (request) => (
                  <Space orientation="vertical" size={4}>
                    <Typography.Text type="secondary">执行请求编号：{shortRecordId(request.id)}</Typography.Text>
                    <Typography.Text type="secondary">任务编号：{shortRecordId(request.task_id)}</Typography.Text>
                  </Space>
                ),
              }}
            />
            {selectedPreflightReviews.length > 0 && (
              <Table<RunnerPreflightReviewSummary>
                title={() => "已有审查记录"}
                dataSource={selectedPreflightReviews}
                rowKey="id"
                pagination={false}
                size="small"
                columns={[
                  { title: "状态", dataIndex: "status", width: 100, render: (s: string) => statusLabel(s) },
                  { title: "风险", dataIndex: "risk_level", width: 70 },
                  { title: "关联审批", dataIndex: "approval_id", ellipsis: true },
                  {
                    title: "阻断原因",
                    dataIndex: "blocked_reasons",
                    render: (reasons: string[]) => reasons.map(reasonLabel).join("、"),
                  },
                  { title: "安全说明", dataIndex: "safety_summary", ellipsis: true },
                ]}
              />
            )}
          </Space>
        ) : (
          <Typography.Text type="secondary">
            请先审批草案以查看可创建审查的执行请求。
          </Typography.Text>
        )}
      </Card>

      <Card title="执行许可">
        <Alert type="warning" showIcon
          title="放行会记录任务是否满足进入试跑的条件；它本身不执行任务。"
          style={{ marginBottom: 12 }} />
        {!canWrite ? <Typography.Text type="secondary">桌面宿主未连接。</Typography.Text>
        : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={8} style={{ width: "100%" }}>
            <Table<RunnerPreflightReviewSummary>
              dataSource={selectedPreflightReviews}
              rowKey="id" pagination={false} size="small"
              columns={[
                { title: "审查记录", dataIndex: "id", ellipsis: true },
                {
                  title: "审批状态",
                  dataIndex: "approval_id",
                  width: 120,
                  render: (approvalId: string) => <Tag>{statusLabel(approvalById.get(approvalId)?.status ?? "unknown")}</Tag>,
                },
                {
                  title: "操作", width: 200,
                  render: (_: unknown, pf: RunnerPreflightReviewSummary) => {
                    const pfGate = selectedGates.find(g => g.preflight_review_id === pf.id);
                    if (pfGate) {
                      return (
                        <Space size={4}>
                          <Tag color={pfGate.status === "revoked" ? "red" : "orange"}>{statusLabel(pfGate.status)}</Tag>
                          {pfGate.status !== "revoked" && (
                            <Button size="small" danger loading={revokingGate === pfGate.id}
                              disabled={revokingGate !== null}
                              onClick={() => handleRevokeGate(pfGate.id)}>撤销许可记录</Button>
                          )}
                        </Space>
                      );
                    }
                    const preflightApproval = approvalById.get(pf.approval_id);
                    if (preflightApproval?.status !== "approved") {
                      return <Tag color="default">需先批准审查</Tag>;
                    }
                    return (
                      <Button size="small" loading={creatingGate === pf.id}
                        disabled={creatingGate !== null}
                        onClick={() => handleCreateGate(pf.id)}>创建执行许可记录</Button>
                    );
                  },
                },
              ]} />
            {selectedGates.length > 0 && (
              <Table<RunnerExecutionGateSummary>
                title={() => "已有执行许可记录"}
                dataSource={selectedGates}
                rowKey="id" pagination={false} size="small"
                columns={[
                  { title: "状态", dataIndex: "status", width: 150, render: (s: string) => statusLabel(s) },
                  { title: "可执行", dataIndex: "can_execute", width: 90, render: (v: boolean) => <Tag>{boolLabel(v)}</Tag> },
                  { title: "阶段边界锁定", dataIndex: "stage_boundary_locked", width: 110, render: (v: boolean) => <Tag>{boolLabel(v)}</Tag> },
                  { title: "阻断原因", dataIndex: "blocked_reasons", render: (r: string[]) => r.map(reasonLabel).join("、") },
                  { title: "撤销原因", dataIndex: "revoked_reason", render: (v: string | null) => optionalTextLabel(v) },
                ]} />
            )}
          </Space>
        ) : <Typography.Text type="secondary">请先审批草案。</Typography.Text>}
      </Card>

      <Card title="只读预演">
        <Alert type="warning" showIcon
          title="试跑会生成计划、命令清单和影响文件清单，用来确认任务准备得是否足够清楚。"
          style={{ marginBottom: 12 }} />
        {!canWrite ? <Typography.Text type="secondary">桌面宿主未连接。</Typography.Text>
        : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={8} style={{ width: "100%" }}>
            <Table<RunnerExecutionGateSummary>
              dataSource={selectedGates ?? gates.filter(g => execPreview.runner_requests.some(rr => rr.id === g.runner_request_id))}
              rowKey="id" pagination={false} size="small"
              columns={[
                { title: "执行许可", dataIndex: "id", ellipsis: true },
                { title: "状态", dataIndex: "status", width: 160, render: (s: string) => statusLabel(s) },
                {
                  title: "操作", width: 200,
                  render: (_: unknown, g: RunnerExecutionGateSummary) => {
                    if (g.status === "revoked") return <Tag color="red">执行许可已撤销</Tag>;
                    const dr = dryRuns.find(d => d.gate_id === g.id);
                    if (dr) {
                      return (
                        <Space size={4}>
                          <Tag color={dr.status === "revoked" ? "red" : "orange"}>{statusLabel(dr.status)}</Tag>
                          {dr.status !== "revoked" && (
                            <Button size="small" danger loading={revokingDryRun === dr.id}
                              disabled={revokingDryRun !== null}
                              onClick={() => handleRevokeDryRun(dr.id)}>撤销预演</Button>
                          )}
                        </Space>
                      );
                    }
                    return (
                      <Button size="small" loading={creatingDryRun === g.id}
                        disabled={creatingDryRun !== null}
                        onClick={() => handleCreateDryRun(g.id)}>生成只读预演</Button>
                    );
                  },
                },
              ]} />
            {dryRuns.length > 0 && (
              <Table<RunnerDryRunSummary>
                title={() => "已有只读预演"}
                dataSource={dryRuns.filter(d => execPreview.runner_requests.some(rr => rr.id === d.runner_request_id))}
                rowKey="id" pagination={false} size="small"
                columns={[
                  { title: "状态", dataIndex: "status", width: 160, render: (s: string) => statusLabel(s) },
                  { title: "可执行", dataIndex: "can_execute", width: 90, render: (v: boolean) => <Tag>{boolLabel(v)}</Tag> },
                  { title: "阶段边界锁定", dataIndex: "stage_boundary_locked", width: 110, render: (v: boolean) => <Tag>{boolLabel(v)}</Tag> },
                  { title: "计划命令数", dataIndex: "planned_commands", render: (cmds: string[]) => String(cmds.length) },
                  { title: "影响文件数", dataIndex: "planned_file_changes", render: (fc: {path:string}[]) => String(fc.length) },
                  { title: "允许文件数", dataIndex: "allowed_files", render: (af: string[]) => String(af.length) },
                  { title: "阻断原因", dataIndex: "blocked_reasons", render: (r: string[]) => r.map(reasonLabel).join("、") },
                ]} />
            )}
          </Space>
        ) : <Typography.Text type="secondary">请先审批草案。</Typography.Text>}
      </Card>

      <Card title="执行范围锁">
        <Alert type="warning" showIcon title="范围锁会固定允许文件和禁止路径，避免后续执行越界。" style={{ marginBottom: 12 }} />
        {!canWrite ? <Typography.Text type="secondary">桌面宿主未连接。</Typography.Text>
        : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={8} style={{ width: "100%" }}>
            <Table<RunnerDryRunSummary>
              dataSource={dryRuns.filter(d => execPreview.runner_requests.some(rr => rr.id === d.runner_request_id))}
              rowKey="id" pagination={false} size="small"
              columns={[
                { title: "只读预演", dataIndex: "id", ellipsis: true },
                { title: "状态", dataIndex: "status", width: 160, render: (s: string) => statusLabel(s) },
                { title: "命令数", dataIndex: "planned_commands", render: (cmds: string[]) => String(cmds.length), width: 60 },
                {
                  title: "操作", width: 200,
                  render: (_: unknown, dr: RunnerDryRunSummary) => {
                    if (dr.status === "revoked") return <Tag color="red">预演已撤销</Tag>;
                    const lk = execLocks.find(l => l.dry_run_id === dr.id);
                    if (lk) {
                      return (
                        <Space size={4}>
                          <Tag color={lk.status === "revoked" ? "red" : "blue"}>{statusLabel(lk.status)}</Tag>
                          {lk.status !== "revoked" && (
                            <Button size="small" danger loading={revokingLock === lk.id}
                              disabled={revokingLock !== null}
                              onClick={() => handleRevokeLock(lk.id)}>撤销范围锁</Button>
                          )}
                        </Space>
                      );
                    }
                    return (
                      <Button size="small" loading={creatingLock === dr.id}
                        disabled={creatingLock !== null}
                        onClick={() => handleCreateLock(dr.id)}>锁定执行范围</Button>
                    );
                  },
                },
              ]} />
            {execLocks.length > 0 && (
              <Table<RunnerExecutionLockSummary>
                title={() => "已有执行范围锁"}
                dataSource={execLocks.filter(l => execPreview.runner_requests.some(rr => rr.id === l.runner_request_id))}
                rowKey="id" pagination={false} size="small"
                columns={[
                  { title: "状态", dataIndex: "status", width: 80, render: (s: string) => statusLabel(s) },
                  { title: "可执行", dataIndex: "can_execute", width: 90, render: (v: boolean) => <Tag>{boolLabel(v)}</Tag> },
                  { title: "允许文件", dataIndex: "allowed_files", render: (f: string[]) => String(f.length) },
                  { title: "禁止路径", dataIndex: "denied_paths", render: (d: string[]) => d.slice(0,3).join(", ") + (d.length > 3 ? "..." : "") },
                  { title: "检查点策略", dataIndex: "checkpoint_strategy", ellipsis: true, render: (v: string) => checkpointStrategyLabel(v) },
                ]} />
            )}
          </Space>
        ) : <Typography.Text type="secondary">请先审批草案。</Typography.Text>}
      </Card>

              </Space>
            ),
          },
          {
            key: "execute",
            label: "5 执行",
            children: (
              <Space orientation="vertical" size={16} style={{ width: "100%" }}>
                <Alert
                  type="warning"
                  showIcon
                  title="这是当前唯一会真实执行的一步：只允许白名单命令和沙箱文件，不自动提交版本。"
                />

      <Card title="阶段 34 最小执行">
        <Alert type="error" showIcon title="点击执行前请确认范围锁无误。此步只写入沙箱文件，并执行固定白名单命令。" style={{ marginBottom: 12 }} />
        {!canWrite ? <Typography.Text type="secondary">桌面宿主未连接。</Typography.Text>
        : execPreview && execPreview.draft.status === "instantiated" ? (
          <Space orientation="vertical" size={8} style={{ width: "100%" }}>
            <Table<RunnerExecutionLockSummary>
              dataSource={execLocks.filter(l => l.status === "locked" && execPreview.runner_requests.some(rr => rr.id === l.runner_request_id))}
              rowKey="id" pagination={false} size="small"
              columns={[
                { title: "执行范围锁", dataIndex: "id", ellipsis: true },
                { title: "状态", dataIndex: "status", width: 80, render: (s: string) => statusLabel(s) },
                {
                  title: "操作", width: 200,
                  render: (_: unknown, lk: RunnerExecutionLockSummary) => {
                    const existing = minimalRuns.find(m => m.execution_lock_id === lk.id);
                    if (existing) {
                      return <Space size={4}><Tag color={existing.status === "succeeded" ? "green" : existing.status === "failed" ? "red" : "orange"}>{statusLabel(existing.status)}</Tag><Typography.Text type="secondary">已执行</Typography.Text></Space>;
                    }
                    return <Button size="small" type="primary" danger loading={creatingMinimalRun === lk.id} disabled={creatingMinimalRun !== null} onClick={() => handleCreateMinimalRun(lk.id)}>执行阶段34最小任务</Button>;
                  },
                },
              ]} />
            {minimalRuns.length > 0 && (
              <Table<RunnerMinimalRunSummary>
                title={() => "已有执行记录"}
                dataSource={minimalRuns.filter(m => execPreview.runner_requests.some(rr => rr.id === m.runner_request_id))}
                rowKey="id" pagination={false} size="small"
                columns={[
                  { title: "状态", dataIndex: "status", width: 120, render: (s: string) => statusLabel(s) },
                  { title: "写入文件", dataIndex: "written_files", render: (f: string[]) => f.join(", ") },
                  { title: "命令数", dataIndex: "command_plan", render: (c: string[]) => String(c.length) },
                  { title: "失败原因", dataIndex: "failure_summary", render: (v: string | null) => optionalTextLabel(v) },
                ]} />
            )}
          </Space>
        ) : <Typography.Text type="secondary">请先审批草案。</Typography.Text>}
      </Card>

              </Space>
            ),
          },
        ]}
      />
            ),
          },
        ]}
      />
    </Space>
  );
}

function errorText(error: unknown): string {
  return userErrorLabel(error, "操作失败，请稍后重试");
}

const MODEL_DRAFT_ERROR_LABELS: Record<string, string> = {
  feature_disabled: "真实模型开关未开启",
  missing_key: "缺少模型 API Key",
  missing_base_url: "缺少模型 Base URL",
  invalid_base_url: "模型 Base URL 不符合白名单",
  invalid_request: "二次确认或输入不符合要求",
  provider_error: "模型服务返回错误",
  timeout: "模型请求超时",
  network_error: "网络请求失败",
  response_too_large: "模型响应超过限制",
  audit_write_failed: "真实模型调用审计记录写入失败",
};
