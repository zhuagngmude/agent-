# Agent Permission Contract

Date: 2026-06-10

Stage: MVP-0.2 permission design contract. This document is a specification only. It does not enable real Runner execution, real model calls, cloud sync, full RBAC/ABAC, or secret access.

Current code support is limited to `services/api/agent-permissions.js` and `scripts/verify-agent-permissions.ps1`. They validate mock profile expansion and forbidden capability checks only; they do not enforce runtime authorization or grant any execution capability.

## Purpose

The product may later support an architect Agent or all Agents with broad management permissions. To avoid ambiguity, "full permission" must be split into explicit capability groups.

The core rule:

```text
Agents may receive broad planning, orchestration, and request permissions.
Agents must not receive self-approval, raw-secret access, or direct local execution permissions.
```

This keeps future high-trust Agent modes compatible with the current safety model.

## Permission Layers

### 1. Planning Permissions

These permissions let an Agent reason about the project and produce plans.

```text
canViewProject
canReadKnowledge
canPlanArchitecture
canDraftTasks
canDraftWorkflow
canReviewArchitecture
canProposeModelUse
```

Allowed behavior:

- Read project metadata and non-secret knowledge.
- Draft architecture plans, task breakdowns, workflow changes, and review notes.
- Recommend model/provider choices without using keys or making provider calls.

Not allowed behavior:

- Write files directly.
- Execute commands.
- Modify Git.
- Call real providers.
- Approve its own plan.

### 2. Orchestration Permissions

These permissions let an Agent coordinate other Agents inside product state.

```text
canCreateTasks
canAssignTasks
canAssignAgents
canSpawnSubAgents
canSetTaskPriority
canRequestAgentConfigChange
```

Allowed behavior:

- Create or update task proposals.
- Assign tasks to Agents.
- Spawn controlled sub Agents within configured limits.
- Request Agent configuration changes.

Not allowed behavior:

- Bypass Agent spawn depth / count limits.
- Grant itself or another Agent execution, approval, or secret access directly.
- Apply Agent configuration changes without the Agent config approval/application flow.

### 3. Request Permissions

These permissions let an Agent ask the system to perform high-impact work through guarded services.

```text
canRequestExecution
canRequestFileWrite
canRequestCommand
canRequestNetwork
canRequestGitOperation
canRequestModelConnectivity
canRequestModelCall
```

Allowed behavior:

- Create Approval Requests for Runner work.
- Create manual Model Gateway requests when the relevant feature contract exists.
- Include a plan, affected files, reason, risk level, and required confirmation fields.

Not allowed behavior:

- Execute the requested action directly.
- Turn a request into an approved Runner job without Approval Service.
- Trigger real provider calls from page load, Agent background jobs, or Runner jobs.

### 4. Approval Permissions

Approval permissions decide whether high-impact work may proceed.

```text
canReviewApproval
canRecommendApproval
canApproveLowRisk
canApproveHighRisk
```

Current MVP-0.2 rule:

```text
No Agent may approve high-risk execution, approve secret access, approve its own request, or approve work that produces Runner execution.
```

Future possibility:

- An Agent may recommend approval or flag risk.
- Low-risk non-execution approvals may be considered later.
- Human confirmation remains required for high-risk operations, raw secret use, Runner execution, Git operations, and real provider requests.

### 5. Execution Permissions

Execution permissions belong to guarded services, not normal Agents.

```text
canExecuteRunnerJob
canWriteFiles
canDeleteFiles
canExecuteCommands
canModifyGit
canMakeNetworkRequests
```

Current rule:

- These permissions are not granted directly to Agents.
- Runner may only execute approved Runner jobs.
- Runner cannot approve its own jobs.
- Runner cannot expand file scope, command scope, network scope, or Git scope beyond the approved plan.

### 6. Secret Permissions

Secret permissions are the most restrictive.

```text
canReferenceSecretPresence
canRequestSecretUse
canAccessRawSecrets
```

Allowed for Agents:

- `canReferenceSecretPresence`: see non-sensitive booleans such as "env var configured".
- `canRequestSecretUse`: ask backend services to use a server-side secret through a fixed, reviewed workflow.

