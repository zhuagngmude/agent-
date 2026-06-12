const agentRunRoleCatalog = [
  {
    role: "architect",
    label: "Architect",
    agentId: "agent_architect",
    agentName: "架构师 Agent",
    model: "gpt-high-reasoning",
    summaryFocus: "总体方案",
    parentRole: "",
  },
  {
    role: "scheduler",
    label: "Scheduler",
    agentId: "",
    agentName: "Scheduler",
    model: "local-scheduler",
    summaryFocus: "任务拆分",
    parentRole: "architect",
  },
  {
    role: "frontend",
    label: "Frontend",
    agentId: "agent_frontend",
    agentName: "前端 Agent",
    model: "claude-ui",
    summaryFocus: "界面与交互",
    parentRole: "scheduler",
  },
  {
    role: "backend",
    label: "Backend",
    agentId: "agent_backend",
    agentName: "Backend Agent",
    model: "gpt-api",
    summaryFocus: "接口与数据",
    parentRole: "scheduler",
  },
  {
    role: "qa",
    label: "QA",
    agentId: "agent_qa",
    agentName: "QA Agent",
    model: "gpt-qa",
    summaryFocus: "测试与验收",
    parentRole: "scheduler",
  },
  {
    role: "docs",
    label: "Docs",
    agentId: "agent_docs",
    agentName: "文档 Agent",
    model: "gpt-docs",
    summaryFocus: "文档与交接",
    parentRole: "scheduler",
  },
  {
    role: "reviewer",
    label: "Reviewer",
    agentId: "agent_reviewer",
    agentName: "审查 Agent",
    model: "gemini-long-context",
    summaryFocus: "风险汇总",
    parentRole: "scheduler",
  },
];

const agentRunRoleOrder = agentRunRoleCatalog.map((item) => item.role);

function agentRunRoleInfo(role) {
  return agentRunRoleCatalog.find((item) => item.role === role) || null;
}

