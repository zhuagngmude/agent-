const http = require("http");
const fs = require("fs");
const path = require("path");
const { URL } = require("url");
const data = require("./mock-data");

const port = Number(process.env.AGENT_SWARM_API_PORT || 8787);
const runtimeStateFile = path.resolve(__dirname, "..", "..", "data", "mock", "runtime-state.json");

function sendJson(res, statusCode, body) {
  const payload = JSON.stringify(body, null, 2);
  res.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET,POST,PATCH,OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
  });
  res.end(payload);
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = "";
    req.on("data", (chunk) => {
      raw += chunk;
      if (raw.length > 1_000_000) {
        reject(new Error("Request body too large"));
        req.destroy();
      }
    });
    req.on("end", () => {
      if (!raw) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(raw));
      } catch (error) {
        reject(error);
      }
    });
  });
}

function withProject(pathname, suffix) {
  const prefix = `/api/projects/${data.projectId}`;
  return pathname === `${prefix}${suffix}`;
}

function findApproval(id) {
  return data.approvals.find((item) => item.id === id);
}

function findTask(id) {
  return data.tasks.find((item) => item.id === id);
}

function findAgent(id) {
  return data.agents.find((item) => item.id === id);
}

function findRunnerJob(id) {
  return data.runnerJobs.find((item) => item.id === id);
}

function serializeRuntimeState() {
  return {
    version: 1,
    updatedAt: new Date().toISOString(),
    approvals: data.approvals.map((approval) => ({
      id: approval.id,
      status: approval.status,
      riskLevel: approval.riskLevel || "",
      riskTone: approval.riskTone || "",
      requestAgentId: approval.requestAgentId || "",
      requestAgentName: approval.requestAgentName || "",
      operationTypes: approval.operationTypes || [],
      reason: approval.reason || "",
      checkpoint: approval.checkpoint || {},
      affectedFiles: approval.affectedFiles || [],
      diffSummary: approval.diffSummary || "",
      diffPreview: approval.diffPreview || [],
      requiresSecondConfirm: approval.requiresSecondConfirm === true,
      targetService: approval.targetService || "",
      changeRequest: approval.changeRequest || null,
      rejectReason: approval.rejectReason || "",
      runnerJobId: approval.runnerJobId || "",
      patchArtifactId: approval.patchArtifactId || "",
      approvedAt: approval.approvedAt || "",
      rejectedAt: approval.rejectedAt || "",
      patchOnlyAt: approval.patchOnlyAt || "",
      updatedAt: approval.updatedAt || "",
    })),
    tasks: data.tasks.map((task) => ({
      id: task.id,
      status: task.status,
      startedAt: task.startedAt || "",
      completedAt: task.completedAt || "",
      failedAt: task.failedAt || "",
      cancelledAt: task.cancelledAt || "",
      failureReason: task.failureReason || "",
      updatedAt: task.updatedAt || "",
    })),
    runnerJobs: data.runnerJobs.map((job) => ({ ...job })),
  };
}

function applyRuntimeState(state) {
  if (!state) return;

  if (Array.isArray(state.approvals)) {
    state.approvals.forEach((storedApproval) => {
      let approval = findApproval(storedApproval.id);
      if (!approval) {
        approval = { id: storedApproval.id };
        data.approvals.push(approval);
      }

      [
        "status",
        "riskLevel",
        "riskTone",
        "requestAgentId",
        "requestAgentName",
        "operationTypes",
        "reason",
        "checkpoint",
        "affectedFiles",
        "diffSummary",
        "diffPreview",
        "requiresSecondConfirm",
        "targetService",
        "changeRequest",
        "rejectReason",
        "runnerJobId",
        "patchArtifactId",
        "approvedAt",
        "rejectedAt",
        "patchOnlyAt",
        "updatedAt",
        "createdAt",
      ].forEach((key) => {
        if (storedApproval[key] !== undefined) {
          approval[key] = storedApproval[key];
        }
      });
    });
  }

  if (Array.isArray(state.tasks)) {
    state.tasks.forEach((storedTask) => {
      const task = findTask(storedTask.id);
      if (!task) return;

      [
        "status",
        "startedAt",
        "completedAt",
        "failedAt",
        "cancelledAt",
        "failureReason",
        "updatedAt",
      ].forEach((key) => {
        if (storedTask[key] !== undefined) {
          task[key] = storedTask[key];
        }
      });
    });
  }

  if (Array.isArray(state.runnerJobs)) {
    data.runnerJobs.splice(0, data.runnerJobs.length, ...state.runnerJobs.map((job) => ({ ...job })));
  }
}

