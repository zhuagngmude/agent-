const defaultResponseBodyLimitBytes = 4096;
const defaultSummaryLimitBytes = 2048;

const redactionRules = [
  { pattern: /sk-[a-z0-9_-]+/gi, replacement: "[REDACTED_SECRET]" },
  { pattern: /(["']?authorization["']?\s*:\s*["']?\s*bearer\s+)[^"'\s,}]+/gi, replacement: "$1[REDACTED_SECRET]" },
  { pattern: /(["']?api[_-]?key["']?\s*[:=]\s*["']?)[^"',\s}]+/gi, replacement: "$1[REDACTED_SECRET]" },
  { pattern: /(["']?token["']?\s*[:=]\s*["']?)[^"',\s}]+/gi, replacement: "$1[REDACTED_SECRET]" },
  { pattern: /(["']?password["']?\s*[:=]\s*["']?)[^"',\s}]+/gi, replacement: "$1[REDACTED_SECRET]" },
  { pattern: /https?:\/\/[^\s"',)]+/gi, replacement: "[REDACTED_URL]" },
];

function noModelGatewayRedactionSideEffects() {
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

function stringifyInput(value) {
  if (value === undefined || value === null) {
    return "";
  }

  if (typeof value === "string") {
    return value;
  }

  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function byteLength(value) {
  return Buffer.byteLength(value, "utf8");
}

function trimToByteLimit(value, limit) {
  let output = value;
  while (byteLength(output) > limit && output.length > 0) {
    output = output.slice(0, -1);
  }
  return output;
}

function redactModelGatewayText(value, options = {}) {
  const limit = Number.isInteger(options.limitBytes) && options.limitBytes > 0
    ? options.limitBytes
    : defaultResponseBodyLimitBytes;
  const input = stringifyInput(value);
  const originalBytes = byteLength(input);
  let text = input;
  let redactionApplied = false;

  for (const rule of redactionRules) {
    const next = text.replace(rule.pattern, rule.replacement);
    if (next !== text) {
      redactionApplied = true;
      text = next;
    }
  }

  const redactedBytes = byteLength(text);
  text = trimToByteLimit(text, limit);
  const truncated = redactedBytes > byteLength(text);
  redactionApplied = redactionApplied || truncated;

  return {
    text,
    originalBytes,
    outputBytes: byteLength(text),
    limitBytes: limit,
    truncated,
    responseBodyLimitExceeded: originalBytes > limit,
    redactionApplied,
    sideEffects: noModelGatewayRedactionSideEffects(),
  };
}

function parsePositiveInteger(value) {
  const number = Number(value);
  return Number.isInteger(number) && number >= 0 ? number : 0;
}

function normalizeTokenUsage(value = {}) {
  return {
    promptTokens: parsePositiveInteger(value.promptTokens ?? value.prompt_tokens),
    completionTokens: parsePositiveInteger(value.completionTokens ?? value.completion_tokens),
    totalTokens: parsePositiveInteger(value.totalTokens ?? value.total_tokens),
  };
}

function normalizeCostEstimate(value = {}) {
  const amount = Number(value.amount);
  return {
    amount: Number.isFinite(amount) && amount >= 0 ? amount : 0,
    currency: typeof value.currency === "string" && value.currency.trim()
      ? value.currency.trim().toUpperCase()
      : "USD",
  };
}

function buildSafeModelCallRecordDraft(input = {}) {
  const summary = redactModelGatewayText(input.structuredSummary || "", {
    limitBytes: input.summaryLimitBytes || defaultSummaryLimitBytes,
  });
  const status = typeof input.status === "string" && input.status.trim()
    ? input.status.trim()
    : "blocked";
  const errorCategory = typeof input.errorCategory === "string" && input.errorCategory.trim()
    ? input.errorCategory.trim()
    : "feature_disabled";

  return {
    modelCallRecordReady: false,
    canWrite: false,
    provider: typeof input.provider === "string" ? input.provider.trim() : "",
    model: typeof input.model === "string" ? input.model.trim() : "",
    purpose: typeof input.purpose === "string" ? input.purpose.trim() : "",
    status,
    errorCategory,
    durationMs: parsePositiveInteger(input.durationMs),
    tokenUsage: normalizeTokenUsage(input.tokenUsage || {}),
    costEstimate: normalizeCostEstimate(input.costEstimate || {}),
    structuredSummary: summary.text,
    redactionApplied: true,
    responseBodyLimitExceeded: summary.responseBodyLimitExceeded,
    storesRawPrompt: false,
    storesRawProviderRequest: false,
    storesRawProviderResponse: false,
    storesRawProviderError: false,
    storesRequestHeaders: false,
    storesResponseHeaders: false,
    storesModelReasoning: false,
    storesKeyMaterial: false,
    providerResponseStored: false,
    validation: {
      summaryOriginalBytes: summary.originalBytes,
      summaryOutputBytes: summary.outputBytes,
      summaryLimitBytes: summary.limitBytes,
      summaryTruncated: summary.truncated,
    },
    sideEffects: noModelGatewayRedactionSideEffects(),
  };
}

module.exports = {
  buildSafeModelCallRecordDraft,
  noModelGatewayRedactionSideEffects,
  redactModelGatewayText,
};
