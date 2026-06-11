const SNAPSHOT_FIELDS = ["permissions", "model", "status", "maxSubAgents", "canSpawnSubAgents"];
const FORBIDDEN_FIELD_PATTERNS = [
  /api.?key/i,
  /secret/i,
  /token/i,
  /authorization/i,
  /provider.?header/i,
  /provider.?response/i,
  /prompt/i,
  /runner/i,
  /command/i,
  /file/i,
  /git/i,
  /network/i,
  /workspace/i,
  /parentAgentId/i,
  /reportsToAgentId/i,
];
const FORBIDDEN_VALUE_PATTERNS = [
  /api[_-]?key/i,
  /secret/i,
  /token=/i,
  /authorization:\s*bearer/i,
  /prompt/i,
  /^[a-zA-Z]:[\\/]/,
  /\/Users\//,
  /\\Users\\/,
];

function noAgentConfigVersionHistorySideEffects() {
  return {
    writesAgents: false,
    writesAgentConfigVersions: false,
    writesAgentConfigApplications: false,
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

function parseJsonLike(value, fallback) {
  if (value && typeof value === "object") return value;
  if (typeof value !== "string" || value.trim() === "") return fallback;
  try {
    return JSON.parse(value);
  } catch {
    return fallback;
  }
}

function versionNumber(row = {}) {
  const parsed = Number(row.version || row.versionNumber || "");
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0;
}

function rowAgentId(row = {}) {
  return row.agentId || row.agent_id || "";
}

function hasForbiddenValue(value) {
  if (value === null || value === undefined) return false;
  if (Array.isArray(value)) return value.some(hasForbiddenValue);
  if (typeof value === "object") return Object.values(value).some(hasForbiddenValue);
  return FORBIDDEN_VALUE_PATTERNS.some((pattern) => pattern.test(String(value)));
}

function safeSnapshotValue(value) {
  return hasForbiddenValue(value) ? "[redacted_forbidden_value]" : value;
}

function normalizeChange(change, index) {
  const validationErrors = [];
  const source = Array.isArray(change)
    ? { field: change[0], before: change[1], after: change[2] }
    : change;

  if (!source || typeof source !== "object") {
    return {
      change: null,
      validationErrors: [`change ${index} must be an object or tuple.`],
    };
  }

  const field = typeof source.field === "string" ? source.field : "";
  if (!SNAPSHOT_FIELDS.includes(field) || FORBIDDEN_FIELD_PATTERNS.some((pattern) => pattern.test(field))) {
    return {
      change: null,
      validationErrors: [`forbidden Agent config change field: ${field || "unknown"}`],
    };
  }

  if (hasForbiddenValue(source.before) || hasForbiddenValue(source.after)) {
    validationErrors.push(`forbidden Agent config change value in field: ${field}`);
  }

  return {
    change: {
      field,
      before: safeSnapshotValue(source.before),
      after: safeSnapshotValue(source.after),
    },
    validationErrors,
  };
}

function normalizeChanges(changes) {
  if (!Array.isArray(changes)) {
    return { changes: [], validationErrors: [] };
  }

  const normalized = [];
  const validationErrors = [];
  changes.forEach((change, index) => {
    const result = normalizeChange(change, index);
    validationErrors.push(...result.validationErrors);
    if (result.change) {
      normalized.push(result.change);
    }
  });

  return { changes: normalized, validationErrors };
}

function normalizeSnapshot(snapshot = {}) {
  const normalized = {};
  const validationErrors = [];

  for (const key of Object.keys(snapshot || {})) {
    if (FORBIDDEN_FIELD_PATTERNS.some((pattern) => pattern.test(key))) {
      validationErrors.push(`forbidden Agent config snapshot field: ${key}`);
    }
  }

  for (const field of SNAPSHOT_FIELDS) {
    if (Object.prototype.hasOwnProperty.call(snapshot, field)) {
      if (hasForbiddenValue(snapshot[field])) {
        validationErrors.push(`forbidden Agent config snapshot value in field: ${field}`);
      }
      normalized[field] = safeSnapshotValue(snapshot[field]);
    }
  }

  return { snapshot: normalized, validationErrors };
}

function normalizeVersionRow(row = {}) {
  const rawSnapshot = parseJsonLike(row.configSnapshot || row.config_snapshot, {});
  const { snapshot, validationErrors } = normalizeSnapshot(rawSnapshot);
  const changesResult = normalizeChanges(parseJsonLike(row.changes, []));
  return {
    id: row.id || "",
    projectId: row.projectId || row.project_id || "",
    agentId: rowAgentId(row),
    version: versionNumber(row),
    approvalId: row.approvalId || row.approval_id || "",
    applicationId: row.applicationId || row.application_id || "",
    configSnapshot: snapshot,
    changes: changesResult.changes,
    appliedBy: row.appliedBy || row.applied_by || "",
    appliedAt: row.appliedAt || row.applied_at || "",
    createdAt: row.createdAt || row.created_at || "",
    validationErrors: [...validationErrors, ...changesResult.validationErrors],
  };
}

function buildAgentConfigVersionHistory({ agent, versions = [], restoreVersion } = {}) {
  const validationErrors = [];

  if (!agent) {
    validationErrors.push("target agent is required.");
  }

  if (!Array.isArray(versions)) {
    validationErrors.push("versions must be an array.");
  }

  const agentId = agent?.id || "";
  const normalizedVersions = (Array.isArray(versions) ? versions : [])
    .map(normalizeVersionRow)
    .filter((version) => version.agentId === agentId)
    .sort((a, b) => b.version - a.version);

  for (const version of normalizedVersions) {
    if (version.version === 0) {
      validationErrors.push(`version number is required for ${version.id || "unknown version"}.`);
    }
    validationErrors.push(...version.validationErrors);
  }

  const seen = new Set();
  for (const version of normalizedVersions) {
    if (seen.has(version.version)) {
      validationErrors.push(`duplicate Agent config version: ${version.version}`);
    }
    seen.add(version.version);
  }

  const currentVersion = normalizedVersions[0] || null;
  const requestedRestoreNumber = Number(restoreVersion || "");
  const restoreCandidates = currentVersion
    ? normalizedVersions.filter((version) => version.version < currentVersion.version)
    : [];
  const restoreVersionRow = Number.isInteger(requestedRestoreNumber) && requestedRestoreNumber > 0
    ? restoreCandidates.find((version) => version.version === requestedRestoreNumber) || null
    : restoreCandidates[0] || null;

  if (Array.isArray(versions) && versions.length > 0 && normalizedVersions.length === 0 && agent) {
    validationErrors.push("no versions belong to target Agent.");
  }
  if (normalizedVersions.length > 0 && !currentVersion) {
    validationErrors.push("current version is required.");
  }
  if (Number.isInteger(requestedRestoreNumber) && requestedRestoreNumber > 0 && !restoreVersionRow) {
    validationErrors.push("restore version must exist and be older than current version.");
  }

  return {
    ok: validationErrors.length === 0,
    versionHistory: true,
    readOnly: true,
    canWrite: false,
    agentId,
    validationErrors,
    currentVersion,
    restoreVersion: restoreVersionRow,
    restoreCandidates,
    versions: normalizedVersions,
    rollbackSourceReady: Boolean(currentVersion && restoreVersionRow && validationErrors.length === 0),
    sideEffects: noAgentConfigVersionHistorySideEffects(),
  };
}

module.exports = {
  buildAgentConfigVersionHistory,
  noAgentConfigVersionHistorySideEffects,
};
