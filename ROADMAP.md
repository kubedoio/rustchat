# RustChat Roadmap

This roadmap describes the direction of RustChat as a continuing, independently
developed self-hosted collaboration platform. It is a living document and will
be updated as priorities evolve.

> RustChat is under active development and moving toward a supported
> self-hosted collaboration product. It is not yet production-ready for all
> organizations.

## Current Phase — Production-Readiness and Repository Integrity

**Theme**: Close the known production blockers, make repository/release signals
authoritative, and publish the pending `v0.5.1` release as a normal continuing
RustChat release.

Priorities:

- [~] **Repository integrity hardening** — Implement
      [ADR-006](docs/adr/ADR-006-repository-integrity-and-maintainability.md):
      authoritative green-main/security aggregation, verified artifact
      promotion, GitHub protection, release-tag enforcement, and bounded
      maintainability guards. — **In progress:** P001–P005 (green-main gates,
      verified protection, persistence-boundary guard, module-growth guard,
      docs/repository hygiene) are merged; P006 (migration/recovery evidence),
      P007 (Buzz compatibility record), and P008 (release verdict) remain.
- [ ] **Publish v0.5.1** — Version metadata is already at `0.5.1` in source;
      validate release gates (backend, frontend, push-proxy, Docker Compose,
      migration/recovery evidence, smoke tests) and publish the tagged release.
      (A `v0.5.0` tag exists but no `v0.5.0` GitHub Release has been published;
      the latest published release is `v0.4.1`.)
- [ ] **Realtime durability** — Durable event replay/outbox for reconnecting
      clients, graceful WebSocket shutdown, and explicit resync signaling
      instead of silent event drops.
- [ ] **Authorization completeness** — Close remaining membership-revocation
      and scheduled-post authorization gaps.
- [ ] **Audit and observability** — Broaden audit-log coverage beyond the
      current minimal set and keep readiness/migration state accurate.
- [ ] **Message validation and pagination** — `client_msg_id` idempotency,
      cursor pagination on hot channel-history paths.
- [x] **Rate limiting coverage** — File upload and search endpoints (wired: `upload_ip_rate_limit` and `websocket_ip_rate_limit` in `backend/src/api/v4/mod.rs`, `search_ip_rate_limit` in `backend/src/api/search.rs`; the Redis-backed service tests in `backend/tests/test_rate_limiting.rs` cover the limiting service — endpoint-level middleware tests are not yet present).
- [x] **Defects** — Resolve the open compliance-export and audit-dashboard
      bugs honestly (working behavior or explicitly unsupported).
      — **Resolved (2026-09-28):** compliance export is explicitly
      unsupported in the admin UI (phantom endpoint removed; the feature is
      Phase 2); the audit-dashboard JSON export was fixed (it previously
      downloaded `{}` because a Blob was passed to `JSON.stringify`, and
      its request failed with a 400 because the raw filter object —
      empty-string params and bare `yyyy-MM-dd` dates — violated the
      backend's `AuditLogQuery` deserialization contract; both defects
      are fixed via a param builder shared with the list request, with a
      wire-format regression test); and the
      dashboard is relabeled honestly as "Membership Policy Audit" since all
      of its data (summary cards, failure stats, log table) comes from the
      membership-policy audit table. The system audit log keeps its own view.

## Next — Operational Hardening and Incremental Architecture Cleanup

- [x] **Persistence-boundary guard** — Prevent new direct SQL persistence from
      accumulating in API handlers; migrate existing debt only in bounded,
      tested slices. (RI-C04 active; pilot: `backend/src/api/site.rs`.)
- [x] **Module growth control** — Add responsibility/growth baselines and
      decompose only the highest-value oversized modules. (RI-C05 active;
      pilot: `backend/src/services/posts.rs`.)
- [ ] **Test coverage** — Expand backend integration tests and frontend E2E
      coverage.
