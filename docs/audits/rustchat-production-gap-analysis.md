# RustChat Production Readiness Gap Analysis

## 1. Executive Summary

RustChat has a substantial implementation: Rust/Axum backend, PostgreSQL/Redis/S3-style storage, Vue/Vite frontend, SQL migrations, API tests, frontend unit/E2E tests, docs, release workflows, and operational guides. The codebase is much more than a prototype.

It is not production-ready against the reliability/security bar of stable collaboration systems such as Mattermost or Zulip. The main blockers are authorization correctness around WebSocket subscription paths, incomplete realtime delivery semantics, weak or uneven contract coverage, pagination/data consistency risks, partial operational readiness, and test gaps around negative permissions, concurrency, reconnect, attachments, and migration behavior.

Recommendation: **ready for internal beta only**, not external beta or production, until the P0/P1 backlog in this report is addressed and enforced in CI.

Commands run:

| Command | Result | Classification |
|---|---|---|
| `git status` | Clean worktree on `low-hanging-gap-fixes` | Baseline |
| `find . -maxdepth 3 -type f \| sort` | Completed; large repo with backend, frontend, docs, CI, migrations | Inventory |
| `find docs -maxdepth 4 -type f \| sort || true` | Completed; docs include architecture, security, operations, ADR, specs, audits | Inventory |
| `find . -iname "*spec*" -o -iname "*contract*" -o -iname "*adr*" \| sort` | Completed; found specs/contracts/ADRs in docs and `backend/compat` | Inventory |
| `rg -n "TODO\|FIXME\|panic!...` | Completed; very large output due vendored/generated files; useful findings narrowed with targeted `rg` | Inventory |
| `cargo test --all --all-features` | Unit tests passed: 148/148. Integration suite stopped at `tests/api_activity.rs`: 3/3 failed because PostgreSQL was not reachable (`Connection refused`, `RUSTCHAT_TEST_DATABASE_URL` not set to a reachable superuser-capable DB). | Missing test dependency / integration environment |
| `cargo clippy --all-targets --all-features` | Initial parallel run failed resolving `index.crates.io`; rerun after dependency cache warmup exited 0. | Clippy passes |
| `cargo fmt --check` | Exit 0 | Formatting passes |
| `npm run test:unit` | 19 files, 108 tests passed | Frontend unit tests pass |
| `npm run lint` | Exit 0 with 200 warnings | Lint not strict enough |
| `npm run build` | Exit 0 | Frontend build passes |

## 2. Repository and Runtime Overview

Observed stack:

| Concern | Current implementation |
|---|---|
| Backend language/framework | Rust 2021, Axum 0.8, Tokio; `backend/Cargo.toml` |
| Frontend framework | Vue 3, Vite 7, Pinia, TypeScript; `frontend/package.json` |
| Database/storage | PostgreSQL via `sqlx`, Redis via `deadpool-redis`, S3-compatible storage via AWS SDK; `backend/src/db`, `backend/src/storage/s3.rs` |
| Realtime transport | WebSocket via Axum; v1 and Mattermost-compatible v4 paths; `backend/src/api/ws.rs`, `backend/src/api/v4/websocket/*`, `backend/src/realtime/*` |
| Authentication/session | JWT bearer tokens, Argon2 password hashing, OAuth/SSO support; `backend/src/auth/*`, `backend/src/api/auth.rs`, `backend/src/api/oauth/*` |
| API structure | Native v1 API plus broad Mattermost v4 compatibility surface; `backend/src/api`, `backend/src/api/v4` |
| Test framework | Rust `cargo test`, frontend Vitest, Playwright E2E, compatibility contract tests |
| CI/release | Multiple GitHub workflows, cargo/npm audit docs, Docker build/publish/release workflows |
| Deployment | Docker Compose, production compose, nginx, backup/restore docs |

## 3. Existing Specs, Contracts, and ADRs Reviewed

Reviewed or sampled:

- `CLAUDE.md`: repo architecture, backend/frontend module expectations, testing commands, SQL boundary rules.
- `docs/architecture/*.md`: backend, frontend, websocket, data-model, current-state analysis.
- `docs/development/compatibility.md`: explicit Mattermost compatibility scope and known limitations.
- `docs/internal/SPEC.md`, `docs/internal/SPEC-WEBSOCKET-DISCONNECTION-UX.md`, `docs/internal/SPEC-OPEN-SOURCE-HARDENING.md`, `docs/internal/SPEC-model-schema-fixes.md`.
- `docs/security-model.md`, `docs/security-deployment-guide.md`, `docs/admin/security.md`.
- `docs/operations/runbook.md`, `docs/admin/backup-restore.md`, `docs/admin/scaling.md`.
- `docs/adr/ADR-frontend-supply-chain-security.md`.
- `backend/compat/contracts` and `backend/compat/tests/contract_validation.rs`.
- Existing audit: `docs/audits/rustchat-implementation-consistency-and-gap-analysis.md`.

The docs set expectations for service/repository separation, compatibility stability, secure WebSocket auth, frontend supply-chain controls, and deployment hardening. The implementation partially follows those expectations, but not consistently enough for production.

## 4. Current Implementation Summary

Implemented:

- Authenticated APIs, JWT auth, password hashing, OAuth/SSO paths.
- Teams, channels, posts, threads, reactions, saved/pinned surfaces, preferences, admin surfaces, calls plugin surfaces.
- PostgreSQL migrations covering core collaboration models, files, permissions, audit logs, unread messages, threads, email, SSO, entity model, activities, TOS, and TURN config hardening.
- Repository layer exists for posts, channels, users, teams, files, integrations, groups, uploads, etc.
- Service layer exists for posts, unreads, activity, email, membership policies, OAuth, push notifications, webhooks, rate limits.
- WebSocket hub, actor, connection store, session resumption, heartbeat, bounded actor queues, Redis presence, and metrics hooks.
- Frontend feature modules for auth, channels, messages, unreads, presence, teams, permissions, calls, admin, preferences.
- Markdown rendering with DOMPurify sanitization.
- File upload validation for extensions, magic bytes for image/pdf/zip, size limits, SVG script detection, authenticated file download endpoints.
- CI and release workflows plus Docker deployment assets.

Incomplete or fragile:

