# RustChat Roadmap

This roadmap describes the direction of RustChat. It is a living document and will be updated as priorities evolve.

> RustChat is under active development and moving toward a supported self-hosted collaboration product. It is not yet production-ready for all organizations.

## Current Phase: Foundation Hardening

**Theme**: Stabilize core functionality, improve compatibility, and make the project approachable for operators and contributors.

### Public Preview Gate

RustChat can move to public preview only when the preview contract is explicit and the critical reliability checks pass in CI. The current gate is:

| Area | Preview status | Requirement before public preview |
|------|----------------|-----------------------------------|
| File uploads and downloads | Blocker until verified | Unauthorized users cannot attach files to channels they cannot access; file responses use RustChat-authenticated URLs; CI covers member and non-member flows. |
| Unsupported compatibility actions | Blocker until verified | Unsupported Mattermost-compatible mutations return explicit `501` responses and frontend surfaces do not expose controls that imply support. |
| Configuration and deployment docs | Blocker until guarded | Public docs and deployment templates must not reintroduce removed query-token or retired S3 public URL settings; docs CI runs the config drift check. |
| Mention and unread semantics | Preview caveat | Current unread paths use token-boundary matching; persisted mention-target storage remains a post-preview hardening item unless scale testing proves it blocks preview. |
| SAML/LDAP, plugins, advanced admin/compliance | Preview caveat | README and compatibility docs must state these are unsupported, stubbed, or future work. |
| Session revocation and audit completeness | Post-preview hardening | Token/session revocation and broader audit coverage require separate design and migration work before stable release. |

### Near-Term (Next 1–3 Months)

- [ ] **Mattermost API v4 Parity** — Expand mobile client compatibility coverage
- [ ] **WebSocket Reliability** — Improve reconnection handling and state synchronization
- [ ] **Call Stability** — Harden SFU signaling and media plane edge cases
- [x] **Repository Polish** — Open-source readiness: docs, CI, security pipeline, governance
- [ ] **Test Coverage** — Expand backend integration tests and frontend E2E coverage

### Medium-Term (3–6 Months)

- [ ] **Plugin Framework** — Move beyond stubs to a working plugin model
- [ ] **Search Improvements** — Better indexing, filtering, and performance
- [ ] **Backup & Restore** — Documented and tested data protection procedures
- [ ] **Observability** — Structured metrics, health checks, and alerting guides
- [ ] **Multi-Team Support** — Harden team isolation and cross-team features

### Long-Term (6–12 Months)

- [ ] **Federation Research** — Evaluate server-to-server messaging protocols
- [ ] **Advanced Admin Tools** — Bulk user management, compliance exports
- [ ] **Performance at Scale** — Database query optimization, caching strategy
- [ ] **1.0 Stable Release** — Declare production readiness with LTS support policy

## What We Are Not Planning

To set clear expectations, the following are not on the current roadmap:

- SaaS hosting by the core team (RustChat is strictly self-hosted)
- Native desktop or mobile apps (we target Mattermost mobile app compatibility instead)
- Commercial plugin marketplace

## How to Influence the Roadmap

- Open a [feature request](https://github.com/rustchatio/rustchat/issues/new/choose)
- Start a [discussion](https://github.com/rustchatio/rustchat/discussions)
- For significant architectural proposals, write an ADR and open a PR

## Completed Milestones

| Date | Milestone |
|------|-----------|
| 2026-03 | Entity Foundation Complete — API keys, rate limiting, mobile compatibility (95.1%) |
| 2026-02 | VoIP Push Notifications — Mobile call ringing for Android and iOS |
| 2026-01 | V4 API Coverage — Broad Mattermost compatibility for mobile clients |
| 2025-12 | Real-time WebSocket Layer — Redis-backed pub/sub clustering |
