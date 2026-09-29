# Tier 3 Design Note: Durable Realtime Replay / Outbox

**Status:** draft, requesting maintainer sponsorship + design review
(architectal tier: storage/data model + protocol compatibility).
**Date:** 2026-09-29 (rev 2 — revised after an independent design review that
found and fixed two blockers: the `assigned_seq`/`durable_seq` invariant was
self-contradictory with a silent-event-loss path (now §3.3 "Sequencing
invariant"), and Slice 2 overstated its deliverable (now re-scoped in §4);
plus eight should-fixes incorporated).
**References:** `docs/plans/2026-09-28-gap-closure-implementation-plan.md` §5.1,
`docs/plans/2026-09-29-m2-membership-revocation-design.md` (multi-node gap),
`.governance/risk-tiers.yml` (architectal tier: `adr_required: true`,
`design_review: true`, maintainer sponsorship).

---

## 0. Premise check (what is actually true in the tree today)

The plan's §5.1 premise is **mostly accurate but imprecise in two ways**, in the
same spirit as the M2 note's partial-staleness finding. Verified against the
current tree:

**Accurate:**

- Replay state is purely in-memory (`backend/src/realtime/connection_store.rs`),
  created once at startup in `backend/src/bootstrap/dependencies.rs:56`
  (`connection_store: ConnectionStore::new(...)`). A server restart loses all
  connection sessions, sequence counters, and replay buffers.
- The replay window is bounded at the **last 128 messages per connection**
  (`connection_store.rs:23`, `MESSAGE_BUFFER_SIZE`) and inactive connection
  state expires after **5 minutes** (`connection_store.rs:21`,
  `CONNECTION_TTL = 300s`), swept every 60s (`connection_store.rs:25`).
- `docs/architecture/realtime.md:55-61` already documents this honestly:
  "Replay for reconnecting clients is currently best-effort and in-memory …
  A durable event outbox is tracked as a production-hardening item."

**Imprecise:**

1. **"`resync_required` beyond the window" does not exist today.** The server
   emits `resync_required` in exactly three situations — hub receiver lag
   (`backend/src/api/v4/websocket/connection.rs:227-231`, reason
   `"hub_lagged"`), actor event-queue overflow
   (`backend/src/realtime/websocket_actor.rs:50-71`, reason `"queue_full"`),
   and the `WsEvent::ResyncRequired` forward
   (`connection.rs:324-335`). When the 128-message buffer overflows or the
   5-minute TTL expires, the failure is **silent** (see §1.2).
2. **The native frontend ignores `resync_required` entirely.**
   `rg resync_required frontend/src` returns nothing. `useWebSocket.ts`'s
   message switch (`frontend/src/composables/useWebSocket.ts:571-833`) has no
   case for it, and nothing subscribes via `onEvent`. The signal is currently
   write-only.
3. Restart is not *total* silence: because a reconnecting client presents a
   `connection_id`, `should_send_reconnect_snapshot`
   (`backend/src/api/v4/websocket/resumption.rs:38-46`) is true and the client
   receives an `initial_load` summary snapshot
   (`connection.rs:201-209`, built by `build_reconnect_snapshot`,
   `resumption.rs:122-319`). What is lost is *event-level* replay — individual
   `posted`/`post_edited`/`reaction_*` events that the snapshot's counts and
   the current-channel refetch do not cover.

---

## 1. Problem statement

### 1.1 How sequencing and replay work today (verified)

- **Connection IDs and sequence numbers** are per-connection and issued in
  memory. `ConnectionStore::resume_or_create`
  (`connection_store.rs:196-276`) either resumes an existing entry (verifying
  `user_id` matches — hijack rejection, `connection_store.rs:209`) or mints a
  fresh UUID with `initial_seq = 1` (`connection_store.rs:241-244`; hello is
  seq 0, see `connection.rs:141-153`).
- **Seq assignment happens at fan-out time, per connection.** The v4 hub
  forward task (`connection.rs:216-282`) receives each `WsEnvelope` from the
  hub's per-connection broadcast channel (`hub.rs:90`, capacity 100), maps it
  to a Mattermost message, then calls
  `replay_store.queue_message(&replay_connection_id, replay_payload)`
  (`connection.rs:271`) which atomically increments the connection's counter
  and appends `{event, data, broadcast}` to the ring buffer
  (`connection_store.rs:317-325`, `79-104`). The reconnect snapshot is also
  sequenced and buffered (`resumption.rs:89-94`).
- **Replay on reconnect:** `WebSocketActor::new` → `resume_or_create`
  (`websocket_actor.rs:180-182`) returns missed messages
  (`get_missed_messages(since_seq)` — a simple `seq > since_seq` filter,
  `connection_store.rs:107-114`), which are replayed after hello
  (`connection.rs:187-199`) via `replay_message_to_ws_message`
  (`websocket_actor.rs:306-323`).
- Only the v4 endpoint participates. The v1 endpoint (`/api/v1/ws`) has no
  `WebSocketActor`/`connection_store` usage (grep over `backend/src/api`).

### 1.2 What breaks, and for whom

1. **Server restart / crash (all users).** All connection state is lost. The
   client's reconnect with `connection_id=X&sequence_number=N` cannot be
   satisfied; a *new* connection ID is minted and logged
   ("Resume request could not be satisfied; creating fresh websocket stream",
   `connection_store.rs:246-254`). The client gets hello seq 0 + an
   `initial_load` snapshot — counts and statuses, but **no replay of the
   individual missed events**. Users in any channel other than the currently
   open one see stale message lists, missing edits/deletions/reactions until
   they manually navigate or refresh.
2. **Replay window overflow (>128 events while disconnected, one user).**
   Silent truncation: the ring buffer drops the oldest entries
   (`connection_store.rs:88-91`), and `get_missed_messages` happily returns
   the surviving tail. The client observes a seq discontinuity (e.g. last
   seen seq 10, next received seq 100) and — because it only tracks
   `max(seq)` (`useWebSocket.ts:557-560`) — accepts it. Events 11–99 are lost
   with **no signal at all**. The busier the server, the more likely a brief
   disconnect crosses 128 events.
3. **TTL expiry (>5 min disconnected, one user).** Same as restart: resume
   silently fails, fresh connection ID, snapshot only.
4. **`resync_required` is not a contract.** It is sent unsequenced via
   `send_raw` (so it is itself never replayed), only for internal overflow
   reasons, and no client in this repo consumes it.
5. **Multi-node (documented production topology,
   `docs/architecture/overview.md:301-335`).** Per-connection sequencing and
   buffering live on the node that owns the socket. Redis pub/sub fan-out
   (`realtime/cluster_broadcast.rs`) is at-most-once: if a `Broadcast`
   cluster message is dropped, the destination node never queues the event,
   so it is absent from that node's in-memory buffer *and* would be absent
   from any node-local durable buffer. The M2 design note records the
   cluster-wide revocation half of this gap
   (`docs/plans/2026-09-29-m2-membership-revocation-design.md:55-62`).

---

## 2. Goals / non-goals

### Goals

1. **G1 — Restart-surviving replay:** a client that reconnects with a known
   `connection_id` + `sequence_number` within the retention window gets its
   exact missed events replayed in order, with per-connection sequence
   continuity, regardless of server restarts.
2. **G2 — Explicit window contract:** when replay cannot be satisfied
   (retention expired, compaction removed the needed range, durability
   watermark behind the client's seq), the server sends a defined
   `resync_required` message *and the frontend acts on it*.
3. **G3 — Bounded retention:** durable replay state is compacted on a
   schedule; growth is bounded by (retention window × event rate ×
   connection count), with a per-connection event cap.
4. **G4 — Wire compatibility:** no change to the existing v4 message framing,
   hello semantics, or reconnect parameters. Everything new is additive.
   Mattermost clients must continue to work unchanged.
5. **G5 — Operational safety:** feature-flagged, observable (metrics), and
   reversible without a migration rollback.

### Non-goals (explicitly deferred)

- **Multi-node event-loss guarantees.** Durable replay *does* work
  cross-node for the reconnect path (the store is Postgres, shared), but we
  do **not** fix at-most-once Redis pub/sub delivery between nodes. If a
  cluster broadcast is dropped before a connection's node queues it, the
  event never enters that connection's replay stream. Fixing that requires
  persisting before broadcast (a much larger restructuring of the fan-out
  path). Documented as a known limitation, mirroring the M2 note's stance
  (`m2-membership-revocation-design.md:121`).
- **v1 endpoint replay.** v1 has no sequencing today; out of scope.
- **Transactional (same-transaction) event capture.** See §3.3 — the Buzz
  outbox's core guarantee is intentionally *not* reused here.
- **Offline push delivery, message-history pagination, `client_msg_id`
  idempotency** (separate Tier 3 item #2 in the plan).
- **Exactly-once client state.** Replay is at-least-once from the server's
  perspective; clients already tolerate duplicate/out-of-order events via
  store upserts (`messageStore.handleNewMessage` etc.).

---

## 3. Proposed design

### 3.1 Overview

Keep the existing in-memory `ConnectionStore` as the **hot path** (seq
assignment, live ring buffer, sub-millisecond replay for the common
short-disconnect case). Add a **durable shadow** in PostgreSQL:

- a `realtime_connection_sessions` row per connection (identity, ownership,
  durability watermark, retention deadline), and
- `realtime_connection_events` rows — the *same* `{event, data, broadcast}`
  payloads the ring buffer already stores — written **asynchronously via
  group-commit batches** from the fan-out path.

On reconnect, replay resolution is: in-memory buffer first (unchanged), then
the durable table (new), then explicit `resync_required` + fresh connection.
Per-connection sequencing is **preserved, not replaced**: sequence numbers
remain assigned at fan-out time by the node owning the connection, which is
what makes the wire contract unchanged.

### 3.2 Schema (new migration)

Next migration number after `20260928110000_group_syncable_active_grants.sql`,
following the existing naming convention
(`YYYYMMDDHHMMSS_snake_case_description.sql`), e.g.
`20261001000000_realtime_replay_outbox.sql`:

```sql
-- Durable replay state for the v4 reliable-websocket resumption protocol.
CREATE TABLE IF NOT EXISTS realtime_connection_sessions (
    -- Same UUIDs the server already mints (connection_store.rs:241).
    connection_id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Node that owns the socket (observability; sticky sessions expected).
    node_id VARCHAR(255),
    -- Highest seq durably persisted for this connection (watermark).
    durable_seq BIGINT NOT NULL DEFAULT 0,
    -- Highest seq ever assigned by the in-memory counter, including
    -- unflushed/dropped rows. INVARIANT: assigned_seq >= durable_seq;
    -- (assigned_seq - durable_seq) is exactly the unflushed/lost tail.
    assigned_seq BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Retention deadline; compaction deletes sessions past this.
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS realtime_connection_events (
    connection_id UUID NOT NULL
        REFERENCES realtime_connection_sessions(connection_id) ON DELETE CASCADE,
    -- Per-connection sequence number, assigned at fan-out time.
    seq BIGINT NOT NULL,
    -- Mattermost event name ('posted', 'post_edited', ...).
    event VARCHAR(128) NOT NULL,
    -- The replay payload: {"event","data","broadcast"} with the same
    -- fields/values the ring buffer stores today (connection.rs:265-270).
    -- (JSONB does not preserve byte/key order — "semantically identical",
    -- not byte-identical.)
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (connection_id, seq)
);

-- Compaction scan: expired sessions.
CREATE INDEX IF NOT EXISTS idx_realtime_sessions_expiry
    ON realtime_connection_sessions (expires_at);
```

Notes:

- The primary key `(connection_id, seq)` both orders replay scans and makes
  the write path idempotent (a re-flushed batch after a partial failure uses
  `ON CONFLICT DO NOTHING`). No additional index on `(connection_id, seq)` is
  needed — it would duplicate the PK btree.
- `user_id … ON DELETE CASCADE` ensures deleted users' replay state cannot
  linger (privacy).
- No pgvector / no Postgres-beyond-16 features are used; JSONB + BIGINT only.
  Postgres 16+ constraint (`docs/architecture/overview.md:25`) is unaffected.
- **Migrations are irreversible and require human review**
  (`AGENTS.md` general rule; `docs/architecture/overview.md:100`). Rollback
  strategy is flag-off + code revert, not migration revert (see §5).

### 3.3 Who writes, when, and in what transaction — and what we reuse from Buzz

**Honest assessment of `backend/src/integrations/outbox.rs` reuse:**

| Buzz outbox property | Reusable here? |
|---|---|
| Same-transaction enqueue (`enqueue_message_created_in_tx`, `buzz/dispatcher.rs:40-102`, called inside the post-creation tx at `services/posts/posting.rs:611`) | **No.** Realtime events reach the replay path *after* commit, after hub fan-out, after per-connection permission filtering (subscriptions, `exclude_user_id`), and after Mattermost mapping. The DB transaction is long gone. Making capture transactional would mean restructuring the entire fan-out path (and would persist events for connections that never receive them). We accept a bounded non-durable tail instead (§3.5). |
| Status machine `pending/in_flight/delivered/dead_letter` + bounded retry with jitter (`outbox.rs:17-44`, `69-103`) | **No.** The "remote" here is the client's socket; there is no retryable delivery — reconnect replay *is* the retry. |
| `FOR UPDATE SKIP LOCKED` claiming (`outbox.rs:140-170`) | **No.** Each connection's events are written by exactly one node/task; no competing claimants. |
| Bounded error truncation (`truncate_error`, `outbox.rs:317-328`), repository-module layout, migration + metrics wiring | **Yes** — same code style and operational surface. |
| Idempotency via unique key (`(provider, idempotency_key)`, migration `20260925000003_integration_outbox.sql:36`) | **Analog only** — our dedup key is the natural `(connection_id, seq)` PK. |
| Retention/cleanup | **Not present in Buzz** — `integration_outbox` has no deletion job (delivered rows are retained for audit). The realtime outbox **must** compact (privacy + growth); the pattern to follow is `backend/src/jobs/retention.rs` (bounded batches, config-driven cutoffs). |

**Write path (new `RealtimeReplayWriter`, wired into the v4 hub forward task):**

**Sequencing invariant (the load-bearing detail).** Both session columns are
"highest seq" values — `durable_seq` = highest seq durably persisted;
`assigned_seq` = highest seq ever assigned by the in-memory counter,
including rows not yet flushed or dropped by backpressure. The maintained
invariant is `assigned_seq >= durable_seq`. The in-memory counter is a
`fetch_add` counter whose first queued event *receives* the seed value
(`ConnectionState::new` → `next_sequence()`, `connection_store.rs:57-71`;
verified by `test_new_connection_first_queued_sequence_is_one`), so a seed of
`S` means the first new event is seq `S`. Therefore, on any (re)build of
in-memory state from durable state, the counter is seeded at
**`max(assigned_seq, client_requested_seq) + 1`** — the `+1` is mandatory:

- without it, a crash where the client saw *less* than `durable_seq`
  (batch flushed, client hadn't received it) would seed at `durable_seq`,
  assign that same seq to a new event, and the `ON CONFLICT DO NOTHING`
  write would **silently discard** it (a persisted row with that PK exists) —
  reintroducing exactly the silent loss G2 exists to eliminate;
- against `client_requested_seq` (the highest seq the client *received*), the
  `+1` prevents reusing a seq the client already processed.

This mirrors the fresh-connection convention (nothing assigned = 0 → first
event = 1).

1. Connection established → `INSERT INTO realtime_connection_sessions` (or
   `UPDATE … SET last_activity_at, expires_at` on resume). For a resumed
   session, the in-memory counter is seeded at
   `max(durable.assigned_seq, client_requested_seq) + 1` per the invariant
   above, so new events always start above every persisted row and above
   everything the client has seen.
2. `queue_message` assigns seq + appends to the ring buffer exactly as today,
   **and** hands `(connection_id, seq, payload)` to the batcher.
3. The batcher flushes every `flush_interval` (default 250 ms) or
   `flush_batch` rows (default 128), whichever first, in one multi-row
   `INSERT … ON CONFLICT (connection_id, seq) DO NOTHING`, then advances
   `durable_seq` to the **highest seq actually persisted in the batch** and
   `assigned_seq` to the **current in-memory counter maximum — even when rows
   were dropped by backpressure** — in the same transaction. Group commit:
   one batched multi-row INSERT transaction per flush, not one transaction
   per event.
4. Backpressure: if the DB is slow/down, the batcher's bounded queue fills;
   policy is *drop oldest unflushed*; `durable_seq` stays at what was
   flushed while `assigned_seq` keeps advancing (preserving
   `assigned_seq >= durable_seq`). A reconnect that needs seqs between the
   watermark and the client's last seq then fails cleanly into
   `resync_required` (`durable_watermark_behind`) instead of silently
   truncating — the exact failure mode that is silent today.
5. Disconnect keeps today's behavior (state retained for resumption) and
   additionally refreshes `expires_at = now() + retention`.

**Batcher topology (S5):** one shared per-process queue keyed by
`connection_id`, with per-connection grouping at flush time (a flush
transaction batches rows across connections but writes/advances each
connection's session row for exactly its own rows). The per-connection cap
(§3.6) is enforced at flush time, where the per-connection row count is
known. If a client reconnects without sending `sequence_number` (treated as
`requested_seq = 0`), the gap check applies unchanged: a session whose
earliest stored seq is above 1 routes to `resync_required` — more honest
than today's behavior, which silently replays a truncated buffer.

Config (env-driven, `config` crate): `realtime_replay.enabled` (default
**false** initially, flipped to true after burn-in), `.retention_hours`
(default 24), `.max_events_per_connection` (default 10,000 — the durable
equivalent of today's 128-entry ring buffer, but time-bounded),
`.flush_interval_ms`, `.flush_batch`.

### 3.4 Replay algorithm on reconnect

`resume_or_create` gains a durable fallback (behind the flag). Pseudo-flow in
`WebSocketActor::new` / `run_connection`:

```
resume_or_create(connection_id, user_id, requested_seq):
  1. In-memory hit (unchanged fast path):
     existing entry, user matches → replay from ring buffer.        [today's code]
  2. Durable lookup (new; only when step 1 missed):
     SELECT * FROM realtime_connection_sessions WHERE connection_id = $1
       AND expires_at > now()
     - No row / expired / user mismatch → fall through to step 3.
     - Row found, user matches:
       a. Load events: SELECT seq, payload FROM realtime_connection_events
          WHERE connection_id = $1 AND seq > $2 ORDER BY seq LIMIT $cap
          ($2 = requested_seq, $cap = 1000), and — in the same read
          transaction — SELECT MIN(seq) for the session (the gap check
          below needs the true oldest stored seq, which the LIMIT-ed scan
          cannot reveal).
       b. Gap check: if MIN(seq) > requested_seq + 1 (compaction removed
          needed rows), or the session's durable_seq < requested_seq
          (unflushed tail lost), → emit resync_required(reason) and
          continue to step 3 (fresh connection + snapshot).
       c. Otherwise: rebuild in-memory state from the session row (counter
          seeded at max(assigned_seq, requested_seq) + 1 — the §3.3
          sequencing invariant), replay rows (sent after hello, same as
          today's missed-message path, connection.rs:187-199), refresh the
          session row's last_activity_at/expires_at.
       d. If more rows exist beyond LIMIT: keep streaming in batches up to
          max_events_per_connection; beyond that → resync_required.
          (Defensive only: the §3.6 cap keeps stored rows ≤
          max_events_per_connection, so this branch is reachable mainly on
          cap reconfiguration or a trim race, not in steady state.)
  3. Fresh connection (today's behavior): new UUID, hello seq 0
     (connection.rs:149-153), then the existing reconnect snapshot.
```

Ordering guarantees: per-connection seq is a single monotonic counter
assigned under one atomic (`connection_store.rs:69-71`), so durable rows for
a connection are strictly ordered by construction; the `(connection_id, seq)`
PK scan returns them in order. No cross-connection ordering is claimed or
needed. If the LB routes a reconnect to a different live node while the old
node still holds the socket, two writers can briefly touch one connection's
rows; `ON CONFLICT DO NOTHING` plus last-writer-wins on the session row's
watermarks makes this safe.

**Deletion invariant the gap check depends on:** durable deletions are
strictly **prefix-only** (oldest seqs first) — the per-connection cap deletes
lowest seqs (§3.6 step 2) and backpressure drops oldest unflushed rows
(§3.3 step 4). Interior holes therefore cannot form; a gap is always
detectable as `MIN(seq) > requested_seq + 1`. This invariant must be stated
in the repository code and covered by tests, because the algorithm's
correctness rests on it.

**Long-replay consistency:** a replay that streams in batches (step 2d)
races the compaction/cap trimming. The initial scan and gap check run in a
single read transaction (one consistent snapshot); each subsequent batch
re-checks `MIN(seq) > requested_seq + 1` and aborts into `resync_required`
if trimming has removed rows the replay still needs.

**Replayed rows are not re-buffered** into the new (empty) in-memory ring
after a durable replay — an explicit choice: the next quick reconnect falls
back to durable again (correct, slightly slower). Tested behavior, not an
oversight.

**Entitlement on replay (open question Q3):** rows replayed were filtered by
the subscription state *at fan-out time*. If the user has since lost channel
membership (the M2 revocation work), step 2c would deliver pre-revocation
events they were legitimately entitled to then but may no longer be. Options:
replay as-sequenced (simple, matches "you would have seen these") vs.
re-filter by current membership using `payload->broadcast->channel_id`
(safe, costs a membership check per replayed channel event). Default in this
draft: replay as-sequenced; revisit with M2 Slice B.

### 3.5 The `resync_required` signal contract

Today's shape (unsequenced, via `send_raw`):

```json
{ "event": "resync_required", "data": { "reason": "hub_lagged" } }
```
(`connection.rs:227-231`, `325-335`; reasons today: `hub_lagged`,
`queue_full`.)

Proposed contract (additive, keeps the existing shape):

```json
{
  "event": "resync_required",
  "data": {
    "reason": "replay_window_exceeded"
          | "session_expired"
          | "durable_watermark_behind"
          | "replay_limit_exceeded"
          | "hub_lagged"        // existing
          | "queue_full",       // existing
    "connection_id": "<uuid>",   // new, additive
    "last_seq": 128              // new: highest seq the server can account for
  }
}
```

- Sent **before** hello when a resume request cannot be satisfied (so the
  client knows the upcoming hello seq 0 / new connection_id means "resync",
  not a bug), and inline (current behavior) for live-connection overflows.
  **Mechanism (new, S1):** today hello is sent unconditionally first
  (`connection.rs:178`) and resume resolution happens inside
  `WebSocketActor::new` (`websocket_actor.rs:180-182`), so pre-hello emission
  requires an explicit change: `resume_or_create` returns a typed result —
  `Resume(memory)` | `ResumeDurable(rows)` | `Resync(reason)` — and
  `run_connection` sends `resync_required` via `send_raw` *before* hello only
  in the `Resync` case. This ordering change on the reconnect path is the
  wire-observable delta most likely to surprise Mattermost clients; it routes
  through the v4 compat review (`docs/architecture/realtime.md:87-92`) with
  that called out.
- Remains unsequenced/`send_raw` — it is a control signal, not a replayable
  event (and the client must act on it even when its seq state is invalid,
  which is exactly when it fires).
- **Frontend behavior (the missing half):** `useWebSocket.ts` gains a
  `resync_required` case that (a) requests a fresh snapshot
  (`sendAction('reconnect', {})` — the action the server already answers with
  `initial_load`, `connection.rs:538-545`), (b) refetches the current
  channel's messages (extending today's onopen refetch,
  `useWebSocket.ts:489-495`), and (c) relies on the snapshot for unreads:
  `applyInitialLoadSnapshot` refreshes unreads by directly assigning
  `unreadStore.channelUnreads`/`channelMentions`
  (`useWebSocket.ts:406-410`); the live-event handlers
  (`handleUnreadUpdate`/`applyPostUnread`, `unreadStore.ts:134-141`) are not
  involved in the snapshot path. Unknown `data` fields are additive; **this
  repo's frontend** currently has no `resync_required` consumer at all
  (verified: repo-wide grep empty), so new fields cannot break it. External
  Mattermost clients are governed by a separate contract this repo cannot
  verify — treat "ignored harmlessly" as unproven for them, which is why the
  compat review above is mandatory rather than assumed.

### 3.6 Retention / compaction job

New job module `backend/src/jobs/realtime_replay_retention.rs` following the
`jobs/retention.rs` pattern (bounded batches, `CancellationToken`, config
cutoffs):

1. Every run (default interval 10 min): delete
   `realtime_connection_sessions` (cascade to events) where
   `expires_at < now()`, in batches (e.g. 500 sessions).
2. Per-connection cap: when `queue_message`-side writes would push a session
   past `max_events_per_connection`, delete that session's lowest seqs so the
   *newest* window is kept (mirrors the ring buffer's `pop_front` semantics,
   `connection_store.rs:88-91`) — done inside the batcher, cheap because it
   is known at flush time.
3. Metrics: sessions pruned, events pruned, flush lag (assigned_seq −
   durable_seq), replay hits by source (memory / durable / resync), flush
   errors, and a counter for `resync_required` **emissions by reason**
   (pre-hello + inline — the single most diagnostic signal for this
   feature's health) — extending the existing `rustchat_ws_*` /
   outbox-style metrics in `telemetry/metrics.rs`.

### 3.7 Failure modes

| Failure | Behavior |
|---|---|
| Server crash / restart | Unflushed tail (≤ flush_interval of events) lost. Client resumes from durable rows; if its `requested_seq` > `durable_seq`, it gets `resync_required(durable_watermark_behind)` + snapshot. Everything flushed replays exactly. |
| Postgres down / slow | Batcher queue fills → drop-oldest-unflushed + watermark stalls. Live delivery is **unaffected** (in-memory path unchanged — durability is a shadow, not a gate). Reconnects in this window get resync or memory replay. |
| Replay storm after restart (many clients reconnect at once) | Replay reads are `LIMIT`-ed PK-range scans; worst case each client pulls ≤ `max_events_per_connection` rows once. Acceptable for the documented single-node/small-team scale; flagged for multi-node sizing in Q5. |
| Table growth runaway | Retention job + per-connection cap + `expires_at` refresh only on real activity. Bounded by retention × active connections. |
| Session hijack attempt | Same guard as today: durable lookup verifies `user_id` (`connection_store.rs:209`, test `test_resume_rejects_connection_hijack`); mismatch → fresh connection. |
| Duplicate replay (client retries reconnect) | Idempotent: same rows, same seqs; client's `max(seq)` tracking dedups. |
| Feature flag off | Code paths inert; tables empty (job still runs, no-ops); behavior identical to today. |

---

## 4. Slicing plan

Each slice is independently shippable, reversible (flag or additive), and
validated per the repo ladder
(`cargo fmt --all -- --check && cargo clippy --all-targets --all-features -D warnings && cargo test --lib`,
then integration tests where the behavior requires Postgres). Size guidance:
architectural tier has no hard file/line cap (`.governance/risk-tiers.yml:36-38`;
migration PR sizing additionally routes through
`.governance/pr-size-limits.yml`), but slices are kept near the
elevated-tier shape (≤10 files / ~300 lines) where practical.

**Slice 0 — ADR + maintainer decision.**
This note promoted to `docs/…adr…` (or plans/) after review; records Q1–Q6
decisions.
*Validation:* design review sign-off.

**Slice 1 — Schema + repository (no behavior change).**
Migration `20261001…_realtime_replay_outbox.sql` + a
`realtime/replay_store.rs` repository (session upsert, event insert batch,
replay scan + MIN(seq), watermark update, prune queries) + unit/integration
tests against a live DB (pattern: `backend/tests/`, e.g. the lifecycle suite
referenced in the M2 note).
*Validation:* migration applies on the integration stack; repository tests
green (the repo uses runtime `sqlx::query` strings — no compile-time macros,
no offline `.sqlx` data to regenerate; the query strings are exercised by
the repository tests themselves).
*Reversible:* table unused; drop in a later migration if abandoned.

**Slice 2 — Durable session registry + restart routing (flagged, default off).**
Write session rows (with `assigned_seq` floor) on connect/resume/disconnect;
step-2 durable lookup in `resume_or_create` with the hijack check; gap
detection routes to today's fresh-connection behavior (no new signal yet).
**Scope honesty (B2):** with no events persisted yet, a restart still cannot
replay events — Slice 2 delivers *session identity continuity* (the
connection_id survives restarts; no silent re-minting), the durable hijack
guard, and the seq floor that prevents seq reuse across a restart. Event
replay (G1) lands in Slice 3.
*Validation:* integration tests: reconnect after in-memory state destruction
resolves to the *same* session row (identity preserved); hijack-mismatch
(user B presenting user A's connection_id) is rejected; unrecoverable resume
routes explicitly to fresh connection + snapshot; new events after a restart
resume start above the pre-restart seq floor (no reuse). Existing
`api_v4_websocket_lifecycle.rs` suite stays green.
*Reversible:* flag off.

**Slice 3 — Write-behind event persistence + watermark → delivers G1.**
Batcher wired into the v4 hub forward task (`connection.rs:271` area) +
`queue_message` shadow write; backpressure policy; flush-lag metric.
*Validation:* unit tests for batcher (ordering, ON CONFLICT idempotency,
drop-oldest, `assigned_seq >= durable_seq` invariant under backpressure);
integration test: events assigned **while connected**, then in-memory state
destruction (simulated restart), then reconnect with the same
`connection_id`/`sequence_number` → identical events replay from Postgres in
order with continuous seqs (the hub forward task is aborted on disconnect,
`connection.rs:379-380`, so no events are ever assigned while detached —
the test exercises the connected-then-restart path); load sanity: existing
ws benchmarks/metrics do not regress.
*Reversible:* flag off.

**Slice 4 — `resync_required` contract completion (server).**
New reasons (`session_expired`, `replay_window_exceeded`,
`durable_watermark_behind`, `replay_limit_exceeded`), `connection_id` +
`last_seq` fields, pre-hello emission on failed resume. Also emit
`replay_window_exceeded` from the **existing in-memory** window-overflow
path (fixes the silent truncation in §1.2 even with the flag off —
small, separately reviewable).
*Validation:* unit tests for each reason; assert message shape against the
documented contract; compat review (v4 surface).
*Reversible:* additive wire fields; worst case old clients ignore them.

**Slice 5 — Frontend resync handling.**
`resync_required` case in `useWebSocket.ts` (§3.5), unit tests (M6 of the
plan already calls for `useWebSocket.ts` coverage — coordinate).
*Validation:* `npm run lint && npm run test:unit && npm run build`; manual
E2E: kill server, restart, verify client recovers without manual refresh.
*Reversible:* pure frontend.

**Slice 6 — Retention/compaction job + config + docs.**
Job module, config knobs, metrics, `docs/architecture/realtime.md` rewrite
of the "Known durability limits" section (lines 55-61), ROADMAP update
(RI-C06 hygiene per plan §7).
*Validation:* job tests (expiry, cap, cascade); docs CI.
*Reversible:* job disabled via config.

**Slice 7 (deferred, needs its own note) — multi-node hardening.**
Cluster broadcast durability (persist-before-broadcast or Redis Streams) and
M2 Slice B cluster-wide revocation. Explicitly out of scope here.

---

## 5. Risks & mitigations

| Risk | Mitigation |
|---|---|
| **Write amplification:** one durable row per (connection, event); a `posted` to a 500-member channel with ~800 connections is ~800 rows. | Group-commit batching (amortized multi-row INSERT); the in-memory buffer already copies per connection today (`connection.rs:271` per-connection `queue_message`), so the *multiplicity* is not new — only the persistence cost is. Flag default-off; measure flush lag + DB write IOPS during burn-in; `max_events_per_connection` caps the worst case. If still too hot: persist only for sessions that actually disconnect-and-resume (start buffering on first miss — complexity trade-off, Q4). |
| **Table growth / privacy:** payloads contain full message content. | Short default retention (24 h) vs. Buzz's indefinite audit retention; per-connection cap; `ON DELETE CASCADE` from users; compaction job with metrics; document in data-model docs. |
| **Migration is irreversible** (repo rule; `docs/architecture/overview.md:100`). | Schema is additive (two new tables, no changes to existing tables); rollback = flag off + code revert; a later drop-migration is possible with human review if abandoned. Migrations run automatically at startup, so the tables are created in **every** environment (including flag-off ones) from the moment the migration lands — accepted explicitly. Backup→restore CI (M3, plan §4) covers migration safety in CI. |
| **Old-client compatibility** (Mattermost mobile/desktop speak this protocol). | Wire contract unchanged: same hello, same reconnect params, replayed frames are ordinary sequenced messages, `resync_required` already exists on the wire and is ignored harmlessly by clients that don't know it. New `data` fields are additive. v4 + `realtime/` are compat-reviewer co-approval paths (`docs/architecture/realtime.md:87-92`) — slices 2-4 route through that review. |
| **Correctness regression in the hot reconnect path** (elevated risk: `realtime/`, `api/v4/`). | Memory fast path untouched when flag off; durable path is additive fallback only after the memory miss; the existing lifecycle regression tests plus new restart/hijack/gap tests gate each slice; architectural-tier review (design review + ADR) before implementation. |
| **Flush tail loss window** (crash between send and flush). | Watermark check converts silent loss into explicit `resync_required`; window bounded by flush interval (250 ms default) — strictly better than today, where *everything* since disconnect is lost on restart. |
| **Replay of pre-revocation events** (interaction with M2). | Documented decision (Q3); replay-as-sequenced default matches "events you were already sent"; revisit alongside M2 Slice B. |

---

## 6. Open questions for the maintainer

1. **Q1 — Retention default.** 24 h of durable replay per connection, or
   shorter (e.g. 6 h)? Drives table size and the value of G1 for
   overnight-disconnected clients. (Mattermost's own server keeps a similar
   per-connection buffer in memory with a much shorter effective window.)
2. **Q2 — Flag default at ship time.** Ship Slice 2-3 with
   `realtime_replay.enabled=false` and flip after burn-in, or on by default
   given single-node scale? Burn-in period length?
3. **Q3 — Replay entitlement.** Replay as-sequenced (draft default) vs.
   re-filter replayed channel events against current membership? The latter
   is safer post-revocation (M2) but adds a membership check per replayed
   event.
4. **Q4 — Write path optimization.** Persist for all connections from the
   start (simple, draft default), or only begin persisting a connection's
   events after its first missed reconnect (cuts write volume sharply,
   complicates the "first disconnect is durable" guarantee)?
5. **Q5 — Multi-node ambition.** Confirm deferral of cluster-broadcast
   durability (Slice 7). If multi-node is nearer-term than assumed, the
   persist-before-broadcast design (global event table, per-connection
   cursors) should be evaluated *before* Slice 1 lands, because it implies a
   different schema (per-user/global event log rather than per-connection
   rows).
6. **Q6 — Should `initial_load` snapshots remain unconditional on every
   reconnect?** With durable replay, the snapshot is partially redundant
   (and it is the heaviest reconnect cost — a multi-join query,
   `resumption.rs:122-319`). Option: skip the snapshot when replay fully
   covered the gap. Risk: masks replay bugs; needs a fallback.

---

## 7. Alternatives considered

**(a) Do nothing / document the gap.**
`docs/architecture/realtime.md:55-61` already documents the limitation, and
clients do reconcile over REST (snapshot + current-channel refetch).
- Pros: zero cost; the doc is honest.
- Cons: the failure is *silent* in two of three modes (window overflow,
  restart), the `resync_required` signal is dead code client-side, and every
  restart degrades every connected client to summary-level consistency —
  stale edits/reactions/deletions in non-open channels until navigation.
  This is the plan's own "single highest-impact technical gap" (§5.1);
  accepting it permanently contradicts the roadmap item.

**(b) Periodic / on-gap client-side full resync.**
Keep the server as-is; make the frontend detect any seq discontinuity or
connection_id change and refetch *everything* (all channels' messages,
unreads, presence) on every gap.
- Pros: no backend/schema change; no new tables; simple mental model.
- Cons: does not survive restart *replay* any better — it replaces precise
  event replay with bulk REST traffic (N channels × history per user per
  reconnect; a reconnect storm after restart multiplies this into a read
  amplification spike against Postgres). Still no server-side signal for the
  silent window-overflow case (the client can detect a seq jump, but not
  know what it missed in summary form). Degrades mobile clients (the
  Mattermost-protocol ones) not governed by our frontend. Rejected as the
  primary design; elements of it are retained as the `resync_required`
  *fallback* behavior (§3.5), which is exactly what the fallback should be:
  rare, explicit, and bounded.

**(c) Durable per-connection outbox (this design).**
- Pros: exact restart-surviving replay with an unchanged wire contract;
  explicit failure signal; bounded, compactable storage; memory hot path
  untouched; flag-gated rollout.
- Cons: new tables + background job; one batched write per (connection,
  event); a small crash window (unflushed tail) that is converted into an
  explicit resync rather than eliminated; multi-node delivery gaps remain
  (documented, Q5).

**(d) Persist-before-broadcast global event log** (considered, rejected for
now): write every `WsEnvelope` once globally inside the service transaction
(Buzz-style), with per-connection durable cursors and fan-out-time
entitlement replay.
- Pros: fixes multi-node loss too; single row per event.
- Cons: restructures the fan-out path (`hub.broadcast_local`'s
  subscription/exclusion semantics would have to be re-evaluated at replay
  time — re-deriving "would this connection have received this?" from
  historical subscription state is subtle and risks compat drift); coupling
  of DB write latency to the hot broadcast path; substantially larger
  change than the gap demands for the documented single-node default
  topology. Revisit under Q5/Slice 7.

---

## Appendix A — Key file references (verified 2026-09-29)

| Behavior | Location |
|---|---|
| In-memory store creation (restart loses everything) | `backend/src/bootstrap/dependencies.rs:56` |
| TTL / buffer size / cleanup constants | `backend/src/realtime/connection_store.rs:21-25` |
| Seq assignment + ring buffer | `connection_store.rs:69-104`, `queue_message` `:317-325` |
| Silent window overflow (`pop_front`) | `connection_store.rs:88-91` |
| Resume-or-create, fresh-ID fallback, hijack check | `connection_store.rs:196-276` (`:209` user check, `:241-254` fresh ID + log) |
| Missed-message filter | `connection_store.rs:107-114` |
| Replay path (hello → replay → snapshot) | `backend/src/api/v4/websocket/connection.rs:141-209` |
| Per-connection seq at fan-out | `connection.rs:265-273` |
| `resync_required` emissions (hub lag / queue full / event) | `connection.rs:227-231`, `websocket_actor.rs:50-71`, `connection.rs:324-335` |
| Reconnect snapshot trigger + builder | `backend/src/api/v4/websocket/resumption.rs:38-46`, `:59-319` |
| Hub per-connection channel (capacity 100) | `backend/src/realtime/hub.rs:90` |
| Cluster fan-out (Redis, at-most-once) | `backend/src/realtime/cluster_broadcast.rs` |
| Buzz transactional outbox (pattern source) | `backend/src/integrations/outbox.rs`, `buzz/dispatcher.rs:40-102`, migration `20260925000003_integration_outbox.sql` |
| Retention-job pattern | `backend/src/jobs/retention.rs` |
| Client resume params + seq tracking + reconnect action | `frontend/src/composables/useWebSocket.ts:60-61`, `:453-460`, `:557-560`, `:499` |
| Client has no `resync_required` handler | `useWebSocket.ts:571-833` (absent; repo-wide grep empty) |
| Hello sent unconditionally first (pre-hello resync needs new mechanism) | `backend/src/api/v4/websocket/connection.rs:178` |
| Hub forward task aborted on disconnect (no seqs assigned while detached) | `connection.rs:379-380` |
| In-memory counter is fetch_add; seed value is the first assigned seq | `backend/src/realtime/connection_store.rs:57-71`, test `:483-493` |
| Snapshot refreshes unreads by direct store assignment | `frontend/src/composables/useWebSocket.ts:406-410` |
| Unreads resync surface | `frontend/src/features/unreads/stores/unreadStore.ts:134-141` |
| Durability-limit documentation | `docs/architecture/realtime.md:55-61` |
| Multi-node gap (M2 note) | `docs/plans/2026-09-29-m2-membership-revocation-design.md:55-62`, `:121` |
| Governance: architectural tier (ADR, design review, sponsorship) | `.governance/risk-tiers.yml:35-58` |
