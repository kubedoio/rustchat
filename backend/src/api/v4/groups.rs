use crate::api::AppState;
use crate::auth::policy::permissions;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use crate::models::channel::ChannelType;
use crate::repositories::GroupRepository;
use crate::repositories::group_repository::{
    GroupListRow, GroupRow, GroupSyncableRow, TrackedMembershipRow,
};
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

const GROUP_SOURCE_CUSTOM: &str = "custom";
const GROUP_SOURCE_LDAP: &str = "ldap";
const GROUP_SOURCE_PLUGIN_PREFIX: &str = "plugin_";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SyncableKind {
    Team,
    Channel,
}

impl SyncableKind {
    fn as_db_str(self) -> &'static str {
        match self {
            Self::Team => "team",
            Self::Channel => "channel",
        }
    }

    fn as_mm_type(self) -> &'static str {
        match self {
            Self::Team => "Team",
            Self::Channel => "Channel",
        }
    }
}

#[derive(Debug, Deserialize)]
struct CreateGroupRequest {
    name: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    source: Option<String>,
    remote_id: Option<String>,
    allow_reference: Option<bool>,
    user_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PatchGroupRequest {
    name: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    allow_reference: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GroupSyncablePatch {
    auto_add: Option<bool>,
    scheme_admin: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GroupModifyMembersRequest {
    user_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DesiredMembership {
    target_type: String,
    target_id: Uuid,
    user_id: Uuid,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/groups", get(get_groups).post(create_group))
        .route("/groups/{group_id}", get(get_group).delete(delete_group))
        .route("/groups/{group_id}/patch", put(patch_group))
        .route("/groups/{group_id}/restore", post(restore_group))
        .route(
            "/groups/{group_id}/teams/{team_id}/link",
            post(link_group_team_syncable).delete(unlink_group_team_syncable),
        )
        .route(
            "/groups/{group_id}/channels/{channel_id}/link",
            post(link_group_channel_syncable).delete(unlink_group_channel_syncable),
        )
        .route(
            "/groups/{group_id}/teams/{team_id}",
            get(get_group_team_syncable),
        )
        .route(
            "/groups/{group_id}/channels/{channel_id}",
            get(get_group_channel_syncable),
        )
        .route("/groups/{group_id}/teams", get(get_group_team_syncables))
        .route(
            "/groups/{group_id}/channels",
            get(get_group_channel_syncables),
        )
        .route(
            "/groups/{group_id}/teams/{team_id}/patch",
            put(patch_group_team_syncable),
        )
        .route(
            "/groups/{group_id}/channels/{channel_id}/patch",
            put(patch_group_channel_syncable),
        )
        .route("/groups/{group_id}/stats", get(get_group_stats))
        .route(
            "/groups/{group_id}/members",
            get(get_group_members)
                .post(add_group_members)
                .delete(delete_group_members),
        )
        .route("/groups/names", post(get_groups_by_names))
}

fn ts_millis(ts: DateTime<Utc>) -> i64 {
    ts.timestamp_millis()
}

fn team_type_value(is_public: bool) -> &'static str {
    if is_public {
        "O"
    } else {
        "I"
    }
}

fn channel_type_value(channel_type: ChannelType) -> &'static str {
    match channel_type {
        ChannelType::Public => "O",
        ChannelType::Private => "P",
        ChannelType::Direct => "D",
        ChannelType::Group => "G",
    }
}

fn group_json(row: &GroupListRow) -> Value {
    json!({
        "id": encode_mm_id(row.id),
        "name": row.name,
        "display_name": row.display_name,
        "description": row.description,
        "source": row.source,
        "remote_id": row.remote_id,
        "create_at": ts_millis(row.created_at),
        "update_at": ts_millis(row.updated_at),
        "delete_at": row.deleted_at.map(ts_millis).unwrap_or(0),
        "has_syncables": row.has_syncables,
        "member_count": row.member_count,
        "allow_reference": row.allow_reference,
    })
}

fn group_member_json(
    group_id: Uuid,
    user_id: Uuid,
    created_at: DateTime<Utc>,
    delete_at: i64,
) -> Value {
    json!({
        "group_id": encode_mm_id(group_id),
        "user_id": encode_mm_id(user_id),
        "create_at": ts_millis(created_at),
        "delete_at": delete_at,
    })
}

async fn emit_received_group_event(state: &AppState, group: &GroupListRow) {
    let group_payload = group_json(group);
    let group_encoded = serde_json::to_string(&group_payload).unwrap_or_else(|_| "{}".to_string());
    let event = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::ReceivedGroup,
        json!({ "group": group_encoded }),
        None,
    );
    state.ws_hub.broadcast(event).await;
}

async fn emit_group_member_event(
    state: &AppState,
    user_id: Uuid,
    group_member_payload: Value,
    is_add: bool,
) {
    let event_type = if is_add {
        crate::realtime::EventType::GroupMemberAdd
    } else {
        crate::realtime::EventType::GroupMemberDeleted
    };
    let group_member_encoded =
        serde_json::to_string(&group_member_payload).unwrap_or_else(|_| "{}".to_string());

    let event = crate::realtime::WsEnvelope::event(
        event_type,
        json!({ "group_member": group_member_encoded }),
        None,
    )
    .with_broadcast(crate::realtime::WsBroadcast {
        channel_id: None,
        team_id: None,
        user_id: Some(user_id),
        exclude_user_id: None,
    });
    state.ws_hub.broadcast(event).await;
}

async fn emit_group_syncable_event(
    state: &AppState,
    syncable_kind: SyncableKind,
    syncable_id: Uuid,
    group_id: Uuid,
    associated: bool,
) {
    let (event_type, broadcast) = match (syncable_kind, associated) {
        (SyncableKind::Team, true) => (
            crate::realtime::EventType::ReceivedGroupAssociatedToTeam,
            crate::realtime::WsBroadcast {
                channel_id: None,
                team_id: Some(syncable_id),
                user_id: None,
                exclude_user_id: None,
            },
        ),
        (SyncableKind::Team, false) => (
            crate::realtime::EventType::ReceivedGroupNotAssociatedToTeam,
            crate::realtime::WsBroadcast {
                channel_id: None,
                team_id: Some(syncable_id),
                user_id: None,
                exclude_user_id: None,
            },
        ),
        (SyncableKind::Channel, true) => (
            crate::realtime::EventType::ReceivedGroupAssociatedToChannel,
            crate::realtime::WsBroadcast {
                channel_id: Some(syncable_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            },
        ),
        (SyncableKind::Channel, false) => (
            crate::realtime::EventType::ReceivedGroupNotAssociatedToChannel,
            crate::realtime::WsBroadcast {
                channel_id: Some(syncable_id),
                team_id: None,
                user_id: None,
                exclude_user_id: None,
            },
        ),
    };

    let event =
        crate::realtime::WsEnvelope::event(event_type, json!({ "group_id": group_id }), None)
            .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;
}

fn can_manage_system_groups(auth: &crate::api::v4::extractors::MmAuthUser) -> bool {
    auth.has_permission(&permissions::SYSTEM_MANAGE)
        || auth.has_permission(&permissions::ADMIN_FULL)
}

fn require_system_groups_read(auth: &crate::api::v4::extractors::MmAuthUser) -> ApiResult<()> {
    if can_manage_system_groups(auth) {
        return Ok(());
    }

    Err(AppError::Forbidden(
        "Insufficient permissions to read group management data".to_string(),
    ))
}

fn require_system_groups_write(auth: &crate::api::v4::extractors::MmAuthUser) -> ApiResult<()> {
    if can_manage_system_groups(auth) {
        return Ok(());
    }

    Err(AppError::Forbidden(
        "Insufficient permissions to manage groups".to_string(),
    ))
}

async fn fetch_group_for_syncable(state: &AppState, group_id: Uuid) -> ApiResult<GroupRow> {
    let group = GroupRepository::new(&state.db)
        .get_group_row_by_id(group_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    Ok(group)
}

async fn is_team_admin_or_owner(state: &AppState, team_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
    let is_admin = GroupRepository::new(&state.db)
        .is_team_admin_or_owner(team_id, user_id)
        .await?;

    Ok(is_admin)
}

async fn can_manage_channel_syncable(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
) -> ApiResult<bool> {
    let is_channel_admin = GroupRepository::new(&state.db)
        .is_channel_admin(channel_id, user_id)
        .await?;

    if is_channel_admin {
        return Ok(true);
    }

    let team_id = GroupRepository::new(&state.db)
        .get_channel_team_id(channel_id)
        .await?;

    let Some(team_id) = team_id else {
        return Ok(false);
    };

    is_team_admin_or_owner(state, team_id, user_id).await
}

fn ensure_group_is_syncable(group: &GroupRow) -> ApiResult<()> {
    if group.source == GROUP_SOURCE_LDAP || group.source.starts_with(GROUP_SOURCE_PLUGIN_PREFIX) {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "Only LDAP or plugin groups can be linked to syncables".to_string(),
    ))
}

async fn verify_link_unlink_permission(
    state: &AppState,
    auth: &crate::api::v4::extractors::MmAuthUser,
    group: &GroupRow,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    if can_manage_system_groups(auth) {
        return Ok(());
    }

    // Non-system group managers can only link referenceable groups.
    if !group.allow_reference {
        return Err(AppError::Forbidden(
            "Insufficient permissions to link non-referenceable group".to_string(),
        ));
    }

    match kind {
        SyncableKind::Team => {
            let is_team_admin = is_team_admin_or_owner(state, syncable_id, auth.user_id).await?;
            if !is_team_admin {
                return Err(AppError::Forbidden(
                    "Insufficient permissions to link group to team".to_string(),
                ));
            }
        }
        SyncableKind::Channel => {
            let can_manage = can_manage_channel_syncable(state, syncable_id, auth.user_id).await?;
            if !can_manage {
                return Err(AppError::Forbidden(
                    "Insufficient permissions to link group to channel".to_string(),
                ));
            }
        }
    }

    Ok(())
}

async fn ensure_syncable_exists(
    state: &AppState,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    match kind {
        SyncableKind::Team => {
            let exists = GroupRepository::new(&state.db).team_exists(syncable_id).await?;
            if !exists {
                return Err(AppError::NotFound("Team not found".to_string()));
            }
        }
        SyncableKind::Channel => {
            let exists = GroupRepository::new(&state.db).channel_exists(syncable_id).await?;
            if !exists {
                return Err(AppError::NotFound("Channel not found".to_string()));
            }
        }
    }

    Ok(())
}

async fn syncable_payload(
    state: &AppState,
    row: &GroupSyncableRow,
    kind: SyncableKind,
) -> ApiResult<Value> {
    let mut payload = json!({
        "group_id": encode_mm_id(row.group_id),
        "auto_add": row.auto_add,
        "scheme_admin": row.scheme_admin,
        "create_at": ts_millis(row.create_at),
        "update_at": ts_millis(row.update_at),
        "delete_at": row.delete_at.map(ts_millis).unwrap_or(0),
        "type": kind.as_mm_type(),
    });

    match kind {
        SyncableKind::Team => {
            let team = GroupRepository::new(&state.db)
                .get_team_meta(row.syncable_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Team not found".to_string()))?;

            payload["team_id"] = json!(encode_mm_id(team.id));
            payload["team_display_name"] = json!(team.display_name.unwrap_or(team.name));
            payload["team_type"] = json!(team_type_value(team.is_public));
        }
        SyncableKind::Channel => {
            let channel = GroupRepository::new(&state.db)
                .get_channel_meta(row.syncable_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Channel not found".to_string()))?;

            payload["channel_id"] = json!(encode_mm_id(channel.id));
            payload["channel_display_name"] = json!(channel.display_name.unwrap_or(channel.name));
            payload["channel_type"] = json!(channel_type_value(channel.channel_type));
            payload["team_id"] = json!(encode_mm_id(channel.team_id));
            payload["team_display_name"] =
                json!(channel.team_display_name.unwrap_or(channel.team_name));
            payload["team_type"] = json!(team_type_value(channel.team_is_public));
        }
    }

    Ok(payload)
}

fn parse_user_ids(user_ids: &[String]) -> ApiResult<Vec<Uuid>> {
    let mut parsed = Vec::with_capacity(user_ids.len());
    for user_id in user_ids {
        let uuid = parse_mm_or_uuid(user_id)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid user_id: {user_id}")))?;
        parsed.push(uuid);
    }
    Ok(parsed)
}

async fn load_group_syncables(
    state: &AppState,
    group_id: Uuid,
) -> ApiResult<Vec<GroupSyncableRow>> {
    let rows = GroupRepository::new(&state.db)
        .list_group_syncables(group_id)
        .await?;

    Ok(rows)
}

fn spawn_reconcile_syncable(
    state: AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) {
    tokio::spawn(async move {
        if let Err(err) = reconcile_group_syncable(&state, group_id, kind, syncable_id).await {
            tracing::warn!(
                group_id = %group_id,
                syncable_id = %syncable_id,
                syncable_type = %kind.as_db_str(),
                error = %err,
                "Group syncable reconciliation failed"
            );
        }
    });
}

fn spawn_reconcile_group_syncables(state: AppState, group_id: Uuid) {
    tokio::spawn(async move {
        let syncables = match load_group_syncables(&state, group_id).await {
            Ok(rows) => rows,
            Err(err) => {
                tracing::warn!(group_id = %group_id, error = %err, "Failed to load group syncables for reconcile");
                return;
            }
        };

        for row in syncables {
            let kind = if row.syncable_type == "team" {
                SyncableKind::Team
            } else {
                SyncableKind::Channel
            };
            if let Err(err) =
                reconcile_group_syncable(&state, row.group_id, kind, row.syncable_id).await
            {
                tracing::warn!(
                    group_id = %row.group_id,
                    syncable_id = %row.syncable_id,
                    syncable_type = %row.syncable_type,
                    error = %err,
                    "Group syncable reconciliation failed"
                );
            }
        }
    });
}

async fn cleanup_tracking_membership(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
    tracked: &TrackedMembershipRow,
) -> ApiResult<()> {
    GroupRepository::new(&state.db)
        .delete_group_syncable_membership(
            group_id,
            kind.as_db_str(),
            syncable_id,
            &tracked.target_type,
            tracked.target_id,
            tracked.user_id,
        )
        .await?;

    let kept_by_other_syncable = GroupRepository::new(&state.db)
        .has_other_syncable_memberships(&tracked.target_type, tracked.target_id, tracked.user_id)
        .await?;

    if kept_by_other_syncable {
        return Ok(());
    }

    if tracked.target_type == "team" {
        GroupRepository::new(&state.db)
            .remove_team_member(tracked.target_id, tracked.user_id)
            .await?;
    } else {
        GroupRepository::new(&state.db)
            .remove_channel_member(tracked.target_id, tracked.user_id)
            .await?;
    }

    Ok(())
}

async fn ensure_membership(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
    user_id: Uuid,
    scheme_admin: bool,
) -> ApiResult<bool> {
    let role = if scheme_admin { "admin" } else { "member" };

    let rows_affected = if target_type == "team" {
        GroupRepository::new(&state.db)
            .ensure_team_member(target_id, user_id, role)
            .await?
    } else {
        GroupRepository::new(&state.db)
            .ensure_channel_member(target_id, user_id, role)
            .await?
    };

    Ok(rows_affected > 0)
}

async fn reconcile_group_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let syncable = GroupRepository::new(&state.db)
        .get_group_syncable(group_id, kind.as_db_str(), syncable_id)
        .await?;

    let Some(syncable) = syncable else {
        return Ok(());
    };

    let group_user_ids = GroupRepository::new(&state.db)
        .list_group_user_ids(group_id)
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
                let channel_team_id = GroupRepository::new(&state.db)
                    .get_channel_team_id(syncable_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("Channel not found".to_string()))?;

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

    let existing_tracked = GroupRepository::new(&state.db)
        .list_group_syncable_memberships(group_id, kind.as_db_str(), syncable_id)
        .await?;

    let mut tracked_set: HashSet<TrackedMembershipRow> = existing_tracked.iter().cloned().collect();

    for desired_membership in &desired {
        let key = TrackedMembershipRow {
            target_type: desired_membership.target_type.clone(),
            target_id: desired_membership.target_id,
            user_id: desired_membership.user_id,
        };

        if tracked_set.contains(&key) {
            continue;
        }

        let inserted = ensure_membership(
            state,
            &desired_membership.target_type,
            desired_membership.target_id,
            desired_membership.user_id,
            syncable.scheme_admin,
        )
        .await?;

        if inserted {
            GroupRepository::new(&state.db)
                .insert_group_syncable_membership(
                    group_id,
                    kind.as_db_str(),
                    syncable_id,
                    &desired_membership.target_type,
                    desired_membership.target_id,
                    desired_membership.user_id,
                )
                .await?;

            tracked_set.insert(key);
        }
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

async fn cleanup_unlinked_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let tracked_rows = GroupRepository::new(&state.db)
        .list_group_syncable_memberships(group_id, kind.as_db_str(), syncable_id)
        .await?;

    for tracked in tracked_rows {
        cleanup_tracking_membership(state, group_id, kind, syncable_id, &tracked).await?;
    }

    Ok(())
}

/// GET /api/v4/groups
async fn get_groups(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<Value>>> {
    require_system_groups_read(&auth)?;

    let groups = GroupRepository::new(&state.db).list_groups().await?;

    Ok(Json(groups.iter().map(group_json).collect()))
}

/// POST /api/v4/groups
async fn create_group(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(group): Json<CreateGroupRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Value>)> {
    require_system_groups_write(&auth)?;

    let source = group
        .source
        .as_deref()
        .unwrap_or(GROUP_SOURCE_CUSTOM)
        .to_ascii_lowercase();

    if source != GROUP_SOURCE_CUSTOM {
        return Err(AppError::BadRequest(
            "Only custom groups can be created from this endpoint".to_string(),
        ));
    }

    let allow_reference = group.allow_reference.unwrap_or(true);
    if !allow_reference {
        return Err(AppError::BadRequest(
            "Custom groups must allow references".to_string(),
        ));
    }

    if group
        .remote_id
        .as_ref()
        .is_some_and(|remote_id| !remote_id.is_empty())
    {
        return Err(AppError::BadRequest(
            "Custom groups cannot have remote_id".to_string(),
        ));
    }

    let display_name = group
        .display_name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("display_name is required".to_string()))?;

    let user_ids = parse_user_ids(group.user_ids.as_deref().unwrap_or(&[]))?;

    let created = GroupRepository::new(&state.db)
        .create_group_with_members(
            group.name.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            display_name,
            group.description.unwrap_or_default(),
            source,
            allow_reference,
            user_ids,
        )
        .await?;

    let row = GroupListRow {
        id: created.id,
        name: created.name,
        display_name: created.display_name,
        description: created.description,
        source: created.source,
        remote_id: created.remote_id,
        allow_reference: created.allow_reference,
        created_at: created.created_at,
        updated_at: created.updated_at,
        deleted_at: created.deleted_at,
        has_syncables: false,
        member_count: i64::from(group.user_ids.as_ref().map(Vec::len).unwrap_or(0) as i32),
    };

    emit_received_group_event(&state, &row).await;

    Ok((axum::http::StatusCode::CREATED, Json(group_json(&row))))
}

/// GET /api/v4/groups/{group_id}
async fn get_group(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_system_groups_read(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let group = GroupRepository::new(&state.db)
        .get_group_list_by_id(group_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    Ok(Json(group_json(&group)))
}

/// PUT /api/v4/groups/{group_id}/patch
async fn patch_group(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
    Json(patch): Json<PatchGroupRequest>,
) -> ApiResult<Json<Value>> {
    require_system_groups_write(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let current = GroupRepository::new(&state.db)
        .get_group_row_by_id_unchecked(group_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    if current.source == GROUP_SOURCE_CUSTOM && patch.allow_reference == Some(false) {
        return Err(AppError::BadRequest(
            "Custom groups must allow references".to_string(),
        ));
    }

    let updated = GroupRepository::new(&state.db)
        .update_group(
            group_id,
            patch
                .name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            patch
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            patch.description,
            patch.allow_reference,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    emit_received_group_event(&state, &updated).await;

    Ok(Json(group_json(&updated)))
}

/// DELETE /api/v4/groups/{group_id}
async fn delete_group(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_system_groups_write(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let deleted_group = GroupRepository::new(&state.db)
        .soft_delete_group(group_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    emit_received_group_event(&state, &deleted_group).await;

    Ok(Json(json!({"status": "OK"})))
}

/// POST /api/v4/groups/{group_id}/restore
async fn restore_group(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_system_groups_write(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let group = GroupRepository::new(&state.db)
        .restore_group(group_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    emit_received_group_event(&state, &group).await;

    Ok(Json(group_json(&group)))
}

async fn link_group_syncable_by_kind(
    state: AppState,
    auth: crate::api::v4::extractors::MmAuthUser,
    group_id: String,
    syncable_id: String,
    kind: SyncableKind,
    patch: GroupSyncablePatch,
) -> ApiResult<(axum::http::StatusCode, Json<Value>)> {
    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let syncable_id = parse_mm_or_uuid(&syncable_id)
        .ok_or_else(|| AppError::BadRequest("Invalid syncable_id".to_string()))?;

    let group = fetch_group_for_syncable(&state, group_id).await?;
    ensure_group_is_syncable(&group)?;
    ensure_syncable_exists(&state, kind, syncable_id).await?;
    verify_link_unlink_permission(&state, &auth, &group, kind, syncable_id).await?;

    let syncable = GroupRepository::new(&state.db)
        .upsert_group_syncable(
            group_id,
            kind.as_db_str(),
            syncable_id,
            patch.auto_add.unwrap_or(false),
            patch.scheme_admin.unwrap_or(false),
        )
        .await?;

    spawn_reconcile_syncable(state.clone(), group_id, kind, syncable_id);
    emit_group_syncable_event(&state, kind, syncable_id, group_id, true).await;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(syncable_payload(&state, &syncable, kind).await?),
    ))
}

async fn unlink_group_syncable_by_kind(
    state: AppState,
    auth: crate::api::v4::extractors::MmAuthUser,
    group_id: String,
    syncable_id: String,
    kind: SyncableKind,
) -> ApiResult<Json<Value>> {
    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let syncable_id = parse_mm_or_uuid(&syncable_id)
        .ok_or_else(|| AppError::BadRequest("Invalid syncable_id".to_string()))?;
    let group = fetch_group_for_syncable(&state, group_id).await?;
    ensure_group_is_syncable(&group)?;
    ensure_syncable_exists(&state, kind, syncable_id).await?;
    verify_link_unlink_permission(&state, &auth, &group, kind, syncable_id).await?;

    let deleted = GroupRepository::new(&state.db)
        .delete_group_syncable(group_id, kind.as_db_str(), syncable_id)
        .await?;

    if deleted == 0 {
        return Err(AppError::NotFound("Group syncable not found".to_string()));
    }

    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(err) = cleanup_unlinked_syncable(&state_clone, group_id, kind, syncable_id).await
        {
            tracing::warn!(
                group_id = %group_id,
                syncable_id = %syncable_id,
                syncable_type = %kind.as_db_str(),
                error = %err,
                "Group syncable unlink cleanup failed"
            );
        }
    });

    emit_group_syncable_event(&state, kind, syncable_id, group_id, false).await;

    Ok(Json(json!({"status": "OK"})))
}

async fn get_group_syncable_by_kind(
    state: AppState,
    auth: crate::api::v4::extractors::MmAuthUser,
    group_id: String,
    syncable_id: String,
    kind: SyncableKind,
) -> ApiResult<Json<Value>> {
    require_system_groups_read(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let syncable_id = parse_mm_or_uuid(&syncable_id)
        .ok_or_else(|| AppError::BadRequest("Invalid syncable_id".to_string()))?;

    let row = GroupRepository::new(&state.db)
        .get_group_syncable(group_id, kind.as_db_str(), syncable_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group syncable not found".to_string()))?;

    Ok(Json(syncable_payload(&state, &row, kind).await?))
}

async fn get_group_syncables_by_kind(
    state: AppState,
    auth: crate::api::v4::extractors::MmAuthUser,
    group_id: String,
    kind: SyncableKind,
) -> ApiResult<Json<Vec<Value>>> {
    require_system_groups_read(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let rows = GroupRepository::new(&state.db)
        .list_group_syncables_by_type(group_id, kind.as_db_str())
        .await?;

    let mut response = Vec::with_capacity(rows.len());
    for row in rows {
        response.push(syncable_payload(&state, &row, kind).await?);
    }

    Ok(Json(response))
}

async fn patch_group_syncable_by_kind(
    state: AppState,
    auth: crate::api::v4::extractors::MmAuthUser,
    group_id: String,
    syncable_id: String,
    kind: SyncableKind,
    patch: GroupSyncablePatch,
) -> ApiResult<Json<Value>> {
    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let syncable_id = parse_mm_or_uuid(&syncable_id)
        .ok_or_else(|| AppError::BadRequest("Invalid syncable_id".to_string()))?;
    let group = fetch_group_for_syncable(&state, group_id).await?;
    ensure_group_is_syncable(&group)?;
    ensure_syncable_exists(&state, kind, syncable_id).await?;
    verify_link_unlink_permission(&state, &auth, &group, kind, syncable_id).await?;

    let row = GroupRepository::new(&state.db)
        .patch_group_syncable(
            group_id,
            kind.as_db_str(),
            syncable_id,
            patch.auto_add,
            patch.scheme_admin,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Group syncable not found".to_string()))?;

    spawn_reconcile_syncable(state.clone(), group_id, kind, syncable_id);

    Ok(Json(syncable_payload(&state, &row, kind).await?))
}

/// POST /api/v4/groups/{group_id}/teams/{team_id}/link
async fn link_group_team_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, team_id)): Path<(String, String)>,
    Json(patch): Json<GroupSyncablePatch>,
) -> ApiResult<(axum::http::StatusCode, Json<Value>)> {
    link_group_syncable_by_kind(state, auth, group_id, team_id, SyncableKind::Team, patch).await
}

/// POST /api/v4/groups/{group_id}/channels/{channel_id}/link
async fn link_group_channel_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, channel_id)): Path<(String, String)>,
    Json(patch): Json<GroupSyncablePatch>,
) -> ApiResult<(axum::http::StatusCode, Json<Value>)> {
    link_group_syncable_by_kind(
        state,
        auth,
        group_id,
        channel_id,
        SyncableKind::Channel,
        patch,
    )
    .await
}

/// DELETE /api/v4/groups/{group_id}/teams/{team_id}/link
async fn unlink_group_team_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, team_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    unlink_group_syncable_by_kind(state, auth, group_id, team_id, SyncableKind::Team).await
}

/// DELETE /api/v4/groups/{group_id}/channels/{channel_id}/link
async fn unlink_group_channel_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, channel_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    unlink_group_syncable_by_kind(state, auth, group_id, channel_id, SyncableKind::Channel).await
}

/// GET /api/v4/groups/{group_id}/teams/{team_id}
async fn get_group_team_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, team_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    get_group_syncable_by_kind(state, auth, group_id, team_id, SyncableKind::Team).await
}

/// GET /api/v4/groups/{group_id}/channels/{channel_id}
async fn get_group_channel_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, channel_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    get_group_syncable_by_kind(state, auth, group_id, channel_id, SyncableKind::Channel).await
}

/// GET /api/v4/groups/{group_id}/teams
async fn get_group_team_syncables(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    get_group_syncables_by_kind(state, auth, group_id, SyncableKind::Team).await
}

/// GET /api/v4/groups/{group_id}/channels
async fn get_group_channel_syncables(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    get_group_syncables_by_kind(state, auth, group_id, SyncableKind::Channel).await
}

/// PUT /api/v4/groups/{group_id}/teams/{team_id}/patch
async fn patch_group_team_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, team_id)): Path<(String, String)>,
    Json(patch): Json<GroupSyncablePatch>,
) -> ApiResult<Json<Value>> {
    patch_group_syncable_by_kind(state, auth, group_id, team_id, SyncableKind::Team, patch).await
}

