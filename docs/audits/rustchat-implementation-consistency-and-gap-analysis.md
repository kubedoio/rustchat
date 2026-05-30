# RustChat Implementation Consistency and Gap Analysis

Date: 2026-05-30  
Scope: repository inspection, documented command discovery, backend/frontend static traceability, and local fast/integration checks where possible.  
Constraint: no implementation, refactor, feature creation, or spec rewrite was performed.

## 1. Executive summary

RustChat is not a paper design. The repository contains a substantial working implementation: Rust/Axum backend, native `/api/v1`, Mattermost-compatible `/api/v4`, PostgreSQL migrations, Redis-backed realtime and unread paths, S3-compatible storage, Vue 3 frontend, admin UI, calls/push plumbing, CI, security workflows, and integration tests.

The main gap is consistency and release stability, not the absence of a product. Several docs now overstate beta maturity or describe older behavior. The most important implementation gaps found in this audit are:

| Priority | Finding |
|---|---|
| P0 | Native file upload can associate a file with any `channel_id` without verifying channel membership. |
| P0 | Native file APIs return presigned S3 download URLs despite docs claiming authenticated proxy-only file access. |
| P1 | Several v4 post endpoints return `OK` without performing the requested mutation, creating contract false positives. |
| P1 | Scheduled post creation lacks a channel membership/authorization check before writing scheduled posts. |
| P1 | Mention/unread counting uses simple SQL `LIKE` patterns for `@user`, `@all`, `@channel`, and `@here`; this is not stable mention semantics. |
| P1 | Configuration docs still describe removed or renamed security/storage variables. |
| P1 | The README and user/admin docs advertise complete SAML, audit logs, search, calls, and API key/integration maturity more strongly than the code and compatibility docs support. |
| P2 | Frontend has two WebSocket client paths; the old composable is the actively used one, while the newer `core/websocket/WebSocketManager.ts` is not the main runtime path. |
| P2 | Test coverage is broad but uneven: backend integration exists, but frontend E2E does not cover many high-risk flows such as files, permissions, admin, calls, search, and thread edge cases. |
| P2 | Local `node_modules` was stale relative to `frontend/package.json`; `npm run deps:inventory` failed, which would break the CI inventory artifact on a similarly stale install. |

Verification summary:

| Command | Result |
|---|---|
| `cd backend && cargo fmt --all -- --check` | Passed |
| `cd backend && cargo test --lib --no-fail-fast -- --nocapture` | Passed: 143 tests |
| `cd backend && cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `cd frontend && npm run check:dependency-policy` | Passed |
| `cd frontend && npm run format:check` | Passed |
| `cd frontend && npm run lint` | Passed with 200 warnings |
| `cd frontend && npm run test:unit` | Passed: 19 files, 108 tests |
| `cd frontend && npm run build` | Passed |
| `cd frontend && npm run deps:inventory` | Failed: installed packages invalid/stale against manifest ranges |
| `docker compose -f docker-compose.integration.yml up -d` | Passed; test PostgreSQL/Redis/S3 became healthy |
| `cd backend && cargo test --no-fail-fast -- --nocapture` with documented integration env | Sandboxed attempt reached backend unit tests, then integration targets failed to connect to local PostgreSQL with `Operation not permitted`; not rerun on QA system per operator instruction |

## 2. Documentation inventory

High-signal documentation and contract inventory found in this repository:

| Category | Files |
|---|---|
| Root README/status/process | `README.md`, `ROADMAP.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `RELEASING.md`, `SECURITY.md`, `SUPPORT.md`, `GOVERNANCE.md`, `MAINTAINERS.md`, `DESIGN.md` |
| User docs | `docs/user/README.md`, `docs/user/features.md`, `docs/user/quick-start.md`, `docs/user/connection-status.md`, `docs/user/troubleshooting.md` |
| Admin/deployment docs | `docs/admin/installation.md`, `docs/admin/configuration.md`, `docs/admin/security.md`, `docs/admin/sso.md`, `docs/admin/email.md`, `docs/admin/push-notifications.md`, `docs/admin/reverse-proxy.md`, `docs/admin/scaling.md`, `docs/admin/backup-restore.md`, `docs/deployment.md`, `docs/quickstart.md`, `docs/security-deployment-guide.md`, `docs/security-zero-trust-guide.md`, `docs/operations/runbook.md` |
| Architecture docs | `docs/architecture.md`, `docs/architecture/overview.md`, `docs/architecture/backend.md`, `docs/architecture/frontend.md`, `docs/architecture/data-model.md`, `docs/architecture/calls-deployment.md`, `docs/architecture/current-state-analysis-2026-04-26.md`, `docs/architecture/websocket.md` |
| API/compat contracts | `docs/compatibility-scope.md`, `docs/development/compatibility.md`, `docs/reference/compatibility-matrix.md`, `docs/MATTERMOST_CLIENTS.md`, `backend/compat/README.md`, `backend/compat/contracts/*.schema.json` |
| ADRs/decision notes | `docs/adr/ADR-frontend-supply-chain-security.md`, `docs/adr/README.md`, `docs/decision-notes/code-quality-audit-2026-05.md`, `docs/decisions/README.md` |
| Internal specs/plans/audits | `docs/internal/SPEC.md`, `docs/internal/SPEC-WEBSOCKET-DISCONNECTION-UX.md`, `docs/internal/SPEC-OPEN-SOURCE-HARDENING.md`, `docs/internal/SPEC-model-schema-fixes.md`, `docs/internal/implementation-gap-analysis-2026-05-22.md`, `docs/internal/code-quality-audit-2026-05-19.md`, `docs/internal/pagination_audit.md`, `docs/internal/CHANNEL_BUGS_FIX_PLAN.md` |
| Development/test docs | `docs/development/testing.md`, `docs/development/local-setup.md`, `docs/development/code-style.md`, `docs/development/ownership.md`, `docs/development/agent-model.md`, `backend/tests/test_status.md`, `frontend/README.md`, `frontend/DEVELOPER_GUIDE.md` |
| CI/release docs/workflows | `.github/workflows/ci.yml`, `integration.yml`, `security.yml`, `compat.yml`, `release.yml`, `docker-publish.yml`, `docs/release-process.md`, `docs/development/releasing.md` |
| Deployment manifests | `docker-compose.yml`, `docker-compose.prod.yml`, `docker-compose.fast.yml`, `docker-compose.integration.yml`, `docker/backend.Dockerfile`, `docker/frontend.Dockerfile`, `push-proxy/docker-compose.yml`, `push-proxy/DOCKER_DEPLOYMENT.md` |

