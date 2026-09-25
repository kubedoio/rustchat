# Architecture Overview

**Last updated:** 2026-09-25

This is the canonical RustChat architecture overview. More detailed per-area
documents live alongside it in `docs/architecture/`.

---

## 1. System Overview

RustChat is a self-hosted team collaboration platform composed of 3 runtime services and 1 offline analysis tool:

| Service | Language | Purpose |
|---|---|---|
| `backend` | Rust (Axum 0.8 + Tokio) | HTTP API, WebSocket hub, business logic, DB |
| `frontend` | Vue 3.5 + TypeScript + Pinia | Single-page web application |
| `push-proxy` | Rust | Mobile push notification gateway (FCM/APNS) |
| `tools/mm-compat` | Python | Offline Mattermost compatibility analysis tooling |

**External dependencies:**

| Dependency | Purpose | Required |
|---|---|---|
| PostgreSQL 16+ (with pgvector) | Primary data store, vector search for RAG | Yes |
| Redis 7+ | Pub/sub for cross-instance events, rate limiting, sessions | Yes |
| S3-compatible (RustFS) | File storage | Yes |
| FCM / APNS | Mobile push notifications (via push-proxy) | Optional |
| SMTP | Email notifications, password reset | Optional |
| OAuth providers | SSO login (configurable) | Optional |

```
┌────────────────────────────────────────────────────────────────────┐
│                        Web / SPA Client                            │
│                  (Vue 3.5 + TypeScript + Pinia)                    │
└──────────────────────────────┬─────────────────────────────────────┘
                               │ REST + WebSocket
                               ▼
┌────────────────────────────────────────────────────────────────────┐
│                      RustChat API Server                           │
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

### Authentication Flow

```
┌─────────┐    POST /api/v1/auth/login    ┌──────────┐
│  Client │ ─────────────────────────────▶│ Backend  │
│         │                               │          │
│         │◀──────────────────────────────│          │
│         │    { access_token, refresh }  │          │
│         │                               │          │
│         │    WS /api/v1/ws              │          │
│         │    Authorization: Bearer ...  │          │
│         │ ─────────────────────────────▶│          │
│         │                               │          │
│         │◀──────────────────────────────│          │
│         │    Real-time events           │          │
└─────────┘                               └──────────┘
```

1. Client POSTs credentials to `/api/v1/auth/login`
2. Backend validates against PostgreSQL, issues JWT access + refresh tokens
3. Client includes `Authorization: Bearer <token>` header on all API requests
4. WebSocket connections authenticate with the same token via header (or `Sec-WebSocket-Protocol` fallback)
5. Refresh token endpoint (`/api/v1/auth/refresh`) silently extends sessions

### AI Agent Runtime

AI agents are first-class channel participants backed by normal `users` rows with `entity_type = 'agent'`. Agent configuration lives in `agent_configs`, channel overrides live in `agent_channel_settings`, and conversation memory lives in `agent_memories`.

The runtime is optional. At startup the backend initializes `AgentRuntime` only when an LLM provider is configured. If no provider key is present, the application still starts and logs that agent runtime is disabled; admin CRUD for stored agent configuration can still exist, but agents will not generate responses.

**Agent response flow:**
```
User posts message
  → post service persists and broadcasts message
  → AgentRuntime checks channel assignments and trigger rules
  → runtime builds prompt from system prompt, recent channel context, memory, and RAG
  → LLM provider generates or streams a response
  → agent post is persisted and broadcast through the WebSocket hub
```

Triggering is channel-scoped. Agents can respond when mentioned with the normal `@username` syntax, and agents configured with `respond_to_all` can answer every message in assigned channels. Agents must be channel members before they can read or respond in a channel.

### LLM Provider Layer

LLM calls are isolated behind a provider registry in `services/llm/`. The runtime currently registers the OpenAI provider when `RUSTCHAT_OPENAI_API_KEY` or `OPENAI_API_KEY` is set. Agent configs store provider, model, temperature, max tokens, and prompt settings; sensitive provider tokens are encrypted before storage.

The provider layer keeps the runtime independent from a single vendor. Additional providers can be added behind the same trait without changing the API handlers or frontend management views.

### RAG Pipeline

Knowledge bases provide retrieval-augmented generation for agents. Documents are stored in S3-compatible storage, metadata and chunks are stored in PostgreSQL, and embeddings use `pgvector` for semantic search. A knowledge base can contain uploaded documents or documents synced from an external source such as RustShare.

**Retrieval flow:**
```
Document upload or sync
  → extract text
  → split into chunks
  → generate embeddings
  → store chunk metadata and vectors in PostgreSQL
  → retrieve top matches when an assigned agent is prompted
```

RAG requires PostgreSQL with the `pgvector` extension. OpenAI embeddings are used when an OpenAI key is available; deployments can use the documented local embedding fallback for air-gapped environments.

### Tool Calling

The tool framework lets agents use registered server-side tools during generation. Tools are available only when their backing credentials are configured. For example, the web search tool is registered when `TAVILY_API_KEY` is present.

Tool execution is mediated by the backend, not by the browser. The runtime exposes tool schemas to the LLM provider, validates requested tool calls, executes registered tools, and feeds tool results back into the response context. This keeps external credentials out of the frontend and allows operators to disable tools by removing their environment variables.

---

## 3. Frontend

**Stack:** Vue 3.5, TypeScript, Pinia (state), Vue Router, Vite (build)

**Directory structure:**
```
frontend/src/
├── core/          # Shared primitives: entities, errors, websocket infrastructure
├── features/      # Domain feature modules (auth, calls, channels, messages, …)
├── api/           # API client functions
├── components/    # Vue components
├── composables/   # Vue composables
└── stores/        # Legacy Pinia stores (deprecated, being migrated to features/)
```

**Feature module pattern** — every feature follows the same layers:
```
features/[feature]/
├── repositories/    # Data access (API calls)
├── services/        # Business logic
├── stores/          # Pinia state (no business logic)
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

