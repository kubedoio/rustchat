//! Keycloak OIDC group synchronization service.
//!
//! Synchronizes Keycloak groups and memberships into RustChat groups/syncables.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};
use uuid::Uuid;

use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

const GROUP_SOURCE_KEYCLOAK: &str = "plugin_keycloak";

#[derive(Debug, Clone, serde::Serialize)]
pub struct KeycloakSyncReport {
    pub groups_processed: usize,
    pub groups_upserted: usize,
    pub group_members_added: usize,
    pub group_members_removed: usize,
    pub syncables_upserted: usize,
    pub syncables_removed: usize,
    pub users_mapped: usize,
}

#[derive(Debug, Deserialize)]
struct KeycloakTokenResponse {
    access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeycloakGroup {
    id: String,
    name: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    attributes: HashMap<String, Vec<String>>,
    #[serde(default)]
    sub_groups: Vec<KeycloakGroup>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct KeycloakUser {
    id: String,
    username: Option<String>,
    email: Option<String>,
}

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
struct GroupSyncableRow {
    syncable_type: String,
    syncable_id: Uuid,
    auto_add: bool,
    scheme_admin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

pub fn spawn_periodic_keycloak_sync(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval_secs = state.config.keycloak_sync.interval_seconds.max(30);
        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if !state.config.keycloak_sync.enabled {
                continue;
            }

            if let Err(err) = run_full_sync(&state).await {
                warn!(error = %err, "Periodic Keycloak sync failed");
            }
        }
    })
}

pub async fn run_full_sync(state: &AppState) -> ApiResult<KeycloakSyncReport> {
    validate_sync_config(state)?;

    let token = fetch_admin_token(state).await?;
    let mut groups = fetch_groups(state, &token).await?;
    flatten_groups(&mut groups);

    let provider_key = &state.config.keycloak_sync.provider_key;
    let mut report = KeycloakSyncReport {
        groups_processed: 0,
        groups_upserted: 0,
        group_members_added: 0,
        group_members_removed: 0,
        syncables_upserted: 0,
        syncables_removed: 0,
        users_mapped: 0,
    };

    let mut desired_remote_ids = HashSet::new();

    for kc_group in &groups {
        desired_remote_ids.insert(kc_group.id.clone());
        report.groups_processed += 1;

        let group_id = upsert_group(state, kc_group).await?;
        report.groups_upserted += 1;

        let kc_members = fetch_group_members(state, &token, &kc_group.id).await?;
        let desired_user_ids =
            resolve_rustchat_users_for_keycloak_members(state, provider_key, &kc_members).await?;
        report.users_mapped += desired_user_ids.len();

        let (added_users, removed_users) =
            sync_group_members(state, group_id, &desired_user_ids).await?;
        report.group_members_added += added_users.len();
        report.group_members_removed += removed_users.len();
        for (user_id, created_at) in added_users {
            emit_group_member_event(state, group_id, user_id, created_at, 0, true).await;
        }
        let delete_at = Utc::now().timestamp_millis();
        for (user_id, created_at) in removed_users {
            emit_group_member_event(state, group_id, user_id, created_at, delete_at, false).await;
        }

        let dm_acl_enabled =
            parse_bool_attribute(&kc_group.attributes, "rustchat_dm_acl").unwrap_or(false);
        sync_group_dm_acl_flag(state, group_id, dm_acl_enabled).await?;

        let (syncables_upserted, syncables_removed, changed_syncables) =
            sync_group_syncables_from_attributes(state, group_id, kc_group).await?;
        report.syncables_upserted += syncables_upserted;
        report.syncables_removed += syncables_removed;

        reconcile_group_syncables(state, group_id).await?;
        for (kind, syncable_id, removed_link) in changed_syncables {
            if removed_link {
                cleanup_unlinked_syncable(state, group_id, kind, syncable_id).await?;
                emit_group_syncable_event(state, group_id, kind, syncable_id, false).await;
            } else {
                reconcile_group_syncable(state, group_id, kind, syncable_id).await?;
                emit_group_syncable_event(state, group_id, kind, syncable_id, true).await;
            }
        }
    }

    deactivate_removed_keycloak_groups(state, &desired_remote_ids).await?;

    info!(
        groups = report.groups_processed,
        upserted = report.groups_upserted,
        members_added = report.group_members_added,
        members_removed = report.group_members_removed,
        syncables_upserted = report.syncables_upserted,
        syncables_removed = report.syncables_removed,
        "Keycloak full sync completed"
    );

    Ok(report)
}

