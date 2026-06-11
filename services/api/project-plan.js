const PROJECT_PLAN_AGENT_ASSIGNMENTS = [
  {
    role: "frontend",
    agentId: "agent_frontend",
    title: "Frontend interaction slice",
    description: "Turn the approved project plan into the first usable web UI flow and state wiring.",
    priority: "high",
    riskLevel: "medium",
    relatedFiles: ["virtual/frontend-plan.md"],
    operationTypes: ["runner_request_readonly", "frontend_plan"],
  },
  {
    role: "backend",
    agentId: "agent_backend",
    title: "Backend API and state slice",
    description: "Define local API endpoints, state transitions, and persistence boundaries for the approved plan.",
    priority: "high",
    riskLevel: "medium",
    relatedFiles: ["virtual/backend-plan.md"],
    operationTypes: ["runner_request_readonly", "backend_plan"],
  },
  {
    role: "qa",
    agentId: "agent_qa",
    title: "QA verification slice",
    description: "Draft acceptance checks, blocked paths, and no-side-effect verification for the approved plan.",
    priority: "medium",
    riskLevel: "low",
    relatedFiles: ["virtual/qa-plan.md"],
    operationTypes: ["runner_request_readonly", "qa_plan"],
  },
  {
    role: "docs",
    agentId: "agent_docs",
    title: "Docs and handoff slice",
    description: "Update user-facing and AI-facing documents for the approved project plan.",
    priority: "medium",
    riskLevel: "low",
    relatedFiles: ["virtual/docs-plan.md"],
    operationTypes: ["runner_request_readonly", "docs_plan"],
  },
  {
    role: "reviewer",
    agentId: "agent_reviewer",
    title: "Review and safety slice",
    description: "Review the generated tasks, Runner requests, and safety boundaries before any future execution.",
    priority: "high",
    riskLevel: "medium",
    relatedFiles: ["virtual/review-plan.md"],
    operationTypes: ["runner_request_readonly", "review_plan"],
  },
];

function noProjectPlanRequestSideEffects() {
  return {
    writesProjectFiles: false,
    modifiesGit: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
    makesNetworkRequests: false,
    triggersAgents: false,
  };
}

function normalizePlanId(value, nowMs = Date.now()) {
  const raw = String(value || "").trim().toLowerCase();
  const slug = raw
    .replace(/[^a-z0-9_-]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 64);
  return slug || `project_plan_${nowMs}`;
}

function taskIdForPlan(planId, role) {
  return `task_${planId}_${role}`;
}

function runnerJobIdForTask(taskId) {
  return `runner_job_${taskId}`;
}

function buildProjectPlanDraft({ projectId, body = {}, agents = [], now = new Date().toISOString() }) {
  const idea = String(body.idea || "").trim();
  const constraints = String(body.constraints || "").trim();
  if (!idea) {
    return {
      error: "project_idea_required",
      message: "Project plan request requires a non-empty idea.",
    };
  }

  const planId = normalizePlanId(body.planId);
  const agentById = new Map((agents || []).map((agent) => [agent.id, agent]));
  const summary = `MVP plan draft for: ${idea}`;
  const safety = {
    ...noProjectPlanRequestSideEffects(),
    writesRuntimeState: false,
    writesSqlite: false,
    createsApproval: false,
    createsTasks: false,
    createsRunnerJobs: false,
  };

  const tasks = PROJECT_PLAN_AGENT_ASSIGNMENTS.map((item, index) => {
    const taskId = taskIdForPlan(planId, item.role);
    const agent = agentById.get(item.agentId);
    return {
      id: taskId,
      title: `${item.title}: ${idea}`,
      description: `${item.description}${constraints ? ` Constraints: ${constraints}` : ""}`,
      status: "queued",
      priority: item.priority,
      assignedAgentId: item.agentId,
      assignedAgentName: agent?.name || item.agentId,
      role: item.role,
      riskLevel: item.riskLevel,
      relatedFiles: item.relatedFiles,
      requiresApproval: false,
      dependsOn: index === 0 ? [] : [taskIdForPlan(planId, PROJECT_PLAN_AGENT_ASSIGNMENTS[0].role)],
      operationTypes: item.operationTypes,
      runnerJobId: runnerJobIdForTask(taskId),
    };
  });

  const runnerRequests = tasks.map((task) => ({
    id: task.runnerJobId,
    taskId: task.id,
    approvalId: `approval_project_plan_${planId}`,
    status: "queued",
    operationTypes: task.operationTypes,
    affectedFiles: task.relatedFiles,
    checkpoint: "",
    safetyNote: "Read-only MVP-0.3 Runner request queue item. No command, file write, network request, or Git change is executed.",
  }));

  return {
    id: planId,
    projectId,
    idea,
    constraints,
    summary,
    status: "draft",
    generatedBy: "local_deterministic_template",
    requestedBy: body.requestedBy || "local_user",
    createdAt: now,
    tasks,
    runnerRequests,
    sideEffects: safety,
  };
}