For coverage details see [Compatibility Scope](../compatibility-scope.md).

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

### File Upload Flow

```
┌─────────┐    POST /api/v1/files    ┌──────────┐    PUT      ┌─────────┐
│  Client │ ───────────────────────▶│ Backend  │ ──────────▶│   S3    │
│         │  multipart/form-data    │          │  internal  │ (RustFS)│
│         │                         │          │  client    │         │
│         │◀─────────────────────── │          │◀───────────│         │
│         │  { file_id, api url }   │          │            │         │
│         │                         │          │            │         │
│         │    GET /api/v1/files/...│          │            │         │
│         │ ───────────────────────▶│          │  proxy     │         │
│         │◀─────────────────────── │          │◀───────────│         │
│         │  file bytes (auth'd)    │          │            │         │
└─────────┘                         └──────────┘            └─────────┘
```

1. Client uploads file via multipart POST to backend
2. Backend validates channel membership before reading the upload body
3. Backend uploads to S3-compatible storage through the server-side storage client
4. Backend returns file metadata and an authenticated RustChat API URL to client
5. Client requests file download through backend (authenticated)
6. Backend proxies the file from S3 with auth check — no presigned URL leaks to end users

The native presign endpoint is intentionally unsupported for end-user clients. Uploads should use multipart `POST /api/v1/files`, and reads should use the returned RustChat API URL. Mattermost-compatible file link responses and custom emoji image responses also return or stream through RustChat API URLs rather than direct S3 links.

### Frontend Data Flow

```
Component → Store → Service → Repository → API (HTTP)
                ↑
WebSocket → Handler → Service → Store update
```

---

## 7. Deployment Topology

### Single-Node (Evaluation / Small Team)

```
┌─────────────────────────────────────────────┐
│                 Single Host                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │
│  │ Nginx   │  │Backend  │  │Frontend     │ │
│  │(TLS)    │  │(Port   │  │(Port 8080)  │ │
│  │(443)    │  │ 3000)   │  │             │ │
│  └────┬────┘  └────┬────┘  └─────────────┘ │
│       │            │                        │
│       └────────────┘                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────────┐ │
│  │PostgreSQL│  │  Redis  │  │S3 (RustFS)  │ │
│  │(Port   │  │(Port   │  │(Port 9000)   │ │
│  │ 5432)   │  │ 6379)   │  │             │ │
│  └─────────┘  └─────────┘  └─────────────┘ │
└─────────────────────────────────────────────┘
```

### Multi-Node (Production)

```
                        ┌─────────────┐
                        │  Load Balancer│
                        │   (TLS)       │
                        └──────┬──────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        ┌──────────┐     ┌──────────┐     ┌──────────┐
        │ Backend 1│     │ Backend 2│     │ Backend N│
        │          │◄───►│          │◄───►│          │
        └────┬─────┘     └────┬─────┘     └────┬─────┘
             │                │                │
             └────────────────┼────────────────┘
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
              ┌──────────┐        ┌──────────┐
              │ PostgreSQL│        │  Redis   │
              │  (HA)     │        │ (Cluster)│
              └──────────┘        └──────────┘
                    │
                    ▼
              ┌──────────┐
              │S3 (Cloud) │
              └──────────┘
```

**Key differences:**
- Load balancer distributes HTTP traffic (sticky sessions recommended for WebSocket)
- Redis pub/sub synchronizes real-time events across backend instances
- PostgreSQL and S3 are shared external services
- Frontend is a static SPA that can be served by CDN or any static file server

---

## 8. Key Design Decisions

- **Axum over Actix-web:** Tower middleware ecosystem, async-first, ergonomic extractors.
- **Separate push-proxy service:** Isolates FCM/APNS credentials; can be scaled/deployed independently.
- **Redis for cross-instance fan-out:** Enables horizontal scaling of API servers without sticky sessions.
- **SQLx compile-time query checks:** Prevents schema/query drift at the cost of requiring a live DB at compile time (see `SQLX_OFFLINE` flag for CI).
- **Two WebSocket endpoints (v1 + v4):** Avoids breaking the native web app while maintaining Mattermost client compatibility. Shared core prevents logic drift between them.
- **Feature-based frontend structure:** Avoids the 960-line store antipattern; enforces single responsibility (avg 105 lines/file post-refactor).
- **Authenticated file delivery:** Files flow through RustChat API URLs with authorization checks instead of presigned S3 URLs.

---

## Further Reading

- [Realtime Architecture](./realtime.md) — WebSocket hub, wire formats, and client connection-state model
- [Backend Architecture](./backend.md)
- [Frontend Architecture](./frontend.md)
- [Data Model](./data-model.md)
- [Integrations](./integrations.md)
- [Calls Deployment](./calls-deployment.md)
