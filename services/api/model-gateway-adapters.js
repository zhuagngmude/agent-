const disabledAdapterName = "disabled_provider_connectivity_adapter";
const openAiCompatRelayAdapterName = "openai_compat_relay_connectivity_adapter_interface";
const chengRelayManualPingModel = "gpt-5.4-mini";
const chengRelayManualPingEndpointPath = "/v1/chat/completions";

const disabledProviderAdapterRegistry = {
  openai: {
    providerAdapterId: "openai_disabled_connectivity_adapter",
    provider: "openai",
    providerLabel: "OpenAI",
    mode: "disabled",
    connectivityTestModel: "gpt-4.1-mini",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
  },
  openai_compat: {
    providerAdapterId: "openai_compat_disabled_connectivity_adapter",
    futureProviderAdapterId: "openai_compat_manual_connectivity_adapter",
    provider: "openai_compat",
    providerLabel: "OpenAI-compatible Relay",
    mode: "disabled",
    futureMode: "interface_disabled",
    connectivityTestModel: chengRelayManualPingModel,
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
  },
  anthropic: {
    providerAdapterId: "anthropic_disabled_connectivity_adapter",
    provider: "anthropic",
    providerLabel: "Anthropic",
    mode: "disabled",
    connectivityTestModel: "claude-3-5-haiku-latest",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
  },
  google: {
    providerAdapterId: "google_disabled_connectivity_adapter",
    provider: "google",
    providerLabel: "Google Gemini",
    mode: "disabled",
    connectivityTestModel: "gemini-1.5-flash",
    maxTimeoutMs: 5000,
    maxResponseBodyLimitBytes: 4096,
  },
};

function disabledProviderConnectivityAdapter(request) {
  const providerAdapter = disabledProviderAdapterRegistry[request.provider] || null;

  return {
    adapter: disabledAdapterName,
    providerAdapterId: providerAdapter?.providerAdapterId || "",
    providerAdapterMode: providerAdapter?.mode || "disabled",
    ok: false,
    provider: request.provider,
    model: request.model,
    purpose: request.purpose,
    result: "blocked",
    errorCategory: "feature_disabled",
    realProviderRequestAttempted: false,
    providerResponseStored: false,
    durationMs: 0,
    redactionApplied: true,
    blockedReasons: [
      "Provider adapter is disabled in MVP-0.5.",
      "Real provider requests are blocked by the Model Gateway feature flag boundary.",
    ],
  };
}

