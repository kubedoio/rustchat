# RustChat Architecture — Current State Analysis

**Date:** 2026-04-26  
**Scope:** Full-stack architecture review across backend, frontend, database, realtime, and infrastructure  
**Method:** Document review + parallel codebase exploration (6 focused subagents)

---

## Executive Summary

RustChat is a **functionally mature, self-hosted team collaboration platform** with a well-conceived architecture that generally matches its documentation. However, the codebase exhibits signs of **rapid feature growth without corresponding architectural maintenance**. The most critical issues are:

1. **Frontend migration limbo** — A clean feature-based architecture is built but the UI still runs on legacy monolithic stores
2. **Severe documentation drift** — The data model doc is months behind and actively misleading
3. **Backend code organization debt** — Missing repository abstraction, oversized services, and startup logic polluting the router builder
4. **Realtime memory hygiene** — Unbounded growth in WebSocket subscription maps
5. **Production deployment gaps** — Dev-optimized Docker Compose with no K8s manifests, monitoring, or automated backups

| Area | Grade | Summary |
|------|-------|---------|
| Backend API Layer | **B+** | Clean v1/v4 separation, mature WebSocket dual-protocol support, but file bloat and router pollution |
| Backend Services | **B-** | Functional and secure, but missing repository layer, god services, tight AppState coupling |
| Database / Models | **B** | Solid schema with soft deletes and JSONB flexibility, but severe doc drift and 2–3 model-schema mismatches |
| Realtime / WebSocket | **B+** | Mature cluster broadcast, session resumption, presence lifecycle, but memory leaks and v1 robustness gap |
| Frontend | **C+** | Infrastructure for clean architecture exists, but migration never completed; UI runs on legacy stores |
| Infrastructure / Deployment | **B-** | Strong security defaults and CI, but production deployment artifacts are incomplete |

---

## 1. Backend API Layer

### What Works Well

- **Clean v1/v4 separation** — Native and Mattermost-compatible APIs are cleanly isolated; v4 does not leak into v1
- **Shared WebSocket core** — `websocket_core.rs` centralizes auth, presence, limits, and command handling for both endpoints
- **Security-conscious defaults** — Explicit per-route body limits (64KB / 1MB / 50MB), panic-to-JSON recovery, secure auth token transport only
- **v4 compatibility depth** — Session resumption, message replay (128-msg buffer), reconnect snapshots, binary msgpack, Mattermost event name mapping
- **Cluster-aware presence** — Redis-backed connection counting with 90s TTL heartbeats; fails closed when Redis is unavailable

### Concerns

| Severity | Issue | Location | Details |
|----------|-------|----------|---------|
| 🔴 High | File size bloat | `api/admin.rs` (2,591L), `api/oauth.rs` (1,961L), `api/v4/calls_plugin/mod.rs` (4,234L) | Violates "thin handlers" guideline; should be modularized like `v4/channels/` |
| 🔴 High | Startup logic in router builder | `api/mod.rs` | Spawns 5 background workers inside `router()` — anti-pattern |
| 🟡 Medium | v1/v4 auth extractor divergence | `api/auth.rs` vs `api/v4/extractors.rs` | `MmAuthUser` supports cookie auth; `AuthUser` does not. Some v1 endpoints already have MM-compatible routes |
| 🟡 Medium | v4 501 blind spot | `api/v4/mod.rs` | Fallback returns structured 501 but no metrics — gaps are invisible |
| 🟡 Medium | Unauthenticated WS upgrade | `api/v4/websocket.rs` | Accepts connection before auth for MM mobile compatibility |

### Documented vs. Actual

| Claim | Reality | Match |
|-------|---------|-------|
| "Keep handlers thin; place logic in services" | ⚠️ Partial — v4 compat and admin handlers are thick | ⚠️ |
| "Preserve MM API response signatures" | ✅ Extensive `mm::` models, `x-mm-compat` header, `encode_mm_id` | ✅ |
| "All public functions return `Result`" | ✅ `ApiResult<T>` throughout | ✅ |

