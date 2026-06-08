const titles = window.AGENT_SWARM_NAV || {};
let appData = window.AGENT_SWARM_DATA || {};
const statusConfig = window.AGENT_SWARM_STATUS || {};
const apiBase = "http://127.0.0.1:8787";
const projectId = "project_agent_swarm";

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
    approvalList.innerHTML = appData.approvalRequests.map((item) => `
      <div>
        <strong>${escapeHtml(item.file)}</strong>
        <p>修改类型：${escapeHtml(item.type)} · 申请人：${escapeHtml(item.agent)}</p>
        <span class="risk ${escapeHtml(item.riskTone)}">${escapeHtml(item.risk)}</span><small>${escapeHtml(item.diff)}</small>
      </div>
    `).join("");
    document.querySelector("#approvalSummary").textContent = `共 ${appData.approvalRequests.length} 项待审批`;
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

function normalizeDashboard(apiData) {
  const fallback = window.AGENT_SWARM_DATA || {};
  const pendingApprovals = apiData.pendingApprovals || [];
  const taskQueue = apiData.taskQueue || [];
  const agentStatus = apiData.agentStatus || [];

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
    workflow: fallback.workflow,
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
    })),
    taskQueue: taskQueue.map((task) => ({
      icon: task.priority === "high" ? "!" : "T",
      tone: task.priority === "high" ? "red" : "purple",
      title: task.title,
      type: task.priority === "high" ? "高优先级" : "任务",
      eta: task.status,
      status: task.status,
    })),
    agents: agentStatus.map((agent, index) => ({
      avatar: String.fromCharCode(65 + index),
      name: agent.name,
      version: agent.version,
      status: agent.status,
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

function statusLabel(group, status) {
  return statusConfig[group]?.[status]?.label || status;
}

function statusTone(group, status) {
  return statusConfig[group]?.[status]?.tone || "neutral";
}

function approvalAction(status) {
  return statusConfig.approval?.[status]?.action || "查看";
}

function renderApprovalPage(selectedIndex = 0) {
  const approvals = appData.approvalRequests || [];
  const list = document.querySelector("#approvalPageList");
  const count = document.querySelector("#approvalPageCount");
  const detail = document.querySelector("#approvalDetail");
  const detailRisk = document.querySelector("#approvalDetailRisk");

  if (!list || !detail || approvals.length === 0) return;

  if (count) count.textContent = approvals.length;

  list.innerHTML = approvals.map((item, index) => `
    <div class="${index === selectedIndex ? "active" : ""}" data-approval-index="${index}">
      <strong>${escapeHtml(item.file)}</strong>
      <p>修改类型：${escapeHtml(item.type)} · 申请人：${escapeHtml(item.agent)} · ${escapeHtml(statusLabel("approval", item.status))}</p>
      <span class="risk ${escapeHtml(item.riskTone)}">${escapeHtml(item.risk)}</span>
      <small>${escapeHtml(approvalAction(item.status))}</small>
    </div>
  `).join("");

  const item = approvals[selectedIndex] || approvals[0];
  if (detailRisk) {
    detailRisk.textContent = item.risk;
    detailRisk.className = `badge ${item.riskTone === "high" ? "red" : item.riskTone === "mid" ? "orange" : "green"}`;
  }

  detail.innerHTML = `
    <div class="approval-meta">
      <div><span>申请 Agent</span><strong>${escapeHtml(item.agent)}</strong></div>
      <div><span>当前状态</span><strong>${escapeHtml(statusLabel("approval", item.status))}</strong></div>
      <div><span>操作类型</span><strong>${escapeHtml(item.operationTypes.join(" / "))}</strong></div>
      <div><span>Git checkpoint</span><strong>${escapeHtml(item.checkpoint)}</strong></div>
    </div>
    <div class="approval-meta">
      <div><span>修改原因</span><strong>${escapeHtml(item.reason)}</strong></div>
      <div><span>执行后果</span><strong>会影响 ${escapeHtml(item.affectedFiles.length)} 个本地文件，执行前必须由用户确认。</strong></div>
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
  if (allowButton) {
    allowButton.disabled = item.status !== "pending";
    allowButton.textContent = item.riskTone === "high" ? "二次确认后允许执行" : "允许执行";
  }

  list.querySelectorAll("[data-approval-index]").forEach((row) => {
    row.addEventListener("click", () => renderApprovalPage(Number(row.dataset.approvalIndex)));
  });
}

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
  try {
    appData = await loadDashboardFromApi();
  } catch (error) {
    console.info("Using local fallback data:", error.message);
  }

  renderDashboard();
  renderApprovalPage();
}

boot();
