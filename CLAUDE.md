# RustChat Claude Compatibility Entry Point

**Canonical contributor and agent policy:** [AGENTS.md](AGENTS.md)

Claude-based tooling must follow `AGENTS.md`, `.governance/**`, CODEOWNERS,
and the same validation requirements as every other contributor. This file is a
compatibility/bootstrap entry point only; it does not define an independent
architecture or policy.

## Fast orientation

- Backend: `backend/` — Rust/Axum, PostgreSQL/SQLx, Redis, S3-compatible storage.
- Frontend: `frontend/` — Vue 3 + TypeScript + Vite + Pinia.
- Push proxy: `push-proxy/` — separate Rust service for APNS/FCM.
- Architecture: `docs/architecture/overview.md`.
- Development/testing: `docs/development/README.md` and
  `docs/development/testing.md`.
- Repository governance: `GOVERNANCE.md` and `.governance/**`.

## Important rule

Do not infer desired architecture from old reports or this compatibility file.
Read the current ADRs under `docs/adr/**` and current code before editing.

In particular, direct SQL still exists in some API handlers as acknowledged
technical debt. New persistence should follow current repository/service
boundaries and ADR-006 once that ADR is accepted; do not assume the existing
tree is already fully migrated.

For commands, path boundaries, context discipline, DCO, and final validation,
use [AGENTS.md](AGENTS.md).
