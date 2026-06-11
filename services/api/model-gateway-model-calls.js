const { buildSafeModelCallRecordDraft } = require("./model-gateway-redaction");

const allowedModelCallStatuses = ["blocked", "pending", "running", "succeeded", "failed"];
const allowedErrorCategories = [
  "feature_disabled",
  "missing_key",
  "invalid_request",
  "unsupported_provider",
  "unsupported_model",
  "timeout",
  "provider_unavailable",
  "network_error",
  "response_body_limit",
  "redaction_failed",
  "unknown",
];

const modelCallColumns = [
  "id",
  "project_id",
  "purpose",
  "provider",
  "provider_adapter_id",
  "model",
  "status",
  "request_source",
  "request_hash",
  "response_schema_version",
  "token_usage",
  "cost_estimate",
  "duration_ms",
  "error_category",
  "redaction_applied",
  "structured_summary",
  "related_approval_id",
  "related_agent_run_id",
  "related_task_id",
  "runtime_event_id",
  "created_by",
  "started_at",
  "completed_at",
  "failed_at",
  "created_at",
  "updated_at",
];

function noModelCallWriteDraftSideEffects() {
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

function parseString(value) {
  return typeof value === "string" ? value.trim() : "";
}

function parsePositiveInteger(value) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : 0;
}

