//! Group-syncable membership convergence.
//!
//! Implements the grant semantics shared by every group-sync producer
//! (the Keycloak worker and the v4 admin reconcile path): the effective
//! role of a (target, user) membership is the **union of all active
//! syncable grants** — admin iff any active grant confers admin — and a
//! membership is revoked when no active grant keeps it alive. "Active"
//! means the owning group is not soft-deleted and the owning syncable is
//! not unlinked (`group_syncable_active_grants`).
//!
//! Only memberships created by sync (or already sync-granted) are tracked;
//! manually created memberships are never adopted, converged, or revoked
//! by group sync.

use std::collections::HashSet;

use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct DesiredMembership {
    target_type: String,
    target_id: Uuid,
    user_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TrackedMembershipRow {
    target_type: String,
    target_id: Uuid,
    user_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct GroupSyncableRow {
    pub(crate) syncable_type: String,
    pub(crate) syncable_id: Uuid,
    pub(crate) auto_add: bool,
    pub(crate) scheme_admin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncableKind {
    Team,
    Channel,
}

impl SyncableKind {
    pub(crate) fn as_db_str(self) -> &'static str {
        match self {
            Self::Team => "team",
            Self::Channel => "channel",
        }
    }

    /// Mattermost wire naming for the syncable type (v4 group events).
    pub(crate) fn as_mm_type(self) -> &'static str {
        match self {
            Self::Team => "Team",
            Self::Channel => "Channel",
        }
    }
}

/// Identifies the syncable a grant belongs to (the tracking-table key
/// prefix shared by grant tracking and cleanup).
#[derive(Debug, Clone, Copy)]
struct SyncableRef {
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
}

impl SyncableRef {
    fn new(group_id: Uuid, kind: SyncableKind, syncable_id: Uuid) -> Self {
        Self {
            group_id,
            kind,
            syncable_id,
        }
    }
}

/// Record (or refresh) the grant this syncable confers on the member.
/// Runs on the caller's connection/transaction so grants compose into
/// the reconcile transaction that holds the syncable's link-row lock.
async fn upsert_tracking_membership(
    conn: &mut sqlx::PgConnection,
    syncable: SyncableRef,
    target_type: &str,
    target_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO group_syncable_memberships
            (group_id, syncable_type, syncable_id, target_type, target_id, user_id, role)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (group_id, syncable_type, syncable_id, target_type, target_id, user_id)
        DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(syncable.group_id)
    .bind(syncable.kind.as_db_str())
    .bind(syncable.syncable_id)
    .bind(target_type)
    .bind(target_id)
    .bind(user_id)
    .bind(role)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Converge the stored membership role to the union of all active syncable
/// grants for this (target, user): `admin` iff any grant confers admin.
/// This makes overlapping syncables order-independent — the last writer no
/// longer decides whether an admin stays admin. Only ACTIVE grants count
/// (owning group and syncable alive, via group_syncable_active_grants), so
/// a soft-deleted group or unlinked syncable stops conferring its role.
/// With no grants left the CASE falls through to `member`, so a concurrent
/// cleanup cannot write NULL or fail on the NOT NULL constraint.
async fn converge_role_to_grants(
    conn: &mut sqlx::PgConnection,
    target_type: &str,
    target_id: Uuid,
    user_id: Uuid,
) -> ApiResult<()> {
    if target_type == "team" {
        sqlx::query(
            r#"
            UPDATE team_members SET role = (
                SELECT CASE WHEN bool_or(g.role = 'admin') THEN 'admin' ELSE 'member' END
                FROM group_syncable_active_grants g
                WHERE g.target_type = 'team'
                  AND g.target_id = team_members.team_id
                  AND g.user_id = team_members.user_id
            )
            WHERE team_id = $1 AND user_id = $2
            "#,
        )
        .bind(target_id)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE channel_members SET role = (
                SELECT CASE WHEN bool_or(g.role = 'admin') THEN 'admin' ELSE 'member' END
                FROM group_syncable_active_grants g
                WHERE g.target_type = 'channel'
                  AND g.target_id = channel_members.channel_id
                  AND g.user_id = channel_members.user_id
            )
            WHERE channel_id = $1 AND user_id = $2
            "#,
        )
        .bind(target_id)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Ensure the membership for a single syncable grant exists with the role
/// that grant confers, and track the grant for cleanup.
///
/// Semantics (Mattermost group-sync):
/// - If the membership does not exist, it is created with the granted role
///   and the grant is tracked.
/// - If it exists and is sync-granted (tracked by any syncable), the grant
///   is tracked and the stored role is converged to the union of all
///   active grants, so overlapping syncables are order-independent and a
///   `scheme_admin` flip still downgrades on later passes.
/// - If it exists but was never granted by sync (a manually created
///   membership), it is left untouched and untracked: group removal must
///   not revoke memberships the sync never granted.
async fn ensure_membership(
    conn: &mut sqlx::PgConnection,
    syncable: SyncableRef,
    target_type: &str,
    target_id: Uuid,
    user_id: Uuid,
    scheme_admin: bool,
) -> ApiResult<()> {
    let role = if scheme_admin { "admin" } else { "member" };

    // Common case: the membership does not exist yet — create it with the
    // granted role and track the grant. DO NOTHING (not DO UPDATE) so a
    // pre-existing membership is detected rather than adopted.
    let created = if target_type == "team" {
        sqlx::query(
            "INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (team_id, user_id) DO NOTHING",
        )
        .bind(target_id)
        .bind(user_id)
        .bind(role)
        .execute(&mut *conn)
        .await?
        .rows_affected()
            > 0
    } else {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (channel_id, user_id) DO NOTHING",
        )
        .bind(target_id)
        .bind(user_id)
        .bind(role)
        .execute(&mut *conn)
        .await?
        .rows_affected()
            > 0
    };

    if created {
        upsert_tracking_membership(conn, syncable, target_type, target_id, user_id, role).await?;
        converge_role_to_grants(conn, target_type, target_id, user_id).await?;
        return Ok(());
    }

    // The membership pre-exists. Only converge and track it when it is
    // sync-granted (an ACTIVE grant exists); manual memberships are left
    // alone so they can never be revoked by group removal.
    let sync_granted: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM group_syncable_active_grants
            WHERE target_type = $1
              AND target_id = $2
              AND user_id = $3
        )
        "#,
    )
    .bind(target_type)
    .bind(target_id)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?;

    if !sync_granted {
        return Ok(());
    }

    upsert_tracking_membership(conn, syncable, target_type, target_id, user_id, role).await?;
    converge_role_to_grants(conn, target_type, target_id, user_id).await?;
    Ok(())
}

