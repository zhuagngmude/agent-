const projectId = "project_agent_swarm";

const status = {
  approval: {
    draft: { label: "草稿", tone: "neutral", action: "继续编辑" },
    pending: { label: "等待审批", tone: "warn", action: "进入审查" },
    approved: { label: "已批准", tone: "ok", action: "等待执行" },
    rejected: { label: "已拒绝", tone: "danger", action: "查看原因" },
    patch_only: { label: "只生成补丁", tone: "neutral", action: "下载补丁" },
    executed: { label: "已执行", tone: "ok", action: "查看结果" },
    rolled_back: { label: "已回滚", tone: "danger", action: "查看回滚" },
    expired: { label: "已过期", tone: "neutral", action: "重新申请" },
  },
  task: {
    queued: { label: "排队中", tone: "neutral" },
    running: { label: "进行中", tone: "ok" },
    blocked: { label: "已阻塞", tone: "danger" },
    waiting_user: { label: "等待用户", tone: "warn" },
    completed: { label: "已完成", tone: "ok" },
    failed: { label: "失败", tone: "danger" },
    cancelled: { label: "已取消", tone: "neutral" },
  },
  agent: {
    running: { label: "运行中", tone: "ok" },
    idle: { label: "空闲中", tone: "neutral" },
    waiting: { label: "等待中", tone: "warn" },
    failed: { label: "异常", tone: "danger" },
    disabled: { label: "已禁用", tone: "neutral" },
  },
};

const project = {
  id: projectId,
  name: "agent蜂群 MVP",
  status: "running",
  phase: "MVP-0.4",
  description: "多 AI 智能体协作调度系统，优先打磨任务流、审批确认和知识库联动。",
};

const agents = [
  {
    id: "agent_architect",
    name: "架构师 Agent",
    role: "architect",
    status: "running",
    version: "v0.3.0",
    model: "gpt-high-reasoning",
    canSpawnSubAgents: true,
    maxSubAgents: 5,
    parentAgentId: "",
    childAgentIds: ["agent_frontend", "agent_backend", "agent_qa", "agent_docs", "agent_reviewer"],
    reportsToAgentId: "",
    spawnDepth: 0,
    permissions: ["read_project", "plan_tasks", "review_architecture"],
  },
  {
    id: "agent_frontend",
    name: "前端 Agent",
    role: "frontend",
    status: "running",
    version: "v0.2.0",
    model: "claude-ui",
    canSpawnSubAgents: false,
    maxSubAgents: 0,
    parentAgentId: "agent_architect",
    childAgentIds: [],
    reportsToAgentId: "agent_architect",
    spawnDepth: 1,
    permissions: ["read_project", "write_frontend_patch"],
  },
  {
    id: "agent_backend",
    name: "Backend Agent",
    role: "backend",
    status: "running",
    version: "v0.3.0",
    model: "gpt-api",
    canSpawnSubAgents: false,
    maxSubAgents: 0,
    parentAgentId: "agent_architect",
    childAgentIds: [],
    reportsToAgentId: "agent_architect",
    spawnDepth: 1,
    permissions: ["read_project", "draft_api_contract", "write_backend_patch"],
  },
  {
    id: "agent_qa",
    name: "QA Agent",
    role: "qa",
    status: "idle",
    version: "v0.3.0",
    model: "gpt-qa",
    canSpawnSubAgents: false,
    maxSubAgents: 0,
    parentAgentId: "agent_architect",
    childAgentIds: [],
    reportsToAgentId: "agent_architect",
    spawnDepth: 1,
    permissions: ["read_project", "draft_acceptance_checks", "verify_no_side_effects"],
  },
  {
    id: "agent_docs",
    name: "文档 Agent",
    role: "docs",
    status: "idle",
    version: "v0.1.5",
    model: "gpt-docs",
    canSpawnSubAgents: false,
    maxSubAgents: 0,
    parentAgentId: "agent_architect",
    childAgentIds: [],
    reportsToAgentId: "agent_architect",
    spawnDepth: 1,
    permissions: ["read_project", "write_docs_patch"],
  },
  {
    id: "agent_reviewer",
    name: "审查 Agent",
    role: "reviewer",
    status: "running",
    version: "v0.1.8",
    model: "gemini-long-context",
    canSpawnSubAgents: false,
    maxSubAgents: 0,
    parentAgentId: "agent_architect",
    childAgentIds: [],
    reportsToAgentId: "agent_architect",
    spawnDepth: 1,
    permissions: ["read_project", "review_risk", "review_diff"],
  },
];

