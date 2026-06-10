const disabledAdapterName = "disabled_provider_connectivity_adapter";

const disabledProviderAdapterRegistry = {
  openai: {
    providerAdapterId: "openai_disabled_connectivity_adapter",
    provider: "openai",
    providerLabel: "OpenAI",
    mode: "disabled",
  },
  anthropic: {
    providerAdapterId: "anthropic_disabled_connectivity_adapter",
    provider: "anthropic",
    providerLabel: "Anthropic",
    mode: "disabled",
  },
  google: {
    providerAdapterId: "google_disabled_connectivity_adapter",
    provider: "google",
    providerLabel: "Google Gemini",
    mode: "disabled",
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
      "Provider adapter is disabled in MVP-0.2.",
      "Real provider requests are blocked by the Model Gateway feature flag boundary.",
    ],
  };
}

module.exports = {
  disabledProviderAdapterRegistry,
  disabledProviderConnectivityAdapter,
};
