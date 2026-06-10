const http = require("http");
const fs = require("fs");
const path = require("path");
const { URL } = require("url");
const data = require("./mock-data");
const { defaultDbFile, readDashboardFromSqlite, readProjectSnapshotFromSqlite } = require("./db/sqlite-read");
const { resetSqliteState, runSqliteWrite } = require("./db/sqlite-write");
const {
  modelGatewayConnectivityTest,
  modelGatewayDryRun,
  modelGatewayStatus,
} = require("./model-gateway");
const {
  agentPermissionProfiles,
  validateAgentCapabilities,
} = require("./agent-permissions");

const port = Number(process.env.AGENT_SWARM_API_PORT || 8787);
const runtimeStateFile = path.resolve(__dirname, "..", "..", "data", "mock", "runtime-state.json");
const dashboardSource = process.env.AGENT_SWARM_DASHBOARD_SOURCE || "mock";
const projectRoot = path.resolve(__dirname, "..", "..");
const sqliteDbFile = process.env.AGENT_SWARM_SQLITE_DB || defaultDbFile;

function localTrialInfo() {
  return {
    mode: sqliteReadEnabled() ? "sqlite" : "mock",
    persistence: sqliteReadEnabled() ? "sqlite" : "runtime_state_file",
    apiUrl: `http://127.0.0.1:${port}`,
    webUrl: "http://127.0.0.1:5175/index.html",
    projectRoot,
    sqliteDbFile,
    runtimeStateFile,
    commands: {
      start: "powershell -ExecutionPolicy Bypass -File scripts\\start-local.ps1",
      status: "powershell -ExecutionPolicy Bypass -File scripts\\status-local.ps1",
      stop: "powershell -ExecutionPolicy Bypass -File scripts\\stop-local.ps1",
      reset: "Invoke-RestMethod -Method Post http://127.0.0.1:8787/api/runtime-state/reset",
    },
    safety: {
      runnerExecutesCommands: false,
      runnerWritesFiles: false,
      realModelCalls: false,
      cloudSync: false,
    },
  };
}

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

function dashboardResponse() {
  if (dashboardSource !== "sqlite") {
    return data.dashboard();
  }

  try {
    return readDashboardFromSqlite(data.projectId);
  } catch (error) {
    console.warn(`[sqlite-dashboard] ${error.message}. Falling back to mock dashboard.`);
    return data.dashboard();
  }
}

function sqliteReadEnabled() {
  return dashboardSource === "sqlite";
}

function projectSnapshotResponse(fallback) {
  if (!sqliteReadEnabled()) {
    return fallback();
  }

  try {
    return readProjectSnapshotFromSqlite(data.projectId);
  } catch (error) {
    console.warn(`[sqlite-read] ${error.message}. Falling back to mock response.`);
    return null;
  }
}

function sendSqliteWriteResult(res, result) {
  sendJson(res, result.statusCode || 200, result.body || {});
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

function findAgentConfigApplication(id) {
  return data.agentConfigApplications.find((item) => item.id === id);
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
    agentConfigApplications: data.agentConfigApplications.map((item) => ({ ...item })),
  };
}

