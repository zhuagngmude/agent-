const modelGatewayRequestShapes = {
  dryRun: {
    purpose: "connectivity_check",
    requiredFields: ["provider", "model", "purpose"],
    optionalFields: [],
    blockedByDefault: true,
  },
  connectivityTest: {
    purpose: "manual_connectivity_test",
    requiredFields: ["provider", "model", "purpose", "secondConfirm", "confirmText"],
    optionalFields: ["timeoutMs", "responseBodyLimitBytes"],
    blockedByDefault: true,
  },
};

const modelGatewayProviderCatalog = [
  {
    id: "openai",
    label: "OpenAI",
    keyEnvVar: "AGENT_SWARM_OPENAI_API_KEY",
    baseUrlEnvVar: "",
    baseUrlRequired: false,
    connectivityTestModel: "gpt-4.1-mini",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
    providerAdapterId: "openai_disabled_connectivity_adapter",
    providerAdapterMode: "disabled",
    futureProviderAdapterId: "",
    futureProviderAdapterMode: "",
  },
  {
    id: "openai_compat",
    label: "OpenAI-compatible Relay",
    keyEnvVar: "AGENT_SWARM_OPENAI_COMPAT_API_KEY",
    baseUrlEnvVar: "AGENT_SWARM_OPENAI_COMPAT_BASE_URL",
    baseUrlRequired: true,
    connectivityTestModel: "gpt-5.4-mini",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
    providerAdapterId: "openai_compat_disabled_connectivity_adapter",
    providerAdapterMode: "disabled",
    futureProviderAdapterId: "openai_compat_manual_connectivity_adapter",
    futureProviderAdapterMode: "interface_disabled",
  },
  {
    id: "anthropic",
    label: "Anthropic",
    keyEnvVar: "AGENT_SWARM_ANTHROPIC_API_KEY",
    baseUrlEnvVar: "",
    baseUrlRequired: false,
    connectivityTestModel: "claude-3-5-haiku-latest",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
    providerAdapterId: "anthropic_disabled_connectivity_adapter",
    providerAdapterMode: "disabled",
    futureProviderAdapterId: "",
    futureProviderAdapterMode: "",
  },
  {
    id: "google",
    label: "Google Gemini",
    keyEnvVar: "AGENT_SWARM_GOOGLE_API_KEY",
    baseUrlEnvVar: "",
    baseUrlRequired: false,
    connectivityTestModel: "gemini-1.5-flash",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
    providerAdapterId: "google_disabled_connectivity_adapter",
    providerAdapterMode: "disabled",
    futureProviderAdapterId: "",
    futureProviderAdapterMode: "",
  },
];

function modelGatewayContract() {
  return {
    version: "mvp-0.6",
    boundary: "disabled",
    requestShapes: modelGatewayRequestShapes,
    providerCatalog: modelGatewayProviderCatalog.map((provider) => ({
      id: provider.id,
      label: provider.label,
      keyEnvVar: provider.keyEnvVar,
      baseUrlEnvVar: provider.baseUrlEnvVar,
      baseUrlRequired: provider.baseUrlRequired,
      connectivityTestModel: provider.connectivityTestModel,
      maxTimeoutMs: provider.maxTimeoutMs,
      maxResponseBodyLimitBytes: provider.maxResponseBodyLimitBytes,
      providerAdapterId: provider.providerAdapterId,
      providerAdapterMode: provider.providerAdapterMode,
      futureProviderAdapterId: provider.futureProviderAdapterId,
      futureProviderAdapterMode: provider.futureProviderAdapterMode,
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
      "Model Gateway remains disabled in MVP-0.6.",
      "Request shape and provider metadata are frozen for this stage.",
      "Real model calls are not enabled here.",
    ],
  };
}

function providerCatalogById(providerId) {
  return modelGatewayProviderCatalog.find((item) => item.id === providerId) || null;
}

module.exports = {
  modelGatewayContract,
  modelGatewayProviderCatalog,
  modelGatewayRequestShapes,
  providerCatalogById,
};
