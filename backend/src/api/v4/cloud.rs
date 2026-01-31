use crate::api::AppState;
use crate::error::ApiResult;
use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cloud/limits", get(get_cloud_limits))
        .route("/cloud/products", get(get_cloud_products))
        .route("/cloud/payment", post(create_cloud_payment_info))
        .route("/cloud/payment/confirm", post(confirm_cloud_payment))
        .route("/cloud/customer", get(get_cloud_customer))
        .route("/cloud/customer/address", put(update_cloud_customer_address))
        .route("/cloud/subscription", get(get_cloud_subscription))
        .route("/cloud/installation", get(get_cloud_installation))
        .route("/cloud/subscription/invoices", get(get_cloud_subscription_invoices))
        .route("/cloud/webhook", post(cloud_webhook))
        .route("/cloud/preview/modal_data", get(get_cloud_preview_modal_data))
}

/// GET /api/v4/cloud/limits
async fn get_cloud_limits(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/cloud/products
async fn get_cloud_products(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/cloud/payment
async fn create_cloud_payment_info(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// POST /api/v4/cloud/payment/confirm
async fn confirm_cloud_payment(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/cloud/customer
async fn get_cloud_customer(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// PUT /api/v4/cloud/customer/address
async fn update_cloud_customer_address(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/cloud/subscription
async fn get_cloud_subscription(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/cloud/installation
async fn get_cloud_installation(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}

/// GET /api/v4/cloud/subscription/invoices
async fn get_cloud_subscription_invoices(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}

/// POST /api/v4/cloud/webhook
async fn cloud_webhook(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({"status": "OK"})))
}

/// GET /api/v4/cloud/preview/modal_data
async fn get_cloud_preview_modal_data(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(json!({})))
}
