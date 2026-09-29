# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1] - 2026-06-19

> Note: `0.5.1` has been the source-tree version since 2026-06-19. This section
> was consolidated on 2026-09-29 from all changes merged since then, ahead of
> the release.

### Security
- Default environment is now `production`; permissive development CORS is gated by `RUSTCHAT_ALLOW_DEV_CORS` and rejected in production.
- Resumable file uploads now enforce the same extension/content-type allowlist as multipart uploads.
- Channel call toggles now require channel-management permission.
- Outgoing webhook and slash-command URLs are validated at creation time and resolved/validated at request time, with redirects disabled, to prevent DNS-rebinding SSRF.
- Retention cleanup now deletes S3 objects and includes an optional orphan scanner configured by `RUSTCHAT_RETENTION_ORPHAN_SCAN_*` environment variables.
- Frontend container now runs as the non-root `rustchat` user.
- Email notification bodies now HTML-escape user-controlled content (HTML injection).
- Single-use tokens (invites/password resets) are race-safe; login throttling counts only failed attempts and no longer trusts spoofable proxy IP headers.
- Admin operations are restricted by a role allowlist; retention cleanup runs are bounded.
- OIDC account linking now requires a verified email address.
- Per-user agent trigger limits and an agent-to-agent loop guard bound runaway AI-agent execution.
- Development Docker Compose services bind to loopback by default instead of all interfaces.
- SVG upload screening hardened: general event-handler pattern, active URI schemes (`javascript:`/`data:`), SMIL `href` rewrites, remote `style` `url()` fetches, `<!ENTITY>` declarations, and whitespace/namespaced-element bypasses are all rejected.
- `quinn-proto` updated to 0.11.15 (RUSTSEC-2026-0185); all other outstanding Dependabot security findings cleared.

### Fixed
- Include `client_msg_id` in v1 channel posts list queries so message history loads instead of returning HTTP 500.
- Channel names are validated and empty-state copy improved.
- Realtime hub drops empty user index entries and logs dropped broadcasts instead of silently losing them.
- Push notifications contain the real `post_id`, redact push tokens from logs, and truncate at character boundaries.
- Push proxy refreshes APNS JWTs before expiry and caps nonce tracking.
- Calls register participants recovered via the channel-membership fallback.
- Keycloak/IdP group sync converges roles to the union of active grants (downgrades take effect), survives dangling syncables, and unlink is atomic and serialized against concurrent reconciles.
- Upload sessions are capped per user and purged when expired (database bloat).
- SMTP send time is bounded; the RAG chunker rejects degenerate overlap; `MIGRATIONS_DIR` resolves correctly.
- Knowledge uploads verify content types; mention highlighting no longer corrupts links; RustShare downloads are bounded; email sending recovers from crashes.
- Admin audit-dashboard JSON export downloads the actual entries (it previously saved `{}` because a Blob was serialized) and no longer fails with 400 on filtered exports; the dashboard is relabeled honestly as "Membership Policy Audit".
- Production `unwrap()` calls removed from the hybrid-search sort and security-header construction (fail-fast at startup with the offending field named instead).
- Repository-integrity and CI gate regressions repaired (promotion-gate pagination, migration-matrix append-only guard, CodeQL toolchain handling).

### Added
- AI Agents & Ecosystem feature support: Channel-participant AI agents with LLM providers (GPT models), optional tools integration (Tavily search), pgvector search for RAG knowledge bases, RustShare sync sources, and user feedback tracking (thumbs up/down).
- Comprehensive AI Agents administration documentation and runtime integration guidance.
- Standard Bot accounts creation and management documentation.
- Docker Compose quickstart troubleshooting tips and S3 private bucket security configuration guidance.
- Repository-integrity program with CI enforcement: migration-upgrade-proof matrix (RI-C07), module-growth control (RI-C05), direct-SQL baseline with tripwire guard (RI-C04), and live branch-protection verification via `scripts/verify-protection.sh` (RI-C02).
- Buzz integration as an optional, isolated external bridge (`docs/integrations/`).
- Release-readiness tooling: `scripts/check-release-ready.sh` and `scripts/release-notes-check.sh`.

### Changed
- Refactored and expanded Architecture, User, Security, and Runbook documents to detail the new AI Agents module, PGVector requirements, and optional runtime flags.
- Retired the abandoned Buzz-pivot program: removed `docs/pivot/`, ADR-004, `scripts/pivot/`, and the pivot architecture-guard workflow; ADR-005 now records that RustChat continues as an independent product with Buzz as an optional external integration target.
- Consolidated the documentation hierarchy: single canonical architecture overview, substantive realtime and integrations documentation, new `docs/integrations/` section (RustShare), and merged development guides (`docs/development.md` folded into `docs/development/`).
- Corrected stale documented facts: Rust MSRV (1.95+), Node.js requirement (24+), push-proxy default port, and the distinction between source version (0.5.1) and the latest published release (v0.4.1).
- Rewrote the roadmap around the continuing product, with verified evidence pointers for completed items.
- Fixed repository identity drift: active project links and container registry references now point to `kubedoio/rustchat`.
- Runtime/bootstrap architecture cleanup and dead-code removal (frontend thread-panel decomposition, legacy stores, generated artifacts).
- CI moved to `ubuntu-latest` runners and merge/promotion/release gate integrity restored.
- Dependency refreshes across backend, frontend, push-proxy, Docker base images, and GitHub Actions (Dependabot groups).

