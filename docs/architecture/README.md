# Architecture Documentation

This section documents the system architecture of RustChat.

## Overview

RustChat is a self-hosted team collaboration platform built with:
- **Backend:** Rust (Axum + Tokio + SQLx)
- **Frontend:** Svelte 5 + TypeScript
- **Database:** PostgreSQL 16+
- **Cache/Events:** Redis 7+
- **Storage:** S3-compatible (RustFS)

## Documentation Sections

### System Architecture
- [Overview](./overview.md) - High-level system design and data flow
- [Backend Architecture](./backend.md) - Rust backend deep dive
- [Frontend Architecture](./frontend.md) - Svelte frontend structure
- [WebSocket Architecture](./websocket.md) - Real-time communication
- [Calls Deployment](./calls-deployment.md) - Voice/video calls infrastructure

### Data Model
- [Database Schema](./data-model.md) - PostgreSQL schema overview
- Entity relationships
- Migration strategy

## Key Design Decisions

- **Dual API Strategy:** Native `/api/v1` + Mattermost-compatible `/api/v4`
- **Separate Push Proxy:** FCM/APNS credentials isolated in dedicated service
- **Redis for Clustering:** Cross-instance event fan-out without sticky sessions
- **SQLx Compile-Time Checks:** Schema/query validation at compile time
- **Feature-Based Frontend:** Domain-driven module organization

## Diagrams

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
│  /api/v4/*  ──── Mattermost-compatible API (mobile/desktop)        │
└─────────────────────────┬──────────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
   PostgreSQL           Redis           S3-compatible
   (primary data)    (events/cache)     (file storage)
```

---

*For API compatibility details: See [Compatibility](../development/compatibility.md).*
