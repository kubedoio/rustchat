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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DesiredMembership {
    target_type: String,
    target_id: Uuid,
    user_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq, Hash)]
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
async fn upsert_tracking_membership(
    state: &AppState,
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
    .execute(&state.db)
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
    state: &AppState,
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
        .execute(&state.db)
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
        .execute(&state.db)
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
    state: &AppState,
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
        .execute(&state.db)
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
        .execute(&state.db)
        .await?
        .rows_affected()
            > 0
    };

    if created {
        upsert_tracking_membership(state, syncable, target_type, target_id, user_id, role).await?;
        converge_role_to_grants(state, target_type, target_id, user_id).await?;
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
    .fetch_one(&state.db)
    .await?;

    if !sync_granted {
        return Ok(());
    }

    upsert_tracking_membership(state, syncable, target_type, target_id, user_id, role).await?;
    converge_role_to_grants(state, target_type, target_id, user_id).await?;
    Ok(())
}

/// Remove one syncable's tracking row and converge the membership it
/// tracked: revoke it when no other active grant keeps it alive, otherwise
/// re-converge the role to the remaining grants.
async fn cleanup_tracking_membership(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
    tracked: &TrackedMembershipRow,
) -> ApiResult<()> {
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
    .execute(&state.db)
    .await?;

    let kept_by_other_syncable: bool = sqlx::query_scalar(
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
    .bind(&tracked.target_type)
    .bind(tracked.target_id)
    .bind(tracked.user_id)
    .fetch_one(&state.db)
    .await?;

    if kept_by_other_syncable {
        // Other syncables still grant this membership: converge the role to
        // the union of the remaining grants instead of leaving it at
        // whatever the removed syncable last wrote (an admin grant removed
        // second must not strand the member at 'member' — or vice versa).
        converge_role_to_grants(
            state,
            &tracked.target_type,
            tracked.target_id,
            tracked.user_id,
        )
        .await?;
        return Ok(());
    }

    if tracked.target_type == "team" {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(tracked.target_id)
            .bind(tracked.user_id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(tracked.target_id)
            .bind(tracked.user_id)
            .execute(&state.db)
            .await?;
    }

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
        reconcile_group_syncable(state, group_id, kind, syncable_id).await?;
    }
    Ok(())
}

/// Reconcile one syncable: ensure every desired membership (converging
/// roles to the union of active grants) and clean up tracked memberships
/// that are no longer desired.
pub(crate) async fn reconcile_group_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let syncable: Option<GroupSyncableRow> = sqlx::query_as(
        r#"
        SELECT gs.group_id, gs.syncable_type, gs.syncable_id, gs.auto_add, gs.scheme_admin
        FROM group_syncables gs
        JOIN groups g ON g.id = gs.group_id AND g.deleted_at IS NULL
        WHERE gs.group_id = $1
          AND gs.syncable_type = $2
          AND gs.syncable_id = $3
          AND gs.delete_at IS NULL
        "#,
    )
    .bind(group_id)
    .bind(kind.as_db_str())
    .bind(syncable_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(syncable) = syncable else {
        return Ok(());
    };

    let group_user_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&state.db)
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
                        .fetch_optional(&state.db)
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

    let existing_tracked: Vec<TrackedMembershipRow> = sqlx::query_as(
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
    .fetch_all(&state.db)
    .await?;

    for desired_membership in &desired {
        // ensure_membership creates the membership with the granted role,
        // tracks the grant, and converges the stored role to the union of
        // all active grants. Re-running it on every pass — including for
        // already-tracked members — is what makes a `scheme_admin` flip
        // converge on the next sync.
        ensure_membership(
            state,
            SyncableRef::new(group_id, kind, syncable_id),
            &desired_membership.target_type,
            desired_membership.target_id,
            desired_membership.user_id,
            syncable.scheme_admin,
        )
        .await?;
    }

    for tracked in existing_tracked {
        let desired_key = DesiredMembership {
            target_type: tracked.target_type.clone(),
            target_id: tracked.target_id,
            user_id: tracked.user_id,
        };
        if !desired.contains(&desired_key) {
            cleanup_tracking_membership(state, group_id, kind, syncable_id, &tracked).await?;
        }
    }

    Ok(())
}

/// Remove every membership a syncable granted (used when the syncable is
/// unlinked or its group disappears from the identity provider).
pub(crate) async fn cleanup_unlinked_syncable(
    state: &AppState,
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
    .fetch_all(&state.db)
    .await?;

    for tracked in tracked_rows {
        cleanup_tracking_membership(state, group_id, kind, syncable_id, &tracked).await?;
    }
    Ok(())
}