### Removed
- Duplicate/pointer documents: `docs/architecture.md`, `docs/architecture/architecture-overview.md`, `docs/architecture/websocket.md`, `docs/MATTERMOST_CLIENTS.md`.
- Dead frontend code: unused thread-panel decomposition, thread composer, and scaffold test.

### Known limitations
- **Compliance export is not implemented** — the admin UI labels it "Not implemented"; only a no-op compatibility stub exists on the backend.
- **No durable realtime replay** — reconnect replay is in-memory and bounded (about 5 minutes / last 128 messages per connection); events missed beyond that window, or across a server restart, are not recovered and require a reload. A durable outbox/replay is planned.
- **Message sending is not idempotent across reconnects** — `client_msg_id` is sent by the frontend but not yet enforced server-side.
- **The SPA is not yet served with an enforcing CSP** — the backend API presets no longer allow inline scripts and the built frontend ships no inline or cross-origin scripts, but nginx serves the SPA document with the target policy in `Content-Security-Policy-Report-Only` only. Promoting it to enforcing requires a manual pass to confirm the known runtime allowances (Cloudflare Turnstile script, embedded call frames).
- **The UI is English-only** — no i18n layer exists yet.

## [0.5.0] - 2026-06-10

### Added
- Channel archive and restore flows now emit Mattermost-compatible system messages and realtime events.
- Frontend support for archive and restore system messages so channel lifecycle changes are visible in conversation history.

### Changed
- Reconciled product documentation and compatibility notes with current implementation gaps and release readiness.
- Tuned dependency update policy and CI behavior for more reliable release preparation.
- Updated backend, frontend, push-proxy, Docker base image, CodeQL, and Scorecard dependencies from the post-RC dependency refresh.

### Fixed
- Synchronized channel archive state between `deleted_at` and `is_archived` to keep API responses and persistence consistent.
- Restored channels now update correctly when websocket payloads use Mattermost channel objects.
- Channel update errors now distinguish duplicate-name and not-found responses more accurately.
- Test notification requests now use the v4 API client path.
- Backend integration test, DCO, Scorecard, and nightly workflow regressions that blocked reliable validation.
- Backend release image builds now use a locked Cargo registry cache and architecture-specific target caches to avoid concurrent multi-platform BuildKit unpack races.
- Release image publishing and PR Docker image checks now run on the self-hosted runner fleet with a Docker socket access preflight.

## [0.5.0-rc.3] - 2026-06-08

### Fixed
- Release image publishing now runs on the self-hosted runner fleet to avoid GitHub-hosted Docker Hub pull timeouts during Buildx setup.

## [0.5.0-rc.2] - 2026-06-01

### Fixed
- Backend release image builds now use a locked Cargo registry cache and architecture-specific target caches to avoid concurrent multi-platform BuildKit unpack races.

## [0.5.0-rc.1] - 2026-05-31

### Added
- Channel archive and restore flows now emit Mattermost-compatible system messages and realtime events.
- Frontend support for archive and restore system messages so channel lifecycle changes are visible in conversation history.

### Changed
- Reconciled product documentation and compatibility notes with current implementation gaps and release readiness.
- Tuned dependency update policy and CI behavior for more reliable release preparation.

### Fixed
- Synchronized channel archive state between `deleted_at` and `is_archived` to keep API responses and persistence consistent.
- Restored channels now update correctly when websocket payloads use Mattermost channel objects.
- Channel update errors now distinguish duplicate-name and not-found responses more accurately.
- Test notification requests now use the v4 API client path.
- Backend integration test, DCO, Scorecard, and nightly workflow regressions that blocked reliable validation.

## [0.4.1] - 2026-05-22

### Fixed
- **DCO Conformance**: Rewrote PR branch history and successfully re-signed all 35 branch commits to satisfy Developer Certificate of Origin (`Signed-off-by`) specifications.
- **GitGuardian Scans**: Added a customized `.gitguardian.yaml` ruleset to prevent false-positive alerts on workflows, test suites, and documentation.

### Security
- **Secret Remediation**: Purged static high-entropy dummy keys from the repository's git commit history to satisfy strict GitGuardian checks.
- **CI Hardening**: Replaced static test environment variables in `.github/workflows/ci.yml` with dynamic runtime key generation (`openssl rand -hex 32`) to prevent key leaks and enhance workflow security.

## [0.4.0] - 2026-05-21

### Added
- **WebSocket Disconnection UX**: Progressive disconnection handling to prevent users from acting on stale data.
  - Three visual states: Reconnecting (< 5s), Disconnected (5-30s), Failed (> 30s).
  - Connection status banner with countdown timer and manual retry option.
  - Full-screen modal with reconnect/refresh actions for extended disconnections.
  - Header connection indicator dot (🟢🟡🟠🔴) showing real-time status.
  - Message composer disabled with tooltip during disconnections.
  - Content dimming (80% → 60%) to indicate potentially stale data.
  - Automatic sync of missed messages and unread counts on reconnect.
