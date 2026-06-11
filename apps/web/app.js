const titles = window.AGENT_SWARM_NAV || {};
let appData = window.AGENT_SWARM_DATA || {};
const statusConfig = window.AGENT_SWARM_STATUS || {};
const apiBase = "http://127.0.0.1:8787";
const projectId = "project_agent_swarm";
let selectedApprovalIndex = 0;
let selectedTaskIndex = 0;
let selectedRunnerJobIndex = 0;
let selectedAgentIndex = 0;
let selectedAgentChangeType = "model";
let selectedAgentConfigApplicationId = "";
let approvalActionRunning = false;
let runtimeStateRunning = false;
let taskActionRunning = false;
let agentChangeRequestRunning = false;
let agentConfigApplyRunning = false;
let agentConfigCancelRunning = false;
let agentConfigRollbackRequestRunning = false;
let projectPlanRequestRunning = false;
const agentConfigVersionHistoryByAgentId = new Map();
const agentConfigRollbackPreviewByApplicationId = new Map();

function pendingApprovalRequests() {
  return (appData.approvalRequests || []).filter((item) => item.status === "pending");
}

function taskHasEnabledAction(task) {
  if (!task) return false;
  if (["queued", "blocked", "waiting_user", "failed", "cancelled"].includes(task.status)) return true;
  if (task.status === "running") return true;
  return !["completed", "failed", "cancelled"].includes(task.status);
}

