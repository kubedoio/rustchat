# RustChat Implementation Gap Analysis

**Date:** 2026-05-22  
**Scope:** Repository-level static review of backend, frontend, push proxy, docs, tests, and deployment assets.  
**Baseline:** "Supported self-hosted team collaboration product", not "complete Mattermost clone".

## Executive Summary

RustChat is substantially implemented as a working collaboration platform. The core product exists: authentication, teams, channels, posts, threads, files, reactions, unreads, presence, search, admin surfaces, SSO/OAuth, calls, push notification plumbing, and broad Mattermost API compatibility are all represented in code.

The project is not yet production-complete. The remaining work is less about inventing the product and more about hardening: production rate/security verification, frontend architecture consolidation, calls/SFU testing, operational packaging, and closing stubbed compatibility/enterprise endpoints.

Estimated completion against a supported self-hosted product target:

| Area | Implemented | Remaining | Status |
|---|---:|---:|---|
| Core messaging product | 80-85% | 15-20% | Functional, needs hardening and polish |
| Mattermost mobile compatibility | 90-95% | 5-10% | Broadly implemented; docs are stale |
| Frontend application | 70-75% | 25-30% | Feature-rich UI, architecture cleanup needed |
| Admin/security features | 60-70% | 30-40% | Many surfaces exist; several controls incomplete |
| Calls and push notifications | 60-70% | 30-40% | Implemented path, low test confidence |
| Plugin/enterprise compatibility | 20-35% | 65-80% | Mostly stubs or narrow compatibility shims |
| Testing and CI confidence | 45-55% | 45-55% | Good base, gaps on critical runtime paths |
| Production operations | 45-55% | 45-55% | Docker/dev ops exist; production runbooks need depth |

Overall estimate: **about 70-75% implemented** for a small-team self-hosted release candidate, with **25-30% remaining** before calling it broadly production-ready. If the target is "full Mattermost-compatible enterprise platform", the implementation is closer to **50-60%** because plugins, LDAP/SAML, compliance, advanced admin, and enterprise workflows remain shallow.

## What Is Implemented

### Backend

The Rust/Axum backend is active and broad. Implemented modules include:

- Native `/api/v1/*` APIs for auth, users, teams, channels, posts, files, search, preferences, activity, calls, admin, SSO, integrations, playbooks, unreads, and health.
- Mattermost-compatible `/api/v4/*` APIs across auth, users, teams, channels, posts, threads, files, emoji, status, config, plugins metadata, calls plugin compatibility, roles, system, preferences, categories, hooks, groups, bots, jobs, terms, and more.
- WebSocket/realtime layer with internal and v4-compatible message shapes.
- PostgreSQL schema with **74 migrations**.
- Redis-backed pieces for realtime/unreads/rate limiting.
- S3-compatible file storage.
- SMTP email subsystem, password reset, email verification, and template infrastructure.
- OAuth/OIDC-related functionality and SSO administration surfaces.
- Push notification service integration plus separate `push-proxy` service.
- Security headers, auth policy, API keys, body limits, and production secret checks.

### Frontend

The Vue/Vite frontend is also substantial:

- Auth flows: login, register, forgot/reset password.
- Main chat workspace: teams, channels, messages, composer, files, threads, reactions, pinned/saved/search panels, members, direct messages.
- Calls UI: active call, incoming call, video call modal, calls settings.
- Admin console: users, teams, permissions, server settings, SSO, email, audit logs, terms, security, integrations, health.
- Settings/profile/preferences UI.
- Shared HTTP client, feature modules, stores, composables, typed entities, and tests.
- Unit test script includes 29 frontend test files; Playwright E2E covers auth, composer, WebSocket disconnect, DM consistency, and settings parity.

### Compatibility

Older repository documentation said mobile-critical Mattermost coverage was **39/41 (95.1%)** as of 2026-03-17. The previously missing endpoints are now present in code and the compatibility docs have been updated:

- `POST /api/v4/emoji` is implemented in `backend/src/api/v4/emoji.rs`.
- `POST /api/v4/posts/search` and `POST /api/v4/teams/{team_id}/posts/search` are implemented in `backend/src/api/v4/posts/search.rs`.

The implementation appears pragmatic rather than fully Mattermost-complete. For example, post search is currently `ILIKE`-based and returns empty match arrays, so it is useful but not a full advanced search engine.

## What Still Needs Implementation

### Critical Production Gaps