pub async fn resync_user(state: &AppState, user_id: Uuid) -> ApiResult<KeycloakSyncReport> {
    validate_sync_config(state)?;
    let provider_key = &state.config.keycloak_sync.provider_key;

    let (provider, external_id): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT auth_provider, auth_provider_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if provider.as_deref() != Some(provider_key.as_str()) || external_id.is_none() {
        return Ok(KeycloakSyncReport {
            groups_processed: 0,
            groups_upserted: 0,
            group_members_added: 0,
            group_members_removed: 0,
            syncables_upserted: 0,
            syncables_removed: 0,
            users_mapped: 0,
        });
    }
    let external_id = external_id.unwrap_or_default();

    let token = fetch_admin_token(state).await?;
    let mut groups = fetch_groups(state, &token).await?;
    flatten_groups(&mut groups);

    let mut report = KeycloakSyncReport {
        groups_processed: 0,
        groups_upserted: 0,
        group_members_added: 0,
        group_members_removed: 0,
        syncables_upserted: 0,
        syncables_removed: 0,
        users_mapped: 1,
    };

    for kc_group in &groups {
        let group_id = upsert_group(state, kc_group).await?;
        report.groups_upserted += 1;
        report.groups_processed += 1;
        let dm_acl_enabled =
            parse_bool_attribute(&kc_group.attributes, "rustchat_dm_acl").unwrap_or(false);
        sync_group_dm_acl_flag(state, group_id, dm_acl_enabled).await?;

        let members = fetch_group_members(state, &token, &kc_group.id).await?;
        let is_member = members.iter().any(|m| m.id == external_id);
        let currently_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

        if is_member && !currently_member {
            let created_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
                r#"
                INSERT INTO group_members (group_id, user_id)
                VALUES ($1, $2)
                ON CONFLICT (group_id, user_id) DO NOTHING
                RETURNING created_at
                "#,
            )
            .bind(group_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
            report.group_members_added += 1;
            emit_group_member_event(
                state,
                group_id,
                user_id,
                created_at.unwrap_or_else(Utc::now),
                0,
                true,
            )
            .await;
            reconcile_group_syncables(state, group_id).await?;
        } else if !is_member && currently_member {
            let deleted_created_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
                "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2 RETURNING created_at",
            )
                .bind(group_id)
                .bind(user_id)
                .fetch_optional(&state.db)
                .await?;
            report.group_members_removed += 1;
            emit_group_member_event(
                state,
                group_id,
                user_id,
                deleted_created_at.unwrap_or_else(Utc::now),
                Utc::now().timestamp_millis(),
                false,
            )
            .await;
            reconcile_group_syncables(state, group_id).await?;
        }
    }

    Ok(report)
}

fn validate_sync_config(state: &AppState) -> ApiResult<()> {
    let cfg = &state.config.keycloak_sync;
    if cfg.admin_base_url.trim().is_empty()
        || cfg.realm.trim().is_empty()
        || cfg.client_id.trim().is_empty()
        || cfg.client_secret.trim().is_empty()
    {
        return Err(AppError::Config(
            "Keycloak sync config is incomplete".to_string(),
        ));
    }
    Ok(())
}

