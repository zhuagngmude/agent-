/**
 * ConsoleDashboardPage — Agent 蜂群画布式主控台。
 *
 * 主页保持只读态势展示和原有“全自动执行”入口，只把主工作区改成
 * 蓝图画布视觉：点阵背景、节点、连线、工具条和小地图。
 */

import { useRef, useState } from "react";
import type { MouseEvent, PointerEvent, WheelEvent } from "react";
import {
  AlertTriangle,
  Bot,
  FileText,
  Maximize2,
  MessageSquareText,
  Minus,
  Plus,
  RefreshCw,
  Sparkles,
  Trash2,
  Workflow,
} from "lucide-react";

import type {
  AgentSummary,
  ApprovalSummary,
  ProjectSummary,
  TaskSummary,
  TaskStatus,
} from "@agent-swarm/shared";
import type { PageKey } from "../routes/mainNavItems";
import { statusLabel } from "../utils/labels";
import { autoGenerateProjectPlanTasks, autoRunSwarmIdea, chatWithController, classifyProjectIntake, isTauriHost } from "../utils/desktopHost";
import { userErrorLabel } from "../utils/userError";

type ConsoleDashboardPageProps = {
  project: ProjectSummary;
  tasks: TaskSummary[];
  agents: AgentSummary[];
  approvals: ApprovalSummary[];
  connectionStatus: "loading" | "browser" | "connected" | "error";
  message?: string;
  onNavigate: (page: PageKey) => void;
  onRefresh?: () => void;
};

type AgentProgress = {
  agent: AgentSummary;
  task: TaskSummary | null;
  progress: number;
  state: "idle" | "queued" | "running" | "waiting" | "blocked" | "done";
  stateLabel: string;
  tone: "blue" | "green" | "amber" | "red" | "slate";
};

type BlueprintModuleKind = "start" | "agent" | "manager" | "slot" | "condition" | "summary";

type BlueprintMenuState = {
  x: number;
  y: number;
  worldX: number;
  worldY: number;
};

type BlueprintCustomModule = {
  id: string;
  kind: BlueprintModuleKind;
  title: string;
  subtitle: string;
  x: number;
  y: number;
};

type BlueprintPortSide = "left" | "right";

type BlueprintConnection = {
  id: string;
  fromId: string;
  toId: string;
};

type BlueprintSelectedPort = {
  moduleId: string;
  side: BlueprintPortSide;
};

type BlueprintDragState = {
  id: string;
  pointerId: number;
  offsetX: number;
  offsetY: number;
};

type BlueprintViewportState = {
  x: number;
  y: number;
  scale: number;
};

type BlueprintPanState = {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  startX: number;
  startY: number;
};

type ControllerChatMessage = {
  id: string;
  role: "user" | "controller";
  text: string;
};

const BLUEPRINT_MODULE_OPTIONS: Array<{
  kind: BlueprintModuleKind;
  title: string;
  subtitle: string;
}> = [
  { kind: "start", title: "开始", subtitle: "用户输入需求" },
  { kind: "agent", title: "智能体", subtitle: "执行具体模块任务" },
  { kind: "manager", title: "管理器", subtitle: "拆分和派发一组任务" },
  { kind: "slot", title: "槽位", subtitle: "承载并行工作分支" },
  { kind: "condition", title: "条件", subtitle: "按判断结果分流" },
  { kind: "summary", title: "汇总", subtitle: "收敛输出和结论" },
];

const BASIC_WORKFLOW_TEMPLATE = [
  BLUEPRINT_MODULE_OPTIONS[0],
  BLUEPRINT_MODULE_OPTIONS[2],
  BLUEPRINT_MODULE_OPTIONS[1],
  BLUEPRINT_MODULE_OPTIONS[3],
  BLUEPRINT_MODULE_OPTIONS[4],
  BLUEPRINT_MODULE_OPTIONS[5],
];

const DEFAULT_WORKFLOW_MODULES: BlueprintCustomModule[] = BASIC_WORKFLOW_TEMPLATE.map((option, index) => ({
  id: `default-${option.kind}`,
  kind: option.kind,
  title: option.title,
  subtitle: option.subtitle,
  x: 520 + (index % 2) * 240,
  y: 150 + Math.floor(index / 2) * 130,
}));