function defaultTaskIndex(tasks) {
  const actionableIndex = tasks.findIndex(taskHasEnabledAction);
  return actionableIndex === -1 ? 0 : actionableIndex;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function renderDashboard() {
  const project = appData.project;
  if (project) {
    document.querySelector("[data-project-name]").textContent = project.name;
    document.querySelector("[data-project-status]").textContent = project.status;
    document.querySelector("[data-project-phase]").textContent = `项目阶段：${project.phase}`;
    document.querySelector("[data-project-description]").textContent = `项目描述：${project.description}`;
  }

  const metrics = document.querySelector("#dashboardMetrics");
  if (metrics && appData.dashboardMetrics) {
    metrics.innerHTML = appData.dashboardMetrics.map((item) => `
      <article class="metric-card ${escapeHtml(item.tone)}">
        <span>${escapeHtml(item.icon)}</span>
        <p>${escapeHtml(item.label)}</p>
        <strong>${escapeHtml(item.value)}</strong>
        <small>${escapeHtml(item.note)}</small>
      </article>
    `).join("");
  }

  const workflowSteps = document.querySelector("#workflowSteps");
  if (workflowSteps && appData.workflow?.steps) {
    workflowSteps.innerHTML = appData.workflow.steps.map((step, index) => `
      ${index === 0 ? "" : '<div class="line"></div>'}
      <div class="agent-step ${escapeHtml(step.tone)}">
        <b>${escapeHtml(step.name)}</b>
        <span>${escapeHtml(step.detail)}</span>
        <i style="--p:${escapeHtml(step.progress)}"></i>
      </div>
    `).join("");
  }

  const workflowStats = document.querySelector("#workflowStats");
  if (workflowStats && appData.workflow?.stats) {
    workflowStats.innerHTML = appData.workflow.stats
      .map(([label, value]) => `<div><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`)
      .join("");
  }

  const approvalList = document.querySelector("#approvalList");
  if (approvalList && appData.approvalRequests) {
    const pendingApprovals = pendingApprovalRequests();
    const pendingApprovalCount = pendingApprovals.length;
    approvalList.innerHTML = pendingApprovals.length === 0 ? `
      <div>
        <strong>暂无待审批项</strong>
        <p>连接本地 API 后会显示 Runner 或 Agent 配置审批申请。</p>
      </div>
    ` : pendingApprovals.map((item) => `
      <div>
        <strong>${escapeHtml(item.file)}</strong>
        <p>修改类型：${escapeHtml(item.type)} · 申请人：${escapeHtml(item.agent)}</p>
        <span class="risk ${escapeHtml(item.riskTone)}">${escapeHtml(item.risk)}</span><small>${escapeHtml(item.diff)}</small>
      </div>
    `).join("");
    document.querySelector("#approvalSummary").textContent = `共 ${pendingApprovalCount} 项待审批`;
    document.querySelectorAll("[data-page='approval'] em, .approval-card .card-head h2 em")
      .forEach((item) => { item.textContent = pendingApprovalCount; });
  }

  const taskQueue = document.querySelector("#taskQueue");
  if (taskQueue && appData.taskQueue) {
    taskQueue.innerHTML = appData.taskQueue.length === 0 ? `
      <li><span class="icon purple">T</span><b>暂无任务队列数据</b><em>等待本地 API</em><strong>未加载</strong></li>
    ` : appData.taskQueue.map((item) => `
      <li><span class="icon ${escapeHtml(item.tone)}">${escapeHtml(item.icon)}</span><b>${escapeHtml(item.title)}</b><em>${escapeHtml(item.type)} · ${escapeHtml(statusLabel("task", item.status))}</em><strong>${escapeHtml(item.eta)}</strong></li>
    `).join("");
  }

  const agentStatusList = document.querySelector("#agentStatusList");
  if (agentStatusList && appData.agents) {
    agentStatusList.innerHTML = appData.agents.map((agent) => `
      <li><span class="avatar small">${escapeHtml(agent.avatar)}</span><b>${escapeHtml(agent.name)}</b><em>${escapeHtml(agent.version)}</em><strong>${escapeHtml(statusLabel("agent", agent.status))}</strong></li>
    `).join("");
  }

  const gitCheckpointList = document.querySelector("#gitCheckpointList");
  if (gitCheckpointList && appData.gitCheckpoints) {
    gitCheckpointList.innerHTML = appData.gitCheckpoints.length === 0 ? `
      <li><b>Git</b><span>暂无保存点数据</span><em>等待本地 API</em></li>
    ` : appData.gitCheckpoints.map((item) => `
      <li><b>${escapeHtml(item.hash)}</b><span>${escapeHtml(item.message)}</span><em>${escapeHtml(item.time)}</em></li>
    `).join("");
  }

  const knowledgeUpdateList = document.querySelector("#knowledgeUpdateList");
  if (knowledgeUpdateList && appData.knowledgeUpdates) {
    knowledgeUpdateList.innerHTML = appData.knowledgeUpdates.map((item) => `
      <li><span class="doc ${escapeHtml(item.tone)}">${escapeHtml(item.mark)}</span><b>${escapeHtml(item.title)}</b><em>${escapeHtml(item.detail)}</em><strong>${escapeHtml(item.time)}</strong></li>
    `).join("");
  }

  const apiKeyList = document.querySelector("#apiKeyList");
  if (apiKeyList && appData.apiKeys) {
    apiKeyList.innerHTML = appData.apiKeys.map((item) => `
      <div class="key-card">
        <b>${escapeHtml(item.name)}</b><span>${escapeHtml(item.status)}</span>
        <p>${escapeHtml(item.detail)}</p>
        <div class="bar"><i style="--p:${escapeHtml(item.usage)}"></i></div>
        <small>预算使用：${escapeHtml(item.usage)}</small>
      </div>
    `).join("");
  }
}

function renderAgentsPage(selectedIndex = selectedAgentIndex) {
  const agents = appData.agents || [];
  const agentById = new Map(agents.map((agent) => [agent.id, agent]));
  const board = document.querySelector("#agentBoard");
  const modelList = document.querySelector("#agentModelList");
  const relationList = document.querySelector("#agentRelationList");
  const configRules = document.querySelector("#agentConfigRules");
  const changePreview = document.querySelector("#agentChangePreview");
  const applicationsPanel = document.querySelector("#agentConfigApplications");
  const detail = document.querySelector("#agentDetail");
  const detailStatus = document.querySelector("#agentDetailStatus");

  if (!board || !modelList || !relationList || !configRules || !changePreview || !detail) return;

  if (agents.length === 0) {
    board.innerHTML = `<div><b>暂无智能体</b><span>本地 API 当前没有返回 Agent 数据。</span><em>只读</em></div>`;
    modelList.innerHTML = `<p class="muted">暂无模型分配数据。</p>`;
    relationList.innerHTML = `<p class="muted">暂无子 Agent 关系数据。</p>`;
    configRules.innerHTML = `<p class="muted">暂无配置规则数据。</p>`;
    changePreview.innerHTML = `<p class="muted">暂无可预览的 Agent 配置变更。</p>`;
    detail.innerHTML = `<div class="approval-meta"><div><span>当前状态</span><strong>暂无智能体</strong></div></div>`;
    if (detailStatus) {
      detailStatus.textContent = "只读";
      detailStatus.className = "badge orange";
    }
    return;
  }

  selectedAgentIndex = Math.min(Math.max(selectedIndex, 0), agents.length - 1);

  board.innerHTML = agents.map((agent, index) => `
    <div class="${index === selectedAgentIndex ? "active" : ""}" data-agent-index="${index}">
      <b>${escapeHtml(agent.name)}</b>
      <span>${escapeHtml(agent.roleLabel || agent.role || "未设置角色")} · ${escapeHtml(statusLabel("agent", agent.status))}</span>
      <em>${escapeHtml(agent.model || "未配置模型")}</em>
      <p>${escapeHtml(agent.permissionSummary || "暂无权限说明")}</p>
      <small>子 Agent：${agent.canSpawnSubAgents ? `最多 ${escapeHtml(agent.maxSubAgents)} 个` : "不允许创建"}</small>
    </div>
  `).join("");

  modelList.innerHTML = agents.map((agent) => `
    <div>
      <strong>${escapeHtml(agent.name)}</strong>
      <span>${escapeHtml(agent.model || "未配置模型")}</span>
      <em class="badge ${badgeClassForStatus("agent", agent.status)}">${escapeHtml(statusLabel("agent", agent.status))}</em>
    </div>
  `).join("");

  relationList.innerHTML = agents
    .filter((agent) => (agent.childAgentIds || []).length > 0)
    .map((agent) => `
      <div>
        <strong>${escapeHtml(agent.name)}</strong>
        <span>可创建子 Agent：最多 ${escapeHtml(agent.maxSubAgents ?? 0)} 个</span>
        <ul>
          ${(agent.childAgentIds || []).map((childId) => {
            const child = agentById.get(childId);
            return `<li><b>${escapeHtml(child?.name || childId)}</b><em>${escapeHtml(child?.roleLabel || child?.role || "未设置角色")} · 汇总回 ${escapeHtml(agent.name)}</em></li>`;
          }).join("")}
        </ul>
      </div>
    `).join("") || `<p class="muted">当前没有 Agent 声明子 Agent 关系。</p>`;

  configRules.innerHTML = agentConfigRuleGroups().map((group) => `
    <div>
      <strong>${escapeHtml(group.title)}</strong>
      <span>${escapeHtml(group.note)}</span>
      <ul>${group.items.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
    </div>
  `).join("");

  const agent = agents[selectedAgentIndex] || agents[0];
  if (!agentConfigVersionHistoryByAgentId.has(agent.id)) {
    void refreshAgentConfigVersionHistory(agent.id);
  }
  const parentAgent = agentById.get(agent.parentAgentId);
  const reportAgent = agentById.get(agent.reportsToAgentId);
  if (detailStatus) {
    detailStatus.textContent = statusLabel("agent", agent.status);
    detailStatus.className = `badge ${badgeClassForStatus("agent", agent.status)}`;
  }

  detail.innerHTML = `
    <div class="approval-meta">
      <div><span>Agent ID</span><strong>${escapeHtml(agent.id || "未记录")}</strong></div>
      <div><span>角色</span><strong>${escapeHtml(agent.roleLabel || agent.role || "未设置角色")}</strong></div>
      <div><span>模型</span><strong>${escapeHtml(agent.model || "未配置模型")}</strong></div>
      <div><span>版本</span><strong>${escapeHtml(agent.version || "未记录")}</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>是否允许创建子 Agent</span><strong>${agent.canSpawnSubAgents ? "允许" : "不允许"}</strong></div>
      <div><span>最大子 Agent 数</span><strong>${escapeHtml(agent.maxSubAgents ?? 0)}</strong></div>
      <div><span>当前状态</span><strong>${escapeHtml(statusLabel("agent", agent.status))}</strong></div>
      <div><span>安全说明</span><strong>当前只读，不会修改 Agent 配置。</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>父 Agent</span><strong>${escapeHtml(parentAgent?.name || "无")}</strong></div>
      <div><span>汇总目标</span><strong>${escapeHtml(reportAgent?.name || "无")}</strong></div>
      <div><span>派生深度</span><strong>${escapeHtml(agent.spawnDepth ?? 0)}</strong></div>
      <div><span>当前子 Agent</span><strong>${escapeHtml((agent.childAgentIds || []).length)} 个</strong></div>
    </div>
    <div class="task-files">
      <h3>权限</h3>
      <ul>${(agent.permissions || []).map((permission) => `<li>${escapeHtml(permission)}</li>`).join("") || "<li>暂无权限</li>"}</ul>
    </div>
    ${renderAgentConfigVersionHistory(agent)}
  `;

  board.querySelectorAll("[data-agent-index]").forEach((card) => {
    card.addEventListener("click", () => renderAgentsPage(Number(card.dataset.agentIndex)));
  });

  document.querySelectorAll("[data-agent-change]").forEach((button) => {
    button.classList.toggle("active", button.dataset.agentChange === selectedAgentChangeType);
    button.onclick = () => {
      selectedAgentChangeType = button.dataset.agentChange;
      renderAgentsPage(selectedAgentIndex);
    };
  });

  changePreview.innerHTML = renderAgentChangePreview(agent, selectedAgentChangeType);
  if (applicationsPanel) {
    applicationsPanel.innerHTML = renderAgentConfigApplications(agent);
    applicationsPanel.querySelectorAll("[data-agent-config-application-id]").forEach((card) => {
      card.addEventListener("click", () => {
        selectedAgentConfigApplicationId = card.dataset.agentConfigApplicationId;
        renderAgentsPage(selectedAgentIndex);
      });
    });
  }

  const submitChangeButton = document.querySelector("#submitAgentChangeRequest");
  if (submitChangeButton) {
    submitChangeButton.disabled = agentChangeRequestRunning;
    submitChangeButton.onclick = () => runAgentChangeRequest();
  }

  const mockApplyButton = document.querySelector("#mockApplyAgentConfigApplication");
  if (mockApplyButton) {
    const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
    const approval = application ? approvalStatusForApplication(application) : null;
    mockApplyButton.disabled = agentConfigApplyRunning || !canMockApplyAgentConfigApplication(application, approval);
    mockApplyButton.onclick = () => runAgentConfigApplicationApply();
  }

  const mockCancelButton = document.querySelector("#mockCancelAgentConfigApplication");
  if (mockCancelButton) {
    const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
    mockCancelButton.disabled = agentConfigCancelRunning || !canMockCancelAgentConfigApplication(application);
    mockCancelButton.onclick = () => runAgentConfigApplicationCancel();
  }

  const rollbackRequestButton = document.querySelector("#previewAgentConfigRollbackRequest");
  if (rollbackRequestButton) {
    const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
    rollbackRequestButton.disabled = agentConfigRollbackRequestRunning || application?.status !== "applied";
    rollbackRequestButton.onclick = () => runAgentConfigRollbackRequestPreview();
  }
}

function normalizeDashboard(apiData) {
  const fallback = window.AGENT_SWARM_DATA || {};
  const pendingApprovals = apiData.pendingApprovals || [];
  const taskQueue = apiData.taskQueue || [];
  const agentStatus = apiData.agentStatus || [];
  const workflows = apiData.workflows || [];
  const runnerJobs = apiData.runnerJobs || [];
  const agentConfigApplications = apiData.agentConfigApplications || [];
  const agentById = new Map(agentStatus.map((agent) => [agent.id, agent]));
  const primaryWorkflow = workflows[0];

  return {
    ...fallback,
    project: {
      name: apiData.project?.name || fallback.project?.name || "agent蜂群 MVP",
      status: apiData.project?.status || fallback.project?.status || "running",
      phase: apiData.project?.phase || fallback.project?.phase || "MVP-0.2",
      description: apiData.project?.description || fallback.project?.description || "",
    },
    featureFlags: {
      ...(fallback.featureFlags || {}),
      ...(apiData.featureFlags || {}),
    },
    dashboardMetrics: [
      { label: "活跃智能体", value: apiData.metrics?.activeAgents ?? "-", note: "来自本地 API", tone: "purple", icon: "A" },
      { label: "待确认事项", value: apiData.metrics?.pendingApprovals ?? "-", note: "Runner 审批优先", tone: "orange", icon: "!" },
      { label: "活跃任务", value: apiData.metrics?.activeTasks ?? "-", note: "运行中与排队中", tone: "blue", icon: "T" },
      { label: "Git 检查点", value: apiData.metrics?.gitCheckpoints ?? "-", note: "项目保存点", tone: "green", icon: "G" },
      { label: "Token 消耗", value: "-", note: "真实模型调用未接入", tone: "violet", icon: "K" },
      { label: "模型使用", value: apiData.metrics?.modelCount ?? "-", note: "模型配置草案", tone: "cyan", icon: "M" },
    ],
    workflow: primaryWorkflow ? {
      id: primaryWorkflow.id,
      name: primaryWorkflow.name,
      status: primaryWorkflow.status,
      description: primaryWorkflow.description || "",
      updatedAt: primaryWorkflow.updatedAt || "",
      steps: primaryWorkflow.steps || fallback.workflow?.steps || [],
      stats: primaryWorkflow.stats || fallback.workflow?.stats || [],
      nodes: primaryWorkflow.nodes || [],
      edges: primaryWorkflow.edges || [],
    } : fallback.workflow,
    runnerStatus: {
      connected: apiData.runnerStatus?.connected === true,
      runnerId: apiData.runnerStatus?.runnerId || "",
      version: apiData.runnerStatus?.version || "",
      workspacePath: apiData.runnerStatus?.workspacePath || "",
      permissions: apiData.runnerStatus?.permissions || {},
      lastHeartbeatAt: apiData.runnerStatus?.lastHeartbeatAt || "",
    },
    approvalRequests: pendingApprovals.map((item) => ({
      file: item.affectedFiles?.[0] || item.id,
      type: (item.operationTypes || []).join(" / ") || "unknown",
      agent: item.requestAgentName || item.requestAgentId || "Agent",
      risk: item.riskLevel === "high" ? "高风险" : item.riskLevel === "medium" ? "中风险" : "低风险",
      riskTone: item.riskTone || (item.riskLevel === "high" ? "high" : item.riskLevel === "medium" ? "mid" : "low"),
      diff: item.diffSummary || "",
      status: item.status,
      reason: item.reason || "",
      checkpoint: item.checkpoint?.commit || "",
      operationTypes: item.operationTypes || [],
      affectedFiles: item.affectedFiles || [],
      diffPreview: item.diffPreview || [],
      id: item.id,
      targetService: item.targetService || "runner",
      requiresSecondConfirm: item.requiresSecondConfirm === true,
    })),
    taskQueue: taskQueue.map((task) => ({
      icon: task.priority === "high" ? "!" : "T",
      tone: task.status === "completed" ? "green" : task.priority === "high" ? "red" : "purple",
      title: task.title,
      type: task.priority === "high" ? "高优先级" : "任务",
      eta: statusLabel("task", task.status),
      status: task.status,
      id: task.id,
      description: task.description || "",
      assignedAgentId: task.assignedAgentId || "",
      assignedAgentName: agentById.get(task.assignedAgentId)?.name || task.assignedAgentId || "未分配",
      priority: task.priority || "",
      riskLevel: task.riskLevel || "low",
      relatedFiles: task.relatedFiles || [],
      requiresApproval: task.requiresApproval === true,
      dependsOn: task.dependsOn || [],
      startedAt: task.startedAt || "",
      completedAt: task.completedAt || "",
      failedAt: task.failedAt || "",
      cancelledAt: task.cancelledAt || "",
      failureReason: task.failureReason || "",
    })),
    runnerJobs: runnerJobs.map((job) => ({
      id: job.id,
      approvalId: job.approvalId || "",
      taskId: job.taskId || "",
      status: job.status || "queued",
      operationTypes: job.operationTypes || [],
      affectedFiles: job.affectedFiles || [],
      checkpoint: job.checkpoint || "",
      safetyNote: job.safetyNote || "",
      createdAt: job.createdAt || "",
      updatedAt: job.updatedAt || "",
    })),
    agentConfigApplications: agentConfigApplications.map((item) => ({
      id: item.id,
      approvalId: item.approvalId || "",
      agentId: item.agentId || "",
      agentName: item.agentName || agentById.get(item.agentId)?.name || item.agentId || "",
      changeType: item.changeType || "",
      changes: item.changes || [],
      status: item.status || "pending_apply",
      createdAt: item.createdAt || "",
      updatedAt: item.updatedAt || "",
      appliedAt: item.appliedAt || "",
      appliedBy: item.appliedBy || "",
      applyConfirmText: item.applyConfirmText || "",
      cancelledAt: item.cancelledAt || "",
      cancelledBy: item.cancelledBy || "",
      cancelReason: item.cancelReason || "",
    })),
    agents: agentStatus.map((agent, index) => ({
      avatar: String.fromCharCode(65 + index),
      id: agent.id,
      name: agent.name,
      role: agent.role,
      roleLabel: roleLabel(agent.role),
      version: agent.version,
      status: agent.status,
      model: agent.model,
      canSpawnSubAgents: agent.canSpawnSubAgents === true,
      maxSubAgents: agent.maxSubAgents ?? 0,
      parentAgentId: agent.parentAgentId || "",
      childAgentIds: agent.childAgentIds || [],
      reportsToAgentId: agent.reportsToAgentId || "",
      spawnDepth: agent.spawnDepth ?? 0,
      permissions: agent.permissions || [],
      permissionSummary: (agent.permissions || []).join(" / "),
    })),
    gitCheckpoints: (apiData.gitCheckpoints || []).map((item) => ({
      hash: item.commit,
      message: item.message,
      time: item.createdAt || "",
    })),
    knowledgeUpdates: (apiData.knowledgeUpdates || []).map((item) => ({
      mark: item.section || "文档",
      tone: "blue",
      title: item.document,
      detail: item.relatedFeature || item.status,
      time: item.updatedAt || "",
    })),
    apiKeys: fallback.apiKeys,
  };
}

async function loadDashboardFromApi() {
  const response = await fetch(`${apiBase}/api/projects/${projectId}/dashboard`);
  if (!response.ok) {
    throw new Error(`API returned ${response.status}`);
  }
  return normalizeDashboard(await response.json());
}

async function postApprovalAction(approvalId, action, body = {}) {
  const response = await fetch(`${apiBase}/api/approvals/${approvalId}/${action}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postTaskAction(taskId, action, body = {}) {
  const response = await fetch(`${apiBase}/api/tasks/${taskId}/${action}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postProjectPlanRequest(body = {}) {
  const response = await fetch(`${apiBase}/api/projects/${projectId}/project-plan-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postAgentChangeRequest(agentId, body = {}) {
  const response = await fetch(`${apiBase}/api/agents/${agentId}/change-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postAgentConfigApplicationApply(applicationId, body = {}) {
  const response = await fetch(`${apiBase}/api/agent-config-applications/${applicationId}/apply`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postAgentConfigApplicationCancel(applicationId, body = {}) {
  const response = await fetch(`${apiBase}/api/agent-config-applications/${applicationId}/cancel`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function postAgentConfigRollbackRequest(applicationId, body = {}) {
  const response = await fetch(`${apiBase}/api/agent-config-applications/${applicationId}/rollback-request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function requestAgentConfigVersionHistory(agentId) {
  const response = await fetch(`${apiBase}/api/agents/${encodeURIComponent(agentId)}/config-version-history`);
  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function refreshAgentConfigVersionHistory(agentId, options = {}) {
  const current = agentConfigVersionHistoryByAgentId.get(agentId);
  if (!agentId || current?.loading || (current?.history && !options.force)) return;

  agentConfigVersionHistoryByAgentId.set(agentId, { loading: true });
  try {
    const history = await requestAgentConfigVersionHistory(agentId);
    agentConfigVersionHistoryByAgentId.set(agentId, { loading: false, history });
  } catch (error) {
    agentConfigVersionHistoryByAgentId.set(agentId, { loading: false, error: error.message });
  }

  const selectedAgent = appData.agents?.[selectedAgentIndex];
  if (selectedAgent?.id === agentId) {
    renderAgentsPage(selectedAgentIndex);
  }
}

function refreshSelectedAgentConfigVersionHistory(options = {}) {
  const selectedAgent = appData.agents?.[selectedAgentIndex];
  if (selectedAgent?.id) {
    void refreshAgentConfigVersionHistory(selectedAgent.id, options);
  }
}

function formatConfigValue(value) {
  if (Array.isArray(value)) return value.length > 0 ? value.join(" / ") : "空";
  if (value === true) return "是";
  if (value === false) return "否";
  if (value === null || value === undefined || value === "") return "未记录";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function versionLabel(version) {
  if (!version) return "无";
  const time = version.appliedAt || version.createdAt || "未记录时间";
  const approval = version.approvalId ? ` · 审批 ${version.approvalId}` : "";
  return `v${version.version || "?"} · ${time}${approval}`;
}

function renderConfigSnapshot(snapshot = {}) {
  const entries = Object.entries(snapshot || {});
  if (entries.length === 0) return "<li>暂无快照字段</li>";
  return entries
    .map(([field, value]) => `<li><b>${escapeHtml(field)}</b><span>${escapeHtml(formatConfigValue(value))}</span></li>`)
    .join("");
}

function renderVersionChanges(version) {
  const changes = Array.isArray(version?.changes) ? version.changes : [];
  if (changes.length === 0) return "<li>暂无版本变更明细</li>";
  return changes.map((change) => {
    if (Array.isArray(change)) {
      const [field, before, after] = change;
      return `<li>${escapeHtml(field || "field")}：${escapeHtml(formatConfigValue(before))} -> ${escapeHtml(formatConfigValue(after))}</li>`;
    }
    const before = change.before !== undefined ? change.before : change.current;
    const after = change.after !== undefined ? change.after : change.restore;
    return `<li>${escapeHtml(change.field || "field")}：${escapeHtml(formatConfigValue(before))} -> ${escapeHtml(formatConfigValue(after))}</li>`;
  }).join("");
}

function renderAgentConfigVersionHistory(agent) {
  const state = agentConfigVersionHistoryByAgentId.get(agent.id);
  if (!state || state.loading) {
    return `
      <div class="task-files">
        <h3>配置版本历史</h3>
        <ul><li>正在读取版本历史，只读查询不会写入配置或创建 Runner job。</li></ul>
      </div>
    `;
  }
  if (state.error) {
    return `
      <div class="task-files">
        <h3>配置版本历史</h3>
        <ul><li>读取失败：${escapeHtml(state.error)}</li></ul>
      </div>
    `;
  }

  const history = state.history || {};
  const versions = Array.isArray(history.versions) ? history.versions : [];
  const candidates = Array.isArray(history.restoreCandidates) ? history.restoreCandidates : [];
  return `
    <div class="task-files">
      <h3>配置版本历史</h3>
      <ul>
        <li><b>当前版本</b>：${escapeHtml(versionLabel(history.currentVersion))}</li>
        <li><b>默认回滚来源</b>：${escapeHtml(versionLabel(history.restoreVersion))}</li>
        <li><b>可回退版本</b>：${escapeHtml(candidates.length)} 个</li>
        <li><b>回滚来源就绪</b>：${history.rollbackSourceReady ? "是" : "否"}</li>
        <li><b>只读保护</b>：${history.readOnly && history.canWrite === false ? "已确认" : "未确认"}</li>
        ${versions.length === 0 ? "<li>暂无真实版本记录；SQLite 真实应用成功后才会生成版本。</li>" : ""}
      </ul>
    </div>
  `;
}

function renderAgentConfigRollbackVersionDetails(versionHistoryState) {
  if (!versionHistoryState || versionHistoryState.loading) {
    return `<p class="muted">正在读取 Agent 配置版本历史，回滚来源暂不可确认。</p>`;
  }
  if (versionHistoryState.error) {
    return `<p class="muted">版本历史读取失败：${escapeHtml(versionHistoryState.error)}</p>`;
  }

  const history = versionHistoryState.history || {};
  const candidates = Array.isArray(history.restoreCandidates) ? history.restoreCandidates : [];
  const currentVersion = history.currentVersion;
  const restoreVersion = history.restoreVersion;
  return `
    <div class="task-files">
      <h3>当前版本快照</h3>
      <ul>${renderConfigSnapshot(currentVersion?.configSnapshot || {})}</ul>
    </div>
    <div class="task-files">
      <h3>默认回滚来源快照</h3>
      <ul>${renderConfigSnapshot(restoreVersion?.configSnapshot || {})}</ul>
    </div>
    <div class="task-files">
      <h3>当前版本变更</h3>
      <ul>${renderVersionChanges(currentVersion)}</ul>
    </div>
    <div class="task-files">
      <h3>可回退版本</h3>
      <ul>
        ${candidates.map((version) => `<li>${escapeHtml(versionLabel(version))}</li>`).join("") || "<li>暂无可回退版本</li>"}
      </ul>
    </div>
  `;
}

async function requestRuntimeState(method, path = "/api/runtime-state") {
  const response = await fetch(`${apiBase}${path}`, { method });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function requestModelGatewayStatus() {
  const response = await fetch(`${apiBase}/api/model-gateway/status`);
  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

async function requestModelGatewayDryRun() {
  const response = await fetch(`${apiBase}/api/model-gateway/dry-run`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      provider: "openai",
      model: "gpt-4.1-mini",
      purpose: "connectivity_check",
      promptPreview: "settings panel connectivity preview",
      requestedBy: "local_user",
    }),
  });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
}

function localTrialModeLabel(mode) {
  if (mode === "sqlite") return "SQLite 本地持久化";
  if (mode === "mock") return "Mock 运行态";
  return "未知模式";
}

function localTrialBadgeClass(mode) {
  if (mode === "sqlite") return "badge green";
  if (mode === "mock") return "badge orange";
  return "badge gray";
}

function statusLabel(group, status) {
  return statusConfig[group]?.[status]?.label || status;
}

function statusTone(group, status) {
  return statusConfig[group]?.[status]?.tone || "neutral";
}

function roleLabel(role) {
  const labels = {
    architect: "架构规划",
    frontend: "前端实现",
    backend: "后端实现",
    docs: "文档维护",
    reviewer: "安全审查",
    scheduler: "任务调度",
    executor: "执行协调",
    qa: "测试验证",
  };
  return labels[role] || role || "未设置角色";
}

function agentConfigRuleGroups() {
  return [
    {
      title: "可编辑字段",
      note: "后续可以开放编辑，但仍需要记录变更历史。",
      items: ["Agent 名称", "使用模型", "启用 / 禁用状态", "权限列表", "是否允许创建子 Agent", "最大子 Agent 数"],
    },
    {
      title: "必须审批字段",
      note: "修改后可能改变系统能力边界，需要走 Approval Service。",
      items: ["权限列表", "是否允许创建子 Agent", "最大子 Agent 数", "代码执行请求权限", "API Key / 模型 Key 访问权限"],
    },
    {
      title: "暂时只读字段",
      note: "先作为系统身份和关系数据展示，不在 MVP-0.2 开放修改。",
      items: ["Agent ID", "角色类型", "父 Agent", "派生深度", "汇总目标", "创建来源"],
    },
    {
      title: "禁止子 Agent 修改",
      note: "子 Agent 不能扩权，也不能改写自己的归属关系。",
      items: ["自己的权限", "父 Agent", "汇总目标", "API Key", "Runner 执行权限", "其他 Agent 的配置"],
    },
  ];
}

function agentChangeDraft(agent, type) {
  const drafts = {
    model: {
      title: "模型切换",
      risk: "中风险",
      riskClass: "orange",
      requiresApproval: true,
      reason: "模型变化会影响输出风格、成本和上下文能力，需要审批后才能保存。",
      changes: [
        ["model", agent.model || "未配置模型", "gpt-high-reasoning"],
      ],
    },
    spawn: {
      title: "子 Agent 权限调整",
      risk: "高风险",
      riskClass: "red",
      requiresApproval: true,
      reason: "允许创建子 Agent 会扩大调度能力边界，必须经过 Approval Service。",
      changes: [
        ["canSpawnSubAgents", agent.canSpawnSubAgents ? "允许" : "不允许", "允许"],
        ["maxSubAgents", String(agent.maxSubAgents ?? 0), "3"],
      ],
    },
    permission: {
      title: "权限升级",
      risk: "高风险",
      riskClass: "red",
      requiresApproval: true,
      reason: "新增代码执行请求权限会影响 Runner 安全边界，必须二次确认。",
      permissionProfile: agent.role === "reviewer" ? "reviewer_agent" : "executor_agent",
      changes: [
        ["permissions", (agent.permissions || []).join(" / ") || "无", agent.role === "reviewer" ? "reviewer_agent" : "executor_agent"],
      ],
    },
  };

  return drafts[type] || drafts.model;
}

function agentChangeRequestBody(agent, type) {
  const draft = agentChangeDraft(agent, type);
  return {
    changeType: type,
    riskLevel: draft.riskClass === "red" ? "high" : draft.riskClass === "orange" ? "medium" : "low",
    reason: draft.reason,
    permissionProfile: type === "permission" ? draft.permissionProfile : "",
    changes: draft.changes.map(([field, before, after]) => ({ field, before, after })),
  };
}

function renderAgentChangePreview(agent, type) {
  const draft = agentChangeDraft(agent, type);
  return `
    <div class="agent-change-preview">
      <div class="approval-meta">
        <div><span>目标 Agent</span><strong>${escapeHtml(agent.name)}</strong></div>
        <div><span>变更类型</span><strong>${escapeHtml(draft.title)}</strong></div>
        <div><span>风险等级</span><strong><em class="badge ${draft.riskClass}">${escapeHtml(draft.risk)}</em></strong></div>
        <div><span>是否需要审批</span><strong>${draft.requiresApproval ? "需要" : "不需要"}</strong></div>
      </div>
      <div class="approval-meta">
        <div><span>保存状态</span><strong>仅预览，当前不会写入配置。</strong></div>
        <div><span>审批原因</span><strong>${escapeHtml(draft.reason)}</strong></div>
      </div>
      <div class="task-files">
        <h3>字段变更</h3>
        <ul>${draft.changes.map(([field, before, after]) => `<li>${escapeHtml(field)}：${escapeHtml(before)} -> ${escapeHtml(after)}</li>`).join("")}</ul>
      </div>
      <div class="agent-change-submit">
        <button class="neutral-action" id="submitAgentChangeRequest">生成审批申请</button>
      </div>
    </div>
  `;
}

function agentConfigApplicationStatusLabel(status) {
  const labels = {
    pending_apply: "待应用",
    applied: "已应用",
    cancelled: "已取消",
  };
  return labels[status] || status || "未知";
}

function agentConfigApplicationStatusBadge(status) {
  if (status === "applied") return "green";
  if (status === "cancelled") return "gray";
  return "orange";
}

function changeTypeLabel(type) {
  const labels = {
    model: "模型切换",
    spawn: "子 Agent 权限",
    permission: "权限升级",
  };
  return labels[type] || type || "配置变更";
}

function approvalStatusForApplication(application) {
  return (appData.approvalRequests || []).find((approval) => approval.id === application.approvalId);
}

function renderAgentConfigApplicationChecklist(application, approval) {
  const checks = [
    ["来源审批", approval?.status === "approved" ? "已批准" : "需确认审批状态"],
    ["目标服务", approval?.targetService === "agent_config" ? "agent_config" : "需确认目标服务"],
    ["Runner 队列", approval?.runnerJobId ? "异常：存在 Runner job" : "不会生成 Runner job"],
    ["应用入口", application.status === "pending_apply" ? "等待人工应用入口开放" : agentConfigApplicationStatusLabel(application.status)],
  ];

  return checks.map(([label, value]) => `
    <li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>
  `).join("");
}

function canMockApplyAgentConfigApplication(application, approval) {
  return application?.status === "pending_apply"
    && approval?.status === "approved"
    && approval?.targetService === "agent_config"
    && !approval?.runnerJobId;
}

function canMockCancelAgentConfigApplication(application) {
  return application?.status === "pending_apply";
}

function renderAgentConfigManualApplyConditions(application, approval) {
  const isReady = canMockApplyAgentConfigApplication(application, approval);
  const conditions = [
    ["确认方式", "需要用户二次确认"],
    ["Mock 接口", "POST /api/agent-config-applications/:applicationId/apply"],
    ["应用范围", "只修改目标 Agent 的已审批字段"],
    ["状态流转", "pending_apply -> applied / cancelled"],
    ["取消入口", canMockCancelAgentConfigApplication(application) ? "可执行 Mock 取消" : "不可取消当前状态"],
    ["当前结论", isReady ? "可执行 Mock 应用状态流转" : "暂不满足应用前置条件"],
  ];

  return conditions.map(([label, value]) => `
    <li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>
  `).join("");
}

function renderAgentConfigRealApplyGate(application, approval) {
  const flags = appData.featureFlags || {};
  const enabled = flags.agentConfigRealApplyEnabled === true;
  const sqliteRequired = flags.agentConfigRealApplyRequiresSqlite !== false;
  const gateReady = enabled && canMockApplyAgentConfigApplication(application, approval);
  const requirements = [
    ["API feature flag", enabled ? "AGENT_SWARM_ENABLE_AGENT_CONFIG_REAL_APPLY=true" : "off by default"],
    ["Storage mode", sqliteRequired ? "SQLite only" : "not restricted"],
    ["Dry-run proof", "required and must match this application"],
    ["Second confirmation", "required"],
    ["Operator identity", "requestedBy required"],
    ["Git checkpoint", "acknowledgement required"],
    ["Rollback plan", "acceptance required"],
    ["Runner/model/secret side effects", "must remain blocked"],
  ];

  return `
    <div class="real-apply-gate ${enabled ? "enabled" : "disabled"}">
      <div class="real-apply-gate-head">
        <strong>SQLite real apply gate</strong>
        <span class="badge ${enabled ? "green" : "orange"}">${enabled ? "flag enabled" : "flag off"}</span>
      </div>
      <p>${enabled
        ? "This API process can accept a guarded real-apply request, but the web UI does not submit it directly."
        : "Default UI path stays status-only. Real Agent config writes require an operator-started API feature flag and a verified request body."}</p>
      <ul>${requirements.map(([label, value]) => `
        <li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>
      `).join("")}</ul>
      <button type="button" class="neutral-action" disabled>${gateReady ? "Real apply requires operator request body" : "Real apply unavailable in UI"}</button>
    </div>
  `;
}

function renderAgentConfigApplicationAudit(application, approval) {
  const audit = [
    ["应用时间", application.appliedAt || "尚未应用"],
    ["确认人", application.appliedBy || "尚未记录"],
    ["确认文本", application.applyConfirmText || "尚未记录"],
    ["取消时间", application.cancelledAt || "尚未取消"],
    ["取消人", application.cancelledBy || "尚未记录"],
    ["取消原因", application.cancelReason || "尚未记录"],
    ["Agent 配置写入", "未执行真实写入"],
    ["Runner job", approval?.runnerJobId ? `异常：${approval.runnerJobId}` : "未生成"],
    ["执行类型", "仅 Mock 状态流转"],
  ];

  return audit.map(([label, value]) => `
    <li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>
  `).join("");
}

function renderAgentConfigRollbackReview(application, approval, versionHistoryState) {
  const history = versionHistoryState?.history || {};
  const historyLoading = !versionHistoryState || versionHistoryState.loading;
  const historyError = versionHistoryState?.error || "";
  const rollbackSourceReady = Boolean(history.rollbackSourceReady);
  const versions = Array.isArray(history.versions) ? history.versions : [];
  const restoreCandidates = Array.isArray(history.restoreCandidates) ? history.restoreCandidates : [];
  const canReviewRollback = application.status === "applied"
    && Boolean(application.appliedAt)
    && Array.isArray(application.changes)
    && application.changes.length > 0
    && approval?.targetService === "agent_config"
    && !approval?.runnerJobId
    && rollbackSourceReady;
  const review = [
    ["回滚入口", "当前未开放；必须重新创建 Agent 配置审批"],
    ["应用状态", application.status === "applied" ? "已应用，可进入回滚前审查" : "尚未应用，不需要回滚"],
    ["应用审计", application.appliedAt ? "已记录应用时间和确认信息" : "缺少应用审计，不允许回滚"],
    ["来源审批", approval?.targetService === "agent_config" ? "可追溯到 agent_config 审批" : "需先确认来源审批"],
    ["Runner job", approval?.runnerJobId ? `异常：${approval.runnerJobId}` : "未生成 Runner job"],
    ["字段差异", application.changes?.length ? `可基于 ${application.changes.length} 个字段生成反向变更草案` : "缺少字段差异"],
    ["版本历史", historyLoading ? "读取中" : historyError ? `读取失败：${historyError}` : `${versions.length} 个版本记录`],
    ["当前版本", history.currentVersion ? versionLabel(history.currentVersion) : "暂无当前版本"],
    ["默认回滚来源", history.restoreVersion ? versionLabel(history.restoreVersion) : "暂无可用来源"],
    ["可回退版本", `${restoreCandidates.length} 个`],
    ["回滚来源就绪", rollbackSourceReady ? "是" : "否"],
    ["当前结论", canReviewRollback ? "可展示完整回滚前审查；仍不执行真实回滚" : "暂不满足回滚前审查条件"],
  ];

  return review.map(([label, value]) => `
    <li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>
  `).join("");
}

function renderAgentConfigRollbackRequestPreview(application) {
  const state = agentConfigRollbackPreviewByApplicationId.get(application.id);
  if (!state) {
    return "";
  }
  if (state.loading) {
    return `
      <div class="application-checklist">
        <h3>回滚请求预检查结果</h3>
        <ul><li><b>状态</b><span>正在读取版本历史并生成只读 diff</span></li></ul>
      </div>
    `;
  }
  if (state.error) {
    return `
      <div class="application-checklist">
        <h3>回滚请求预检查结果</h3>
        <ul><li><b>状态</b><span>预检查失败：${escapeHtml(state.error)}</span></li></ul>
      </div>
    `;
  }

  const result = state.result || {};
  const history = result.versionHistory || {};
  const currentVersion = history.currentVersion;
  const restoreVersion = history.restoreVersion;
  const restoreDiff = Array.isArray(result.restoreDiff)
    ? result.restoreDiff
    : Array.isArray(result.rollbackPreview?.diff)
      ? result.rollbackPreview.diff
      : [];
  const fieldCount = result.rollbackPreview?.fieldCount ?? restoreDiff.length;
  const validationErrors = Array.isArray(result.validationErrors) ? result.validationErrors : [];
  const summary = [
    ["当前版本", currentVersion ? versionLabel(currentVersion) : `v${result.currentVersion || "无"}`],
    ["恢复版本", restoreVersion ? versionLabel(restoreVersion) : `v${result.restoreVersion || "无"}`],
    ["restore diff 字段", `${fieldCount} 个`],
    ["回滚来源就绪", history.rollbackSourceReady ? "是" : "否"],
    ["canCreateApproval", result.canCreateApproval === true ? "true" : "false"],
    ["requestReady", result.requestReady === true ? "true" : "false"],
  ];

  return `
    <div class="application-checklist">
      <h3>回滚请求预检查结果</h3>
      <ul>
        ${summary.map(([label, value]) => `<li><b>${escapeHtml(label)}</b><span>${escapeHtml(value)}</span></li>`).join("")}
        ${validationErrors.map((error) => `<li><b>校验</b><span>${escapeHtml(error)}</span></li>`).join("")}
      </ul>
    </div>
    <div class="task-files">
      <h3>restore diff</h3>
      <ul>
        ${restoreDiff.map((change) => {
          const current = change.current !== undefined ? change.current : change.before;
          const restore = change.restore !== undefined ? change.restore : change.after;
          return `<li>${escapeHtml(change.field || "field")}：${escapeHtml(formatConfigValue(current))} -> ${escapeHtml(formatConfigValue(restore))}<small>${escapeHtml(change.action || "restore")}</small></li>`;
        }).join("") || "<li>暂无可展示字段变化</li>"}
      </ul>
    </div>
  `;
}

function renderAgentConfigApplications(agent) {
  const applications = (appData.agentConfigApplications || [])
    .filter((item) => item.agentId === agent.id);

  if (applications.length === 0) {
    return `
      <p class="muted">当前 Agent 暂无待应用配置变更。审批通过后会先出现在这里，不会直接修改 Agent 配置。</p>
      ${renderAgentConfigRealApplyGate(null, null)}
    `;
  }

  if (!applications.some((item) => item.id === selectedAgentConfigApplicationId)) {
    selectedAgentConfigApplicationId = applications[0].id;
  }

  const selectedApplication = applications.find((item) => item.id === selectedAgentConfigApplicationId) || applications[0];
  const selectedApproval = approvalStatusForApplication(selectedApplication);
  const versionHistoryState = agentConfigVersionHistoryByAgentId.get(agent.id);

  return `
    <div class="application-review-layout">
      <div class="application-list">
        ${applications.map((item) => `
          <button class="${item.id === selectedApplication.id ? "active" : ""}" type="button" data-agent-config-application-id="${escapeHtml(item.id)}">
            <span>
              <strong>${escapeHtml(changeTypeLabel(item.changeType))}</strong>
              <small>${escapeHtml(item.approvalId || "未记录审批")}</small>
            </span>
            <em class="badge ${agentConfigApplicationStatusBadge(item.status)}">${escapeHtml(agentConfigApplicationStatusLabel(item.status))}</em>
          </button>
        `).join("")}
      </div>
      <div class="application-review-detail">
        <div class="application-head">
          <strong>${escapeHtml(changeTypeLabel(selectedApplication.changeType))}</strong>
          <span class="badge ${agentConfigApplicationStatusBadge(selectedApplication.status)}">${escapeHtml(agentConfigApplicationStatusLabel(selectedApplication.status))}</span>
        </div>
        <div class="approval-meta">
          <div><span>目标 Agent</span><strong>${escapeHtml(selectedApplication.agentName || agent.name)}</strong></div>
          <div><span>来源审批</span><strong>${escapeHtml(selectedApplication.approvalId || "未记录")}</strong></div>
          <div><span>审批状态</span><strong>${escapeHtml(statusLabel("approval", selectedApproval?.status || "approved"))}</strong></div>
          <div><span>更新时间</span><strong>${escapeHtml(selectedApplication.updatedAt || selectedApplication.createdAt || "未记录")}</strong></div>
        </div>
        <div class="application-checklist">
          <h3>应用前检查</h3>
          <ul>${renderAgentConfigApplicationChecklist(selectedApplication, selectedApproval)}</ul>
        </div>
        <div class="application-checklist">
          <h3>人工应用确认条件</h3>
          <ul>${renderAgentConfigManualApplyConditions(selectedApplication, selectedApproval)}</ul>
        </div>
        ${renderAgentConfigRealApplyGate(selectedApplication, selectedApproval)}
        <div class="application-checklist">
          <h3>应用审计记录</h3>
          <ul>${renderAgentConfigApplicationAudit(selectedApplication, selectedApproval)}</ul>
        </div>
        <div class="application-checklist">
          <h3>回滚前审查</h3>
          <ul>${renderAgentConfigRollbackReview(selectedApplication, selectedApproval, versionHistoryState)}</ul>
        </div>
        ${renderAgentConfigRollbackRequestPreview(selectedApplication)}
        ${renderAgentConfigRollbackVersionDetails(versionHistoryState)}
        <div class="task-files">
          <h3>字段变更</h3>
          <ul>
            ${(selectedApplication.changes || []).map((change) => `
              <li>${escapeHtml(change.field || "field")}：${escapeHtml(change.before || "")} -> ${escapeHtml(change.after || "")}</li>
            `).join("") || "<li>暂无字段变更明细</li>"}
          </ul>
        </div>
        <div class="agent-change-submit">
          <button class="neutral-action" id="mockApplyAgentConfigApplication">模拟应用状态</button>
          <button class="danger-action" id="mockCancelAgentConfigApplication">模拟取消应用</button>
          <button class="neutral-action" id="previewAgentConfigRollbackRequest">回滚请求预检查</button>
        </div>
        <small>Mock 状态流转：只更新待应用记录状态，不会写入 Agent 配置，也不会生成 Runner job。</small>
      </div>
    </div>
  `;
}

async function runAgentConfigApplicationApply() {
  const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
  const approval = application ? approvalStatusForApplication(application) : null;
  if (!canMockApplyAgentConfigApplication(application, approval) || agentConfigApplyRunning) return;

  agentConfigApplyRunning = true;
  setAgentChangeFeedback("正在提交 Mock 应用状态...");
  renderAgentsPage(selectedAgentIndex);

  try {
    const result = await postAgentConfigApplicationApply(application.id, {
      secondConfirm: true,
      confirmText: "我确认仅执行 Agent 配置 Mock 应用状态流转",
      appliedBy: "local_user",
    });
    appData = await loadDashboardFromApi();
    selectedAgentConfigApplicationId = result.application?.id || selectedAgentConfigApplicationId;
    refreshSelectedAgentConfigVersionHistory({ force: true });
    renderDashboard();
    renderAgentsPage(selectedAgentIndex);
    setAgentChangeFeedback("已标记为已应用；Agent 配置未被修改。", "success");
  } catch (error) {
    setAgentChangeFeedback(`Mock 应用失败：${error.message}`, "error");
  } finally {
    agentConfigApplyRunning = false;
    renderAgentsPage(selectedAgentIndex);
  }
}

async function runAgentConfigApplicationCancel() {
  const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
  if (!canMockCancelAgentConfigApplication(application) || agentConfigCancelRunning) return;

  agentConfigCancelRunning = true;
  setAgentChangeFeedback("正在提交 Mock 取消状态...");
  renderAgentsPage(selectedAgentIndex);

  try {
    const result = await postAgentConfigApplicationCancel(application.id, {
      reason: "用户在控制台取消待应用 Agent 配置变更",
      cancelledBy: "local_user",
    });
    appData = await loadDashboardFromApi();
    selectedAgentConfigApplicationId = result.application?.id || selectedAgentConfigApplicationId;
    refreshSelectedAgentConfigVersionHistory({ force: true });
    renderDashboard();
    renderAgentsPage(selectedAgentIndex);
    setAgentChangeFeedback("已标记为已取消；Agent 配置未被修改。", "success");
  } catch (error) {
    setAgentChangeFeedback(`Mock 取消失败：${error.message}`, "error");
  } finally {
    agentConfigCancelRunning = false;
    renderAgentsPage(selectedAgentIndex);
  }
}

async function runAgentConfigRollbackRequestPreview() {
  const application = (appData.agentConfigApplications || []).find((item) => item.id === selectedAgentConfigApplicationId);
  if (!application || application.status !== "applied" || agentConfigRollbackRequestRunning) return;

  agentConfigRollbackRequestRunning = true;
  agentConfigRollbackPreviewByApplicationId.set(application.id, { loading: true });
  setAgentChangeFeedback("正在预检查回滚请求...");
  renderAgentsPage(selectedAgentIndex);

  try {
    const result = await postAgentConfigRollbackRequest(application.id, {
      secondConfirm: true,
      confirmText: "rollback request preview only",
      requestedBy: "local_user",
      reason: "preview rollback request without creating approval",
    });
    agentConfigRollbackPreviewByApplicationId.set(application.id, { result });
    const errors = Array.isArray(result.validationErrors) && result.validationErrors.length > 0
      ? `；${result.validationErrors.join(" / ")}`
      : "";
    const diffCount = result.rollbackPreview?.fieldCount ?? (result.restoreDiff || []).length;
    setAgentChangeFeedback(`回滚请求仍是禁用预检查：canCreateApproval=${result.canCreateApproval}，restore diff=${diffCount} 个字段${errors}`, "success");
  } catch (error) {
    agentConfigRollbackPreviewByApplicationId.set(application.id, { error: error.message });
    setAgentChangeFeedback(`回滚请求预检查失败：${error.message}`, "error");
  } finally {
    agentConfigRollbackRequestRunning = false;
    renderAgentsPage(selectedAgentIndex);
  }
}

async function runAgentChangeRequest() {
  const agent = appData.agents?.[selectedAgentIndex];
  if (!agent?.id || agentChangeRequestRunning) return;

  agentChangeRequestRunning = true;
  setAgentChangeFeedback("正在生成审批申请...");
  renderAgentsPage(selectedAgentIndex);

  try {
    const body = agentChangeRequestBody(agent, selectedAgentChangeType);
    const result = await postAgentChangeRequest(agent.id, body);
    appData = await loadDashboardFromApi();
    refreshSelectedAgentConfigVersionHistory({ force: true });
    renderDashboard();
    renderAgentsPage(selectedAgentIndex);
    renderApprovalPage(selectedApprovalIndex);
    setAgentChangeFeedback(`已生成审批申请：${result.approval?.id || "未知 ID"}`, "success");
  } catch (error) {
    setAgentChangeFeedback(`生成失败：${error.message}`, "error");
  } finally {
    agentChangeRequestRunning = false;
    renderAgentsPage(selectedAgentIndex);
  }
}

function approvalAction(status) {
  return statusConfig.approval?.[status]?.action || "查看";
}

function setApiStatus(mode, text) {
  const status = document.querySelector("#apiStatus");
  if (!status) return;
  status.className = `api-status ${mode}`;
  status.textContent = text;
}

function setApprovalFeedback(text, mode = "") {
  const feedback = document.querySelector("#approvalFeedback");
  if (!feedback) return;
  feedback.className = `approval-feedback ${mode}`.trim();
  feedback.textContent = text;
}

function setRuntimeStateFeedback(text, mode = "") {
  const feedback = document.querySelector("#runtimeStateFeedback");
  if (!feedback) return;
  feedback.className = `approval-feedback ${mode}`.trim();
  feedback.textContent = text;
}

function setTaskFeedback(text, mode = "") {
  const feedback = document.querySelector("#taskFeedback");
  if (!feedback) return;
  feedback.className = `approval-feedback ${mode}`.trim();
  feedback.textContent = text;
}

function setAgentChangeFeedback(text, mode = "") {
  const feedback = document.querySelector("#agentChangeFeedback");
  if (!feedback) return;
  feedback.className = `approval-feedback ${mode}`.trim();
  feedback.textContent = text;
}

function setProjectPlanFeedback(text, mode = "") {
  const feedback = document.querySelector("#projectPlanFeedback");
  if (!feedback) return;
  feedback.className = `approval-feedback ${mode}`.trim();
  feedback.textContent = text;
}

function setRuntimeStateButtons(disabled) {
  document.querySelectorAll("#exportRuntimeState, #resetRuntimeState, #clearRuntimeState")
    .forEach((button) => { button.disabled = disabled; });
}

function renderApprovalPage(selectedIndex = 0) {
  const approvals = pendingApprovalRequests();
  const list = document.querySelector("#approvalPageList");
  const count = document.querySelector("#approvalPageCount");
  const detail = document.querySelector("#approvalDetail");
  const detailRisk = document.querySelector("#approvalDetailRisk");
  const feedback = document.querySelector("#approvalFeedback");

  if (!list || !detail) return;

  if (approvals.length === 0) {
    if (count) count.textContent = "0";
    list.innerHTML = `<div><strong>暂无待审批项</strong><p>Runner 没有新的本地执行申请。</p></div>`;
    detail.innerHTML = `<div class="approval-meta"><div><span>当前状态</span><strong>无待处理审批</strong></div></div>`;
    if (detailRisk) {
      detailRisk.textContent = "安全";
      detailRisk.className = "badge green";
    }
    document.querySelectorAll("#patchOnlyAction, #rejectApprovalAction, #approveApprovalAction, #viewDiffAction")
      .forEach((button) => { button.disabled = true; });
    return;
  }

  selectedApprovalIndex = Math.min(Math.max(selectedIndex, 0), approvals.length - 1);
  if (count) count.textContent = approvals.length;

  list.innerHTML = approvals.map((item, index) => `
    <div class="${index === selectedApprovalIndex ? "active" : ""}" data-approval-index="${index}">
      <strong>${escapeHtml(item.file)}</strong>
      <p>修改类型：${escapeHtml(item.type)} · 申请人：${escapeHtml(item.agent)} · ${escapeHtml(statusLabel("approval", item.status))}</p>
      <span class="risk ${escapeHtml(item.riskTone)}">${escapeHtml(item.risk)}</span>
      <small>${escapeHtml(approvalAction(item.status))}</small>
    </div>
  `).join("");

  const item = approvals[selectedApprovalIndex] || approvals[0];
  const isAgentConfigApproval = item.targetService === "agent_config";
  const isProjectPlanApproval = item.targetService === "project_plan";
  const approvalEffectText = isAgentConfigApproval
    ? "只创建 Agent 配置审批申请，当前不会修改 Agent 配置，也不会进入 Runner 队列。"
    : isProjectPlanApproval
      ? "批准后创建 Agent 任务和只读 Runner request queue 记录；当前不会执行命令、写文件、调用模型或修改 Git。"
      : `只生成只读 Runner job 记录，展示 ${escapeHtml(item.affectedFiles.length)} 个影响文件；当前不会执行命令、写文件或修改 Git。`;
  const projectPlanApprovalNote = isProjectPlanApproval
    ? `
    <div class="approval-files project-plan-approval-note">
      <h3>MVP-0.3 project plan approval</h3>
      <ul>
        <li>批准后创建 frontend / backend / qa / docs / reviewer 五个 Agent 任务。</li>
        <li>同时生成五条只读 Runner request queue 记录。</li>
        <li>不会执行命令、写项目文件、调用真实模型、发起网络请求或修改 Git。</li>
      </ul>
    </div>
  `
    : "";
  if (detailRisk) {
    detailRisk.textContent = item.risk;
    detailRisk.className = `badge ${item.riskTone === "high" ? "red" : item.riskTone === "mid" ? "orange" : "green"}`;
  }

  detail.innerHTML = `
    <div class="approval-meta">
      <div><span>申请 Agent</span><strong>${escapeHtml(item.agent)}</strong></div>
      <div><span>当前状态</span><strong>${escapeHtml(statusLabel("approval", item.status))}</strong></div>
      <div><span>操作类型</span><strong>${escapeHtml(item.operationTypes.join(" / "))}</strong></div>
      <div><span>目标服务</span><strong>${escapeHtml(item.targetService)}</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>修改原因</span><strong>${escapeHtml(item.reason)}</strong></div>
      <div><span>审批后果</span><strong>${approvalEffectText}</strong></div>
    </div>
    <div class="approval-files">
      <h3>影响文件</h3>
      <ul>${item.affectedFiles.map((file) => `<li>${escapeHtml(file)}</li>`).join("")}</ul>
    </div>
    ${projectPlanApprovalNote}
    <div class="approval-diff">
      <h3>差异预览</h3>
      ${item.diffPreview.map((line) => `<code class="${line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : ""}">${escapeHtml(line)}</code>`).join("")}
    </div>
  `;

  const allowButton = document.querySelector("#approveApprovalAction");
  const patchOnlyButton = document.querySelector("#patchOnlyAction");
  const rejectButton = document.querySelector("#rejectApprovalAction");
  const viewDiffButton = document.querySelector("#viewDiffAction");
  const isPending = item.status === "pending";

  if (allowButton) {
    allowButton.disabled = !isPending || approvalActionRunning;
    allowButton.textContent = item.targetService === "agent_config"
      ? "批准 Agent 配置申请"
      : isProjectPlanApproval
        ? "批准计划并分配任务"
        : "批准并生成只读 Runner job";
  }
  if (patchOnlyButton) patchOnlyButton.disabled = !isPending || approvalActionRunning;
  if (rejectButton) rejectButton.disabled = !isPending || approvalActionRunning;
  if (viewDiffButton) viewDiffButton.disabled = approvalActionRunning;

  list.querySelectorAll("[data-approval-index]").forEach((row) => {
    row.addEventListener("click", () => renderApprovalPage(Number(row.dataset.approvalIndex)));
  });
}

function badgeClassForStatus(group, status) {
  const tone = statusTone(group, status);
  if (tone === "ok") return "green";
  if (tone === "warn") return "orange";
  if (tone === "danger") return "red";
  return "gray";
}

function riskLabel(riskLevel) {
  return riskLevel === "high" ? "高风险" : riskLevel === "medium" ? "中风险" : "低风险";
}

function renderWorkflowPage() {
  const workflow = appData.workflow;
  const page = document.querySelector("#workflowPage");
  if (!page || !workflow) return;

  const steps = workflow.steps || [];
  const stats = workflow.stats || [];
  const nodes = workflow.nodes || [];
  const edges = workflow.edges || [];
  const disabled = projectPlanRequestRunning ? "disabled" : "";

  page.innerHTML = `
    <div class="project-plan-request">
      <div>
        <span>MVP-0.3</span>
        <strong>项目计划审批</strong>
        <p>项目想法会先生成本地确定性计划草案，确认后才拆分 Agent 任务和只读 Runner request queue。</p>
      </div>
      <label>
        <span>项目想法</span>
        <textarea id="projectPlanIdea" rows="4" ${disabled}>做一个本地客户线索跟进工具</textarea>
      </label>
      <label>
        <span>约束</span>
        <input id="projectPlanConstraints" type="text" value="Mock/SQLite only; no real Runner; no real model calls" ${disabled}>
      </label>
      <div class="agent-change-submit">
        <button class="neutral-action" id="submitProjectPlanRequest" type="button" ${disabled}>生成计划审批草案</button>
      </div>
      <p class="approval-feedback" id="projectPlanFeedback" role="status"></p>
    </div>
    <div class="workflow-summary">
      <div>
        <span>当前流程</span>
        <strong>${escapeHtml(workflow.name || "未命名流程")}</strong>
        <p>${escapeHtml(workflow.description || "暂无流程说明")}</p>
      </div>
      <div>
        <span>状态</span>
        <strong>${escapeHtml(workflow.status || "unknown")}</strong>
        <p>${escapeHtml(workflow.updatedAt || "未记录更新时间")}</p>
      </div>
    </div>
    <div class="flow">
      ${steps.map((step, index) => `
        ${index === 0 ? "" : '<div class="line"></div>'}
        <div class="agent-step ${escapeHtml(step.tone || "purple")}">
          <b>${escapeHtml(step.name)}</b>
          <span>${escapeHtml(step.detail || "")}</span>
          <i style="--p:${escapeHtml(step.progress || "0%")}"></i>
        </div>
      `).join("")}
    </div>
    <div class="workflow-stats">
      ${stats.map(([label, value]) => `<div><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`).join("")}
    </div>
    <div class="workflow-readonly-grid">
      <section>
        <h3>节点</h3>
        <ul>${nodes.map((node) => `<li><b>${escapeHtml(node.label)}</b><span>${escapeHtml(node.type)}</span></li>`).join("")}</ul>
      </section>
      <section>
        <h3>依赖连线</h3>
        <ul>${edges.map((edge) => `<li><b>${escapeHtml(edge.from)} → ${escapeHtml(edge.to)}</b><span>${escapeHtml(edge.label || "依赖")}</span></li>`).join("")}</ul>
      </section>
    </div>
  `;

  const button = page.querySelector("#submitProjectPlanRequest");
  if (button) {
    button.addEventListener("click", () => runProjectPlanRequest());
  }
}

function runnerJobStatusLabel(status) {
  const labels = {
    queued: "只读排队",
    running: "Mock 流转中",
    succeeded: "已成功",
    failed: "已失败",
    cancelled: "已取消",
  };
  return labels[status] || status;
}

function runnerJobBadgeClass(status) {
  if (status === "failed") return "red";
  if (status === "running") return "orange";
  if (status === "cancelled") return "gray";
  return "green";
}

function runnerPermissionLabel(value) {
  if (value === true) return "允许";
  if (value === false) return "禁止";
  if (value === "approval_required") return "需要审批";
  return value || "未配置";
}

function renderRunnerStatus() {
  const runnerStatus = appData.runnerStatus || {};
  const statusBadge = document.querySelector("#runnerConnectionStatus");
  const detail = document.querySelector("#runnerStatusDetail");
  const permissions = runnerStatus.permissions || {};

  if (statusBadge) {
    statusBadge.textContent = runnerStatus.connected ? "已连接" : "未连接";
    statusBadge.className = `badge ${runnerStatus.connected ? "green" : "gray"}`;
  }

  if (!detail) return;

  detail.innerHTML = `
    <div class="approval-meta runner-status-meta">
      <div><span>Runner ID</span><strong>${escapeHtml(runnerStatus.runnerId || "未连接")}</strong></div>
      <div><span>版本</span><strong>${escapeHtml(runnerStatus.version || "未记录")}</strong></div>
      <div><span>工作区</span><strong>${escapeHtml(runnerStatus.workspacePath || "未配置")}</strong></div>
      <div><span>最后心跳</span><strong>${escapeHtml(runnerStatus.lastHeartbeatAt || "未记录")}</strong></div>
    </div>
    <div class="runner-permissions">
      <span>读文件：${escapeHtml(runnerPermissionLabel(permissions.readFiles))}</span>
      <span>写文件：${escapeHtml(runnerPermissionLabel(permissions.writeFiles))}</span>
      <span>执行命令：${escapeHtml(runnerPermissionLabel(permissions.executeCommands))}</span>
      <span>网络请求：${escapeHtml(runnerPermissionLabel(permissions.networkRequests))}</span>
    </div>
    <p class="runner-safety-note">当前为 Mock 只读状态页：Runner job 只是审批后的排队记录，不会执行本地命令、不会写文件、不会发起网络请求，也不会修改 Git。</p>
  `;
}

function renderRuntimePage(selectedIndex = selectedRunnerJobIndex) {
  const jobs = appData.runnerJobs || [];
  const count = document.querySelector("#runnerJobCount");
  const queuedCount = document.querySelector("#runnerQueuedCount");
  const failedCount = document.querySelector("#runnerFailedCount");
  const tableBody = document.querySelector("#runnerJobTable tbody");
  const detail = document.querySelector("#runnerJobDetail");
  const detailStatus = document.querySelector("#runnerJobDetailStatus");

  if (count) count.textContent = jobs.length;
  if (queuedCount) queuedCount.textContent = jobs.filter((job) => job.status === "queued").length;
  if (failedCount) failedCount.textContent = jobs.filter((job) => job.status === "failed").length;
  renderRunnerStatus();
  if (!tableBody || !detail) return;

  if (jobs.length === 0) {
    tableBody.innerHTML = `<tr><td colspan="5">暂无 Runner job。审批通过后只会生成只读记录。</td></tr>`;
    detail.innerHTML = `
      <div class="approval-meta">
        <div><span>当前状态</span><strong>暂无 Runner job</strong></div>
        <div><span>安全说明</span><strong>当前只读，不会执行本地命令、写文件或修改 Git。</strong></div>
      </div>
    `;
    if (detailStatus) {
      detailStatus.textContent = "只读";
      detailStatus.className = "badge orange";
    }
    return;
  }

  selectedRunnerJobIndex = Math.min(Math.max(selectedIndex, 0), jobs.length - 1);

  tableBody.innerHTML = jobs.map((job, index) => `
    <tr class="${index === selectedRunnerJobIndex ? "active" : ""}" data-runner-job-index="${index}">
      <td><strong>${escapeHtml(job.id)}</strong></td>
      <td>${escapeHtml(job.approvalId || "无")}</td>
      <td><span class="badge ${runnerJobBadgeClass(job.status)}">${escapeHtml(runnerJobStatusLabel(job.status))}</span></td>
      <td>${escapeHtml(job.checkpoint || "未记录")}</td>
      <td>${escapeHtml((job.affectedFiles || []).length)} 个文件</td>
    </tr>
  `).join("");

  const job = jobs[selectedRunnerJobIndex] || jobs[0];
  if (detailStatus) {
    detailStatus.textContent = runnerJobStatusLabel(job.status);
    detailStatus.className = `badge ${runnerJobBadgeClass(job.status)}`;
  }

  detail.innerHTML = `
    <div class="approval-meta">
      <div><span>Job ID</span><strong>${escapeHtml(job.id)}</strong></div>
      <div><span>来源审批</span><strong>${escapeHtml(job.approvalId || "无")}</strong></div>
      <div><span>关联任务</span><strong>${escapeHtml(job.taskId || "未关联")}</strong></div>
      <div><span>Git checkpoint</span><strong>${escapeHtml(job.checkpoint || "未记录")}</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>操作类型</span><strong>${escapeHtml((job.operationTypes || []).join(" / ") || "未记录")}</strong></div>
      <div><span>创建时间</span><strong>${escapeHtml(job.createdAt || "未记录")}</strong></div>
      <div><span>更新时间</span><strong>${escapeHtml(job.updatedAt || "未记录")}</strong></div>
      <div><span>安全说明</span><strong>当前只读，只表示已批准记录；不会执行本地命令、写文件或修改 Git。</strong></div>
    </div>
    <div class="task-files">
      <h3>影响文件</h3>
      <ul>${(job.affectedFiles || []).map((file) => `<li>${escapeHtml(file)}</li>`).join("") || "<li>暂无影响文件</li>"}</ul>
    </div>
  `;

  tableBody.querySelectorAll("[data-runner-job-index]").forEach((row) => {
    row.addEventListener("click", () => renderRuntimePage(Number(row.dataset.runnerJobIndex)));
  });
}

function renderTaskPage(selectedIndex = null) {
  const tasks = appData.taskQueue || [];
  const tableBody = document.querySelector("#taskPageTable tbody");
  const count = document.querySelector("#taskPageCount");
  const detail = document.querySelector("#taskDetail");
  const statusBadge = document.querySelector("#taskDetailStatus");

  if (!tableBody || !detail) return;

  if (tasks.length === 0) {
    if (count) count.textContent = "0";
    tableBody.innerHTML = `<tr><td colspan="4">暂无任务</td></tr>`;
    detail.innerHTML = `<div class="approval-meta"><div><span>当前状态</span><strong>无任务</strong></div></div>`;
    document.querySelectorAll("#startTaskAction, #completeTaskAction, #failTaskAction, #cancelTaskAction")
      .forEach((button) => { button.disabled = true; });
    return;
  }

  const nextIndex = Number.isInteger(selectedIndex) ? selectedIndex : defaultTaskIndex(tasks);
  selectedTaskIndex = Math.min(Math.max(nextIndex, 0), tasks.length - 1);
  if (count) count.textContent = tasks.length;

  tableBody.innerHTML = tasks.map((task, index) => `
    <tr class="${index === selectedTaskIndex ? "active" : ""}" data-task-index="${index}">
      <td><strong>${escapeHtml(task.title)}</strong></td>
      <td>${escapeHtml(task.assignedAgentName || "未分配")}</td>
      <td><span class="badge ${badgeClassForStatus("task", task.status)}">${escapeHtml(statusLabel("task", task.status))}</span></td>
      <td>${escapeHtml(riskLabel(task.riskLevel))}</td>
    </tr>
  `).join("");

  const task = tasks[selectedTaskIndex] || tasks[0];
  if (statusBadge) {
    statusBadge.textContent = statusLabel("task", task.status);
    statusBadge.className = `badge ${badgeClassForStatus("task", task.status)}`;
  }

  detail.innerHTML = `
    <div class="approval-meta">
      <div><span>任务标题</span><strong>${escapeHtml(task.title)}</strong></div>
      <div><span>负责人</span><strong>${escapeHtml(task.assignedAgentName || "未分配")}</strong></div>
      <div><span>优先级</span><strong>${escapeHtml(task.priority || "未设置")}</strong></div>
      <div><span>风险等级</span><strong>${escapeHtml(riskLabel(task.riskLevel))}</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>任务说明</span><strong>${escapeHtml(task.description || "暂无说明")}</strong></div>
      <div><span>依赖任务</span><strong>${escapeHtml(task.dependsOn?.length ? task.dependsOn.join(" / ") : "无")}</strong></div>
      <div><span>是否需要审批</span><strong>${task.requiresApproval ? "是" : "否"}</strong></div>
      <div><span>最近更新</span><strong>${escapeHtml(task.updatedAt || task.completedAt || task.startedAt || "未记录")}</strong></div>
    </div>
    <div class="task-files">
      <h3>关联文件</h3>
      <ul>${(task.relatedFiles || []).map((file) => `<li>${escapeHtml(file)}</li>`).join("") || "<li>暂无关联文件</li>"}</ul>
    </div>
  `;

  const startButton = document.querySelector("#startTaskAction");
  const completeButton = document.querySelector("#completeTaskAction");
  const failButton = document.querySelector("#failTaskAction");
  const cancelButton = document.querySelector("#cancelTaskAction");
  const isTerminal = ["completed", "failed", "cancelled"].includes(task.status);

  if (startButton) startButton.disabled = taskActionRunning || !["queued", "blocked", "waiting_user", "failed", "cancelled"].includes(task.status);
  if (completeButton) completeButton.disabled = taskActionRunning || task.status !== "running";
  if (failButton) failButton.disabled = taskActionRunning || isTerminal;
  if (cancelButton) cancelButton.disabled = taskActionRunning || isTerminal;

  tableBody.querySelectorAll("[data-task-index]").forEach((row) => {
    row.addEventListener("click", () => renderTaskPage(Number(row.dataset.taskIndex)));
  });
}

async function renderLocalTrialStatus() {
  const container = document.querySelector("#localTrialStatus");
  const modeBadge = document.querySelector("#localTrialModeBadge");
  const runtimeBadge = document.querySelector("#runtimeStateModeBadge");
  if (!container) return;

  try {
    const result = await requestRuntimeState("GET");
    const info = result.localTrial || {};
    const mode = result.mode || info.mode || "unknown";
    const persistence = info.persistence === "sqlite" ? "SQLite 数据库" : "Mock runtime-state 文件";
    const storagePath = mode === "sqlite" ? info.sqliteDbFile : info.runtimeStateFile;
    const realApplyEnabled = info.safety?.agentConfigRealApplyEnabled === true;

    if (modeBadge) {
      modeBadge.textContent = localTrialModeLabel(mode);
      modeBadge.className = localTrialBadgeClass(mode);
    }
    if (runtimeBadge) {
      runtimeBadge.textContent = persistence;
      runtimeBadge.className = localTrialBadgeClass(mode);
    }

    container.innerHTML = `
      <div class="local-trial-grid">
        <div><span>当前模式</span><strong>${escapeHtml(localTrialModeLabel(mode))}</strong></div>
        <div><span>状态保存</span><strong>${escapeHtml(persistence)}</strong></div>
        <div><span>API 地址</span><strong>${escapeHtml(info.apiUrl || apiBase)}</strong></div>
        <div><span>Web 地址</span><strong>${escapeHtml(info.webUrl || window.location.href)}</strong></div>
        <div><span>Agent config real apply</span><strong>${realApplyEnabled ? "feature flag enabled" : "off by default"}</strong></div>
        <div class="wide-row"><span>状态文件</span><strong>${escapeHtml(storagePath || "未返回")}</strong></div>
      </div>
      <div class="local-command-list">
        <div><span>查看状态</span><code>${escapeHtml(info.commands?.status || "powershell -ExecutionPolicy Bypass -File scripts\\status-local.ps1")}</code></div>
        <div><span>停止试用</span><code>${escapeHtml(info.commands?.stop || "powershell -ExecutionPolicy Bypass -File scripts\\stop-local.ps1")}</code></div>
        <div><span>重置数据</span><code>${escapeHtml(info.commands?.reset || "Invoke-RestMethod -Method Post http://127.0.0.1:8787/api/runtime-state/reset")}</code></div>
      </div>
      <p class="runner-safety-note">当前仍不会执行本地命令、不会写文件、不会调用真实模型、不会云同步。网页只展示命令，不会替你停止本地进程。</p>
    `;
  } catch (error) {
    if (modeBadge) {
      modeBadge.textContent = "离线";
      modeBadge.className = "badge red";
    }
    if (runtimeBadge) {
      runtimeBadge.textContent = "不可确认";
      runtimeBadge.className = "badge red";
    }
    container.innerHTML = `
      <div class="local-trial-grid">
        <div><span>当前状态</span><strong>无法连接本地 API</strong></div>
        <div><span>建议操作</span><strong>运行 scripts\\start-local.ps1</strong></div>
      </div>
      <p class="approval-feedback error">读取本地试用状态失败：${escapeHtml(error.message)}</p>
    `;
  }
}

function ensureModelGatewayContainers() {
  const integrationsGrid = document.querySelector("#integrations .subpage-grid");
  if (integrationsGrid && !document.querySelector("#modelGatewayIntegrationStatus")) {
    integrationsGrid.insertAdjacentHTML("beforeend", `
      <article class="panel wide">
        <div class="card-head">
          <h2>Model Gateway</h2>
          <span class="badge orange" id="modelGatewayIntegrationBadge">disabled</span>
        </div>
        <div class="model-gateway-status" id="modelGatewayIntegrationStatus">
          <p class="muted">Reading model gateway status...</p>
        </div>
      </article>
    `);
  }

  const settingsPanel = document.querySelector("#settings .panel.wide");
  if (settingsPanel && !document.querySelector("#modelGatewaySettingsStatus")) {
    settingsPanel.insertAdjacentHTML("beforeend", `
      <div class="model-gateway-status" id="modelGatewaySettingsStatus">
        <p class="muted">Reading model gateway status...</p>
      </div>
    `);
  }
}

function renderModelGatewayStatusCard(status) {
  const providers = status.providers || [];
  const blockedReasons = status.blockedReasons || [];
  const safety = status.safety || {};

  return `
    <div class="model-gateway-summary">
      <div><span>Gateway</span><strong>${escapeHtml(status.gatewayMode || "disabled")}</strong></div>
      <div><span>Real calls</span><strong>${status.realModelCallsAllowed ? "allowed" : "disabled"}</strong></div>
      <div><span>Boundary</span><strong>${escapeHtml(status.serviceBoundary || "server_only")}</strong></div>
    </div>
    <div class="model-provider-list">
      ${providers.map((provider) => `
        <div>
          <strong>${escapeHtml(provider.label || provider.id)}</strong>
          <span class="badge ${provider.configured ? "green" : "gray"}">${provider.configured ? "env present" : "env missing"}</span>
          <small>${escapeHtml(provider.keyEnvVar || "")}</small>
        </div>
      `).join("")}
    </div>
    <p class="runner-safety-note">API keys stay server-side. This status endpoint does not store keys, expose keys to the frontend, create tasks, create approvals, create Runner jobs, write the database, or make provider network requests.</p>
    <ul class="model-gateway-reasons">
      ${blockedReasons.map((reason) => `<li>${escapeHtml(reason)}</li>`).join("")}
      ${safety.makesNetworkRequests === false ? "<li>Provider network requests are disabled.</li>" : ""}
    </ul>
  `;
}

function renderModelGatewayDryRunCard(result) {
  const sideEffects = result.sideEffects || {};
  const sideEffectItems = [
    ["SQLite writes", sideEffects.writesSqlite],
    ["Runtime state writes", sideEffects.writesRuntimeState],
    ["Task creation", sideEffects.createsTasks],
    ["Approval creation", sideEffects.createsApprovals],
    ["Runner job creation", sideEffects.createsRunnerJobs],
    ["Agent trigger", sideEffects.triggersAgents],
    ["Real model call", sideEffects.callsRealModel],
    ["Prompt/result logging", sideEffects.logsPromptOrResult],
  ];
  const errors = result.validationErrors || [];

  return `
    <div class="model-gateway-dryrun" id="modelGatewayDryRunResult">
      <div class="card-head compact">
        <h3>Connectivity Dry-Run</h3>
        <span class="badge ${result.requestValid ? "green" : "orange"}">${result.requestValid ? "request valid" : "request blocked"}</span>
      </div>
      <div class="model-gateway-summary dryrun-summary">
        <div><span>Provider</span><strong>${escapeHtml(result.provider || "openai")}</strong></div>
        <div><span>Env var</span><strong>${result.keyConfigured ? "present" : "missing"}</strong></div>
        <div><span>Would call provider</span><strong>${result.wouldCallProvider ? "yes" : "no"}</strong></div>
      </div>
      <div class="model-gateway-side-effects">
        ${sideEffectItems.map(([label, active]) => `
          <div>
            <span>${escapeHtml(label)}</span>
            <strong class="${active ? "risk-on" : "risk-off"}">${active ? "yes" : "no"}</strong>
          </div>
        `).join("")}
      </div>
      ${errors.length > 0 ? `<ul class="model-gateway-reasons">${errors.map((error) => `<li>${escapeHtml(error)}</li>`).join("")}</ul>` : ""}
      <p class="runner-safety-note">Dry-run is a backend-only preview. It validates provider configuration and safety switches, but it does not send prompts, call providers, write storage, create tasks, create approvals, trigger Agents, or create Runner jobs.</p>
    </div>
  `;
}

async function renderModelGatewayStatus() {
  ensureModelGatewayContainers();
  const containers = [
    document.querySelector("#modelGatewayIntegrationStatus"),
    document.querySelector("#modelGatewaySettingsStatus"),
  ].filter(Boolean);
  const badge = document.querySelector("#modelGatewayIntegrationBadge");
  if (containers.length === 0) return;

  try {
    const status = await requestModelGatewayStatus();
    const dryRun = await requestModelGatewayDryRun();
    const html = `${renderModelGatewayStatusCard(status)}${renderModelGatewayDryRunCard(dryRun)}`;
    containers.forEach((container) => { container.innerHTML = html; });
    if (badge) {
      badge.textContent = status.realModelCallsAllowed ? "enabled" : "disabled";
      badge.className = `badge ${status.realModelCallsAllowed ? "green" : "orange"}`;
    }
  } catch (error) {
    containers.forEach((container) => {
      container.innerHTML = `<p class="approval-feedback error">Model Gateway status unavailable: ${escapeHtml(error.message)}</p>`;
    });
    if (badge) {
      badge.textContent = "offline";
      badge.className = "badge red";
    }
  }
}

async function runProjectPlanRequest() {
  if (projectPlanRequestRunning) return;

  const idea = document.querySelector("#projectPlanIdea")?.value?.trim() || "";
  const constraints = document.querySelector("#projectPlanConstraints")?.value?.trim() || "";
  if (!idea) {
    setProjectPlanFeedback("请输入项目想法。", "error");
    return;
  }

  projectPlanRequestRunning = true;
  setProjectPlanFeedback("正在生成计划审批草案...");
  renderWorkflowPage();

  let feedbackText = "";
  let feedbackMode = "";
  try {
    const result = await postProjectPlanRequest({
      idea,
      constraints,
      requestedBy: "local_user",
    });
    appData = await loadDashboardFromApi();
    renderDashboard();
    renderAgentsPage();
    renderWorkflowPage();
    renderRuntimePage();
    renderTaskPage(selectedTaskIndex);
    renderApprovalPage(selectedApprovalIndex);
    feedbackText = `已生成计划审批草案：${result.approval?.id || "未知 ID"}`;
    feedbackMode = "success";
  } catch (error) {
    feedbackText = `生成失败：${error.message}`;
    feedbackMode = "error";
  } finally {
    projectPlanRequestRunning = false;
    renderWorkflowPage();
    if (feedbackText) {
      setProjectPlanFeedback(feedbackText, feedbackMode);
    }
  }
}

async function runApprovalAction(action) {
  const item = pendingApprovalRequests()[selectedApprovalIndex];
  if (!item?.id || approvalActionRunning) return;

  const actionLabels = {
    approve: item.targetService === "agent_config"
      ? "批准 Agent 配置申请"
      : item.targetService === "project_plan"
        ? "批准计划并分配任务"
        : "生成只读 Runner job",
    reject: "拒绝",
    "patch-only": "只生成补丁",
  };

  approvalActionRunning = true;
  setApprovalFeedback(`正在提交：${actionLabels[action]}...`);
  renderApprovalPage(selectedApprovalIndex);

  try {
    const body = action === "approve"
      ? { secondConfirm: item.requiresSecondConfirm || item.riskTone === "high" }
      : action === "reject"
        ? { reason: "用户在控制台拒绝本次 Runner 申请" }
        : {};

    await postApprovalAction(item.id, action, body);
    appData = await loadDashboardFromApi();
    selectedApprovalIndex = Math.min(selectedApprovalIndex, Math.max(pendingApprovalRequests().length - 1, 0));
    refreshSelectedAgentConfigVersionHistory({ force: true });
    renderDashboard();
    renderAgentsPage();
    renderWorkflowPage();
    renderRuntimePage();
    renderApprovalPage(selectedApprovalIndex);
    setApprovalFeedback(`已提交：${actionLabels[action]}`, "success");
  } catch (error) {
    setApprovalFeedback(`提交失败：${error.message}`, "error");
  } finally {
    approvalActionRunning = false;
    renderApprovalPage(selectedApprovalIndex);
  }
}

async function runTaskAction(action) {
  const task = appData.taskQueue?.[selectedTaskIndex];
  if (!task?.id || taskActionRunning) return;

  const actionLabels = {
    start: "开始任务",
    complete: "标记完成",
    fail: "标记失败",
    cancel: "取消任务",
  };

  taskActionRunning = true;
  setTaskFeedback(`正在提交：${actionLabels[action]}...`);
  renderTaskPage(selectedTaskIndex);

  try {
    const body = action === "fail" ? { reason: "用户在控制台标记任务失败" } : {};
    await postTaskAction(task.id, action, body);
    appData = await loadDashboardFromApi();
    const tasks = appData.taskQueue || [];
    selectedTaskIndex = Math.min(selectedTaskIndex, Math.max(tasks.length - 1, 0));
    if (!taskHasEnabledAction(tasks[selectedTaskIndex])) {
      selectedTaskIndex = defaultTaskIndex(tasks);
    }
    refreshSelectedAgentConfigVersionHistory({ force: true });
    renderDashboard();
    renderAgentsPage();
    renderWorkflowPage();
    renderRuntimePage();
    renderTaskPage(selectedTaskIndex);
    setTaskFeedback(`已提交：${actionLabels[action]}`, "success");
  } catch (error) {
    setTaskFeedback(`提交失败：${error.message}`, "error");
  } finally {
    taskActionRunning = false;
    renderTaskPage(selectedTaskIndex);
  }
}

document.querySelector("#viewDiffAction")?.addEventListener("click", () => {
  document.querySelector(".approval-diff")?.scrollIntoView({ behavior: "smooth", block: "nearest" });
});

document.querySelector("#patchOnlyAction")?.addEventListener("click", () => {
  runApprovalAction("patch-only");
});

document.querySelector("#rejectApprovalAction")?.addEventListener("click", () => {
  runApprovalAction("reject");
});

document.querySelector("#approveApprovalAction")?.addEventListener("click", () => {
  runApprovalAction("approve");
});

document.querySelector("#startTaskAction")?.addEventListener("click", () => {
  runTaskAction("start");
});

document.querySelector("#completeTaskAction")?.addEventListener("click", () => {
  runTaskAction("complete");
});

document.querySelector("#failTaskAction")?.addEventListener("click", () => {
  runTaskAction("fail");
});

document.querySelector("#cancelTaskAction")?.addEventListener("click", () => {
  runTaskAction("cancel");
});

async function runRuntimeStateAction(action) {
  if (runtimeStateRunning) return;

  const actionText = {
    export: "导出状态",
    reset: "恢复 Seed 数据",
    clear: "清理运行态并恢复 Seed",
  }[action];

  runtimeStateRunning = true;
  setRuntimeStateButtons(true);
  setRuntimeStateFeedback(`正在处理：${actionText}...`);

  try {
    if (action === "export") {
      const result = await requestRuntimeState("GET");
      const blob = new Blob([`${JSON.stringify(result.state, null, 2)}\n`], { type: "application/json" });
      const link = document.createElement("a");
      link.href = URL.createObjectURL(blob);
      link.download = `agent-swarm-runtime-state-${new Date().toISOString().slice(0, 10)}.json`;
      link.click();
      URL.revokeObjectURL(link.href);
    } else if (action === "reset") {
      await requestRuntimeState("POST", "/api/runtime-state/reset");
      appData = await loadDashboardFromApi();
      refreshSelectedAgentConfigVersionHistory({ force: true });
      renderDashboard();
      renderAgentsPage();
      renderWorkflowPage();
      renderRuntimePage();
      renderApprovalPage(selectedApprovalIndex);
      renderTaskPage(selectedTaskIndex);
      await renderLocalTrialStatus();
    } else if (action === "clear") {
      await requestRuntimeState("DELETE");
      appData = await loadDashboardFromApi();
      refreshSelectedAgentConfigVersionHistory({ force: true });
      renderDashboard();
      renderAgentsPage();
      renderWorkflowPage();
      renderRuntimePage();
      renderApprovalPage(selectedApprovalIndex);
      renderTaskPage(selectedTaskIndex);
      await renderLocalTrialStatus();
    }

    const completionText = action === "export"
      ? "已导出当前本地试用状态快照"
      : `${actionText}已完成；不会删除 SQLite 数据库文件，也不会停止本地服务或执行 Runner。`;
    setRuntimeStateFeedback(completionText, "success");
  } catch (error) {
    setRuntimeStateFeedback(`处理失败：${error.message}`, "error");
  } finally {
    runtimeStateRunning = false;
    setRuntimeStateButtons(false);
  }
}

document.querySelector("#exportRuntimeState")?.addEventListener("click", () => {
  runRuntimeStateAction("export");
});

document.querySelector("#resetRuntimeState")?.addEventListener("click", () => {
  runRuntimeStateAction("reset");
});

document.querySelector("#clearRuntimeState")?.addEventListener("click", () => {
  runRuntimeStateAction("clear");
});

function activatePage(page) {
  const title = titles[page];
  if (!title) return;

  document
    .querySelectorAll("[data-page]")
    .forEach((item) => item.classList.toggle("active", item.dataset.page === page));

  document
    .querySelectorAll(".page-view")
    .forEach((view) => view.classList.toggle("active", view.id === page));

  document.querySelector("#pageTitle").textContent = title[0];
  document.querySelector("#pageCrumb").textContent = title[1];
}

document.querySelectorAll("[data-page]").forEach((button) => {
  button.addEventListener("click", () => activatePage(button.dataset.page));
});

document.querySelectorAll("[data-page-link]").forEach((button) => {
  button.addEventListener("click", () => activatePage(button.dataset.pageLink));
});

async function boot() {
  setApiStatus("connecting", "本地 API：连接中");
  try {
    appData = await loadDashboardFromApi();
    setApiStatus("connected", "本地 API：已连接");
  } catch (error) {
    console.info("Using local fallback data:", error.message);
    setApiStatus("offline", "本地 API：离线模式");
  }

  renderDashboard();
  renderAgentsPage();
  renderWorkflowPage();
  renderRuntimePage();
  renderApprovalPage();
  renderTaskPage();
  renderLocalTrialStatus();
  renderModelGatewayStatus();
}

boot();