function sanitizeIdSegment(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function compactText(value) {
  return String(value || "")
    .replace(/\s+/g, " ")
    .trim();
}

function snippet(value, limit = 96) {
  const text = compactText(value);
  if (!text) return "";
  if (text.length <= limit) return text;
  return `${text.slice(0, Math.max(0, limit - 1)).trimEnd()}…`;
}

function tokenEstimateFromText(text) {
  const normalized = compactText(text);
  if (!normalized) {
    return 16;
  }
  return Math.max(24, Math.ceil(normalized.length / 3.4));
}

function buildTokenUsage(inputSummary, outputSummary) {
  const promptTokens = tokenEstimateFromText(inputSummary);
  const completionTokens = tokenEstimateFromText(outputSummary);
  return {
    promptTokens,
    completionTokens,
    totalTokens: promptTokens + completionTokens,
  };
}

function buildCostEstimate(totalTokens, model) {
  const rateByModel = {
    "gpt-high-reasoning": 0.00036,
    "local-scheduler": 0.00014,
    "claude-ui": 0.00024,
    "gpt-api": 0.00022,
    "gpt-qa": 0.0002,
    "gpt-docs": 0.00018,
    "gemini-long-context": 0.00026,
  };
  const amount = Number(((totalTokens || 0) * (rateByModel[model] || 0.0002)).toFixed(4));
  return {
    amount,
    currency: "CNY",
  };
}

function buildAgentRunInputSummary({ role, idea, constraints, parentOutputSummary }) {
  const roleInfo = agentRunRoleInfo(role);
  const lines = [];

  if (role === "architect") {
    lines.push(`需求：${snippet(idea, 120) || "未提供"}`);
    lines.push(`约束：${snippet(constraints, 120) || "无"}`);
    lines.push("目标：生成总体方案、风险边界和后续拆分方向。");
    return lines.join("\n");
  }

  if (role === "scheduler") {
    lines.push(`承接总体方案：${snippet(parentOutputSummary, 120) || "未提供"}`);
    lines.push("目标：拆分 frontend / backend / qa / docs / reviewer 的工作包。");
    lines.push(`关注点：${roleInfo?.summaryFocus || "任务拆分"}`);
    return lines.join("\n");
  }

  lines.push(`来源：${snippet(parentOutputSummary, 120) || "未提供"}`);
  lines.push(`关注点：${roleInfo?.summaryFocus || "当前角色建议"}`);
  lines.push(`需求：${snippet(idea, 96) || "未提供"}`);
  return lines.join("\n");
}

function buildAgentRunOutputSummary({ role, idea, constraints, parentOutputSummary, simulateFailureRole, upstreamFailed }) {
  const ideaSnippet = snippet(idea, 84) || "本次需求";
  const constraintSnippet = snippet(constraints, 84) || "无额外约束";

  if (upstreamFailed) {
    return `上游链路已失败，当前角色仅保留阻塞记录，不继续推进。`;
  }

  switch (role) {
    case "architect":
      return [
        `围绕「${ideaSnippet}」形成总体方案。`,
        "建议先保持 Mock / SQLite first，再拆分后续实现面。",
        `约束摘要：${constraintSnippet}`,
      ].join("\n");
    case "scheduler":
      return [
        "拆分顺序：frontend -> backend -> qa -> docs -> reviewer。",
        "每个角色只输出建议，不写文件，不执行命令。",
        "Reviewer 需要汇总风险后再进入审查视图。",
      ].join("\n");
    case "frontend":
      return [
        `前端重点是链路列表、详情面板、状态标签和失败节点可视化。`,
        `对应需求：${ideaSnippet}`,
      ].join("\n");
    case "backend":
      return [
        "后端重点是 agent_runs 表、链路创建 API、列表/详情读取和审计记录。",
        `约束摘要：${constraintSnippet}`,
      ].join("\n");
    case "qa":
      return [
        "QA 重点覆盖成功、失败、blocked 和空数据场景。",
        "同时确认不会触碰真实 Runner、文件写入或模型调用。",
      ].join("\n");
    case "docs":
      return [
        "文档重点是数据模型、API 契约、路线和验证脚本。",
        "交接内容需要把 stage 3 / stage 4 边界写清楚。",
      ].join("\n");
    case "reviewer":
      return [
        "风险汇总：链路记录本身可追踪，但仍必须保持只读执行面。",
        `审查结论：${parentOutputSummary ? snippet(parentOutputSummary, 120) : "等待上游建议"}`,
      ].join("\n");
    default:
      return `围绕「${ideaSnippet}」生成本地 Agent Run 记录。`;
  }
}

function normalizeAgentRunRequest(body = {}) {
  const idea = compactText(body.idea || body.request || body.prompt || "");
  const constraints = compactText(body.constraints || "");
  const requestedBy = compactText(body.requestedBy || body.createdBy || "local_user") || "local_user";
  const simulateFailureRole = compactText(body.simulateFailureRole || body.failAtRole || "");
  const chainLabel = compactText(body.chainLabel || body.title || "");
  const validationErrors = [];

  if (!idea) {
    validationErrors.push("idea is required.");
  }

  if (simulateFailureRole && !agentRunRoleInfo(simulateFailureRole)) {
    validationErrors.push("simulateFailureRole must use a supported Agent Run role.");
  }

  return {
    idea,
    constraints,
    requestedBy,
    simulateFailureRole,
    chainLabel,
    validationErrors,
  };
}

function createAgentRunChain({ projectId, body = {}, agents = [], chainId = "", createdAt = "" }) {
  const request = normalizeAgentRunRequest(body);
  if (request.validationErrors.length > 0) {
    return {
      ok: false,
      validationErrors: request.validationErrors,
      agentRuns: [],
      runtimeEvents: [],
      chain: null,
    };
  }

  const now = createdAt || new Date().toISOString();
  const safeChainId = chainId || `agent_run_chain_${sanitizeIdSegment(now).slice(0, 24)}_${Math.random().toString(16).slice(2, 8)}`;
  const agentById = new Map((agents || []).map((agent) => [agent.id, agent]));
  const createdRuns = [];
  const runtimeEvents = [];
  const agentRunRoleByRole = new Map(agentRunRoleCatalog.map((item) => [item.role, item]));
  let upstreamFailed = false;
  let failedRunId = "";

  for (let index = 0; index < agentRunRoleOrder.length; index += 1) {
    const role = agentRunRoleOrder[index];
    const roleInfo = agentRunRoleByRole.get(role);
    const parentRole = roleInfo?.parentRole || "";
    const parentRun = parentRole ? createdRuns.find((run) => run.role === parentRole) : null;
    const agent = roleInfo?.agentId ? agentById.get(roleInfo.agentId) : null;
    const runId = `agent_run_${safeChainId}_${String(index + 1).padStart(2, "0")}_${role}`;
    const inputSummary = buildAgentRunInputSummary({
      role,
      idea: request.idea,
      constraints: request.constraints,
      parentOutputSummary: parentRun?.outputSummary || "",
    });
    const runStatus = upstreamFailed
      ? "blocked"
      : request.simulateFailureRole === role
        ? "failed"
        : "succeeded";
    const outputSummary = buildAgentRunOutputSummary({
      role,
      idea: request.idea,
      constraints: request.constraints,
      parentOutputSummary: parentRun?.outputSummary || "",
      simulateFailureRole: request.simulateFailureRole,
      upstreamFailed,
    });
    const tokenUsage = buildTokenUsage(inputSummary, outputSummary);
    const costEstimate = buildCostEstimate(tokenUsage.totalTokens, roleInfo?.model || "local-scheduler");
    const errorCategory = runStatus === "failed"
      ? "simulated_failure"
      : runStatus === "blocked"
        ? "upstream_failure"
        : "";
    const errorMessage = runStatus === "failed"
      ? `Simulated failure at ${role}.`
      : runStatus === "blocked"
        ? `Blocked after ${failedRunId || "upstream"} failed.`
        : "";

    const run = {
      id: runId,
      projectId,
      chainId: safeChainId,
      rootRunId: createdRuns.length > 0 ? createdRuns[0].rootRunId : runId,
      parentRunId: parentRun?.id || "",
      sequence: index + 1,
      role,
      agentId: roleInfo?.agentId || "",
      agentName: agent?.name || roleInfo?.agentName || role,
      model: agent?.model || roleInfo?.model || "",
      status: runStatus,
      inputSummary,
      outputSummary,
      tokenUsage,
      costEstimate,
      errorCategory,
      errorMessage,
      requestedBy: request.requestedBy,
      chainLabel: request.chainLabel || "",
      createdAt: now,
      startedAt: runStatus === "blocked" ? "" : now,
      completedAt: runStatus === "succeeded" ? now : "",
      failedAt: runStatus === "failed" ? now : "",
      updatedAt: now,
    };

    if (runStatus === "failed") {
      upstreamFailed = true;
      failedRunId = runId;
    }
    if (runStatus === "blocked") {
      upstreamFailed = true;
    }

    createdRuns.push(run);
    runtimeEvents.push({
      id: `runtime_event_agent_run_${runId}_recorded_${sanitizeIdSegment(now)}_${Math.random().toString(16).slice(2, 8)}`,
      projectId,
      entityType: "agent_run",
      entityId: runId,
      eventType: runStatus,
      beforeState: null,
      afterState: {
        id: run.id,
        chainId: run.chainId,
        rootRunId: run.rootRunId,
        parentRunId: run.parentRunId,
        sequence: run.sequence,
        role: run.role,
        agentId: run.agentId,
        agentName: run.agentName,
        model: run.model,
        status: run.status,
        errorCategory: run.errorCategory,
        errorMessage: run.errorMessage,
        requestedBy: run.requestedBy,
        updatedAt: run.updatedAt,
      },
      actor: request.requestedBy,
      reason: run.errorCategory || "agent_run_recorded",
      createdAt: now,
    });
  }

  const chain = summarizeAgentRunChain(createdRuns, request);
  return {
    ok: true,
    validationErrors: [],
    agentRuns: createdRuns,
    runtimeEvents,
    chain,
  };
}

function summarizeAgentRunChain(agentRuns, request = {}) {
  const runs = Array.isArray(agentRuns) ? agentRuns : [];
  const chainId = runs[0]?.chainId || request.chainId || "";
  const rootRun = runs[0] || null;
  const failedRun = runs.find((run) => run.status === "failed") || null;
  const blockedRuns = runs.filter((run) => run.status === "blocked");
  const succeededRuns = runs.filter((run) => run.status === "succeeded");
  const lastRun = runs[runs.length - 1] || null;
  const status = failedRun
    ? "failed"
    : blockedRuns.length > 0
      ? "blocked"
      : runs.length > 0 && succeededRuns.length === runs.length
        ? "succeeded"
        : lastRun?.status || "queued";

  return {
    chainId,
    rootRunId: rootRun?.id || "",
    requestedBy: request.requestedBy || rootRun?.requestedBy || "",
    chainLabel: request.chainLabel || rootRun?.chainLabel || "",
    idea: request.idea || "",
    constraints: request.constraints || "",
    simulateFailureRole: request.simulateFailureRole || "",
    status,
    totalRuns: runs.length,
    succeededRuns: succeededRuns.length,
    blockedRuns: blockedRuns.length,
    failedRunId: failedRun?.id || "",
    failedRole: failedRun?.role || "",
    createdAt: rootRun?.createdAt || "",
    updatedAt: lastRun?.updatedAt || rootRun?.updatedAt || "",
    inputSummary: rootRun?.inputSummary || "",
    outputSummary: lastRun?.outputSummary || rootRun?.outputSummary || "",
    summary: runs.length > 0
      ? `${runs.map((run) => run.role).join(" → ")}`
      : "暂无 Agent Run 记录",
  };
}

function sortAgentRunsBySequence(agentRuns = []) {
  return [...agentRuns].sort((left, right) => {
    const sequenceDelta = (left.sequence || 0) - (right.sequence || 0);
    if (sequenceDelta !== 0) return sequenceDelta;
    const createdDelta = String(left.createdAt || "").localeCompare(String(right.createdAt || ""));
    if (createdDelta !== 0) return createdDelta;
    return String(left.id || "").localeCompare(String(right.id || ""));
  });
}

function buildAgentRunChainViews(agentRuns = []) {
  const runsByChain = new Map();
  sortAgentRunsBySequence(agentRuns).forEach((run) => {
    if (!runsByChain.has(run.chainId)) {
      runsByChain.set(run.chainId, []);
    }
    runsByChain.get(run.chainId).push(run);
  });

  return [...runsByChain.values()]
    .map((runs) => ({
      chain: summarizeAgentRunChain(runs, { chainId: runs[0]?.chainId || "" }),
      agentRuns: runs,
    }))
    .sort((left, right) => {
      const updatedDelta = String(right.chain.updatedAt || "").localeCompare(String(left.chain.updatedAt || ""));
      if (updatedDelta !== 0) return updatedDelta;
      return String(right.chain.createdAt || "").localeCompare(String(left.chain.createdAt || ""));
    });
}

function noAgentRunSideEffects() {
  return {
    writesSqlite: false,
    writesRuntimeState: false,
    writesRuntimeEvents: false,
    writesProjectFiles: false,
    modifiesGit: false,
    createsApprovals: false,
    createsTasks: false,
    createsRunnerJobs: false,
    triggersAgents: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
  };
}

module.exports = {
  agentRunRoleCatalog,
  agentRunRoleOrder,
  agentRunRoleInfo,
  buildAgentRunInputSummary,
  buildAgentRunOutputSummary,
  buildCostEstimate,
  buildTokenUsage,
  buildAgentRunChainViews,
  compactText,
  createAgentRunChain,
  normalizeAgentRunRequest,
  noAgentRunSideEffects,
  sortAgentRunsBySequence,
  sanitizeIdSegment,
  snippet,
  summarizeAgentRunChain,
};