Archived docs under `docs/archive/` and previous analyses under `previous-analyses/` were inventoried but not treated as current contracts unless referenced by current docs.

## 3. Existing specs/contracts/ADRs map

| Source | Requirement or decision | Implementation area | Status |
|---|---|---|---|
| `docs/adr/ADR-frontend-supply-chain-security.md` | npm only; one committed lockfile; `npm ci --ignore-scripts`; policy check; dependency review/audit; internal fetch wrapper | `frontend/package.json`, `.github/workflows/ci.yml`, `.github/workflows/security.yml`, `docker/frontend.Dockerfile`, `frontend/src/api/http/HttpClient.ts` | Mostly implemented |
| `docs/internal/SPEC.md` | Frontend supply-chain hardening and dependency minimization | Same as ADR plus `docs/frontend-dependency-policy.md` | Mostly implemented; historical "current-state" sections stale |
| `docs/compatibility-scope.md` | Mobile-critical Mattermost compatibility on `/api/v4/*` and `/api/v4/websocket`; known v4 stubs documented | `backend/src/api/v4/`, `backend/src/mattermost_compat/`, `backend/compat/` | Partially implemented; headline plausible, full v4 not complete |
| `docs/reference/compatibility-matrix.md` | 41/41 mobile-critical endpoints, 8/8 core WebSocket events, basic search caveat | v4 routes, `backend/src/api/v4/websocket/` | Partially confirmed; full endpoint list source not present in this audit |
| `docs/architecture/overview.md` | Shared WebSocket core, v1/v4 wire formats, Redis fan-out, connection resumption | `backend/src/api/websocket_core.rs`, `backend/src/api/ws.rs`, `backend/src/api/v4/websocket/`, `backend/src/realtime/` | Mostly implemented; v1 doc appears stale for current web app usage |
| `docs/architecture.md` | File downloads are authenticated through backend, no presigned URL leaks | `backend/src/api/files.rs`, `backend/src/storage/s3.rs` | Contradicted by native file API |
| `docs/security-model.md` | JWT auth, Redis session/rate state, RBAC, WebSocket auth policy | `backend/src/auth/`, `backend/src/config/mod.rs`, `backend/src/middleware/rate_limit.rs` | Partially implemented; session/query-token text stale |
| `docs/admin/configuration.md` | Complete env var reference | `backend/src/config/mod.rs`, `docker-compose*.yml` | Incomplete/stale |
| `docs/development/testing.md` | Backend integration tests require live PostgreSQL/Redis/S3; frontend unit/build/E2E; CI gates | `backend/tests/`, `frontend/src/**/*.test.ts`, `frontend/e2e/`, workflows | Mostly accurate, but frontend now has unit tests despite older statement saying E2E only |
| `backend/compat/contracts/*.schema.json` | v4 shape contracts for user/channel/post/error | v4 handlers and contract validation utilities | Partially tested via integration helpers; not a complete OpenAPI contract suite |

## 4. Requirement-to-implementation traceability matrix

