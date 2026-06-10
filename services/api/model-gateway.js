const {
  disabledProviderAdapterRegistry,
  disabledProviderConnectivityAdapter,
} = require("./model-gateway-adapters");

const modelGatewayProviders = [
  { id: "openai", label: "OpenAI", envVar: "AGENT_SWARM_OPENAI_API_KEY" },
  {
    id: "openai_compat",
    label: "OpenAI-compatible Relay",
    envVar: "AGENT_SWARM_OPENAI_COMPAT_API_KEY",
    baseUrlEnvVar: "AGENT_SWARM_OPENAI_COMPAT_BASE_URL",
    requiresBaseUrl: true,
  },
  { id: "anthropic", label: "Anthropic", envVar: "AGENT_SWARM_ANTHROPIC_API_KEY" },
  { id: "google", label: "Google Gemini", envVar: "AGENT_SWARM_GOOGLE_API_KEY" },
];
const manualConnectivityTestFlagEnvVar = "AGENT_SWARM_ENABLE_MODEL_CONNECTIVITY_TEST";

function providerById(providerId) {
  return modelGatewayProviders.find((item) => item.id === providerId);
}

function providerAdapterPolicy(providerId) {
  return disabledProviderAdapterRegistry[providerId] || null;
}

function parseProviderId(request) {
  return typeof request.provider === "string" ? request.provider.trim().toLowerCase() : "";
}

function parseString(value) {
  return typeof value === "string" ? value.trim() : "";
}

function parsePositiveInteger(value, fallback) {
  if (value === undefined || value === null || value === "") {
    return fallback;
  }

  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    return fallback;
  }

  return parsed;
}

function baseUrlStatus(provider, options = {}) {
  if (!provider?.requiresBaseUrl) {
    return {
      required: false,
      envVar: "",
      configured: false,
      valid: true,
      validationError: "",
    };
  }

  const rawBaseUrl = options.acceptanceOnlyBaseUrl !== undefined
    ? parseString(options.acceptanceOnlyBaseUrl)
    : parseString(process.env[provider.baseUrlEnvVar]);
  const configured = Boolean(rawBaseUrl);

  if (!configured) {
    return {
      required: true,
      envVar: provider.baseUrlEnvVar,
      configured: false,
      valid: false,
      validationError: "base URL is required.",
    };
  }

  try {
    const parsedUrl = new URL(rawBaseUrl);
    const hostname = parsedUrl.hostname.toLowerCase();
    const isLocalhost = hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
    const isPrivateIpv4 = /^(10\.|192\.168\.|172\.(1[6-9]|2\d|3[0-1])\.)/.test(hostname);
    const valid = parsedUrl.protocol === "https:" && !isLocalhost && !isPrivateIpv4;
    return {
      required: true,
      envVar: provider.baseUrlEnvVar,
      configured: true,
      valid,
      validationError: valid ? "" : "base URL must be https and must not target localhost or private networks.",
    };
  } catch {
    return {
      required: true,
      envVar: provider.baseUrlEnvVar,
      configured: true,
      valid: false,
      validationError: "base URL is invalid.",
    };
  }
}

function modelGatewaySideEffects() {
  return {
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
  };
}

function modelGatewayFeatureFlags() {
  return {
    manualConnectivityTestEnvVar: manualConnectivityTestFlagEnvVar,
    manualConnectivityTestRequested: process.env[manualConnectivityTestFlagEnvVar] === "true",
    manualConnectivityTestActive: false,
    realProviderRequestsAllowed: false,
  };
}

