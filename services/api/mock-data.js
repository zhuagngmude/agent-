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
  phase: "MVP-0.2",
  description: "多 AI 智能体协作调度系统，优先打磨任务流、审批确认和知识库联动。",
};

const agents = [
  {
    id: "agent_architect",
    name: "架构师 Agent",
    role: "architect",
    status: "running",
    version: "v0.2.0",
    model: "gpt-high-reasoning",
    canSpawnSubAgents: true,
    maxSubAgents: 3,
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
    permissions: ["read_project", "write_frontend_patch"],
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
    permissions: ["read_project", "review_risk", "review_diff"],
  },
];

const tasks = [
  {
    id: "task_frontend_mock_data",
    title: "抽出前端 mock 数据模型",
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
    status: "completed",
    priority: "medium",
    assignedAgentId: "agent_architect",
    riskLevel: "low",
    relatedFiles: ["docs/api-draft.md"],
    requiresApproval: false,
    dependsOn: [],
  },
];

const approvals = [
  {
    id: "approval_runner_permissions",
    status: "pending",
    riskLevel: "high",
    riskTone: "high",
    requestAgentId: "agent_frontend",
    requestAgentName: "后端 Agent",
    operationTypes: ["file_write", "git_checkpoint", "audit_log_update"],
    reason: "新增 Runner 写入审批状态机，阻止本地执行绕过用户确认。",
    checkpoint: {
      required: true,
      created: true,
      commit: "a5d3f2c",
    },
    affectedFiles: ["runner/permissions.py", "server/audit_log.go", "docs/ai-maintenance.md"],
    diffSummary: "+120 -36",
    diffPreview: [
      "- return runner.execute(command)",
      "+ approval = require_user_approval(command, changed_files)",
      "+ return runner.execute(command) if approval.allowed else PatchOnlyResult()",
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
      commit: "a5d3f2c",
    },
    affectedFiles: ["docs/ai-maintenance.md"],
    diffSummary: "+56 -0",
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
    requestAgentName: "测试 Agent",
    operationTypes: ["file_write"],
    reason: "为 Runner 审批流程增加回归测试，避免后续绕开确认步骤。",
    checkpoint: {
      required: true,
      created: true,
      commit: "a5d3f2c",
    },
    affectedFiles: ["tests/runner-approval.spec.ts"],
    diffSummary: "+210 -10",
    diffPreview: [
      "+ expect(request.status).toBe('pending')",
      "+ expect(request.requiresSecondConfirm).toBe(true)",
    ],
    requiresSecondConfirm: false,
    createdAt: "2026-06-08T12:10:00Z",
  },
];

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
    document: "下一步开发路线.md",
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
    total: 1230000,
    today: 82000,
  },
  estimatedCost: {
    currency: "CNY",
    today: 128.4,
    month: 245.6,
  },
  byModel: [
    { provider: "openai", model: "gpt", tokens: 500000 },
    { provider: "anthropic", model: "claude", tokens: 400000 },
    { provider: "google", model: "gemini", tokens: 330000 },
  ],
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
    { role: "reviewer", provider: "google", model: "gemini-long-context" },
  ],
  apiKeys: [
    { provider: "openai", configured: true, display: "已加密保存" },
    { provider: "anthropic", configured: true, display: "已加密保存" },
  ],
  security: {
    logRedaction: true,
    syncSecretsToCloud: false,
    runnerWriteRequiresApproval: true,
  },
};

function dashboard() {
  return {
    project,
    metrics: {
      activeAgents: 18,
      pendingApprovals: approvals.filter((item) => item.status === "pending").length,
      activeTasks: tasks.filter((item) => item.status === "running" || item.status === "queued").length,
      gitCheckpoints: gitCheckpoints.length,
      tokenUsage: "1.23M",
      modelCount: settings.models.length,
    },
    workflowSummary: {
      totalAgents: 24,
      totalTasks: 68,
      completedTasks: 36,
      successRate: 0.923,
      averageResponseMs: 1200,
    },
    pendingApprovals: approvals,
    taskQueue: tasks,
    agentStatus: agents,
    gitCheckpoints,
    knowledgeUpdates,
    usageSummary: usage,
    integrationHealth: integrations,
  };
}

module.exports = {
  projectId,
  status,
  project,
  agents,
  tasks,
  approvals,
  gitCheckpoints,
  knowledgeUpdates,
  usage,
  integrations,
  settings,
  dashboard,
};