| Requirement | Backend implementation | Frontend implementation | Implementation status | Docs accurate |
|---|---|---|---|---|
| Login/session handling | `backend/src/api/auth.rs`, `backend/src/auth/jwt.rs`, `backend/src/auth/extractors.rs` | `frontend/src/features/auth/`, `frontend/src/api/client.ts` | Implemented JWT auth; no confirmed per-user token revocation | Partially stale |
| Public/private/direct/group channels | `backend/src/api/channels.rs`, `backend/src/api/v4/channels/`, `backend/src/repositories/channel_repository.rs` | `frontend/src/api/channels.ts`, channel stores/components | Mostly implemented | Mostly accurate |
| Message send/edit/delete | `backend/src/api/posts.rs`, `backend/src/services/posts.rs`, `backend/src/api/v4/posts.rs` | `frontend/src/api/posts.ts`, message store/composer | Implemented for normal posts; several advanced v4 actions stubbed | Overstated for advanced v4 |
| Threads/replies | `backend/src/services/posts.rs`, `backend/src/api/v4/threads.rs`, thread migrations | `frontend/src/components/thread/`, `frontend/src/features/messages/stores/threadStore.ts` | Implemented | Mostly accurate |
| Reactions | Native/v4 reactions in post APIs, migration `20260222000002_create_reactions.sql` | Message item/store, emoji picker | Implemented | Accurate |
| Unreads/read markers | `backend/src/services/unreads.rs`, `backend/src/api/unreads.rs`, v4 unread routes | `frontend/src/features/unreads/`, sidebar | Implemented but semantics are basic and likely drift-prone | Overstated |
| Mentions | SQL `LIKE` in unread services, markdown rendering in frontend | Composer autocomplete and notification prefs | Partial; no parser-backed mention model confirmed | Overstated |
| File attachments | Native/v4 file APIs, S3 storage, validation | `frontend/src/api/files.ts`, upload components/previews | Implemented with authorization gaps in native path | Docs contradicted |
| Search | `backend/src/api/search.rs`, v4 post/file search | `frontend/src/components/channel/SearchPanel.vue`, search modal | Basic search implemented | Docs caveat exists in compatibility docs; README overstates |
| Pinned/saved messages | Native post pin/save routes and migrations | panels/components | Implemented | Mostly accurate |
| WebSocket realtime | `backend/src/api/v4/websocket/`, `backend/src/realtime/` | `frontend/src/composables/useWebSocket.ts` | Implemented, with duplicate frontend path risk | Partially stale |
| Presence/typing | WebSocket core, status routes | presence store, typing indicator | Implemented | Mostly accurate |
| Admin controls/audit logs | `backend/src/api/admin*.rs`, `audit_logs` migration | `frontend/src/views/admin/*` | Broad implementation; not all controls confirmed end-to-end | README overstates some enterprise maturity |
| SSO/OAuth/SAML | OAuth implemented; SAML v4 endpoints are compatibility stubs | SSO admin/settings views | OAuth/OIDC partial; SAML not confirmed functional | README/admin docs overstate |
| Rate limiting | `backend/src/middleware/rate_limit.rs`, `backend/src/services/rate_limit.rs` | N/A | Implemented for IP and entity paths | Prior internal audit stale |
| Supply-chain hardening | CI/security workflows, npm policy, Dockerfile | N/A | Mostly implemented | Current ADR accurate |

## 5. Requirement-to-test traceability matrix

| Area | Test evidence | Test status |
|---|---|---|
| Backend unit/security helpers | `cargo test --lib` passed 143 tests, including auth policy, websocket mapping, rate-limit helper, security headers, connection store, hub cleanup | Good unit coverage |
| Backend integration | `backend/tests/*.rs` includes auth, users, permissions, v4 channels/posts/threads/unreads, websocket lifecycle, calls signaling, security, services | Present; full suite was started in this audit |
| Frontend unit/component | `npm run test:unit` passed 19 files / 108 tests | Good for selected components/stores |
| Frontend E2E | `frontend/e2e/auth.spec.ts`, `composer.spec.ts`, `dm-consistency.spec.ts`, `settings_parity.spec.ts`, `websocket-disconnection.spec.ts` | Present but narrow |
| Contract validation | `backend/compat/contracts/*.schema.json`, `backend/compat/tests/contract_validation.rs`, `compat.yml` OpenAPI diff workflow | Partial; not all routes have executable schema assertions |
| File authorization | Tests not confirmed for native upload with unauthorized `channel_id` | Gap |
| Scheduled post authorization | Tests not confirmed for unauthorized scheduled post create/update/delete | Gap |
| Advanced v4 post actions | Tests not confirmed for move/restore/reveal/burn side effects | Gap |
| Search semantics | Tests cover escaping/clamping; not full Mattermost search semantics | Partial |
| Permission/UI parity | Frontend permissions unit tests exist; backend permissions tests exist | Partial; end-to-end permission denial UX gaps remain |

## 6. Current implementation map

Backend:

| Area | Files |
|---|---|
| Router composition | `backend/src/api/mod.rs`, `backend/src/api/v4/mod.rs` |
| Auth/JWT/API keys | `backend/src/auth/`, `backend/src/api/auth.rs`, `backend/src/api/v1/entities.rs` |
| Channels/teams/users | `backend/src/api/channels.rs`, `backend/src/api/teams.rs`, `backend/src/api/users.rs`, `backend/src/api/v4/channels/`, `backend/src/api/v4/teams/`, `backend/src/api/v4/users/` |
| Posts/threads/reactions/unreads | `backend/src/api/posts.rs`, `backend/src/services/posts.rs`, `backend/src/services/unreads.rs`, `backend/src/api/v4/posts.rs`, `backend/src/api/v4/threads.rs` |
| Files/storage | `backend/src/api/files.rs`, `backend/src/api/v4/files.rs`, `backend/src/storage/s3.rs`, `backend/src/api/file_validation.rs` |
| WebSocket/realtime | `backend/src/api/ws.rs`, `backend/src/api/v4/websocket/`, `backend/src/api/websocket_core.rs`, `backend/src/realtime/` |
| Admin/operations | `backend/src/api/admin*.rs`, `backend/src/api/v4/system.rs`, migrations |
| Calls/push | `backend/src/api/calls.rs`, `backend/src/api/v4/calls_plugin/`, `backend/src/calls/`, `backend/src/services/push_notifications.rs`, `push-proxy/` |

