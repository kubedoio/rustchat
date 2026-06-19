//! Mattermost mobile/web compatibility API stubs
//!
//! Consolidated placeholder and stub endpoints to satisfy protocol requirements
//! for features not implemented in this build.

use crate::api::v4::extractors::MmAuthUser;
use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde_json::{json, Value};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        // AI stubs
        .route("/ai/agents", get(get_ai_agents))
        .route("/ai/services", get(get_ai_services))
        .route("/agents", get(get_agents))
        .route("/agents/status", get(get_agents_status))
        .route("/llmservices", get(get_llm_services))
        // Brand stubs
        .route(
            "/brand/image",
            get(get_brand_image)
                .post(upload_brand_image)
                .delete(delete_brand_image),
        )
        // Cloud stubs
        .route("/cloud/limits", get(get_cloud_limits))
        .route("/cloud/products", get(get_cloud_products))
        .route("/cloud/payment", post(create_cloud_payment_info))
        .route("/cloud/payment/confirm", post(confirm_cloud_payment))
        .route("/cloud/customer", get(get_cloud_customer))
        .route(
            "/cloud/customer/address",
            put(update_cloud_customer_address),
        )
        .route("/cloud/subscription", get(get_cloud_subscription))
        .route("/cloud/installation", get(get_cloud_installation))
        .route(
            "/cloud/subscription/invoices",
            get(get_cloud_subscription_invoices),
        )
        .route("/cloud/webhook", post(cloud_webhook))
        .route(
            "/cloud/preview/modal_data",
            get(get_cloud_preview_modal_data),
        )
        // Cluster stubs
        .route("/cluster/status", get(get_cluster_status))
        .route("/remotecluster", get(get_remote_clusters))
        .route("/remotecluster/{remote_id}", get(get_remote_cluster))
        .route(
            "/remotecluster/{remote_id}/generate_invite",
            post(generate_remote_cluster_invite),
        )
        .route(
            "/remotecluster/accept_invite",
            post(accept_remote_cluster_invite),
        )
        .route(
            "/remotecluster/{remote_id}/sharedchannelremotes",
            get(get_remote_cluster_shared_channels),
        )
        .route(
            "/remotecluster/{remote_id}/channels/{channel_id}/invite",
            post(invite_remote_cluster_to_channel),
        )
        .route(
            "/remotecluster/{remote_id}/channels/{channel_id}/uninvite",
            post(uninvite_remote_cluster_from_channel),
        )
        // Compliance stubs
        .route(
            "/compliance/reports",
            get(get_compliance_reports).post(create_compliance_report),
        )
        .route(
            "/compliance/reports/{report_id}",
            get(get_compliance_report),
        )
        .route(
            "/compliance/reports/{report_id}/download",
            get(download_compliance_report),
        )
        // Dialogs stubs
        .route("/actions/dialogs/open", post(open_dialog))
        .route("/actions/dialogs/submit", post(submit_dialog))
        .route("/actions/dialogs/lookup", post(lookup_dialog))
        // IP Filtering stubs
        .route("/ip_filtering", get(get_ip_filters))
        .route("/ip_filtering/my_ip", get(get_my_ip))
        // LDAP stubs
        .route("/ldap/sync", post(sync_ldap))
        .route("/ldap/test", post(test_ldap))
        .route("/ldap/test_connection", post(test_ldap_connection))
        .route("/ldap/test_diagnostics", post(test_ldap_diagnostics))
        .route("/ldap/groups", get(get_ldap_groups))
        .route("/ldap/groups/{remote_id}/link", post(link_ldap_group))
        .route("/ldap/migrateid", post(ldap_migrate_id))
        .route(
            "/ldap/certificate/public",
            post(add_ldap_public_certificate).delete(remove_ldap_public_certificate),
        )
        .route(
            "/ldap/certificate/private",
            post(add_ldap_private_certificate).delete(remove_ldap_private_certificate),
        )
        .route(
            "/ldap/users/{user_id}/group_sync_memberships",
            post(sync_ldap_user_group_sync_memberships),
        )
        // Recaps stubs
        .route("/recaps", get(get_recaps))
        .route("/recaps/{recap_id}", get(get_recap))
        .route("/recaps/{recap_id}/read", post(mark_recap_read))
        .route("/recaps/{recap_id}/regenerate", post(regenerate_recap))
        // SAML stubs
        .route("/saml/metadata", get(get_saml_metadata))
        .route("/saml/metadatafromidp", post(get_saml_metadata_from_idp))
        .route(
            "/saml/certificate/idp",
            post(add_saml_idp_certificate).delete(remove_saml_idp_certificate),
        )
        .route(
            "/saml/certificate/public",
            post(add_saml_public_certificate).delete(remove_saml_public_certificate),
        )
        .route(
            "/saml/certificate/private",
            post(add_saml_private_certificate).delete(remove_saml_private_certificate),
        )
        .route("/saml/certificate/status", get(get_saml_certificate_status))
        .route("/saml/reset_auth_data", post(reset_saml_auth_data))
        // Shared Channels stubs
        .route("/sharedchannels/{team_id}", get(get_shared_channels))
        .route(
            "/sharedchannels/remote_info/{remote_id}",
            get(get_remote_cluster_info),
        )
        .route(
            "/sharedchannels/{channel_id}/remotes",
            get(get_channel_remotes),
        )
        .route(
            "/sharedchannels/users/{user_id}/can_dm/{other_user_id}",
            get(can_dm_user),
        )
        // Data Retention stubs
        .route(
            "/data_retention/policy",
            get(get_global_data_retention_policy),
        )
        .route(
            "/data_retention/policies_count",
            get(get_data_retention_policies_count),
        )
        .route("/data_retention/policies", get(get_data_retention_policies))
        .route(
            "/data_retention/policies/{policy_id}",
            get(get_data_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/teams",
            get(get_teams_for_retention_policy).post(add_teams_to_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/teams/search",
            post(search_teams_for_retention_policy).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::rate_limit::search_ip_rate_limit,
            )),
        )
        .route(
            "/data_retention/policies/{policy_id}/channels",
            get(get_channels_for_retention_policy).post(add_channels_to_retention_policy),
        )
        .route(
            "/data_retention/policies/{policy_id}/channels/search",
            post(search_channels_for_retention_policy).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::rate_limit::search_ip_rate_limit,
            )),
        )
        .route(
            "/users/{user_id}/data_retention/team_policies",
            get(get_user_team_policies),
        )
        .route(
            "/users/{user_id}/data_retention/channel_policies",
            get(get_user_channel_policies),
        )
        .route("/nps", post(submit_nps))
}