---

## 2. Backend Services Layer

### What Works Well

- **Good error handling** — `AppError` with `thiserror`, custom error enums for complex services (`EmailServiceError`, `PushNotificationError`)
- **Security-conscious design** — Anti-enumeration in password reset, SSRF protection in webhooks, constant-time token compares
- **Background workers** — Push notifications, outgoing webhooks, Keycloak sync, reconciliation are fire-and-forget
- **Workflow-based email** — Outbox pattern with quiet hours, throttling, locale fallback

### Concerns

| Severity | Issue | Details |
|----------|-------|---------|
| 🔴 High | `posts.rs` is a god service (1,037L) | Handles CRUD, permissions, mentions, activities, push, webhooks, playbooks, DM resurrection, WS broadcast |
| 🔴 High | **No repository layer** | Services mix business logic with raw SQLx queries. Only `PolicyRepository` exists as an outlier |
| 🟡 Medium | `AppState` as service locator | 15+ fields passed everywhere; hard to identify actual dependencies |
| 🟡 Medium | Services construct other services inline | `EmailService::new(db.clone())` called in multiple places instead of shared instance |
| 🟡 Medium | `unreads.rs` complexity | Dual Redis schemas (legacy + v2) + dirty-marker reconciler + DB fallback |
| 🟡 Medium | No service traits / hard to mock | Free functions + concrete structs make unit testing without full DB/Redis impossible |

### Documented vs. Actual

| Claim | Reality | Match |
|-------|---------|-------|
| "Handler Structure: Keep handlers thin; place logic in services" | ✅ Handlers are thin; logic is in services | ✅ |
| "Repository: Data access layer, Service: Business logic" | ❌ No repositories — services do both | ❌ |

---

## 3. Database & Models

### What Works Well

- **Soft delete strategy** — Consistent `deleted_at`, `deleted_by`, `delete_reason` across users, teams, channels, orgs
- **JSONB flexibility** — `props`, `notify_props`, `entity_metadata`, `policy_json` without schema proliferation
- **`sqlx(default)` for evolution** — Non-breaking additive schema changes
- **73 migrations since Jan 2026** — Active, healthy development velocity
- **Good index coverage** — Channel membership, post time-series, session cleanup

### Concerns

| Severity | Issue | Details |
|----------|-------|---------|
| 🔴 High | **Severe documentation drift** | `docs/architecture/data-model.md` is months behind: documents non-existent `sessions` table, wrong column names, missing 10+ entities |
| 🔴 High | Model-schema mismatches | `TeamMember` lacks `presence` column (added Apr 2026); `Post` lacks `has_reactions`; two different `Reaction` structs |
| 🟡 Medium | Epoch timestamp inconsistency | `reactions`, `channel_bookmarks`, `scheduled_posts` use `BIGINT` epoch millis; rest use `TIMESTAMPTZ` + `DateTime<Utc>` |
| 🟡 Medium | `activities` overly restrictive FKs | `channel_id`, `team_id`, `post_id` are `NOT NULL` — DMs or deleted contexts will fail |
| 🟢 Low | `UserRole` enum unused | `User.role` is plain `String`; enum exists but is not used for the DB field |

---

## 4. Realtime / WebSocket System

### What Works Well

- **Dual-protocol maturity** — Internal `WsEnvelope` + Mattermost `WebSocketMessage` with edge mapping layer
- **Cluster broadcast** — Redis pub/sub with echo suppression, auto-reconnect (5s backoff), heartbeat protocol
- **Session resumption (v4)** — `ConnectionStore` with 128-msg ring buffer, 5-min TTL, monotonic `seq` numbers
- **Presence lifecycle** — DB + in-memory + Redis triple-track; manual presence protection; offline only when truly last connection
- **TCP resilience** — Unix-specific `SO_KEEPALIVE` tuning for mobile carrier drops

### Concerns

