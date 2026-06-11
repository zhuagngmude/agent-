const { disabledProviderAdapterRegistry, normalizeChengRelayBaseUrl } = require("./model-gateway-adapters");

const realModelProjectPlanFlagEnvVar = "AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN";

const providerConfigRegistry = {
  openai_compat: {
    id: "openai_compat",
    label: "OpenAI-compatible Relay",
    keyEnvVar: "AGENT_SWARM_OPENAI_COMPAT_API_KEY",
    baseUrlEnvVar: "AGENT_SWARM_OPENAI_COMPAT_BASE_URL",
    baseUrlRequired: true,
    keySource: "server_env",
    baseUrlSource: "server_env",
    allowedPurposes: ["project_plan_generation"],
    defaultModel: "gpt-5.4-mini",
    allowedModels: ["gpt-5.4-mini"],
  },
};

function parseString(value) {
  return typeof value === "string" ? value.trim() : "";
}

function noProviderConfigResolverSideEffects() {
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
    readsRawSecrets: false,
    returnsRawSecrets: false,
    makesNetworkRequests: false,
    modifiesGit: false,
    writesProjectFiles: false,
  };
}

function projectPlanModelFeatureFlags(env = process.env) {
  return {
    realModelProjectPlanEnvVar: realModelProjectPlanFlagEnvVar,
    realModelProjectPlanRequested: env[realModelProjectPlanFlagEnvVar] === "true",
    realModelProjectPlanActive: false,
    realProviderRequestsAllowed: false,
  };
}

function configuredEnv(env, envVar) {
  return Boolean(parseString(env[envVar]));
}

function resolveBaseUrlStatus(providerConfig, env) {
  if (!providerConfig?.baseUrlRequired) {
    return {
      envVar: "",
      source: "",
      required: false,
      configured: false,
      valid: true,
      errorCategory: "",
      validationErrors: [],
      valueReturned: false,
      normalizedValueReturned: false,
      endpointUrlReturned: false,
    };
  }

  const configured = configuredEnv(env, providerConfig.baseUrlEnvVar);
  if (!configured) {
    return {
      envVar: providerConfig.baseUrlEnvVar,
      source: providerConfig.baseUrlSource,
      required: true,
      configured: false,
      valid: false,
      errorCategory: "missing_base_url",
      validationErrors: ["base URL is required."],
      valueReturned: false,
      normalizedValueReturned: false,
      endpointUrlReturned: false,
    };
  }

  const normalized = normalizeChengRelayBaseUrl(env[providerConfig.baseUrlEnvVar]);
  return {
    envVar: providerConfig.baseUrlEnvVar,
    source: providerConfig.baseUrlSource,
    required: true,
    configured: true,
    valid: normalized.ok,
    errorCategory: normalized.ok ? "" : normalized.errorCategory,
    validationErrors: normalized.validationErrors,
    valueReturned: false,
    normalizedValueReturned: false,
    endpointUrlReturned: false,
  };
}

function resolverErrorCategory(validationErrors, blockingCategories) {
  if (validationErrors.length > 0) return "invalid_request";
  if (blockingCategories.includes("unsupported_provider")) return "unsupported_provider";
  if (blockingCategories.includes("unsupported_model")) return "unsupported_model";
  if (blockingCategories.includes("missing_key")) return "missing_key";
  if (blockingCategories.includes("missing_base_url")) return "missing_base_url";
  if (blockingCategories.includes("invalid_base_url")) return "invalid_base_url";
  return "feature_disabled";
}

function resolveModelGatewayProviderConfig(input = {}, options = {}) {
  const env = options.env || process.env;
  const provider = parseString(input.provider).toLowerCase();
  const purpose = parseString(input.purpose);
  const requestedModel = parseString(input.model);
  const providerConfig = providerConfigRegistry[provider] || null;
  const adapterPolicy = disabledProviderAdapterRegistry[provider] || null;
  const validationErrors = [];
  const blockingCategories = [];
  const flags = projectPlanModelFeatureFlags(env);

  if (!provider) {
    validationErrors.push("provider is required.");
  } else if (!providerConfig) {
    blockingCategories.push("unsupported_provider");
  }

  const model = requestedModel || providerConfig?.defaultModel || "";
  const purposeSupported = Boolean(providerConfig?.allowedPurposes.includes(purpose));
  const modelSupported = Boolean(providerConfig?.allowedModels.includes(model));

  if (!purpose) {
    validationErrors.push("purpose is required.");
  } else if (providerConfig && !purposeSupported) {
    validationErrors.push("purpose is not supported by this provider config.");
  }

  if (!model) {
    validationErrors.push("model is required.");
  } else if (providerConfig && !modelSupported) {
    blockingCategories.push("unsupported_model");
  }

  const keyConfigured = providerConfig ? configuredEnv(env, providerConfig.keyEnvVar) : false;
  if (providerConfig && !keyConfigured) {
    blockingCategories.push("missing_key");
  }

  const baseUrl = resolveBaseUrlStatus(providerConfig, env);
  if (providerConfig && baseUrl.required && !baseUrl.configured) {
    blockingCategories.push("missing_base_url");
  } else if (providerConfig && baseUrl.required && !baseUrl.valid) {
    blockingCategories.push("invalid_base_url");
  }

  if (!flags.realModelProjectPlanActive || !flags.realProviderRequestsAllowed) {
    blockingCategories.push("feature_disabled");
  }

  const uniqueBlockingCategories = [...new Set(blockingCategories)];

  return {
    ok: false,
    result: "blocked",
    errorCategory: resolverErrorCategory(validationErrors, uniqueBlockingCategories),
    provider,
    providerSupported: Boolean(providerConfig),
    purpose,
    purposeSupported,
    model,
    modelSupported,
    featureFlags: flags,
    validationErrors,
    blockingCategories: uniqueBlockingCategories,
    configSource: {
      providerSource: "server_config",
      modelSource: "server_config",
      keySource: providerConfig?.keySource || "",
      baseUrlSource: providerConfig?.baseUrlSource || "",
      acceptsClientProvider: false,
      acceptsClientModel: false,
      acceptsClientApiKey: false,
      acceptsClientBaseUrl: false,
      acceptsClientHeaders: false,
      acceptsClientProviderBody: false,
    },
    keyStatus: {
      envVar: providerConfig?.keyEnvVar || "",
      source: providerConfig?.keySource || "",
      configured: keyConfigured,
      valueReturned: false,
      suffixReturned: false,
      maskedFragmentReturned: false,
    },
    baseUrlStatus: baseUrl,
    adapterPolicy: {
      providerAdapterId: adapterPolicy?.providerAdapterId || "",
      futureProviderAdapterId: adapterPolicy?.futureProviderAdapterId || "",
      mode: adapterPolicy?.mode || "disabled",
      futureMode: adapterPolicy?.futureMode || "",
      maxTimeoutMs: adapterPolicy?.maxTimeoutMs || 0,
      maxResponseBodyLimitBytes: adapterPolicy?.maxResponseBodyLimitBytes || 0,
    },
    sideEffects: noProviderConfigResolverSideEffects(),
  };
}

module.exports = {
  noProviderConfigResolverSideEffects,
  projectPlanModelFeatureFlags,
  providerConfigRegistry,
  resolveModelGatewayProviderConfig,
};
