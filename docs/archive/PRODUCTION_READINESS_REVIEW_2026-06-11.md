# RustChat Production Readiness Review

**Date:** 2026-06-11  
**Branch reviewed:** `main` (after PR #181 + security/auth-boundary-fixes)  
**Previous audits:** `docs/audits/rustchat-production-gap-analysis.md`, `docs/audits/rustchat-implementation-consistency-and-gap-analysis.md`

---

## 1. Executive Summary

RustChat has matured significantly since the May 2026 audits. Backend unit tests grew from **143 → 173**, frontend unit tests from **108 → 300**, four P0 security gaps were closed, file upload authorization was fixed, and the native file API now proxies downloads instead of leaking presigned S3 URLs.

**Verdict:** The codebase is approaching **public preview readiness** but is not yet **production-grade**. The remaining blockers are concentrated in:

1. **Realtime durability** (in-memory replay, no graceful shutdown, silent event drops)
2. **Authorization completeness** (scheduled post delete, bulk membership revocation)
3. **Observability & audit** (audit logs only used in 2 places, readiness lacks migration check)
4. **Message validation & pagination** (no idempotency, offset pagination on hot paths)
5. **Rate limiting coverage** (file upload and search unprotected)

---

## 2. What's Improved (Verified Closed)

| Gap | Prior Severity | Status | Evidence |
|-----|---------------|--------|----------|
| GAP-1: WS channel subscription auth | P0 | **CLOSED** | `websocket_core.rs` checks `is_channel_member` before subscribe |
| GAP-2: WS typing event auth | P0 | **CLOSED** | Both v1 and v4 typing paths validate membership |
| GAP-3: Frontend reconnect state | P0 | **CLOSED** | `useWebSocket.ts` tracks `connection_id` + `last_seq` |
| GAP-4: Transactional post core | P0 | **CLOSED** | `create_post` uses service-level tx for DB side effects |
| F-001: Native file upload membership | P0 | **CLOSED** | `files.rs::upload_file` calls `check_upload_channel_access` |
| F-002: Presigned URL leak | P0 | **CLOSED** | Upload/download return `/api/v1/files/{id}/content` proxy URLs |
| F-003: v4 post action stubs | P1 | **CLOSED** | All return `501 Not Implemented` via `post_action_not_implemented` |
| F-004 create/update: Scheduled post auth | P1 | **CLOSED** | `create_scheduled_post` and `update_scheduled_post` call `require_channel_membership` |
| Cross-team channel/DM access | P1 | **CLOSED** | `channels.rs` now verifies requester team membership |
| Password change verification | P1 | **CLOSED** | `change_password` verifies `current_password` against hash |
| Agent KB team scoping | P1 | **CLOSED** | `knowledge.rs` checks agent creator is in same team as KB |
| Status endpoint auth | P2 | **CLOSED** | `get_user_status` and `get_statuses_by_ids` require `AuthUser` |
| GAP-23: SVG validation | P1 | **CLOSED** | SVG blocked at extension level; `validate_svg` is dead code |
| GAP-21: Markdown XSS | P1 | **CLOSED** | DOMPurify + 10-case XSS corpus test suite exists |
| GAP-25: Secrets in repo | P2 | **CLOSED** | `.gitignore` excludes `.env` and `secrets/`; nothing tracked |

---

## 3. Critical Production Blockers (P0 / P1)

### 3.1 Realtime Durability & Operations

#### GAP-10: Realtime replay is in-memory only
- **Risk:** Server restart or >128 messages between reconnects = lost events
- **Evidence:** `connection_store.rs` uses `VecDeque` bounded to 128, TTL 5 min. No outbox / durable event table.
- **Fix:** Document replay as best-effort + require REST reconciliation on reconnect, or add durable outbox.

#### GAP-11: Subscription revocation incomplete
- **Risk:** Users removed from channels may continue receiving events
- **Evidence:** `remove_channel_member_by_id` (v4) and `remove_member` (v1) now unsubscribe, BUT:
  - `admin_teams.rs::remove_team_member` does **not** unsubscribe
  - `v4/teams/members.rs::remove_team_member` does **not** unsubscribe
  - `leave_team` unsubscribes from team but **not** from individual channels
- **Fix:** Add hub unsubscribe to all membership removal paths.

#### GAP-12: Slow clients drop events silently
- **Risk:** Users display stale state without knowing they missed events
- **Evidence:** `websocket_actor.rs` drops events when queue full (256). `connection.rs` drops on broadcast lag. No `resync_required` event or close code sent.
- **Fix:** Send `resync_required` event or close with policy code on overflow/lag.

#### GAP-28: No graceful shutdown
- **Risk:** Deploys drop all WS connections abruptly
- **Evidence:** `main.rs` uses bare `axum::serve(...).await` with no `with_graceful_shutdown`, no SIGTERM/SIGINT handler. `SERVICE_RESTART` close code (1012) is defined but never used.
- **Fix:** Add signal handling, stop accepting new connections, drain WS with 1012 close code.

### 3.2 Authorization Completeness

#### F-004 delete: `delete_scheduled_post` lacks channel membership check
- **Risk:** Users can delete scheduled posts for channels they left
- **Evidence:** `v4/posts.rs:1151-1177` checks ownership via `delete_scheduled_post(scheduled_id, auth.user_id)` but never verifies current channel membership.
- **Fix:** Fetch scheduled post's `channel_id`, then `require_channel_membership` before delete.

#### GAP-15: File ownership validation outside transaction
- **Risk:** Race condition between pre-tx read and tx write
- **Evidence:** `services/posts.rs:352-387` validates file ownership/channel before tx. `services/posts.rs:407-423` links files inside tx with `UPDATE ... WHERE post_id IS NULL`, but does NOT re-verify `uploader_id` or `channel_id`.
- **Fix:** Move ownership/channel validation inside the transaction.

### 3.3 Observability & Audit

#### GAP-26: Readiness lacks migration check
- **Risk:** K8s routes traffic to nodes with pending migrations
- **Evidence:** `api/health.rs::readiness` checks DB, Redis, S3, email outbox, but NOT `schema_migrations` / pending migrations.
- **Fix:** Query migration state in readiness probe.

#### GAP-27: Audit logging almost unused
- **Risk:** Security incidents are un-investigable
- **Evidence:** `insert_admin_audit_log` exists but is only called in 2 places (`admin_users.rs:251` soft-delete, `admin_users.rs:314` wipe). Missing for: permission changes, config changes, team/channel deletion, SSO/email changes, file access, failed logins.
- **Fix:** Add audit calls to all security-sensitive admin/auth actions.

### 3.4 Data Consistency & Validation

#### GAP-13: Message validation too narrow
- **Risk:** Oversized messages, malicious markdown, duplicate messages
- **Evidence:** `validate_create_post` checks empty, max length, max files, root post validity, but NOT:
  - Markdown/HTML safety (backend trusts frontend sanitizer)
  - `client_msg_id` deduplication (accepted in API but never checked for duplicates)
- **Fix:** Add idempotency check on `client_msg_id`; consider backend markdown safety validation.

#### F-005: Mention/unread uses regex over raw text
- **Risk:** False positive mentions in code blocks, URLs, inline code
- **Evidence:** `services/unreads.rs` uses PostgreSQL regex `LOWER(message) ~ pattern` directly on raw message text. `parse_mentions` in `posts.rs` (code-block/URL-aware) is **never called** by unreads.
- **Fix:** Either pre-compute mentions at post creation and store in a `post_mentions` table, or make unreads use the same parser as post creation.

#### GAP-16: Offset pagination on hot paths
- **Risk:** Missing/duplicate messages during concurrent inserts
- **Evidence:** `PostRepository::list_by_channel` uses `OFFSET` ordering by `created_at DESC`. Keyset methods exist but are not the default.
- **Fix:** Migrate channel history to cursor pagination.

### 3.5 Security & Abuse

#### GAP-24: Rate limiting missing on file upload and search
- **Risk:** Resource exhaustion, enumeration
- **Evidence:** Auth (login/register/reset) and WS connect have IP rate limits. File upload (`v4/files.rs`, `v4/uploads.rs`) and ALL search endpoints (`v4/posts/search`, `v4/users/search`, `v4/files/search`, `api/search.rs`) have **no** rate-limit middleware.
- **Fix:** Add per-IP and per-user rate limits to upload and search.

---

## 4. High-Priority Gaps (P1)

| Gap | Risk | Current State |
|-----|------|---------------|
| GAP-17: Migration verification insufficient | Bad migration blocks/corrupts production upgrades | No CI job tests empty→latest or snapshot→latest migrations |
| GAP-18: Error contract inconsistent | Clients cannot reliably handle errors | Error mapping exists but negative contract tests are sparse |
| GAP-19: Undocumented v4 stubs | Clients depend on incomplete surfaces | Some stubs now return 501, but full endpoint inventory missing |
| GAP-20: Optimistic message retry incomplete | Users see false sent states or duplicates | `client_msg_id` exists but no dedup on backend |
| GAP-22: Frontend/backend permission parity | UI hides actions backend still allows | Permission matrix exists but no end-to-end parity tests |
| GAP-29: Fanout needs scale limits | Large channels cause CPU/memory spikes | `WsHub::broadcast_local` is linear; no load tests |
| GAP-30: Large channel frontend rendering | Scroll jank, memory pressure | No virtualization observed; no perf E2E for 5k messages |

---

## 5. Code Quality Findings

### Backend
- **63 `unwrap()` / `expect()` calls** in `src/api/`, `src/services/`, `src/repositories/` (excluding tests). Most are in v4 websocket connection mapping functions where data shapes are controlled, but a few are in API handlers (e.g., `channels/posts.rs:first().unwrap()`, `turn.rs:88 unwrap()`).
- **No `panic!()` calls** in production code paths.
- **All SQL queries are parameterized** (`bind()` usage verified). No raw string interpolation of user input observed.
- **cargo clippy:** passes cleanly.
- **cargo audit:** 0 vulnerabilities.

### Frontend
- **230 ESLint warnings** (up from 200 in prior audit). Mostly `any` types. No errors.
- **7 `v-html` usages** in 5 files. All pipe through DOMPurify + corpus tests.
- **npm audit:** 0 vulnerabilities.
- **Build:** passes.

---

## 6. Recommended CI Quality Gates

Before calling RustChat "production ready":

| Gate | Current | Target |
|------|---------|--------|
| Backend unit tests | 173 passing | 250+ (permission matrix, message validation, idempotency) |
| Frontend unit tests | 300 passing | Maintain + add markdown/reconnect corpus |
| Integration tests | Partial (needs live DB) | Full suite in CI with Postgres/Redis/S3 |
| Contract tests | Partial v4 schemas | All public v4 endpoints + error shapes |
| E2E tests | 5 specs | Full auth→send→edit→delete→upload→search→unread flow |
| cargo clippy | Passes | Enforce `-D warnings` |
| cargo audit | 0 vulns | Maintain weekly |
| npm audit | 0 vulns | Maintain weekly |
| Frontend lint | 230 warnings | Ratchet to 100, then 0 |
| Migration test | None | CI job: empty→latest + snapshot→latest |
| Load test | None | WS fanout baseline (1k/5k conns) |
| Secret scan | GitGuardian configured | Add pre-commit hook |

---

## 7. Top 10 Next Actions (Priority Order)

1. **Add `delete_scheduled_post` channel membership check** — 1 file, ~5 lines
2. **Add rate limiting to file upload and search endpoints** — middleware layer, ~3 routes
3. **Add migration check to readiness probe** — `health.rs`, 1 query
4. **Move file ownership validation inside post creation transaction** — `services/posts.rs`
5. **Add graceful shutdown with WS drain** — `main.rs` + signal handling
6. **Add slow-client resync signal** — `websocket_actor.rs` / `connection.rs`
7. **Expand audit logging to admin/auth actions** — instrument 10+ handlers
8. **Add `client_msg_id` idempotency check** — `services/posts.rs`
9. **Fix remaining subscription revocation gaps** — `admin_teams.rs`, `v4/teams/members.rs`, `leave_team`
10. **Add integration tests for the above** — `backend/tests/`

---

## 8. Final Verdict

**Current maturity: Internal beta / early public preview**

RustChat is solid enough for trusted users and active engineering support. The remaining P0/P1 gaps are **operational and authorization completeness issues**, not fundamental architectural problems. The recent fixes (transactional posts, WS auth, file upload auth, reconnect state) show the codebase can be hardened systematically.

**Production readiness ETA:** After closing the 10 actions above plus adding integration tests and a migration CI gate, RustChat would be suitable for **controlled production** (small-to-medium teams, with active monitoring). Full enterprise production would additionally require durable event replay, cursor pagination, and load-tested fanout limits.
