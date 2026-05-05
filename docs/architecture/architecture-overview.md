# Architecture Overview

**Last updated:** 2026-03-22
**Source docs consolidated:** `docs/architecture.md`, `docs/websocket_architecture.md`

---

## 1. System Overview

rustchat is a self-hosted team collaboration platform composed of 3 runtime services and 1 offline analysis tool:

| Service | Language | Purpose |
|---|---|---|
| `backend` | Rust (Axum 0.8 + Tokio) | HTTP API, WebSocket hub, business logic, DB |
| `frontend` | Svelte 5 + TypeScript | Single-page web application |
| `push-proxy` | Rust | Mobile push notification gateway (FCM/APNS) |
| `tools/mm-compat` | Python | Offline Mattermost compatibility analysis tooling |

**External dependencies:**

| Dependency | Purpose | Required |
|---|---|---|
| PostgreSQL 16+ | Primary data store | Yes |
| Redis 7+ | Pub/sub for cross-instance events, rate limiting, sessions | Yes |
| S3-compatible (RustFS) | File storage | Yes |
| FCM / APNS | Mobile push notifications (via push-proxy) | Optional |
| SMTP | Email notifications, password reset | Optional |
| OAuth providers | SSO login (configurable) | Optional |

```
┌────────────────────────────────────────────────────────────────────┐
│                        Web / SPA Client                            │
│                  (Svelte 5 + TypeScript)                           │
└──────────────────────────────┬─────────────────────────────────────┘
                               │ REST + WebSocket
                               ▼
┌────────────────────────────────────────────────────────────────────┐
│                      rustchat API Server                           │
│                      (Axum 0.8 + Tokio)                            │
│                                                                    │
│  /api/v1/*  ──── native API (internal clients)                     │
│  /api/v4/*  ──── Mattermost-compatible API (mobile/desktop clients)│
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     Service Layer                            │  │
│  │  auth · channels · posts · files · realtime · jobs · a2a    │  │
│  └──────────────────────┬───────────────────────────────────────┘  │
└─────────────────────────┼──────────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
   PostgreSQL           Redis           S3-compatible
   (primary data)    (events/cache)     (file storage)

┌──────────────────────────────┐
│       push-proxy             │
│  (Rust, separate service)    │
│  Receives events from        │
│  backend → delivers to       │
│  FCM (Android) / APNS (iOS)  │
└──────────────────────────────┘
```

---

## 2. Backend

**Stack:** Rust, Axum 0.8, Tokio, SQLx (compile-time checked queries), Tower middleware

**Top-level module structure** (`backend/src/`):

| Module | Responsibility |
|---|---|
| `a2a/` | Agent-to-agent communication layer |
| `api/` | HTTP handlers: v1 native API + v4 Mattermost-compatible API |
| `auth/` | Authentication: JWT generation/validation, password hashing (Argon2id) |
| `config/` | Environment-based configuration (the `config` crate) |
| `db/` | PostgreSQL connection pool, SQLx macros, migration runner |
| `error/` | Structured error types with HTTP status mapping |
| `jobs/` | Background job workers (async task queue) |
| `mattermost_compat/` | Mattermost-specific response transformation utilities |
| `middleware/` | Axum middleware: auth extraction, rate limiting, logging, CORS |
| `models/` | Data models: User, Channel, Post, Team, File, Entity, etc. |
| `realtime/` | WebSocket hub: connection management, event fan-out, cluster broadcast via Redis |
| `services/` | Business logic: one service per domain area (channels, posts, files, …) |
| `storage/` | S3-compatible file upload/download |
| `telemetry/` | Structured JSON logging with `tracing` |

Note: `channels` and `posts` are sub-modules under `api/v1/` and `services/`, not top-level modules.

**Request lifecycle:**
```
Request → Middleware (auth, rate-limit, CORS) → Router → Handler → Service → DB/Storage
                                                                           ↓
Response ← JSON serialization ← Result<T, AppError>
```

**Migrations:** `backend/migrations/` — SQLx numbered migrations, run automatically at startup. Irreversible — see `.governance/pr-size-limits.yml` for migration PR size constraints.

### WebSocket Hub

Two WebSocket endpoints share a common core (`api/websocket_core.rs`) but present different wire formats:

| Endpoint | Clients | Wire format |
|---|---|---|
| `/api/v1/ws` | rustchat web app, internal clients | Internal envelope (`type`, `event`, `data`, `channel_id`) |
| `/api/v4/websocket` | Mattermost mobile/desktop clients | Mattermost framing (`event`, `data`, `broadcast`, `seq`) |

