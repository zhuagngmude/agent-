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
    const pendingApprovalCount = appData.approvalRequests.filter((item) => item.status === "pending").length;
    approvalList.innerHTML = appData.approvalRequests.map((item) => `
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
    taskQueue.innerHTML = appData.taskQueue.map((item) => `
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
    gitCheckpointList.innerHTML = appData.gitCheckpoints.map((item) => `
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
    board.innerHTML = `<div><b>暂无智能体</b><span>Mock API 当前没有返回 Agent 数据。</span><em>只读</em></div>`;
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
    dashboardMetrics: [
      { label: "活跃智能体", value: apiData.metrics?.activeAgents ?? "-", note: "来自 mock API", tone: "purple", icon: "A" },
      { label: "待确认事项", value: apiData.metrics?.pendingApprovals ?? "-", note: "Runner 审批优先", tone: "orange", icon: "!" },
      { label: "活跃任务", value: apiData.metrics?.activeTasks ?? "-", note: "运行中与排队中", tone: "blue", icon: "T" },
      { label: "Git 检查点", value: apiData.metrics?.gitCheckpoints ?? "-", note: "项目保存点", tone: "green", icon: "G" },
      { label: "Token 消耗", value: apiData.metrics?.tokenUsage ?? "-", note: "预算追踪", tone: "violet", icon: "K" },
      { label: "模型使用", value: apiData.metrics?.modelCount ?? "-", note: "模型配置数", tone: "cyan", icon: "M" },
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

async function requestRuntimeState(method, path = "/api/runtime-state") {
  const response = await fetch(`${apiBase}${path}`, { method });
  const result = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(result.message || result.error || `API returned ${response.status}`);
  }
  return result;
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
      changes: [
        ["permissions", (agent.permissions || []).join(" / ") || "无", `${(agent.permissions || []).join(" / ")} / request_code_execution`],
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

function renderAgentConfigApplications(agent) {
  const applications = (appData.agentConfigApplications || [])
    .filter((item) => item.agentId === agent.id);

  if (applications.length === 0) {
    return `<p class="muted">当前 Agent 暂无待应用配置变更。审批通过后会先出现在这里，不会直接修改 Agent 配置。</p>`;
  }

  if (!applications.some((item) => item.id === selectedAgentConfigApplicationId)) {
    selectedAgentConfigApplicationId = applications[0].id;
  }

  const selectedApplication = applications.find((item) => item.id === selectedAgentConfigApplicationId) || applications[0];
  const selectedApproval = approvalStatusForApplication(selectedApplication);

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
        <div class="application-checklist">
          <h3>应用审计记录</h3>
          <ul>${renderAgentConfigApplicationAudit(selectedApplication, selectedApproval)}</ul>
        </div>
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

function setRuntimeStateButtons(disabled) {
  document.querySelectorAll("#exportRuntimeState, #resetRuntimeState, #clearRuntimeState")
    .forEach((button) => { button.disabled = disabled; });
}

function renderApprovalPage(selectedIndex = 0) {
  const approvals = appData.approvalRequests || [];
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
      <div><span>执行后果</span><strong>${item.targetService === "agent_config" ? "只创建 Agent 配置审批申请，当前不会修改 Agent 配置，也不会进入 Runner 队列。" : `会影响 ${escapeHtml(item.affectedFiles.length)} 个本地文件，执行前必须由用户确认。`}</strong></div>
    </div>
    <div class="approval-files">
      <h3>影响文件</h3>
      <ul>${item.affectedFiles.map((file) => `<li>${escapeHtml(file)}</li>`).join("")}</ul>
    </div>
    <div class="approval-diff">
      <h3>差异预览</h3>
      ${item.diffPreview.map((line) => `<code class="${line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : ""}">${escapeHtml(line)}</code>`).join("")}
    </div>
  `;

  const allowButton = document.querySelector(".danger-action");
  const patchOnlyButton = document.querySelector("#patchOnlyAction");
  const rejectButton = document.querySelector("#rejectApprovalAction");
  const viewDiffButton = document.querySelector("#viewDiffAction");
  const isPending = item.status === "pending";

  if (allowButton) {
    allowButton.disabled = !isPending || approvalActionRunning;
    allowButton.textContent = item.requiresSecondConfirm || item.riskTone === "high" ? "二次确认后允许执行" : "允许执行";
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

  page.innerHTML = `
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
}

function runnerJobStatusLabel(status) {
  const labels = {
    queued: "等待执行",
    running: "执行中",
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
    <p class="runner-safety-note">当前为 Mock 只读状态页：不会执行本地命令、不会写文件、不会发起网络请求，也不会修改 Git。</p>
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
    tableBody.innerHTML = `<tr><td colspan="5">暂无 Runner job。审批通过后会出现在这里。</td></tr>`;
    detail.innerHTML = `
      <div class="approval-meta">
        <div><span>当前状态</span><strong>暂无 Runner job</strong></div>
        <div><span>安全说明</span><strong>当前只读，不会执行本地命令。</strong></div>
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
      <div><span>安全说明</span><strong>当前只读，不会执行本地命令。</strong></div>
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

function renderTaskPage(selectedIndex = 0) {
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

  selectedTaskIndex = Math.min(Math.max(selectedIndex, 0), tasks.length - 1);
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

async function runApprovalAction(action) {
  const item = appData.approvalRequests?.[selectedApprovalIndex];
  if (!item?.id || approvalActionRunning) return;

  const actionLabels = {
    approve: "允许执行",
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
    selectedApprovalIndex = Math.min(selectedApprovalIndex, Math.max((appData.approvalRequests || []).length - 1, 0));
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
    selectedTaskIndex = Math.min(selectedTaskIndex, Math.max((appData.taskQueue || []).length - 1, 0));
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
    reset: "重置 Mock 数据",
    clear: "清理状态文件",
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
      renderDashboard();
      renderAgentsPage();
      renderWorkflowPage();
      renderRuntimePage();
      renderApprovalPage(selectedApprovalIndex);
      renderTaskPage(selectedTaskIndex);
    } else if (action === "clear") {
      await requestRuntimeState("DELETE");
      appData = await loadDashboardFromApi();
      renderDashboard();
      renderAgentsPage();
      renderWorkflowPage();
      renderRuntimePage();
      renderApprovalPage(selectedApprovalIndex);
      renderTaskPage(selectedTaskIndex);
    }

    setRuntimeStateFeedback(`已完成：${actionText}`, "success");
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

document.querySelectorAll("[data-page]").forEach((button) => {
  button.addEventListener("click", () => {
    const page = button.dataset.page;
    const title = titles[page];

    if (!title) return;

    document
      .querySelectorAll("[data-page]")
      .forEach((item) => item.classList.toggle("active", item === button));

    document
      .querySelectorAll(".page-view")
      .forEach((view) => view.classList.toggle("active", view.id === page));

    document.querySelector("#pageTitle").textContent = title[0];
    document.querySelector("#pageCrumb").textContent = title[1];
  });
});

async function boot() {
  setApiStatus("connecting", "Mock API：连接中");
  try {
    appData = await loadDashboardFromApi();
    setApiStatus("connected", "Mock API：已连接");
  } catch (error) {
    console.info("Using local fallback data:", error.message);
    setApiStatus("offline", "Mock API：离线模式");
  }

  renderDashboard();
  renderAgentsPage();
  renderWorkflowPage();
  renderRuntimePage();
  renderApprovalPage();
  renderTaskPage();
}

boot();
