/**
 * ConsoleDashboardPage — Agent 蜂群对话式主控台。
 *
 * ## 阶段状态
 * 本组件是"前端 UI 主控台与蜂群工作流迁移计划"第一版的主控台实现，
 * 从 design/agent-console-progress.html 设计原型（纯 HTML/CSS 视觉稿）
 * 转化为 React + TypeScript + Ant Design + CSS Variables 的生产组件。
 *
 * - design/agent-console-progress.html：设计阶段的高保真视觉原型（静态 HTML）。
 * - ConsoleDashboardPage.tsx（本文件）：生产代码，接入真实 overview 数据。
 *
 * ## 布局
 * 左侧 Agent 进度卡 → 中间对话式主控区 → 右侧安全仪表
 * 顶部薄态势栏（不抢占对话区注意力）
 *
 * ## 安全约束
 * 本阶段不接入写入按钮、不创建任务、不触发 Runner、不绕过审批链。
 */

import { useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  Bot,
  CheckCircle2,
  Clock3,
  FileText,
  LockKeyhole,
  MessageSquareText,
  ShieldCheck,
  Sparkles,
} from "lucide-react";

import type {
  AgentSummary,
  ApprovalSummary,
  ClassifyProjectIntakeResponse,
  ProjectSummary,
  TaskSummary,
  TaskStatus,
} from "@agent-swarm/shared";
import type { PageKey } from "../routes/mainNavItems";
import type { IdeaGuidanceHandoff } from "../components/IdeaGuidancePanel";
import { roleLabel, statusLabel, agentNameLabel } from "../utils/labels";
import { autoGenerateProjectPlanTasks, autoRunSwarmIdea, classifyProjectIntake, isTauriHost } from "../utils/desktopHost";
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

type ControllerAssignment = {
  title: string;
  owner: string;
  description: string;
};

const IDEA_GUIDANCE_HANDOFF_KEY = "agent_swarm_idea_guidance_handoff";

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

function getAgentInitials(name: string): string {
  const cleanedName = name.replace(/^示例\s*Agent[:：]?/, "").trim();
  return cleanedName.slice(0, 2) || "AI";
}

function getRoleLabel(role: string): string {
  return roleLabel(role);
}

