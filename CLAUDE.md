# RustChat Agent Context

RustChat is a high-performance, secure collaboration platform built with Rust (axum) and Vue 3. It provides a native API plus a Mattermost-compatible v4 API surface.

---

## Prerequisites

- **Rust** 1.95+
- **Node.js** 24+
- **Docker** + Docker Compose (for PostgreSQL, Redis, object storage)
- **sqlx-cli** (optional but recommended): `cargo install sqlx-cli --no-default-features --features postgres`

Run `scripts/dev-setup.sh` to validate the environment, create `.env` from `.env.example`, start Docker dependencies, run migrations, and install frontend dependencies.

---

## Build Commands

| Component | Command |
|-----------|---------|
| Backend | `cd backend && cargo build --release` |
| Frontend (dev) | `cd frontend && npm run dev` |
| Frontend (prod) | `cd frontend && npm run build` |
| Push Proxy | `cd push-proxy && cargo build --release` |
| All (Docker) | `docker compose up -d --build` |

The backend binary is named `rustchat`. The push-proxy binary is named `push-proxy`.

---

## Test Commands

| Component | Command |
|-----------|---------|
| Backend unit / integration | `cd backend && cargo test` |
| Frontend unit tests | `cd frontend && npm run test:unit` |
| Frontend E2E | `cd frontend && npm run test:e2e` |
| Push Proxy | `cd push-proxy && cargo test` |

Backend integration tests live in `backend/tests/` and require a running database. The E2E suite uses Playwright and spins up its own web server.

---

## Lint & Format Commands

| Component | Command |
|-----------|---------|
| Backend lint | `cd backend && cargo clippy --all-targets -- -D warnings` |
| Frontend lint | `cd frontend && npm run lint` |
| Frontend format check | `cd frontend && npm run format:check` |
| Frontend format fix | `cd frontend && npm run format` |
| Push Proxy lint | `cd push-proxy && cargo clippy --all-targets -- -D warnings` |
| Frontend type check | `cd frontend && npx vue-tsc --noEmit` |
| Frontend dead-code check | `cd frontend && npx knip` |

---

## Architecture Overview

### Backend (`backend/`)

- **Web framework**: axum + tokio
- **Dual API surface**:
  - Native v1 API: `src/api/v1/` and top-level route modules (`auth.rs`, `channels.rs`, `posts.rs`, etc.)
  - Mattermost-compatible v4 API: `src/api/v4/`
- **Repository pattern**: All SQL should live in `src/repositories/` (e.g., `post_repository.rs`, `user_repository.rs`). Services in `src/services/` call repositories; they do **not** contain `sqlx::query` directly.
- **State management**: Shared Axum state holds the DB pool, Redis pool, WebSocket hub (`WsHub`), S3 client, and config.
- **Realtime**: WebSocket hub (`src/realtime/`) broadcasts events to connected clients.
- **Storage**: S3-compatible object storage (default: RustFS via Docker Compose) for file uploads.

### Frontend (`frontend/`)

- **Stack**: Vue 3 + TypeScript + Vite + Pinia + Tailwind CSS v4
- **Feature-based modules**: New code is organized under `src/features/*/` (e.g., `auth`, `channels`, `messages`, `presence`, `permissions`).
- **Migration in progress**: Some legacy Pinia stores remain in `src/stores/`; new stores should go in `src/features/<feature>/stores/`.
- **Components**: Use `<script setup>` for all new Vue components.
- **Atomic components**: Reusable UI primitives live in `src/components/atomic/`.

### Push Proxy (`push-proxy/`)

- Standalone Rust service that forwards push notifications to Apple Push Notification service (APNs) and Firebase Cloud Messaging (FCM).
- Built with axum; shares Rust toolchain version with backend.

---

## Database

- **Engine**: PostgreSQL
- **Access**: sqlx (compile-time checked queries)
- **Migrations**: `backend/migrations/` (timestamped `.sql` files)
- **Run migrations**: `cd backend && sqlx migrate run`
- **Prepare offline query data** (for CI): `cd backend && cargo sqlx prepare`

Docker Compose starts Postgres, Redis, and RustFS automatically via `docker compose up -d postgres redis rustfs`.

---

## Important Conventions

1. **SQL belongs in repositories**. Never write `sqlx::query` in service layers or API handlers. Services orchestrate business logic by calling repository methods.
2. **Vue components use `<script setup>`**. Avoid the Options API in new code.
3. **Feature-based stores**. When adding new Pinia stores, place them in `src/features/<feature>/stores/` rather than the legacy `src/stores/` root.
4. **Type safety**. Frontend uses `vue-tsc` for type checking. Backend uses `sqlx` compile-time query verification (requires `DATABASE_URL` or prepared query data).
5. **Environment variables**. Required secrets are listed in `.env.example`. Never commit real values. Generate secrets with `openssl rand -hex 32`.

---

## Quick Start

```bash
# 1. Validate environment and start dependencies
./scripts/dev-setup.sh

# 2. Edit .env and set required secrets

# 3. Start backend
cd backend && cargo run

# 4. Start frontend (new terminal)
cd frontend && npm run dev

# 5. Health check
curl -s http://localhost:3000/api/v1/health/live | jq .
```

---

## Project Structure

```
backend/           Rust backend (axum, sqlx, tokio)
  src/
    api/           Route handlers (v1 + v4)
    repositories/  SQL/data access layer
    services/      Business logic layer
    models/        Domain types
    realtime/      WebSocket hub
    middleware/    Axum middleware
  migrations/      sqlx database migrations
  tests/           Integration tests

frontend/          Vue 3 + Vite frontend
  src/
    features/      Feature-based modules (preferred)
    components/    Shared components (atomic/, ui/, etc.)
    stores/        Legacy Pinia stores (mid-migration)
    router/        Vue Router config
  e2e/             Playwright E2E tests

push-proxy/        Rust push notification proxy

docs/              VitePress documentation site
scripts/           Dev setup, smoke tests, release checks
docker/            Dockerfiles for all services
```
