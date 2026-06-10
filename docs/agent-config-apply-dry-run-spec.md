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

## Change Plan Field Whitelist

Current executable helper:

```text
validateAgentConfigChangePlan(...)
```

Current status: helper-only and no-write. It validates future write plans before dry-run/apply-gate can proceed, but it does not modify Agent config.

Allowed fields for the current phase:

- `permissions`
- `model`
- `status`
- `maxSubAgents`
- `canSpawnSubAgents`

Forbidden change-plan content:

- API keys, raw secrets, tokens, authorization headers, provider headers, provider responses, prompts, and local private paths.
- Runner, tool, command, file, Git, network, workspace, parent Agent, and reporting relationship fields.
- Arbitrary complete Agent config JSON from the client.
- `all=true` permission grants.
- Forbidden Agent capabilities such as direct Runner execution, local file writes/deletes, command execution, Git modification, network requests, high-risk/self approval, or raw secret access.

The dry-run response includes `changePlanValidation`. Future real apply must require this result to be `ok=true` before considering any write.

## First Real Apply Gate

The first real Agent config apply implementation must be a separate commit after dry-run is stable.

Current executable gate helper:

```text
buildAgentConfigRealApplyGate(...)
```

Current status: helper-only and feature-disabled. It may prove future preconditions with `preconditionsReady=true`, but it must still return `ok=false`, `gateReady=false`, `canApply=false`, `blockedReasons=["feature_disabled"]`, and all-false side effects.

Required gate inputs before any future real write can be considered:

- Matching dry-run result for the same application, approval, and target Agent.
- Dry-run result is the current feature-disabled preview: `dryRun=true`, `ok=false`, `canApply=false`, and `blockedReasons` includes `feature_disabled`.
- Dry-run result has no validation errors.
- Dry-run result has `changePlanValidation.ok=true`.
- Dry-run result has all side effects false.
- Application exists and remains `pending_apply`.
- Source approval exists, is `approved`, targets `agent_config`, and has no Runner job.
- Target Agent exists.
- Request includes `secondConfirm=true`, non-empty `confirmText`, and non-empty `requestedBy`.
- Request includes `gitCheckpoint.created=true` and a checkpoint commit id.
- Request includes `rollbackPlanAccepted=true`.

## Transaction Plan

Current executable helper:

```text
buildAgentConfigApplyTransactionPlan(...)
```

Current status: helper-only and feature-disabled. A valid plan may return `planReady=true`, but it must still return `ok=false`, `canWrite=false`, `blockedReasons=["feature_disabled"]`, and all-false side effects.

The future real write set must be one transaction:

1. Update `agents` current state.
2. Insert one `agent_config_versions` row.
3. Mark the `agent_config_applications` row as applied.
4. Insert one `runtime_events` audit row.

Transaction guards:

- Application must still be `pending_apply` at write time.
- Source approval must still be `approved`, target `agent_config`, and have no Runner job.
- Target Agent row must exist.
- Target version must equal current Agent config version + 1.
- `agent_id + version` must not already exist in `agent_config_versions`.
- All writes must commit together or roll back together.
- Runtime event insert must be part of the same transaction.
- The transaction plan must not create Runner jobs, execute Runner, call models, read raw secrets, modify files, or modify Git.

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

Current executable helper:

```text
buildAgentConfigRollbackRequest(...)
```

Current status: helper-only and feature-disabled. A valid rollback request may return `requestReady=true`, but it must still return `ok=false`, `canCreateApproval=false`, `blockedReasons=["feature_disabled"]`, draft-only approval/application objects, and all-false side effects.

The helper is covered by:

```text
scripts/verify-agent-config-rollback-request.ps1
```

Rollback must:

- Reference the original application ID.
- Reference the version being rolled back from.
- Reference the version intended to restore.
- Generate a new `agent_config` approval.
- Require human second confirmation.
- Require the original application to be `applied`.
- Require the source approval to be `approved`, target `agent_config`, and have no Runner job.
- Require current and restore versions to belong to the target Agent.
- Require the restore version to be older than the current version.
- Require at least one changed rollback field.
- Pass dry-run validation.
- Insert a new `agent_config_versions` row if applied.
- Keep the old version history intact.

Rollback must not:

- Delete `agent_config_versions`.
- Overwrite `agent_config_versions`.
- Directly edit `agents` without approval.
- Create an approval/application during the request-helper phase while the feature is disabled.
- Run Git or Runner commands.
- Touch `_internal/`, `design/image2/`, `data/local/`, logs, runtime-state, or secrets.

## Acceptance Checklist

Before any real apply endpoint can be enabled:

- Dry-run endpoint exists and is covered by Mock and SQLite verification.
- Helper-level regression covers preconditions that normal HTTP flow cannot create, including unapproved source approval and source approval with a Runner job.
- Real apply gate helper exists and is covered by `scripts/verify-agent-config-apply-gate.ps1`.
- Real apply gate can report `preconditionsReady=true` for valid input while still keeping `gateReady=false`, `canApply=false`, and `feature_disabled`.
- Real apply gate rejects missing requestedBy, missing Git checkpoint, missing rollback acceptance, missing or mismatched dry-run proof, dry-run validation errors, dry-run side effects, and source approval with Runner job.
- Field whitelist helper exists and is covered by `scripts/verify-agent-config-fields.ps1`.
- Dry-run and real apply gate both reject unsupported fields, forbidden fields, forbidden values, `all=true`, and forbidden Agent capabilities.
- Transaction plan helper exists and is covered by `scripts/verify-agent-config-transaction-plan.ps1`.
- Transaction plan stays `canWrite=false` / `feature_disabled` while proving the future write set and rollback-on-failure guards.
- Rollback request helper exists and is covered by `scripts/verify-agent-config-rollback-request.ps1`.
- Rollback request can report `requestReady=true` for valid input while still keeping `ok=false`, `canCreateApproval=false`, and `feature_disabled`.
- Rollback request rejects non-applied original application, unapproved source approval, source approval with Runner job, wrong target service, wrong Agent version ownership, restore version not older than current version, missing confirmation/requester/reason, and no changed fields.
- Rollback request drafts a new approval, new application, and future new version without creating them.
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