const DEFAULT_WORKFLOW_CONNECTIONS: BlueprintConnection[] = DEFAULT_WORKFLOW_MODULES.slice(0, -1).map((module, index) => ({
  id: `default-connection-${index}`,
  fromId: module.id,
  toId: DEFAULT_WORKFLOW_MODULES[index + 1].id,
}));

const BLUEPRINT_WORLD_WIDTH = 1180;
const BLUEPRINT_WORLD_HEIGHT = 680;
const BLUEPRINT_NODE_WIDTH = 190;
const BLUEPRINT_NODE_HEIGHT = 88;
const BLUEPRINT_MIN_SCALE = 0.62;
const BLUEPRINT_MAX_SCALE = 1.7;

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function getTaskProgress(status: TaskStatus): number {
  switch (status) {
    case "queued":
      return 18;
    case "running":
      return 62;
    case "waiting_user":
      return 78;
    case "blocked":
      return 48;
    case "failed":
      return 44;
    case "cancelled":
      return 8;
    case "completed":
      return 100;
  }
}

function getAgentProgress(agent: AgentSummary, tasks: TaskSummary[]): AgentProgress {
  const assignedTasks = tasks.filter((task) => task.assigned_agent_id === agent.id);
  const activeTask =
    assignedTasks.find((task) => task.status === "running") ??
    assignedTasks.find((task) => task.status === "waiting_user") ??
    assignedTasks.find((task) => task.status === "blocked" || task.status === "failed") ??
    assignedTasks.find((task) => task.status === "queued") ??
    assignedTasks.find((task) => task.status === "completed") ??
    null;

  if (!activeTask) {
    return {
      agent,
      task: null,
      progress: agent.status === "idle" ? 0 : 12,
      state: "idle",
      stateLabel: "待命",
      tone: "slate",
    };
  }

  const doneCount = assignedTasks.filter((task) => task.status === "completed").length;
  const baseProgress = getTaskProgress(activeTask.status);
  const progress =
    assignedTasks.length > 1
      ? Math.min(100, Math.round((doneCount / assignedTasks.length) * 70 + baseProgress * 0.3))
      : baseProgress;

  if (activeTask.status === "completed") {
    return { agent, task: activeTask, progress, state: "done", stateLabel: "已交付", tone: "green" };
  }

  if (activeTask.status === "blocked" || activeTask.status === "failed") {
    return { agent, task: activeTask, progress, state: "blocked", stateLabel: statusLabel(activeTask.status), tone: "red" };
  }

  if (activeTask.status === "waiting_user") {
    return { agent, task: activeTask, progress, state: "waiting", stateLabel: "等你确认", tone: "amber" };
  }

  if (activeTask.status === "running") {
    return { agent, task: activeTask, progress, state: "running", stateLabel: "工作中", tone: "blue" };
  }

  return { agent, task: activeTask, progress, state: "queued", stateLabel: "排队中", tone: "slate" };
}

function getConnectionLabel(connectionStatus: ConsoleDashboardPageProps["connectionStatus"]): string {
  switch (connectionStatus) {
    case "connected":
      return "桌面真实数据";
    case "browser":
      return "浏览器示例数据";
    case "error":
      return "连接失败，展示示例数据";
    case "loading":
      return "正在连接";
  }
}

function getMissionText(project: ProjectSummary, tasks: TaskSummary[], pendingApprovalCount: number): string {
  if (pendingApprovalCount > 0) {
    return `${project.name} 有 ${pendingApprovalCount} 个动作等待确认`;
  }

  const runningTask = tasks.find((task) => task.status === "running" || task.status === "waiting_user");
  if (runningTask) {
    return `正在推进：${runningTask.title}`;
  }

  const queuedTask = tasks.find((task) => task.status === "queued");
  if (queuedTask) {
    return `下一步排队：${queuedTask.title}`;
  }

  return `${project.name} 等待你发起下一步`;
}

function getBlueprintModuleIcon(kind: BlueprintModuleKind) {
  switch (kind) {
    case "start":
      return <Sparkles size={14} aria-hidden="true" />;
    case "agent":
      return <Bot size={14} aria-hidden="true" />;
    case "manager":
      return <Workflow size={14} aria-hidden="true" />;
    case "slot":
      return <FileText size={14} aria-hidden="true" />;
    case "condition":
      return <AlertTriangle size={14} aria-hidden="true" />;
    case "summary":
      return <MessageSquareText size={14} aria-hidden="true" />;
  }
}

