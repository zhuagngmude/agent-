const {
  agentPermissionProfiles,
  validateAgentCapabilities,
} = require("./agent-permissions");

const allowedAgentConfigFields = [
  "permissions",
  "model",
  "status",
  "maxSubAgents",
  "canSpawnSubAgents",
];

const forbiddenAgentConfigFieldTokens = [
  "apikey",
  "authorization",
  "command",
  "env",
  "file",
  "git",
  "header",
  "network",
  "parent",
  "prompt",
  "providerresponse",
  "rawsecret",
  "reportsto",
  "runner",
  "secret",
  "token",
  "tool",
  "workspace",
];

const forbiddenAgentConfigValuePatterns = [
  { pattern: /api[_-]?key/i, message: "change plan must not contain API key fields or values." },
  { pattern: /authorization/i, message: "change plan must not contain authorization headers." },
  { pattern: /bearer\s+[a-z0-9._-]+/i, message: "change plan must not contain bearer tokens." },
  { pattern: /raw[_-]?secret/i, message: "change plan must not contain raw secrets." },
  { pattern: /secret/i, message: "change plan must not contain secrets." },
  { pattern: /provider[_-]?response/i, message: "change plan must not contain provider responses." },
  { pattern: /prompt/i, message: "change plan must not contain prompts." },
  { pattern: /[A-Za-z]:[\\/]+Users[\\/]+/i, message: "change plan must not contain local private paths." },
  { pattern: /\/Users\//i, message: "change plan must not contain local private paths." },
];

function noAgentConfigChangePlanSideEffects() {
  return {
    writesAgents: false,
    writesAgentConfigVersions: false,
    writesRuntimeEvents: false,
    writesSqlite: false,
    writesRuntimeState: false,
    createsApprovals: false,
    createsRunnerJobs: false,
    executesRunner: false,
    callsRealModel: false,
    readsRawSecrets: false,
  };
}

function normalizeFieldName(field) {
  return typeof field === "string" ? field.trim() : "";
}

function fieldToken(field) {
  return normalizeFieldName(field).toLowerCase().replace(/[^a-z0-9]/g, "");
}

function splitPermissionText(value) {
  if (Array.isArray(value)) {
    return value.flatMap((item) => splitPermissionText(item));
  }

  if (typeof value !== "string") {
    return [];
  }

  return value
    .split(/[,\n/]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function permissionValidationInputFromValue(value) {
  if (typeof value === "string" && agentPermissionProfiles[value.trim()]) {
    return { profile: value.trim() };
  }

  if (value && typeof value === "object" && !Array.isArray(value) && value.all === true) {
    return { all: true };
  }

  return { capabilities: splitPermissionText(value) };
}

function validateNonPermissionValueSafety(change) {
  const text = JSON.stringify({
    before: change?.before,
    after: change?.after,
  });
  const validationErrors = [];

  for (const item of forbiddenAgentConfigValuePatterns) {
    if (item.pattern.test(text)) {
      validationErrors.push(item.message);
    }
  }

  return validationErrors;
}

function validateFieldValue(change) {
  const field = normalizeFieldName(change?.field);
  const validationErrors = [];
  let permissionValidation = null;

  if (field === "permissions") {
    const input = permissionValidationInputFromValue(change?.after);
    if (!input.profile && input.all !== true && (!Array.isArray(input.capabilities) || input.capabilities.length === 0)) {
      validationErrors.push("permissions change must include a profile or explicit capabilities.");
    } else {
      permissionValidation = validateAgentCapabilities(input);
      if (!permissionValidation.ok) {
        validationErrors.push(...permissionValidation.validationErrors);
      }
    }
  } else {
    validationErrors.push(...validateNonPermissionValueSafety(change));
  }

  if (field === "model" && typeof change?.after !== "string") {
    validationErrors.push("model change must use a string value.");
  }

  if (field === "status" && !["running", "idle", "waiting", "failed", "disabled"].includes(change?.after)) {
    validationErrors.push("status change must use a supported Agent status.");
  }

  if (field === "maxSubAgents") {
    const nextValue = Number(change?.after);
    if (!Number.isInteger(nextValue) || nextValue < 0 || nextValue > 20) {
      validationErrors.push("maxSubAgents change must be an integer between 0 and 20.");
    }
  }

  if (field === "canSpawnSubAgents" && typeof change?.after !== "boolean") {
    validationErrors.push("canSpawnSubAgents change must use a boolean value.");
  }

  return { validationErrors, permissionValidation };
}

function validateAgentConfigChangePlan({ changes } = {}) {
  const validationErrors = [];
  const unsupportedFields = [];
  const forbiddenFields = [];
  const allowedFields = [];
  const permissionValidations = [];

  if (!Array.isArray(changes) || changes.length === 0) {
    validationErrors.push("changes must be a non-empty array.");
  }

  const safeChanges = Array.isArray(changes) ? changes : [];
  safeChanges.forEach((change, index) => {
    if (!change || typeof change !== "object" || Array.isArray(change)) {
      validationErrors.push(`change ${index} must be an object.`);
      return;
    }

    const field = normalizeFieldName(change.field);
    if (!field) {
      validationErrors.push(`change ${index} field is required.`);
      return;
    }

    const token = fieldToken(field);
    const forbidden = forbiddenAgentConfigFieldTokens.some((item) => token.includes(item));
    const allowed = allowedAgentConfigFields.includes(field);

    if (forbidden) {
      forbiddenFields.push(field);
      validationErrors.push(`forbidden Agent config field: ${field}`);
      return;
    }

    if (!allowed) {
      unsupportedFields.push(field);
      validationErrors.push(`unsupported Agent config field: ${field}`);
      return;
    }

    allowedFields.push(field);
    const fieldValueValidation = validateFieldValue(change);
    validationErrors.push(...fieldValueValidation.validationErrors);
    if (fieldValueValidation.permissionValidation) {
      permissionValidations.push(fieldValueValidation.permissionValidation);
    }
  });

  return {
    ok: validationErrors.length === 0,
    allowedFields: [...new Set(allowedFields)],
    unsupportedFields: [...new Set(unsupportedFields)],
    forbiddenFields: [...new Set(forbiddenFields)],
    validationErrors,
    permissionValidations,
    sideEffects: noAgentConfigChangePlanSideEffects(),
  };
}

module.exports = {
  allowedAgentConfigFields,
  forbiddenAgentConfigFieldTokens,
  noAgentConfigChangePlanSideEffects,
  validateAgentConfigChangePlan,
};