async fn fetch_admin_token(state: &AppState) -> ApiResult<String> {
    let cfg = &state.config.keycloak_sync;
    let base = cfg.admin_base_url.trim_end_matches('/');
    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        base, cfg.realm
    );

    let response = state
        .http_client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Keycloak token request failed: {}", e)))?;

    if response.status() != StatusCode::OK {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::ExternalService(format!(
            "Keycloak token request failed: {} {}",
            status, body
        )));
    }

    let parsed: KeycloakTokenResponse = response.json().await.map_err(|e| {
        AppError::ExternalService(format!("Invalid Keycloak token response: {}", e))
    })?;

    Ok(parsed.access_token)
}

async fn fetch_groups(state: &AppState, token: &str) -> ApiResult<Vec<KeycloakGroup>> {
    let cfg = &state.config.keycloak_sync;
    let base = cfg.admin_base_url.trim_end_matches('/');
    let groups_url = format!("{}/admin/realms/{}/groups", base, cfg.realm);

    let response = state
        .http_client
        .get(groups_url)
        .query(&[
            ("briefRepresentation", "false"),
            ("max", "10000"),
            ("first", "0"),
        ])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| AppError::ExternalService(format!("Keycloak groups request failed: {}", e)))?;

    if response.status() != StatusCode::OK {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::ExternalService(format!(
            "Keycloak groups request failed: {} {}",
            status, body
        )));
    }

    response
        .json::<Vec<KeycloakGroup>>()
        .await
        .map_err(|e| AppError::ExternalService(format!("Invalid Keycloak groups response: {}", e)))
}

async fn fetch_group_members(
    state: &AppState,
    token: &str,
    group_id: &str,
) -> ApiResult<Vec<KeycloakUser>> {
    let cfg = &state.config.keycloak_sync;
    let base = cfg.admin_base_url.trim_end_matches('/');

    let mut first = 0usize;
    let mut all = Vec::new();
    loop {
        let url = format!(
            "{}/admin/realms/{}/groups/{}/members",
            base, cfg.realm, group_id
        );
        let response = state
            .http_client
            .get(&url)
            .query(&[("first", first), ("max", 500)])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Keycloak members request failed: {}", e))
            })?;

        if response.status() != StatusCode::OK {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Keycloak members request failed: {} {}",
                status, body
            )));
        }

        let page = response.json::<Vec<KeycloakUser>>().await.map_err(|e| {
            AppError::ExternalService(format!("Invalid Keycloak members response: {}", e))
        })?;

        if page.is_empty() {
            break;
        }
        first += page.len();
        all.extend(page);
    }

    Ok(all)
}

fn flatten_groups(groups: &mut Vec<KeycloakGroup>) {
    fn walk(group: KeycloakGroup, out: &mut Vec<KeycloakGroup>) {
        let mut base = group.clone();
        let sub = std::mem::take(&mut base.sub_groups);
        out.push(base);
        for child in sub {
            walk(child, out);
        }
    }

    let source = std::mem::take(groups);
    let mut flat = Vec::new();
    for group in source {
        walk(group, &mut flat);
    }
    *groups = flat;
}

async fn upsert_group(state: &AppState, group: &KeycloakGroup) -> ApiResult<Uuid> {
    let display_name = if group.name.trim().is_empty() {
        group.path.clone().unwrap_or_else(|| group.id.clone())
    } else {
        group.name.clone()
    };
    let description = group.path.clone().unwrap_or_default();

    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO groups (name, display_name, description, source, remote_id, allow_reference, deleted_at)
        VALUES (NULL, $1, $2, $3, $4, TRUE, NULL)
        ON CONFLICT (source, remote_id) DO UPDATE SET
            display_name = EXCLUDED.display_name,
            description = EXCLUDED.description,
            allow_reference = TRUE,
            deleted_at = NULL,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(display_name)
    .bind(description)
    .bind(GROUP_SOURCE_KEYCLOAK)
    .bind(&group.id)
    .fetch_one(&state.db)
    .await?;

    Ok(row.0)
}