function saveRuntimeState() {
  fs.mkdirSync(path.dirname(runtimeStateFile), { recursive: true });
  const tmpFile = `${runtimeStateFile}.tmp`;
  fs.writeFileSync(tmpFile, `${JSON.stringify(serializeRuntimeState(), null, 2)}\n`, "utf8");
  fs.renameSync(tmpFile, runtimeStateFile);
}

function loadRuntimeState() {
  if (!fs.existsSync(runtimeStateFile)) {
    saveRuntimeState();
    return;
  }

  const state = JSON.parse(fs.readFileSync(runtimeStateFile, "utf8"));
  applyRuntimeState(state);
}

function resetRuntimeState() {
  data.resetRuntimeData();
  saveRuntimeState();
}

function clearRuntimeState() {
  data.resetRuntimeData();
  if (fs.existsSync(runtimeStateFile)) {
    fs.rmSync(runtimeStateFile, { force: true });
  }
}

function upsertRunnerJobFromApproval(approval) {
  const runnerJobId = `runner_job_${approval.id}`;
  const existing = findRunnerJob(runnerJobId);
  const job = existing || {
    id: runnerJobId,
    approvalId: approval.id,
    taskId: "",
    status: "queued",
    operationTypes: approval.operationTypes || [],
    affectedFiles: approval.affectedFiles || [],
    checkpoint: approval.checkpoint?.commit || "",
    createdAt: new Date().toISOString(),
  };

  job.status = existing?.status || "queued";
  job.approvalId = approval.id;
  job.operationTypes = approval.operationTypes || [];
  job.affectedFiles = approval.affectedFiles || [];
  job.checkpoint = approval.checkpoint?.commit || "";
  job.updatedAt = new Date().toISOString();

  if (!existing) {
    data.runnerJobs.push(job);
  }

  return job;
}

function riskTone(riskLevel) {
  if (riskLevel === "high") return "high";
  if (riskLevel === "medium") return "mid";
  return "low";
}

function createAgentChangeApproval(agent, body) {
  const now = new Date().toISOString();
  const changeType = body.changeType || "model";
  const riskLevel = body.riskLevel || (changeType === "model" ? "medium" : "high");
  const changes = Array.isArray(body.changes) ? body.changes : [];
  const approvalId = `approval_agent_${agent.id}_${changeType}`;
  const existing = findApproval(approvalId);
  const diffPreview = changes.length
    ? changes.map((change) => `~ ${change.field}: ${change.before} -> ${change.after}`)
    : [`~ ${changeType}: 等待补充变更字段`];

  const approval = existing || {
    id: approvalId,
    requestAgentId: agent.id,
    requestAgentName: agent.name,
    operationTypes: ["agent_config_change"],
    affectedFiles: [`agent-config/${agent.id}`],
  };

  approval.status = "pending";
  approval.riskLevel = riskLevel;
  approval.riskTone = riskTone(riskLevel);
  approval.reason = body.reason || `申请修改 ${agent.name} 的 Agent 配置。`;
  approval.checkpoint = {
    required: true,
    created: false,
    commit: "",
  };
  approval.diffSummary = `${changes.length} fields`;
  approval.diffPreview = diffPreview;
  approval.requiresSecondConfirm = riskLevel === "high";
  approval.targetService = "agent_config";
  approval.createdAt = existing?.createdAt || now;
  approval.updatedAt = now;
  approval.changeRequest = {
    agentId: agent.id,
    changeType,
    changes,
  };

  if (!existing) {
    data.approvals.push(approval);
  }

  return approval;
}

