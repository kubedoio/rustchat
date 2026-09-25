//! Admin API for the optional Buzz integration.
//!
//! Minimal surface, admin-authenticated throughout (`require_admin`):
//!
//! * connections: list / create / get / update / delete / rotate key / test
//! * channel mappings: list / upsert / delete
//! * deliveries: list / retry (dead-lettered rows only)
//!
//! Security invariants:
//!
//! * The private signing key is write-only: accepted on create/rotate,
//!   encrypted at rest, never included in any response.
//! * Relay URLs must pass the SSRF-safe URL policy; HTTPS is required in
//!   production.
//! * The integration must be enabled in configuration; otherwise every
//!   endpoint fails fast with an explicit error instead of half-working.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::admin::require_admin;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::integrations::buzz::connector::BuzzConnectorProvider;
use crate::integrations::buzz::http::HttpBuzzConnectorProvider;
use crate::integrations::buzz::repository::{
    BuzzChannelMappingRecord, BuzzConnectionRecord, BuzzRepository, NewBuzzConnection,
};
use crate::integrations::outbox::{OutboxRepository, OutboxStatus};
use crate::services::audit::{audit, AuditAction};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    let connections = Router::new()
        .route(
            "/connections",
            get(list_connections).post(create_connection),
        )
        .route(
            "/connections/{id}",
            get(get_connection)
                .patch(update_connection)
                .delete(delete_connection),
        )
        .route("/connections/{id}/key", post(rotate_connection_key))
        .route("/connections/{id}/test", post(test_connection))
        .route(
            "/connections/{id}/mappings",
            get(list_mappings).put(upsert_mapping),
        )
        .route(
            "/connections/{id}/mappings/{mapping_id}",
            delete(delete_mapping),
        )
        .route("/connections/{id}/deliveries", get(list_deliveries))
        .route("/deliveries/{outbox_id}/retry", post(retry_delivery));

    Router::new().nest("/admin/integrations/buzz", connections)
}

/// Guard: the integration must be enabled in configuration.
fn require_integration_enabled(state: &AppState) -> ApiResult<()> {
    if state.config.integrations.buzz.enabled {
        Ok(())
    } else {
        Err(AppError::Validation(
            "the Buzz integration is disabled; set RUSTCHAT_INTEGRATIONS_BUZZ_ENABLED=true \
             and restart to use this API"
                .to_string(),
        ))
    }
}

/// Response view of a connection — never includes key material.
#[derive(serde::Serialize)]
struct ConnectionResponse {
    id: Uuid,
    name: String,
    relay_url: String,
    bridge_pubkey: String,
    enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<BuzzConnectionRecord> for ConnectionResponse {
    fn from(c: BuzzConnectionRecord) -> Self {
        Self {
            id: c.id,
            name: c.name,
            relay_url: c.relay_url,
            bridge_pubkey: c.bridge_pubkey,
            enabled: c.enabled,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(serde::Serialize)]
struct MappingResponse {
    id: Uuid,
    connection_id: Uuid,
    rustchat_channel_id: Uuid,
    buzz_channel_id: Uuid,
    outbound_enabled: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<BuzzChannelMappingRecord> for MappingResponse {
    fn from(m: BuzzChannelMappingRecord) -> Self {
        Self {
            id: m.id,
            connection_id: m.connection_id,
            rustchat_channel_id: m.rustchat_channel_id,
            buzz_channel_id: m.buzz_channel_id,
            outbound_enabled: m.outbound_enabled,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct CreateConnectionRequest {
    name: String,
    relay_url: String,
    /// Bridge signing key (hex or nsec). Write-only.
    private_key: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct UpdateConnectionRequest {
    name: Option<String>,
    relay_url: Option<String>,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct RotateKeyRequest {
    /// New bridge signing key (hex or nsec). Write-only.
    private_key: String,
}

#[derive(Deserialize)]
struct UpsertMappingRequest {
    rustchat_channel_id: Uuid,
    buzz_channel_id: Uuid,
    #[serde(default = "default_true")]
    outbound_enabled: bool,
}

#[derive(Deserialize)]
struct ListDeliveriesQuery {
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn audit_buzz(
    state: &AppState,
    actor: Uuid,
    action: AuditAction,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
) {
    // Best-effort: audit failures must not fail the admin operation.
    let db = state.db.clone();
    let _ = tokio::spawn(async move {
        let _ = audit(
            &db,
            Some(actor),
            action,
            "buzz_integration",
            target_id,
            metadata,
        )
        .await;
    })
    .await;
}

async fn list_connections(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<ConnectionResponse>>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    let connections = BuzzRepository::new(&state.db).list_connections().await?;
    Ok(Json(connections.into_iter().map(Into::into).collect()))
}

async fn create_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateConnectionRequest>,
) -> ApiResult<Json<ConnectionResponse>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    if state.config.is_production()
        && !input
            .relay_url
            .trim()
            .to_ascii_lowercase()
            .starts_with("https://")
    {
        return Err(AppError::Validation(
            "relay URL must use HTTPS in production".to_string(),
        ));
    }

    let connection = BuzzRepository::new(&state.db)
        .create_connection(
            NewBuzzConnection {
                name: input.name,
                relay_url: input.relay_url,
                private_key: input.private_key,
                enabled: input.enabled,
            },
            &state.config.encryption_key,
        )
        .await?;

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzConnectionCreate,
        Some(connection.id),
        serde_json::json!({ "name": connection.name, "relay_url": connection.relay_url }),
    )
    .await;

    Ok(Json(connection.into()))
}

async fn get_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ConnectionResponse>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    let connection = BuzzRepository::new(&state.db)
        .get_connection(id)
        .await?
        .ok_or_else(|| AppError::NotFound("connection not found".to_string()))?;
    Ok(Json(connection.into()))
}

async fn update_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateConnectionRequest>,
) -> ApiResult<Json<ConnectionResponse>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    if state.config.is_production() {
        if let Some(url) = &input.relay_url {
            if !url.trim().to_ascii_lowercase().starts_with("https://") {
                return Err(AppError::Validation(
                    "relay URL must use HTTPS in production".to_string(),
                ));
            }
        }
    }

    let connection = BuzzRepository::new(&state.db)
        .update_connection(id, input.name, input.relay_url, input.enabled)
        .await?;

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzConnectionUpdate,
        Some(connection.id),
        serde_json::json!({ "name": connection.name, "enabled": connection.enabled }),
    )
    .await;

    Ok(Json(connection.into()))
}

async fn rotate_connection_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<RotateKeyRequest>,
) -> ApiResult<Json<ConnectionResponse>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    let connection = BuzzRepository::new(&state.db)
        .rotate_connection_key(id, &input.private_key, &state.config.encryption_key)
        .await?;

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzConnectionKeyRotate,
        Some(connection.id),
        serde_json::json!({ "bridge_pubkey": connection.bridge_pubkey }),
    )
    .await;

