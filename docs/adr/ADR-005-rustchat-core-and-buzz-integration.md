# ADR-005: RustChat Core Continuation and Buzz as an Optional Integration Target

**Date:** 2026-09-25
**Status:** Accepted
**Risk tier:** architectural
**Supersedes:** ADR-004 (retired with the Buzz pivot program; see git history)

## Context

ADR-004 proposed freezing the standalone RustChat implementation and rebuilding
RustChat as an enterprise distribution on top of `block/buzz`. That direction
was abandoned before the final "legacy freeze" release was ever published. The
repository now needs an explicit, current architectural decision that matches
reality:

- RustChat has a working, actively maintained architecture: a Rust/Axum
  backend with PostgreSQL as the authoritative database, a Redis
  realtime/coordination layer, S3-compatible storage, a Vue web frontend,
  native `/api/v1/*` and Mattermost-compatible `/api/v4/*` APIs, RustChat's own
  WebSocket implementation, a RustChat push proxy, and AI agents / RAG /
  RustShare integration.
- Buzz remains interesting as a collaboration system, but RustChat must not be
  replaced by it, and RustChat must not depend on it for core operation.
- Any connection between the two systems must be an optional, loosely coupled
  integration across documented external protocol boundaries.

## Decision

RustChat continues as an independently developed, self-hosted collaboration
platform. The existing architecture remains the product foundation.

RustChat owns, without delegation to any external collaboration system:

- product behavior,
- persistence,
- authentication,
- authorization,
- the frontend,
- the native API (`/api/v1/*`),
- the compatibility API (`/api/v4/*`),
- realtime semantics,
- AI/RAG capabilities,
- operational lifecycle.

Buzz (`block/buzz`) is an **optional external integration target**. The
architectural relationship is:

```text
RustChat core
    |
    +-- optional integration boundary
             |
             +-- block/buzz
```

Buzz integration occurs exclusively behind an external adapter/connector
boundary that lives in RustChat (for example, a connector module and
integration services) and speaks Buzz's documented external protocol/API
surface only. Integration details must be verified against the current
`block/buzz` main branch before implementation; do not rely on retired pivot
documents.

## Explicit prohibitions

RustChat must not:

- replace its backend with Buzz,
- replace its Vue frontend with Buzz clients,
- make Buzz a mandatory runtime dependency,
- share PostgreSQL schemas with Buzz,
- access Buzz internal database tables,
- vendor Buzz source,
- depend on private/internal Buzz Rust crates,
- translate the RustChat domain model into Buzz's internal model,
- block normal RustChat operations when Buzz is unavailable.

## Allowed integration

Behind the connector boundary, RustChat may:

- configure one or more Buzz endpoints,
- authenticate to Buzz through supported external protocols,
- submit and query Buzz events,
- subscribe to supported Buzz event streams,
- bridge explicitly configured channels/events,
- expose selected RustChat functionality to Buzz,
- integrate RustChat AI/RustShare capabilities where authorization allows.

Mattermost compatibility on `/api/v4/*` remains a supported RustChat strategy.
It is not affected by this ADR and may only be changed by a future ADR.

## Availability rule

```text
RustChat availability > Buzz integration availability
```

Concretely:

- With no Buzz connection configured, RustChat behavior must be
  indistinguishable from a build with the integration absent.
- Buzz failures (DNS, timeout, authentication, 429, 5xx, malformed responses)
  must be isolated to the integration subsystem. Core messaging, realtime,
  authentication, and administration continue unaffected.
- RustChat state changes must not synchronously depend on Buzz network
  success; deliveries to Buzz go through a durable, retrying, observable
  delivery path (integration outbox).
- Events created by the bridge must carry a stable origin marker so the
  integration can recognize its own output and prevent forwarding loops.

## Consequences

- Positive: the product keeps a single, coherent domain model and no
  external availability coupling; Buzz becomes additive value instead of
  architectural risk.
- Positive: integration code is reviewable behind one boundary and can be
  feature/configuration gated.
- Negative: any Buzz capability RustChat wants to mirror must be modeled
  explicitly (identity, channel, and event mappings) rather than inherited.
- Negative: the integration boundary requires its own delivery durability,
  observability, and security work (outbox, metrics, secret handling).

## Verification

Every Buzz-facing change must demonstrate, with tests:

1. RustChat core works with the integration disabled or unconfigured.
2. Buzz outage scenarios do not affect core RustChat behavior.
3. Integration deliveries are durable, idempotent, and observable.
4. Integration credentials are never exposed through APIs, logs, or metrics.
5. Loop prevention is exercised by tests.
