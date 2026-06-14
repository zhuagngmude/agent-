# scripts

Local startup and verification entrypoints for `agent-swarm`.

## Start / Stop

- `start-dev.ps1`: start the Mock API and open the web app.
- `start-local.ps1`: start the SQLite trial API and local static web app.
- `status-local.ps1`: inspect API, web, SQLite, and pid status.
- `stop-local.ps1`: stop the local trial and clean pid files.

## Verification

- `verify-project-plan-flow.ps1`: helper-only project-plan approval flow.
- `verify-mock-flows.ps1`: isolated Mock flow checks on `8789`.
- `verify-sqlite-flows.ps1`: isolated SQLite flow checks on `8788`.
- `verify-local-ui.ps1`: browser smoke for the running local trial on `8787/5175`, including the Agent Run record page.
- `verify-model-gateway.ps1`: disabled Model Gateway and relay boundary checks.
- `verify-real-model-admission.ps1`: disabled project-plan model request admission checks, provider config resolver checks, redaction / safe-record checks, model_calls write-draft helper checks, and isolated disabled route checks; no provider calls.
- `verify-agent-permissions.ps1`
- `verify-agent-config-fields.ps1`
- `verify-agent-config-dry-run.ps1`
- `verify-agent-config-apply-gate.ps1`
- `verify-agent-config-transaction-plan.ps1`
- `verify-agent-config-rollback-request.ps1`
- `verify-agent-config-version-history.ps1`
- `verify-agent-config-real-apply-sqlite.ps1`
- `verify-agent-config-safety-loop.ps1`

The mock / SQLite flow scripts now also cover Agent Run chain recording, execution request lifecycle transitions, read-only execution request views, and runtime event auditing.

Tauri/Rust write commands are accepted through Rust checks instead of a PowerShell script. See [docs/write-commands-acceptance.md](../docs/write-commands-acceptance.md) for the current `create_task`, `update_task_status`, `create_approval`, `approve_approval`, `reject_approval`, and `patch_only_approval` evidence.

## Ports

- `8787`: manual local trial default
- `8788`: SQLite verification
- `8789`: Mock verification
- `8790`: feature-gated SQLite real-apply verification

## Notes

- Verification scripts must not call real model providers, execute Runner, modify Git, or touch protected local state.
- `verify-real-model-admission.ps1` uses isolated port `8791` and a temp runtime state file under `%TEMP%`.
- See [docs/demo-checklist.md](../docs/demo-checklist.md) for the human demo path.
