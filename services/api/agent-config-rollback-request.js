function noAgentConfigRollbackRequestSideEffects() {
  return {
    writesAgents: false,
    writesAgentConfigVersions: false,
    writesAgentConfigApplications: false,
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

const SNAPSHOT_FIELDS = ["permissions", "model", "status", "maxSubAgents", "canSpawnSubAgents"];
const FORBIDDEN_FIELD_PATTERNS = [
  /api.?key/i,
  /secret/i,
  /token/i,
  /authorization/i,
  /provider.?header/i,
  /provider.?response/i,
  /prompt/i,
  /runner/i,
  /command/i,
  /file/i,
  /git/i,
  /network/i,
  /workspace/i,
  /parentAgentId/i,
  /reportsToAgentId/i,
];
const FORBIDDEN_VALUE_PATTERNS = [
  /api[_-]?key/i,
  /secret/i,
  /token=/i,
  /authorization:\s*bearer/i,
  /prompt/i,
  /^[a-zA-Z]:[\\/]/,
  /\/Users\//,
  /\\Users\\/,
];

function versionValue(version = {}) {
  const source = version || {};
  const parsed = Number(source.version || source.versionNumber || "");
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0;
}

function versionAgentId(version = {}) {
  const source = version || {};
  return source.agentId || source.agent_id || "";
}

function hasForbiddenValue(value) {
  if (value === null || value === undefined) return false;
  if (Array.isArray(value)) return value.some(hasForbiddenValue);
  if (typeof value === "object") return Object.values(value).some(hasForbiddenValue);
  return FORBIDDEN_VALUE_PATTERNS.some((pattern) => pattern.test(String(value)));
}

function snapshotValidationErrors(snapshot = {}, label) {
  const errors = [];
  for (const field of Object.keys(snapshot || {})) {
    if (FORBIDDEN_FIELD_PATTERNS.some((pattern) => pattern.test(field))) {
      errors.push(`${label} snapshot contains forbidden field: ${field}`);
    }
  }
  for (const field of SNAPSHOT_FIELDS) {
    if (Object.prototype.hasOwnProperty.call(snapshot, field) && hasForbiddenValue(snapshot[field])) {
      errors.push(`${label} snapshot contains forbidden value in field: ${field}`);
    }
  }
  return errors;
}

function safeDiffValue(value) {
  return hasForbiddenValue(value) ? "[redacted_forbidden_value]" : value;
}

function safeChangeForResponse(change) {
  const source = Array.isArray(change)
    ? { field: change[0], before: change[1], after: change[2] }
    : change;
  const field = typeof source?.field === "string" ? source.field : "";
  if (!SNAPSHOT_FIELDS.includes(field) || FORBIDDEN_FIELD_PATTERNS.some((pattern) => pattern.test(field))) {
    return null;
  }

  return {
    field,
    before: safeDiffValue(source.before),
    after: safeDiffValue(source.after),
  };
}

function safeChangesForResponse(changes) {
  if (!Array.isArray(changes)) return [];
  return changes
    .map(safeChangeForResponse)
    .filter(Boolean);
}

function safeVersionForResponse(version) {
  if (!version) return null;
  const rawSnapshot = version.configSnapshot || version.config_snapshot || {};
  const configSnapshot = {};
  for (const field of SNAPSHOT_FIELDS) {
    if (Object.prototype.hasOwnProperty.call(rawSnapshot, field)) {
      configSnapshot[field] = safeDiffValue(rawSnapshot[field]);
    }
  }

  return {
    id: version.id || "",
    projectId: version.projectId || version.project_id || "",
    agentId: versionAgentId(version),
    version: versionValue(version),
    approvalId: version.approvalId || version.approval_id || "",
    applicationId: version.applicationId || version.application_id || "",
    configSnapshot,
    changes: safeChangesForResponse(version.changes),
    appliedBy: version.appliedBy || version.applied_by || "",
    appliedAt: version.appliedAt || version.applied_at || "",
    createdAt: version.createdAt || version.created_at || "",
  };
}

function buildRollbackChanges({ currentVersion, restoreVersion } = {}) {
  const currentSnapshot = currentVersion?.configSnapshot || currentVersion?.config_snapshot || {};
  const restoreSnapshot = restoreVersion?.configSnapshot || restoreVersion?.config_snapshot || {};

  return SNAPSHOT_FIELDS
    .filter((field) => JSON.stringify(currentSnapshot[field]) !== JSON.stringify(restoreSnapshot[field]))
    .map((field) => ({
      field,
      before: safeDiffValue(currentSnapshot[field]),
      after: safeDiffValue(restoreSnapshot[field]),
      current: safeDiffValue(currentSnapshot[field]),
      restore: safeDiffValue(restoreSnapshot[field]),
      action: "restore",
    }));
}

function buildAgentConfigRollbackRequest({
  originalApplication,
  sourceApproval,
  agent,
  currentVersion,
  restoreVersion,
  body = {},
} = {}) {
  const validationErrors = [];
  const blockedReasons = ["feature_disabled"];

  if (!originalApplication) {
    validationErrors.push("original application is required.");
  }
  if (originalApplication && originalApplication.status !== "applied") {
    validationErrors.push(`original application must be applied, got ${originalApplication.status}.`);
  }
  if (!sourceApproval) {
    validationErrors.push("source approval is required.");
  }
  if (sourceApproval && sourceApproval.status !== "approved") {
    validationErrors.push(`source approval must be approved, got ${sourceApproval.status}.`);
  }
  if (sourceApproval && sourceApproval.targetService !== "agent_config") {
    validationErrors.push("source approval targetService must be agent_config.");
  }
  if (sourceApproval && sourceApproval.runnerJobId) {
    validationErrors.push("source approval must not have a Runner job.");
  }
  if (!agent) {
    validationErrors.push("target agent is required.");
  }
  if (!currentVersion) {
    validationErrors.push("current version is required.");
  }
  if (!restoreVersion) {
    validationErrors.push("restore version is required.");
  }

  const currentVersionNumber = versionValue(currentVersion);
  const restoreVersionNumber = versionValue(restoreVersion);
  if (currentVersion && versionAgentId(currentVersion) !== agent?.id) {
    validationErrors.push("current version must belong to target Agent.");
  }
  if (restoreVersion && versionAgentId(restoreVersion) !== agent?.id) {
    validationErrors.push("restore version must belong to target Agent.");
  }
  if (currentVersionNumber === 0) {
    validationErrors.push("current version number is required.");
  }
  if (restoreVersionNumber === 0) {
    validationErrors.push("restore version number is required.");
  }
  if (currentVersionNumber > 0 && restoreVersionNumber > 0 && restoreVersionNumber >= currentVersionNumber) {
    validationErrors.push("restore version must be older than current version.");
  }
  validationErrors.push(
    ...snapshotValidationErrors(currentVersion?.configSnapshot || currentVersion?.config_snapshot || {}, "current version")
  );
  validationErrors.push(
    ...snapshotValidationErrors(restoreVersion?.configSnapshot || restoreVersion?.config_snapshot || {}, "restore version")
  );
  if (body.secondConfirm !== true) {
    validationErrors.push("secondConfirm=true is required.");
  }
  if (!body.confirmText) {
    validationErrors.push("confirmText is required.");
  }
  if (!body.requestedBy) {
    validationErrors.push("requestedBy is required.");
  }
  if (!body.reason) {
    validationErrors.push("rollback reason is required.");
  }

  const rollbackChanges = buildRollbackChanges({ currentVersion, restoreVersion });
  if (currentVersion && restoreVersion && rollbackChanges.length === 0) {
    validationErrors.push("rollback must include at least one changed field.");
  }

  const rollbackApprovalId = originalApplication
    ? `approval_rollback_${originalApplication.id}_to_v${restoreVersionNumber || "unknown"}`
    : "";
  const rollbackApplicationId = rollbackApprovalId
    ? `agent_config_application_${rollbackApprovalId}`
    : "";
  const futureVersion = currentVersionNumber > 0 ? currentVersionNumber + 1 : 0;

  return {
    ok: false,
    rollbackRequest: true,
    requestReady: validationErrors.length === 0,
    canCreateApproval: false,
    blockedReasons,
    validationErrors,
    originalApplicationId: originalApplication?.id || "",
    sourceApprovalId: sourceApproval?.id || originalApplication?.approvalId || "",
    agentId: agent?.id || originalApplication?.agentId || "",
    currentVersion: currentVersionNumber,
    restoreVersion: restoreVersionNumber,
    futureVersion,
    versionHistory: {
      rollbackSourceReady: Boolean(currentVersion && restoreVersion && currentVersionNumber > 0 && restoreVersionNumber > 0),
      currentVersion: safeVersionForResponse(currentVersion),
      restoreVersion: safeVersionForResponse(restoreVersion),
    },
    restoreDiff: rollbackChanges,
    rollbackPreview: {
      fieldCount: rollbackChanges.length,
      diff: rollbackChanges,
    },
    approvalDraft: {
      id: rollbackApprovalId,
      targetService: "agent_config",
      status: "pending",
      riskLevel: "high",
      requiresSecondConfirm: true,
      runnerJobId: "",
      operationTypes: ["agent_config_rollback"],
      reason: body.reason || "",
      requestedBy: body.requestedBy || "",
      changeRequest: {
        agentId: agent?.id || "",
        changeType: "rollback",
        rollbackFromApplicationId: originalApplication?.id || "",
        rollbackFromVersion: currentVersionNumber,
        rollbackToVersion: restoreVersionNumber,
        newVersionWillBe: futureVersion,
        changes: rollbackChanges,
      },
    },
    applicationDraft: {
      id: rollbackApplicationId,
      approvalId: rollbackApprovalId,
      agentId: agent?.id || "",
      changeType: "rollback",
      status: "pending_apply_after_approval",
      changes: rollbackChanges,
    },
    rollbackRules: {
      createsNewApproval: true,
      createsNewApplication: true,
      createsNewVersionOnFutureApply: true,
      deletesVersionHistory: false,
      overwritesVersionHistory: false,
      directlyUpdatesAgents: false,
      createsRunnerJob: false,
      executesRunner: false,
    },
    nextRequiredChecks: [
      "rollback approval must be approved",
      "rollback application must pass dry-run",
      "rollback application must pass field whitelist",
      "rollback application must pass real apply gate",
      "rollback application must pass transaction plan",
    ],
    sideEffects: noAgentConfigRollbackRequestSideEffects(),
  };
}

module.exports = {
  buildAgentConfigRollbackRequest,
  noAgentConfigRollbackRequestSideEffects,
};