function applyRuntimeState(state) {
  if (!state) return;

  if (Array.isArray(state.approvals)) {
    state.approvals.forEach((storedApproval) => {
      let approval = findApproval(storedApproval.id);
      const isExistingApproval = Boolean(approval);
      if (!approval) {
        approval = { id: storedApproval.id };
        data.approvals.push(approval);
      }

      const mutableApprovalKeys = [
        "status",
        "rejectReason",
        "runnerJobId",
        "patchArtifactId",
        "approvedAt",
        "rejectedAt",
        "patchOnlyAt",
        "updatedAt",
      ];
      const dynamicApprovalKeys = [
        ...mutableApprovalKeys,
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
        "createdAt",
      ];

      (isExistingApproval ? mutableApprovalKeys : dynamicApprovalKeys).forEach((key) => {
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

  if (Array.isArray(state.agentConfigApplications)) {
    data.agentConfigApplications.splice(
      0,
      data.agentConfigApplications.length,
      ...state.agentConfigApplications.map((item) => ({ ...item }))
    );
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

function upsertAgentConfigApplicationFromApproval(approval) {
  const changeRequest = approval.changeRequest || {};
  const agentId = changeRequest.agentId || approval.requestAgentId || "";
  const applicationId = `agent_config_application_${approval.id}`;
  const existing = findAgentConfigApplication(applicationId);
  const now = new Date().toISOString();
  const application = existing || {
    id: applicationId,
    approvalId: approval.id,
    createdAt: now,
  };

  application.agentId = agentId;
  application.agentName = findAgent(agentId)?.name || approval.requestAgentName || agentId;
  application.changeType = changeRequest.changeType || "";
  application.changes = Array.isArray(changeRequest.changes) ? changeRequest.changes : [];
  application.status = existing?.status || "pending_apply";
  application.updatedAt = now;

  if (!existing) {
    data.agentConfigApplications.push(application);
  }

  return application;
}

function riskTone(riskLevel) {
  if (riskLevel === "high") return "high";
  if (riskLevel === "medium") return "mid";
  return "low";
}

function noPermissionValidationSideEffects() {
  return {
    writesSqlite: false,
    writesRuntimeState: false,
    createsTasks: false,
    createsApprovals: false,
    createsRunnerJobs: false,
    triggersAgents: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
  };
}

function invalidPermissionValidation(message, input = {}) {
  return {
    ok: false,
    profile: input.profile || "",
    allFlagRequested: input.all === true,
    capabilities: Array.isArray(input.capabilities) ? input.capabilities : [],
    unknownCapabilities: [],
    forbiddenCapabilities: [],
    validationErrors: [message],
    sideEffects: noPermissionValidationSideEffects(),
  };
}

function splitPermissionText(value) {
  if (Array.isArray(value)) {
    return value.flatMap((item) => splitPermissionText(item));
  }

  if (typeof value !== "string") {
    return [];
  }

  return value
    .split(/[,\n/]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function permissionValidationInputFromChangeRequest(body = {}) {
  const input = {};
  if (body.all === true) {
    input.all = true;
  }

  const profile = body.permissionProfile || body.profile;
  if (typeof profile === "string" && profile.trim()) {
    input.profile = profile.trim();
  }

  if (Array.isArray(body.capabilities) && body.capabilities.length > 0) {
    input.capabilities = body.capabilities;
  }

  if (!input.profile && !input.capabilities) {
    const permissionChange = Array.isArray(body.changes)
      ? body.changes.find((change) => change && change.field === "permissions")
      : null;
    const afterValue = permissionChange?.after;
    const afterText = typeof afterValue === "string" ? afterValue.trim() : "";

    if (afterText && agentPermissionProfiles[afterText]) {
      input.profile = afterText;
    } else {
      const capabilities = splitPermissionText(afterValue);
      if (capabilities.length > 0) {
        input.capabilities = capabilities;
      }
    }
  }

  return input;
}

function validatePermissionChangeRequest(body = {}) {
  if (body.changeType !== "permission") {
    return null;
  }

  const input = permissionValidationInputFromChangeRequest(body);
  if (!input.profile && !input.capabilities && input.all !== true) {
    return invalidPermissionValidation("permission change must include a profile or explicit capabilities.", input);
  }

  return validateAgentCapabilities(input);
}

function noAgentConfigDryRunSideEffects() {
  return {
    writesAgents: false,
    writesAgentConfigVersions: false,
    writesRuntimeEvents: false,
    writesSqlite: false,
    writesRuntimeState: false,
    createsApprovals: false,
    createsRunnerJobs: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
  };
}

function noAgentConfigApplyGateSideEffects() {
  return {
    writesAgents: false,
    writesAgentConfigVersions: false,
    writesRuntimeEvents: false,
    writesSqlite: false,
    writesRuntimeState: false,
    createsApprovals: false,
    createsRunnerJobs: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
  };
}

function allAgentConfigSideEffectsFalse(sideEffects = {}) {
  return [
    "writesAgents",
    "writesAgentConfigVersions",
    "writesRuntimeEvents",
    "writesSqlite",
    "writesRuntimeState",
    "createsApprovals",
    "createsRunnerJobs",
    "executesRunner",
    "callsRealModel",
    "readsRawSecrets",
  ].every((key) => sideEffects[key] === false);
}

function buildAgentConfigRealApplyGate({ application, approval, agent, dryRun, body = {} }) {
  const blockedReasons = ["feature_disabled"];
  const validationErrors = [];
  const requiredChecks = {
    applicationExists: Boolean(application),
    applicationPending: application?.status === "pending_apply",
    sourceApprovalExists: Boolean(approval),
    sourceApprovalApproved: approval?.status === "approved",
    sourceApprovalTargetsAgentConfig: approval?.targetService === "agent_config",
    sourceApprovalHasNoRunnerJob: Boolean(approval) && !approval.runnerJobId,
    targetAgentExists: Boolean(agent),
    secondConfirm: body.secondConfirm === true,
    confirmText: Boolean(body.confirmText),
    requestedBy: Boolean(body.requestedBy),
    gitCheckpointPresent: body.gitCheckpoint?.created === true && Boolean(body.gitCheckpoint?.commit),
    rollbackPlanAccepted: body.rollbackPlanAccepted === true,
    dryRunResultProvided: Boolean(dryRun),
    dryRunMatchesApplication: dryRun?.applicationId === application?.id,
    dryRunMatchesApproval: dryRun?.approvalId === application?.approvalId,
    dryRunMatchesAgent: dryRun?.agentId === application?.agentId,
    dryRunValidationPassed: Array.isArray(dryRun?.validationErrors) && dryRun.validationErrors.length === 0,
    dryRunIsDisabledPreview: dryRun?.dryRun === true
      && dryRun?.ok === false
      && dryRun?.canApply === false
      && Array.isArray(dryRun?.blockedReasons)
      && dryRun.blockedReasons.includes("feature_disabled"),
    dryRunHasNoSideEffects: allAgentConfigSideEffectsFalse(dryRun?.sideEffects),
  };

  if (!requiredChecks.applicationExists) {
    validationErrors.push("application not found.");
  }
  if (application && !requiredChecks.applicationPending) {
    validationErrors.push(`application must be pending_apply, got ${application.status}.`);
  }
  if (!requiredChecks.sourceApprovalExists) {
    validationErrors.push("source approval not found.");
  }
  if (approval && !requiredChecks.sourceApprovalApproved) {
    validationErrors.push(`source approval must be approved, got ${approval.status}.`);
  }
  if (approval && !requiredChecks.sourceApprovalTargetsAgentConfig) {
    validationErrors.push("source approval targetService must be agent_config.");
  }
  if (approval && !requiredChecks.sourceApprovalHasNoRunnerJob) {
    validationErrors.push("source approval must not have a Runner job.");
  }
  if (!requiredChecks.targetAgentExists) {
    validationErrors.push("target agent not found.");
  }
  if (!requiredChecks.secondConfirm) {
    validationErrors.push("secondConfirm=true is required.");
  }
  if (!requiredChecks.confirmText) {
    validationErrors.push("confirmText is required.");
  }
  if (!requiredChecks.requestedBy) {
    validationErrors.push("requestedBy is required.");
  }
  if (!requiredChecks.gitCheckpointPresent) {
    validationErrors.push("gitCheckpoint.created=true and gitCheckpoint.commit are required.");
  }
  if (!requiredChecks.rollbackPlanAccepted) {
    validationErrors.push("rollbackPlanAccepted=true is required.");
  }
  if (!requiredChecks.dryRunResultProvided) {
    validationErrors.push("dryRun result is required before real apply.");
  }
  if (dryRun) {
    if (!requiredChecks.dryRunMatchesApplication) {
      validationErrors.push("dryRun applicationId must match application.");
    }
    if (!requiredChecks.dryRunMatchesApproval) {
      validationErrors.push("dryRun approvalId must match source approval.");
    }
    if (!requiredChecks.dryRunMatchesAgent) {
      validationErrors.push("dryRun agentId must match target Agent.");
    }
    if (!requiredChecks.dryRunValidationPassed) {
      validationErrors.push("dryRun must have no validation errors.");
    }
    if (!requiredChecks.dryRunIsDisabledPreview) {
      validationErrors.push("dryRun must be the current feature-disabled preview.");
    }
    if (!requiredChecks.dryRunHasNoSideEffects) {
      validationErrors.push("dryRun side effects must all be false.");
    }
  }

  const changedFields = Array.isArray(dryRun?.writePlan?.changedFields)
    ? dryRun.writePlan.changedFields
    : Array.isArray(application?.changes)
      ? application.changes.map((change) => change?.field).filter(Boolean)
      : [];
  const parsedVersion = Number(agent?.configVersion || agent?.versionNumber || "");
  const currentVersion = Number.isFinite(parsedVersion) ? parsedVersion : 1;
  const targetVersion = Number(dryRun?.writePlan?.targetVersion) || currentVersion + 1;
  const preconditionsReady = validationErrors.length === 0;

  return {
    ok: false,
    realApplyGate: true,
    gateReady: false,
    preconditionsReady,
    canApply: false,
    blockedReasons,
    validationErrors,
    requiredChecks,
    applicationId: application?.id || "",
    approvalId: application?.approvalId || approval?.id || "",
    agentId: application?.agentId || agent?.id || "",
    writePlan: {
      wouldUpdateAgent: false,
      wouldCreateVersion: false,
      wouldWriteRuntimeEvent: false,
      transactionRequired: true,
      targetVersion,
      changedFields,
    },
    rollbackPlan: {
      rollbackRequiresNewApproval: true,
      rollbackAction: "create_new_agent_config_application",
      wouldRestoreVersion: currentVersion,
    },
    auditPlan: {
      requiresRequestedBy: true,
      requiresConfirmText: true,
      requiresGitCheckpoint: true,
      storesRawSecrets: false,
      createsRunnerJob: false,
    },
    sideEffects: noAgentConfigApplyGateSideEffects(),
  };
}

function buildAgentConfigApplyDryRun({ application, approval, agent, body = {} }) {
  const blockedReasons = ["feature_disabled"];
  const validationErrors = [];

  if (!application) {
    validationErrors.push("application not found.");
  }

  if (application && application.status !== "pending_apply") {
    validationErrors.push(`application must be pending_apply, got ${application.status}.`);
  }

  if (!approval) {
    validationErrors.push("source approval not found.");
  }

  if (approval) {
    if (approval.status !== "approved") {
      validationErrors.push(`source approval must be approved, got ${approval.status}.`);
    }
    if (approval.targetService !== "agent_config") {
      validationErrors.push("source approval targetService must be agent_config.");
    }
    if (approval.runnerJobId) {
      validationErrors.push("source approval must not have a Runner job.");
    }
  }

  if (!agent) {
    validationErrors.push("target agent not found.");
  }

  if (body.secondConfirm !== true) {
    validationErrors.push("secondConfirm=true is required.");
  }

  if (!body.confirmText) {
    validationErrors.push("confirmText is required.");
  }

  const changedFields = Array.isArray(application?.changes)
    ? application.changes.map((change) => change?.field).filter(Boolean)
    : [];
  const parsedVersion = Number(agent?.configVersion || agent?.versionNumber || "");
  const currentVersion = Number.isFinite(parsedVersion) ? parsedVersion : 1;
  const targetVersion = currentVersion + 1;

  return {
    ok: false,
    dryRun: true,
    applicationId: application?.id || "",
    approvalId: application?.approvalId || approval?.id || "",
    agentId: application?.agentId || agent?.id || "",
    canApply: false,
    blockedReasons,
    validationErrors,
    writePlan: {
      wouldUpdateAgent: false,
      wouldCreateVersion: false,
      wouldWriteRuntimeEvent: false,
      targetVersion,
      changedFields,
    },
    rollbackPlan: {
      rollbackRequiresNewApproval: true,
      wouldRestoreVersion: currentVersion,
      rollbackAction: "create_new_agent_config_application",
    },
    sideEffects: noAgentConfigDryRunSideEffects(),
  };
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
    permissionProfile: body.permissionProfile || body.profile || "",
    capabilities: Array.isArray(body.capabilities) ? body.capabilities : [],
    permissionValidation: body.permissionValidation || null,
  };

  if (!existing) {
    data.approvals.push(approval);
  }

  return approval;
}

async function handleApprovalAction(req, res, approvalId, action) {
  const body = await readBody(req);
  if (sqliteReadEnabled()) {
    sendSqliteWriteResult(res, runSqliteWrite("approvalAction", {
      projectId: data.projectId,
      approvalId,
      action,
      body,
    }));
    return;
  }

  const approval = findApproval(approvalId);
  if (!approval) {
    sendJson(res, 404, { error: "approval_not_found" });
    return;
  }

  if (action === "approve") {
    if (approval.requiresSecondConfirm && body.secondConfirm !== true) {
      sendJson(res, 409, {
        error: "second_confirm_required",
        message: "High risk approval requires secondConfirm=true.",
      });
      return;
    }
    approval.status = "approved";
    let agentConfigApplication = null;
    if (approval.targetService === "agent_config") {
      approval.runnerJobId = "";
      agentConfigApplication = upsertAgentConfigApplicationFromApproval(approval);
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
      agentConfigApplicationId: agentConfigApplication?.id || "",
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
  const body = await readBody(req);
  const permissionValidation = validatePermissionChangeRequest(body);
  if (permissionValidation && !permissionValidation.ok) {
    sendJson(res, 422, {
      error: "agent_permission_validation_failed",
      message: "Agent permission change request failed mock validation.",
      permissionValidation,
    });
    return;
  }

  if (permissionValidation) {
    body.permissionValidation = permissionValidation;
  }

  if (sqliteReadEnabled()) {
    sendSqliteWriteResult(res, runSqliteWrite("createAgentChangeRequest", {
      projectId: data.projectId,
      agentId,
      body,
    }));
    return;
  }

  const agent = findAgent(agentId);
  if (!agent) {
    sendJson(res, 404, { error: "agent_not_found" });
    return;
  }

  const approval = createAgentChangeApproval(agent, body);
  saveRuntimeState();
  sendJson(res, 201, {
    approval,
    permissionValidation: permissionValidation || null,
    message: "Agent change request created. Agent config was not modified.",
  });
}

async function handleAgentConfigApplicationApply(req, res, applicationId) {
  const body = await readBody(req);
  if (sqliteReadEnabled()) {
    sendSqliteWriteResult(res, runSqliteWrite("agentConfigApplicationAction", {
      projectId: data.projectId,
      applicationId,
      action: "apply",
      body,
    }));
    return;
  }

  const application = findAgentConfigApplication(applicationId);
  if (!application) {
    sendJson(res, 404, { error: "agent_config_application_not_found" });
    return;
  }

  const approval = findApproval(application.approvalId);
  if (!approval) {
    sendJson(res, 409, {
      error: "source_approval_not_found",
      message: "Agent config application must reference an existing approval.",
    });
    return;
  }

  if (body.secondConfirm !== true) {
    sendJson(res, 409, {
      error: "second_confirm_required",
      message: "Mock agent config apply requires secondConfirm=true.",
    });
    return;
  }

  if (!body.confirmText) {
    sendJson(res, 409, {
      error: "confirm_text_required",
      message: "Mock agent config apply requires confirmText.",
    });
    return;
  }

  if (application.status !== "pending_apply") {
    sendJson(res, 409, {
      error: "application_not_pending_apply",
      message: `Agent config application cannot apply from status ${application.status}.`,
    });
    return;
  }

  if (approval.status !== "approved" || approval.targetService !== "agent_config" || approval.runnerJobId) {
    sendJson(res, 409, {
      error: "application_preconditions_failed",
      message: "Agent config application requires approved agent_config approval without Runner job.",
    });
    return;
  }

  const now = new Date().toISOString();
  application.status = "applied";
  application.appliedAt = now;
  application.appliedBy = body.appliedBy || "local_user";
  application.applyConfirmText = body.confirmText;
  application.updatedAt = now;
  saveRuntimeState();

  sendJson(res, 200, {
    application,
    message: "Mock application status changed to applied. Agent config was not modified.",
  });
}

async function handleAgentConfigApplicationDryRun(req, res, applicationId) {
  const body = await readBody(req);
  const snapshot = sqliteReadEnabled() ? projectSnapshotResponse(() => null) : null;
  const applications = snapshot?.agentConfigApplications || data.agentConfigApplications;
  const approvals = snapshot?.approvals || data.approvals;
  const agents = snapshot?.agents || data.agents;
  const application = applications.find((item) => item.id === applicationId);
  const approval = application
    ? approvals.find((item) => item.id === application.approvalId)
    : null;
  const agent = application
    ? agents.find((item) => item.id === application.agentId)
    : null;

  if (!application) {
    sendJson(res, 404, {
      error: "agent_config_application_not_found",
      ok: false,
      dryRun: true,
      canApply: false,
      applicationId: "",
      blockedReasons: ["application_not_found"],
      sideEffects: noAgentConfigDryRunSideEffects(),
    });
    return;
  }

  sendJson(res, 200, buildAgentConfigApplyDryRun({
    application,
    approval,
    agent,
    body,
  }));
}

async function handleAgentConfigApplicationCancel(req, res, applicationId) {
  const body = await readBody(req);
  if (sqliteReadEnabled()) {
    sendSqliteWriteResult(res, runSqliteWrite("agentConfigApplicationAction", {
      projectId: data.projectId,
      applicationId,
      action: "cancel",
      body,
    }));
    return;
  }

  const application = findAgentConfigApplication(applicationId);
  if (!application) {
    sendJson(res, 404, { error: "agent_config_application_not_found" });
    return;
  }

  if (application.status !== "pending_apply") {
    sendJson(res, 409, {
      error: "application_not_pending_apply",
      message: `Agent config application cannot cancel from status ${application.status}.`,
    });
    return;
  }

  if (!body.reason) {
    sendJson(res, 409, {
      error: "cancel_reason_required",
      message: "Mock agent config cancel requires reason.",
    });
    return;
  }

  const now = new Date().toISOString();
  application.status = "cancelled";
  application.cancelledAt = now;
  application.cancelledBy = body.cancelledBy || "local_user";
  application.cancelReason = body.reason;
  application.updatedAt = now;
  saveRuntimeState();

  sendJson(res, 200, {
    application,
    message: "Mock application status changed to cancelled. Agent config was not modified.",
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
  const body = await readBody(req);
  if (sqliteReadEnabled()) {
    sendSqliteWriteResult(res, runSqliteWrite("transitionTask", {
      projectId: data.projectId,
      taskId,
      action,
      body,
    }));
    return;
  }

  const task = findTask(taskId);
  if (!task) {
    sendJson(res, 404, { error: "task_not_found" });
    return;
  }

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
    if (sqliteReadEnabled()) {
      const snapshot = projectSnapshotResponse(() => null);
      sendJson(res, 200, {
        mode: "sqlite",
        stateFile: runtimeStateFile,
        localTrial: localTrialInfo(),
        sqliteRuntimeState: true,
        state: {
          version: 1,
          updatedAt: new Date().toISOString(),
          approvals: snapshot?.approvals || [],
          tasks: snapshot?.tasks || [],
          runnerJobs: snapshot?.runnerJobs || [],
          agentConfigApplications: snapshot?.agentConfigApplications || [],
        },
      });
      return;
    }

    sendJson(res, 200, {
      mode: "mock",
      stateFile: runtimeStateFile,
      localTrial: localTrialInfo(),
      exists: fs.existsSync(runtimeStateFile),
      state: serializeRuntimeState(),
    });
    return;
  }

  if (req.method === "GET" && pathname === "/api/model-gateway/status") {
    sendJson(res, 200, modelGatewayStatus());
    return;
  }

  if (req.method === "POST" && pathname === "/api/model-gateway/dry-run") {
    const body = await readBody(req);
    sendJson(res, 200, modelGatewayDryRun(body));
    return;
  }

  if (req.method === "POST" && pathname === "/api/model-gateway/connectivity-test") {
    const body = await readBody(req);
    sendJson(res, 200, modelGatewayConnectivityTest(body));
    return;
  }

  if (req.method === "POST" && pathname === "/api/runtime-state/reset") {
    if (sqliteReadEnabled()) {
      resetSqliteState();
      const snapshot = projectSnapshotResponse(() => null);
      sendJson(res, 200, {
        ok: true,
        mode: "sqlite",
        message: "SQLite state reset from seed.",
        state: {
          version: 1,
          updatedAt: new Date().toISOString(),
          approvals: snapshot?.approvals || [],
          tasks: snapshot?.tasks || [],
          runnerJobs: snapshot?.runnerJobs || [],
          agentConfigApplications: snapshot?.agentConfigApplications || [],
        },
      });
      return;
    }

    resetRuntimeState();
    sendJson(res, 200, {
      ok: true,
      stateFile: runtimeStateFile,
      state: serializeRuntimeState(),
    });
    return;
  }

  if (req.method === "DELETE" && pathname === "/api/runtime-state") {
    if (sqliteReadEnabled()) {
      resetSqliteState();
      sendJson(res, 200, {
        ok: true,
        mode: "sqlite",
        message: "SQLite state reset from seed. Database file was kept.",
      });
      return;
    }

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
    sendJson(res, 200, dashboardResponse());
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/agents")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { agents: snapshot?.agents || data.agents });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/agent-config-applications")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { applications: snapshot?.agentConfigApplications || data.agentConfigApplications });
    return;
  }

  const agentChangeRequest = pathname.match(/^\/api\/agents\/([^/]+)\/change-requests$/);
  if (req.method === "POST" && agentChangeRequest) {
    await handleAgentChangeRequest(req, res, agentChangeRequest[1]);
    return;
  }

  const agentConfigApplicationDryRun = pathname.match(/^\/api\/agent-config-applications\/([^/]+)\/dry-run$/);
  if (req.method === "POST" && agentConfigApplicationDryRun) {
    await handleAgentConfigApplicationDryRun(req, res, agentConfigApplicationDryRun[1]);
    return;
  }

  const agentConfigApplicationApply = pathname.match(/^\/api\/agent-config-applications\/([^/]+)\/apply$/);
  if (req.method === "POST" && agentConfigApplicationApply) {
    await handleAgentConfigApplicationApply(req, res, agentConfigApplicationApply[1]);
    return;
  }

  const agentConfigApplicationCancel = pathname.match(/^\/api\/agent-config-applications\/([^/]+)\/cancel$/);
  if (req.method === "POST" && agentConfigApplicationCancel) {
    await handleAgentConfigApplicationCancel(req, res, agentConfigApplicationCancel[1]);
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/tasks")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { tasks: snapshot?.tasks || data.tasks });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/workflows")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { workflows: snapshot?.workflows || data.workflows });
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
    const snapshot = projectSnapshotResponse(() => null);
    const sourceApprovals = snapshot?.approvals || data.approvals;
    const approvals = sourceApprovals.filter((item) => {
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
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, snapshot?.runnerStatus || {
      ...data.runnerStatus,
      lastHeartbeatAt: new Date().toISOString(),
    });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/runner/jobs")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { jobs: snapshot?.runnerJobs || data.runnerJobs });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/git/checkpoints")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { checkpoints: snapshot?.gitCheckpoints || data.gitCheckpoints });
    return;
  }

  if (req.method === "GET" && withProject(pathname, "/knowledge/updates")) {
    const snapshot = projectSnapshotResponse(() => null);
    sendJson(res, 200, { updates: snapshot?.knowledgeUpdates || data.knowledgeUpdates });
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

if (require.main === module) {
  loadRuntimeState();

  const server = http.createServer((req, res) => {
    handleRequest(req, res).catch((error) => {
      sendJson(res, 500, { error: "internal_error", message: error.message });
    });
  });

  server.listen(port, "127.0.0.1", () => {
    console.log(`agent蜂群 mock API listening on http://127.0.0.1:${port}`);
  });
}

module.exports = {
  buildAgentConfigRealApplyGate,
  buildAgentConfigApplyDryRun,
  noAgentConfigApplyGateSideEffects,
  noAgentConfigDryRunSideEffects,
};