function getModulePortPosition(module: BlueprintCustomModule, side: BlueprintPortSide) {
  return {
    x: module.x + (side === "left" ? -12 : BLUEPRINT_NODE_WIDTH - 12),
    y: module.y + 32,
  };
}

function getConnectionPath(from: BlueprintCustomModule, to: BlueprintCustomModule): string {
  const start = getModulePortPosition(from, "right");
  const end = getModulePortPosition(to, "left");
  const distance = Math.max(80, Math.abs(end.x - start.x) * 0.55);
  return `M ${start.x} ${start.y} C ${start.x + distance} ${start.y}, ${end.x - distance} ${end.y}, ${end.x} ${end.y}`;
}

function getControllerReply(idea: string): string {
  const text = idea.trim();
  const normalized = text.toLowerCase();

  if (/连线|链接|连接|线|端口/.test(text)) {
    return "连线表示模块之间的流转关系。点一个模块右侧端口，再点另一个模块左侧端口，就能把两个模块连起来；以后它会表示任务、数据或审批结果从前一个模块交给后一个模块。";
  }

  if (/拖|移动|拖动|画布|缩放|放大|缩小/.test(text)) {
    return "画布空白处按住左键可以拖动画布，鼠标滚轮可以放大缩小。模块本身也能直接拖动，缩放后端口和连线会跟着一起移动。";
  }

  if (/模块|节点|开始|管理器|智能体|槽位|条件|汇总/.test(text)) {
    return "基础工作流由六个模块组成：开始负责接收需求，管理器负责拆分任务，智能体负责执行，槽位负责承载分支，条件负责判断走向，汇总负责收敛结果。你可以右键画布添加模块，也可以点左侧基础工作流快速添加。";
  }

  if (/执行|runner|审批|确认|安全/.test(normalized)) {
    return "真正执行代码、写文件、调用外部模型时，必须先经过审批和 Runner 安全链路。当前主控台还是前端预览版，只会改画布，不会直接执行危险动作。";
  }

  if (/添加|生成|创建|流程|工作流/.test(text)) {
    return "我可以先帮你生成一条最小工作流：开始 -> 管理器 -> 智能体 -> 槽位 -> 条件 -> 汇总。当前是前端预览，会把模块和连线放到画布上，后面再接真实总控 Agent。";
  }

  if (/怎么|如何|为什么|是什么|啥|不懂|不会/.test(text)) {
    return "你可以直接问我这些模块、连线、执行器、模型网关、审批、Runner 是什么意思。我会先用简单话解释；如果你要做东西，我再把它变成画布上的工作流。";
  }

  return "我先按总控理解：你是在描述一个目标或问题。现在我可以先帮你解释概念，或者把它拆成基础工作流模块。后面接入模型网关后，这里会变成真正的总控 Agent 对话。";
}

