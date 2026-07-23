# ADR-004: Adopt Buzz as the RustChat Upstream Foundation

**Date:** 2026-07-23
**Status:** Proposed — becomes Accepted when merged
**Risk tier:** architectural

## Context

RustChat was created as a standalone Rust collaboration platform with a Vue web client and Mattermost-compatible APIs because no suitable Rust-based, agent-native upstream existed at the time.

Buzz now provides a broader collaboration substrate with a Rust relay, signed event model, channels, threads, DMs, agents, workflows, Git collaboration, desktop clients, and Flutter mobile clients. RustChat has no customers and no production data migration requirement. Continuing both the standalone RustChat stack and a Buzz-based product would duplicate effort and divide a small team across messaging, clients, mobile, agents, workflows, and enterprise functionality.

Kubedo's intended differentiation is enterprise and sovereign operation rather than ownership of every chat primitive. The differentiated areas are identity lifecycle, Keycloak integration, policy, compliance, managed deployment, RustShare knowledge, permission-aware RAG, support, and EU-oriented operations.

## Decision

RustChat will adopt `block/buzz` as its upstream collaboration foundation.

The following decisions are included:

1. The original RustChat implementation will receive one final standalone release and then enter security-maintenance-only status.
2. Mattermost compatibility is retired from the forward architecture.
3. The Vue frontend is retired from forward development after the final standalone release.
4. RustChat desktop, web, Android, and iOS clients will derive from Buzz clients and will be rebranded and tested as RustChat distributions.
5. Kubedo will maintain a minimal fork of Buzz for branding, packaging, configuration hooks, and changes that cannot yet be accepted upstream.
6. General-purpose fixes and features will be proposed upstream first.
7. RustChat enterprise functionality will live outside Buzz core and communicate through documented contracts.
8. RustShare will provide permission-aware company knowledge through MCP, APIs, events, and explicit identity mappings.
9. Buzz protocol semantics, event kinds, relay authorization, and schema ownership will not be changed downstream without a separate accepted ADR.
10. RustChat will transparently acknowledge Buzz as its upstream foundation and preserve required Apache-2.0 attribution.

## Architecture boundary

```text
RustChat clients (Buzz-derived)
          |
          | Buzz protocol and supported APIs
          v
Kubedo Buzz fork  <----- regularly synchronized ----- block/buzz
          |
          | documented contracts only
          v
RustChat enterprise services
  - identity lifecycle and Keycloak integration
  - organization and policy control plane
  - RustShare MCP/RAG bridge
  - retention, audit presentation, and export
  - deployment, backup, restore, monitoring, and support
```

## Alternatives considered

### Continue standalone RustChat

Rejected because it requires Kubedo to independently maintain a collaboration backend, web UI, mobile compatibility, desktop experience, agents, workflows, and realtime behavior while also developing RustShare, O3K, and CellHV.

### Embed selected Buzz crates into the existing backend

Rejected because it would preserve two competing domain models and create a hybrid architecture with unclear ownership of identity, messages, events, permissions, and storage.

### Copy Buzz into this repository and modify it freely

Rejected because copied source would make upstream synchronization difficult and would encourage silent protocol and schema divergence.

### Run RustChat and Buzz as permanent peer systems

Rejected as the target architecture because it would create duplicated users, channels, messages, clients, and operational responsibility. A temporary bridge is allowed only during evaluation or staged bootstrap.

## Consequences

### Positive

- Kubedo inherits a larger and faster-growing collaboration core.
- Agent, workflow, Git, desktop, and mobile development can be shared with an upstream community.
- RustChat can focus on enterprise and sovereign differentiation.
- There is no customer migration cost at the time of decision.
- The architecture becomes more aligned with human-and-agent collaboration.

### Negative

- RustChat becomes dependent on Buzz architecture and release quality.
- Kubedo must continuously synchronize and test its fork.
- Buzz is still young and some mobile, push, workflow, and huddle functions remain incomplete.
- Rebranding and distributing desktop/mobile clients creates ongoing release and store obligations.
- Enterprise identity must coexist with Buzz cryptographic identity rather than replacing it casually.

### Risks and mitigations

- **Deep fork risk:** enforce a patch ledger, patch budget, architecture guard, and upstream-first policy.
- **Upstream abandonment risk:** preserve the legal and technical ability to maintain the Apache-2.0 fork independently.
- **Protocol divergence risk:** run upstream conformance tests unchanged and prohibit downstream event/schema changes without ADR approval.
- **LLM-generated architecture drift:** require contracts, tests, and human architectural review for every generated change.
- **Mobile immaturity risk:** treat mobile as a gated product workstream, not an inherited finished feature.

## Acceptance criteria

This ADR is considered successfully implemented when:

1. The final standalone RustChat release is tagged and preserved.
2. `kubedoio/buzz` exists as a tracked fork with an upstream-sync process.
3. The RustChat repository contains no copied Buzz source directory.
4. Enterprise services use documented contracts and pass conformance tests.
5. Two upstream update cycles complete without unplanned protocol or schema divergence.
6. Branded clients pass desktop and mobile release gates.