Frontend:

| Area | Files |
|---|---|
| API client | `frontend/src/api/client.ts`, `frontend/src/api/http/HttpClient.ts`, `frontend/src/api/*.ts` |
| Routing/app shell | `frontend/src/router/index.ts`, `frontend/src/App.vue`, `frontend/src/components/layout/*` |
| Feature stores | `frontend/src/features/*/stores`, legacy `frontend/src/stores/*` |
| Chat UI | `frontend/src/views/main/ChannelView.vue`, `frontend/src/components/channel/*`, `frontend/src/components/composer/*` |
| WebSocket | Active old composable `frontend/src/composables/useWebSocket.ts`; newer manager `frontend/src/core/websocket/*` |
| Admin/settings | `frontend/src/views/admin/*`, `frontend/src/components/settings/*` |

## 7. What already works well

- The backend route surface is explicit and broad across native and v4 APIs.
- Security hardening has improved compared with older internal audits: IP rate limiting is implemented, query-token WebSocket auth is rejected, OAuth token delivery is forced to cookie, and production HTTPS/CORS validation exists.
- WebSocket internals now include connection-limit enforcement, presence lifecycle, session resumption, message replay buffering, and subscription cleanup tests.
- The frontend has a centralized fetch-based HTTP client, dependency policy checks, and a meaningful unit/component test suite.
- CI has path-filtered backend/frontend checks, integration workflow, CodeQL, cargo audit, cargo deny, npm audit, dependency review, scorecard, release, and Docker workflows.
- File validation checks extensions, size limits, signatures for images/PDF/ZIP, UTF-8 for text, and basic SVG script rejection.

## 8. Critical implementation gaps

### Finding F-001: Native file upload allows unauthorized channel association

- Severity: P0
- Affected area: files, authorization, channel privacy
- Related spec/contract/ADR: `docs/architecture.md` file access flow; README "authenticated file access"
- Evidence from code: `backend/src/api/files.rs:95-162` writes uploaded bytes, validates content, uploads to S3, then stores `query.channel_id` without checking `ChannelRepository::is_channel_member`; v4 does check membership in `backend/src/api/v4/files.rs:153-162`.
- Evidence from docs: `docs/architecture.md:147-151` says downloads are authenticated through the backend; README says authenticated file access is a security property.
- Implementation status: partially implemented; v4 upload enforces membership, native upload does not.
- Test status: not confirmed; no specific native unauthorized channel file-association test found.
- Why it matters: a user can attach metadata for a private channel they do not belong to if they know or guess a channel UUID, creating access-control and data-integrity risk.
- Expected stable behavior: file upload with `channel_id` must require channel membership before storing metadata or returning any file reference.
- Suggested fix: in native upload and presign paths, verify channel membership before creating S3 keys and DB rows; reject if not a member.
- Recommended tests: integration test where user A uploads with private channel ID owned by user B and receives 403; verify no DB row and no accessible S3 object reference.

### Finding F-002: Native file APIs return presigned S3 URLs despite proxy-only documentation

- Severity: P0
- Affected area: files, deployment, security posture
- Related spec/contract/ADR: `docs/architecture.md` file upload flow
- Evidence from code: `backend/src/api/files.rs:164-166` returns a presigned download URL after upload; `backend/src/api/files.rs:237-250` returns presigned upload URLs.
- Evidence from docs: `docs/architecture.md:150-151` says the client downloads through the backend and there are "no presigned URL leaks to end users"; README security section makes the same claim at a high level.
- Implementation status: contradicted.
- Test status: not confirmed for "no presigned URL exposed".
- Why it matters: presigned URLs can bypass backend authorization during their validity window and make reverse-proxy, audit, and revocation behavior weaker.
- Expected stable behavior: client receives stable authenticated RustChat URLs, not direct S3 URLs, unless the docs explicitly define and secure presigned URL semantics.
- Suggested fix: either proxy downloads as documented or revise docs and add short TTL, scope, audit, and revocation constraints.
- Recommended tests: upload and download tests asserting returned URLs are RustChat API paths when proxy-only mode is intended.

### Finding F-003: v4 post action endpoints acknowledge unsupported mutations

- Severity: P1
- Affected area: Mattermost compatibility, API contract, data integrity
- Related spec/contract/ADR: `docs/compatibility-scope.md`, `docs/reference/compatibility-matrix.md`
- Evidence from code: `backend/src/api/v4/posts.rs:515-570` checks membership for move/restore/reveal/burn but returns `status_ok()` without moving, restoring, revealing, or burning; `rewrite_post` returns input text unchanged at `backend/src/api/v4/posts.rs:577-583`.
- Evidence from docs: compatibility docs state mobile-critical compatibility is implemented while enterprise/stub gaps are documented generally, but these routes are mounted as normal handlers rather than explicit 501 stubs.
- Implementation status: partial/stubbed but success-shaped.
- Test status: not confirmed for side effects.
- Why it matters: clients and tests can treat success responses as completed operations, causing data drift and user-visible false positives.
- Expected stable behavior: unsupported mutation routes should return explicit 501/unsupported or perform the actual mutation atomically.
- Suggested fix: convert to explicit `mm_not_implemented` until implemented, or implement side effects with permission checks and tests.
- Recommended tests: assert DB state changes for implemented actions; assert 501 for intentionally unsupported actions.

