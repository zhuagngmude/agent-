# Module Stability Map

Date: 2026-06-10

Stage: MVP-0.2 maintenance map. This document explains which parts of the repository are stable anchors, which parts may be refactored, and which parts are runtime/local artifacts. It is a guide for future cleanup, deletion, and extension work.

This document is documentation only. It does not enable real Runner execution, real model calls, cloud sync, full permissions, or secret access.

## Stability Levels

Use these labels before deleting, moving, or rewriting files.

```text
P0 Anchor       Do not delete or casually rewrite. These hold product, safety, or verification boundaries.
P1 Contract     Can evolve, but only with docs and matching verification updates.
P2 Refactorable Can be reorganized if behavior and tests remain stable.
P3 Runtime      Local/generated state. Do not commit. Usually safe to recreate through scripts.
P4 Protected    Do not read/touch/commit unless the user explicitly asks.
```

## P0 Anchors

These files and modules are the current stability backbone.

```text
AGENTS.md
README.md
dev-docs/AI开发维护手册.md
dev-docs/下一步开发路线.md
dev-docs/新窗口交接说明.md
docs/api-draft.md
docs/data-model-draft.md
docs/sqlite-seed-plan.md
docs/runner-safety-acceptance.md
docs/agent-permission-contract.md
docs/agent-config-apply-dry-run-spec.md
docs/module-stability-map.md
services/api/server.js
services/api/model-gateway.js
services/api/model-gateway-adapters.js
services/api/agent-permissions.js
services/api/mock-data.js
services/api/db/
scripts/verify-model-gateway.ps1
scripts/verify-agent-permissions.ps1
scripts/verify-local-ui.ps1
scripts/verify-sqlite-flows.ps1
scripts/start-local.ps1
scripts/status-local.ps1
scripts/stop-local.ps1
data/migrations/
data/seed/
apps/web/index.html
apps/web/app.js
apps/web/styles.css
apps/web/data.js
```

Rules:

- Do not delete these as "cleanup".
- Do not rename or move them without updating docs and verification scripts in the same commit.
- If their behavior changes, update at least the nearest contract document and run the relevant acceptance script.

## P1 Contracts

These docs define future behavior and may evolve as the product direction changes.

```text
docs/cheng-relay-manual-ping-spec.md
docs/relay-provider-info-checklist.md
docs/deepseek-provider-info-checklist.md
docs/demo-checklist.md
docs/tech-stack-notes.md
docs/runner-safety-acceptance.md
docs/agent-permission-contract.md
```

Rules:

- These may be edited when the product decision changes.
- Keep them consistent with API, script, and handoff docs.
- Do not weaken safety language while implementing a feature unless the feature intentionally changes the boundary and adds verification.

## P2 Refactorable Areas

These areas may be reorganized when there is a concrete benefit.

```text
apps/web/
services/api/
scripts/
docs/
dev-docs/
packages/
services/runner/
services/worker/
apps/desktop/
```

Rules:

- Refactor only one concern per commit.
- Preserve current API response shapes unless the API draft is updated.
- Preserve current UI smoke coverage unless `verify-local-ui.ps1` is updated.
- Preserve Model Gateway blocked/no-side-effect coverage unless `verify-model-gateway.ps1` is updated.
- Preserve Agent permission profile boundary coverage unless `verify-agent-permissions.ps1` is updated.
- Empty placeholder folders such as `apps/desktop/`, `services/runner/`, `services/worker/`, and `packages/` can remain as architecture markers. Delete them only after updating the project skeleton docs.

## P3 Runtime And Generated State

These are local state, logs, process files, generated browser tooling, or local databases. They must not be committed.

```text
data/local/
data/mock/runtime-state.json
data/mock/runtime-state.json.tmp
logs/
.playwright-cli/
*.sqlite
*.sqlite-shm
*.sqlite-wal
node_modules/
dist/
build/
coverage/
```

Rules:

- Do not commit these.
- Do not use these as source-of-truth docs.
- It is usually safe to recreate SQLite state through `scripts/seed-sqlite.ps1` or `scripts/verify-sqlite-flows.ps1`.
- Do not manually delete runtime state during a feature task unless that reset is the task and the user expects it.

