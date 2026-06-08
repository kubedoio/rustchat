# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
