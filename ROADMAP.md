# RustChat Roadmap

This roadmap describes the direction of RustChat as a continuing, independently
developed self-hosted collaboration platform. It is a living document and will
be updated as priorities evolve.

> RustChat is under active development and moving toward a supported
> self-hosted collaboration product. It is not yet production-ready for all
> organizations.

## Current Phase — Production-Readiness and Correctness

**Theme**: Close the known production blockers, keep the existing
architecture, and publish the pending `v0.5.1` release as a normal continuing
RustChat release.

Priorities:

- [ ] **Publish v0.5.1** — Version metadata is already at `0.5.1` in source;
      validate release gates (backend, frontend, push-proxy, Docker Compose,
      smoke tests) and publish the tagged release.
- [ ] **Realtime durability** — Durable event replay/outbox for reconnecting
      clients, graceful WebSocket shutdown, and explicit resync signaling
      instead of silent event drops.
- [ ] **Authorization completeness** — Close remaining membership-revocation
      and scheduled-post authorization gaps.
- [ ] **Audit and observability** — Broaden audit-log coverage beyond the
      current minimal set; add migration state to the readiness probe.
- [ ] **Message validation and pagination** — `client_msg_id` idempotency,
      cursor pagination on hot channel-history paths.
- [ ] **Rate limiting coverage** — File upload and search endpoints.
- [ ] **Defects** — Resolve the open compliance-export and audit-dashboard
      bugs honestly (working behavior or explicitly unsupported).

## Next — Operational Hardening and Architecture Cleanup

- [ ] **Runtime composition cleanup** — Extract application bootstrap
      (dependencies, agent runtime, background workers) from the API router
      into a supervised bootstrap layer.
- [ ] **Test coverage** — Expand backend integration tests and frontend E2E
      coverage; migration upgrade tests (empty→latest, snapshot→latest).
- [ ] **Backup & restore** — Documented and tested data protection procedures.
- [ ] **Observability** — Structured metrics, health checks, and alerting
      guides.
- [ ] **Search improvements** — Better indexing, filtering, and performance.

## Then — Integration Ecosystem

- [ ] **Buzz integration (optional)** — An isolated, opt-in connector that
      bridges configured RustChat channels with a Buzz relay over its
      documented external protocol. RustChat remains authoritative; Buzz
      availability never affects RustChat core (see
      [ADR-005](docs/adr/ADR-005-rustchat-core-and-buzz-integration.md)).
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
| 2026-06 | AI Agents & Ecosystem — agent runtime, RAG with pgvector, Tavily tools, analytics and feedback |
| 2026-03 | Entity Foundation Complete — API keys, rate limiting, mobile compatibility (95.1%) |
| 2026-02 | VoIP Push Notifications — Mobile call ringing for Android and iOS |
| 2026-01 | V4 API Coverage — Broad Mattermost compatibility for mobile clients |
| 2025-12 | Real-time WebSocket Layer — Redis-backed pub/sub clustering |
