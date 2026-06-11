# docs

Formal technical docs for `agent-swarm`.

## Start Here

- [api-draft.md](./api-draft.md): current API contract and stage boundaries.
- [data-model-draft.md](./data-model-draft.md): current entity and table draft.
- [demo-checklist.md](./demo-checklist.md): local demo and verification checklist.
- [runner-safety-acceptance.md](./runner-safety-acceptance.md): Runner safety contract.
- [module-stability-map.md](./module-stability-map.md): protected, contract, and refactorable areas.

## Current Feature Specs

- [agent-config-apply-dry-run-spec.md](./agent-config-apply-dry-run-spec.md)
- [mvp-0.3-project-plan-flow-spec.md](./mvp-0.3-project-plan-flow-spec.md)
- [mvp-0.4-execution-layer-plan.md](./mvp-0.4-execution-layer-plan.md)
- [mvp-0.4-execution-layer-draft.md](./mvp-0.4-execution-layer-draft.md)
- [agent-permission-contract.md](./agent-permission-contract.md)

## Model Gateway Notes

- [relay-provider-info-checklist.md](./relay-provider-info-checklist.md)
- [cheng-relay-manual-ping-spec.md](./cheng-relay-manual-ping-spec.md)
- [deepseek-provider-info-checklist.md](./deepseek-provider-info-checklist.md)

## Platform Notes

- [sqlite-seed-plan.md](./sqlite-seed-plan.md)
- [tech-stack-notes.md](./tech-stack-notes.md)

## Verification

Verification scripts are documented in [../scripts/README.md](../scripts/README.md).

Current stage: MVP-0.4 accepted; Stage 2 real-model admission design is now closed as a disabled-only scaffold with `model_calls` as a draft plus helper / route prep, and the helper-only `model_calls` write / migration draft scaffold is in place. No real Runner, no real model calls, no cloud sync.