| Severity | Issue | Details |
|----------|-------|---------|
| 🔴 High | **Unbounded memory growth** | `channel_subscriptions` and `team_subscriptions` in `WsHub` are **never cleaned up** when users disconnect. Code explicitly acknowledges this as a TODO |
| 🔴 High | v1 endpoint lacks actor model | No ping/pong, no write timeouts, no close-code management — relies on OS TCP stack for dead connection detection |
| 🟡 Medium | `broadcast_local` holds connections lock during full broadcast | Can stall `add_connection` / `remove_connection` under high load |
| 🟡 Medium | `ConnectionStore::update_subscriptions` is a no-op | `team_ids` / `channel_ids` fields in `ConnectionState` are never populated |
| 🟡 Medium | Redis subscriber drops messages during reconnect | 5s reconnect window = lost events; no Redis Stream persistence |
| 🟡 Medium | `map_envelope_to_mm` silently drops unmapped events | Compatibility gaps are invisible |
| 🟢 Low | Cluster heartbeat exists but is not scheduled | `send_heartbeat()` defined but never called |

---

## 5. Frontend

### What Works Well

- **Feature module infrastructure is complete** — 15 features with Repository/Service/Store/Handler separation
- **Branded entity IDs** — `UserId`, `ChannelId`, `MessageId` prevent accidental ID mixing
- **Railway-oriented `Result<T,E>`** — Clean error handling with `success()` / `failure()` helpers
- **Custom HTTP client** — Fetch-based with interceptors, timeout, upload progress
- **Readonly store exports** — Feature stores use `readonly()` for unidirectional data flow

### Concerns

| Severity | Issue | Details |
|----------|-------|---------|
| 🔴 High | **Feature stores are built but unused by UI** | Components import from `stores/` (legacy), never from `features/*/stores/`. The new architecture is effectively dead code from the UI perspective |
| 🔴 High | **Dual WebSocket system** | Legacy `useWebSocket.ts` (864L, actively used) vs new `WebSocketManager.ts` (never wired). `registerWebSocketHandlers()` is never called |
| 🟡 Medium | Legacy stores are massive and violate thin-store pattern | `stores/calls.ts` (1,005L) has WebRTC logic; `stores/messages.ts` (709L) has author resolution, reaction merging |
| 🟡 Medium | Cross-dependency chaos | Legacy stores import feature stores; feature stores import legacy stores — circular dependency risk |
| 🟡 Medium | Two `Message` types coexist | `core/entities/Message.ts` (new) and `stores/messages.ts` `Message` (legacy) with different shapes |
| 🟡 Medium | Components not co-located with features | `components/` organized by type/topic, not by feature — contradicts documented architecture |
| 🟡 Medium | Permissions feature incomplete | Only `capabilities.ts` + tests; no repository, service, or store |

### Documented vs. Actual

| Claim | Reality | Grade |
|-------|---------|-------|
| "Code organized by feature, not by type" | Components are by type; features lack components | **C** |
| "Repository → Service → Store → Handler" | ✅ Built in all mature features | **A** |
| "Store: State management — thin stores" | Legacy stores violate this; feature stores are correct | **D** / **A** |
| "Use branded types for IDs" | ✅ Used in `core/entities/` | **A** |

---

## 6. Infrastructure & Deployment

### What Works Well

- **Security-first configuration** — `.env.example` warns against insecure defaults, production secret validation, OAuth code exchange (no URL tokens)
- **Push-proxy isolation** — Separate service, Dockerfile, compose file, read-only credential mounts
- **CI/CD rigor** — Backend CI with sccache + mold, frontend dependency review, compat diff workflow, multi-arch Docker publish
- **Compatibility-first culture** — Governance prohibits agents from touching `api/v4/` without co-approval; smoke tests verify contracts
- **Feature flags for rollout** — Unread v2, post unread WS, etc. are gated with explicit "do not enable in production" warnings

### Concerns