async fn resolve_rustchat_users_for_keycloak_members(
    state: &AppState,
    provider_key: &str,
    members: &[KeycloakUser],
) -> ApiResult<HashSet<Uuid>> {
    let external_ids: Vec<String> = members.iter().map(|m| m.id.clone()).collect();
    if external_ids.is_empty() {
        return Ok(HashSet::new());
    }

    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM users WHERE deleted_at IS NULL AND auth_provider = $1 AND auth_provider_id = ANY($2)",
    )
    .bind(provider_key)
    .bind(&external_ids)
    .fetch_all(&state.db)
    .await?;

    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn sync_group_members(
    state: &AppState,
    group_id: Uuid,
    desired_user_ids: &HashSet<Uuid>,
) -> ApiResult<(
    Vec<(Uuid, chrono::DateTime<chrono::Utc>)>,
    Vec<(Uuid, chrono::DateTime<chrono::Utc>)>,
)> {
    let current_user_ids: HashSet<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&state.db)
            .await?
            .into_iter()
            .collect();

    let mut added_users = Vec::new();
    let mut removed_users = Vec::new();

    for user_id in desired_user_ids.difference(&current_user_ids) {
        let created_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            r#"
            INSERT INTO group_members (group_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (group_id, user_id) DO NOTHING
            RETURNING created_at
            "#,
        )
        .bind(group_id)
        .bind(*user_id)
        .fetch_optional(&state.db)
        .await?;
        if let Some(created_at) = created_at {
            added_users.push((*user_id, created_at));
        }
    }

    for user_id in current_user_ids.difference(desired_user_ids) {
        let deleted_created_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2 RETURNING created_at",
        )
        .bind(group_id)
        .bind(*user_id)
        .fetch_optional(&state.db)
        .await?;
        if let Some(created_at) = deleted_created_at {
            removed_users.push((*user_id, created_at));
        }
    }

    Ok((added_users, removed_users))
}

async fn sync_group_syncables_from_attributes(
    state: &AppState,
    group_id: Uuid,
    group: &KeycloakGroup,
) -> ApiResult<(usize, usize, Vec<(SyncableKind, Uuid, bool)>)> {
    let auto_add = parse_bool_attribute(&group.attributes, "rustchat_auto_add").unwrap_or(true);
    let scheme_admin =
        parse_bool_attribute(&group.attributes, "rustchat_scheme_admin").unwrap_or(false);

    let desired_team_ids = parse_uuid_attribute_list(&group.attributes, "rustchat_team_ids");
    let desired_channel_ids = parse_uuid_attribute_list(&group.attributes, "rustchat_channel_ids");

    let current: Vec<GroupSyncableRow> = sqlx::query_as(
        r#"
        SELECT syncable_type, syncable_id, auto_add, scheme_admin
        FROM group_syncables
        WHERE group_id = $1
          AND delete_at IS NULL
        "#,
    )
    .bind(group_id)
    .fetch_all(&state.db)
    .await?;

    let mut changed = Vec::new();
    let mut upserted = 0usize;
    let mut removed = 0usize;

    let mut current_map: HashMap<(String, Uuid), GroupSyncableRow> = HashMap::new();
    for row in current {
        current_map.insert((row.syncable_type.clone(), row.syncable_id), row);
    }

    for team_id in &desired_team_ids {
        let key = ("team".to_string(), *team_id);
        let existing = current_map.remove(&key);
        if existing
            .as_ref()
            .map(|row| row.auto_add == auto_add && row.scheme_admin == scheme_admin)
            .unwrap_or(false)
        {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO group_syncables (group_id, syncable_type, syncable_id, auto_add, scheme_admin, delete_at)
            VALUES ($1, 'team', $2, $3, $4, NULL)
            ON CONFLICT (group_id, syncable_type, syncable_id)
            DO UPDATE SET auto_add = EXCLUDED.auto_add, scheme_admin = EXCLUDED.scheme_admin, delete_at = NULL, update_at = NOW()
            "#,
        )
        .bind(group_id)
        .bind(team_id)
        .bind(auto_add)
        .bind(scheme_admin)
        .execute(&state.db)
        .await?;
        upserted += 1;
        changed.push((SyncableKind::Team, *team_id, false));
    }

    for channel_id in &desired_channel_ids {
        let key = ("channel".to_string(), *channel_id);
        let existing = current_map.remove(&key);
        if existing
            .as_ref()
            .map(|row| row.auto_add == auto_add && row.scheme_admin == scheme_admin)
            .unwrap_or(false)
        {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO group_syncables (group_id, syncable_type, syncable_id, auto_add, scheme_admin, delete_at)
            VALUES ($1, 'channel', $2, $3, $4, NULL)
            ON CONFLICT (group_id, syncable_type, syncable_id)
            DO UPDATE SET auto_add = EXCLUDED.auto_add, scheme_admin = EXCLUDED.scheme_admin, delete_at = NULL, update_at = NOW()
            "#,
        )
        .bind(group_id)
        .bind(channel_id)
        .bind(auto_add)
        .bind(scheme_admin)
        .execute(&state.db)
        .await?;
        upserted += 1;
        changed.push((SyncableKind::Channel, *channel_id, false));
    }

    for ((_syncable_type, _syncable_id), row) in current_map {
        sqlx::query(
            "DELETE FROM group_syncables WHERE group_id = $1 AND syncable_type = $2 AND syncable_id = $3",
        )
        .bind(group_id)
        .bind(&row.syncable_type)
        .bind(row.syncable_id)
        .execute(&state.db)
        .await?;

        removed += 1;
        let kind = if row.syncable_type == "team" {
            SyncableKind::Team
        } else {
            SyncableKind::Channel
        };
        changed.push((kind, row.syncable_id, true));
    }

    Ok((upserted, removed, changed))
}