    Ok(Json(connection.into()))
}

async fn delete_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    BuzzRepository::new(&state.db).delete_connection(id).await?;

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzConnectionDelete,
        Some(id),
        serde_json::json!({}),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn test_connection(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    let connection = BuzzRepository::new(&state.db)
        .get_connection(id)
        .await?
        .ok_or_else(|| AppError::NotFound("connection not found".to_string()))?;
    let runtime = connection.into_runtime(&state.config.encryption_key)?;

    // Read-only authenticated round-trip against the relay.
    let provider = HttpBuzzConnectorProvider;
    let connector = provider
        .connector_for(&runtime)
        .await
        .map_err(AppError::ExternalService)?;
    connector.probe().await.map_err(AppError::ExternalService)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_mappings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(connection_id): Path<Uuid>,
) -> ApiResult<Json<Vec<MappingResponse>>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    let mappings = BuzzRepository::new(&state.db)
        .list_mappings(connection_id)
        .await?;
    Ok(Json(mappings.into_iter().map(Into::into).collect()))
}

async fn upsert_mapping(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(connection_id): Path<Uuid>,
    Json(input): Json<UpsertMappingRequest>,
) -> ApiResult<Json<MappingResponse>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    // Mapping authorization: the RustChat channel must exist (FK also
    // enforces this, but a clear error beats a raw constraint violation).
    let channel_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM channels WHERE id = $1")
            .bind(input.rustchat_channel_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    if channel_exists == 0 {
        return Err(AppError::Validation(
            "rustchat_channel_id does not reference an existing channel".to_string(),
        ));
    }

    let mapping = BuzzRepository::new(&state.db)
        .upsert_mapping(
            connection_id,
            input.rustchat_channel_id,
            input.buzz_channel_id,
            input.outbound_enabled,
        )
        .await?;

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzMappingUpsert,
        Some(mapping.id),
        serde_json::json!({
            "connection_id": mapping.connection_id,
            "rustchat_channel_id": mapping.rustchat_channel_id,
            "buzz_channel_id": mapping.buzz_channel_id,
            "outbound_enabled": mapping.outbound_enabled,
        }),
    )
    .await;

    Ok(Json(mapping.into()))
}

async fn delete_mapping(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((connection_id, mapping_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;
    let deleted = BuzzRepository::new(&state.db)
        .delete_mapping(connection_id, mapping_id)
        .await?;
    if !deleted {
        return Err(AppError::NotFound("mapping not found".to_string()));
    }

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzMappingDelete,
        Some(mapping_id),
        serde_json::json!({ "connection_id": connection_id }),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn list_deliveries(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(connection_id): Path<Uuid>,
    Query(query): Query<ListDeliveriesQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    let status = match query.status.as_deref() {
        None | Some("") => None,
        Some(raw) => Some(
            OutboxStatus::parse(raw)
                .ok_or_else(|| AppError::Validation(format!("unknown status: {raw}")))?,
        ),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let deliveries = OutboxRepository::new(&state.db)
        .list_for_connection(connection_id, status.as_ref(), limit, offset)
        .await?;

    Ok(Json(serde_json::json!({
        "deliveries": deliveries
            .into_iter()
            .map(|d| serde_json::json!({
                "id": d.id,
                "event_type": d.event_type,
                "status": d.status,
                "attempts": d.attempts,
                "next_attempt_at": d.next_attempt_at,
                "last_error": d.last_error,
                "remote_event_id": d.remote_event_id,
                "created_at": d.created_at,
                "updated_at": d.updated_at,
            }))
            .collect::<Vec<_>>(),
    })))
}

async fn retry_delivery(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(outbox_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    require_admin(&auth)?;
    require_integration_enabled(&state)?;

    let requeued = OutboxRepository::new(&state.db)
        .requeue_dead_letter("buzz", outbox_id)
        .await?;
    if !requeued {
        return Err(AppError::NotFound(
            "delivery not found or not in dead_letter state".to_string(),
        ));
    }

    audit_buzz(
        &state,
        auth.user_id,
        AuditAction::BuzzDeliveryRetry,
        Some(outbox_id),
        serde_json::json!({}),
    )
    .await;

    Ok(Json(serde_json::json!({ "requeued": true })))
}
