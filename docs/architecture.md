# Architecture

## High-Level Overview

RustChat is composed of three runtime services and four external dependencies:

| Service | Technology | Purpose |
|---------|------------|---------|
| **Backend** | Rust (Axum 0.8 + Tokio) | HTTP API, WebSocket hub, business logic |
| **Frontend** | Svelte 5 + TypeScript | Single-page web application |
| **Push Proxy** | Rust (Axum) | Mobile push notification gateway (FCM / APNS) |

**External dependencies:**

| Dependency | Purpose | Required |
|------------|---------|----------|
| PostgreSQL 16+ | Primary data store | Yes |
| Redis 7+ | Pub/sub, rate limiting, sessions | Yes |
| S3-compatible storage | File uploads | Yes |
| FCM / APNS | Mobile push notifications | Optional |
| SMTP | Email, password reset | Optional |
| OAuth providers | SSO login | Optional |
| Meilisearch | Full-text search | Optional (via `search` profile) |

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Web Client    │────▶│  RustChat API    │◀────│  Push Proxy     │
│  (Svelte SPA)   │     │  (Rust / Axum)   │     │ (Mobile Push)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
      ┌──────────┐      ┌──────────┐      ┌──────────────┐
      │PostgreSQL│      │  Redis   │      │S3-compatible │
      │(Primary) │      │(Pub/Sub) │      │(File Store)  │
      └──────────┘      └──────────┘      └──────────────┘
```

---

## Request Flow

A typical HTTP request flows through these layers:

```
Client → Reverse Proxy (optional) → Axum Router → Middleware → Handler → Service → Database/S3/Redis
```

1. **Client request** → Reverse proxy (optional, for TLS)
2. **Axum router** → Matches path to handler
3. **Middleware** → Auth (JWT validation), rate limiting, CORS, tracing, request ID
4. **Handler** → Thin HTTP layer, delegates to services
5. **Service** → Business logic, database queries, external calls
6. **Response** → Serialized via `Result<T, AppError>` with consistent JSON error format

---

## Authentication Flow

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
4. WebSocket connection uses the same token via header (or query param in development)
5. Refresh token endpoint (`/api/v1/auth/refresh`) silently extends sessions

---

## API Surface

The backend exposes two API versions on the same HTTP server:

| API | Path | Purpose |
|-----|------|---------|
| **Native** | `/api/v1/*` | Web client, modern features |
| **Compatible** | `/api/v4/*` | Mattermost-compatible for mobile/desktop clients |

Both share the same service layer and database. The v4 layer translates request/response formats to match Mattermost's API contract.

---

## Real-Time Communication

Two WebSocket endpoints share a common event core:

| Endpoint | Format | Clients |
|----------|--------|---------|
| `/api/v1/ws` | Internal envelope | Web app |
| `/api/v4/websocket` | Mattermost framing | Mobile apps, desktop clients |

### WebSocket Event Flow

```
┌─────────┐     WS connection      ┌──────────┐
│ Client A│◀──────────────────────▶│ Backend 1│
│         │                        │          │
│         │                        │  Redis   │
│         │                        │  pub/sub │
│         │                        │          │
│ Client B│◀──────────────────────▶│ Backend 2│
└─────────┘                        └──────────┘
```

1. Client A sends a message via WebSocket to Backend 1
2. Backend 1 persists to PostgreSQL
3. Backend 1 publishes event to Redis pub/sub channel
4. Backend 2 (and any other instances) receives the event from Redis
5. Backend 2 forwards the event to connected clients (including Client B)

This design allows horizontal scaling: add more backend instances behind a load balancer, and Redis coordinates real-time delivery across all of them.

---

## File Upload Flow

```
┌─────────┐    POST /api/v1/files    ┌──────────┐    PUT      ┌─────────┐
│  Client │ ───────────────────────▶│ Backend  │ ──────────▶│   S3    │
│         │  multipart/form-data    │          │  presigned │ (RustFS)│
│         │                         │          │  URL       │         │
│         │◀─────────────────────── │          │◀───────────│         │
│         │  { file_id, url }       │          │            │         │
│         │                         │          │            │         │
│         │    GET /api/v1/files/...│          │            │         │
│         │ ───────────────────────▶│          │            │         │
│         │                         │          │  proxy     │         │
│         │◀─────────────────────── │          │◀───────────│         │
│         │  file bytes (auth'd)    │          │            │         │
└─────────┘                         └──────────┘            └─────────┘
```

1. Client uploads file via multipart POST to backend
2. Backend generates a presigned URL and uploads to S3-compatible storage
3. Backend returns file metadata to client
4. Client requests file download through backend (authenticated)
5. Backend proxies the file from S3 with auth check — no presigned URL leaks to end users

---

## Deployment Topology

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

## Further Reading

- [Architecture Overview (detailed)](./architecture/overview.md)
- [Backend Architecture](./architecture/backend.md)
- [Frontend Architecture](./architecture/frontend.md)
- [Data Model](./architecture/data-model.md)
- [WebSocket Design](./architecture/websocket.md)
- [Calls Deployment](./architecture/calls-deployment.md)