async function handleApprovalAction(req, res, approvalId, action) {
  const approval = findApproval(approvalId);
  if (!approval) {
    sendJson(res, 404, { error: "approval_not_found" });
    return;
  }

  const body = await readBody(req);

  if (action === "approve") {
    if (approval.requiresSecondConfirm && body.secondConfirm !== true) {
      sendJson(res, 409, {
        error: "second_confirm_required",
        message: "High risk approval requires secondConfirm=true.",
      });
      return;
    }
    approval.status = "approved";
    if (approval.targetService === "agent_config") {
      approval.runnerJobId = "";
    } else {
      const runnerJob = upsertRunnerJobFromApproval(approval);
      approval.runnerJobId = runnerJob.id;
    }
    approval.approvedAt = new Date().toISOString();
    approval.updatedAt = approval.approvedAt;
    saveRuntimeState();
    sendJson(res, 200, {
      id: approval.id,
      status: approval.status,
      runnerJobId: approval.runnerJobId,
    });
    return;
  }

  if (action === "reject") {
    approval.status = "rejected";
    approval.rejectReason = body.reason || "";
    approval.rejectedAt = new Date().toISOString();
    approval.updatedAt = approval.rejectedAt;
    saveRuntimeState();
    sendJson(res, 200, { id: approval.id, status: approval.status });
    return;
  }

  if (action === "patch-only") {
    approval.status = "patch_only";
    approval.patchArtifactId = `patch_${approval.id}`;
    approval.patchOnlyAt = new Date().toISOString();
    approval.updatedAt = approval.patchOnlyAt;
    saveRuntimeState();
    sendJson(res, 200, {
      id: approval.id,
      status: approval.status,
      patchArtifactId: approval.patchArtifactId,
    });
    return;
  }

  sendJson(res, 404, { error: "unknown_approval_action" });
}

async function handleAgentChangeRequest(req, res, agentId) {
  const agent = findAgent(agentId);
  if (!agent) {
    sendJson(res, 404, { error: "agent_not_found" });
    return;
  }

  const body = await readBody(req);
  const approval = createAgentChangeApproval(agent, body);
  saveRuntimeState();
  sendJson(res, 201, {
    approval,
    message: "Agent change request created. Agent config was not modified.",
  });
}

function transitionTask(task, action, body) {
  const now = new Date().toISOString();
  const terminalStatuses = ["completed", "failed", "cancelled"];

  if (action === "start") {
    if (!["queued", "blocked", "waiting_user", "failed", "cancelled"].includes(task.status)) {
      return { error: "task_cannot_start", message: `Task cannot start from status ${task.status}.` };
    }
    task.status = "running";
    task.startedAt = now;
    delete task.completedAt;
    delete task.failedAt;
    delete task.cancelledAt;
    delete task.failureReason;
  } else if (action === "complete") {
    if (task.status !== "running") {
      return { error: "task_cannot_complete", message: "Only running tasks can be completed." };
    }
    task.status = "completed";
    task.completedAt = now;
  } else if (action === "fail") {
    if (terminalStatuses.includes(task.status)) {
      return { error: "task_already_terminal", message: `Task is already ${task.status}.` };
    }
    task.status = "failed";
    task.failedAt = now;
    task.failureReason = body.reason || "用户在控制台标记为失败";
  } else if (action === "cancel") {
    if (terminalStatuses.includes(task.status)) {
      return { error: "task_already_terminal", message: `Task is already ${task.status}.` };
    }
    task.status = "cancelled";
    task.cancelledAt = now;
  } else {
    return { error: "unknown_task_action", message: "Unknown task action." };
  }

  task.updatedAt = now;
  return null;
}

async function handleTaskAction(req, res, taskId, action) {
  const task = findTask(taskId);
  if (!task) {
    sendJson(res, 404, { error: "task_not_found" });
    return;
  }

  const body = await readBody(req);
  const transitionError = transitionTask(task, action, body);
  if (transitionError) {
    sendJson(res, 409, transitionError);
    return;
  }

  saveRuntimeState();
  sendJson(res, 200, { task });
}