- **Channel Management**: Channel creators can now update and delete their channels.
  - Edit channel name, display name, and description via channel context menu.
  - Delete channels with confirmation (soft delete).
  - Real-time updates via WebSocket when channels are modified.
- **Private Channels**: Merged into main Channels sidebar section with lock icon indicator.
- **Browse Channels**: Fixed public channel discovery and joining.
- **Message Notifications**: Browser notifications now show for all new messages, not just mentions.
- **Composer Fix**: Send button now properly enables after attachment upload completes.

### Fixed
- Admin panel team members now load correctly (fixed missing `presence` column in SQL query)
- Thread view now displays replies properly (fixed API response format mismatch)
- Typing indicators now appear when other users are typing (fixed v1 WebSocket message format conversion)
- Real-time message deletion now works correctly (standardized WebSocket payload)

### Changed
- **License**: Changed from MIT to Apache-2.0 across all project metadata.
- **Governance**: Added GOVERNANCE.md, CODE_OF_CONDUCT.md, SUPPORT.md, MAINTAINERS.md, DCO.md, and CONTRIBUTING.md for community-driven development.
- **README**: Added product screenshots, improved quickstart guide, and honest capability disclosures.
- **Security**: Removed hardcoded TURN server defaults and S3 domain references from codebase and migrations.
- **Cleanup**: Removed internal AI tooling files (`.agents/`, `.kimi/skills/`, `.specify/`) from tracked files.
- **CI/CD**: Added OpenSSF Scorecard, security scanning, DCO check, and integration test workflows.

## [0.3.5] - 2026-03-09

### Added
- VoIP Push Notification support for call ringing on mobile devices.
  - Push Proxy service with FCM (Android) and APNS (iOS) support.
  - Data-only FCM messages for Android call notifications (high priority, direct boot).
  - APNS VoIP push support for iOS CallKit integration (prepared, requires credentials).
  - Backend integration with `sub_type: "calls"` for mobile app call identification.
  - Call UUID generation for VoIP session tracking.
- Documentation for mobile push notification architecture and implementation requirements.

### Changed
- Docker Compose configuration to include push-proxy service on port 3001.
- Backend push notification service to route calls through push proxy.
- Version bump to 0.3.5 reflecting significant new features and maturity.

### Security
- Fixed protobuf vulnerability (RUSTSEC-2024-0437) by upgrading prometheus 0.13 -> 0.14.
- Fixed rustls-pemfile warning (RUSTSEC-2025-0134) by upgrading yup-oauth2 11 -> 12.
- Fixed dompurify XSS vulnerability (GHSA-v2wj-7wpq-c8vv).
- Fixed rollup path traversal vulnerability (GHSA-mw96-cpmx-2vgc).
- Updated AWS-LC to latest versions (aws-lc-rs 1.15.3 -> 1.16.1, aws-lc-sys 0.36.0 -> 0.38.0).

## [0.3.1] - 2026-02-12

### Added
- Mobile compatibility analysis and verification artifacts for calls and messaging attachment flows.
- Release version bump across backend and frontend metadata to `0.3.1`.

### Fixed
- Desktop call screen sharing flow stabilization so screen-on/screen-off control paths are functional end-to-end.
- Mattermost mobile calls now start working reliably with improved call signaling/state sync behavior.
- Mobile ringing/notification lifecycle alignment (including dismissal persistence and state refresh behavior).
- Mobile message history attachment visibility after re-login by preserving file metadata in post-list responses.

## [0.3.0] - 2026-02-07

### Added
- CI quality gates for backend and frontend build/test workflows.
- Expanded Mattermost API v4 compatibility coverage and status reporting.
- Calls plugin architecture improvements (state handling, signaling path hardening).
- Stronger deployment documentation and operational guidance.

### Changed
- WebSocket stack rationalization and cleanup for more predictable runtime behavior.
- Release metadata and project versioning updated to `0.3.0`.
- Documentation updated to reflect current implementation status and compatibility scope.

### Fixed
- Multiple test suite and integration issues that blocked reliable validation.
- Semantic compatibility gaps where endpoints existed but behavior was incomplete.
- Configuration and environment drift between docs, compose, and runtime behavior.
- Various reliability and maintainability issues across API and realtime layers.

### Security
- Tighter production posture for default settings and deployment guidance.
- Better separation between development-friendly and production-safe defaults.

### Deployment
- This release is considered deployment-ready for managed environments with proper production configuration (TLS, secrets, database backups, and monitoring).

## [0.0.1] - 2026-01-24

### Added
- Initial working version of RustChat.
- Real-time messaging via WebSockets.
- Thread support.
- Unread messages system.
- S3-compatible file uploads (RustFS).
- User presence and status.
- Organization and Team structures.

### Fixed
- Disappearing messages issue (schema mismatch).
- Thread reply UI duplication.
