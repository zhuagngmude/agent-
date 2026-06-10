const disabledAdapterName = "disabled_provider_connectivity_adapter";

function disabledProviderConnectivityAdapter(request) {
  return {
    adapter: disabledAdapterName,
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
  disabledProviderConnectivityAdapter,
};
