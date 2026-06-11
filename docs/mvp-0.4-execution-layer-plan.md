# MVP-0.4 Execution Layer Plan

Date: 2026-06-11

This is the safest execution plan for the next stage. It is a plan only. It does not enable real Runner execution, real model calls, cloud sync, or a full permission system.

## Stage Table

| Step | Scope | Deliverable | Exit Check | Must Stay Off |
| --- | --- | --- | --- | --- |
| 1 | Execution request shape | Define the smallest approved execution request and review payload | Request shape is fixed and documented | Free-form commands, free-form prompts |
| 2 | Launch gate | Add a launch gate that only accepts approved, scoped work | No job starts without approval and scope lock | Direct Runner start, direct Agent self-approval |
| 3 | Lifecycle | Track start / finish / fail / pause / review states | UI and API show the same lifecycle | Hidden execution state |
| 4 | Audit | Record before / after state for each execution event | Runtime events are emitted for every state change | Silent state changes |
| 5 | Review UI | Show read-only execution review and result summary | Review UI is read-only and consistent with API | Free-form execution control |
| 6 | Safety checks | Keep checkpoint, second confirmation, and scope limits | High-risk paths stay blocked unless explicitly approved | Broad file, Git, or network access |

## Implementation Order

1. Lock the request schema.
2. Wire the approval-to-execution gate.
3. Add lifecycle status tracking.
4. Add runtime audit events.
5. Build read-only review surfaces.
6. Verify that the existing MVP-0.3 project plan flow still behaves unchanged.

## Safety Rules

- No execution without an approved request.
- No execution without scope lock.
- No Git mutation without checkpoint.
- No raw secret exposure.
- No model prompt/result leakage.
- No cloud sync.
- No permission broadening by failure.

## First Verification Set

- `node --check apps/web/app.js`
- `node --check services/api/server.js`
- `node --check services/api/mock-data.js`
- `powershell -ExecutionPolicy Bypass -File scripts/verify-project-plan-flow.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/verify-mock-flows.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/verify-sqlite-flows.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/verify-local-ui.ps1`
- `git diff --check`

## Not Yet In Scope

- full RBAC / ABAC
- cloud sync
- arbitrary command execution
- arbitrary model calling
- broad filesystem access
- broad Git automation

