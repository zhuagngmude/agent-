const capabilityGroups = {
  planning: [
    "canViewProject",
    "canReadKnowledge",
    "canPlanArchitecture",
    "canDraftTasks",
    "canDraftWorkflow",
    "canReviewArchitecture",
    "canProposeModelUse",
  ],
  orchestration: [
    "canCreateTasks",
    "canAssignTasks",
    "canAssignAgents",
    "canSpawnSubAgents",
    "canSetTaskPriority",
    "canRequestAgentConfigChange",
  ],
  request: [
    "canRequestExecution",
    "canRequestFileWrite",
    "canRequestCommand",
    "canRequestNetwork",
    "canRequestGitOperation",
    "canRequestModelConnectivity",
    "canRequestModelCall",
  ],
  approval: [
    "canReviewApproval",
    "canRecommendApproval",
    "canApproveLowRisk",
    "canApproveHighRisk",
    "canApproveOwnRequest",
  ],
  execution: [
    "canExecuteRunnerJob",
    "canWriteFiles",
    "canDeleteFiles",
    "canExecuteCommands",
    "canModifyGit",
    "canMakeNetworkRequests",
  ],
  secretAccess: [
    "canReferenceSecretPresence",
    "canRequestSecretUse",
    "canAccessRawSecrets",
  ],
};

const forbiddenAgentCapabilities = [
  "canApproveHighRisk",
  "canApproveOwnRequest",
  "canExecuteRunnerJob",
  "canWriteFiles",
  "canDeleteFiles",
  "canExecuteCommands",
  "canModifyGit",
  "canMakeNetworkRequests",
  "canAccessRawSecrets",
];

const agentPermissionProfiles = {
  architect_admin: [
    "canViewProject",
    "canReadKnowledge",
    "canPlanArchitecture",
    "canDraftTasks",
    "canDraftWorkflow",
    "canReviewArchitecture",
    "canProposeModelUse",
    "canCreateTasks",
    "canAssignTasks",
    "canAssignAgents",
    "canSpawnSubAgents",
    "canSetTaskPriority",
    "canRequestAgentConfigChange",
    "canRequestExecution",
    "canRequestModelConnectivity",
    "canRequestModelCall",
    "canReferenceSecretPresence",
    "canRequestSecretUse",
  ],
  executor_agent: [
    "canViewProject",
    "canReadKnowledge",
    "canDraftTasks",
    "canRequestExecution",
    "canRequestFileWrite",
    "canRequestCommand",
    "canRequestGitOperation",
  ],
  reviewer_agent: [
    "canViewProject",
    "canReadKnowledge",
    "canReviewArchitecture",
    "canReviewApproval",
    "canRecommendApproval",
  ],
  all_agents_full_management: [
    "canViewProject",
    "canReadKnowledge",
    "canPlanArchitecture",
    "canDraftTasks",
    "canDraftWorkflow",
    "canReviewArchitecture",
    "canProposeModelUse",
    "canCreateTasks",
    "canAssignTasks",
    "canAssignAgents",
    "canSpawnSubAgents",
    "canSetTaskPriority",
    "canRequestAgentConfigChange",
    "canRequestExecution",
    "canRequestFileWrite",
    "canRequestCommand",
    "canRequestNetwork",
    "canRequestGitOperation",
    "canRequestModelConnectivity",
    "canRequestModelCall",
    "canReviewApproval",
    "canRecommendApproval",
    "canReferenceSecretPresence",
    "canRequestSecretUse",
  ],
};

function unique(values) {
  return [...new Set(values)];
}

function allKnownCapabilities() {
  return unique(Object.values(capabilityGroups).flat());
}

function normalizeCapabilityList(capabilities) {
  if (!Array.isArray(capabilities)) {
    return [];
  }

  return unique(
    capabilities
      .filter((capability) => typeof capability === "string")
      .map((capability) => capability.trim())
      .filter(Boolean)
  );
}

function expandAgentPermissionProfile(profileName) {
  const profile = typeof profileName === "string" ? profileName.trim() : "";
  const capabilities = agentPermissionProfiles[profile];

  return {
    ok: Boolean(capabilities),
    profile,
    capabilities: capabilities ? [...capabilities] : [],
    validationErrors: capabilities ? [] : ["permission profile is not supported."],
  };
}

function validateAgentCapabilities(input = {}) {
  const profile = typeof input.profile === "string" ? input.profile.trim() : "";
  const allFlagRequested = input.all === true;
  const profileResult = profile ? expandAgentPermissionProfile(profile) : null;
  const capabilities = normalizeCapabilityList(
    profileResult?.ok ? profileResult.capabilities : input.capabilities
  );
  const knownCapabilities = allKnownCapabilities();
  const validationErrors = [];
  const unknownCapabilities = capabilities.filter((capability) => !knownCapabilities.includes(capability));
  const forbiddenCapabilities = capabilities.filter((capability) =>
    forbiddenAgentCapabilities.includes(capability)
  );

  if (allFlagRequested) {
    validationErrors.push("all=true is not a valid Agent permission contract.");
  }

  if (profile && !profileResult.ok) {
    validationErrors.push(...profileResult.validationErrors);
  }

  for (const capability of unknownCapabilities) {
    validationErrors.push(`unknown capability: ${capability}`);
  }

  for (const capability of forbiddenCapabilities) {
    validationErrors.push(`forbidden Agent capability: ${capability}`);
  }

  return {
    ok: validationErrors.length === 0,
    profile,
    allFlagRequested,
    capabilities,
    unknownCapabilities,
    forbiddenCapabilities,
    validationErrors,
    sideEffects: {
      writesSqlite: false,
      writesRuntimeState: false,
      createsTasks: false,
      createsApprovals: false,
      createsRunnerJobs: false,
      triggersAgents: false,
      executesRunner: false,
      callsRealModel: false,
      readsRawSecrets: false,
    },
  };
}

module.exports = {
  agentPermissionProfiles,
  allKnownCapabilities,
  capabilityGroups,
  expandAgentPermissionProfile,
  forbiddenAgentCapabilities,
  validateAgentCapabilities,
};
