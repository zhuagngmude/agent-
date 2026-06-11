# MVP-0.3 Project Plan Approval Flow

Date: 2026-06-11

Status: implemented for local Mock and SQLite verification.

## Scope

MVP-0.3 adds the first local project planning loop:

1. The user enters a project idea and constraints.
2. The API builds a deterministic local plan draft.
3. The draft becomes a pending `project_plan` approval.
4. After user approval, the plan is split into five queued tasks:
   - frontend
   - backend
   - qa
   - docs
   - reviewer
5. Each task is assigned to the matching Agent.
6. The system creates one read-only Runner request queue record per task.

This is still a local prototype. It does not call a model and it does not execute Runner work.

## API

### `POST /api/projects/:projectId/project-plan-requests`

Creates or updates a pending project plan approval.

Request:

```json
{
  "planId": "local_customer_lead_tracker",
  "idea": "Build a local customer lead tracker",
  "constraints": "Mock/SQLite only; no real Runner; no real model calls",
  "requestedBy": "local_user"
}
```

Response:

```json
{
  "approval": {
    "id": "approval_project_plan_local_customer_lead_tracker",
    "status": "pending",
    "targetService": "project_plan",
    "runnerJobId": "",
    "changeRequest": {
      "type": "project_plan"
    }
  },
  "plan": {
    "id": "local_customer_lead_tracker",
    "status": "draft",
    "generatedBy": "local_deterministic_template",
    "tasks": [],
    "runnerRequests": []
  },
  "sideEffects": {
    "writesProjectFiles": false,
    "modifiesGit": false,
    "executesRunner": false,
    "callsRealModel": false,
    "readsRawSecrets": false,
    "makesNetworkRequests": false,
    "triggersAgents": false,
    "createsTasks": false,
    "createsRunnerJobs": false
  }
}
```

The endpoint may persist the approval in Mock runtime state or SQLite, but it must not create tasks or Runner request records before approval.

### `POST /api/approvals/:approvalId/approve`

When the approval has `targetService=project_plan`, approval creates planned tasks and read-only Runner request queue records.

Response fields specific to project plan approvals:

```json
{
  "id": "approval_project_plan_local_customer_lead_tracker",
  "status": "approved",
  "runnerJobId": "",
  "createdTaskIds": [],
  "createdRunnerJobIds": [],
  "sideEffects": {
    "writesProjectFiles": false,
    "modifiesGit": false,
    "executesRunner": false,
    "callsRealModel": false,
    "readsRawSecrets": false,
    "makesNetworkRequests": false,
    "triggersAgents": false,
    "createsApproval": false,
    "createsTasks": true,
    "createsRunnerJobs": true
  }
}
```

`runnerJobId` must stay empty because this approval creates multiple read-only request records, not a single executable Runner job.

## State Rules

- Draft creation requires a non-empty `idea`.
- Draft generation is deterministic and local (`generatedBy=local_deterministic_template`).
- Draft generation must not trigger Agents, call real models, execute Runner, write project files, make network requests, modify Git, or read raw secrets.
- Draft generation must not create tasks or Runner request records.
- Approval must require second confirmation.
- Approval may create exactly five tasks for frontend, backend, qa, docs, and reviewer.
- Approval may create exactly five read-only Runner request queue records.
- Runner request records must include `runner_request_readonly`.
- Runner request records must reference planned tasks.
- Runner request records must include a safety note stating no command execution, file write, network request, or Git change occurs.
- Re-approving or re-instantiating an already-created plan must not duplicate tasks or Runner request records.

## Storage

Mock mode stores project plan approvals in the temporary runtime state file configured by `AGENT_SWARM_RUNTIME_STATE_FILE` or the default ignored runtime-state file.

SQLite mode stores project plan approvals in `approvals`. After approval, SQLite writes planned tasks into `tasks`, read-only Runner request records into `runner_jobs`, and audit entries into `runtime_events`.

No new table is required for MVP-0.3; the full draft is stored in `approvals.change_request.plan`.

## Verification

Project plan helper contract:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-project-plan-flow.ps1
```

Mock API flow:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
```

SQLite API flow:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
```

These scripts must continue to pass without real model credentials, Runner execution, cloud sync, local project file writes, Git mutation, or raw-secret access.

## Not Enabled

- No real model call.
- No real Runner execution.
- No local project file write.
- No Git checkpoint creation or Git mutation.
- No network request.
- No cloud sync.
- No full permission system.
- No direct Agent self-approval or Agent-triggered execution.