async function handleRequest(req, res) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  const { pathname } = url;

  if (req.method === "OPTIONS") {
    sendJson(res, 204, {});
    return;
  }

  if (req.method === "GET" && pathname === "/api/health") {
    sendJson(res, 200, { ok: true, service: "agent-swarm-api", projectId: data.projectId });
    return;
  }

  if (req.method === "GET" && pathname === "/api/runtime-state") {
    sendJson(res, 200, {
      stateFile: runtimeStateFile,
      exists: fs.existsSync(runtimeStateFile),
      state: serializeRuntimeState(),
    });
    return;
  }

  if (req.method === "POST" && pathname === "/api/runtime-state/reset") {
    resetRuntimeState();
    sendJson(res, 200, {
      ok: true,
      stateFile: runtimeStateFile,
      state: serializeRuntimeState(),
    });
    return;
  }

  if (req.method === "DELETE" && pathname === "/api/runtime-state") {
    clearRuntimeState();
    sendJson(res, 200, {
      ok: true,
      stateFile: runtimeStateFile,
      exists: fs.existsSync(runtimeStateFile),
      message: "Runtime state cleared. Restarting the API will recreate the file from mock defaults.",
    });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/dashboard")) {
    sendJson(res, 200, data.dashboard());
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/agents")) {
    sendJson(res, 200, { agents: data.agents });
    return;
  }

  const agentChangeRequest = pathname.match(/^\/api\/agents\/([^/]+)\/change-requests$/);
  if (req.method === "POST" && agentChangeRequest) {
    await handleAgentChangeRequest(req, res, agentChangeRequest[1]);
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/tasks")) {
    sendJson(res, 200, { tasks: data.tasks });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/workflows")) {
    sendJson(res, 200, { workflows: data.workflows });
    return;
  }

  const taskDetail = pathname.match(/^\/api\/tasks\/([^/]+)$/);
  if (req.method === "GET" && taskDetail) {
    const task = findTask(taskDetail[1]);
    sendJson(res, task ? 200 : 404, task || { error: "task_not_found" });
    return;
  }

  const taskAction = pathname.match(/^\/api\/tasks\/([^/]+)\/(start|complete|fail|cancel)$/);
  if (req.method === "POST" && taskAction) {
    await handleTaskAction(req, res, taskAction[1], taskAction[2]);
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/approvals")) {
    const status = url.searchParams.get("status");
    const riskLevel = url.searchParams.get("riskLevel");
    const approvals = data.approvals.filter((item) => {
      if (status && item.status !== status) return false;
      if (riskLevel && item.riskLevel !== riskLevel) return false;
      return true;
    });
    sendJson(res, 200, { approvals });
    return;
  }

  const approvalDetail = pathname.match(/^\/api\/approvals\/([^/]+)$/);
  if (req.method === "GET" && approvalDetail) {
    const approval = findApproval(approvalDetail[1]);
    sendJson(res, approval ? 200 : 404, approval || { error: "approval_not_found" });
    return;
  }

  const approvalAction = pathname.match(/^\/api\/approvals\/([^/]+)\/(approve|reject|patch-only)$/);
  if (req.method === "POST" && approvalAction) {
    await handleApprovalAction(req, res, approvalAction[1], approvalAction[2]);
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/runner/status")) {
    sendJson(res, 200, {
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
      lastHeartbeatAt: new Date().toISOString(),
    });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/runner/jobs")) {
    sendJson(res, 200, { jobs: data.runnerJobs });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/git/checkpoints")) {
    sendJson(res, 200, { checkpoints: data.gitCheckpoints });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/knowledge/updates")) {
    sendJson(res, 200, { updates: data.knowledgeUpdates });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/usage")) {
    sendJson(res, 200, data.usage);
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/settings")) {
    sendJson(res, 200, data.settings);
    return;
  }

  sendJson(res, 404, { error: "not_found", path: pathname });
}

loadRuntimeState();

const server = http.createServer((req, res) => {
  handleRequest(req, res).catch((error) => {
    sendJson(res, 500, { error: "internal_error", message: error.message });
  });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`agent蜂群 mock API listening on http://127.0.0.1:${port}`);
});