fn parse_bool_attribute(attrs: &HashMap<String, Vec<String>>, key: &str) -> Option<bool> {
    let raw = attrs.get(key)?.first()?.trim().to_ascii_lowercase();
    match raw.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_uuid_attribute_list(attrs: &HashMap<String, Vec<String>>, key: &str) -> HashSet<Uuid> {
    let mut out = HashSet::new();
    let Some(values) = attrs.get(key) else {
        return out;
    };
    for value in values {
        for piece in value.split(',') {
            let trimmed = piece.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(id) = parse_mm_or_uuid(trimmed) {
                out.insert(id);
            }
        }
    }
    out
}

async fn sync_group_dm_acl_flag(state: &AppState, group_id: Uuid, enabled: bool) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO group_dm_acl_flags (group_id, enabled, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (group_id)
        DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = NOW()
        "#,
    )
    .bind(group_id)
    .bind(enabled)
    .execute(&state.db)
    .await?;
    Ok(())
}

fn group_member_payload(
    group_id: Uuid,
    user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    delete_at: i64,
) -> serde_json::Value {
    serde_json::json!({
        "group_id": encode_mm_id(group_id),
        "user_id": encode_mm_id(user_id),
        "create_at": created_at.timestamp_millis(),
        "delete_at": delete_at,
    })
}