### Finding F-004: Scheduled post creation lacks channel membership check

- Severity: P1
- Affected area: posts, authorization, scheduled jobs
- Related spec/contract/ADR: permission/security model
- Evidence from code: `backend/src/api/v4/posts.rs:1005-1038` parses channel ID and inserts scheduled post via repository without a visible membership or post-create permission check; update/delete check owner but not channel membership.
- Evidence from docs: `docs/security-model.md:24-33` says permissions are checked at the API handler layer before services.
- Implementation status: partially implemented.
- Test status: not confirmed.
- Why it matters: scheduled post creation can become a write primitive into channels a user should not access.
- Expected stable behavior: scheduled posts require channel membership and post-create permission for the target channel.
- Suggested fix: call the same channel membership/post permission checks used for immediate post creation before repository writes.
- Recommended tests: unauthorized scheduled post create/update against private channel returns 403 and creates no row.

### Finding F-005: Mention and unread semantics are string-matching based

- Severity: P1
- Affected area: unreads, notifications, mentions
- Related spec/contract/ADR: README mentions, unread tracking, notification preferences
- Evidence from code: `backend/src/services/unreads.rs:145-160` and `backend/src/services/unreads.rs:237-253` count mentions using SQL `LIKE '%@' || username || '%'`, `LIKE '%@all%'`, `LIKE '%@channel%'`, and `LIKE '%@here%'`.
- Evidence from docs: README advertises mentions and unread tracking as core product behavior at `README.md:51-63`; notification settings UI exposes mention keywords in `frontend/src/components/settings/notifications/NotificationsTab.vue`.
- Implementation status: partial.
- Test status: not confirmed for false positives/false negatives, `@here` online filtering, channel mention preferences, or word-boundary behavior.
- Why it matters: stable chat systems must avoid notifying users on substrings and must respect `@here` online/presence, channel mention suppression, and custom keywords.
- Expected stable behavior: parser-backed mention extraction stored with posts or computed from tokens; exact user IDs; preference-aware notification fanout.
- Suggested fix: introduce a mention extraction layer and persist mention targets; update unread/push/email paths to use it.
- Recommended tests: `@ann` does not mention `anna`; code blocks do not trigger mentions if product policy says they should not; `@here` only counts eligible online users; channel-level `ignore_channel_mentions` is honored.

### Finding F-006: Configuration docs are stale for removed security options and renamed S3 option

- Severity: P1
- Affected area: deployment, operations, security
- Related spec/contract/ADR: `docs/admin/configuration.md`, `docs/security-model.md`
- Evidence from code: `backend/src/config/mod.rs:792-804` requires `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie` and rejects `RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=true`; code uses `s3_public_endpoint`, and Compose uses `RUSTCHAT_S3_PUBLIC_ENDPOINT`.
- Evidence from docs: `docs/admin/configuration.md:51` lists `RUSTCHAT_S3_PUBLIC_URL`; `docs/admin/configuration.md:85-86` says query-token auth defaults to true and OAuth delivery can be header or cookie; `docs/security-model.md:18` says query-string tokens are disabled by default in production, not removed.
- Implementation status: docs contradicted by code.
- Test status: config unit tests exist, but doc consistency is not tested.
- Why it matters: admins following docs can deploy configs that fail startup or assume insecure transports still exist.
- Expected stable behavior: docs and examples match accepted environment variables and fail-fast behavior.
- Suggested fix: update current docs to remove query-token/header delivery options and rename `RUSTCHAT_S3_PUBLIC_URL` to `RUSTCHAT_S3_PUBLIC_ENDPOINT`.
- Recommended tests: config docs lint that extracts `RUSTCHAT_*` names from docs and checks against config/compose allowlist.

### Finding F-007: README overstates enterprise/preview maturity

- Severity: P1
- Affected area: product docs, release readiness
- Related spec/contract/ADR: README, compatibility scope
- Evidence from code: SAML and LDAP v4 endpoints return 501-style stubs in `backend/src/api/v4/saml.rs` and `backend/src/api/v4/ldap.rs`; plugin management returns `mm_not_implemented` in `backend/src/api/v4/plugins.rs`; some advanced post actions are success-shaped stubs.
- Evidence from docs: `README.md:65-69` advertises SSO, granular permissions, audit logs, API keys; `README.md:232-254` marks core platform/calls/admin/mobile support as implemented; limitations mention plugins/custom attributes/bots but not SAML/LDAP or advanced action stubs.
- Implementation status: partially implemented but overstated.
- Test status: coverage exists for many admin and v4 paths but not full enterprise behavior.
- Why it matters: beta users need accurate maturity signals for deployment risk.
- Expected stable behavior: README separates "implemented", "partial", "compatibility stub", and "not supported" with no ambiguity.
- Suggested fix: downgrade high-level claims or add precise caveats and link to compatibility scope.
- Recommended tests: not code tests; add release checklist requiring README/status docs to be reviewed against stub inventory.

