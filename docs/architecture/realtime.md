# Realtime Architecture

RustChat's realtime layer is the WebSocket hub in `backend/src/realtime/`
together with the shared connection core in `backend/src/api/websocket_core.rs`.
This document describes the endpoints, wire formats, fan-out model, and client
connection-state behavior.

## Endpoints and wire formats

Two WebSocket endpoints share a common core (`api/websocket_core.rs`) but
present different wire formats:

| Endpoint | Clients | Wire format |
|---|---|---|
| `/api/v1/ws` | Internal clients and integration tests | Internal envelope (`type`, `event`, `data`, `channel_id`) |
| `/api/v4/websocket` | The native web app and Mattermost mobile/desktop clients | Mattermost framing (`event`, `data`, `broadcast`, `seq`) |

The shared core handles:

- Auth token normalization (header + `Sec-WebSocket-Protocol` fallback)
- Connection limit enforcement
- Default team/channel subscription bootstrap
- Presence lifecycle: `online` on connect, `offline` when the last connection for a user drops
- Shared commands: `subscribe_channel`, `unsubscribe_channel`, `typing`, `presence`, `ping`→`pong`

Channel subscription requires channel membership: `websocket_core.rs` verifies
`is_channel_member` before a subscribe is accepted, and the typing paths on
both v1 and v4 validate membership as well.

### v4-specific behavior

- Optional auth challenge exchange (`action=authentication_challenge`)
- Session resumption with `connection_id` and `sequence_number`
- Mattermost event name mapping: `posted`, `typing`, `post_edited`, `status_change`, etc.

## Event fan-out

```
Service writes event
  → realtime::hub
  → broadcast to subscribed local connections
  → Redis pub/sub → other backend instances → their hubs → their connections
```

1. A client sends a message (or a service emits an event) on Backend 1
2. Backend 1 persists to PostgreSQL
3. Backend 1 publishes the event to a Redis pub/sub channel
4. Every other backend instance receives the event from Redis
5. Each instance forwards the event to its locally connected, subscribed clients

This design allows horizontal scaling: add more backend instances behind a
load balancer, and Redis coordinates real-time delivery across all of them.
Sticky sessions are still recommended for WebSocket connections.

### Known durability limits

Replay for reconnecting clients is currently best-effort and in-memory (a
bounded per-connection buffer). A server restart or a long gap between
reconnects can lose realtime events; clients must reconcile state over REST
after reconnecting. A durable event outbox is tracked as a production-hardening
item on the roadmap.

## Client-side connection state management

The frontend implements progressive disconnection UX to handle network
interruptions gracefully:

| State | Duration | Visual | Behavior |
|-------|----------|--------|----------|
| `connected` | — | 🟢 Green dot | Normal operation |
| `reconnecting` | < 5s | 🟡 Yellow banner, 80% opacity | Auto-retry with backoff |
| `disconnected` | 5-30s | 🟠 Orange banner, 60% opacity | Manual retry option |
| `failed` | > 30s | 🔴 Full-screen modal | Explicit reconnect required |

State transitions:

```
CONNECTED → onClose → RECONNECTING (auto-retry, exponential backoff)
RECONNECTING → timeout 5s → DISCONNECTED (manual retry available)
DISCONNECTED → timeout 30s → FAILED (must reconnect or refresh)
```

On reconnect, the client resynchronizes missed messages and unread counts over
REST. See the user-facing behavior description in
[User Guide — Connection Status](../user/connection-status.md).

## Related paths

Changes to `backend/src/realtime/` are compatibility-sensitive (elevated risk
tier) because the v4 event contracts are part of the Mattermost compatibility
surface — see [Compatibility Scope](../compatibility-scope.md) and
`.governance/protected-paths.yml`.
