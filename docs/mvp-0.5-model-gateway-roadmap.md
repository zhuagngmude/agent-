# MVP-0.5 Model Gateway Roadmap

Date: 2026-06-11

This is a working roadmap for the next safety boundary. It does not enable real provider calls, cloud sync, or a broader permission system.

## Goal

Turn the existing disabled Model Gateway skeleton into a clearly named stage with a stable contract:

1. Model Gateway status remains read-only.
2. Dry-run and connectivity-test stay blocked by default.
3. Relay/provider metadata stays backend-only and redacted.
4. Verification stays isolated and no-side-effect.
5. The repo gets one clean place to look for the next-stage boundary.

## What Already Exists

- `services/api/model-gateway.js`
- `services/api/model-gateway-adapters.js`
- `scripts/verify-model-gateway.ps1`
- `docs/relay-provider-info-checklist.md`
- `docs/deepseek-provider-info-checklist.md`
- `docs/cheng-relay-manual-ping-spec.md`

## Skeleton Status

| Area | Status | Notes |
| --- | --- | --- |
| Status endpoint | Present | Already reports disabled boundary and provider metadata. |
| Dry-run preview | Present | Already returns blocked preflight with no side effects. |
| Connectivity test | Present | Already remains blocked and redacted by default. |
| Relay adapter shape | Present | Already models the future relay interface without enabling it. |
| Verification | Present | Already covers no-side-effect and blocked paths. |
| Formal stage doc | Missing | This file fills that gap. |

## Optimization Opportunities

- Keep one canonical stage document instead of scattering the same boundary wording across many docs.
- Keep provider-specific relay notes in the checklist docs, not the API draft.
- Keep model gateway verification isolated from Runner and Agent config verification.
- Do not broaden the skeleton into a real provider path until the safety contract changes on purpose.

## Not In Scope

- Real provider SDKs
- Real provider network requests
- Client-supplied API keys or base URLs
- Model prompt/result logging
- Cloud sync
- Full RBAC/ABAC

## Next Stage Anchor

- [MVP-0.6 to MVP-1.0 Roadmap](./mvp-0.6-to-1.0-roadmap.md)
- [MVP-0.6 to MVP-1.0 Plan Book](./mvp-0.6-to-1.0-plan-book.md)