function preflightErrorCategory(blockingCategories) {
  if (blockingCategories.includes("unsupported_provider")) return "unsupported_provider";
  if (blockingCategories.includes("unsupported_model")) return "unsupported_model";
  if (blockingCategories.includes("missing_key")) return "missing_key";
  if (blockingCategories.includes("missing_base_url")) return "invalid_request";
  if (blockingCategories.includes("invalid_base_url")) return "invalid_request";
  if (blockingCategories.includes("timeout")) return "timeout";
  if (blockingCategories.includes("provider_error")) return "provider_unavailable";
  if (blockingCategories.includes("response_body_limit")) return "invalid_request";
  if (blockingCategories.includes("feature_disabled")) return "feature_disabled";
  return "feature_disabled";
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
      baseUrlEnvVar: provider.baseUrlEnvVar || "",
      baseUrlConfigured: provider.requiresBaseUrl ? Boolean(process.env[provider.baseUrlEnvVar]) : false,
      baseUrlRequired: provider.requiresBaseUrl === true,
      providerAdapterId: disabledProviderAdapterRegistry[provider.id]?.providerAdapterId || "",
      providerAdapterMode: disabledProviderAdapterRegistry[provider.id]?.mode || "disabled",
      connectivityTestModel: disabledProviderAdapterRegistry[provider.id]?.connectivityTestModel || "",
      maxTimeoutMs: disabledProviderAdapterRegistry[provider.id]?.maxTimeoutMs || 5000,
      maxResponseBodyLimitBytes: disabledProviderAdapterRegistry[provider.id]?.maxResponseBodyLimitBytes || 4096,
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

function modelGatewayConnectivityPreflight(request, options = {}) {
  const providerId = parseProviderId(request);
  const model = parseString(request.model);
  const purpose = parseString(request.purpose);
  const confirmText = parseString(request.confirmText);
  const provider = providerById(providerId);
  const adapterPolicy = providerAdapterPolicy(providerId);
  const maxTimeoutMs = adapterPolicy?.maxTimeoutMs || 5000;
  const maxResponseBodyLimitBytes = adapterPolicy?.maxResponseBodyLimitBytes || 4096;
  const timeoutMs = parsePositiveInteger(request.timeoutMs, maxTimeoutMs);
  const responseBodyLimitBytes = parsePositiveInteger(
    request.responseBodyLimitBytes,
    maxResponseBodyLimitBytes
  );
  const validationErrors = [];
  const blockingCategories = [];
  const featureFlags = modelGatewayFeatureFlags();
  const acceptanceSimulation = parseString(options.acceptanceSimulation);
  const relayBaseUrl = baseUrlStatus(provider, options);
  const keyConfigured = provider
    ? options.acceptanceOnlyKeyConfigured === true
      ? true
      : options.acceptanceOnlyKeyConfigured === false
        ? false
        : Boolean(process.env[provider.envVar])
    : false;
  const modelSupported = Boolean(adapterPolicy && model && model === adapterPolicy.connectivityTestModel);
  const timeoutWithinLimit = timeoutMs <= maxTimeoutMs;
  const responseBodyLimitWithinLimit = responseBodyLimitBytes <= maxResponseBodyLimitBytes;

  if (!providerId) {
    validationErrors.push("provider is required.");
  } else if (!provider) {
    validationErrors.push("provider is not supported.");
    blockingCategories.push("unsupported_provider");
  }

  if (!model) {
    validationErrors.push("model is required.");
  } else if (provider && !modelSupported) {
    validationErrors.push("model is not supported for manual connectivity test.");
    blockingCategories.push("unsupported_model");
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

  if (!timeoutWithinLimit) {
    validationErrors.push("timeoutMs exceeds provider limit.");
    blockingCategories.push("timeout");
  }

  if (!responseBodyLimitWithinLimit) {
    validationErrors.push("responseBodyLimitBytes exceeds provider limit.");
    blockingCategories.push("response_body_limit");
  }

  if (!keyConfigured && provider) {
    blockingCategories.push("missing_key");
  }

  if (relayBaseUrl.required && !relayBaseUrl.configured) {
    blockingCategories.push("missing_base_url");
  } else if (relayBaseUrl.required && !relayBaseUrl.valid) {
    blockingCategories.push("invalid_base_url");
  }

  if (!featureFlags.manualConnectivityTestActive || !featureFlags.realProviderRequestsAllowed) {
    blockingCategories.push("feature_disabled");
  }

  if (acceptanceSimulation === "timeout") {
    blockingCategories.push("timeout");
  } else if (acceptanceSimulation === "provider_error") {
    blockingCategories.push("provider_error");
  }

  const uniqueBlockingCategories = [...new Set(blockingCategories)];

  return {
    ok: false,
    result: "blocked",
    errorCategory: preflightErrorCategory(uniqueBlockingCategories),
    provider: providerId,
    model,
    purpose,
    requestValid: validationErrors.length === 0,
    validationErrors,
    providerSupported: Boolean(provider),
    modelSupported,
    keyEnvVar: provider?.envVar || "",
    keyConfigured,
    baseUrlEnvVar: relayBaseUrl.envVar,
    baseUrlConfigured: relayBaseUrl.configured,
    baseUrlRequired: relayBaseUrl.required,
    baseUrlValid: relayBaseUrl.valid,
    baseUrlValidationError: relayBaseUrl.validationError,
    featureFlags,
    timeoutMs,
    responseBodyLimitBytes,
    limits: {
      maxTimeoutMs,
      maxResponseBodyLimitBytes,
    },
    checks: {
      providerSupported: Boolean(provider),
      modelPresent: Boolean(model),
      modelSupported,
      purposeValid: purpose === "manual_connectivity_test",
      secondConfirmPresent: request.secondConfirm === true,
      confirmTextPresent: Boolean(confirmText),
      featureEnabled: featureFlags.manualConnectivityTestActive === true,
      realProviderRequestsAllowed: featureFlags.realProviderRequestsAllowed === true,
      keyConfigured,
      baseUrlConfigured: relayBaseUrl.configured,
      baseUrlValid: relayBaseUrl.valid,
      timeoutWithinLimit,
      responseBodyLimitWithinLimit,
    },
    acceptanceSimulation,
    blockingCategories: uniqueBlockingCategories,
    realProviderRequestAttempted: false,
    sideEffects: modelGatewaySideEffects(),
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
    baseUrlEnvVar: provider?.baseUrlEnvVar || "",
    baseUrlConfigured: provider?.requiresBaseUrl ? Boolean(process.env[provider.baseUrlEnvVar]) : false,
    baseUrlRequired: provider?.requiresBaseUrl === true,
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
  const provider = providerById(providerId);
  const preflight = modelGatewayConnectivityPreflight(request);

  const adapterResult = disabledProviderConnectivityAdapter({
    provider: providerId,
    model,
    purpose,
  });

  return {
    ok: false,
    provider: providerId,
    model,
    purpose,
    requestValid: preflight.requestValid,
    validationErrors: preflight.validationErrors,
    providerSupported: Boolean(provider),
    modelSupported: preflight.modelSupported,
    keyEnvVar: provider?.envVar || "",
    keyConfigured: preflight.keyConfigured,
    baseUrlEnvVar: preflight.baseUrlEnvVar,
    baseUrlConfigured: preflight.baseUrlConfigured,
    baseUrlRequired: preflight.baseUrlRequired,
    baseUrlValid: preflight.baseUrlValid,
    featureFlags: modelGatewayFeatureFlags(),
    preflight,
    realModelCallsAllowed: false,
    adapter: adapterResult.adapter,
    providerAdapterId: adapterResult.providerAdapterId,
    providerAdapterMode: adapterResult.providerAdapterMode,
    realProviderRequestAttempted: adapterResult.realProviderRequestAttempted,
    result: adapterResult.result,
    errorCategory: adapterResult.errorCategory,
    providerResponseStored: adapterResult.providerResponseStored,
    durationMs: adapterResult.durationMs,
    redactionApplied: adapterResult.redactionApplied,
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
  modelGatewayConnectivityPreflight,
  modelGatewayDryRun,
  modelGatewayStatus,
};