function buildProjectPlanApproval({ projectId, plan, existingApproval = null, now = new Date().toISOString() }) {
  const approvalId = `approval_project_plan_${plan.id}`;
  return {
    id: approvalId,
    status: "pending",
    riskLevel: "medium",
    riskTone: "mid",
    requestAgentId: "agent_architect",
    requestAgentName: "架构师 Agent",
    operationTypes: ["project_plan_approval", "agent_task_assignment", "runner_request_queue"],
    reason: `Review MVP-0.3 project plan draft for: ${plan.idea}`,
    checkpoint: {
      required: false,
      created: false,
      commit: "",
    },
    affectedFiles: [`project-plan/${plan.id}`, ...plan.tasks.map((task) => `task/${task.id}`)],
    diffSummary: `${plan.tasks.length} tasks / ${plan.runnerRequests.length} read-only runner requests`,
    diffPreview: [
      `+ Project idea: ${plan.idea}`,
      `+ Plan draft source: ${plan.generatedBy}`,
      `+ Task assignments: ${plan.tasks.map((task) => `${task.role}:${task.assignedAgentId}`).join(", ")}`,
      "+ Runner queue: read-only request records only; no execution.",
      "+ Safety: no real model call, no local file write, no command execution, no Git change.",
    ],
    requiresSecondConfirm: true,
    targetService: "project_plan",
    createdAt: existingApproval?.createdAt || now,
    updatedAt: now,
    runnerJobId: "",
    changeRequest: {
      type: "project_plan",
      changeType: "project_plan",
      projectId,
      plan,
      sideEffects: {
        ...noProjectPlanRequestSideEffects(),
        writesRuntimeState: false,
        writesSqlite: false,
        createsApproval: false,
        createsTasks: false,
        createsRunnerJobs: false,
      },
    },
  };
}

function buildProjectPlanApprovalFromRequest({ projectId, body = {}, agents = [], existingApproval = null, now = new Date().toISOString() }) {
  const plan = buildProjectPlanDraft({ projectId, body, agents, now });
  if (plan.error) return plan;
  const approval = buildProjectPlanApproval({ projectId, plan, existingApproval, now });
  return { plan, approval };
}

function isProjectPlanApproval(approval = {}) {
  const changeRequest = approval.changeRequest || {};
  return approval.targetService === "project_plan"
    || changeRequest.type === "project_plan"
    || changeRequest.changeType === "project_plan";
}

function instantiateProjectPlanApproval({ approval, tasks = [], runnerJobs = [], now = new Date().toISOString() }) {
  const plan = approval?.changeRequest?.plan;
  if (!plan || !Array.isArray(plan.tasks) || !Array.isArray(plan.runnerRequests)
    || plan.tasks.length === 0 || plan.runnerRequests.length === 0) {
    return {
      error: "invalid_project_plan_approval",
      message: "Project plan approval must contain plan tasks and runnerRequests.",
    };
  }

  const taskIds = new Set();
  for (const plannedTask of plan.tasks) {
    if (!plannedTask || typeof plannedTask.id !== "string" || !plannedTask.id.trim()) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan approval contains a task without an id.",
      };
    }
    if (taskIds.has(plannedTask.id)) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan approval contains duplicate task ids.",
      };
    }
    taskIds.add(plannedTask.id);
  }

  const runnerJobIds = new Set();
  for (const runnerRequest of plan.runnerRequests) {
    if (!runnerRequest || typeof runnerRequest.id !== "string" || !runnerRequest.id.trim()) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan approval contains a Runner request without an id.",
      };
    }
    if (runnerJobIds.has(runnerRequest.id)) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan approval contains duplicate Runner request ids.",
      };
    }
    if (!taskIds.has(runnerRequest.taskId)) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan Runner request must reference a planned task.",
      };
    }
    if (!Array.isArray(runnerRequest.operationTypes)
      || !runnerRequest.operationTypes.includes("runner_request_readonly")) {
      return {
        error: "invalid_project_plan_approval",
        message: "Project plan Runner request must remain read-only.",
      };
    }
    runnerJobIds.add(runnerRequest.id);
  }

  const createdTasks = [];
  const createdRunnerJobs = [];

  plan.tasks.forEach((plannedTask) => {
    let task = tasks.find((item) => item.id === plannedTask.id);
    if (!task) {
      task = {
        id: plannedTask.id,
        createdAt: now,
      };
      tasks.push(task);
      createdTasks.push(task.id);
    }

    task.title = plannedTask.title;
    task.description = plannedTask.description;
    task.status = task.status || "queued";
    task.priority = plannedTask.priority || "medium";
    task.assignedAgentId = plannedTask.assignedAgentId;
    task.riskLevel = plannedTask.riskLevel || "low";
    task.relatedFiles = plannedTask.relatedFiles || [];
    task.requiresApproval = plannedTask.requiresApproval === true;
    task.dependsOn = plannedTask.dependsOn || [];
    task.updatedAt = now;
  });

  plan.runnerRequests.forEach((runnerRequest) => {
    let job = runnerJobs.find((item) => item.id === runnerRequest.id);
    if (!job) {
      job = {
        id: runnerRequest.id,
        createdAt: now,
      };
      runnerJobs.push(job);
      createdRunnerJobs.push(job.id);
    }

    job.approvalId = approval.id;
    job.taskId = runnerRequest.taskId || "";
    job.status = job.status || "queued";
    job.operationTypes = runnerRequest.operationTypes || ["runner_request_readonly"];
    job.affectedFiles = runnerRequest.affectedFiles || [];
    job.checkpoint = "";
    job.safetyNote = runnerRequest.safetyNote
      || "Read-only MVP-0.3 Runner request queue item. No execution.";
    job.updatedAt = now;
  });

  return {
    planId: plan.id,
    createdTaskIds: createdTasks,
    createdRunnerJobIds: createdRunnerJobs,
    taskIds: plan.tasks.map((task) => task.id),
    runnerJobIds: plan.runnerRequests.map((job) => job.id),
  };
}

module.exports = {
  PROJECT_PLAN_AGENT_ASSIGNMENTS,
  buildProjectPlanApprovalFromRequest,
  buildProjectPlanDraft,
  buildProjectPlanApproval,
  instantiateProjectPlanApproval,
  isProjectPlanApproval,
  noProjectPlanRequestSideEffects,
};