- [ ] **Migration upgrade matrix** — Prove empty→latest and latest published
      stable→latest upgrades with application readiness evidence.
      (Scope note: CI already proves schema convergence continuously —
      empty→HEAD and latest-published-stable→HEAD via
      `scripts/migration-matrix.sh`, see `docs/development/ci-gates.md`.
      What remains open is the release-grade matrix with full application
      readiness evidence, per `docs/repo-current-state.md`.)
- [x] **Backup & restore** — Documented and tested data protection procedures.
      (Documented: docs/operations/runbook.md "Backup Procedures" and
      "Restore Procedure and Verification". Tested: the CI **Backup Restore**
      job runs `scripts/backup-restore-check.sh` on schema/script changes,
      proving a custom-format `pg_dump` restores into a fresh database with
      identical schema objects, sqlx migration bookkeeping, row counts,
      content checksums, and referential integrity. Production backup
      scheduling remains operator-side.)
- [ ] **Observability** — Structured metrics, health checks, and alerting
      guides.
- [ ] **Search improvements** — Better indexing, filtering, and performance.

## Integration Ecosystem

- [x] **Buzz integration phase 1 (optional outbound bridge)** — Isolated,
      opt-in connector with a durable outbox; RustChat remains authoritative and
      Buzz availability does not affect core posting
      ([ADR-005](docs/adr/ADR-005-rustchat-core-and-buzz-integration.md)).
- [ ] **Buzz compatibility verification** — Pin and test the external protocol
      contract used by RustChat without depending on Buzz internals or latest
      upstream HEAD.
- [ ] **Plugin framework** — Move beyond compatibility stubs to a working
      plugin model.
- [ ] **RustShare deepening** — Richer permission-aware knowledge sync for
      agents.
- [ ] **Mattermost API v4 parity** — Continue expanding mobile client
      compatibility coverage where it serves users.

## Later — Scaling, Compliance, and 1.0

- [ ] **Multi-team hardening** — Team isolation and cross-team features.
- [ ] **Compliance** — Working compliance export, retention policy
      enforcement, and audit completeness.
- [ ] **Federation research** — Evaluate server-to-server messaging
      protocols and interoperability options.
- [ ] **Performance at scale** — Database query optimization, caching
      strategy, distributed SFU mesh for calls.
- [ ] **1.0 stable release** — Declare production readiness with an LTS
      support policy.

## What We Are Not Planning

To set clear expectations, the following are not on the current roadmap:

- SaaS hosting by the core team (RustChat is strictly self-hosted)
- Native desktop or mobile apps (we target Mattermost mobile app
  compatibility instead)
- Commercial plugin marketplace
- Replacing the RustChat backend, frontend, or data model with any external
  collaboration system

Mattermost compatibility remains a supported RustChat strategy unless
explicitly changed by a future ADR.

## How to Influence the Roadmap

- Open a [feature request](https://github.com/kubedoio/rustchat/issues/new/choose)
- Start a [discussion](https://github.com/kubedoio/rustchat/discussions)
- For significant architectural proposals, write an ADR and open a PR

## Completed Milestones

| Date | Milestone |
|------|-----------|
| 2026-09 | Runtime/bootstrap architecture cleanup — single AppState, supervised workers, typed runtime composition |
| 2026-09 | Buzz phase-1 external integration — optional outbound bridge with durable outbox and isolated failure semantics |
| 2026-06 | AI Agents & Ecosystem — agent runtime, RAG with pgvector, Tavily tools, analytics and feedback |
| 2026-03 | Entity Foundation Complete — API keys, rate limiting, mobile compatibility (95.1%) |
| 2026-02 | VoIP Push Notifications — Mobile call ringing for Android and iOS |
| 2026-01 | V4 API Coverage — Broad Mattermost compatibility for mobile clients |
| 2025-12 | Real-time WebSocket Layer — Redis-backed pub/sub clustering |
