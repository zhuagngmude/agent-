# MVP-0.4 Execution Layer Draft

Date: 2026-06-11

This is the next-stage draft for the true execution layer. It is a spec only. It does not enable real Runner execution, real model calls, cloud sync, or a full permission system.

## Goal

Move from local planning and read-only queueing to guarded, auditable execution:

1. Approvals can carry an executable plan.
2. Runner jobs can be launched only from approved work.
3. Execution can be inspected, paused, failed, and reviewed.
4. Real model calls stay backend-only and fixed-shape.
5. Every high-risk action keeps approval, checkpoint, scope lock, and audit visibility.

## What This Stage Adds

- Real Runner execution behind Approval Service.
- Fixed-shape model request paths for guarded planning / execution helpers.
- Stronger execution records: command preview, file scope, checkpoint, result summary, and runtime events.
- Failure handling that stops on error instead of broadening scope automatically.
- Read-only UI for execution review, not free-form execution control.

## What Must Stay True

- No direct Agent self-approval.
- No execution without approved scope.
- No file writes outside approved scope.
- No Git mutation without checkpoint.
- No raw secret exposure.
- No model prompt/result leakage.
- No cloud sync yet.

## Minimum Acceptance

1. A job can only start from an approved request.
2. High-risk actions require second confirmation.
3. Execution scope is locked before start.
4. Start / finish / fail states are visible in UI and API.
5. Runtime events record before / after state for execution.
6. Failure does not auto-escalate permissions or widen scope.
7. The existing MVP-0.3 project plan flow still works unchanged.

## First Implementation Slice

- execution request schema
- approved job launch gate
- execution status lifecycle
- execution audit event
- read-only execution review UI

## Not Yet Included

- full RBAC / ABAC
- cloud sync
- arbitrary model prompt builder
- arbitrary command runner
- broad filesystem access
- broad Git automation

