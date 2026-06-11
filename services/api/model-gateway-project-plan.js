const projectPlanPurpose = "project_plan_generation";
const realModelProjectPlanFlagEnvVar = "AGENT_SWARM_ENABLE_REAL_MODEL_PROJECT_PLAN";
const { resolveModelGatewayProviderConfig } = require("./model-gateway-provider-config");

const defaultProjectPlanProvider = "openai_compat";
const defaultProjectPlanModel = "gpt-5.4-mini";

const forbiddenClientFieldTokens = [
  "apikey",
  "api_key",
  "authorization",
  "baseurl",
  "base_url",
  "endpoint",
  "headers",
  "providerbody",
  "providerrequestbody",
  "provideroptions",
  "requestbody",
  "systemprompt",
  "prompt",
  "stream",
  "tools",
  "toolcalls",
  "functions",
  "files",
  "runnerjobid",
  "runner_job_id",
];

const forbiddenValuePatterns = [
  { pattern: /sk-[a-z0-9_-]+/i, message: "request must not contain OpenAI-style API keys." },
  { pattern: /api[_-]?key\s*=/i, message: "request must not contain API key values." },
  { pattern: /authorization\s*:\s*bearer/i, message: "request must not contain authorization headers." },
  { pattern: /token\s*=/i, message: "request must not contain token values." },
  { pattern: /password\s*=/i, message: "request must not contain password values." },
];

function parseString(value) {
  return typeof value === "string" ? value.trim() : "";
}

function fieldToken(field) {
  return String(field || "").replace(/[^a-z0-9_]/gi, "").toLowerCase();
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
    readsRawSecrets: false,
    returnsRawSecrets: false,
    makesNetworkRequests: false,
    modifiesGit: false,
    writesProjectFiles: false,
  };
}

function featureFlags() {
  return {
    realModelProjectPlanEnvVar: realModelProjectPlanFlagEnvVar,
    realModelProjectPlanRequested: process.env[realModelProjectPlanFlagEnvVar] === "true",
    realModelProjectPlanActive: false,
    realProviderRequestsAllowed: false,
  };
}

function collectForbiddenClientFields(value, path = []) {
  if (value === null || value === undefined || typeof value !== "object") {
    return [];
  }

  if (Array.isArray(value)) {
    return value.flatMap((item, index) => collectForbiddenClientFields(item, [...path, String(index)]));
  }

  const findings = [];
  for (const [key, child] of Object.entries(value)) {
    const token = fieldToken(key);
    if (forbiddenClientFieldTokens.some((item) => token.includes(item))) {
      findings.push([...path, key].join("."));
    }
    findings.push(...collectForbiddenClientFields(child, [...path, key]));
  }
  return findings;
}

function collectForbiddenValues(value, path = []) {
  if (value === null || value === undefined) {
    return [];
  }

  if (typeof value === "string") {
    return forbiddenValuePatterns
      .filter((item) => item.pattern.test(value))
      .map((item) => ({
        path: path.join(".") || "request",
        message: item.message,
      }));
  }

  if (Array.isArray(value)) {
    return value.flatMap((item, index) => collectForbiddenValues(item, [...path, String(index)]));
  }

  if (typeof value === "object") {
    return Object.entries(value).flatMap(([key, child]) => collectForbiddenValues(child, [...path, key]));
  }

  return [];
}

function errorCategoryFor(validationErrors, blockingCategories) {
  if (validationErrors.length > 0) return "invalid_request";
  if (blockingCategories.includes("missing_key")) return "missing_key";
  if (blockingCategories.includes("unsupported_provider")) return "unsupported_provider";
  if (blockingCategories.includes("unsupported_model")) return "unsupported_model";
  return "feature_disabled";
}

function buildProjectPlanGenerationModelRequest(request = {}, options = {}) {
  const projectId = parseString(request.projectId);
  const purpose = parseString(request.purpose);
  const idea = parseString(request.idea);
  const constraints = parseString(request.constraints);
  const requestedBy = parseString(request.requestedBy);
  const provider = parseString(options.provider) || defaultProjectPlanProvider;
  const model = parseString(options.model) || defaultProjectPlanModel;
  const validationErrors = [];
  const blockingCategories = [];
  const flags = featureFlags();
  const providerConfig = resolveModelGatewayProviderConfig({
    provider,
    model,
    purpose: projectPlanPurpose,
  }, options.providerConfigOptions || {});

  if (!projectId) {
    validationErrors.push("projectId is required.");
  }

  if (purpose !== projectPlanPurpose) {
    validationErrors.push("purpose must be project_plan_generation.");
  }

  if (!idea) {
    validationErrors.push("idea is required.");
  }

  if (!requestedBy) {
    validationErrors.push("requestedBy is required.");
  }

  const forbiddenFields = collectForbiddenClientFields(request);
  if (forbiddenFields.length > 0) {
    validationErrors.push(`request contains forbidden client-controlled fields: ${forbiddenFields.join(", ")}.`);
  }

  const forbiddenValues = collectForbiddenValues(request);
  for (const finding of forbiddenValues) {
    validationErrors.push(`${finding.path}: ${finding.message}`);
  }

  if (!flags.realModelProjectPlanActive || !flags.realProviderRequestsAllowed) {
    blockingCategories.push("feature_disabled");
  }

  const uniqueBlockingCategories = [...new Set(blockingCategories)];
  const requestValid = validationErrors.length === 0;

  return {
    ok: false,
    result: "blocked",
    errorCategory: errorCategoryFor(validationErrors, uniqueBlockingCategories),
    purpose: projectPlanPurpose,
    provider,
    model,
    requestShape: "project_plan_generation_v1",
    requestValid,
    validationErrors,
    providerConfig,
    featureFlags: flags,
    blockingCategories: uniqueBlockingCategories,
    realProviderRequestAttempted: false,
    providerResponseStored: false,
    redactionApplied: true,
    inputSummary: {
      projectIdPresent: Boolean(projectId),
      ideaPresent: Boolean(idea),
      ideaLength: idea.length,
      constraintsPresent: Boolean(constraints),
      constraintsLength: constraints.length,
      requestedByPresent: Boolean(requestedBy),
    },
    requestPolicy: {
      providerSource: "server_config",
      modelSource: "server_config",
      acceptsClientApiKey: false,
      acceptsClientBaseUrl: false,
      acceptsClientHeaders: false,
      acceptsClientProviderBody: false,
      acceptsFreeFormPrompt: false,
      acceptsSystemPrompt: false,
      acceptsStreamSetting: false,
      acceptsFiles: false,
      acceptsToolCalls: false,
      acceptsRunnerJob: false,
      writesProjectPlanApprovalOnlyAfterLaterRouteApproval: true,
    },
    outputTarget: {
      structuredSummaryOnly: true,
      targetService: "project_plan",
      createsTasksBeforeApproval: false,
      createsRunnerRequestsBeforeApproval: false,
      storesRawProviderResponse: false,
      logsPromptOrResult: false,
    },
    blockedReasons: [
      "Real project plan model calls are feature-disabled.",
      "This helper only validates the future request shape.",
      "No provider SDK is loaded and no provider network request is attempted.",
    ],
    sideEffects: sideEffects(),
  };
}

module.exports = {
  buildProjectPlanGenerationModelRequest,
  projectPlanGenerationFeatureFlags: featureFlags,
  projectPlanGenerationSideEffects: sideEffects,
};
