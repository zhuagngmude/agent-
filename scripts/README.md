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
- `verify-local-ui.ps1`: browser smoke for the running local trial on `8787/5175`.
- `verify-model-gateway.ps1`: MVP-0.6 Model Gateway contract freeze and relay boundary checks.
- `verify-agent-permissions.ps1`
- `verify-agent-config-fields.ps1`
- `verify-agent-config-dry-run.ps1`
- `verify-agent-config-apply-gate.ps1`
- `verify-agent-config-transaction-plan.ps1`
- `verify-agent-config-rollback-request.ps1`
- `verify-agent-config-version-history.ps1`
- `verify-agent-config-real-apply-sqlite.ps1`
- `verify-agent-config-safety-loop.ps1`

The mock / SQLite flow scripts now also cover execution request lifecycle transitions, read-only execution request views, and runtime event auditing.

## Ports

- `8787`: manual local trial default
- `8788`: SQLite verification
- `8789`: Mock verification
- `8790`: feature-gated SQLite real-apply verification

## Notes

- Verification scripts must not call real model providers, execute Runner, modify Git, or touch protected local state.
- See [docs/demo-checklist.md](../docs/demo-checklist.md) for the human demo path.