| Severity | Issue | Details |
|----------|-------|---------|
| 🟡 Medium | No production Docker Compose | Same `docker-compose.yml` for dev and prod — exposes Postgres (5432), Redis (6379), RustFS (9000/9001) |
| 🟡 Medium | Missing `HEALTHCHECK` in main Dockerfiles | Only `backend.Dockerfile.local` has one; production images lack it |
| 🟡 Medium | No Kubernetes / Helm manifests | Docs mention K8s support but no official charts exist |
| 🟡 Medium | No monitoring stack | Prometheus mentioned in tech stack but no `docker-compose.monitoring.yml` or Grafana configs |
| 🟡 Medium | CI does not run full-stack E2E | `mm_compat_smoke.sh` and `mm_mobile_smoke.sh` are not run in CI |
| 🟡 Medium | No automated backup/DR | Manual `pg_dump` mentioned; no cron or automated scripts |
| 🟢 Low | Frontend uses OpenResty unnecessarily | `nginx.conf` does not use Lua features; adds image size without benefit |

---

## Cross-Cutting Themes

### Theme 1: Migration Limbo
The most pervasive issue across the codebase is **partial migrations that were never completed**:
- **Frontend:** Feature architecture built but UI still on legacy stores
- **Backend:** One `PolicyRepository` exists as a proof of concept, but no broader repository layer adoption
- **WebSocket:** New `WebSocketManager` built but never wired into the app
- **Unread system:** v2 hash schema built alongside legacy string keys

### Theme 2: Documentation Drift
Multiple architecture documents are stale or misleading:
- `docs/architecture/data-model.md` — severely outdated (wrong tables, wrong columns, missing entities)
- `docs/architecture/frontend.md` — describes the target state, not the actual mixed legacy/feature reality
- Migration naming docs claim "numbered sequentially" but actual files use timestamps

### Theme 3: Rapid Growth Outpacing Organization
The codebase grew from a clean prototype to a feature-rich platform without corresponding organizational refactoring:
- `posts.rs` (1,037L) accumulated 7+ responsibilities
- `admin.rs` (2,591L) and `calls_plugin/mod.rs` (4,234L) are single files doing the work of modules
- 73 migrations in ~4 months indicates very high velocity, but some schema decisions (epoch timestamps, `activities` FKs) suggest insufficient design review

### Theme 4: Production Readiness Gaps
The project is clearly optimized for development velocity and small-scale Docker Compose deployments. Production-grade concerns (network isolation, K8s, monitoring, automated backups, full-stack CI) are documented as concepts but not materialized as version-controlled infrastructure.

---

## Prioritized Recommendations

### Immediate (This Sprint)
1. **Fix model-schema mismatches** — Add `presence` to `TeamMember`, `has_reactions` to `Post`, reconcile `Reaction` structs
2. **Wire new WebSocket manager** — Replace legacy `useWebSocket.ts` with `WebSocketManager.ts` + feature handlers
3. **Add `HEALTHCHECK` to production Dockerfiles**

### Short-Term (Next 2 Sprints)
4. **Extract repository layer from `posts.rs`** — Separate data access from business logic; use as template for other services
5. **Clean up `WsHub` subscription maps** — Add periodic cleanup task or track subscriptions per-connection
6. **Rewrite `docs/architecture/data-model.md`** — Sync with actual schema
7. **Add `docker-compose.prod.yml`** — Remove exposed DB/Redis ports, add resource limits

### Medium-Term (Next Quarter)
8. **Complete frontend migration** — Migrate component imports from `stores/*` to `features/*`
9. **Extract `api/mod.rs` startup logic** — Move background worker spawns to `main.rs` or dedicated startup module
10. **Add Prometheus + Grafana stack** — `docker-compose.monitoring.yml` with basic dashboards
11. **Add full-stack E2E to CI** — Spin up compose stack, run smoke tests

### Long-Term
12. **Kubernetes Helm chart** — For production deployments
13. **Unify timestamp conventions** — Migrate epoch-millis tables to `TIMESTAMPTZ`
14. **Service traits for testability** — Define async traits for core services to enable mocking

---

*Analysis conducted via parallel codebase exploration of backend API, services, database, realtime, frontend, and infrastructure layers, cross-referenced with existing architecture documentation.*
