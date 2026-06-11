# AGENTS.md

This is the short operating guide for `agent-swarm`.

## Current Stage

- Work in `F:\projects\agent-swarm`.
- Current stage: MVP-0.4 accepted; Stage 2 real-model admission design has started.
- Web App first, desktop later.
- Mock / SQLite first, real database later.
- Read-only safety queues first, real Runner later.
- MVP-0.3 project plan approvals and MVP-0.4 execution request lifecycle / runtime event auditing are already implemented and accepted.
- Stage 2 currently only has a `model_calls` data-model draft and admission boundaries; no table, route, provider SDK, or real provider request is enabled.

## Read First

Before changing code in a fresh session, read:

```text
docs/README.md
dev-docs/README.md
dev-docs/新窗口交接说明.md
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
dev-docs/应用真正可用落地计划.md
dev-docs/真实模型接入准入规格.md
```

For local demo work, also read:

```text
docs/demo-checklist.md
scripts/README.md
```

## Do Not Touch Or Commit

- `design/image2/`
- `data/mock/runtime-state.json`
- `data/local/`
- `logs/`
- `.playwright-cli/`
- `_internal/`

Do not add public plans or handoff docs to the project root; keep them in `dev-docs/`.

## Runner Safety

- Runner must not execute commands, write files, delete files, make network requests, or modify Git automatically.
- MVP-0.3 project plan approvals may create read-only Runner request queue records only.
- Real model calls remain disabled until the real-model admission spec, feature flag, redaction, logging, cost tracking, and Model Gateway verification are updated in a separate implementation batch.
- `targetService=agent_config` approvals still must not directly modify Agent config or create Runner jobs.
- High-risk actions require second confirmation and a Git checkpoint.

## Development Workflow

- Prefer small, verifiable changes.
- Use `rg` / `rg --files` for search.
- Use `apply_patch` for manual edits.
- Update docs when behavior changes:
  - `docs/api-draft.md` for API/state shape changes
  - `docs/data-model-draft.md` for data model changes
  - `docs/demo-checklist.md` for demo/verification changes
  - `dev-docs/下一步开发路线.md` for roadmap/status changes
  - `dev-docs/AI开发维护手册.md` for AI-facing maintenance notes

## Useful Checks

```powershell
node --check apps\web\app.js
node --check services\api\server.js
node --check services\api\mock-data.js
powershell -ExecutionPolicy Bypass -File scripts\verify-project-plan-flow.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
git status --short
```

## Local Run

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-local.ps1
powershell -ExecutionPolicy Bypass -File scripts\status-local.ps1
powershell -ExecutionPolicy Bypass -File scripts\stop-local.ps1
```

Mock default:

```text
http://127.0.0.1:8787
```
