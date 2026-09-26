# Buzz Integration (optional, outbound)

Status: **phase one — outbound channel bridging** (RustChat → Buzz only).

Buzz ([github.com/block/buzz](https://github.com/block/buzz)) is a
self-hosted team communication platform built on the Nostr protocol. It is
integrated strictly as an **external system over its public HTTP API**:
RustChat vendors nothing, forks nothing, and shares no database with Buzz.
When the integration is disabled (the default), RustChat's behavior is
indistinguishable from a build without it.

## Upstream verification

Everything in this document was verified against the Buzz `main` branch at
commit **`b65cff31a4c5f4a0af63952b60a21fd73195321a`**
("fix(ci): run the admin disabled-mode DB test in the PostgreSQL lane
(#7900)", 2026-09-25), primarily in `ARCHITECTURE.md`,
`crates/buzz-relay/src/api/bridge.rs`, `crates/buzz-auth/src/nip98.rs` and
`crates/buzz-sdk/src/builders.rs`. No private or undocumented event kinds are
used. **If you change this integration, re-verify against current `main` and
update the SHA.**

## The external API we depend on

### HTTP bridge (primary seam)

| Endpoint | Purpose | Notes |
|---|---|---|
| `POST {relay_url}/events` | Submit a signed Nostr event | Body: NIP-01 event JSON. 200 → `{"event_id", "accepted", "message"}`; 400/401/403/404 terminal; 429 rate-limited; 5xx retry. 1 MiB body cap. |
| `POST {relay_url}/count` | Count events matching filters | Authed probe (read-only). Body: JSON array of Nostr filters. |
| `POST {relay_url}/query` | Fetch events matching filters | Not used in phase one. |
| `GET {relay_url}/health` | Liveness | Unauthenticated. |

The relay binds the request to a community ("tenant") from the **Host
header**; unknown hosts fail closed with 404. The configured `relay_url`
must therefore be a host Buzz actually serves.

### NIP-98 HTTP authentication

Every bridge request carries:

```
Authorization: Nostr <base64(JSON of a kind-27235 Nostr event)>
```

The auth event is signed with the bridge key and must contain:

* exactly one `u` tag: the full request URL (e.g. `https://buzz.example.com/events`),
* exactly one `method` tag: `POST`,
* at most one `payload` tag: SHA-256 hex of the request body (we always include it),
* `created_at` within ±60 seconds (we sign fresh per request; the relay
  also replays-protects auth event ids).

### Channel and message semantics

* Buzz channels are NIP-29 groups identified by a UUID carried in the `h` tag.
* Chat messages are **kind 9** events (`KIND_STREAM_MESSAGE`): tag
  `["h", "<channel-uuid>"]`, content = plain text (≤ 64 KiB).
* To create a channel: kind 9007 (`h`, `name`, optional `visibility`,
  `channel_type`, `about`, `ttl`).
* **Membership is enforced**: the bridge pubkey must be a member of the
  target Buzz channel. Membership is granted on the Buzz side by a channel
  admin (kind 9000 add-member event with `h` + `p` tags). This is a manual
  Buzz-side setup step — see [Operator runbook](#operator-runbook).
* Duplicate event ids are deduplicated by the relay (idempotent submit) —
  this is what makes retry-after-timeout safe.

## Architecture

```
 RustChat post creation (mapped channel)
        │  same DB transaction
        ▼
 integration_outbox row  ──►  dispatcher worker (config-gated)
                                   │  deterministic rebuild + sign
                                   ▼
                          HttpBuzzConnector (NIP-98)
                                   │  POST /events
                                   ▼
                              Buzz relay
```

* **Transactional outbox** (`integration_outbox` table): the post insert and
  the delivery intent commit atomically; no post is ever bridged without
  being persisted, and no bridging intent is lost on crash.
* **Deterministic event ids**: the Nostr `created_at` is fixed to the outbox
  row's enqueue time, so every retry produces the same event id and the
  relay deduplicates. Retries are safe even after ambiguous timeouts.
* **Bounded exponential backoff with jitter**; after the attempt budget
  (default 10) or a terminal error (4xx auth/validation) the row is
  **dead-lettered** and inspectable/retryable via the admin API.
* **Crash recovery**: in-flight rows whose lease (default 5 min) expires are
  reclaimed and retried.
* **Loop prevention**: every bridged message carries non-indexed origin tags
  `["rustchat:bridge", "<connection-id>"]` and
  `["rustchat:post", "<post-id>"]` (consistent with Buzz's own namespaced
  tag convention, e.g. `buzz:workflow`). Any future inbound bridge **must**
  ignore events carrying `rustchat:bridge`.

### Why a new connector abstraction (not the webhook domain)

The existing integration domain (`api/integrations.rs`,
`integration_repository`) models **incoming/outgoing webhooks and slash
commands**: team-scoped, fire-and-forget, token-in-URL, no delivery state.
The Buzz bridge needs capabilities that model does not have and should not
grow for phase one: per-connection encrypted credentials with rotation,
durable delivery with bounded retries and dead-lettering, admin (not team)
ownership, and channel-level routing maps. The seam is therefore a small
[`BuzzConnector`] trait + outbox, while reusing the existing SSRF-safe URL
validation and DNS-pinned HTTP client from the webhook service, the audit
log, and the admin authorization machinery.

### RustChat-authoritative ownership

Mappings live in RustChat (`buzz_channel_mappings`), keyed by RustChat
channel with FKs and unique constraints. The Buzz side only needs the bridge
pubkey to be a channel member. Deleting a RustChat mapping stops bridging;
nothing is deleted on the Buzz side.

## Configuration

```bash
# Master switch (default: false). No worker, no enqueue lookups, and the
# admin API returns an explicit error when disabled.
RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED=true

# Optional tuning (defaults shown)
RUSTCHAT_INTEGRATIONS_BUZZ_POLL_INTERVAL_SECS=5
RUSTCHAT_INTEGRATIONS_BUZZ_MAX_ATTEMPTS=10
RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_BASE_SECS=5
RUSTCHAT_INTEGRATIONS_BUZZ_BACKOFF_MAX_SECS=3600
RUSTCHAT_INTEGRATIONS_BUZZ_IN_FLIGHT_LEASE_SECS=300
RUSTCHAT_INTEGRATIONS_BUZZ_BATCH_SIZE=20

# Run the outbox dispatcher worker in this process (default: true).
# Multi-instance deployments set this to false on every process except one
# designated drainer — claims are FOR UPDATE SKIP LOCKED, so delivery is safe
# with a single active drainer.
RUSTCHAT_INTEGRATIONS_BUZZ_RUN_DISPATCHER=true
```

## Admin API

All endpoints require admin auth and return `422` when the integration is
disabled. The private key is write-only (accepted on create/rotate, never
returned).

| Method & path | Purpose |
|---|---|
| `GET  /api/v1/admin/integrations/buzz/connections` | List connections |
| `POST /api/v1/admin/integrations/buzz/connections` | Create `{name, relay_url, private_key, enabled?}` |
| `GET  /api/v1/admin/integrations/buzz/connections/{id}` | Get connection |
| `PATCH /api/v1/admin/integrations/buzz/connections/{id}` | Update `{name?, relay_url?, enabled?}` |
| `POST /api/v1/admin/integrations/buzz/connections/{id}/key` | Rotate signing key |
| `POST /api/v1/admin/integrations/buzz/connections/{id}/test` | Authed round-trip probe (`POST /count`) |
| `DELETE /api/v1/admin/integrations/buzz/connections/{id}` | Delete (dead-letters active deliveries) |
| `GET  /api/v1/admin/integrations/buzz/connections/{id}/mappings` | List mappings |
| `PUT  /api/v1/admin/integrations/buzz/connections/{id}/mappings` | Upsert `{rustchat_channel_id, buzz_channel_id, outbound_enabled?}` |
| `DELETE /api/v1/admin/integrations/buzz/connections/{id}/mappings/{mapping_id}` | Delete mapping |
| `GET  /api/v1/admin/integrations/buzz/connections/{id}/deliveries?status=&limit=&offset=` | Delivery history |
| `POST /api/v1/admin/integrations/buzz/deliveries/{outbox_id}/retry` | Requeue a dead-lettered delivery |

All admin operations are audit-logged (`buzz.*` actions).

## Security properties

* **Secrets**: the bridge signing key is encrypted at rest (AES-256-GCM via
  the server encryption key), never logged (custom redacted `Debug` on the
  runtime type), never returned by the API, and rotatable without downtime.
* **SSRF**: relay URLs must pass the same policy as outgoing webhooks —
  http(s) only, no private/loopback/link-local/metadata addresses, no
  non-standard ports; the HTTP client pins DNS resolution to vetted
  addresses (no rebinding window), disables redirects and proxies.
  **HTTPS is required in production.**
* **Blast radius**: timeouts (15s) and response-size caps (64 KiB) on every
  relay call; malformed responses are treated as ambiguous and retried
  idempotently.
* **Failure isolation**: the bridge is strictly optional. Enqueueing a
  delivery intent is wrapped in a SAVEPOINT inside the post transaction — if
  it ever fails (e.g. the connection is being deleted concurrently), only the
  bridge intent is rolled back and logged; the RustChat message always
  commits. A Buzz outage can never delay, fail, or roll back a RustChat post.
* **Loop prevention**: origin markers on every bridged event (see above).
* **Metrics**: bounded cardinality — counters/gauges are labeled only by
  fixed vocabularies (`provider`, `outcome`, `status`); no user, channel,
  message, or relay-URL labels.

## Operator runbook

1. Enable the integration (`RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED=true`) and restart.
2. Generate a bridge key: `nostr-tool generate` or any Nostr key tool
   (hex or `nsec…`).
3. On the **Buzz** side: create (or choose) the target channel and add the
   bridge **pubkey** as a member (channel admin issues a kind 9000
   add-member event, e.g. via the Buzz admin UI/CLI).
4. Create the connection via the admin API with the private key.
5. `POST .../connections/{id}/test` until it returns `{"ok": true}`.
6. Map channels: `PUT .../connections/{id}/mappings` with the RustChat
   channel id and the Buzz channel UUID (the `h` tag value).
7. Post a message in the RustChat channel; verify delivery via
   `GET .../connections/{id}/deliveries` and in the Buzz client.

Key rotation: `POST .../connections/{id}/key` with the new key, then add
the new pubkey on the Buzz side. Because the Nostr event id hashes the
signing pubkey, any undelivered rows for that connection are dead-lettered
atomically on rotation (a redelivery under the new key could no longer
deduplicate on the relay); review and requeue them via
`POST .../deliveries/{outbox_id}/retry` once the new identity is ready.

Rollback (feature off): set `RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED=false` and
restart; pending deliveries remain queued (and inspectable) in the outbox.

## Scope and non-goals (phase one)

* Outbound only: RustChat channel messages → Buzz channel messages.
  Thread replies are bridged with a synthetic `root` tag referencing the
  RustChat root post; Buzz-side threading is not reconstructed.
* No bidirectional sync, no inbound messages, no reactions/edits/deletes,
  no user mapping, no media relay (attachments are not bridged).
* Message content is bridged as `"<author display name>: <message>"`,
  signed by the bridge key (per-event authorship is not representable in
  kind 9 without per-user keys — deferred by design).