### Finding F-008: Frontend WebSocket architecture is split between old and new clients

- Severity: P2
- Affected area: frontend state, realtime stability
- Related spec/contract/ADR: `docs/architecture/overview.md` frontend feature structure and WebSocket flow
- Evidence from code: active composable `frontend/src/composables/useWebSocket.ts` connects to `/api/v4/websocket` and handles most events; newer `frontend/src/core/websocket/WebSocketManager.ts` also exists but is not the primary runtime path and assumes a different event handler registration model.
- Evidence from docs: `docs/architecture/overview.md:100-119` says v1 is for the RustChat web app and v4 is for Mattermost clients; current frontend composable uses v4.
- Implementation status: functional but inconsistent.
- Test status: WebSocket disconnection E2E exists; broad event/state parity tests are limited.
- Why it matters: duplicate websocket paths increase the chance of lost events, inconsistent normalization, and untested refactor leftovers.
- Expected stable behavior: one documented frontend WebSocket transport with event normalization and subscription lifecycle tests.
- Suggested fix: designate the active path, remove or finish migration of the inactive path, and update architecture docs.
- Recommended tests: unit tests for event normalization plus E2E for reconnect snapshot, post edit/delete, reaction, channel update/delete, read marker, and presence.

### Finding F-009: Local frontend dependency tree can drift from manifest

- Severity: P2
- Affected area: CI reproducibility, supply chain
- Related spec/contract/ADR: `docs/adr/ADR-frontend-supply-chain-security.md`, `.github/workflows/ci.yml`
- Evidence from code/commands: `npm run deps:inventory` failed locally with `ELSPROBLEMS` for stale installed versions such as Tiptap, date-fns, dompurify, postcss, vitest, and vue-tsc.
- Evidence from docs: ADR requires lockfile-only install and dependency inventory artifact; CI runs `npm ci --ignore-scripts` before inventory.
- Implementation status: policy implemented; local workspace stale.
- Test status: CI should be resilient because it runs `npm ci`, but local stale installs can produce misleading audit results.
- Why it matters: dependency inventory is a release artifact; stale local installs can hide or invent dependency issues.
- Expected stable behavior: inventory generated from a clean `npm ci --ignore-scripts` tree.
- Suggested fix: rerun `npm ci --ignore-scripts` before local inventory checks; consider making `deps:inventory` fail with a clearer "run npm ci" message.
- Recommended tests: CI already covers clean install; add local docs note.

### Finding F-010: Test coverage does not yet match beta-risk surface

- Severity: P2
- Affected area: QA, release readiness
- Related spec/contract/ADR: `docs/development/testing.md`, `.governance/risk-tiers.yml`
- Evidence from code: backend has many tests under `backend/tests/`; frontend has unit tests and five main E2E specs; high-risk flows like native file authorization, scheduled posts, advanced v4 stubs, admin SSO, calls/SFU, file upload/download UX, and permission failure UX are not confirmed covered.
- Evidence from docs: `docs/internal/implementation-gap-analysis-2026-05-22.md` also flags frontend E2E gaps and under-tested realtime/calls/storage paths.
- Implementation status: broad but incomplete.
- Test status: partial.
- Why it matters: stable self-hosted chat relies on regressions being caught before deployment.
- Expected stable behavior: critical flows have integration/E2E coverage and contract tests.
- Suggested fix: add targeted integration/E2E tests for P0/P1 findings before adding new feature scope.
- Recommended tests: see each finding; prioritize file auth, scheduled posts, read/unread, permissions, WebSocket reconnect/event replay, and admin settings.

## 9. Stale, conflicting, or incomplete documentation

| Doc | Drift |
|---|---|
| `docs/admin/configuration.md` | Lists removed/invalid `RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=true` default and header OAuth token delivery; lists `RUSTCHAT_S3_PUBLIC_URL` while code/Compose use `RUSTCHAT_S3_PUBLIC_ENDPOINT`. |
| `docs/security-model.md` | Says query-string WebSocket tokens are disabled by default in production; code removes/rejects them globally. Says active sessions are stored in Redis, but confirmed code primarily uses JWT plus Redis presence/rate/connection state; per-user session revocation was not confirmed. |
| `docs/architecture/overview.md` | Says web app uses `/api/v1/ws`; current frontend uses `/api/v4/websocket`. |
| `docs/architecture.md` | File flow says authenticated backend proxy and no presigned leaks; native API returns presigned URLs. |
| `README.md` | High-level "implemented" lists are stronger than current compatibility/stub reality. |
| `docs/development/testing.md` | Overview says frontend has E2E only and no unit/component framework, but `frontend/package.json` now has `vitest` unit/component tests and CI runs them. |
| `docs/internal/code-quality-audit-2026-05-19.md` | Correctly archived/internal but now stale on rate limiting: current `backend/src/middleware/rate_limit.rs` implements Redis-backed IP limits. |
| `docs/architecture/data-model.md` | Simplified schema omits many current columns/tables and includes a `sessions` table not found in current migrations; useful as conceptual overview only. |

## 10. Mattermost-class maturity comparison matrix

Mattermost is used here only as a stable self-hosted chat maturity baseline.

