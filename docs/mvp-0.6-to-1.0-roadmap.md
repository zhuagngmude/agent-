# MVP-0.6 to MVP-1.0 Roadmap

Date: 2026-06-11

This is the stage skeleton from the current disabled Model Gateway boundary toward the first usable private beta. It is a roadmap only. It does not enable real provider calls, real Runner execution, cloud sync, or a full permission system.

## Guiding Rule

Each stage must add one meaningful boundary or one meaningful safety proof, not a bundle of unrelated features.

## Stage Skeleton

| Stage | Goal | Exit Check | Must Stay Off |
| --- | --- | --- | --- |
| MVP-0.6 | Formalize the model request contract | The request shape, provider metadata, and redaction rules are stable and documented | Real provider SDKs, client API keys, free-form prompts |
| MVP-0.7 | Add approval-backed model request handling | Model requests can be approved, reviewed, and audited without exposing secrets | Direct provider calls from UI, unguarded request submission |
| MVP-0.8 | Add backend persistence and identity baseline | State is persisted with clear project boundaries and stable audit records | Cloud sync, full RBAC/ABAC, ad hoc schema drift |
| MVP-0.9 | Add guarded Runner execution | Approved work can reach a constrained Runner path with scope locks, checkpoints, and audit events | Arbitrary command execution, broad filesystem access, silent retries |
| MVP-1.0 | First usable private beta | The product can complete a guarded approve -> execute -> review loop with stable docs and verification | Default-open real execution, cloud sync, secret leakage |

## What the Skeleton Means

- MVP-0.6 is still about the shape of requests, not power.
- MVP-0.7 is still about approval and audit, not broad execution.
- MVP-0.8 is still about data and identity discipline, not multi-tenant complexity.
- MVP-0.9 is about controlled execution, not general automation.
- MVP-1.0 is the first point where the loop is usable end-to-end under the documented safety rules.

## Suggested Order

1. Lock the model request contract.
2. Add approval and audit around model requests.
3. Stabilize persistence and identity boundaries.
4. Introduce guarded Runner execution.
5. Finish the private beta acceptance loop.

## Shared Non-Goals

- Real provider SDKs before the contract is stable.
- Real Runner execution before checkpoint, scope, and audit rules are in place.
- Cloud sync before the local single-project loop is reliable.
- Full RBAC/ABAC before the permission contract and state model stop moving.