// ==========================================
// Helper functions
// ==========================================

fn stub_not_implemented(id: &str, message: &str, detailed: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "id": id,
            "message": message,
            "detailed_error": detailed,
            "request_id": "",
            "status_code": 501
        })),
    )
}

// ==========================================
// Handlers
// ==========================================

// AI
async fn get_ai_agents(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_ai_services(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_agents(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_agents_status(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_llm_services(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}

// Brand
async fn get_brand_image(_s: State<AppState>) -> ApiResult<axum::response::Response> {
    Ok((StatusCode::NOT_FOUND, "No brand image").into_response())
}
async fn upload_brand_image(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    Ok((StatusCode::CREATED, Json(json!({"status": "OK"}))))
}
async fn delete_brand_image(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}

// Cloud
async fn get_cloud_limits(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_cloud_products(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn create_cloud_payment_info(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn confirm_cloud_payment(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn get_cloud_customer(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn update_cloud_customer_address(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_cloud_subscription(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_cloud_installation(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_cloud_subscription_invoices(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn cloud_webhook(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn get_cloud_preview_modal_data(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}

// Cluster
#[derive(serde::Serialize)]
struct ClusterInfo {
    id: String,
    version: String,
    schema_version: String,
    config_hash: String,
    ipaddress: String,
    hostname: String,
}
async fn get_cluster_status(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Vec<ClusterInfo>>> {
    Ok(Json(vec![ClusterInfo {
        id: "rustchat-node-1".to_string(),
        version: "0.0.1".to_string(),
        schema_version: "1.0.0".to_string(),
        config_hash: "mock-config-hash".to_string(),
        ipaddress: "127.0.0.1".to_string(),
        hostname: "localhost".to_string(),
    }]))
}
async fn get_remote_clusters(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_remote_cluster(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn generate_remote_cluster_invite(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"invite_token": ""})))
}
async fn accept_remote_cluster_invite(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn get_remote_cluster_shared_channels(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn invite_remote_cluster_to_channel(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path((_r, _c)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn uninvite_remote_cluster_from_channel(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path((_r, _c)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}

// Compliance
async fn get_compliance_reports(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn create_compliance_report(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"id": "stub_report_id", "status": "pending"})))
}
async fn get_compliance_report(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn download_compliance_report(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}

// Dialogs
async fn open_dialog(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<(StatusCode, Json<Value>)> {
    Ok(stub_not_implemented(
        "api.actions.dialogs.open.not_implemented.app_error",
        "Interactive dialogs are not implemented.",
        "POST /api/v4/actions/dialogs/open is not supported in this server.",
    ))
}
async fn submit_dialog(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    Ok(stub_not_implemented(
        "api.actions.dialogs.submit.not_implemented.app_error",
        "Interactive dialogs are not implemented.",
        "POST /api/v4/actions/dialogs/submit is not supported in this server.",
    ))
}
async fn lookup_dialog(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    Ok(stub_not_implemented(
        "api.actions.dialogs.lookup.not_implemented.app_error",
        "Interactive dialogs are not implemented.",
        "POST /api/v4/actions/dialogs/lookup is not supported in this server.",
    ))
}

// IP Filtering
async fn get_ip_filters(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_my_ip(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"ip": "127.0.0.1"})))
}

// LDAP
async fn sync_ldap(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<(StatusCode, Json<Value>)> {
    Ok(stub_not_implemented(
        "api.ldap.not_implemented",
        "LDAP feature is not implemented.",
        "LDAP endpoints are available but backend LDAP operations are not implemented.",
    ))
}
async fn test_ldap(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn test_ldap_connection(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn test_ldap_diagnostics(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn get_ldap_groups(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn link_ldap_group(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn ldap_migrate_id(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn add_ldap_public_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn remove_ldap_public_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn add_ldap_private_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn remove_ldap_private_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}
async fn sync_ldap_user_group_sync_memberships(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    sync_ldap(_s, _a).await
}

// Recaps
async fn get_recaps(_s: State<AppState>, _a: MmAuthUser) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_recap(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn mark_recap_read(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn regenerate_recap(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}

// SAML
async fn get_saml_metadata(_s: State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        "<?xml version=\"1.0\"?><EntityDescriptor xmlns=\"urn:oasis:names:tc:SAML:2.0:metadata\"><Error>SAML not configured</Error></EntityDescriptor>",
    ))
}
fn saml_not_implemented() -> ApiResult<(StatusCode, Json<Value>)> {
    Ok(stub_not_implemented(
        "api.saml.not_implemented",
        "SAML feature is not implemented.",
        "SAML endpoints are available but backend SAML operations are not implemented.",
    ))
}
async fn get_saml_metadata_from_idp(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn add_saml_idp_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn remove_saml_idp_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn add_saml_public_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn remove_saml_public_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn add_saml_private_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn remove_saml_private_certificate(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}
async fn get_saml_certificate_status(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({
        "idp_certificate_file": false,
        "public_certificate_file": false,
        "private_key_file": false
    })))
}
async fn reset_saml_auth_data(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_b): Json<Value>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    saml_not_implemented()
}

// Shared Channels
async fn get_shared_channels(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_remote_cluster_info(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_channel_remotes(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn can_dm_user(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path((_u, _o)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(true)))
}

// Data Retention
async fn get_global_data_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_data_retention_policies_count(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"total_count": 0})))
}
async fn get_data_retention_policies(
    _s: State<AppState>,
    _a: MmAuthUser,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}
async fn get_data_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({})))
}
async fn get_teams_for_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn add_teams_to_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn search_teams_for_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_channels_for_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn add_channels_to_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
async fn search_channels_for_retention_policy(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
    Json(_b): Json<Value>,
) -> ApiResult<Json<Vec<Value>>> {
    Ok(Json(vec![]))
}
async fn get_user_team_policies(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}
async fn get_user_channel_policies(
    _s: State<AppState>,
    _a: MmAuthUser,
    Path(_id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"policies": [], "total_count": 0})))
}
async fn submit_nps(
    _s: State<AppState>,
    _a: MmAuthUser,
    Json(_f): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"status": "OK"})))
}