const tasks = [
  {
    id: "task_frontend_mock_data",
    title: "抽出前端 mock 数据模型",
    description: "将首页和关键模块数据从 HTML 中抽离，为后续 API 接入留出稳定数据结构。",
    status: "completed",
    priority: "high",
    assignedAgentId: "agent_frontend",
    riskLevel: "low",
    relatedFiles: ["apps/web/data.js", "apps/web/app.js"],
    requiresApproval: false,
    dependsOn: [],
  },
  {
    id: "task_runner_approval_page",
    title: "打磨 Runner 审批确认页",
    description: "完善 Runner 执行前的审查详情、差异预览、拒绝和只生成补丁流程。",
    status: "running",
    priority: "high",
    assignedAgentId: "agent_frontend",
    riskLevel: "high",
    relatedFiles: ["apps/web/index.html", "apps/web/app.js", "apps/web/styles.css"],
    requiresApproval: true,
    dependsOn: ["task_frontend_mock_data"],
  },
  {
    id: "task_api_contract",
    title: "写 API 草案",
    description: "定义 Dashboard、任务、审批、Runner、知识库等模块的第一版接口边界。",
    status: "completed",
    priority: "medium",
    assignedAgentId: "agent_architect",
    riskLevel: "low",
    relatedFiles: ["docs/api-draft.md"],
    requiresApproval: false,
    dependsOn: [],
  },
  {
    id: "task_task_state_api",
    title: "任务管理接入 Mock API 状态机",
    description: "让任务页支持开始、完成、失败、取消，并把状态写入本地 runtime-state。",
    status: "queued",
    priority: "high",
    assignedAgentId: "agent_architect",
    riskLevel: "medium",
    relatedFiles: ["services/api/server.js", "services/api/mock-data.js", "apps/web/app.js"],
    requiresApproval: false,
    dependsOn: ["task_api_contract"],
  },
];

const approvals = [
  {
    id: "approval_runner_permissions",
    status: "pending",
    riskLevel: "high",
    riskTone: "high",
    requestAgentId: "agent_architect",
    requestAgentName: "架构师 Agent",
    operationTypes: ["file_write", "git_checkpoint", "audit_log_update"],
    reason: "整理 Runner 安全验收标准，确保真实执行前必须经过审批、二次确认和 Git checkpoint。",
    checkpoint: {
      required: true,
      created: true,
      commit: "b84cf43",
    },
    affectedFiles: ["docs/runner-safety-acceptance.md", "docs/api-draft.md", "dev-docs/AI开发维护手册.md"],
    diffSummary: "3 files",
    diffPreview: [
      "+ Runner 真实执行前必须满足 Approval Service 放行。",
      "+ 高风险操作必须二次确认并创建 Git checkpoint。",
      "+ 当前 Demo 仍不执行本地命令、不写文件。",
    ],
    requiresSecondConfirm: true,
    createdAt: "2026-06-08T12:00:00Z",
  },
  {
    id: "approval_docs_safety",
    status: "pending",
    riskLevel: "medium",
    riskTone: "mid",
    requestAgentId: "agent_docs",
    requestAgentName: "文档 Agent",
    operationTypes: ["file_write"],
    reason: "补充 Runner 审批规则，让后续 AI 接手时知道安全边界。",
    checkpoint: {
      required: true,
      created: true,
      commit: "b84cf43",
    },
    affectedFiles: ["dev-docs/AI开发维护手册.md"],
    diffSummary: "1 file",
    diffPreview: [
      "+ 所有本地写文件、删文件、执行命令都必须经过 Approval Service。",
      "+ 高风险操作必须二次确认。",
    ],
    requiresSecondConfirm: false,
    createdAt: "2026-06-08T12:05:00Z",
  },
  {
    id: "approval_runner_tests",
    status: "patch_only",
    riskLevel: "low",
    riskTone: "low",
    requestAgentId: "agent_reviewer",
    requestAgentName: "审查 Agent",
    operationTypes: ["file_write"],
    reason: "为 Mock API 状态流转增加回归验证，避免后续破坏任务、审批和 Agent 配置流程。",
    checkpoint: {
      required: true,
      created: true,
      commit: "2670959",
    },
    affectedFiles: ["scripts/verify-mock-flows.ps1"],
    diffSummary: "1 script",
    diffPreview: [
      "+ 验证任务 start -> complete 状态流转。",
      "+ 验证审批批准后只生成只读 Runner job。",
      "+ 验证 Agent 配置应用/取消 Mock 状态流转。",
    ],
    requiresSecondConfirm: false,
    createdAt: "2026-06-08T12:10:00Z",
  },
];

