# Integrations Architecture

RustChat integrates with external systems through explicit, reviewed
boundaries. This document describes the integration surfaces that exist in the
architecture today and the rules that govern adding new ones.

## Integration surfaces

| Surface | Path | Purpose |
|---|---|---|
| Incoming webhooks | `backend/src/api/integrations.rs` | External systems post messages into channels via per-hook tokens |
| Outgoing webhooks | `backend/src/api/integrations.rs`, `backend/src/services/webhooks.rs` | RustChat POSTs payloads to operator-configured URLs on message events |
| Slash commands | `backend/src/api/integrations.rs` | `/command` handling with external command endpoints |
| Bots and API keys | entities model, `backend/src/api/v1/entities.rs` | Programmatic access for bots and integrations |
| Agent tools | `backend/src/services/tools/` | Server-side tools available to AI agents (e.g. Tavily web search) |
| RustShare sync | `backend/src/services/sync/rustshare/` | Knowledge-base document sync from RustShare for RAG |
| External connectors | `backend/src/integrations/` (see ADR-005) | Optional, isolated integrations with external collaboration systems |

## Security rules for integrations

These rules are enforced today and apply to any new integration:

- **Outgoing URL validation:** outgoing webhook and slash-command URLs are
  validated at creation time and resolved/validated again at request time,
  with redirects disabled, to prevent DNS-rebinding SSRF.
- **Credential isolation:** external credentials (webhook secrets, provider
  API keys) are encrypted at rest and never returned by read APIs or included
  in logs.
- **Membership enforcement:** incoming webhooks can only post into channels;
  file and message access always checks channel membership.
- **Optional by default:** integrations that depend on external credentials
  are registered only when their configuration is present. The application
  starts normally without them.

## External connector boundary (ADR-005)

Integrations with external collaboration systems (for example Buzz) follow the
rules in [ADR-005](../adr/ADR-005-rustchat-core-and-buzz-integration.md):

```text
RustChat core
    ↓
Integration service
    ↓
Connector interface (external protocol only)
    ↓
External system
```

- RustChat is always authoritative for RustChat-owned state.
- External system downtime or removal must not break RustChat core
  functionality.
- Deliveries to external systems go through a durable outbox with retries;
  RustChat transactions never block on external network success.
- Connector credentials are encrypted at rest, rotatable, and never exposed.
- Events created by a bridge carry a stable origin marker to prevent
  forwarding loops.

Integration-specific documentation lives in [docs/integrations/](../integrations/README.md).
