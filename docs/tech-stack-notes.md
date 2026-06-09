# Technology Stack Notes

Date: 2026-06-09

This note records the current technology-stack decision for `agent-swarm`. It separates what is already in the repository from what should be considered later.

## Current Actual Stack

The current stack is appropriate for MVP-0.2 validation because it keeps moving parts small and easy to inspect.

- Frontend: native HTML, CSS, and JavaScript in `apps/web/`.
- Mock backend: Node.js native HTTP server in `services/api/server.js`.
- Mock data and state: JavaScript mock data plus local runtime state under `data/mock/`.
- Scripts: PowerShell scripts under `scripts/`.
- Documentation: Markdown under `README.md`, `AGENTS.md`, `dev-docs/`, and `docs/`.
- Local database experiment: SQLite stored at `data/local/agent-swarm.sqlite`, initialized and seeded through PowerShell scripts that use Python standard-library `sqlite3`.

This stack is not meant to be the final commercial stack. Its job is to validate product flow, API shape, database shape, approval rules, and Runner safety boundaries before introducing heavier frameworks.

## SQLite Local Persistence Path

SQLite is currently a local development and validation path, not the final cloud database decision.

- SQLite mode is enabled by setting `AGENT_SWARM_DASHBOARD_SOURCE=sqlite`.
- Migration lives in `data/migrations/001_initial_sqlite.sql`.
- Seed data lives in `data/seed/project_agent_swarm.seed.json`.
- Read mapping lives in `services/api/db/sqlite-read.js`.
- Write-state transitions live in `services/api/db/sqlite-write.js`.
- Local database files under `data/local/` must stay untracked.

SQLite is useful now because it proves the table design and state transitions without requiring cloud accounts, auth, networking, migrations infrastructure, or production deployment.

## Likely Future Production Stack

After the MVP control flow, database model, and Runner safety rules are stable, the likely production stack can be upgraded.

- Frontend: React with TypeScript, likely Vite or Next.js depending on deployment and routing needs.
- Styling/UI: Tailwind CSS plus a mature component system such as Radix/shadcn-style primitives, while preserving the project-specific visual direction.
- Backend: TypeScript backend or Python FastAPI. The decision should be made after the Agent orchestration and Runner boundary are clearer.
- Database: PostgreSQL, possibly Supabase PostgreSQL for early production because it also offers Auth and RLS.
- Auth/permissions: Supabase Auth/RLS or a dedicated auth layer with RBAC plus ABAC.
- Runner: Python Runner remains a good fit for local file operations, Git checkpoints, test execution, and cross-platform scripting.
- AI model calls: provider SDKs should be isolated behind service boundaries instead of being called directly from UI code.

## Do Not Migrate Yet

Do not migrate to a heavier framework only because it looks more formal. Migrate when the current simple stack becomes the bottleneck.

- Do not move the frontend to React/Next.js until the core screens, state shapes, and approval flows are stable.
- Do not replace SQLite with PostgreSQL until the local schema and state transitions have passed regression checks.
- Do not connect real model APIs until approval, logging, cost tracking, and key-safety rules are ready.
- Do not implement real Runner execution until `docs/runner-safety-acceptance.md` is satisfied.
- Do not add cloud sync or full permissions before the local single-project flow is reliable.

## Current Decision

The current languages and tools are suitable for this stage:

- JavaScript is suitable for the current frontend and mock API because it keeps the prototype fast and inspectable.
- PowerShell is suitable for local Windows development scripts in this project.
- Python is suitable as a SQLite bridge now and remains suitable for the future local Runner.
- Markdown is suitable for product, architecture, and AI handoff records.
- SQLite is suitable as the first real persistence layer for local validation.

The final product will probably need a stronger typed frontend/backend and PostgreSQL, but switching too early would add framework work before the product and safety model are stable.
