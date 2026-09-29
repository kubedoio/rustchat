# M2 Design Note: Membership-Revocation on Channel Removal (GAP-11)

**Status:** design, requesting maintainer sponsorship (elevated risk:
realtime hub + permission enforcement).
**Date:** 2026-09-29
**Owner:** implementation agent (zoorpha); sponsor: @senolcolak
**References:** GAP-11 (production-readiness gap analysis),
`docs/plans/2026-09-28-gap-closure-implementation-plan.md` §5 M2,
`.governance/risk-tiers.yml`.

## 1. Verified current state

The gap analysis premise ("broadcast fanout trusts in-memory subscriptions
and lacks revocation path") is **partially stale**. As of 2026-09-29:

**Already implemented and tested:**

- Manual member removal (both API surfaces) revokes the hub subscription
  immediately after the DB delete:
  - v4: `backend/src/api/v4/channels/members.rs`
    (`remove_channel_member_by_id`) — `repo.remove_member(...)` →
    `state.ws_hub.unsubscribe_channel(user_id, channel_id)` →
    `user_removed` broadcast.
  - v1: `backend/src/api/channels.rs:653` (`remove_member` handler) — same
    pattern, without the broadcast.
- Subscribe-time authorization exists: `backend/src/api/websocket_core.rs`
  (`"subscribe_channel"` command) checks `is_channel_member` against the
  database before adding the in-memory subscription — a removed user
  cannot resubscribe.
- The removed user still receives their own `user_removed` notification:
  user-targeted fanout (`WsBroadcast.user_id`) is connection-based, not
  subscription-based (`backend/src/realtime/hub.rs`, `broadcast_local`).
- Regression coverage exists:
  `backend/tests/api_v4_websocket_lifecycle.rs`
  (`websocket_removed_member_stops_receiving_channel_events`,
  `websocket_non_member_cannot_subscribe_channel`).

## 2. Remaining gaps (the actual M2 scope)

1. **v1 agent-channel removal** — `backend/src/api/v1/agents.rs:470`
   deletes `channel_members` with raw SQL: no hub revocation, no
   `user_removed` event.
2. **Group-driven removal** —
   `backend/src/services/group_membership_sync.rs`
   (`cleanup_tracking_membership` → `DELETE FROM channel_members`,
   `purge_team_syncables`, `purge_channel_syncables`): no revocation, no
   events. The reconcile entry points
   (`reconcile_group_syncable(state, ...)`, `reconcile_group_syncables`)
   hold `AppState`, so wiring is possible at that layer.
3. **Team deletion cascade** —
   `backend/src/repositories/team_repository.rs:321` bulk-deletes
   `channel_members` for all team channels inside the delete transaction:
   no revocation. Call sites: `backend/src/api/teams.rs:185`
   (`delete_team`), `backend/src/api/admin_teams.rs:136`.
4. **Multi-node deployments (the core GAP-11 risk)** —
   `hub.unsubscribe_channel` is **node-local**.
   `ClusterMessage` (`backend/src/realtime/cluster_broadcast.rs`) has only
   `Broadcast` and `Heartbeat` variants; `handle_cluster_message` applies
   no subscription changes. On a clustered deployment, the removed user's
   connections on *other* nodes keep their subscriptions and continue
   receiving channel events until they reconnect. Single-node
   deployments (the default Compose install) are covered.

## 3. Proposed implementation

### Slice A — single-node parity for the un-wired paths (standard risk)

After the membership delete commits, mirror the v4 pattern:
`hub.unsubscribe_channel(user_id, channel_id)` + `user_removed` broadcast.

- agents.rs and group-sync call sites have `AppState` — direct wiring.
- Team-deletion cascade: `TeamRepository::delete_team` returns the
  affected `(user_id, channel_id)` pairs (query before delete, inside the
  same transaction); the API-layer caller revokes after commit. This
  keeps the repository persistence-only (RI-C04 direction) instead of
  passing the hub into the repo layer.
- Group sync: the transactional helpers already track the affected user
  (`tracked.user_id` / membership rows); surface those pairs to the
  reconcile layer for post-commit revocation.

### Slice B — cluster-wide revocation (elevated risk, needs sponsorship)

Add a control message:

```rust
pub enum ClusterMessage {
    Broadcast { ... },
    Heartbeat { ... },
    RevokeChannelSubscription {
        user_id: Uuid,
        channel_id: Uuid,
        origin_node: String, // echo suppression, like Broadcast
    },
}
```

- `handle_cluster_message` applies it via
  `hub.unsubscribe_channel(user_id, channel_id)`.
- New hub method `revoke_channel_subscription(user_id, channel_id)`:
  local unsub + cluster publish (no-op when `cluster_broadcast` is unset,
  i.e. single-node). All four gap-1–3 call sites plus the existing v1/v4
  handlers use it.
- The existing `user_removed` data-plane broadcast is unchanged; the
  control message is separate by design (control ≠ data).

### Test plan

- Extend `backend/tests/api_v4_websocket_lifecycle.rs`:
  removed-via-group-sync and agent-removal variants of
  `websocket_removed_member_stops_receiving_channel_events`.
- Unit test for the cluster path: `handle_cluster_message` with
  `RevokeChannelSubscription` against a second `WsHub` instance asserts
  the subscription is removed locally (no multi-process harness needed).
- Out of scope / documented limitation: a full two-node integration test.

## 4. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Revocation races with an in-flight broadcast | Acceptable: worst case is one already-dispatched event, same window as today's v4 path; ordering is best-effort by design |
| Control message lost (Redis pub/sub is at-most-once) | Reconnect resync is authoritative (subscribe re-checks membership); document revocation as best-effort plus reconciliation-on-reconnect, mirroring the replay-window stance |
| Repo-layer signature change (`delete_team` return value) | Additive return type; no behavior change for existing callers that ignore it |
| Elevated-risk path (realtime + permissions) | This note; maintainer sponsorship before Slice B; Slice A is standard risk and could land first |

## 5. Alternatives considered

- **Authorization check at fanout time** (query membership per broadcast):
  rejected — a DB round-trip on the hot path for every event.
- **Periodic reconciliation only** (rebuild subscriptions from DB every N
  seconds): rejected as sole mechanism — GAP-11's risk window becomes N
  seconds instead of immediate; acceptable only as a backstop.
- **Disconnect the removed user's connections** (`close_all_with_code`):
  rejected — heavier than needed; subscription revocation plus the
  `user_removed` notification lets clients clean up gracefully.

## 6. Requested decision

1. Approve Slice B (cluster `RevokeChannelSubscription`) for
   implementation under sponsorship, or
2. Approve Slice A only and record the multi-node gap as a known
   limitation (single-node deployments — the shipped default — are
   already covered).
