const http = require("http");
const { URL } = require("url");
const data = require("./mock-data");

const port = Number(process.env.AGENT_SWARM_API_PORT || 8787);

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
    sendJson(res, 200, {
      id: approval.id,
      status: approval.status,
      runnerJobId: `runner_job_${approval.id}`,
    });
    return;
  }

  if (action === "reject") {
    approval.status = "rejected";
    approval.rejectReason = body.reason || "";
    sendJson(res, 200, { id: approval.id, status: approval.status });
    return;
  }

  if (action === "patch-only") {
    approval.status = "patch_only";
    sendJson(res, 200, {
      id: approval.id,
      status: approval.status,
      patchArtifactId: `patch_${approval.id}`,
    });
    return;
  }

  sendJson(res, 404, { error: "unknown_approval_action" });
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

const server = http.createServer((req, res) => {
  handleRequest(req, res).catch((error) => {
    sendJson(res, 500, { error: "internal_error", message: error.message });
  });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`agent蜂群 mock API listening on http://127.0.0.1:${port}`);
});