export function ConsoleDashboardPage({
  project,
  tasks,
  agents,
  approvals,
  connectionStatus,
  message,
  onNavigate,
  onRefresh,
}: ConsoleDashboardPageProps) {
  const agentProgress = agents.map((agent) => getAgentProgress(agent, tasks));
  const pendingApprovalCount = approvals.filter((approval) => approval.status === "pending").length;
  const missionText = getMissionText(project, tasks, pendingApprovalCount);
  const connectionLabel = getConnectionLabel(connectionStatus);
  const [intakeIdea, setIntakeIdea] = useState("");
  const [intakeError, setIntakeError] = useState<string | null>(null);
  const [autoRunning, setAutoRunning] = useState(false);
  const [controllerThinking, setControllerThinking] = useState(false);
  const [controllerMessages, setControllerMessages] = useState<ControllerChatMessage[]>([
    {
      id: "controller-welcome",
      role: "controller",
      text: "你可以问我模块、连线、执行器怎么用，也可以让我生成一条基础工作流。",
    },
  ]);
  const [blueprintMenu, setBlueprintMenu] = useState<BlueprintMenuState | null>(null);
  const [customModules, setCustomModules] = useState<BlueprintCustomModule[]>(() => DEFAULT_WORKFLOW_MODULES);
  const [customConnections, setCustomConnections] = useState<BlueprintConnection[]>(() => DEFAULT_WORKFLOW_CONNECTIONS);
  const [selectedPort, setSelectedPort] = useState<BlueprintSelectedPort | null>(null);
  const [draggingModuleId, setDraggingModuleId] = useState<string | null>(null);
  const [blueprintViewport, setBlueprintViewport] = useState<BlueprintViewportState>({ x: 0, y: 0, scale: 1 });
  const [isPanningBlueprint, setIsPanningBlueprint] = useState(false);
  const canvasRef = useRef<HTMLElement | null>(null);
  const activeDragRef = useRef<BlueprintDragState | null>(null);
  const panRef = useRef<BlueprintPanState | null>(null);
  const notice =
    connectionStatus === "error"
      ? (message ?? "桌面宿主连接失败，当前展示只读示例数据。")
      : null;

  const handleAutoRunIntake = async () => {
    const idea = intakeIdea.trim();
    if (!idea) {
      setIntakeError("先写一句你想做什么。");
      return;
    }
    if (!isTauriHost()) {
      setIntakeError("当前是浏览器预览模式。请打开桌面端后让蜂群全自动执行。");
      return;
    }

    setAutoRunning(true);
    setIntakeError(null);
    let projectType: string | null = null;
    let questions: string[] = [];
    try {
      const classification = await classifyProjectIntake({ idea });
      projectType = classification.session.project_type_label;
      questions = classification.session.recommended_questions ?? [];
    } catch {
      projectType = null;
      questions = [];
    }

    const constraints = [
      projectType ? `项目类型：${projectType}` : null,
      questions.length > 0 ? `总控澄清重点：${questions.join("；")}` : null,
      "全自动生成角色任务、执行单，并自动推进到最小执行记录。",
    ]
      .filter(Boolean)
      .join("\n");

    try {
      const input = {
        idea,
        constraints,
        requested_by: "swarm_auto",
      };
      let createdTaskCount = 0;
      try {
        const result = await autoRunSwarmIdea(input);
        createdTaskCount = result.plan.created_task_ids.length;
      } catch (autoError) {
        const autoMessage = String(autoError);
        if (!autoMessage.includes("auto_run_swarm_idea") && !autoMessage.includes("Command")) {
          throw autoError;
        }
        const fallback = await autoGenerateProjectPlanTasks(input);
        createdTaskCount = fallback.created_task_ids.length;
      }
      if (createdTaskCount === 0) {
        throw new Error("蜂群没有生成任何任务，请检查模型配置或任务模板。");
      }
      setIntakeIdea("");
      onRefresh?.();
      onNavigate("tasks");
      window.setTimeout(() => onRefresh?.(), 250);
    } catch (error) {
      setIntakeError(userErrorLabel(error, "蜂群全自动执行失败"));
    } finally {
      setAutoRunning(false);
    }
  };

  const handleControllerChat = async () => {
    const idea = intakeIdea.trim();
    if (!idea) {
      setIntakeError("先写一句你想问什么，或者想让总控做什么。");
      return;
    }

    const shouldCreateWorkflow = /添加|生成|创建|流程|工作流/.test(idea);
    setControllerMessages((current) => [
      ...current,
      { id: `user-${Date.now()}`, role: "user", text: idea },
    ]);

    if (shouldCreateWorkflow) {
      addBasicWorkflowTemplate();
    }

    setIntakeIdea("");
    setIntakeError(null);

    if (!isTauriHost()) {
      setControllerMessages((current) => [
        ...current,
        { id: `controller-${Date.now()}`, role: "controller", text: getControllerReply(idea) },
      ]);
      return;
    }

    setControllerThinking(true);
    try {
      const response = await chatWithController({ message: idea });
      setControllerMessages((current) => [
        ...current,
        { id: `controller-${Date.now()}`, role: "controller", text: response.reply },
      ]);
    } catch (error) {
      setControllerMessages((current) => [
        ...current,
        {
          id: `controller-${Date.now()}`,
          role: "controller",
          text: `${getControllerReply(idea)}\n\n真实总控对话暂时没接通：${userErrorLabel(error)}`,
        },
      ]);
    } finally {
      setControllerThinking(false);
    }
  };

  const addBlueprintModuleFromLibrary = (option: (typeof BLUEPRINT_MODULE_OPTIONS)[number], index: number) => {
    const offsetX = 250 + (index % 2) * 214;
    const offsetY = 230 + Math.floor(index / 2) * 112;
    setCustomModules((current) => [
      ...current,
      {
        id: `module-${Date.now()}-${current.length}`,
        kind: option.kind,
        title: option.title,
        subtitle: option.subtitle,
        x: offsetX,
        y: offsetY,
      },
    ]);
  };

  const addBasicWorkflowTemplate = () => {
    const baseX = 230;
    const baseY = 210;
    const nodes = BASIC_WORKFLOW_TEMPLATE.map((option, index) => ({
      id: `workflow-${Date.now()}-${index}`,
      kind: option.kind,
      title: option.title,
      subtitle: option.subtitle,
      x: baseX + (index % 3) * 230,
      y: baseY + Math.floor(index / 3) * 130,
    }));

    const connections = nodes.slice(0, -1).map((node, index) => ({
      id: `workflow-connection-${Date.now()}-${index}`,
      fromId: node.id,
      toId: nodes[index + 1].id,
    }));

    setCustomModules((current) => [...current, ...nodes]);
    setCustomConnections((current) => [...current, ...connections]);
  };


  const screenToWorld = (clientX: number, clientY: number, rect?: DOMRect) => {
    const canvasRect = rect ?? canvasRef.current?.getBoundingClientRect();
    if (!canvasRect) {
      return { worldX: 0, worldY: 0, screenX: 0, screenY: 0 };
    }

    const screenX = clientX - canvasRect.left;
    const screenY = clientY - canvasRect.top;
    return {
      screenX,
      screenY,
      worldX: (screenX - blueprintViewport.x) / blueprintViewport.scale,
      worldY: (screenY - blueprintViewport.y) / blueprintViewport.scale,
    };
  };

  const zoomBlueprint = (nextScale: number, anchorClientX?: number, anchorClientY?: number) => {
    const canvasRect = canvasRef.current?.getBoundingClientRect();
    const scale = clamp(nextScale, BLUEPRINT_MIN_SCALE, BLUEPRINT_MAX_SCALE);

    if (!canvasRect || anchorClientX === undefined || anchorClientY === undefined) {
      setBlueprintViewport((current) => ({ ...current, scale }));
      return;
    }

    setBlueprintViewport((current) => {
      const screenX = anchorClientX - canvasRect.left;
      const screenY = anchorClientY - canvasRect.top;
      const worldX = (screenX - current.x) / current.scale;
      const worldY = (screenY - current.y) / current.scale;
      return {
        scale,
        x: screenX - worldX * scale,
        y: screenY - worldY * scale,
      };
    });
  };

  const resetBlueprintView = () => {
    setBlueprintViewport({ x: 0, y: 0, scale: 1 });
  };

  const handleCanvasWheel = (event: WheelEvent<HTMLElement>) => {
    event.preventDefault();
    const direction = event.deltaY > 0 ? -1 : 1;
    zoomBlueprint(blueprintViewport.scale + direction * 0.08, event.clientX, event.clientY);
  };

  const handleCanvasContextMenu = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest("button, input, textarea, select, a")) {
      return;
    }

    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    const menuWidth = 150;
    const menuHeight = 226;
    const x = Math.min(Math.max(event.clientX - rect.left, 12), rect.width - menuWidth - 12);
    const y = Math.min(Math.max(event.clientY - rect.top, 12), rect.height - menuHeight - 12);
    const { worldX, worldY } = screenToWorld(event.clientX, event.clientY, rect);
    setBlueprintMenu({
      x,
      y,
      worldX: clamp(worldX, 12, BLUEPRINT_WORLD_WIDTH - BLUEPRINT_NODE_WIDTH),
      worldY: clamp(worldY, 12, BLUEPRINT_WORLD_HEIGHT - BLUEPRINT_NODE_HEIGHT),
    });
  };

  const addBlueprintModule = (option: (typeof BLUEPRINT_MODULE_OPTIONS)[number]) => {
    if (!blueprintMenu) {
      return;
    }

    setCustomModules((current) => [
      ...current,
      {
        id: `module-${Date.now()}-${current.length}`,
        kind: option.kind,
        title: option.title,
        subtitle: option.subtitle,
        x: blueprintMenu.worldX,
        y: blueprintMenu.worldY,
      },
    ]);
    setBlueprintMenu(null);
  };

  const removeBlueprintModule = (event: MouseEvent<HTMLButtonElement>, moduleId: string) => {
    event.stopPropagation();
    setCustomModules((current) => current.filter((module) => module.id !== moduleId));
    setCustomConnections((current) => current.filter((connection) => connection.fromId !== moduleId && connection.toId !== moduleId));
    setSelectedPort((current) => (current?.moduleId === moduleId ? null : current));
  };

  const toggleBlueprintPort = (
    event: MouseEvent<HTMLButtonElement>,
    moduleId: string,
    side: BlueprintPortSide,
  ) => {
    event.stopPropagation();
    setBlueprintMenu(null);

    if (!selectedPort) {
      setSelectedPort({ moduleId, side });
      return;
    }

    if (selectedPort.moduleId === moduleId && selectedPort.side === side) {
      setSelectedPort(null);
      return;
    }

    const fromId = selectedPort.side === "right" ? selectedPort.moduleId : moduleId;
    const toId = selectedPort.side === "right" ? moduleId : selectedPort.moduleId;

    if (fromId === toId) {
      setSelectedPort(null);
      return;
    }

    setCustomConnections((current) => {
      const exists = current.some((connection) => connection.fromId === fromId && connection.toId === toId);
      if (exists) {
        return current;
      }

      return [...current, { id: `connection-${Date.now()}-${current.length}`, fromId, toId }];
    });
    setSelectedPort(null);
  };

  const startModulePointer = (event: PointerEvent<HTMLElement>, module: BlueprintCustomModule) => {
    if (event.button !== 0) {
      return;
    }

    event.stopPropagation();
    setBlueprintMenu(null);
    const { worldX, worldY } = screenToWorld(event.clientX, event.clientY);
    const dragState: BlueprintDragState = {
      id: module.id,
      pointerId: event.pointerId,
      offsetX: worldX - module.x,
      offsetY: worldY - module.y,
    };

    event.currentTarget.setPointerCapture(event.pointerId);
    activeDragRef.current = dragState;
    setDraggingModuleId(module.id);
  };

  const moveModulePointer = (event: PointerEvent<HTMLElement>) => {
    const dragState = activeDragRef.current;
    if (!dragState || dragState.pointerId !== event.pointerId) {
      return;
    }

    event.preventDefault();
    const { worldX, worldY } = screenToWorld(event.clientX, event.clientY);
    const nextX = clamp(worldX - dragState.offsetX, 12, BLUEPRINT_WORLD_WIDTH - BLUEPRINT_NODE_WIDTH);
    const nextY = clamp(worldY - dragState.offsetY, 12, BLUEPRINT_WORLD_HEIGHT - BLUEPRINT_NODE_HEIGHT);

    setCustomModules((current) =>
      current.map((module) => (module.id === dragState.id ? { ...module, x: nextX, y: nextY } : module)),
    );
  };

  const endModulePointer = (event: PointerEvent<HTMLElement>) => {
    activeDragRef.current = null;
    setDraggingModuleId(null);

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const startCanvasPan = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return;
    }

    const target = event.target as HTMLElement;
    if (target.closest("button, input, textarea, select, a, .blueprint-custom-node")) {
      return;
    }

    setBlueprintMenu(null);
    panRef.current = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startX: blueprintViewport.x,
      startY: blueprintViewport.y,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    setIsPanningBlueprint(true);
  };

  const moveCanvasPan = (event: PointerEvent<HTMLElement>) => {
    const pan = panRef.current;
    if (!pan || pan.pointerId !== event.pointerId) {
      return;
    }

    event.preventDefault();
    setBlueprintViewport((current) => ({
      ...current,
      x: pan.startX + event.clientX - pan.startClientX,
      y: pan.startY + event.clientY - pan.startClientY,
    }));
  };

  const endCanvasPan = (event: PointerEvent<HTMLElement>) => {
    panRef.current = null;
    setIsPanningBlueprint(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  return (
    <div className="console-dashboard console-dashboard--canvas">
      {notice ? (
        <div className="console-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <span>{notice}</span>
        </div>
      ) : null}

      <section
        ref={canvasRef}
        className={`blueprint-canvas${isPanningBlueprint ? " is-panning" : ""}`}
        aria-label="主控蓝图画布"
        onClick={() => setBlueprintMenu(null)}
        onContextMenu={handleCanvasContextMenu}
        onPointerDown={startCanvasPan}
        onPointerMove={moveCanvasPan}
        onPointerUp={endCanvasPan}
        onPointerCancel={endCanvasPan}
        onPointerLeave={endCanvasPan}
        onWheel={handleCanvasWheel}
      >
        <div className="blueprint-canvas__tabs" aria-label="蓝图切换">
          <button type="button" className="is-active">
            架构蓝图
          </button>
          <button type="button" onClick={() => onNavigate("workflow")}>
            业务蓝图
          </button>
        </div>

        <div className="blueprint-canvas__toolbar" aria-label="画布工具">
          <button type="button" aria-label="放大" title="放大" onClick={() => zoomBlueprint(blueprintViewport.scale + 0.12)}>
            <Plus size={16} aria-hidden="true" />
          </button>
          <button type="button" aria-label="缩小" title="缩小" onClick={() => zoomBlueprint(blueprintViewport.scale - 0.12)}>
            <Minus size={16} aria-hidden="true" />
          </button>
          <button type="button" aria-label="适应画布" title="适应画布" onClick={resetBlueprintView}>
            <Maximize2 size={15} aria-hidden="true" />
          </button>
          <button type="button" aria-label="刷新" title="刷新" onClick={() => onRefresh?.()}>
            <RefreshCw size={15} aria-hidden="true" />
          </button>
        </div>

        <aside className="blueprint-canvas__minimap" aria-label="蓝图小地图">
          <span />
          <i />
        </aside>

        <div className="blueprint-zoom-badge">{Math.round(blueprintViewport.scale * 100)}%</div>

        <div className="blueprint-canvas__summary">
          <div>
            <span>{connectionLabel}</span>
            <strong>{missionText}</strong>
          </div>
          <button type="button" onClick={() => onNavigate("workflow")}>
            看完整流程
          </button>
        </div>

        <div
          className="blueprint-viewport"
          style={{
            width: BLUEPRINT_WORLD_WIDTH,
            height: BLUEPRINT_WORLD_HEIGHT,
            transform: `translate(${blueprintViewport.x}px, ${blueprintViewport.y}px) scale(${blueprintViewport.scale})`,
          }}
        >
          <svg className="blueprint-canvas__wires" viewBox="0 0 1180 680" aria-hidden="true">
            {customConnections.map((connection) => {
              const from = customModules.find((module) => module.id === connection.fromId);
              const to = customModules.find((module) => module.id === connection.toId);
              if (!from || !to) {
                return null;
              }

              return <path className="is-custom" d={getConnectionPath(from, to)} key={connection.id} />;
            })}
          </svg>

          <article className="blueprint-node blueprint-node--starter">
          <div className="blueprint-node__ports is-left">
            <i className="port port-blue" />
            <i className="port port-amber" />
          </div>
          <div className="blueprint-node__badge">Basic</div>
          <div className="blueprint-node__head">
            <span className="blueprint-node__icon">
              <Workflow size={16} aria-hidden="true" />
            </span>
            <div>
              <h2>基础工作流</h2>
              <p>开始到汇总的最小闭环</p>
            </div>
          </div>
          <div className="blueprint-basic-flow">
            {BASIC_WORKFLOW_TEMPLATE.map((option, index) => (
              <button type="button" key={option.kind} onClick={() => addBlueprintModuleFromLibrary(option, index)}>
                <span>{getBlueprintModuleIcon(option.kind)}</span>
                <div>
                  <strong>{index + 1}. {option.title}</strong>
                  <small>{option.subtitle}</small>
                </div>
              </button>
            ))}
          </div>
          <div className="blueprint-starter-actions">
            <button type="button" onClick={addBasicWorkflowTemplate}>添加整套流程</button>
            <span>单点可加一个节点，按钮可加整条最小工作流。</span>
          </div>
          <div className="blueprint-node__ports is-right">
            <i className="port port-blue" />
            <i className="port port-amber" />
            <i className="port port-slate" />
          </div>
          </article>

          {customModules.map((module, index) => (
            <article
              className={`blueprint-custom-node blueprint-custom-node--${module.kind}${draggingModuleId === module.id ? " is-dragging" : ""}`}
              key={module.id}
              style={{ left: module.x, top: module.y }}
              onPointerDown={(event) => startModulePointer(event, module)}
              onPointerMove={moveModulePointer}
              onPointerUp={endModulePointer}
              onPointerCancel={endModulePointer}
            >
              <button
                type="button"
                className={`port-button port-button--left${selectedPort?.moduleId === module.id && selectedPort.side === "left" ? " is-selected" : ""}`}
                aria-label={`${module.title} 输入端口`}
                onPointerDown={(event) => event.stopPropagation()}
                onClick={(event) => toggleBlueprintPort(event, module.id, "left")}
              />
              <span className="blueprint-custom-node__icon">{getBlueprintModuleIcon(module.kind)}</span>
              <div>
                <h2>{index + 1}. {module.title}</h2>
                <p>{module.subtitle}</p>
              </div>
              <small>右键添加的临时模块</small>
              <button
                type="button"
                className="blueprint-custom-node__delete"
                aria-label={`删除${module.title}`}
                title="删除模块"
                onPointerDown={(event) => event.stopPropagation()}
                onClick={(event) => removeBlueprintModule(event, module.id)}
              >
                <Trash2 size={12} aria-hidden="true" />
              </button>
              <button
                type="button"
                className={`port-button port-button--right${selectedPort?.moduleId === module.id && selectedPort.side === "right" ? " is-selected" : ""}`}
                aria-label={`${module.title} 输出端口`}
                onPointerDown={(event) => event.stopPropagation()}
                onClick={(event) => toggleBlueprintPort(event, module.id, "right")}
              />
            </article>
          ))}
        </div>

        <aside className="blueprint-inspector">
          <div className="blueprint-inspector__button">
            <Sparkles size={18} aria-hidden="true" />
          </div>
          <div className="blueprint-inspector__button">
            <span>文</span>
          </div>
        </aside>

        <div className="blueprint-command" aria-label="主控输入区">
          <div className="blueprint-chat-panel" aria-label="总控对话记录">
            {controllerMessages.slice(-4).map((message) => (
              <div className={`blueprint-chat-bubble is-${message.role}`} key={message.id}>
                <span>{message.role === "user" ? "你" : "总控"}</span>
                <p>{message.text}</p>
              </div>
            ))}
          </div>
          <MessageSquareText size={19} aria-hidden="true" />
          <input
            value={intakeIdea}
            maxLength={1000}
            placeholder="问总控：这个模块怎么用？或者说：生成一个基础工作流"
            onChange={(event) => setIntakeIdea(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void handleControllerChat();
              }
            }}
          />
          <button type="button" disabled={controllerThinking} onClick={() => void handleControllerChat()}>
            {controllerThinking ? "思考中" : "问总控"}
          </button>
          <button type="button" disabled={autoRunning} onClick={() => void handleAutoRunIntake()}>
            {autoRunning ? "蜂群执行中" : "全自动执行"}
          </button>
        </div>
        {intakeError ? <div className="blueprint-error">{intakeError}</div> : null}

        {blueprintMenu ? (
          <div
            className="blueprint-context-menu"
            style={{ left: blueprintMenu.x, top: blueprintMenu.y }}
            role="menu"
            aria-label="添加画布模块"
            onClick={(event) => event.stopPropagation()}
          >
            <strong>节点</strong>
            {BLUEPRINT_MODULE_OPTIONS.map((option) => (
              <button type="button" key={option.kind} onClick={() => addBlueprintModule(option)}>
                <span>{getBlueprintModuleIcon(option.kind)}</span>
                <div>
                  <b>{option.title}</b>
                  <small>{option.subtitle}</small>
                </div>
              </button>
            ))}
          </div>
        ) : null}
      </section>
    </div>
  );
}
