# AGENTS.md

This file is the short AI/IDE handoff rule sheet for `agent-swarm`.

## Project

- Work in `F:\projects\agent-swarm`.
- Current stage: MVP-0.2, Web App + Mock API + local runtime state.
- Web App first, desktop `.exe` later.
- Mock API first, real database later.
- Read-only safety queues first, real Runner execution later.

## Read First

Before changing code in a fresh session, read:

```text
dev-docs/新窗口交接说明.md
dev-docs/下一步开发路线.md
dev-docs/AI开发维护手册.md
docs/api-draft.md
```

For local demo and regression checks, also read:

```text
docs/demo-checklist.md
scripts/README.md
```

## Do Not Touch Or Commit

- Do not touch or commit `design/image2/` unless the user explicitly asks.
- Do not commit `data/mock/runtime-state.json` or `data/mock/runtime-state.json.tmp`.
- Do not read, modify, or commit `_internal/` unless the user explicitly asks.
- Do not commit logs, secrets, API keys, local credentials, generated builds, or dependency folders.
- Do not add planning, research, retrospective, or handoff documents to the project root; put them in `dev-docs/`.

## Runner Safety

- Runner must not execute commands, write files, delete files, make network requests, or modify Git automatically.
- All local write/execute capability must go through Approval Service.
- High-risk actions require second confirmation and a Git checkpoint.
- `targetService=agent_config` approvals must not directly modify Agent config and must not create Runner jobs unless the user explicitly approves a later feature.

## Development Workflow

- Prefer small, verifiable changes.
- Use `rg` / `rg --files` for search.
- Use `apply_patch` for manual file edits.
- Keep edits scoped to the requested feature or bug.
- Update docs when behavior changes:
  - `dev-docs/下一步开发路线.md` for roadmap/status.
  - `dev-docs/AI开发维护手册.md` for AI-facing maintenance notes.
  - `docs/api-draft.md` for API/state shape changes.
- Put public development plans, research notes, retrospectives, and decision drafts in `dev-docs/`, not in the project root.
- Commit after completing a verifiable feature, bug fix, or important docs update.
- Do not commit every tiny edit; commit meaningful checkpoints.

## Useful Checks

```powershell
node --check apps\web\app.js
node --check services\api\server.js
node --check services\api\mock-data.js
powershell -ExecutionPolicy Bypass -File scripts\verify-mock-flows.ps1
git status --short
```

## Local Run

```powershell
powershell -ExecutionPolicy Bypass -File scripts\start-dev.ps1
```

Mock API default:

```text
http://127.0.0.1:8787
```
