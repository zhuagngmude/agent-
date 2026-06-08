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

function serializeRuntimeState() {
  return {
    version: 1,
    updatedAt: new Date().toISOString(),
    approvals: data.approvals.map((approval) => ({
      id: approval.id,
      status: approval.status,
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
  };
}

function applyRuntimeState(state) {
  if (!state) return;

  if (Array.isArray(state.approvals)) {
    state.approvals.forEach((storedApproval) => {
      const approval = findApproval(storedApproval.id);
      if (!approval) return;

      [
        "status",
        "rejectReason",
        "runnerJobId",
        "patchArtifactId",
        "approvedAt",
        "rejectedAt",
        "patchOnlyAt",
        "updatedAt",
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
    approval.runnerJobId = `runner_job_${approval.id}`;
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
      workspacePath: "F:/ai共同体(知识库)/20_项目/agent蜂群",
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