| Capability | RustChat status | Severity if gap | Evidence |
|---|---|---|---|
| Users/profiles | Mostly implemented | P2 | `backend/src/api/users.rs`, v4 users, frontend profile/settings |
| Login/session | Implemented JWT; revocation not confirmed | P1 | auth modules, security docs |
| Teams/workspaces | Implemented | P2 | teams APIs/migrations/frontend |
| Public/private channels | Implemented | P1 | channel APIs/tests |
| Direct/group messages | Implemented | P1 | direct/group channel code and tests |
| Channel membership | Implemented, but file upload bypass found | P0 | channel repositories vs native file upload |
| Roles/permissions | Implemented but uneven | P1 | auth policy, API checks, frontend capabilities |
| Message send/edit/delete | Implemented | P1 | post APIs/services |
| Threads/replies | Implemented | P2 | thread APIs/components |
| Reactions | Implemented | P2 | reactions migration/API/frontend |
| Unread counters/read markers | Implemented but semantics fragile | P1 | `services/unreads.rs` |
| Mentions | Partial string matching | P1 | SQL `LIKE` mention logic |
| Notification preferences | UI/backend prefs exist; delivery completeness not confirmed | P2 | preferences, push/email |
| File attachments | Implemented with P0 auth/proxy gaps | P0 | file APIs |
| Channel file list/browser | Basic file search/list paths exist; UX completeness not confirmed | P2 | v4 files search, frontend file previews |
| Message search | Basic, not advanced | P2 | compatibility docs caveat |
| Pinned/saved messages | Implemented | P3 | post pin/save routes and panels |
| Permalinks | Not confirmed | P2 | no clear current evidence found |
| Typing indicators | Implemented | P3 | websocket core/frontend |
| Presence/online status | Implemented | P2 | websocket core/status APIs |
| Admin controls | Broad but partial | P1 | admin APIs/frontend |
| Moderation controls | Partial/not confirmed | P2 | delete/pin roles; no full moderation model confirmed |
| Audit logs | Present, completeness not confirmed | P2 | `audit_logs`, admin audit APIs |
| Import/export/backup | Backup docs exist; imports/exports v4 present, completeness not confirmed | P2 | admin backup docs, v4 imports_exports |
| Rate limiting/abuse | Implemented | P1 | middleware/services |
| Security headers | Implemented | P1 | `middleware/security_headers.rs` |
| Deployment readiness | Docker Compose and production docs exist; docs drift remains | P1 | compose files/docs |
| CI/test readiness | Strong foundation; gaps remain | P2 | workflows/tests |

## 11. Backend consistency findings

- Architecture matches the documented Axum/SQLx/Redis/S3 split.
- Route surface is extensive, but not all mounted routes are equally mature. Prefer explicit 501 for unsupported v4 compatibility routes over success-shaped stubs.
- Several authorization checks are strong and local to handlers/services, but the native file upload and scheduled post create paths need parity with the rest of the system.
- Background tasks are spawned during router construction: membership reconciliation, keycloak sync if enabled, calls event listener, unread reconciler, status expiry. Tests that construct routers may exercise background paths implicitly.
- SQL query construction in native post listing is parameterized for values; dynamic SQL is limited to server-chosen clauses and placeholders.
- Observability exists through `tracing`, metrics modules, and admin health, but operational completeness was not fully verified.

## 12. Frontend consistency findings

- The frontend has a modern feature/store/API split and a working build.
- Active WebSocket code is the older `frontend/src/composables/useWebSocket.ts`, not the newer `core/websocket/WebSocketManager.ts` path. This should be documented or consolidated.
- API calls generally go through the centralized `HttpClient`; `rg "fetch(" frontend/src` found only the internal client.
- UI permission logic is partially role-derived and partially membership-derived. It can hide or show actions optimistically, but backend remains authoritative.
- Frontend test coverage is useful but narrow relative to the product surface.
- ESLint passes but reports 200 warnings, including many `any` types and several `vue/no-v-html` warnings. Markdown rendering appears to use DOMPurify elsewhere, but each `v-html` site should be kept under sanitizer tests.

## 13. API contract consistency findings

- Native `/api/v1` and v4 `/api/v4` share many backend services but do not always share authorization semantics, as shown by native vs v4 file upload.
- v4 compatibility contracts exist for core shapes but do not prevent success-shaped stubs.
- Frontend native API calls generally match backend routes for posts/channels/files.
- Some frontend interfaces still include compatibility aliases (`root_post_id`, `parent_id`, `edit_at`, `edited_at`) that are pragmatic but should be tested.

## 14. WebSocket event consistency findings

- Backend v4 WebSocket mapping is robustly unit-tested for several event shapes.
- Current web frontend uses `/api/v4/websocket`, despite architecture docs assigning v1 to the web app.
- Backend supports hello, reconnect snapshots, typing, status, post events, reactions, unreads, and calls plugin events.
- Reconnect behavior has dedicated frontend E2E and backend unit tests, but full state reconciliation under dropped events remains a stability risk.

## 15. Data model and migration findings

- Migrations are extensive and cover core entities, unread/read state, files, reactions, threads, categories, custom profiles, calls, email, SSO, audit logs, and entity/API key support.
- `docs/architecture/data-model.md` is a simplified snapshot and is stale relative to current migrations.
- Some migration patterns are additive, but migration rollback safety is not documented beyond "irreversible".
- No migration squashing/recovery policy was confirmed.

