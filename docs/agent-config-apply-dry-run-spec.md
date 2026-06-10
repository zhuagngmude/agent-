# Agent Config Apply Dry-Run And Rollback Spec

Date: 2026-06-10

Status: disabled dry-run endpoint implemented. This document does not enable real Agent config writes, Runner execution, model calls, cloud sync, or a broad runtime permission system.

## Purpose

Before Agent config changes can be written to the real `agents` current state, the project must add a dry-run gate that proves the approved change is valid, reversible, auditable, and still inside the current safety boundary.

The current `POST /api/agent-config-applications/:applicationId/apply` endpoint remains Mock state transition only. It marks an application as `applied`, records confirmation metadata, and must not change Agent config.

## Current Boundary

Currently allowed:

- Create `agent_config` approvals through the Agent change-request flow.
- Validate permission profile changes before approval creation.
- Approve `agent_config` approvals without creating Runner jobs.
- Create `agent_config_applications` records in `pending_apply`.
- Mock-apply or cancel those records for local state-flow verification.

Currently forbidden:

- Mutating `agents` current config.
- Writing `agent_config_versions`.
- Creating Runner jobs for `agent_config` approvals.
- Letting Agents self-approve or self-apply config.
- Calling real model providers.
- Reading raw secrets or API keys.
- Applying rollback without a new approval.

## Disabled Dry-Run Endpoint

Current endpoint:

```text
POST /api/agent-config-applications/:applicationId/dry-run
```

Purpose: validate a pending Agent config application and preview the exact write plan without changing state. MVP-0.2 always keeps the endpoint blocked with `blockedReasons=["feature_disabled"]`.

Required request shape:

```json
{
  "secondConfirm": true,
  "confirmText": "I understand this is a dry-run only.",
  "requestedBy": "local_user"
}
```

Required response shape:

```json
{
  "ok": false,
  "dryRun": true,
  "applicationId": "agent_config_application_approval_agent_agent_reviewer_permission",
  "approvalId": "approval_agent_agent_reviewer_permission",
  "agentId": "agent_reviewer",
  "canApply": false,
  "blockedReasons": ["feature_disabled"],
  "writePlan": {
    "wouldUpdateAgent": false,
    "wouldCreateVersion": false,
    "wouldWriteRuntimeEvent": false,
    "targetVersion": 2,
    "changedFields": ["permissions"]
  },
  "rollbackPlan": {
    "rollbackRequiresNewApproval": true,
    "wouldRestoreVersion": 1,
    "rollbackAction": "create_new_agent_config_application"
  },
  "sideEffects": {
    "writesAgents": false,
    "writesAgentConfigVersions": false,
    "writesRuntimeEvents": false,
    "writesSqlite": false,
    "writesRuntimeState": false,
    "createsApprovals": false,
    "createsRunnerJobs": false,
    "executesRunner": false,
    "callsRealModel": false,
    "readsRawSecrets": false
  }
}
```

MVP-0.2 returns blocked / feature-disabled behavior. The dry-run computes a preview from already-approved local state, but it keeps all side effects false until a later implementation commit explicitly changes the feature gate.

## Dry-Run Validation Rules

The dry-run must reject or block when any condition is false:

- Application exists.
- Application status is `pending_apply`.
- Source approval exists.
- Source approval status is `approved`.
- Source approval `targetService` is `agent_config`.
- Source approval has empty `runnerJobId`.
- Request includes `secondConfirm=true`.
- Request includes non-empty `confirmText`.
- Target Agent exists.
- Change fields are supported for the current phase.
- Permission changes pass `services/api/agent-permissions.js` validation.
- The change plan does not contain unknown fields.
- The change plan does not contain raw secrets, API keys, provider headers, prompts, provider responses, local private paths, or unchecked tool/Runner fields.
- The operation does not require Runner execution.

## First Real Apply Gate

The first real Agent config apply implementation must be a separate commit after dry-run is stable.

It must:

- Add an explicit feature flag separate from Model Gateway and Runner flags.
- Keep `targetService=agent_config` isolated from Runner job creation.
- Run the same dry-run validation before every real apply.
- Update `agents` current config and insert `agent_config_versions` in one transaction.
- Write a runtime event or equivalent audit record.
- Store before/after field summaries, not secrets.
- Reject partial writes.
- Preserve a previous config version for rollback planning.
- Keep UI copy clear that this is Agent config apply, not Runner execution.

It must not:

- Execute Runner.
- Modify files or Git.
- Call model providers.
- Read raw API keys.
- Allow Agent self-approval or self-apply.
- Accept arbitrary config JSON from the client.
- Allow broad `all=true` permission grants.

## Rollback Rules

Rollback is a new approved config change, not a database delete and not a direct revert.

Rollback must:

- Reference the original application ID.
- Reference the version being rolled back from.
- Reference the version intended to restore.
- Generate a new `agent_config` approval.
- Require human second confirmation.
- Pass dry-run validation.
- Insert a new `agent_config_versions` row if applied.
- Keep the old version history intact.

Rollback must not:

- Delete `agent_config_versions`.
- Directly edit `agents` without approval.
- Run Git or Runner commands.
- Touch `_internal/`, `design/image2/`, `data/local/`, logs, runtime-state, or secrets.

## Acceptance Checklist

Before any real apply endpoint can be enabled:

- Dry-run endpoint exists and is covered by Mock and SQLite verification.
- Dry-run blocked state keeps all side effects false.
- Invalid application ID returns a safe error.
- Non-`pending_apply` application is rejected.
- Missing source approval is rejected.
- Non-`approved` source approval is rejected.
- Source approval with `runnerJobId` is rejected.
- Unsupported change fields are rejected.
- Invalid permission capabilities are rejected.
- Dry-run proves target Agent config remains unchanged.
- First real apply has transaction coverage for `agents` and `agent_config_versions`.
- Rollback creates a new approval instead of mutating history.

Until every item passes, the project remains in Mock / dry-run only mode.
