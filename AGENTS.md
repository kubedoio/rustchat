# RustChat Agent & Contributor Guide

Canonical entry point for AI agents and human contributors working in the RustChat repository.

## Project Overview

RustChat is a self-hosted team communication platform. The runtime is split into focused services; compatibility analysis is handled by offline tooling.

```
rustchat/
├── backend/          # Rust API server (Axum + SQLx + PostgreSQL)
│   ├── src/          # a2a, api (v1 + v4), auth, config, db, error, jobs,
│   │                 # mattermost_compat, middleware, models, realtime,
│   │                 # services, storage, telemetry
│   ├── migrations/   # SQLx database migrations
│   └── tests/        # Integration tests
├── frontend/         # Vue 3 + TypeScript SPA
│   ├── src/          # core, features, api, components, composables, stores
│   └── e2e/          # Playwright tests
├── push-proxy/       # Rust service for FCM/APNS mobile push notifications
├── docs/             # Architecture, development, and operational docs
├── scripts/          # Utility and smoke-test scripts
└── tools/mm-compat/  # Offline Mattermost compatibility analysis tooling
```

## Technology Stack

| Layer | Stack |
|---|---|
| Backend | Rust 1.95+, Axum 0.8, Tokio, SQLx, PostgreSQL 16+ (with pgvector), Redis 7+, S3-compatible storage |
| Frontend | Vue 3.5+, TypeScript 5.9+, Vite 8+, Pinia 3+, Tailwind CSS 4+ |
| Push Proxy | Rust 1.95+, Axum 0.8, Tokio, FCM (HTTP v1), APNS (JWT) |
| Compat analysis | Python tooling under `tools/mm-compat/` |

## Build, Test, and Validation

Run the checks that match CI for the area you changed.

### Backend

Fast checks (no running services needed):

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib
```

Backend integration tests require PostgreSQL, Redis, and S3-compatible storage:

```bash
docker compose -f docker-compose.integration.yml up -d
# export the test env vars documented in docs/development/testing.md
cargo test --no-fail-fast
```

### Push Proxy

`push-proxy/` is a separate Cargo workspace. If you change this service, run the same Rust checks there:

```bash
cd push-proxy
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### Frontend

```bash
cd frontend
npm ci --ignore-scripts
npm run apply:dependency-patches
npm run build
npm run test:unit
```

E2E tests require a running full stack; see `docs/development/testing.md`.

## AI Agent Boundaries

Boundaries are enforced by path scope. When in doubt, ask for human review rather than guessing.

| Agent | Allowed paths | Prohibited / gated paths |
|---|---|---|
| `backend-agent` | `backend/src/**`, `backend/tests/**`, `backend/migrations/**`, `push-proxy/**` | `backend/src/auth/**` (explicit approval); `backend/src/api/v4/**` (compat-reviewer co-approval); `backend/src/mattermost_compat/**` (compat-reviewer co-approval); `backend/src/a2a/**` (senior review); `.governance/**`; `frontend/**` |
| `frontend-agent` | `frontend/src/**`, `frontend/e2e/**` | `backend/**`; `.governance/**` |
| `compat-agent` | Read-only: `backend/compat/**`, `backend/src/api/v4/**`, `backend/src/mattermost_compat/**`, `tools/mm-compat/**` | Writes only to `previous-analyses/**`, `docs/superpowers/specs/**`, `docs/superpowers/plans/**`; cannot approve PRs or write production code |

**General rule:** changes to auth, permissions, protocol/storage model, API contract, or migrations require human review.

Source of truth for boundaries: `.governance/agent-contracts.yml`.

## Detailed Guides

- [`docs/development/testing.md`](docs/development/testing.md) — test layers, env vars, CI gates
- [`docs/development/code-style.md`](docs/development/code-style.md) — Rust and TypeScript/Vue conventions
- [`docs/contributor-workflow.md`](docs/contributor-workflow.md) — fork/branch/PR workflow
- [`docs/architecture/overview.md`](docs/architecture/overview.md) — system design and data flow
- [`docs/agent-guides/README.md`](docs/agent-guides/README.md) — agent-specific reference links

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
feat: add channel search endpoint
fix: correct JWT expiry calculation
docs: update contributor guide
test: add channel permission tests
refactor: simplify message rendering
```

Every commit must be signed off per the Developer Certificate of Origin (`DCO.md`):

```bash
git commit -s -m "feat: add channel search endpoint"
```

DCO is enforced in CI; PRs with unsigned commits will fail the required check.