async fn emit_group_member_event(
    state: &AppState,
    group_id: Uuid,
    user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    delete_at: i64,
    is_add: bool,
) {
    let event_type = if is_add {
        crate::realtime::EventType::GroupMemberAdd
    } else {
        crate::realtime::EventType::GroupMemberDeleted
    };
    let payload = group_member_payload(group_id, user_id, created_at, delete_at);
    let payload_text = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    let event = crate::realtime::WsEnvelope::event(
        event_type,
        serde_json::json!({ "group_member": payload_text }),
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
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
    associated: bool,
) {
    let (event_type, broadcast) = match (kind, associated) {
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

    let event = crate::realtime::WsEnvelope::event(
        event_type,
        serde_json::json!({ "group_id": group_id }),
        None,
    )
    .with_broadcast(broadcast);
    state.ws_hub.broadcast(event).await;
}

async fn deactivate_removed_keycloak_groups(
    state: &AppState,
    desired_remote_ids: &HashSet<String>,
) -> ApiResult<()> {
    let existing: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, remote_id FROM groups WHERE source = $1 AND deleted_at IS NULL AND remote_id IS NOT NULL",
    )
    .bind(GROUP_SOURCE_KEYCLOAK)
    .fetch_all(&state.db)
    .await?;

    for (group_id, remote_id) in existing {
        if desired_remote_ids.contains(&remote_id) {
            continue;
        }

        let syncables: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT syncable_type, syncable_id FROM group_syncables WHERE group_id = $1",
        )
        .bind(group_id)
        .fetch_all(&state.db)
        .await?;
        for (syncable_type, syncable_id) in syncables {
            let kind = if syncable_type == "team" {
                SyncableKind::Team
            } else {
                SyncableKind::Channel
            };
            cleanup_unlinked_syncable(state, group_id, kind, syncable_id).await?;
            emit_group_syncable_event(state, group_id, kind, syncable_id, false).await;
        }

        let member_rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as("SELECT user_id, created_at FROM group_members WHERE group_id = $1")
                .bind(group_id)
                .fetch_all(&state.db)
                .await?;
        let delete_at = Utc::now().timestamp_millis();
        for (user_id, created_at) in member_rows {
            emit_group_member_event(state, group_id, user_id, created_at, delete_at, false).await;
        }

        sqlx::query("DELETE FROM group_syncables WHERE group_id = $1")
            .bind(group_id)
            .execute(&state.db)
            .await?;
        sqlx::query("DELETE FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .execute(&state.db)
            .await?;
        sqlx::query(
            "INSERT INTO group_dm_acl_flags (group_id, enabled, updated_at) VALUES ($1, FALSE, NOW()) ON CONFLICT (group_id) DO UPDATE SET enabled = FALSE, updated_at = NOW()",
        )
        .bind(group_id)
        .execute(&state.db)
        .await?;
        sqlx::query("UPDATE groups SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(group_id)
            .execute(&state.db)
            .await?;
    }

    Ok(())
}

async fn reconcile_group_syncables(state: &AppState, group_id: Uuid) -> ApiResult<()> {
    let rows: Vec<(String, Uuid)> = sqlx::query_as(
        "SELECT syncable_type, syncable_id FROM group_syncables WHERE group_id = $1 AND delete_at IS NULL",
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

async fn ensure_membership(
    state: &AppState,
    target_type: &str,
    target_id: Uuid,
    user_id: Uuid,
    scheme_admin: bool,
) -> ApiResult<bool> {
    let role = if scheme_admin { "admin" } else { "member" };

    let affected = if target_type == "team" {
        sqlx::query(
            "INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (team_id, user_id) DO NOTHING",
        )
        .bind(target_id)
        .bind(user_id)
        .bind(role)
        .execute(&state.db)
        .await?
        .rows_affected()
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
    };

    Ok(affected > 0)
}

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
            FROM group_syncable_memberships
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

async fn reconcile_group_syncable(
    state: &AppState,
    group_id: Uuid,
    kind: SyncableKind,
    syncable_id: Uuid,
) -> ApiResult<()> {
    let syncable: Option<GroupSyncableRow> = sqlx::query_as(
        r#"
        SELECT group_id, syncable_type, syncable_id, auto_add, scheme_admin
        FROM group_syncables
        WHERE group_id = $1
          AND syncable_type = $2
          AND syncable_id = $3
          AND delete_at IS NULL
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

    let tracked_set: HashSet<TrackedMembershipRow> = existing_tracked.iter().cloned().collect();

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
            sqlx::query(
                r#"
                INSERT INTO group_syncable_memberships
                    (group_id, syncable_type, syncable_id, target_type, target_id, user_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(group_id)
            .bind(kind.as_db_str())
            .bind(syncable_id)
            .bind(&desired_membership.target_type)
            .bind(desired_membership.target_id)
            .bind(desired_membership.user_id)
            .execute(&state.db)
            .await?;
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