function sideEffects() {
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

function isPrivateIpv4Host(hostname) {
  return /^(10\.|127\.|192\.168\.|172\.(1[6-9]|2\d|3[0-1])\.)/.test(hostname);
}

function normalizeChengRelayBaseUrl(rawBaseUrl) {
  const value = typeof rawBaseUrl === "string" ? rawBaseUrl.trim() : "";
  const validationErrors = [];

  if (!value) {
    return {
      ok: false,
      errorCategory: "missing_base_url",
      validationErrors: ["base URL is required."],
      normalizedBaseUrl: "",
      endpointUrl: "",
    };
  }

  let parsedUrl;
  try {
    parsedUrl = new URL(value);
  } catch {
    return {
      ok: false,
      errorCategory: "invalid_base_url",
      validationErrors: ["base URL is invalid."],
      normalizedBaseUrl: "",
      endpointUrl: "",
    };
  }

  const hostname = parsedUrl.hostname.toLowerCase();
  const pathname = parsedUrl.pathname.replace(/\/+$/, "");

  if (parsedUrl.protocol !== "https:") {
    validationErrors.push("base URL must use https.");
  }

  if (hostname === "localhost" || hostname === "::1" || isPrivateIpv4Host(hostname)) {
    validationErrors.push("base URL must not target localhost, loopback, or private networks.");
  }

  if (parsedUrl.username || parsedUrl.password || parsedUrl.search || parsedUrl.hash) {
    validationErrors.push("base URL must not contain credentials, query strings, or fragments.");
  }

  if (pathname && pathname !== "/v1") {
    validationErrors.push("base URL path must be empty or /v1.");
  }

  if (validationErrors.length > 0) {
    return {
      ok: false,
      errorCategory: "invalid_base_url",
      validationErrors,
      normalizedBaseUrl: "",
      endpointUrl: "",
    };
  }

  const origin = `${parsedUrl.protocol}//${parsedUrl.host}`;
  const normalizedBaseUrl = pathname === "/v1" ? `${origin}/v1` : origin;
  const endpointUrl = pathname === "/v1"
    ? `${normalizedBaseUrl}/chat/completions`
    : `${normalizedBaseUrl}${chengRelayManualPingEndpointPath}`;

  return {
    ok: true,
    errorCategory: "",
    validationErrors: [],
    normalizedBaseUrl,
    endpointUrl,
  };
}

function buildChengRelayManualPingRequest(input = {}) {
  const model = typeof input.model === "string" ? input.model.trim() : "";
  const baseUrlResult = normalizeChengRelayBaseUrl(input.baseUrl);
  const validationErrors = [...baseUrlResult.validationErrors];

  if (!model) {
    validationErrors.push("model is required.");
  } else if (model !== chengRelayManualPingModel) {
    validationErrors.push("model is not supported for cheng relay manual ping.");
  }

  const ok = baseUrlResult.ok && validationErrors.length === 0;

  return {
    ok,
    provider: "openai_compat",
    model,
    endpointUrl: ok ? baseUrlResult.endpointUrl : "",
    method: "POST",
    headers: {
      authorizationSource: "server_env",
      contentType: "application/json",
      acceptsClientHeaders: false,
    },
    body: ok ? {
      model: chengRelayManualPingModel,
      messages: [
        {
          role: "user",
          content: "ping",
        },
      ],
      stream: false,
      max_tokens: 1,
    } : null,
    result: ok ? "ready" : "blocked",
    errorCategory: ok ? "" : (baseUrlResult.errorCategory || "unsupported_model"),
    validationErrors,
    acceptsClientApiKey: false,
    acceptsClientBaseUrl: false,
    acceptsClientPrompt: false,
    acceptsClientHeaders: false,
    acceptsClientStreamSetting: false,
    realProviderRequestAttempted: false,
    providerResponseStored: false,
    sideEffects: sideEffects(),
  };
}

function classifyRelayInterfaceError(preflight) {
  const categories = Array.isArray(preflight?.blockingCategories) ? preflight.blockingCategories : [];

  if (categories.includes("unsupported_provider")) return "unsupported_provider";
  if (categories.includes("unsupported_model")) return "unsupported_model";
  if (categories.includes("missing_key")) return "missing_key";
  if (categories.includes("missing_base_url")) return "invalid_request";
  if (categories.includes("invalid_base_url")) return "invalid_request";
  if (categories.includes("timeout")) return "timeout";
  if (categories.includes("provider_error")) return "provider_unavailable";
  if (categories.includes("response_body_limit")) return "invalid_request";
  return "feature_disabled";
}

function openAiCompatRelayConnectivityAdapter(request) {
  const policy = disabledProviderAdapterRegistry.openai_compat;
  const preflight = request.preflight || {};

  return {
    adapter: openAiCompatRelayAdapterName,
    providerAdapterId: policy.futureProviderAdapterId,
    providerAdapterMode: policy.futureMode,
    ok: false,
    provider: "openai_compat",
    model: request.model || "",
    purpose: request.purpose || "",
    result: "blocked",
    errorCategory: classifyRelayInterfaceError(preflight),
    realProviderRequestAttempted: false,
    providerResponseStored: false,
    durationMs: 0,
    redactionApplied: true,
    interfaceOnly: true,
    requestShape: {
      provider: "openai_compat",
      keySource: "server_env",
      baseUrlSource: "server_env",
      acceptsRequestBaseUrl: false,
      acceptsApiKeyFromClient: false,
      acceptsFreeFormPrompt: false,
      acceptsAgentContext: false,
      acceptsFiles: false,
      acceptsToolCalls: false,
      acceptsRunnerJob: false,
      fixedMinimalPingOnly: true,
      endpointShapeConfirmed: true,
      fixedModel: chengRelayManualPingModel,
      fixedEndpointPath: chengRelayManualPingEndpointPath,
    },
    limits: {
      maxTimeoutMs: policy.maxTimeoutMs,
      maxResponseBodyLimitBytes: policy.maxResponseBodyLimitBytes,
    },
    sideEffects: sideEffects(),
    blockedReasons: [
      "OpenAI-compatible relay adapter is interface-only in this checkpoint.",
      "Relay endpoint shape and model name must be confirmed before any real request.",
      "Real provider requests remain blocked by the MVP-0.5 Model Gateway feature flag boundary.",
    ],
  };
}

module.exports = {
  buildChengRelayManualPingRequest,
  disabledProviderAdapterRegistry,
  disabledProviderConnectivityAdapter,
  normalizeChengRelayBaseUrl,
  openAiCompatRelayConnectivityAdapter,
};