function cloneValue(value) {
  return JSON.parse(JSON.stringify(value));
}

const initialApprovalState = approvals.map((approval) => cloneValue(approval));
const initialTaskState = tasks.map((task) => cloneValue(task));

const workflows = [
  {
    id: "workflow_runner_safe_execute",
    name: "Runner 安全执行流程",
    status: "active",
    description: "从执行计划、用户审批、Git checkpoint 到本地 Runner 执行的受控链路。",
    steps: [
      { id: "step_plan", name: "生成执行计划", detail: "架构师 Agent 输出变更范围", progress: "100%", tone: "purple" },
      { id: "step_review", name: "风险审查", detail: "审查 Agent 检查权限与影响文件", progress: "86%", tone: "blue" },
      { id: "step_approval", name: "用户审批", detail: "Approval Service 等待确认", progress: "68%", tone: "orange" },
      { id: "step_checkpoint", name: "Git checkpoint", detail: "执行前创建保存点", progress: "48%", tone: "green" },
      { id: "step_runner", name: "Runner 执行", detail: "本地 Runner 只执行已批准 job", progress: "28%", tone: "cyan" },
    ],
    stats: [
      ["流程节点", "5"],
      ["当前运行", "2"],
      ["等待审批", String(approvals.filter((item) => item.status === "pending").length)],
      ["已完成任务", String(tasks.filter((item) => item.status === "completed").length)],
      ["平均响应时间", "1.2s"],
    ],
    nodes: [
      { id: "node_plan", type: "agent", label: "生成执行计划", ownerAgentId: "agent_architect" },
      { id: "node_review", type: "agent", label: "风险审查", ownerAgentId: "agent_reviewer" },
      { id: "node_approval", type: "approval", label: "用户审批", ownerAgentId: "" },
      { id: "node_checkpoint", type: "git", label: "Git checkpoint", ownerAgentId: "" },
      { id: "node_runner", type: "runner", label: "本地 Runner 执行", ownerAgentId: "" },
    ],
    edges: [
      { from: "node_plan", to: "node_review", label: "提交计划" },
      { from: "node_review", to: "node_approval", label: "风险通过" },
      { from: "node_approval", to: "node_checkpoint", label: "用户批准" },
      { from: "node_checkpoint", to: "node_runner", label: "保存点已创建" },
    ],
    updatedAt: "2026-06-08T14:00:00Z",
  },
];

const runnerJobs = [];
const initialRunnerJobState = runnerJobs.map((job) => ({ ...job }));
const runtimeEvents = [];
const initialRuntimeEventState = runtimeEvents.map((event) => ({ ...event }));
const agentConfigApplications = [];
const initialAgentConfigApplicationState = agentConfigApplications.map((item) => ({ ...item }));
const runnerStatus = {
  connected: true,
  runnerId: "local_runner_001",
  version: "0.1.0",
  workspacePath: "F:/projects/agent-swarm",
  permissions: {
    readFiles: true,
    writeFiles: "approval_required",
    executeCommands: "approval_required",
    networkRequests: "approval_required",
  },
};

