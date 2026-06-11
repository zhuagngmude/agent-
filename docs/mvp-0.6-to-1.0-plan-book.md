# MVP-0.6 to MVP-1.0 Plan Book

Date: 2026-06-11

This is the working plan book for the skeleton roadmap. Use it to turn the roadmap into a sequence of small commits and verifiable checks.

## How To Use

For each stage:

1. Update the stage doc first.
2. Implement the smallest slice.
3. Run the stage verification.
4. Commit before moving on.

## MVP-0.6 Plan

Focus:

- Freeze the model request shape.
- Keep provider metadata backend-only.
- Keep dry-run and connectivity-test blocked by default.

Deliverables:

- One canonical request schema.
- One canonical provider metadata table.
- One canonical redaction rule set.

Verification:

- `node --check services/api/model-gateway.js`
- `node --check services/api/model-gateway-adapters.js`
- `powershell -ExecutionPolicy Bypass -File scripts/verify-model-gateway.ps1`

## MVP-0.7 Plan

Focus:

- Add approval-backed model request routing.
- Keep model requests auditable and reviewable.
- Keep secrets server-side.

Deliverables:

- Approval-linked model request records.
- Read-only review surfaces.
- Audit events for every state change.

Verification:

- Model request approval flow checks.
- Audit event checks.
- UI smoke for read-only review surfaces.

## MVP-0.8 Plan

Focus:

- Stabilize persistence and identity boundaries.
- Keep state consistent between Mock and SQLite.
- Prevent schema drift from becoming product drift.

Deliverables:

- Stable persistence contract.
- Clear identity and project scoping.
- Clean migration and seed story.

Verification:

- Mock and SQLite flow checks.
- Schema/read consistency checks.
- Diff and encoding checks.

## MVP-0.9 Plan

Focus:

- Introduce guarded Runner execution.
- Enforce scope lock, checkpoint, and second confirmation.
- Keep failure paths non-expansive.

Deliverables:

- Approved execution launch gate.
- Scope-limited Runner records.
- Execution audit and review UI.

Verification:

- Runner safety acceptance checks.
- Execution lifecycle checks.
- Runtime event audit checks.

## MVP-1.0 Plan

Focus:

- Make the guarded loop usable end-to-end.
- Tighten docs, UI, and verification until the workflow is repeatable.
- Keep cloud sync and full permissions out until the local loop is stable.

Deliverables:

- A repeatable approve -> execute -> review workflow.
- A stable set of docs and smoke checks.
- A clear private-beta boundary.

Verification:

- End-to-end smoke run.
- Local UI smoke.
- Stage-specific regression suite.