/// Remove one syncable's tracking row and converge the membership it
/// tracked: revoke it when no other active grant keeps it alive, otherwise
/// re-converge the role to the remaining grants.
///
/// Lock order: the membership row is revoked or converged BEFORE the
/// tracking row is deleted — the canonical order (membership tables, then
/// tracking table) shared with `ensure_membership` and `soft_delete_group`'s
/// transaction. Deleting the tracking row first would invert that order and
/// can deadlock against a concurrent group soft-delete (each transaction
/// holding the row lock the other is waiting on). The remaining-grant
/// computation therefore excludes this syncable's grant explicitly instead
/// of relying on the tracking row already being gone.
async fn cleanup_tracking_membership(
    conn: &mut sqlx::PgConnection,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
    tracked: &TrackedMembershipRow,
) -> ApiResult<()> {
    // Remaining active grants excluding the one being removed, and the
    // role their union confers (as if this syncable's tracking row were
    // already deleted).
    let (remaining_role, remaining_grants): (String, i64) = sqlx::query_as(
        r#"
        SELECT
            CASE WHEN bool_or(g.role = 'admin') THEN 'admin' ELSE 'member' END,
            COUNT(*)
        FROM group_syncable_active_grants g
        WHERE g.target_type = $1
          AND g.target_id = $2
          AND g.user_id = $3
          AND (g.group_id, g.syncable_type, g.syncable_id) IS DISTINCT FROM ($4, $5, $6)
        "#,
    )
    .bind(&tracked.target_type)
    .bind(tracked.target_id)
    .bind(tracked.user_id)
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .fetch_one(&mut *conn)
    .await?;

    if remaining_grants > 0 {
        // Other syncables still grant this membership: converge the role to
        // the union of the remaining grants instead of leaving it at
        // whatever the removed syncable last wrote (an admin grant removed
        // second must not strand the member at 'member' — or vice versa).
        if tracked.target_type == "team" {
            sqlx::query("UPDATE team_members SET role = $1 WHERE team_id = $2 AND user_id = $3")
                .bind(&remaining_role)
                .bind(tracked.target_id)
                .bind(tracked.user_id)
                .execute(&mut *conn)
                .await?;
        } else {
            sqlx::query(
                "UPDATE channel_members SET role = $1 WHERE channel_id = $2 AND user_id = $3",
            )
            .bind(&remaining_role)
            .bind(tracked.target_id)
            .bind(tracked.user_id)
            .execute(&mut *conn)
            .await?;
        }
    } else if tracked.target_type == "team" {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(tracked.target_id)
            .bind(tracked.user_id)
            .execute(&mut *conn)
            .await?;
    } else {
        sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(tracked.target_id)
            .bind(tracked.user_id)
            .execute(&mut *conn)
            .await?;
    }

    // Tracking row is deleted last (canonical lock order — see the doc
    // comment above).
    sqlx::query(
        r#"
        DELETE FROM group_syncable_memberships
        WHERE group_id = $1
          AND syncable_type = $2
          AND syncable_id = $3
          AND target_type = $4
          AND target_id = $5
          AND user_id = $6
        "#,
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .bind(&tracked.target_type)
    .bind(tracked.target_id)
    .bind(tracked.user_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Reconcile every live syncable of a group against its current members.
pub(crate) async fn reconcile_group_syncables(state: &AppState, group_id: Uuid) -> ApiResult<()> {
    let rows: Vec<(String, Uuid)> = sqlx::query_as(
        r#"
        SELECT gs.syncable_type, gs.syncable_id
        FROM group_syncables gs
        JOIN groups g ON g.id = gs.group_id AND g.deleted_at IS NULL
        WHERE gs.group_id = $1
          AND gs.delete_at IS NULL
        "#,
    )
    .bind(group_id)
    .fetch_all(&state.db)
    .await?;

    for (syncable_type, syncable_id) in rows {
        let kind = if syncable_type == "team" {
            SyncableKind::Team
        } else {
            SyncableKind::Channel
        };
        // One failing syncable must not abort the group's remaining
        // reconciliation (or, via the Keycloak worker, the whole cycle):
        // the others still need to converge. Failures are logged and
        // retried on the next pass.
        if let Err(err) = reconcile_group_syncable(state, group_id, kind, syncable_id).await {
            tracing::warn!(
                group_id = %group_id,
                syncable_id = %syncable_id,
                syncable_type = %syncable_type,
                error = %err,
                "Group syncable reconciliation failed"
            );
        }
    }
    Ok(())
}

/// Reconcile one syncable: ensure every desired membership (converging
/// roles to the union of active grants) and clean up tracked memberships
/// that are no longer desired.
///
/// Concurrency contract: the whole reconcile runs in ONE transaction that
/// first takes a `FOR SHARE` lock on the syncable's `group_syncables` row.
/// An atomic unlink (`unlink_group_syncable`) deletes that row before
/// revoking grants; without the share lock, a reconcile racing the unlink
/// could read the link as live, re-grant memberships, and commit them
/// AFTER the unlink's cleanup snapshot — stranding grants whose link row
/// is gone and which nothing revisits. With the lock, the unlink's DELETE
/// blocks until the reconcile commits, and the unlink's cleanup then sees
/// (and removes) everything the reconcile granted. Conversely, a reconcile
/// arriving after the unlink's DELETE finds no live row and skips.
/// Shared (not exclusive) so concurrent reconciles of the same syncable
/// still run in parallel.
///
/// Membership rows are locked in a deterministic order (sorted
/// (target_type, target_id, user_id)) in both loops, so concurrent
/// reconcile transactions cannot deadlock against each other over
/// overlapping user sets.
pub(crate) async fn reconcile_group_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let mut tx = state.db.begin().await?;

    let syncable: Option<GroupSyncableRow> = sqlx::query_as(
        r#"
        SELECT gs.group_id, gs.syncable_type, gs.syncable_id, gs.auto_add, gs.scheme_admin
        FROM group_syncables gs
        JOIN groups g ON g.id = gs.group_id AND g.deleted_at IS NULL
        WHERE gs.group_id = $1
          AND gs.syncable_type = $2
          AND gs.syncable_id = $3
          AND gs.delete_at IS NULL
        FOR SHARE OF gs
        "#,
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(syncable) = syncable else {
        return Ok(());
    };

    // A syncable whose team/channel was hard-deleted (nothing enforces an
    // FK on syncable_id) must not abort reconciliation: erroring here
    // would block every other syncable of the group and, via the Keycloak
    // worker, every later group in the cycle. Treat it as unlinked —
    // revoke the memberships it still grants, drop its tracking rows, and
    // remove the dead link. A group whose IdP attributes still reference
    // the deleted target re-creates the link on the next sync, and this
    // path cleans it up again.
    let target_exists: bool = match kind {
        SyncableKind::Team => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
                .bind(syncable_id)
                .fetch_one(&mut *tx)
                .await?
        }
        SyncableKind::Channel => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1)")
                .bind(syncable_id)
                .fetch_one(&mut *tx)
                .await?
        }
    };
    if !target_exists {
        tracing::warn!(
            group_id = %group_id,
            syncable_id = %syncable_id,
            syncable_type = kind.as_db_str(),
            "Group syncable target no longer exists; revoking its grants and removing the link"
        );
        // Atomically remove the dead link and revoke its grants on this
        // transaction (we already hold the link row's share lock, so the
        // pool-backed wrapper would self-deadlock). On failure everything
        // rolls back — the link survives and the next pass retries.
        unlink_group_syncable_in_tx(&mut tx, group_id, kind, syncable_id).await?;
        tx.commit().await?;
        return Ok(());
    }

    let group_user_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&mut *tx)
            .await?;

    let mut desired = HashSet::new();
    if syncable.auto_add {
        match kind {
            SyncableKind::Team => {
                for user_id in &group_user_ids {
                    desired.insert(DesiredMembership {
                        target_type: "team".to_string(),
                        target_id: syncable_id,
                        user_id: *user_id,
                    });
                }
            }
            SyncableKind::Channel => {
                let channel_team_id: Uuid =
                    sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
                        .bind(syncable_id)
                        .fetch_optional(&mut *tx)
                        .await?
                        .ok_or_else(|| AppError::ChannelNotFound)?;

                for user_id in &group_user_ids {
                    desired.insert(DesiredMembership {
                        target_type: "team".to_string(),
                        target_id: channel_team_id,
                        user_id: *user_id,
                    });
                    desired.insert(DesiredMembership {
                        target_type: "channel".to_string(),
                        target_id: syncable_id,
                        user_id: *user_id,
                    });
                }
            }
        }
    }

    let mut existing_tracked: Vec<TrackedMembershipRow> = sqlx::query_as(
        r#"
        SELECT target_type, target_id, user_id
        FROM group_syncable_memberships
        WHERE group_id = $1
          AND syncable_type = $2
          AND syncable_id = $3
        "#,
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .fetch_all(&mut *tx)
    .await?;

    // Deterministic lock order across concurrent reconciles (see the doc
    // comment above).
    let mut desired: Vec<DesiredMembership> = desired.into_iter().collect();
    desired.sort_unstable();
    existing_tracked.sort_unstable();

    for desired_membership in &desired {
        // ensure_membership creates the membership with the granted role,
        // tracks the grant, and converges the stored role to the union of
        // all active grants. Re-running it on every pass — including for
        // already-tracked members — is what makes a `scheme_admin` flip
        // converge on the next sync.
        ensure_membership(
            &mut tx,
            SyncableRef::new(group_id, kind, syncable_id),
            &desired_membership.target_type,
            desired_membership.target_id,
            desired_membership.user_id,
            syncable.scheme_admin,
        )
        .await?;
    }

    for tracked in &existing_tracked {
        let desired_key = DesiredMembership {
            target_type: tracked.target_type.clone(),
            target_id: tracked.target_id,
            user_id: tracked.user_id,
        };
        if !desired.contains(&desired_key) {
            cleanup_tracking_membership(&mut tx, group_id, kind, syncable_id, tracked).await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Purge every group-syncable link pointing at a team (or at any of its
/// channels — those cascade with the team) together with its tracking
/// rows, inside the caller's hard-delete transaction. Nothing enforces an
/// FK on `group_syncables.syncable_id`, so skipping this leaves dangling
/// links that abort reconciliation for their groups. All memberships
/// those syncables granted target the team or its channels and are
/// removed below (the caller's `DELETE FROM teams` would cascade them
/// anyway; deleting them explicitly first keeps the canonical lock
/// order).
pub(crate) async fn purge_team_syncables(
    tx: &mut sqlx::PgConnection,
    team_id: Uuid,
) -> Result<(), sqlx::Error> {
    // Lock order: link rows, then membership rows, then tracking rows —
    // the canonical order shared with ensure_membership,
    // cleanup_tracking_membership, soft_delete_group, and
    // unlink_group_syncable. Deleting the tracking rows before the
    // membership rows (e.g. leaving both to the caller's cascading team
    // delete) inverts the order and can deadlock against a concurrent
    // reconcile cleanup.
    sqlx::query(
        r#"
        DELETE FROM group_syncables gs
        WHERE (gs.syncable_type = 'team' AND gs.syncable_id = $1)
           OR (gs.syncable_type = 'channel' AND gs.syncable_id IN (
                   SELECT c.id FROM channels c WHERE c.team_id = $1))
        "#,
    )
    .bind(team_id)
    .execute(&mut *tx)
    .await?;

    // Exactly the rows the team delete would cascade — deleted first to
    // take their locks in canonical order.
    sqlx::query("DELETE FROM team_members WHERE team_id = $1")
        .bind(team_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        DELETE FROM channel_members cm
        WHERE cm.channel_id IN (SELECT c.id FROM channels c WHERE c.team_id = $1)
        "#,
    )
    .bind(team_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM group_syncable_memberships gsm
        WHERE (gsm.syncable_type = 'team' AND gsm.syncable_id = $1)
           OR (gsm.syncable_type = 'channel' AND gsm.syncable_id IN (
                   SELECT c.id FROM channels c WHERE c.team_id = $1))
        "#,
    )
    .bind(team_id)
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Purge the group-syncable links pointing at a channel and revoke what
/// they granted, inside the caller's hard-delete transaction.
/// `channel_members` cascade with the channel, but a channel syncable
/// also grants its TEAM memberships — those are revoked here (or
/// re-converged when another active grant keeps them) or the group's
/// members silently keep team access. The links themselves are removed
/// because nothing enforces an FK on `group_syncables.syncable_id` and
/// dangling links abort reconciliation for their groups.
///
/// Lock order: link rows, then membership rows, then tracking rows — the
/// canonical order shared with every other grant-cleanup path. The
/// tracking delete must stay AFTER the team_members statements: moving it
/// earlier would invert the order against `cleanup_tracking_membership`
/// and `soft_delete_group` and reintroduce the deadlock the canonical
/// order exists to prevent.
pub(crate) async fn purge_channel_syncables(
    tx: &mut sqlx::PgConnection,
    channel_id: Uuid,
) -> Result<(), sqlx::Error> {
    // Drop the links first: group_syncable_active_grants then excludes
    // this channel's grants for every statement below.
    sqlx::query("DELETE FROM group_syncables WHERE syncable_type = 'channel' AND syncable_id = $1")
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;

    // Revoke team memberships granted by this channel's syncables that no
    // other active grant keeps alive...
    sqlx::query(
        r#"
        DELETE FROM team_members tm
        WHERE (tm.team_id, tm.user_id) IN (
                  SELECT gsm.target_id, gsm.user_id
                  FROM group_syncable_memberships gsm
                  WHERE gsm.syncable_type = 'channel'
                    AND gsm.syncable_id = $1
                    AND gsm.target_type = 'team')
          AND NOT EXISTS (
                  SELECT 1
                  FROM group_syncable_active_grants g
                  WHERE g.target_type = 'team'
                    AND g.target_id = tm.team_id
                    AND g.user_id = tm.user_id)
        "#,
    )
    .bind(channel_id)
    .execute(&mut *tx)
    .await?;

    // ...and re-converge the roles of members that other active grants
    // still keep.
    sqlx::query(
        r#"
        UPDATE team_members tm SET role = (
            SELECT CASE WHEN bool_or(g.role = 'admin') THEN 'admin' ELSE 'member' END
            FROM group_syncable_active_grants g
            WHERE g.target_type = 'team'
              AND g.target_id = tm.team_id
              AND g.user_id = tm.user_id)
        WHERE (tm.team_id, tm.user_id) IN (
                  SELECT gsm.target_id, gsm.user_id
                  FROM group_syncable_memberships gsm
                  WHERE gsm.syncable_type = 'channel'
                    AND gsm.syncable_id = $1
                    AND gsm.target_type = 'team')
        "#,
    )
    .bind(channel_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "DELETE FROM group_syncable_memberships WHERE syncable_type = 'channel' AND syncable_id = $1",
    )
    .bind(channel_id)
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Remove every membership a syncable granted (used when the syncable is
/// unlinked or its group disappears from the identity provider). Runs on
/// the caller's connection/transaction — see [`unlink_group_syncable`] for
/// the atomic link-removal entry point.
pub(crate) async fn cleanup_unlinked_syncable(
    conn: &mut sqlx::PgConnection,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let tracked_rows: Vec<TrackedMembershipRow> = sqlx::query_as(
        r#"
        SELECT target_type, target_id, user_id
        FROM group_syncable_memberships
        WHERE group_id = $1
          AND syncable_type = $2
          AND syncable_id = $3
        "#,
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .fetch_all(&mut *conn)
    .await?;

    for tracked in tracked_rows {
        cleanup_tracking_membership(conn, group_id, kind, syncable_id, &tracked).await?;
    }
    Ok(())
}

/// Atomically unlink a group syncable: remove its link row and revoke
/// every membership it granted in a single transaction.
///
/// On failure the transaction rolls back — the link survives, so the next
/// reconcile pass (or the next IdP attribute sync) retries the cleanup. A
/// non-transactional unlink that failed mid-cleanup would instead leave
/// the grants active with no retry path: reconcile only iterates live
/// syncables, and nothing else revisits a (group, syncable) tuple whose
/// link row is already gone.
///
/// Lock order: link row first, then membership rows, then tracking rows —
/// the canonical order shared with `ensure_membership`,
/// `cleanup_tracking_membership`, and `soft_delete_group`'s transaction.
/// Inside the transaction the link row is already deleted, so the
/// `group_syncable_active_grants` view excludes this syncable's grants
/// for every revoke/converge decision below (in addition to the explicit
/// per-syncable exclusion in `cleanup_tracking_membership`).
///
/// Returns `Ok(false)` when no live link row exists for the tuple.
pub(crate) async fn unlink_group_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<bool> {
    let mut tx = state.db.begin().await?;
    let unlinked = unlink_group_syncable_in_tx(&mut tx, group_id, kind, syncable_id).await?;
    if !unlinked {
        // Dropping the transaction releases its locks; nothing was changed.
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}

/// The unlink body on the caller's transaction (used by
/// [`unlink_group_syncable`] and by reconciliation's dangling-target
/// branch, which already holds the link row's lock — re-entering through
/// the pool-backed wrapper there would self-deadlock on that lock).
async fn unlink_group_syncable_in_tx(
    tx: &mut sqlx::PgConnection,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<bool> {
    let deleted = sqlx::query(
        "DELETE FROM group_syncables WHERE group_id = $1 AND syncable_type = $2 AND syncable_id = $3",
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if deleted == 0 {
        return Ok(false);
    }

    cleanup_unlinked_syncable(tx, group_id, kind, syncable_id).await?;
    Ok(true)
}
