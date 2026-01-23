# rustchat Architecture

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Web / SPA Client                       │
└─────────────────────────┬───────────────────────────────────┘
                          │ REST / WebSocket
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    rustchat API Server                      │
│                    (Axum + Tokio)                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │
│  │  Auth   │  │ Channels│  │  Posts  │  │   Real-time     │ │
│  │ Module  │  │ Module  │  │ Module  │  │   WebSocket Hub │ │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────────┬────────┘ │
│       │            │            │                │          │
│  ┌────▼────────────▼────────────▼────────────────▼────────┐ │
│  │                    Service Layer                       │ │
│  └────────────────────────┬───────────────────────────────┘ │
└───────────────────────────┼─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
   ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
   │Postgres │         │  Redis  │         │   S3    │
   │ (Data)  │         │ (Cache) │         │ (Files) │
   └─────────┘         └─────────┘         └─────────┘
```

## Core Components

### API Layer (`src/api/`)

HTTP request handling using Axum:
- Route definitions
- Request validation
- Response formatting
- Middleware (auth, logging, CORS)

### Service Layer

Business logic modules:
- **auth** — Authentication, JWT tokens, password hashing
- **orgs** — Organization management
- **teams** — Team membership and permissions
- **channels** — Channel CRUD and access control
- **posts** — Message handling, threads, reactions
- **files** — Upload/download, S3 integration
- **realtime** — WebSocket connections, event fan-out

### Data Layer (`src/db/`)

PostgreSQL with SQLx:
- Connection pool management
- Compile-time checked queries
- Migration runner

### Configuration (`src/config/`)

Environment-based configuration using the `config` crate.

### Error Handling (`src/error/`)

Structured error types with HTTP status mapping.

### Telemetry (`src/telemetry/`)

Structured JSON logging with `tracing`.

## Data Flow

### HTTP Request

```
Request → Middleware → Router → Handler → Service → Repository → Database
                                   ↓
Response ← JSON Serialization ← Result
```

### WebSocket Event

```
Client ← WebSocket Hub ← Event Publisher ← Service ← Database Change
```

## Database Schema

Key tables:
- `organizations` — Multi-tenant organizations
- `users` — User accounts (humans and bots)
- `teams` — Teams within organizations
- `channels` — Communication channels
- `posts` — Messages and threads
- `files` — File metadata

## Scalability

- **Stateless API servers** — Horizontal scaling behind load balancer
- **Redis pub/sub** — Cross-instance event propagation
- **Connection pooling** — Efficient database connections
- **Async I/O** — Non-blocking operations throughout
