const modelGatewayProviders = [
  { id: "openai", label: "OpenAI", envVar: "AGENT_SWARM_OPENAI_API_KEY" },
  { id: "anthropic", label: "Anthropic", envVar: "AGENT_SWARM_ANTHROPIC_API_KEY" },
  { id: "google", label: "Google Gemini", envVar: "AGENT_SWARM_GOOGLE_API_KEY" },
];
const manualConnectivityTestFlagEnvVar = "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST";

function providerById(providerId) {
  return modelGatewayProviders.find((item) => item.id === providerId);
}

function parseProviderId(request) {
  return typeof request.provider === "string" ? request.provider.trim().toLowerCase() : "";
}

function parseString(value) {
  return typeof value === "string" ? value.trim() : "";
}

function modelGatewayFeatureFlags() {
  return {
    manualConnectivityTestEnvVar: manualConnectivityTestFlagEnvVar,
    manualConnectivityTestRequested: process.env[manualConnectivityTestFlagEnvVar] === "true",
    manualConnectivityTestActive: false,
    realProviderRequestsAllowed: false,
  };
}

function modelGatewayStatus() {
  return {
    enabled: false,
    realModelCallsAllowed: false,
    gatewayMode: "disabled",
    serviceBoundary: "server_only",
    featureFlags: modelGatewayFeatureFlags(),
    providers: modelGatewayProviders.map((provider) => ({
      id: provider.id,
      label: provider.label,
      keyEnvVar: provider.envVar,
      configured: Boolean(process.env[provider.envVar]),
      keyExposedToFrontend: false,
      canRunConnectivityTest: false,
    })),
    safety: {
      storesApiKeys: false,
      exposesApiKeysToFrontend: false,
      writesDatabase: false,
      createsTasks: false,
      createsApprovals: false,
      createsRunnerJobs: false,
      runnerExecutesCommands: false,
      logsPromptsOrResponses: false,
      makesNetworkRequests: false,
    },
    blockedReasons: [
      "Real model calls are disabled in MVP-0.2.",
      "Approval, logging, cost tracking, and key-safety rules are not ready.",
      "This endpoint only reports provider configuration boundaries.",
    ],
  };
}

function modelGatewayDryRun(request) {
  const providerId = parseProviderId(request);
  const model = parseString(request.model);
  const purpose = parseString(request.purpose);
  const provider = providerById(providerId);
  const validationErrors = [];

  if (!providerId) {
    validationErrors.push("provider is required.");
  } else if (!provider) {
    validationErrors.push("provider is not supported.");
  }

  if (!model) {
    validationErrors.push("model is required.");
  }

  if (purpose !== "connectivity_check") {
    validationErrors.push("purpose must be connectivity_check.");
  }

  return {
    ok: false,
    dryRun: true,
    provider: providerId,
    requestValid: validationErrors.length === 0,
    validationErrors,
    providerSupported: Boolean(provider),
    keyEnvVar: provider?.envVar || "",
    keyConfigured: provider ? Boolean(process.env[provider.envVar]) : false,
    featureFlags: modelGatewayFeatureFlags(),
    realModelCallsAllowed: false,
    wouldCallProvider: false,
    blockedReasons: [
      "Dry-run does not call real providers.",
      "Real model calls are disabled in MVP-0.2.",
      "Approval, logging, cost tracking, and key-safety rules are not ready.",
    ],
    sideEffects: {
      writesSqlite: false,
      writesRuntimeState: false,
      createsTasks: false,
      createsApprovals: false,
      createsRunnerJobs: false,
      triggersAgents: false,
      callsRealModel: false,
      logsPromptOrResult: false,
    },
  };
}

function modelGatewayConnectivityTest(request) {
  const providerId = parseProviderId(request);
  const model = parseString(request.model);
  const purpose = parseString(request.purpose);
  const confirmText = parseString(request.confirmText);
  const provider = providerById(providerId);
  const validationErrors = [];

  if (!providerId) {
    validationErrors.push("provider is required.");
  } else if (!provider) {
    validationErrors.push("provider is not supported.");
  }

  if (!model) {
    validationErrors.push("model is required.");
  }

  if (purpose !== "manual_connectivity_test") {
    validationErrors.push("purpose must be manual_connectivity_test.");
  }

  if (request.secondConfirm !== true) {
    validationErrors.push("secondConfirm must be true.");
  }

  if (!confirmText) {
    validationErrors.push("confirmText is required.");
  }

  return {
    ok: false,
    provider: providerId,
    model,
    purpose,
    requestValid: validationErrors.length === 0,
    validationErrors,
    providerSupported: Boolean(provider),
    keyEnvVar: provider?.envVar || "",
    keyConfigured: provider ? Boolean(process.env[provider.envVar]) : false,
    featureFlags: modelGatewayFeatureFlags(),
    realModelCallsAllowed: false,
    realProviderRequestAttempted: false,
    result: "not_implemented",
    errorCategory: "not_implemented",
    providerResponseStored: false,
    blockedReasons: [
      "Manual connectivity test is specification-only in MVP-0.2.",
      "Provider SDKs are not loaded and provider network requests are disabled.",
      "Approval, logging, cost tracking, and key-safety rules are not ready.",
    ],
    sideEffects: {
      writesSqlite: false,
      writesRuntimeState: false,
      createsTasks: false,
      createsApprovals: false,
      createsRunnerJobs: false,
      triggersAgents: false,
      callsRealModel: false,
      executesRunner: false,
      logsPromptOrResult: false,
      storesProviderResponse: false,
    },
  };
}

module.exports = {
  modelGatewayConnectivityTest,
  modelGatewayDryRun,
  modelGatewayStatus,
};