## 16. Permission/security findings

- P0: native file upload lacks channel membership enforcement.
- P0: native file flow exposes presigned URLs despite proxy-only security claim.
- P1: scheduled post creation needs membership/post-create authorization.
- P1: docs describe removed insecure WebSocket/OAuth token delivery options.
- Rate limiting is implemented and tested at helper level.
- Security headers exist and are unit-tested.
- JWT token revocation per user/session was not confirmed; zero-trust docs acknowledge global secret rotation as the main revocation mechanism.

## 17. UX/product completeness findings

- Core chat UX exists: channels, messages, composer, threads, reactions, files, search panels, pinned/saved panels, settings, admin.
- README and screenshots convey a polished product, but docs should distinguish complete, partial, compatibility stub, and not implemented states.
- Search is useful but basic. Compatibility docs say this; README says "Powerful search" without the caveat.
- Admin settings UI has many pages. Not all controls were confirmed functional end-to-end.
- Mobile compatibility is a first-class goal, but full Mattermost parity is explicitly not the same as mobile-critical parity.

## 18. Test coverage findings

Commands discovered from `README.md`, `backend/Cargo.toml`, `frontend/package.json`, `.github/workflows/*.yml`, Docker Compose files, and `docs/development/testing.md`.

Local results:

| Command | Result |
|---|---|
| `cd backend && cargo fmt --all -- --check` | Passed |
| `cd backend && cargo test --lib --no-fail-fast -- --nocapture` | Passed: 143 tests |
| `cd backend && cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `cd frontend && npm run check:dependency-policy` | Passed |
| `cd frontend && npm run format:check` | Passed |
| `cd frontend && npm run lint` | Passed with 200 warnings |
| `cd frontend && npm run test:unit` | Passed: 19 files, 108 tests |
| `cd frontend && npm run build` | Passed |
| `cd frontend && npm run deps:inventory` | Failed with `ELSPROBLEMS`; installed `node_modules` tree is stale |
| `docker compose -f docker-compose.integration.yml up -d` | Passed |
| Full backend integration suite | Not confirmed in this audit. A sandboxed attempt reached 143 passing backend unit tests, then integration targets failed because the sandbox could not connect to local PostgreSQL (`Operation not permitted`). The suite was not rerun outside the sandbox because this environment is the running QA system. |

Coverage gaps to prioritize:

| Gap | Recommended test |
|---|---|
| Native file upload authorization | Backend integration |
| Presigned URL/proxy behavior | Backend integration + API contract |
| Scheduled post authorization | Backend integration |
| v4 success-shaped stubs | Contract tests asserting mutation or 501 |
| Mention parsing/unread counts | Service/integration tests with edge-case messages |
| WebSocket reconnect replay | Backend integration + frontend E2E |
| Admin SSO/email/audit settings | Backend integration + frontend E2E |
| Calls/SFU | Integration tests beyond signaling smoke |

## 19. Deployment/operations findings

- Docker Compose default, production, fast, and integration files exist.
- Production Compose sets strict security defaults for HTTPS site URL, explicit CORS, cookie OAuth delivery, and query-token WebSocket disabled.
- Frontend Dockerfile uses `npm ci --ignore-scripts`, applies dependency patches, and builds with Node 24.
- Backend Dockerfile uses Rust 1.95 Alpine, locked release build, SQLX offline build, non-root runtime user, and healthcheck.
- Meilisearch is available behind a `search` profile; default search implementation still appears to be database-backed/basic for posts.
- Backup/restore docs exist; actual restore scripts were not audited in depth.
- Push proxy is a separate service with APNS/FCM configuration docs; delivery completeness was not verified.

## 20. Prioritized implementation roadmap

### P0: Blocks stable beta usage

1. Enforce native file upload/presign channel membership before S3 upload and DB row creation.
2. Decide and implement file delivery policy: backend proxy-only or documented presigned URLs. Align docs, code, and tests.

### P1: Required for serious public preview

1. Fix scheduled post create/update/delete authorization.
2. Convert success-shaped v4 stubs to explicit 501 or implement real side effects.
3. Replace mention/unread string matching with parser-backed mention targets or clearly document basic semantics.
4. Update deployment/security/config docs to match current env vars and removed token transports.
5. Calibrate README "implemented" claims against compatibility/stub reality.
6. Add critical integration tests for file auth, scheduled posts, v4 stubs, mention/unread behavior, and permission failures.

### P2: Important after preview

1. Consolidate frontend WebSocket transport or document migration state.
2. Expand frontend E2E to files, threads, search, admin, permissions, calls, and reconnect replay.
3. Improve search beyond basic matching or explicitly brand it as simple search.
4. Add contract tests that verify v4 route side effects, not just response shapes.
5. Add docs/env consistency linting.

### P3: Polish/future

1. Reduce frontend ESLint warnings and isolate/sanitize `v-html` sites with tests.
2. Refresh `docs/architecture/data-model.md` from current migrations.
3. Add operational docs for migration recovery, database backup verification, and incident/session revocation.
4. Add performance/load tests for channels, posts, WebSocket fanout, search, and unreads.