function getSafetyLabel(connectionStatus: ConsoleDashboardPageProps["connectionStatus"]): string {
  if (connectionStatus === "connected") {
    return "锁定";
  }

  if (connectionStatus === "error") {
    return "降级";
  }

  return "只读";
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

function getGuidanceText(tasks: TaskSummary[], pendingApprovalCount: number): string {
  if (pendingApprovalCount > 0) {
    return "建议先处理审批闸门，确认后再让智能体继续推进。";
  }

  const blockedTask = tasks.find((task) => task.status === "blocked" || task.status === "failed");
  if (blockedTask) {
    return `发现阻塞任务：${blockedTask.title}。建议先复盘原因，再决定恢复、改派或取消。`;
  }

  const runningTask = tasks.find((task) => task.status === "running" || task.status === "waiting_user");
  if (runningTask) {
    return `当前重点是 ${runningTask.title}。我会优先展示相关智能体、审批和安全状态。`;
  }

  if (tasks.length > 0) {
    return "当前没有正在执行的任务，可以从排队任务中选择一个继续推进。";
  }

  return "当前还没有真实任务。你可以先进入项目计划或蜂群工作流，把想法拆成任务。";
}

function getControllerAssignments(
  projectType?: ClassifyProjectIntakeResponse["session"]["project_type"],
): ControllerAssignment[] {
  if (projectType === "software_product") {
    return [
      { title: "需求澄清", owner: "产品智能体", description: "把目标用户、核心问题和第一版范围问清楚。" },
      { title: "界面方案", owner: "前端智能体", description: "整理主界面、交互路径和可点击原型方向。" },
      { title: "实现拆解", owner: "后端智能体", description: "拆分数据、接口、桌面宿主和安全边界。" },
      { title: "验收检查", owner: "审查智能体", description: "检查范围、风险、测试和文档是否闭环。" },
    ];
  }

  if (projectType === "ai_automation") {
    return [
      { title: "流程建模", owner: "计划智能体", description: "明确输入、输出、触发条件和失败处理。" },
      { title: "模型选择", owner: "模型智能体", description: "从受控模型目录选择适合每个角色的模型。" },
      { title: "权限评估", owner: "安全智能体", description: "标出哪些动作能自动做，哪些必须确认。" },
      { title: "预演方案", owner: "执行智能体", description: "先生成只读预演，不直接执行真实动作。" },
    ];
  }

  if (projectType === "content_creation") {
    return [
      { title: "定位澄清", owner: "内容策划智能体", description: "确认受众、主题、风格和产出节奏。" },
      { title: "选题拆解", owner: "文档智能体", description: "生成栏目、脚本、素材清单和第一批选题。" },
      { title: "质量审查", owner: "审查智能体", description: "检查表达、风险词和是否偏离目标。" },
      { title: "发布准备", owner: "执行智能体", description: "只整理清单，不自动发布或上传。" },
    ];
  }

  if (projectType === "business_plan") {
    return [
      { title: "商业假设", owner: "产品智能体", description: "提炼客户、痛点、价值主张和 MVP 假设。" },
      { title: "竞品复盘", owner: "研究智能体", description: "整理替代方案、差异点和验证路径。" },
      { title: "落地路线", owner: "计划智能体", description: "拆出一周、两周、一个月的推进计划。" },
      { title: "风险审查", owner: "审查智能体", description: "检查成本、权限、数据和执行边界。" },
    ];
  }

  return [
    { title: "目标识别", owner: "总控智能体", description: "先判断你是在做产品、自动化、内容、方案还是通用目标。" },
    { title: "关键追问", owner: "想法引导智能体", description: "把模糊想法变成可执行的问题清单。" },
    { title: "任务分派", owner: "计划智能体", description: "根据类型把事情分给对应角色智能体。" },
    { title: "安全闸门", owner: "审查智能体", description: "所有执行、写文件和改 Git 都必须走受控链路。" },
  ];
}

function classifyPreview(idea: string): ClassifyProjectIntakeResponse {
  const normalized = idea.trim().split(/\s+/).join(" ");
  const lower = normalized.toLowerCase();
  const has = (words: string[]) => words.some((word) => lower.includes(word));

  let label = "通用目标";
  let projectType: ClassifyProjectIntakeResponse["session"]["project_type"] = "general_goal";
  let reason = "当前想法还比较开放，暂时无法稳定归入具体项目类型。";
  let confidence = 45;
  let questions = [
    "你最终想得到一个工具、内容、方案，还是一个长期系统？",
    "这个想法主要服务你自己，还是服务其他用户？",
    "你最想先解决的一个具体问题是什么？",
    "有什么明确不能做、不能碰或不想投入的边界？",
    "如果一周内看到第一版，你希望它长什么样？",
  ];

  if (has(["网站", "网页", "app", "应用", "软件", "系统", "小程序", "桌面端", "页面", "功能"])) {
    label = "软件产品";
    projectType = "software_product";
    confidence = 76;
    reason = "你的想法里出现了网站、应用、系统、页面或功能等软件产品信号。";
    questions = [
      "目标用户是谁？他们现在最痛的地方是什么？",
      "第一版必须解决哪一个核心问题？",
      "你希望它运行在桌面端、网页、移动端，还是多端？",
      "第一版必须有哪 3 个功能？哪些明确不做？",
      "你希望多久看到可用的第一版？",
    ];
  } else if (has(["ai", "智能体", "自动", "自动化", "脚本", "批量", "工作流", "爬虫", "整理"])) {
    label = "AI 自动化";
    projectType = "ai_automation";
    confidence = 76;
    reason = "你的想法里出现了自动、脚本、批量、工作流或智能体等自动化信号。";
    questions = [
      "这个自动化的输入是什么？",
      "你希望最终输出什么结果？",
      "它应该由什么事件触发？",
      "哪些动作有风险，必须人工确认？",
      "第一版只跑在本机，还是需要和外部服务连接？",
    ];
  } else if (has(["视频", "短视频", "小说", "文案", "课程", "账号", "脚本", "选题", "内容"])) {
    label = "内容创作";
    projectType = "content_creation";
    confidence = 76;
    reason = "你的想法里出现了视频、小说、文案、课程、账号或选题等内容创作信号。";
    questions = [
      "内容面向谁？他们为什么会关注？",
      "你要做什么主题或系列？",
      "内容形式是文章、短视频、课程、小说，还是混合？",
      "你想要什么风格？",
      "第一周要产出哪些具体内容？",
    ];
  } else if (has(["商业", "创业", "产品", "方案", "用户", "客户", "竞品", "市场", "盈利", "mvp", "立项"])) {
    label = "商业方案";
    projectType = "business_plan";
    confidence = 76;
    reason = "你的想法里出现了创业、产品、用户、竞品、商业模式或 MVP 等商业方案信号。";
    questions = [
      "目标客户是谁？他们愿意为什么付费？",
      "你解决的痛点是否足够高频或高价值？",
      "现有竞品或替代方案是什么？",
      "第一版 MVP 如何验证需求？",
      "你能投入多少时间、预算和资源？",
    ];
  }

  const now = new Date().toISOString();
  return {
    session: {
      id: `preview_${Date.now()}`,
      project_id: "browser_preview",
      raw_idea: idea,
      normalized_idea: normalized,
      project_type: projectType,
      project_type_label: label,
      confidence,
      reason,
      recommended_questions: questions,
      recommended_next_step: "总控先完成澄清和分流，再把任务分配给对应智能体。",
      status: "classified",
      created_by: "browser_preview",
      created_at: now,
      updated_at: now,
    },
    side_effects: {
      calls_real_model: false,
      creates_tasks: false,
      creates_approvals: false,
      executes_runner: false,
      writes_project_files: false,
      modifies_git: false,
    },
  };
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
  const activeAgents = agentProgress.filter((item) => item.state === "running" || item.state === "waiting").length;
  const pendingApprovalCount = approvals.filter((approval) => approval.status === "pending").length;
  const runningTaskCount = tasks.filter((task) => task.status === "running" || task.status === "waiting_user").length;
  const doneTaskCount = tasks.filter((task) => task.status === "completed").length;
  const totalProgress = tasks.length > 0 ? Math.round((doneTaskCount / tasks.length) * 100) : 0;
  const latestTask = tasks.find((task) => task.status === "running") ?? tasks[0] ?? null;
  const missionText = getMissionText(project, tasks, pendingApprovalCount);
  const guidanceText = getGuidanceText(tasks, pendingApprovalCount);
  const connectionLabel = getConnectionLabel(connectionStatus);
  const [intakeIdea, setIntakeIdea] = useState("");
  const [intakeResult, setIntakeResult] = useState<ClassifyProjectIntakeResponse | null>(null);
  const [intakeError, setIntakeError] = useState<string | null>(null);
  const [intakeLoading, setIntakeLoading] = useState(false);
  const controllerAssignments = getControllerAssignments(intakeResult?.session.project_type);
  const notice =
    connectionStatus === "error"
      ? (message ?? "桌面宿主连接失败，当前展示只读示例数据。")
      : connectionStatus === "browser"
        ? "当前是浏览器预览模式，页面展示示例数据；打开桌面端后会读取真实项目、任务、智能体和审批。"
        : null;

  const handleClassifyIntake = async () => {
    const idea = intakeIdea.trim();
    if (!idea) {
      setIntakeError("先写一句你想做什么。");
      return;
    }

    setIntakeError(null);
    setIntakeLoading(true);
    try {
      if (isTauriHost()) {
        const input = {
          idea,
          constraints: "全自动生成角色任务、执行单，并自动推进到最小执行记录；后续再逐步开放真实写文件、命令和 Git。",
          requested_by: "swarm_auto",
        };
        try {
          await autoRunSwarmIdea(input);
        } catch (autoError) {
          const autoMessage = String(autoError);
          if (!autoMessage.includes("auto_run_swarm_idea") && !autoMessage.includes("Command")) {
            throw autoError;
          }
          await autoGenerateProjectPlanTasks(input);
        }
        setIntakeIdea("");
        onRefresh?.();
        onNavigate("workflow");
        return;
      }

      const result = isTauriHost()
        ? await classifyProjectIntake({ idea })
        : classifyPreview(idea);
      setIntakeResult(result);
    } catch (error) {
      setIntakeError(userErrorLabel(error, "想法分流失败，请稍后重试"));
    } finally {
      setIntakeLoading(false);
    }
  };

  const handleOpenGuidance = () => {
    if (intakeResult) {
      const handoff: IdeaGuidanceHandoff = {
        idea: intakeIdea.trim() || intakeResult.session.normalized_idea,
        projectTypeLabel: intakeResult.session.project_type_label,
        reason: intakeResult.session.reason,
        questions: intakeResult.session.recommended_questions,
      };
      window.sessionStorage.setItem(IDEA_GUIDANCE_HANDOFF_KEY, JSON.stringify(handoff));
    }
    onNavigate("projectPlan");
  };

  return (
    <div className="console-dashboard">
      {notice ? (
        <div className="console-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <span>{notice}</span>
        </div>
      ) : null}

      <section className="console-status-strip" aria-label="当前任务态势">
        <article className="console-mission-card">
          <span>{connectionLabel}</span>
          <strong>{missionText}</strong>
        </article>
        <article className="console-metric-card is-safe">
          <span>安全等级</span>
          <strong>{getSafetyLabel(connectionStatus)}</strong>
        </article>
        <article className="console-metric-card">
          <span>活跃智能体</span>
          <strong>{activeAgents} / {agents.length}</strong>
        </article>
        <article className="console-metric-card">
          <span>待确认</span>
          <strong>{pendingApprovalCount}</strong>
        </article>
        <article className="console-metric-card">
          <span>项目推进</span>
          <strong>{totalProgress}%</strong>
        </article>
      </section>

      <div className="console-grid">
        <aside className="agent-rail" aria-label="智能体工作进度">
          <div className="panel-heading">
            <span>智能体队列</span>
            <strong>角色进度</strong>
          </div>
          <div className="agent-progress-list">
            {agentProgress.map((item) => (
              <article className={`agent-progress-card tone-${item.tone}`} key={item.agent.id}>
                <div className="agent-progress-card__top">
                  <div className="agent-avatar">{getAgentInitials(item.agent.name)}</div>
                  <div>
                    <h2>{agentNameLabel(item.agent.name)}</h2>
                    <p>{getRoleLabel(item.agent.role)}</p>
                  </div>
                  <span className="agent-state">{item.stateLabel}</span>
                </div>
                <div className="agent-progress-card__task">
                  {item.task ? item.task.title : "当前无任务，等待总控分配下一步。"}
                </div>
                <div className="progress-track" aria-label={`${item.agent.name} 进度 ${item.progress}%`}>
                  <span style={{ width: `${item.progress}%` }} />
                </div>
                <div className="agent-progress-card__meta">
                  <span>{item.progress}%</span>
                  <span>{item.agent.model ?? "受控模型目录"}</span>
                </div>
              </article>
            ))}
          </div>
        </aside>

        <main className="conversation-stage" aria-label="主控台对话流">
          <div className="panel-heading">
            <span>总控智能体</span>
            <strong>以后只在这里下达目标</strong>
          </div>

          <div className="chat-thread">
            <article className="chat-bubble chat-bubble--user">
              <span>你</span>
              <p>我只想在这里和最高权限的总控智能体对话，由它帮我调度其他智能体。</p>
            </article>
            <article className="chat-bubble chat-bubble--agent">
              <span>主控智能体</span>
              <p>
                可以。这里会成为唯一主入口。你只说目标，我先识别项目类型、追问关键问题，再把任务、模型、智能体和执行权限分发到后面的受控链路里。
              </p>
            </article>
            <article className="controller-flow-card" aria-label="总控处理链路">
              <span>总控处理链路</span>
              <div className="controller-flow-card__steps">
                {["听懂目标", "澄清问题", "分配智能体", "进入受控执行"].map((step, index) => (
                  <div className="controller-flow-card__step" key={step}>
                    <strong>{step}</strong>
                    {index < 3 ? <ArrowRight size={15} aria-hidden="true" /> : null}
                  </div>
                ))}
              </div>
            </article>
            <article className="console-artifact">
              <div>
                <span>项目快照</span>
                <strong>{project.name}</strong>
              </div>
              <p>
                当前阶段：{project.phase}。数据来源：{connectionLabel}。主控台会根据任务、智能体和审批状态推导当前态势。
              </p>
              <div className="artifact-metrics">
                <span>任务 {tasks.length}</span>
                <span>智能体 {agents.length}</span>
                <span>待审 {pendingApprovalCount}</span>
              </div>
            </article>
            <article className="chat-bubble chat-bubble--agent">
              <span>执行观察员</span>
              <p>
                {runningTaskCount > 0
                  ? `当前有 ${runningTaskCount} 个任务处于推进中，右侧会持续显示安全边界和待确认动作。`
                  : guidanceText}
              </p>
            </article>
          </div>

          <div className="command-box" aria-label="主控输入区">
            <MessageSquareText size={19} aria-hidden="true" />
            <input
              value={intakeIdea}
              maxLength={1000}
              placeholder="对总控智能体说：我想做什么？"
              onChange={(event) => setIntakeIdea(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  void handleClassifyIntake();
                }
              }}
            />
            <button type="button" disabled={intakeLoading} onClick={() => void handleClassifyIntake()}>
              {intakeLoading ? "总控梳理中" : "发送给总控"}
            </button>
          </div>
          {intakeError ? <div className="intake-error">{intakeError}</div> : null}
          {intakeResult ? (
            <article className="intake-card">
              <div className="intake-card__header">
                <span>项目类型识别</span>
                <strong>{intakeResult.session.project_type_label}</strong>
                <em>{intakeResult.session.confidence}%</em>
              </div>
              <p>{intakeResult.session.reason}</p>
              <div className="intake-card__questions">
                {intakeResult.session.recommended_questions.map((question, index) => (
                  <span key={question}>{index + 1}. {question}</span>
                ))}
              </div>
              <div className="controller-assignment-card">
                <span>总控分配预案</span>
                <div>
                  {controllerAssignments.map((item) => (
                    <section key={`${item.owner}-${item.title}`}>
                      <strong>{item.title}</strong>
                      <em>{item.owner}</em>
                      <p>{item.description}</p>
                    </section>
                  ))}
                </div>
              </div>
              <div className="intake-card__footer">
                <span>{intakeResult.session.recommended_next_step}</span>
                <small>总控第一步只做分流和澄清；后续会把任务分配下去，但当前不会执行、不写文件、不改 Git。</small>
                <button type="button" onClick={handleOpenGuidance}>
                  让总控进入澄清流程
                </button>
              </div>
            </article>
          ) : null}
        </main>

        <aside className="safety-inspector" aria-label="项目与安全状态">
          <div className="panel-heading">
            <span>安全仪表</span>
            <strong>状态总览</strong>
          </div>

          <section className="instrument-card">
            <div className="instrument-card__icon">
              <ShieldCheck size={18} aria-hidden="true" />
            </div>
            <div>
              <h2>执行边界</h2>
              <p>沙箱 + 版本管理只读 + 二次确认</p>
            </div>
            <i className="instrument-card__lamp" />
          </section>

          <section className="instrument-card">
            <div className="instrument-card__icon is-warning">
              <LockKeyhole size={18} aria-hidden="true" />
            </div>
            <div>
              <h2>审批闸门</h2>
              <p>{pendingApprovalCount > 0 ? `${pendingApprovalCount} 个动作等待确认` : "暂无待确认动作"}</p>
            </div>
            <i className={pendingApprovalCount > 0 ? "instrument-card__lamp is-warning" : "instrument-card__lamp"} />
          </section>

          <section className="instrument-card">
            <div className="instrument-card__icon">
              <Sparkles size={18} aria-hidden="true" />
            </div>
            <div>
              <h2>模型目录</h2>
              <p>仅允许受控模型选择</p>
            </div>
            <i className="instrument-card__lamp" />
          </section>

          <section className="instrument-card">
            <div className="instrument-card__icon">
              <FileText size={18} aria-hidden="true" />
            </div>
            <div>
              <h2>文件变更</h2>
              <p>当前无写入动作</p>
            </div>
            <i className="instrument-card__lamp" />
          </section>

          <section className="next-step-card">
            <span>下一步建议</span>
            <p>{latestTask ? latestTask.title : "先优化外围视觉壳，不接入真实执行按钮；等你确认后再逐步组件化。"}</p>
          </section>

          <section className="activity-stack" aria-label="近期活动">
            <div className="activity-item">
              <CheckCircle2 size={16} aria-hidden="true" />
              <span>想法引导官已完成项目种子梳理</span>
            </div>
            <div className="activity-item">
              <Bot size={16} aria-hidden="true" />
              <span>智能体进度条已接入任务状态推导</span>
            </div>
            <div className="activity-item">
              <Clock3 size={16} aria-hidden="true" />
              <span>真实执行仍等待人工确认</span>
            </div>
          </section>
        </aside>
      </div>
    </div>
  );
}
