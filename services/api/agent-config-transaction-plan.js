function noAgentConfigTransactionPlanSideEffects() {
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

function versionNumber(agent = {}) {
  const parsed = Number(agent.configVersion || agent.versionNumber || agent.configVersionNumber || "");
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 1;
}

function buildUpdatedAgentPreview(agent = {}, changes = []) {
  const preview = {
    id: agent.id || "",
    status: agent.status || "",
    model: agent.model || "",
    permissions: Array.isArray(agent.permissions) ? [...agent.permissions] : agent.permissions || [],
    canSpawnSubAgents: agent.canSpawnSubAgents === true,
    maxSubAgents: Number.isInteger(Number(agent.maxSubAgents)) ? Number(agent.maxSubAgents) : 0,
  };

  for (const change of Array.isArray(changes) ? changes : []) {
    if (!change || typeof change !== "object") continue;
    if (Object.prototype.hasOwnProperty.call(preview, change.field)) {
      preview[change.field] = change.after;
    }
  }

  return preview;
}

function buildAgentConfigApplyTransactionPlan({
  projectId = "project_agent_swarm",
  application,
  approval,
  agent,
  dryRun,
  gate,
  body = {},
} = {}) {
  const validationErrors = [];
  const blockedReasons = ["feature_disabled"];

  if (!application) {
    validationErrors.push("application not found.");
  }
  if (application && application.status !== "pending_apply") {
    validationErrors.push(`application must be pending_apply, got ${application.status}.`);
  }
  if (!approval) {
    validationErrors.push("source approval not found.");
  }
  if (approval && approval.status !== "approved") {
    validationErrors.push(`source approval must be approved, got ${approval.status}.`);
  }
  if (approval && approval.targetService !== "agent_config") {
    validationErrors.push("source approval targetService must be agent_config.");
  }
  if (approval && approval.runnerJobId) {
    validationErrors.push("source approval must not have a Runner job.");
  }
  if (!agent) {
    validationErrors.push("target agent not found.");
  }
  if (!dryRun) {
    validationErrors.push("dryRun result is required.");
  }
  if (dryRun && dryRun.changePlanValidation?.ok !== true) {
    validationErrors.push("dryRun changePlanValidation must be ok.");
  }
  if (!gate) {
    validationErrors.push("real apply gate result is required.");
  }
  if (gate && gate.preconditionsReady !== true) {
    validationErrors.push("real apply gate preconditions must be ready.");
  }
  if (gate && gate.gateReady !== false) {
    validationErrors.push("real apply gate must still be feature-disabled.");
  }
  if (gate && gate.canApply !== false) {
    validationErrors.push("real apply gate must not allow apply in MVP.");
  }
  if (body.secondConfirm !== true) {
    validationErrors.push("secondConfirm=true is required.");
  }
  if (!body.confirmText) {
    validationErrors.push("confirmText is required.");
  }
  if (!body.appliedBy) {
    validationErrors.push("appliedBy is required.");
  }

  const changes = Array.isArray(application?.changes) ? application.changes : [];
  const currentVersion = versionNumber(agent);
  const targetVersion = Number(dryRun?.writePlan?.targetVersion) || currentVersion + 1;
  if (targetVersion !== currentVersion + 1) {
    validationErrors.push("targetVersion must increment current Agent config version by 1.");
  }

  const versionId = application && agent
    ? `agent_config_version_${agent.id}_${targetVersion}_${application.id}`
    : "";
  const runtimeEventId = application
    ? `runtime_event_agent_config_application_${application.id}_real_apply_preview`
    : "";
  const updatedAgentPreview = buildUpdatedAgentPreview(agent, changes);

  return {
    ok: false,
    transactionPlan: true,
    planReady: validationErrors.length === 0,
    canWrite: false,
    blockedReasons,
    validationErrors,
    projectId,
    applicationId: application?.id || "",
    approvalId: application?.approvalId || approval?.id || "",
    agentId: application?.agentId || agent?.id || "",
    transaction: {
      required: true,
      isolation: "sqlite_transaction",
      rollbackOnAnyFailure: true,
      idempotencyKey: application?.id || "",
      duplicateApplyGuard: "agent_config_applications.status must still be pending_apply",
    },
    versionPlan: {
      table: "agent_config_versions",
      id: versionId,
      uniqueKey: "agent_id + version",
      currentVersion,
      targetVersion,
      insertRequired: true,
      deleteHistory: false,
      overwriteHistory: false,
    },
    writeSet: {
      updateAgentsCurrentState: true,
      insertAgentConfigVersion: true,
      updateAgentConfigApplicationStatus: true,
      insertRuntimeEvent: true,
      createRunnerJob: false,
      executeRunner: false,
      callRealModel: false,
      readRawSecrets: false,
    },
    sqliteOperations: [
      {
        order: 1,
        table: "agents",
        operation: "update_current_state",
        where: { id: agent?.id || "", project_id: projectId },
      },
      {
        order: 2,
        table: "agent_config_versions",
        operation: "insert_version",
        id: versionId,
      },
      {
        order: 3,
        table: "agent_config_applications",
        operation: "mark_applied",
        where: { id: application?.id || "", status: "pending_apply" },
      },
      {
        order: 4,
        table: "runtime_events",
        operation: "insert_event",
        id: runtimeEventId,
      },
    ],
    auditPreview: {
      beforeAgent: {
        id: agent?.id || "",
        version: currentVersion,
      },
      afterAgent: {
        ...updatedAgentPreview,
        version: targetVersion,
      },
      changes,
      appliedBy: body.appliedBy || "",
      confirmTextStored: Boolean(body.confirmText),
      storesRawSecrets: false,
    },
    failureGuards: [
      "application must still be pending_apply at write time",
      "source approval must still be approved agent_config without Runner job",
      "target Agent row must exist",
      "agent_id + targetVersion must not already exist",
      "agents update and agent_config_versions insert must commit or roll back together",
      "runtime_events insert must be part of the same transaction",
    ],
    sideEffects: noAgentConfigTransactionPlanSideEffects(),
  };
}

module.exports = {
  buildAgentConfigApplyTransactionPlan,
  noAgentConfigTransactionPlanSideEffects,
};