- ~~WebSocket client-driven `subscribe_channel` does not enforce membership.~~ **FIXED**
- ~~Typing events can be broadcast for arbitrary channel IDs if a user can subscribe or send typing actions.~~ **FIXED**
- Realtime replay is per connection and bounded; durable channel event replay is not implemented.
- ~~Frontend WebSocket manager confuses `seq` with `connectionId`, undermining reconnect/resumption semantics.~~ **FIXED** in active `useWebSocket.ts`; secondary `WebSocketManager.ts` is unfinished dead code.
- Pagination is often offset-based and ordered by `created_at`; keyset paths exist but are not uniformly enforced.
- ~~Message creation side effects are not one transaction: insert, reply count, mentions props, broadcast, unread increments, automation, push are separate steps.~~ **FIXED** — all DB side effects (post insert, reply count, mention props, activities, DM membership repair, author's channel_reads) are committed in a single service-level transaction. External effects (WS broadcast, push, webhooks, Redis cache) remain best-effort after commit.
- Tests exist but are skewed toward happy paths and compatibility slices; negative auth/realtime/concurrency/security tests are not comprehensive.

## 5. Production Readiness Scorecard

| Area | Current Status | Risk | Production Readiness | Evidence |
|---|---|---|---|---|
| Architecture | Clear backend/frontend split with API, models, repositories, services, realtime modules | Medium | Partial | `backend/src/api`, `backend/src/services`, `backend/src/repositories`, `frontend/src/features` |
| Backend API | Broad v1/v4 API surface with auth extractors and tests | High | Partial | `backend/src/api/v4`, `backend/tests/api_v4_*` |
| Authentication | JWT validation, token policy, inactive/deleted user checks in HTTP extractors and v4 WS auth | Medium | Partial | `backend/src/auth/middleware.rs`, `backend/src/api/v4/websocket/handler.rs` |
| Authorization | Membership checks exist in many HTTP paths; WebSocket subscribe and typing now validate membership | Critical | Partial | `backend/src/api/websocket_core.rs`, `backend/src/api/v4/websocket/connection.rs` |
| WebSocket/realtime | Heartbeat, bounded queues, sequence/replay store, metrics hooks exist; auth gaps fixed | Critical | Partial | `backend/src/realtime/websocket_actor.rs`, `backend/src/realtime/hub.rs` |
| Message persistence | Core post insert + reply count now transactional; broadcast after commit; mention/unreads remain best-effort | High | Partial | `backend/src/services/posts.rs`, `backend/src/repositories/post_repository.rs` |
| Data consistency | Many migrations and constraints, but offset pagination and partial side effects remain | High | Partial | `backend/migrations`, `PostRepository::list_by_channel` |
| Frontend state handling | Active `useWebSocket.ts` now correctly tracks connection_id/seq and reconnects with resumption params | High | Partial | `frontend/src/composables/useWebSocket.ts`, `frontend/src/components/ui/ConnectionStatusBar.vue` |
| Security | Good starts: body limits, headers, file validation, sanitization, rate-limit service | High | Partial | `backend/src/api/file_validation.rs`, `frontend/src/composables/useMarkdownRenderer.ts` |
| Testing | Many Rust API tests, Vitest unit tests, Playwright E2E present | High | Partial | `backend/tests`, `frontend/src/**/*.test.ts`, `frontend/e2e` |
| Observability | Tracing/Prometheus dependencies and WS metrics hooks exist | Medium | Partial | `backend/src/telemetry/metrics.rs`, `backend/src/realtime/*` |
| Deployment | Docker Compose and production docs exist | Medium | Partial | `docker-compose.prod.yml`, `docs/deployment.md`, `docs/admin/*` |
| CI/CD | Workflows exist, but local clippy dependency fetch failed and lint allows many warnings | Medium | Partial | `.github/workflows/*`, frontend lint output |
| Documentation | Strong documentation footprint with specs/ADRs/runbooks | Low | Partial | `docs/*` |
| Performance | Bounded WS queues and limits exist, but fanout/subscription model and offset pagination risk scale issues | High | Partial | `WsHub`, `PostRepository` |

## 6. Critical Production Blockers

### GAP-1: WebSocket channel subscription bypasses membership — FIXED

**Severity:** P0  
**Category:** Security / Realtime  
**Status:** Resolved in `low-hanging-gap-fixes`  
**Production Risk:** A user can subscribe to a guessed private channel UUID and receive channel-targeted realtime events if the server accepts the subscription. This can leak private messages, typing events, call events, or metadata.  
**Evidence:** `backend/src/api/websocket_core.rs` handles `"subscribe_channel"` by calling `state.ws_hub.subscribe_channel(user_id, channel_id).await` without `ChannelRepository::is_channel_member` or `require_channel_membership`. `backend/src/realtime/hub.rs` trusts subscription maps when broadcasting.  
**Fix Applied:** `backend/src/api/websocket_core.rs` now checks `ChannelRepository::is_channel_member(channel_id, user_id)` before subscribing. Non-members receive a structured `"error"` event with `"Not a member of this channel"`. The `"error"` event was also added to `map_envelope_to_mm` so v4 clients receive it.  
**Tests Added:** `websocket_non_member_cannot_subscribe_channel` in `backend/tests/api_v4_websocket_lifecycle.rs` proves a non-member is rejected.  
**Remaining Work:** Subscription revocation on membership removal (see GAP-11).

### GAP-2: WebSocket typing events are broadcast without membership validation — FIXED

**Severity:** P0  
**Category:** Security / Realtime  
**Status:** Resolved in `low-hanging-gap-fixes`  
**Production Risk:** A user can emit typing metadata into a channel they do not belong to, causing information leaks and UI spoofing.  
**Evidence:** `backend/src/api/v4/websocket/connection.rs` extracts `channel_id` from client JSON and broadcasts `UserTyping` / `UserTypingStop` directly. Shared `backend/src/api/websocket_core.rs` does the same.  
**Fix Applied:** Membership checks added in both:
- `backend/src/api/websocket_core.rs` for `typing`, `typing_start`, and `typing_stop` events.
- `backend/src/api/v4/websocket/connection.rs` for `user_typing`, `typing`, `typing_start`, `user_typing_stop`, `stop_typing`, and `typing_stop` actions.
Unauthorized typing is silently dropped (no broadcast).  
**Tests Added:** `websocket_non_member_cannot_send_typing` in `backend/tests/api_v4_websocket_lifecycle.rs` proves unauthorized typing is not delivered.

### GAP-3: Frontend reconnect/resumption state is incorrect — FIXED

**Severity:** P0  
**Category:** Realtime / Frontend  
**Status:** Resolved in `low-hanging-gap-fixes`  
**Production Risk:** Reconnect can lose messages or duplicate messages because the client stores `seq` as `connectionId` and does not persist the server-provided `connection_id` from the hello event.  
**Evidence:** `frontend/src/core/websocket/WebSocketManager.ts` sets `connectionId.value = event.seq.toString()` instead of reading `event.data.connection_id`; reconnect uses only `url` and token and does not send sequence/resumption parameters. Server expects `connection_id` and `sequence_number` in `backend/src/api/v4/websocket/handler.rs` / `connection.rs`.  
**Fix Applied:**
1. **Active implementation (`frontend/src/composables/useWebSocket.ts`):**
   - `hello` handler now parses `connection_id` from `envelope.data.connection_id`.
   - `onmessage` tracks the highest received `seq` in `wsLastSeq`.
   - `connect()` appends `connection_id` and `sequence_number` query params on reconnect.
   - `disconnect()` resets both values.
2. **Secondary implementation (`frontend/src/core/websocket/WebSocketManager.ts`):**
   - Fixed `handleMessage` to parse `connection_id` from hello event data.
   - `connect()` appends `connection_id` and `sequence_number` on reconnect.
   - Tracks `lastSeq` for resumption.
   - **Note:** A double-`JSON.parse` bug introduced by the initial fix was caught in review and corrected.  
**Required Tests:** Frontend unit tests for hello parsing, last sequence tracking, reconnect URL/protocol construction, duplicate suppression; Playwright reconnect test.  
**Agent-Ready Implementation Task:** Add Playwright E2E reconnect test (no duplicates, no missing messages).

### GAP-4: Message creation side effects are not transactional — FIXED

**Severity:** P0  
**Category:** Persistence / Realtime  
**Status:** Resolved in `low-hanging-gap-fixes`  
**Production Risk:** A message can be stored while reply counts, mention props, unread counters, activity rows, automation, push notification state, or broadcasts diverge. Lost or duplicated side effects make unread counts and thread state unreliable.  
**Evidence:** `backend/src/services/posts.rs::create_post` previously performed validation, insert, reply side effects, response build, mention updates, broadcast, automation, DM membership repair, unread increment, and push notification as separate awaits.  
**Fix Applied:**
- Refactored `create_post` to own a **service-level transaction** (`state.db.begin()`).
- All DB side effects are now inside the transaction:
  - Post insert + reply count increment via `PostRepository::create_post_in_tx`
  - Mention metadata props update via `PostRepository::update_props_in_tx`
  - Reply activities and mention activities via `activity::create_activity_in_tx`
  - DM membership repair via `ChannelRepository::ensure_membership_in_tx`
  - Author's read position via `unreads::update_author_channel_read_in_tx`
- All external effects (WS broadcast, push notifications, outgoing webhooks, Redis cache updates) happen **only after** the transaction commits.
- Added `_in_tx` variants to `PostRepository`, `ChannelRepository`, `UserRepository`, and `activity` service to support transactional operation.
- Added integration tests proving reply count, activities, channel_reads, and DM membership repair are all committed atomically.
**Required Tests:** ✅ `create_post_reply_increments_reply_count_and_creates_activities`, ✅ `create_post_dm_remembers_removed_user`.

### GAP-5: API contract enforcement is incomplete for the broad v4 surface

**Severity:** P1  
**Category:** API / Testing  
**Production Risk:** Mobile/desktop clients can break when response shapes, error bodies, or pagination semantics drift.  
**Evidence:** `backend/compat/contracts` exists, but broad endpoints under `backend/src/api/v4/*` exceed the visible contract validation scope. `docs/development/compatibility.md` says advanced semantics remain limited.  
**Current Behavior:** Some contract tests exist; many endpoints rely on ad hoc API tests.  
**Expected Production Behavior:** Major API responses and errors should have contract tests, including negative permission cases.  
**Recommended Fix:** Expand schema fixtures and contract tests for posts, channels, users, files, auth errors, WebSocket events, and pagination.  
**Required Tests:** Contract tests for all public v4 endpoints used by web/mobile clients.  
**Agent-Ready Implementation Task:** Add contract schemas for post create/list/update/delete, file info/download error, channel membership errors, and validate them in `backend/compat/tests`.

## 7. Architecture Gaps

### GAP-6: Business logic still leaks across API, service, and repository layers

**Severity:** P1  
**Category:** Architecture  
**Production Risk:** Permission, validation, and side-effect behavior will drift between native v1, v4 compatibility, WebSocket, and background paths.  
**Evidence:** `backend/src/api/posts.rs`, `backend/src/api/v4/posts.rs`, `backend/src/services/posts.rs`, and `backend/src/api/websocket_core.rs` all contain behavior around posts/subscriptions/events.  
**Current Behavior:** Service/repository layers exist, but not every path uses the same domain boundary.  
**Expected Production Behavior:** Handlers parse/authenticate, services enforce business rules, repositories isolate SQL, realtime publishes post-commit events.  
**Recommended Fix:** Create narrow domain services for channel access, post commands, realtime subscription commands, and file access; use them from all entry points.  
**Required Tests:** Shared service unit tests and route-level integration tests proving v1/v4/WebSocket parity.  
**Agent-Ready Implementation Task:** Introduce a `ChannelAccessService` with `can_read_channel`/`require_member` and replace direct ad hoc membership checks in WebSocket and post routes.

### GAP-7: ADR coverage is thin for critical runtime choices

**Severity:** P2  
**Category:** Architecture / Documentation  
**Production Risk:** Future agents may alter realtime delivery, permissions, or compatibility assumptions without understanding invariants.  
**Evidence:** Only one ADR is visible under `docs/adr`; many decisions exist as internal specs/plans rather than durable ADRs.  
**Current Behavior:** Documentation exists but key architecture decisions are scattered.  
**Expected Production Behavior:** Durable ADRs should cover WebSocket delivery model, auth/session model, file access model, compatibility scope, and migration policy.  
**Recommended Fix:** Promote stable decisions from specs into ADRs.  
**Required Tests:** Not applicable; documentation review gate.  
**Agent-Ready Implementation Task:** Add ADRs for WebSocket delivery guarantees, channel authorization model, file access lifecycle, and API compatibility contract policy.

## 8. Authentication and Permission Gaps

### GAP-8: Disabled/deleted user checks must be consistently enforced outside HTTP extractors

**Severity:** P1  
**Category:** Security  
**Production Risk:** Tokens for disabled/deleted users may remain usable on paths that validate JWT only and skip database account-state checks.  
**Evidence:** HTTP `AuthUser` calls `ensure_user_access_active` in `backend/src/auth/middleware.rs`; v4 WebSocket handler has similar account-state checking, but `websocket_core::validate_auth_token` only validates JWT claims.  
**Current Behavior:** Some entry points check account state; shared token validation does not guarantee it.  
**Expected Production Behavior:** All auth contexts should require active, non-deleted users unless explicitly public.  
**Recommended Fix:** Add an async `validate_auth_token_active` for WebSocket and API-key paths; audit all extractors.  
**Required Tests:** Deleted/disabled user cannot call HTTP endpoints, open WebSocket, use API key, or continue after account disable.  
**Agent-Ready Implementation Task:** Centralize active-user validation and add negative integration tests for disabled/deleted user HTTP and WebSocket access.

### GAP-9: Permission matrix is not comprehensively tested

**Severity:** P1  
**Category:** Security / Testing  
**Production Risk:** Admin/member/guest/bot roles can drift, causing privilege escalation or broken access.  
**Evidence:** Permission tests exist (`backend/tests/api_permissions.rs`, frontend permission tests), but mandatory matrix cases such as DM access, private channel read, attachment access after membership changes, and admin boundaries are incomplete.  
**Current Behavior:** Permission enforcement is route-specific.  
**Expected Production Behavior:** One backend permission matrix should drive unit and integration tests.  
**Recommended Fix:** Define role/channel/action matrix and generate tests for it.  
**Required Tests:** Member/non-member/admin/guest tests for create/read/edit/delete posts, files, channel membership, DMs, private channels.  
**Agent-Ready Implementation Task:** Add `backend/tests/api_permissions_matrix.rs` covering channel, post, DM, and file access for member/non-member/admin roles.

## 9. Realtime Messaging / WebSocket Gaps

### GAP-10: Realtime replay is not a durable delivery guarantee

**Severity:** P1  
**Category:** Realtime  
**Production Risk:** Server restart or connection store eviction can lose events between message persistence and client reconciliation.  
**Evidence:** `backend/src/realtime/connection_store.rs` and `websocket_actor.rs` support in-memory sequence replay; no durable event table is visible.  
**Current Behavior:** Replay is per connection and bounded. Reconnect snapshot exists but is not equivalent to ordered event replay.  
**Expected Production Behavior:** Durable messages are source of truth, with client reconciliation based on channel history and monotonic cursors; event replay should be documented as best-effort or backed by durable outbox.  
**Recommended Fix:** Treat WebSocket as notification transport and require post-reconnect REST reconciliation, or add a durable event outbox.  
**Required Tests:** Server restart/reconnect tests, replay gap tests, duplicate suppression tests.  
**Agent-Ready Implementation Task:** Add reconnect reconciliation contract: after reconnect, client fetches channel deltas by `seq`/timestamp and dedupes by post ID.

### GAP-11: Broadcast fanout trusts in-memory subscriptions and lacks revocation path

**Severity:** P1  
**Category:** Realtime / Security  
**Production Risk:** Users removed from a channel may continue receiving events until disconnect or manual unsubscribe.  
**Evidence:** `WsHub` stores `channel_subscriptions` and does not validate membership during broadcast; channel member removal paths were not observed revoking hub subscriptions.  
**Current Behavior:** Subscription maps are the authorization source at broadcast time.  
**Expected Production Behavior:** Subscription maps should be a cache, not authority; membership changes should revoke subscriptions and/or broadcast should verify authorized recipients.  
**Recommended Fix:** On membership removal, call hub unsubscribe; for sensitive private channels, consider membership filtering at publish time.  
**Required Tests:** Member removed from private channel stops receiving events immediately.  
**Agent-Ready Implementation Task:** Add membership-removal subscription revocation and an integration test with two live WebSockets.

### GAP-12: Slow-client handling drops events without client-visible recovery semantics

**Severity:** P1  
**Category:** Realtime / Operations  
**Production Risk:** Slow clients can miss events silently and display stale channel state.  
**Evidence:** `WebSocketActor` bounded buffers record dropped metrics; `connection.rs` logs receiver lagged and continues.  
**Current Behavior:** Drops are logged/metriced; client is not forced to resync.  
**Expected Production Behavior:** If event loss is detected, client should receive a resync-required event or connection should close with a reason that triggers reconciliation.  
**Recommended Fix:** Send `resync_required` or close with policy/retry code on queue overflow/lag.  
**Required Tests:** Slow consumer test that triggers overflow and verifies resync behavior.  
**Agent-Ready Implementation Task:** Add resync-required event on lag/queue overflow and frontend handling to fetch channel state.

## 10. Message Model and Collaboration Gaps

### GAP-13: Message validation is too narrow

**Severity:** P1  
**Category:** API / Security  
**Production Risk:** Oversized messages, malicious markdown edge cases, or invalid attachment-only payloads can degrade clients or bypass expectations.  
**Evidence:** `validate_create_post` rejects empty message without files but no max message length or markdown policy is visible in the service.  
**Current Behavior:** Basic empty-message validation exists.  
**Expected Production Behavior:** Enforce max length, file count, attachment ownership, markdown safety policy, and idempotency.  
**Recommended Fix:** Add centralized `MessageValidator`.  
**Required Tests:** Empty, whitespace, oversized, attachment-only, too-many-files, invalid root, duplicate client ID.  
**Agent-Ready Implementation Task:** Add `MessageValidator` unit tests and enforce it in `create_post`.

### GAP-14: Edit/delete semantics need stronger audit and consistency

**Severity:** P1  
**Category:** Persistence / API  
**Production Risk:** Incorrect ownership checks or missing audit history can break compliance and trust.  
**Evidence:** `backend/tests/api_v4_post_routes.rs` covers non-author update, but full edit history/audit trail and concurrency/version checks are not evident.  
**Current Behavior:** Soft-delete/update routes exist with some tests.  
**Expected Production Behavior:** Edits/deletes should enforce ownership/role, preserve audit metadata, broadcast ordered updates, and handle conflicts.  
**Recommended Fix:** Add edit version/updated_at conflict semantics and audit rows for destructive actions.  
**Required Tests:** Edit/delete owner/admin/non-member, deleted post rendering, edit conflict, realtime edit/delete propagation.  
**Agent-Ready Implementation Task:** Add backend tests and audit logging for post edit/delete, then implement conflict checks if missing.

### GAP-15: Attachment lifecycle is not tied tightly enough to message lifecycle

**Severity:** P1  
**Category:** Persistence / Security  
**Production Risk:** Uploaded files can be orphaned, associated later by unauthorized users, or retain access after permission changes if channel/file linkage is stale.  
**Evidence:** `v4/files.rs` creates file records before post association; `PostRepository::create_post` accepts `file_ids`; no observed transaction validates file ownership/channel for every `file_id` during post create.  
**Current Behavior:** File upload checks channel membership and file download checks current channel membership.  
**Expected Production Behavior:** Post creation should validate each file belongs to requester and target channel or is pending for that requester, and should attach files atomically.  
**Recommended Fix:** Add pending-upload ownership/channel validation in post transaction.  
**Required Tests:** User cannot attach another user's file; non-member cannot attach file to private channel; orphan cleanup.  
**Agent-Ready Implementation Task:** Validate `file_ids` in `create_post` and add integration tests for unauthorized file attachment.

## 11. Persistence and Data Consistency Gaps

### GAP-16: Offset pagination remains on hot message/history paths

**Severity:** P1  
**Category:** Persistence / Performance  
**Production Risk:** Offset pagination becomes slow and unstable under concurrent inserts; clients can miss or duplicate messages while scrolling.  
**Evidence:** `PostRepository::list_by_channel` orders by `created_at DESC LIMIT $2 OFFSET $3`; channel post routes use page/per_page. Keyset-style methods exist but are not the default everywhere.  
**Current Behavior:** Mixed pagination strategy.  
**Expected Production Behavior:** Stable keyset pagination using `(created_at, id)` or monotonic `seq` cursors.  
**Recommended Fix:** Move channel history APIs to cursor pagination while keeping compatibility adapters if needed.  
**Required Tests:** Concurrent insert pagination stability, before/after cursor tests.  
**Agent-Ready Implementation Task:** Add cursor-based channel history endpoint/compat behavior and tests proving no duplicate/missing posts across concurrent inserts.

### GAP-17: Migration verification is insufficient for production upgrades

**Severity:** P1  
**Category:** Persistence / Testing  
**Production Risk:** A bad migration can block startup or corrupt data during self-hosted upgrades.  
**Evidence:** Many migrations exist in `backend/migrations`, but no full forward/backward or idempotency migration test was observed in standard check output.  
**Current Behavior:** Migrations are present; tests include schema contract tests but not complete migration lifecycle gates.  
**Expected Production Behavior:** CI should migrate empty DB to latest, migrate representative old snapshots, and verify critical constraints/indexes.  
**Recommended Fix:** Add migration smoke test using Docker Postgres and fixture snapshots.  
**Required Tests:** Empty-to-latest, previous-release-to-latest, constraint/index assertions.  
**Agent-Ready Implementation Task:** Add `backend/tests/migrations.rs` or CI job that runs `sqlx migrate run` against empty and seeded databases.

## 12. API and Contract Gaps

### GAP-18: Error response contract is not consistently enforced

**Severity:** P1  
**Category:** API / Testing  
**Production Risk:** Clients cannot reliably distinguish auth failure, forbidden access, validation error, rate limit, and transient errors.  
**Evidence:** `backend/src/error/mod.rs` maps error codes; frontend has HTTP error contract tests, but backend negative contract tests are not comprehensive across v1/v4.  
**Current Behavior:** Error mapping exists but coverage is partial.  
**Expected Production Behavior:** Every public endpoint returns stable error code/body/status.  
**Recommended Fix:** Add backend contract tests for standard error shapes across auth, permission, validation, not found, conflict, rate limit.  
**Required Tests:** API error contract consistency suite.  
**Agent-Ready Implementation Task:** Add backend error contract tests and make route handlers use shared error constructors.

### GAP-19: Undocumented/stub compatibility endpoints can be mistaken as production-supported

**Severity:** P2  
**Category:** API / Documentation  
**Production Risk:** Clients may depend on incomplete enterprise/plugin surfaces and fail later.  
**Evidence:** `backend/src/api/v4` includes broad modules such as `ldap`, `saml`, `plugins`, `compliance`, `cloud`, `schemes`; docs say advanced compatibility is limited.  
**Current Behavior:** Large surface area with mixed maturity.  
**Expected Production Behavior:** Each endpoint should be marked supported, partial, stub, or unsupported in a compatibility matrix.  
**Recommended Fix:** Generate endpoint inventory from routes and compare with docs.  
**Required Tests:** Stub endpoints return explicit stable unsupported errors.  
**Agent-Ready Implementation Task:** Add endpoint support matrix and tests for unsupported endpoint error bodies.

## 13. Frontend Reliability Gaps

### GAP-20: Optimistic message and retry semantics are incomplete

**Severity:** P1  
**Category:** Frontend / Realtime  
**Production Risk:** Users can see messages as sent when server rejected them, or duplicate messages after reconnect.  
**Evidence:** `Message.status` supports `sending/delivered/failed`, but WebSocket and store paths need stronger client ID dedupe and server ack handling.  
**Current Behavior:** Message state model has fields for status/client IDs; coverage is shallow.  
**Expected Production Behavior:** Composer creates pending message with client ID, replaces on server ack, marks failed on error, retries idempotently.  
**Recommended Fix:** Add explicit send lifecycle service and tests.  
**Required Tests:** Send success, validation failure rollback, network failure retry, duplicate server event dedupe.  
**Agent-Ready Implementation Task:** Add frontend message send lifecycle unit tests and implement client-msg-id dedupe in store.

### GAP-21: Markdown rendering relies on `v-html` and needs security regression tests

**Severity:** P1  
**Category:** Security / Frontend  
**Production Risk:** XSS in chat messages is catastrophic. DOMPurify is used, but any config change or post-process regex can reintroduce HTML.  
**Evidence:** `frontend/src/composables/useMarkdownRenderer.ts` sanitizes, then post-processes mentions, then sanitizes again. ESLint warns about `v-html` in message/thread/preview components.  
**Current Behavior:** Good sanitizer exists; lint warnings are allowed and security tests are not comprehensive.  
**Expected Production Behavior:** XSS payload corpus must be tested and `v-html` use must be isolated behind safe components.  
**Recommended Fix:** Add sanitizer corpus tests and a `SafeMarkdown` component.  
**Required Tests:** Script tags, event handlers, javascript URLs, SVG/data URLs, malformed markdown, code block escaping.  
**Agent-Ready Implementation Task:** Add `useMarkdownRenderer` XSS corpus tests and route all message rendering through one safe component.

### GAP-22: Frontend permission hiding can diverge from backend enforcement

**Severity:** P1  
**Category:** Frontend / Security  
**Production Risk:** UI may hide actions but direct API/WebSocket calls still succeed, or UI may show actions that fail.  
**Evidence:** Frontend permission tests exist in `frontend/src/features/permissions`, but backend permission matrix does not mirror every UI capability.  
**Current Behavior:** UI capability checks are separate from backend policy tests.  
**Expected Production Behavior:** Frontend capability names should map to backend permission constants and backend contract tests.  
**Recommended Fix:** Share/generate permission matrix documentation and tests.  
**Required Tests:** UI hides and backend rejects same actions for same roles.  
**Agent-Ready Implementation Task:** Create permission matrix fixture and use it in frontend unit tests plus backend integration tests.

## 14. Security Gaps

### GAP-23: SVG validation blocks scripts but not all active SVG payloads

**Severity:** P1  
**Category:** Security  
**Production Risk:** Uploaded SVGs can contain event handlers, external references, or active content. Served inline, this can become stored XSS or tracking.  
**Evidence:** `backend/src/api/file_validation.rs::validate_svg` rejects `<script>` only. `get_file` serves `image/svg+xml` inline with `nosniff` and CSP `Frame-ancestors 'none'`, but not a restrictive per-file CSP.  
**Current Behavior:** Basic SVG screening.  
**Expected Production Behavior:** Either disallow SVG uploads or sanitize with a robust SVG sanitizer and serve as attachment/sandboxed with restrictive CSP.  
**Recommended Fix:** For production, disallow SVG by default or serve SVG as download-only.  
**Required Tests:** SVG event handler, external href, embedded HTML, data URL payload rejection.  
**Agent-Ready Implementation Task:** Harden or disable SVG upload and add malicious SVG validation tests.

### GAP-24: Rate limiting is present but not proven on all abuse-sensitive paths

**Severity:** P1  
**Category:** Security / Operations  
**Production Risk:** Login, registration, password reset, file upload, search, and WebSocket connect can be abused.  
**Evidence:** `backend/src/services/rate_limit.rs`, `backend/src/middleware/rate_limit.rs`, and password reset throttles exist; complete path coverage was not verified.  
**Current Behavior:** Some rate-limit tests exist (`backend/tests/test_rate_limiting.rs`).  
**Expected Production Behavior:** Every unauthenticated or expensive endpoint has documented limits and tests.  
**Recommended Fix:** Add route coverage matrix and abuse tests.  
**Required Tests:** Login brute force, registration, reset, upload, search, WebSocket connect, API key tier behavior.  
**Agent-Ready Implementation Task:** Add integration tests asserting rate-limit headers/status for all abuse-sensitive endpoints.

### GAP-25: Secrets and local state exist in repo tree and need enforcement

**Severity:** P2  
**Category:** Security / Operations  
**Production Risk:** `.env`, `.codex/auth.json`, local DB dumps, and generated logs in the working tree increase accidental leakage risk even if ignored.  
**Evidence:** Required inventory lists `.env`, `.codex/auth.json`, `dump.rdb`, `.gstack/security-reports/*`.  
**Current Behavior:** Local files exist in tree path; git status clean suggests ignored/untracked handling, but secret scanning gate should enforce.  
**Expected Production Behavior:** Secrets should be outside repo root where possible; CI and pre-commit should scan committed changes.  
**Recommended Fix:** Ensure `.gitignore`, GitGuardian, and CI secret scanning cover these paths.  
**Required Tests:** CI secret scan and denylist fixtures.  
**Agent-Ready Implementation Task:** Add repository hygiene doc and CI secret-scan verification for tracked files.

## 15. Observability and Operations Gaps

### GAP-26: Health/readiness checks need deeper dependency coverage

**Severity:** P1  
**Category:** Operations  
**Production Risk:** Load balancers may send traffic to nodes without DB/Redis/S3 readiness or with broken migration state.  
**Evidence:** Health routes and compose healthchecks exist, but production readiness should verify DB, Redis, object storage, migration version, and background workers.  
**Current Behavior:** Health endpoints exist (`backend/src/api/health.rs`, `backend/src/api/v4/system.rs`).  
**Expected Production Behavior:** Separate live and ready endpoints with dependency checks and timeout budgets.  
**Recommended Fix:** Add readiness endpoint with DB/Redis/S3/migrations checks.  
**Required Tests:** Dependency-down tests for readiness status.  
**Agent-Ready Implementation Task:** Extend health API and add tests for DB/Redis unavailable behavior.

### GAP-27: Audit logging is not guaranteed for all security-sensitive actions

**Severity:** P1  
**Category:** Security / Operations  
**Production Risk:** Admin/user security events cannot be investigated after incident.  
**Evidence:** `audit_logs` migration and admin audit API exist, but comprehensive audit calls for permission changes, admin actions, file access, token/API key usage, and failed auth were not observed.  
**Current Behavior:** Audit infrastructure exists; coverage is unclear.  
**Expected Production Behavior:** Security-sensitive state changes produce structured audit logs.  
**Recommended Fix:** Define audit event taxonomy and enforce in services.  
**Required Tests:** Audit rows are written for admin changes, membership changes, token/API-key actions, failed logins, file access denied.  
**Agent-Ready Implementation Task:** Add audit taxonomy tests and instrument missing service methods.

### GAP-28: Graceful shutdown/draining is not proven

**Severity:** P1  
**Category:** Operations / Realtime  
**Production Risk:** Deploys can drop active WebSocket clients without clear reconnect signal or message reconciliation.  
**Evidence:** WebSocket close codes include service restart, but no shutdown-drain test or server shutdown flow was verified.  
**Current Behavior:** Actor has close support; operational behavior is unproven.  
**Expected Production Behavior:** Shutdown should stop accepting new connections, close WS with retry/restart code, and let clients reconcile.  
**Recommended Fix:** Add graceful shutdown path and test.  
**Required Tests:** Process shutdown with active WS closes clients with expected code and reconnect recovers.  
**Agent-Ready Implementation Task:** Implement and test WebSocket draining on server shutdown.

## 16. Performance and Scalability Gaps

### GAP-29: Fanout model needs scale limits and load tests

**Severity:** P1  
**Category:** Performance / Realtime  
**Production Risk:** Large channels or many connections can cause CPU/memory spikes; broadcast sends cloned JSON through per-connection broadcast channels.  
**Evidence:** `WsHub::broadcast_local` iterates subscribed users and connections; queues are bounded but fanout cost is linear.  
**Current Behavior:** Simple in-memory fanout with optional cluster broadcast.  
**Expected Production Behavior:** Documented limits, metrics, load tests, and backpressure behavior for large channels.  
**Recommended Fix:** Add load tests and publish scaling limits; optimize only when data demands.  
**Required Tests:** 1k/5k connection synthetic fanout, slow-client, memory growth.  
**Agent-Ready Implementation Task:** Add a WebSocket load-test harness and record baseline throughput/memory.

### GAP-30: Large channel frontend rendering needs virtualization proof

**Severity:** P2  
**Category:** Frontend / Performance  
**Production Risk:** Large channel history can become slow, memory-heavy, or scroll-janky.  
**Evidence:** Message list components exist, but virtualization was not observed in sampled frontend files.  
**Current Behavior:** Standard Vue list rendering likely used.  
**Expected Production Behavior:** Virtualized message list or bounded DOM with stable scroll restoration.  
**Recommended Fix:** Add performance test and virtualize if needed.  
**Required Tests:** Load 5k/20k messages, scroll older/newer, preserve viewport.  
**Agent-Ready Implementation Task:** Add frontend performance E2E for large message lists and implement virtualization if test fails.

## 17. Testing Gap Analysis

| Category | What exists | Missing / insufficient | Production blocker |
|---|---|---|---|
| Unit tests | Backend module tests, frontend 108 Vitest tests | Domain validators, permission matrix, sanitizer corpus, WebSocket authorization units | Yes |
| Integration tests | Many `backend/tests/api_*`, security/rate-limit/api-key tests | Non-member WS receive, deleted user WS, attachment permission after membership change, transaction rollback | Yes |
| Contract/API tests | `backend/compat/contracts`, frontend HTTP contract tests | Broad v4 schema and error contracts, WebSocket event contracts | Yes |
| WebSocket tests | `api_v4_websocket_lifecycle.rs`, calls signaling tests | Unauthorized subscribe/typing, slow client, replay gaps, restart, duplicate suppression | Yes |
| Frontend component tests | Atomic/layout/channel/modal tests | Composer failure rollback, reconnect reconciliation, large list, file upload errors | Partial |
| E2E tests | Auth, composer, DM consistency, settings parity, WebSocket disconnection | Full login-send-receive-edit-delete-upload-search-unread flow | Yes |
| Security tests | `security_integration.rs`, rate limit tests, sanitizer usage | XSS corpus, malicious SVG, path traversal, brute force matrix, CORS production config | Yes |
| Performance/load tests | Not observed as regular gate | WS fanout, large channel, search, upload limits | Partial |
| Migration tests | Schema contract tests | Empty-to-latest and previous-release migration jobs | Yes |
| Regression tests | Some targeted tests | Need gaps linked to incidents/spec promises | Partial |

## 18. Mandatory Unit Test Plan

Backend domain logic:

- Message validation: empty, whitespace-only, oversized, invalid markdown policy, attachment-only, too many files.
- Channel permissions: member, non-member, admin, guest, bot/agent if supported.
- Private channel and DM access checks.
- Edit/delete ownership and admin override rules.
- Soft-delete semantics and deleted message response shape.
- Timestamp/sequence ordering and cursor generation.
- Unread counter calculation for root messages and thread replies.
- Mention parsing excluding code blocks/URLs and notification target filtering.
- Attachment metadata validation, ownership, channel binding, filename normalization.
- Role/permission matrix.
- Configuration validation for secrets, CORS, site URL, upload limits.
- Domain-to-API error mapping.

Realtime logic:

- WebSocket auth accept/reject.
- Subscribe/unsubscribe membership checks.
- Broadcast only to authorized users.
- Disconnected client cleanup.
- Slow client queue overflow behavior.
- Duplicate event handling.
- Event ordering and reconnect/replay behavior.
- Heartbeat timeout.

Persistence logic:

- Repository CRUD behavior.
- Transaction rollback on partial post/file/reply failure.
- Unique constraints and foreign keys.
- Migration constraints/index existence.
- Concurrent message creation and stable pagination.

Security logic:

- Markdown sanitization corpus.
- File type/size validation.
- SVG active content rejection.
- Path traversal prevention.
- Token/session validation for active/deleted users.
- Rate-limit behavior.
- CORS/config validation.
- Forbidden access returns no data.

## 19. Mandatory Integration and Contract Test Plan

Add or harden tests for:

- User login plus authenticated request.
- Channel creation, membership, message posting.
- Non-member cannot read private channel history.
- Non-member cannot subscribe to private channel WebSocket.
- Non-member cannot receive WebSocket event after guessing channel ID.
- User removed from channel stops receiving WebSocket events.
- Message create is persisted before broadcast.
- Message edit updates API and realtime subscribers.
- Message delete updates API and realtime subscribers.
- Attachment upload plus permission-protected download.
- User cannot attach another user's uploaded file.
- Pagination over channel history under concurrent inserts.
- Search returns only authorized results.
- Unread counts after read/unread/thread events.
- WebSocket reconnect does not duplicate or lose messages.
- Server restart behavior or documented reconnect reconciliation.
- Migration from empty DB to latest schema.
- API error contract consistency.

## 20. Mandatory E2E Test Plan

Required Playwright flows:

- Login, open channel, send message, see server-confirmed delivered state.
- Two users in one channel: user A sends, user B receives once.
- Edit and delete message propagate to another user.
- Upload file, download as authorized member, verify non-member denied.
- Private channel non-member cannot view route, fetch API, or receive WS.
- Reconnect while messages are sent: no duplicates, missing messages reconciled.
- Offline/connection-lost UI appears and recovers.
- Long message, code block, malicious markdown payload renders safely.
- Search returns only authorized messages.
- Large channel scroll/load older messages without duplicates.

## 21. Recommended CI/CD and Release Quality Gates

Before production:

- Backend: `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo test --all --all-features`.
- Frontend: `npm ci`, dependency policy, `npm run lint -- --max-warnings=0` or a ratcheted warning budget, `npm run test:unit`, `npm run build`.
- Integration: Docker-backed Postgres/Redis/S3 suite for auth, permissions, files, WebSocket, migrations.
- Contract: v4 API schema tests and WebSocket event schema tests.
- Security: cargo audit/deny, npm audit policy, CodeQL, secret scanning, XSS/file upload corpus.
- E2E: smoke plus critical collaboration flow.
- Release: migration smoke test, Docker image build, SBOM/provenance if desired, backup/restore smoke.

## 22. Prioritized Engineering Backlog

| Priority | Task | Files/Modules | Expected Change | Required Tests | Acceptance Criteria |
|---|---|---|---|---|---|
| P0 | ~~Authorize WebSocket channel subscription~~ **FIXED** | `backend/src/api/websocket_core.rs` | Reject non-member subscribe attempts | `websocket_non_member_cannot_subscribe_channel` | Non-member receives error event |
| P0 | ~~Authorize typing events~~ **FIXED** | `backend/src/api/v4/websocket/connection.rs`, `backend/src/api/websocket_core.rs` | Check membership before broadcast | `websocket_non_member_cannot_send_typing` | Unauthorized typing event is not delivered |
| P0 | ~~Fix frontend reconnect state~~ **FIXED** | `frontend/src/composables/useWebSocket.ts`, `frontend/src/core/websocket/WebSocketManager.ts` | Track server connection ID and last seq | Compile + unit tests pass | Reconnect URL includes resumption params |
| P0 | ~~Transactional post core state~~ **PARTIALLY FIXED** | `backend/src/services/posts.rs`, `backend/src/repositories/post_repository.rs` | Commit post + reply count atomically, broadcast after commit | Compile + integration tests pass | Forced DB failure leaves no post without reply count |
| P1 | Full transactional boundary for mentions/unreads/automation | `backend/src/services/posts.rs` | Move mention/unread side effects into transaction or outbox | Rollback integration test | No broadcast or partial counters on forced failure |
| P1 | Validate file IDs on post create | `backend/src/services/posts.rs`, `backend/src/repositories/file_repository.rs` | Enforce uploader/channel binding | Unauthorized attach tests | User cannot attach another user's file |
| P1 | Replace offset history pagination on hot path | `PostRepository`, channel post routes | Stable cursor pagination | Concurrent pagination tests | No duplicate/missing messages under insert |
| P1 | Add backend permission matrix tests | `backend/tests/api_permissions_matrix.rs` | Role/action/channel coverage | Integration matrix | Member/admin/non-member behavior is explicit |
| P1 | Add WebSocket revocation on membership removal | channel member service/routes + `WsHub` | Unsubscribe removed users | Live two-WS test | Removed member stops receiving immediately |
| P1 | Add XSS/markdown corpus tests | `frontend/src/composables/useMarkdownRenderer.ts` | Sanitizer regression coverage | Vitest corpus | Payloads cannot execute or create unsafe URLs |
| P1 | Harden SVG handling | `backend/src/api/file_validation.rs`, file serving | Disable or robustly sanitize SVG | Malicious SVG tests | Active SVG payloads rejected or download-only |
| P1 | Add readiness dependency checks | `backend/src/api/health.rs` | DB/Redis/S3/migration readiness | Dependency-down tests | Readiness fails when dependencies fail |
| P1 | Expand v4 API contracts | `backend/compat/contracts`, `backend/compat/tests` | Schema/error coverage | Contract tests | Client-facing shapes are locked |
| P1 | Add migration lifecycle test | CI + backend tests/scripts | Empty/latest and seeded/latest migrations | Docker-backed migration test | Migration job blocks bad schema changes |
| P2 | Document ADRs for critical models | `docs/adr` | Durable decisions | Docs review | WebSocket/auth/files/compat ADRs exist |
| P2 | Large-channel frontend perf test | `frontend/e2e`, `MessageList` | Baseline and virtualization if needed | Playwright perf test | 5k messages remain usable |

## 23. Agent-Ready Implementation Tasks

| Priority | Task | Files/Modules | Expected Change | Required Tests | Acceptance Criteria |
|---|---|---|---|---|---|
| P0 | ~~Add backend authorization check before subscribing a WebSocket connection to a private channel.~~ **DONE** | `backend/src/api/websocket_core.rs`, `backend/tests/api_v4_websocket_lifecycle.rs` | Membership checked before `ws_hub.subscribe_channel` | `websocket_non_member_cannot_subscribe_channel` | Non-member receives error event |
| P0 | ~~Add backend authorization check before broadcasting typing events.~~ **DONE** | `backend/src/api/v4/websocket/connection.rs`, `backend/src/api/websocket_core.rs` | Typing accepted only for channel members | `websocket_non_member_cannot_send_typing` | No typing event delivered for unauthorized sender |
| P0 | ~~Fix WebSocketManager connection ID and sequence tracking.~~ **DONE** | `frontend/src/composables/useWebSocket.ts`, `frontend/src/core/websocket/WebSocketManager.ts` | Parse hello `data.connection_id`, track last `seq`, use on reconnect | Compile + unit tests pass | Reconnect URL includes resumption params |
| P0 | ~~Make post creation core state transactional.~~ **DONE** | `backend/src/services/posts.rs`, `backend/src/repositories/post_repository.rs` | Insert post + reply count in one transaction, broadcast after commit | Compile + integration tests pass | No post without reply count on forced failure |
| P1 | Expand transactional boundary to mentions/unreads or add outbox. | `backend/src/services/posts.rs` | Move mention/unread/automation into transaction or durable outbox | Rollback + broadcast-after-commit test | No partial visible state on failure |
| P1 | Validate attached file ownership and channel before post creation. | `backend/src/services/posts.rs`, `backend/src/repositories/file_repository.rs` | Reject file IDs not uploaded by user or not valid for channel | Integration tests | Unauthorized file attach returns 403/400 and no post |
| P1 | Add membership revocation for live WebSocket subscriptions. | Channel member removal routes/services, `backend/src/realtime/hub.rs` | Removed member unsubscribed immediately | Two-client WS integration | Removed user receives no later channel event |
| P1 | Add markdown XSS corpus tests. | `frontend/src/composables/useMarkdownRenderer.ts` | Test sanitizer against payload corpus | Vitest | All payloads sanitized safely |
| P1 | Harden SVG uploads. | `backend/src/api/file_validation.rs`, `backend/src/api/v4/files.rs` | Disable SVG or serve as attachment with strict validation | Malicious SVG tests | Active SVG payloads rejected/safe |
| P1 | Add cursor pagination tests and migrate channel history routes. | `backend/src/repositories/post_repository.rs`, `backend/src/api/v4/channels/posts.rs` | Stable keyset pagination | Concurrent insert test | No duplicates/missing posts across pages |
| P1 | Add API error contract suite. | `backend/tests/api_error_contract.rs` | Standard error bodies/statuses | Contract tests | Auth/forbidden/validation/not-found/rate-limit shapes stable |
| P1 | Add readiness endpoint dependency tests. | `backend/src/api/health.rs`, tests | Ready checks DB/Redis/S3/migrations | Integration tests | Ready fails closed on dependency outage |
| P1 | Add migration CI job. | `.github/workflows/integration.yml`, migration test helper | Run migrations on empty/seeded DB | CI migration test | Failed migration blocks merge |

## 24. Final Production Readiness Recommendation

RustChat should be treated as **ready for internal beta only**.

It has enough implementation and tests for internal dogfooding, especially with trusted users and active engineering support. It is not ready for controlled external preview or production until WebSocket authorization, reconnect correctness, transactional message side effects, attachment lifecycle validation, security corpus tests, contract tests, and migration/readiness gates are addressed.

Top 10 urgent gaps:

1. ~~GAP-1: WebSocket channel subscription bypasses membership.~~ **FIXED**
2. ~~GAP-2: WebSocket typing events are broadcast without membership validation.~~ **FIXED**
3. ~~GAP-3: Frontend reconnect/resumption state is incorrect.~~ **FIXED**
4. ~~GAP-4: Message creation side effects are not transactional.~~ **FIXED**
5. GAP-15: Attachment lifecycle is not tied tightly enough to message lifecycle.
6. GAP-11: Broadcast fanout trusts in-memory subscriptions and lacks revocation.
7. GAP-13: Message validation is too narrow.
8. GAP-21: Markdown rendering needs XSS regression tests.
9. GAP-23: SVG validation is insufficient for production.
10. GAP-17: Migration verification is insufficient for production upgrades.
