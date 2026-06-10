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

function versionValue(version = {}) {
  const parsed = Number(version.version || version.versionNumber || "");
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0;
}

function versionAgentId(version = {}) {
  return version.agentId || version.agent_id || "";
}

function buildRollbackChanges({ currentVersion, restoreVersion } = {}) {
  const currentSnapshot = currentVersion?.configSnapshot || currentVersion?.config_snapshot || {};
  const restoreSnapshot = restoreVersion?.configSnapshot || restoreVersion?.config_snapshot || {};
  const fields = ["permissions", "model", "status", "maxSubAgents", "canSpawnSubAgents"];

  return fields
    .filter((field) => JSON.stringify(currentSnapshot[field]) !== JSON.stringify(restoreSnapshot[field]))
    .map((field) => ({
      field,
      before: currentSnapshot[field],
      after: restoreSnapshot[field],
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