**Shared core handles:**
- Auth token normalization (header + `Sec-WebSocket-Protocol` fallback)
- Connection limit enforcement
- Default team/channel subscription bootstrap
- Presence lifecycle: `online` on connect, `offline` when last connection drops
- Shared commands: `subscribe_channel`, `unsubscribe_channel`, `typing`, `presence`, `ping`→`pong`

**v4-specific behavior:**
- Optional auth challenge exchange (`action=authentication_challenge`)
- Session resumption (`connection_id`, `sequence_number`)
- Mattermost event name mapping: `posted`, `typing`, `post_edited`, `status_change`, etc.

**Event fan-out:**
```
Service writes event → realtime::hub → broadcast to subscribed connections
                                    → Redis pub/sub → other backend instances → their hubs
```

**Client-side Connection State Management:**

The frontend implements progressive disconnection UX to handle network interruptions gracefully:

| State | Duration | Visual | Behavior |
|-------|----------|--------|----------|
| `connected` | — | 🟢 Green dot | Normal operation |
| `reconnecting` | < 5s | 🟡 Yellow banner, 80% opacity | Auto-retry with backoff |
| `disconnected` | 5-30s | 🟠 Orange banner, 60% opacity | Manual retry option |
| `failed` | > 30s | 🔴 Full-screen modal | Explicit reconnect required |

**State transitions:**
```
CONNECTED → onClose → RECONNECTING (auto-retry, exponential backoff)
RECONNECTING → timeout 5s → DISCONNECTED (manual retry available)
DISCONNECTED → timeout 30s → FAILED (must reconnect or refresh)
```

See [`docs/internal/SPEC-WEBSOCKET-DISCONNECTION-UX.md`](../../docs/internal/SPEC-WEBSOCKET-DISCONNECTION-UX.md) for full specification.

---

## 3. Frontend

**Stack:** Svelte 5, TypeScript, Svelte stores, Vite (build)

**Directory structure:**
```
frontend/src/
├── core/          # Shared primitives: entities, errors, websocket infrastructure
├── features/      # 14 domain feature modules (auth, calls, channels, messages, …)
├── api/           # API client functions
├── components/    # Svelte components
├── shared/        # Shared utilities and helpers
└── stores/        # Legacy stores (deprecated, being migrated to features/)
```

**Feature module pattern** — every feature follows the same layers:
```
features/[feature]/
├── repositories/    # Data access (API calls)
├── services/        # Business logic
├── stores/          # Svelte stores (no business logic)
├── handlers/        # WebSocket event handlers
└── index.ts         # Public API
```

**E2E tests:** Playwright snapshot tests in `frontend/e2e/`. Run with `cd frontend && npx playwright test`.

---

## 4. Push Proxy

Separate Rust service (`push-proxy/`). Receives push notification events from the backend via an internal HTTP call and forwards them to FCM (Android) or APNS (iOS). Deployed separately from the main backend to isolate credential scope.

---

## 5. Mattermost Compatibility Surface

The compatibility surface is the set of paths that external Mattermost clients depend on. Changes here require compat-reviewer co-approval (see `CODEOWNERS`):

| Path | Purpose |
|---|---|
| `backend/src/api/v4/` | Mattermost HTTP API v4 handlers |
| `backend/src/mattermost_compat/` | Response transformation, field mapping utilities |
| `backend/compat/` | Contract JSON schemas + contract validation tests |
| `backend/src/realtime/` | WebSocket hub (v4 event contracts) |

For coverage details see `docs/compatibility-scope.md`.

---

## 6. Data Flow

### HTTP Request Lifecycle

```
Client → [Nginx proxy, port 8080 in Docker] → Axum Router (port 3000) → Middleware (auth, rate-limit, CORS)
       → Handler → Service → DB / Storage
       ← JSON response ← Result<T, AppError>
```

### WebSocket Event Flow

```
Service writes event
  → realtime::hub
  → broadcast to subscribed local connections
  → Redis pub/sub → other backend instances → their hubs → their connections
```

### Frontend Data Flow

```
Component → Store → Service → Repository → API (HTTP)
                ↑
WebSocket → Handler → Service → Store update
```

---

## 7. Key Design Decisions

- **Axum over Actix-web:** Tower middleware ecosystem, async-first, ergonomic extractors.
- **Separate push-proxy service:** Isolates FCM/APNS credentials; can be scaled/deployed independently.
- **Redis for cross-instance fan-out:** Enables horizontal scaling of API servers without sticky sessions.
- **SQLx compile-time query checks:** Prevents schema/query drift at the cost of requiring a live DB at compile time (see `SQLX_OFFLINE` flag for CI).
- **Two WebSocket endpoints (v1 + v4):** Avoids breaking the native web app while maintaining Mattermost client compatibility. Shared core prevents logic drift between them.
- **Feature-based frontend structure:** Avoids the 960-line store antipattern; enforces single responsibility (avg 105 lines/file post-refactor).