const gitCheckpoints = [
  {
    commit: "620d44d",
    message: "Start frontend MVP engineering cleanup",
    type: "feature",
    relatedTaskId: "task_frontend_mock_data",
    createdAt: "2026-06-08T11:00:00Z",
  },
  {
    commit: "9183d81",
    message: "Add API contract draft",
    type: "docs",
    relatedTaskId: "task_api_contract",
    createdAt: "2026-06-08T12:20:00Z",
  },
];

const knowledgeUpdates = [
  {
    id: "knowledge_update_roadmap",
    document: "dev-docs/下一步开发路线.md",
    section: "核心状态机",
    status: "synced",
    relatedFeature: "ApprovalStatus",
    updatedAt: "2026-06-08T12:00:00Z",
  },
  {
    id: "knowledge_update_api",
    document: "docs/api-draft.md",
    section: "Approvals",
    status: "synced",
    relatedFeature: "Approval Service",
    updatedAt: "2026-06-08T12:20:00Z",
  },
];

const usage = {
  tokenUsage: {
    total: 0,
    today: 0,
  },
  estimatedCost: {
    currency: "CNY",
    today: 0,
    month: 0,
  },
  byModel: [],
};

const integrations = [
  { provider: "local_runner", status: "connected", display: "本地 Runner 已连接" },
  { provider: "git", status: "connected", display: "Git 可用" },
  { provider: "github", status: "planned", display: "GitHub 待接入" },
];

const settings = {
  models: [
    { role: "architect", provider: "openai", model: "gpt-high-reasoning" },
    { role: "frontend", provider: "anthropic", model: "claude-ui" },
    { role: "backend", provider: "openai", model: "gpt-api" },
    { role: "qa", provider: "openai", model: "gpt-qa" },
    { role: "reviewer", provider: "google", model: "gemini-long-context" },
  ],
  apiKeys: [
    { provider: "openai", configured: false, display: "未接入真实模型密钥" },
    { provider: "anthropic", configured: false, display: "未接入真实模型密钥" },
  ],
  security: {
    logRedaction: true,
    syncSecretsToCloud: false,
    runnerWriteRequiresApproval: true,
  },
};

function dashboard() {
  const activeAgents = agents.filter((item) => item.status === "running").length;
  const activeTasks = tasks.filter((item) => item.status === "running" || item.status === "queued").length;
  const completedTasks = tasks.filter((item) => item.status === "completed").length;

  return {
    project,
    metrics: {
      activeAgents,
      pendingApprovals: approvals.filter((item) => item.status === "pending").length,
      activeTasks,
      gitCheckpoints: gitCheckpoints.length,
      tokenUsage: "-",
      modelCount: settings.models.length,
    },
    workflowSummary: {
      totalAgents: agents.length,
      totalTasks: tasks.length,
      completedTasks,
      successRate: tasks.length ? completedTasks / tasks.length : 0,
      averageResponseMs: 0,
    },
    workflows,
    runnerStatus: {
      ...runnerStatus,
      lastHeartbeatAt: new Date().toISOString(),
    },
    runnerJobs,
    runtimeEvents,
    agentConfigApplications,
    pendingApprovals: approvals,
    taskQueue: tasks,
    agentStatus: agents,
    gitCheckpoints,
    knowledgeUpdates,
    usageSummary: usage,
    integrationHealth: integrations,
  };
}

function resetRuntimeData() {
  approvals.splice(0, approvals.length, ...initialApprovalState.map((approval) => cloneValue(approval)));
  tasks.splice(0, tasks.length, ...initialTaskState.map((task) => cloneValue(task)));
  runnerJobs.splice(0, runnerJobs.length, ...initialRunnerJobState.map((job) => cloneValue(job)));
  runtimeEvents.splice(0, runtimeEvents.length, ...initialRuntimeEventState.map((event) => cloneValue(event)));
  agentConfigApplications.splice(
    0,
    agentConfigApplications.length,
    ...initialAgentConfigApplicationState.map((item) => cloneValue(item))
  );
}

module.exports = {
  projectId,
  status,
  project,
  agents,
  tasks,
  workflows,
  runnerStatus,
  runnerJobs,
  runtimeEvents,
  agentConfigApplications,
  approvals,
  gitCheckpoints,
  knowledgeUpdates,
  usage,
  integrations,
  settings,
  dashboard,
  resetRuntimeData,
};