function buildModelCallWriteDraft(input = {}) {
  const projectId = parseString(input.projectId) || "project_agent_swarm";
  const purpose = parseString(input.purpose) || "project_plan_generation";
  const provider = parseString(input.provider) || "openai_compat";
  const providerAdapterId = parseString(input.providerAdapterId) || `${provider}_adapter`;
  const model = parseString(input.model) || "gpt-5.4-mini";
  const requestSource = parseString(input.requestSource) || "server_config";
  const requestHash = parseString(input.requestHash);
  const responseSchemaVersion = parsePositiveInteger(input.responseSchemaVersion) || 1;
  const createdBy = parseString(input.createdBy) || "api";
  const status = parseString(input.status) || "blocked";
  const errorCategory = parseString(input.errorCategory) || "feature_disabled";
  const durationMs = parsePositiveInteger(input.durationMs);
  const tokenUsage = input.tokenUsage || {};
  const costEstimate = input.costEstimate || {};
  const structuredSummary = typeof input.structuredSummary === "string" ? input.structuredSummary : "";
  const summaryLimitBytes = Number.isInteger(input.summaryLimitBytes) && input.summaryLimitBytes > 0
    ? input.summaryLimitBytes
    : undefined;
  const relatedApprovalId = parseString(input.relatedApprovalId);
  const relatedAgentRunId = parseString(input.relatedAgentRunId);
  const relatedTaskId = parseString(input.relatedTaskId);
  const modelCallId = parseString(input.modelCallId) || (requestHash ? `model_call_${requestHash.slice(0, 16)}` : "");
  const runtimeEventId = parseString(input.runtimeEventId) || (requestHash ? `runtime_event_model_call_${requestHash.slice(0, 16)}` : "");

  const validationErrors = [];
  if (!projectId) {
    validationErrors.push("projectId is required.");
  }
  if (!requestHash) {
    validationErrors.push("requestHash is required.");
  }
  if (!modelCallId) {
    validationErrors.push("modelCallId is required.");
  }
  if (purpose !== "project_plan_generation") {
    validationErrors.push("purpose must be project_plan_generation.");
  }
  if (provider !== "openai_compat") {
    validationErrors.push("provider must be openai_compat.");
  }
  if (model !== "gpt-5.4-mini") {
    validationErrors.push("model must be gpt-5.4-mini.");
  }
  if (!allowedModelCallStatuses.includes(status)) {
    validationErrors.push(`status must be one of: ${allowedModelCallStatuses.join(", ")}.`);
  }
  if (!allowedErrorCategories.includes(errorCategory)) {
    validationErrors.push(`errorCategory must be one of: ${allowedErrorCategories.join(", ")}.`);
  }
  if (responseSchemaVersion < 1) {
    validationErrors.push("responseSchemaVersion must be greater than or equal to 1.");
  }

  const recordDraft = input.safeRecordDraft || buildSafeModelCallRecordDraft({
    provider,
    model,
    purpose,
    status,
    errorCategory,
    durationMs,
    tokenUsage,
    costEstimate,
    structuredSummary,
    summaryLimitBytes,
  });

  if (!recordDraft || recordDraft.modelCallRecordReady !== false || recordDraft.canWrite !== false) {
    validationErrors.push("safe model-call record draft must remain write-disabled.");
  }
  if (recordDraft.provider !== provider) {
    validationErrors.push("safe record draft provider must match write draft provider.");
  }
  if (recordDraft.model !== model) {
    validationErrors.push("safe record draft model must match write draft model.");
  }
  if (recordDraft.purpose !== purpose) {
    validationErrors.push("safe record draft purpose must match write draft purpose.");
  }
  if (recordDraft.status !== status) {
    validationErrors.push("safe record draft status must match write draft status.");
  }
  if (recordDraft.errorCategory !== errorCategory) {
    validationErrors.push("safe record draft errorCategory must match write draft errorCategory.");
  }

  const planReady = validationErrors.length === 0;

  return {
    ok: false,
    result: "blocked",
    writeDraft: true,
    canWrite: false,
    modelCallRecordReady: false,
    planReady,
    blockedReasons: ["feature_disabled"],
    validationErrors,
    projectId,
    modelCallId,
    requestShape: "model_call_write_draft_v1",
    request: {
      purpose,
      provider,
      providerAdapterId,
      model,
      requestSource,
      requestHash,
      responseSchemaVersion,
    },
    statusFlow: {
      initial: status,
      allowed: [...allowedModelCallStatuses],
      terminal: ["succeeded", "failed"],
    },
    migrationDraft: {
      table: "model_calls",
      projectScoped: true,
      primaryKey: "id",
      sharedFieldSemantics: true,
      sharedStorageSemantics: true,
      runtimeEventLinked: true,
      sameTransactionWithRuntimeEvents: true,
      allowedStatuses: [...allowedModelCallStatuses],
      allowedErrorCategories: [...allowedErrorCategories],
      requiredColumns: [...modelCallColumns],
    },
    storagePlan: {
      mock: {
        backend: "runtime_state",
        table: "model_calls",
        sameFieldSemantics: true,
        sameStatusFlow: true,
        sameAuditLink: true,
      },
      sqlite: {
        backend: "sqlite",
        table: "model_calls",
        sameFieldSemantics: true,
        sameStatusFlow: true,
        sameAuditLink: true,
        transaction: {
          required: true,
          rollbackOnAnyFailure: true,
          sameTransactionAsRuntimeEvent: true,
        },
      },
    },
    writeSet: {
      insertModelCall: true,
      updateModelCallStatus: true,
      insertRuntimeEvent: true,
      createApproval: false,
      createTask: false,
      createRunnerJob: false,
      callRealModel: false,
      readRawSecrets: false,
    },
    auditPreview: {
      createdBy,
      relatedApprovalId,
      relatedAgentRunId,
      relatedTaskId,
      runtimeEventId,
      before: {
        status: "blocked",
        redactionApplied: false,
      },
      after: {
        status,
        redactionApplied: recordDraft.redactionApplied === true,
      },
      storesRawPrompt: false,
      storesRawProviderRequest: false,
      storesRawProviderResponse: false,
      storesRawProviderError: false,
      storesRequestHeaders: false,
      storesResponseHeaders: false,
      storesModelReasoning: false,
      storesKeyMaterial: false,
    },
    recordDraft,
    failureGuards: [
      "Mock and SQLite must share the same model_calls field semantics.",
      "requestHash must be derived from a redacted fixed request envelope.",
      "raw prompt and provider payload must never be stored.",
      "runtime_events must mirror every model call status transition.",
      "SQLite model_calls write and audit event must be one transaction.",
      "model_calls must not create tasks, approvals, Runner jobs, or trigger Agents.",
    ],
    realProviderRequestAttempted: false,
    providerResponseStored: false,
    redactionApplied: recordDraft.redactionApplied === true,
    sideEffects: noModelCallWriteDraftSideEffects(),
  };
}

module.exports = {
  buildModelCallWriteDraft,
  noModelCallWriteDraftSideEffects,
};