/// PUT /api/v4/groups/{group_id}/channels/{channel_id}/patch
async fn patch_group_channel_syncable(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path((group_id, channel_id)): Path<(String, String)>,
    Json(patch): Json<GroupSyncablePatch>,
) -> ApiResult<Json<Value>> {
    patch_group_syncable_by_kind(
        state,
        auth,
        group_id,
        channel_id,
        SyncableKind::Channel,
        patch,
    )
    .await
}

/// GET /api/v4/groups/{group_id}/stats
async fn get_group_stats(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_system_groups_read(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let count = GroupRepository::new(&state.db)
        .count_group_members(group_id)
        .await?;

    Ok(Json(json!({
        "group_id": encode_mm_id(group_id),
        "total_member_count": count,
    })))
}

/// GET /api/v4/groups/{group_id}/members
async fn get_group_members(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_system_groups_read(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;

    let rows = GroupRepository::new(&state.db)
        .list_group_members(group_id)
        .await?;

    let members: Vec<Value> = rows
        .iter()
        .map(|(user_id, created_at)| group_member_json(group_id, *user_id, *created_at, 0))
        .collect();

    Ok(Json(json!({
        "members": members,
        "total_member_count": members.len(),
    })))
}

/// POST /api/v4/groups/{group_id}/members
async fn add_group_members(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
    Json(members): Json<GroupModifyMembersRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<Vec<Value>>)> {
    require_system_groups_write(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let user_ids = parse_user_ids(&members.user_ids)?;

    fetch_group_for_syncable(&state, group_id).await?;

    let mut added = Vec::new();
    for user_id in user_ids {
        let inserted = GroupRepository::new(&state.db)
            .add_group_member(group_id, user_id)
            .await?;

        if let Some(created_at) = inserted {
            let payload = group_member_json(group_id, user_id, created_at, 0);
            emit_group_member_event(&state, user_id, payload.clone(), true).await;
            added.push(payload);
        }
    }

    spawn_reconcile_group_syncables(state.clone(), group_id);

    Ok((axum::http::StatusCode::CREATED, Json(added)))
}

/// DELETE /api/v4/groups/{group_id}/members
async fn delete_group_members(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Path(group_id): Path<String>,
    Json(members): Json<GroupModifyMembersRequest>,
) -> ApiResult<Json<Vec<Value>>> {
    require_system_groups_write(&auth)?;

    let group_id = parse_mm_or_uuid(&group_id)
        .ok_or_else(|| AppError::BadRequest("Invalid group_id".to_string()))?;
    let user_ids = parse_user_ids(&members.user_ids)?;

    fetch_group_for_syncable(&state, group_id).await?;

    let mut deleted = Vec::new();
    let now_ms = Utc::now().timestamp_millis();
    for user_id in user_ids {
        let deleted_row = GroupRepository::new(&state.db)
            .remove_group_member(group_id, user_id)
            .await?;

        if let Some(created_at) = deleted_row {
            let payload = group_member_json(group_id, user_id, created_at, now_ms);
            emit_group_member_event(&state, user_id, payload.clone(), false).await;
            deleted.push(payload);
        }
    }

    spawn_reconcile_group_syncables(state.clone(), group_id);

    Ok(Json(deleted))
}

/// POST /api/v4/groups/names
async fn get_groups_by_names(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(names): Json<Vec<String>>,
) -> ApiResult<Json<Vec<Value>>> {
    require_system_groups_read(&auth)?;

    if names.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let rows = GroupRepository::new(&state.db)
        .list_groups_by_names(names)
        .await?;

    Ok(Json(rows.iter().map(group_json).collect()))
}