1. **Security and abuse controls**
   - Rate limiting now has Redis-backed paths in `backend/src/middleware/rate_limit.rs`, but the full production behavior still needs verification and load/abuse tests.
   - Token generation and Turnstile defaults have been addressed in the current code/config; silent fallback behavior and exposed development services should still be rechecked.

2. **Docker/build correctness**
   - `docker/frontend.Dockerfile` has been moved to `node:24-alpine`.
   - Production Dockerfiles now include health checks; clean Docker builds still need CI/runtime validation.

3. **Operational readiness**
   - A clearly separated production Compose stack now exists in `docker-compose.prod.yml`, but it still needs live deployment validation.
   - No Helm/Kubernetes manifests.
   - No version-controlled monitoring stack.
   - Backup/restore is documented conceptually, but not automated/tested as a product workflow.

### Backend Gaps

- Plugin system is mostly compatibility metadata and 501 responses for upload/install/enable/disable/remove flows.
- LDAP and SAML endpoints are explicit stubs.
- MCP authentication extractor is explicitly Phase 2.
- SES and SendGrid providers are not implemented; SMTP is the practical provider.
- MJML rendering is a placeholder.
- Several v4 enterprise/admin endpoints are compatibility shims rather than complete product features.
- Calls/SFU and realtime internals need more decomposition and tests.
- Large service files still need extraction, especially posts, admin, WebSocket, and calls plugin areas.
- Migrations have no rollback/down strategy.

### Frontend Gaps

- Architecture is split between legacy `frontend/src/stores/*` and newer `frontend/src/features/*`; this creates state synchronization risk.
- `frontend/src/stores/calls.ts`, `frontend/src/composables/useWebSocket.ts`, and composer components are still large and mix transport, domain, and UI concerns.
- E2E coverage does not yet cover the highest-risk user paths: channel/team creation, admin console workflows, calls/video, file upload/download, threads, search, and permission failures.
- Some feature architecture exists but is not consistently wired into the live UI.

### Testing Gaps

Current test footprint is meaningful but not enough for production confidence:

- Backend has 39 Rust test files, but critical realtime, calls/SFU, storage/S3, and background job paths are under-tested or untested.
- Frontend has 29 unit test files and 5 main Playwright specs, but only a small fraction of Vue components are covered.
- Database-dependent integration tests are not consistently part of PR confidence.
- Full-stack compatibility smoke tests are documented but not clearly enforced in ordinary CI.

## Suggested Priority Order

### Phase 1: Release-Blocking Hardening

1. Verify and enforce rate limiting for auth, password reset, and WebSocket routes.
2. Validate clean Docker builds for the default and production Compose files.
3. Add critical-path tests for file upload/download, realtime reconnect, calls signaling, and permissions.
4. Keep compatibility docs synced with the actual implemented endpoints.
5. Add live backup/restore validation for the production Compose stack.

### Phase 2: Product Stabilization

1. Consolidate frontend stores around one canonical architecture.
2. Split large backend services into testable modules.
3. Improve post search beyond basic `ILIKE`, or document it as simple search.
4. Complete production deployment artifacts: prod Compose, backup/restore scripts, monitoring examples.
5. Add CI timeouts and make full-stack smoke tests routine.

### Phase 3: Enterprise/Compatibility Completion

1. Decide whether plugin upload/install is in scope; either implement it or remove public expectations.
2. Implement or explicitly de-scope LDAP, SAML, compliance exports, advanced admin tools, and marketplace-like flows.
3. Add broader Mattermost compatibility contract tests for desktop/mobile clients.
4. Add scale/performance testing for channels, posts, unreads, and WebSocket fanout.

## Documentation Drift To Fix

The docs need a cleanup pass because implementation has moved faster than documentation:

- `docs/repo-current-state.md` has been updated to `v0.4.0`.
- Compatibility docs have been updated to show custom emoji upload and post search as implemented, with search semantics still limited.
- Architecture docs describe a target frontend architecture more than the actual mixed legacy/feature state.
- Data model docs were previously flagged as stale and should be regenerated from migrations/schema.

## Bottom Line

RustChat is past prototype stage. The main collaboration product is implemented enough to use in development, testing, and likely controlled small-team environments. The remaining work is concentrated in production confidence, operational packaging, compatibility completeness, and reducing architectural debt.

Recommended readiness label today: **pre-release / beta**, not stable 1.0.