## P4 Protected Areas

These are explicitly protected in current handoff rules.

```text
design/image2/
_internal/
data/local/
logs/
.playwright-cli/
data/mock/runtime-state.json
```

Rules:

- Do not read, modify, delete, or commit `design/image2/` unless the user explicitly asks.
- Do not read, modify, delete, or commit `_internal/` unless the user explicitly asks.
- Do not commit `data/local/`, logs, `.playwright-cli/`, or runtime state.
- If a future task appears to require one of these, stop and clarify the exact user intent before touching it.

## Safe Additions

Generally safe additions:

- New docs under `docs/` for API, architecture, safety, verification, or provider specs.
- New development handoff or roadmap notes under `dev-docs/`.
- New verification scripts under `scripts/`, if they are no-side-effect or clearly isolate their runtime state.
- New API helpers under `services/api/`, if route behavior and safety contracts remain covered.
- New web UI code under `apps/web/`, if it passes UI smoke checks.

Before adding:

- Choose the smallest module that owns the behavior.
- Update the nearest docs when behavior or contract changes.
- Add or update verification when a safety boundary changes.

## Unsafe Additions

Do not add these without a separate design and verification batch:

- Real provider SDK imports.
- Real OpenAI / Anthropic / Gemini / DeepSeek / cheng.pink network requests.
- Frontend API key inputs wired to Model Gateway.
- Client-provided base URLs, headers, prompts, stream options, or provider request bodies for connectivity tests.
- Runner command execution, file writes, file deletion, Git mutation, or network requests.
- A single `all=true` permission flag that implies approval, execution, or raw secret access.
- Cloud sync.
- Full RBAC/ABAC enforcement.

## Deletion Rules

Safe deletion candidates:

- Dead docs that are superseded and explicitly linked to the replacement.
- Temporary local files already ignored by Git.
- Empty placeholder files only after skeleton docs are updated.
- Old verification code only after equivalent coverage exists elsewhere and passes.

Unsafe deletion candidates:

- Any P0 anchor.
- Any safety contract.
- Any verification script that is still named in `docs/demo-checklist.md`.
- Any migration or seed file used by SQLite flow checks.
- Any API mapper or helper used by current server routes.
- Any file required by `apps/web/index.html`.

Before deleting a tracked file:

1. Search references with `rg`.
2. Update docs and scripts that mention it.
3. Run the relevant checks.
4. Commit the deletion separately from unrelated feature work.

## Standard Verification Sets

For documentation-only changes:

```powershell
git diff --check
git status --short
```

For API / Model Gateway changes:

```powershell
node --check services\api\model-gateway-adapters.js
node --check services\api\model-gateway.js
node --check services\api\server.js
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\verify-model-gateway.ps1
```

For Agent permission profile helper changes:

```powershell
node --check services\api\agent-permissions.js
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-permissions.ps1
```

For UI changes:

```powershell
node --check apps\web\app.js
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\verify-local-ui.ps1
```

For SQLite / state flow changes:

```powershell
node --check services\api\server.js
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
```

For broad cross-boundary changes:

```powershell
node --check apps\web\app.js
node --check services\api\model-gateway-adapters.js
node --check services\api\model-gateway.js
node --check services\api\agent-permissions.js
node --check services\api\server.js
git diff --check
powershell -ExecutionPolicy Bypass -File scripts\verify-model-gateway.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-agent-permissions.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-local-ui.ps1
powershell -ExecutionPolicy Bypass -File scripts\verify-sqlite-flows.ps1
```

## Current Assessment

The current architecture is stable enough for small, verified iteration:

- Web, API, Model Gateway, SQLite, Runner safety, Agent permissions, scripts, and docs have clear ownership.
- Real Runner execution and real model calls remain intentionally blocked.
- Verification scripts provide a practical safety net.

It is not a license to delete or add broad functionality without checks. Stability comes from preserving boundaries and running the right acceptance commands after each change.