Not allowed for Agents:

- `canAccessRawSecrets`.
- Seeing API key values, key suffixes, masked fragments, authorization headers, provider request bodies, provider response bodies, token usage, cost, or raw provider errors during connectivity checks.

## Permission Profiles

Profiles are named bundles of capabilities. They must not hide forbidden capabilities.

### `architect_admin`

Purpose: highest planning and orchestration authority.

Allowed:

```text
canViewProject
canReadKnowledge
canPlanArchitecture
canDraftTasks
canDraftWorkflow
canReviewArchitecture
canCreateTasks
canAssignTasks
canAssignAgents
canSpawnSubAgents
canSetTaskPriority
canRequestAgentConfigChange
canRequestExecution
canRequestModelConnectivity
canRequestModelCall
canReferenceSecretPresence
canRequestSecretUse
```

Still forbidden:

```text
canApproveHighRisk
canApproveOwnRequest
canExecuteRunnerJob
canWriteFiles
canDeleteFiles
canExecuteCommands
canModifyGit
canMakeNetworkRequests
canAccessRawSecrets
```

### `executor_agent`

Purpose: produce implementation plans and request guarded execution.

Allowed:

```text
canViewProject
canReadKnowledge
canDraftTasks
canRequestExecution
canRequestFileWrite
canRequestCommand
canRequestGitOperation
```

Still forbidden:

```text
canApproveHighRisk
canApproveOwnRequest
canExecuteRunnerJob
canAccessRawSecrets
```

### `reviewer_agent`

Purpose: inspect plans, diffs, risk, and acceptance results.

Allowed:

```text
canViewProject
canReadKnowledge
canReviewArchitecture
canReviewApproval
canRecommendApproval
```

Still forbidden:

```text
canApproveHighRisk
canExecuteRunnerJob
canAccessRawSecrets
```

### `all_agents_full_management`

This name may be used only if "full" means broad planning/orchestration/request permissions.

It must not include:

```text
canApproveHighRisk
canApproveOwnRequest
canExecuteRunnerJob
canWriteFiles
canDeleteFiles
canExecuteCommands
canModifyGit
canMakeNetworkRequests
canAccessRawSecrets
```

## Non-Bypass Invariants

These invariants must hold even when an Agent appears to have "full permissions":

1. Approval Service remains the only authorization path for high-risk execution.
2. Runner only executes approved Runner jobs.
3. Agents cannot approve their own requests.
4. High-risk operations require human confirmation.
5. Git checkpoint is required before real Runner writes or Git operations.
6. File scope, command scope, network scope, and Git scope are locked before execution.
7. Secrets stay server-side and are never returned to Agents or frontend code.
8. Model Gateway real provider requests stay backend-only, fixed, manually triggered, and disabled by default until a separate feature flag contract changes that.
9. Agent configuration changes require Approval Service and application flow.
10. Runtime events must record before/after state when real state changes are implemented.

## Why This Avoids Future Bugs

The word "all permissions" is dangerous because it mixes unrelated concepts:

```text
planning
orchestration
requesting
approval
execution
secret access
```

If the product stores these as one boolean, later code may accidentally treat an architect Agent as able to approve, execute, or read secrets. By storing separate capabilities and enforcing non-bypass invariants, the product can safely give an architect Agent broad control without breaking Runner, Model Gateway, or secret boundaries.

## MVP-0.2 Status

Current implementation status:

- This document is a contract plus a local mock validation helper.
- `services/api/agent-permissions.js` defines profile expansion and validation helpers only.
- `scripts/verify-agent-permissions.ps1` verifies safe profile expansion, `all=true` rejection, forbidden capability rejection, unknown capability rejection, and all-false side effects.
- Existing Agent `permissions` arrays remain simple mock/display data.
- Agent config changes still go through approval/application mock state only.
- Runner execution remains disabled.
- Real model calls remain disabled.
- Secret access remains disabled.
- No API route, UI flow, Agent runtime, Runner runtime, Model Gateway path, SQLite mapper, or secret service consumes this helper yet.

Future implementation must add tests before any permission profile can change runtime behavior.
