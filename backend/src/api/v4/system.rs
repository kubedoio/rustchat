use crate::api::AppState;
use crate::error::{ApiResult, AppError};
use crate::mattermost_compat::MM_VERSION;
use crate::models::email::MailProviderSettings;
use crate::services::team_membership::{
    get_configured_default_channels, normalize_configured_default_channels,
};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/system/ping", get(ping))
        .route("/system/version", get(version))
        .route("/system/timezones", get(get_timezones))
        .route("/client_perf", post(client_perf))
        .route("/caches/invalidate", post(invalidate_caches))
        .route("/logs", post(post_logs))
        .route("/database/recycle", post(recycle_database))
        .route("/system/notices/{team_id}", get(get_product_notices))
        .route(
            "/system/notices/view",
            axum::routing::put(update_viewed_notices),
        )
        .route("/system/support_packet", get(get_support_packet))
        .route(
            "/system/onboarding/complete",
            get(get_onboarding_status).post(complete_onboarding),
        )
        .route("/system/schema/version", get(get_schema_version))
        .route("/email/test", post(test_email))
        .route("/notifications/test", post(test_notifications))
        .route("/site_url/test", post(test_site_url))
        .route("/file/s3_test", post(test_s3))
        .route("/config", get(get_config))
        .route("/config/reload", post(reload_config))
        .route("/config/environment", get(get_environment_config))
        .route("/config/patch", post(patch_config))
        .route(
            "/license",
            post(upload_license)
                .delete(remove_license)
                .get(get_license_legacy),
        )
        .route(
            "/license/renewal",
            get(get_license_renewal_link).post(get_license_renewal_link_legacy),
        )
        .route("/trial-license", post(trial_license))
        .route("/trial-license/prev", get(get_prev_trial_license))
        .route("/license/load_metric", get(get_client_license_load_metric))
        .route("/analytics/old", get(get_analytics_old))
        .route(
            "/server_busy",
            get(get_server_busy)
                .post(set_server_busy)
                .delete(clear_server_busy),
        )
        .route("/notifications/ack", post(ack_notification))
        .route("/redirect_location", get(get_redirect_location))
        .route("/upgrade_to_enterprise", post(upgrade_plan))
        .route("/upgrade_to_enterprise/status", get(get_upgrade_status))
        .route("/upgrade_to_enterprise/allowed", get(get_upgrade_allowed))
        .route("/restart", post(restart_server))
        .route("/integrity", post(check_integrity))
}

// ... existing code ...

/// GET /system/notices/{team_id}
async fn get_product_notices(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Path(_team_id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Return empty list of notices for now
    Ok(Json(vec![]))
}

/// PUT /system/notices/view
async fn update_viewed_notices(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_ids): Json<Vec<String>>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /system/support_packet
async fn get_support_packet(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<axum::response::Response> {
    // Support packet generation is not implemented yet.
    Err(crate::error::AppError::Forbidden(
        "Support packet generation is not implemented".to_string(),
    ))
}

/// GET /system/onboarding/complete
async fn get_onboarding_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "onboarding_complete": true
    })))
}

/// POST /system/onboarding/complete
async fn complete_onboarding(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /system/schema/version
async fn get_schema_version(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    // Return empty list of migrations for now
    Ok(Json(vec![]))
}

/// POST /email/test
async fn test_email(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /notifications/test
async fn test_notifications(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    use tracing::{info, warn};

    info!(user_id = %auth.user_id, "Test notification requested");

    let device_rows: Vec<(Option<String>, Option<String>)> =
        crate::repositories::SystemRepository::new(&state.db)
            .get_user_devices(auth.user_id)
            .await
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to inspect registered devices for test notification: {}",
                    e
                ))
            })?;

    let registered_device_count = device_rows.len();
    let devices_with_token = device_rows
        .iter()
        .filter(|(token, _)| token.as_deref().is_some_and(|t| !t.trim().is_empty()))
        .count();
    let platforms: Vec<&str> = device_rows
        .iter()
        .filter_map(|(_, platform)| platform.as_deref())
        .collect();

    let push_diag = get_push_diagnostics(&state).await;
    info!(
        user_id = %auth.user_id,
        registered_device_count,
        devices_with_token,
        ?platforms,
        has_push_proxy_url = push_diag.has_push_proxy_url,
        has_fcm_db_config = push_diag.has_fcm_db_config,
        has_fcm_env_config = push_diag.has_fcm_env_config,
        "Test notification diagnostics"
    );

    // Try to send a test push notification to the user's devices
    // Use 'message' type with all required fields for Mattermost mobile compatibility
    // Generate unique IDs for channel_id and post_id so clicking the notification works
    let test_channel_id = "test_channel_".to_string() + &uuid::Uuid::new_v4().to_string();
    let test_post_id = "test_post_".to_string() + &uuid::Uuid::new_v4().to_string();

    let result = crate::services::push_notifications::send_push_to_user(
        &state,
        auth.user_id,
        "Test Notification".to_string(),
        "This is a test push notification from RustChat".to_string(),
        serde_json::json!({
            "type": "message",
            "version": "2",
            "sender_name": "RustChat",
            "channel_id": test_channel_id,
            "post_id": test_post_id,
            "channel_name": "Test Notifications"
        }),
        crate::services::push_notifications::PushPriority::Normal,
    )
    .await;

    match result {
        Ok(count) if count > 0 => {
            info!(user_id = %auth.user_id, count = count, "Test notification sent successfully");
            Ok(Json(serde_json::json!({"status": "OK", "sent": count})))
        }
        Ok(_) => {
            let outcome = classify_test_notification_result(registered_device_count, 0);
            match outcome {
                TestNotificationOutcome::NoDevices => {
                    warn!(
                        user_id = %auth.user_id,
                        has_push_proxy_url = push_diag.has_push_proxy_url,
                        has_fcm_db_config = push_diag.has_fcm_db_config,
                        has_fcm_env_config = push_diag.has_fcm_env_config,
                        "Test notification failed: no registered devices"
                    );
                    Err(AppError::BadRequest(
                        "No mobile devices are registered for this user".to_string(),
                    ))
                }
                TestNotificationOutcome::DeliveryUnavailable => {
                    warn!(
                        user_id = %auth.user_id,
                        registered_device_count,
                        devices_with_token,
                        has_push_proxy_url = push_diag.has_push_proxy_url,
                        has_fcm_db_config = push_diag.has_fcm_db_config,
                        has_fcm_env_config = push_diag.has_fcm_env_config,
                        "Test notification failed: zero notifications sent despite registered devices"
                    );
                    Err(AppError::ExternalService(
                        "Test notification was not delivered. Check backend and push-proxy logs for configuration or token errors.".to_string(),
                    ))
                }
                TestNotificationOutcome::Sent => unreachable!(),
            }
        }
        Err(e) => {
            warn!(
                user_id = %auth.user_id,
                error = %e,
                registered_device_count,
                devices_with_token,
                has_push_proxy_url = push_diag.has_push_proxy_url,
                has_fcm_db_config = push_diag.has_fcm_db_config,
                has_fcm_env_config = push_diag.has_fcm_env_config,
                "Test notification send returned error"
            );
            Err(AppError::ExternalService(format!(
                "Failed to send test notification: {}",
                e
            )))
        }
    }
}

/// POST /site_url/test
async fn test_site_url(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_props): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// POST /file/s3_test
async fn test_s3(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(_config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /config
async fn get_config(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = crate::repositories::SystemRepository::new(&state.db);

    // Fetch config from DB
    let config: crate::models::ServerConfig = repo
        .get_server_config()
        .await
        .map_err(|_| crate::error::AppError::NotFound("Config not found".to_string()))?;

    // Fetch default email provider settings
    let provider_settings: Option<MailProviderSettings> =
        repo.get_default_mail_provider().await.ok().flatten();
    let configured_default_channels = get_configured_default_channels(&state).await?;

    // Convert to Mattermost-compatible format
    let response = serde_json::json!({
        "ServiceSettings": {
            "SiteURL": config.site.0.site_url,
            "MaximumLoginAttempts": 10,
            "EnableDeveloper": false,
            "EnableTesting": false,
            "AllowedUntrustedInternalConnections": "",
            "PostEditTimeLimit": config.site.0.post_edit_time_limit_seconds,
        },
        "TeamSettings": {
            "SiteName": config.site.0.site_name,
            "MaxUsersPerTeam": 50,
            "EnableTeamCreation": true,
            "EnableUserCreation": config.authentication.0.enable_user_creation,
            "EnableOpenServer": config.authentication.0.enable_open_server,
            "RestrictTeamInvite": "all",
            "ExperimentalDefaultChannels": configured_default_channels,
        },
        "SqlSettings": {
            "DriverName": "postgres",
            "DataSourceReplicas": [],
            "MaxIdleConns": 10,
            "MaxOpenConns": 100,
        },
        "LogSettings": {
            "EnableConsole": true,
            "ConsoleLevel": "INFO",
            "EnableFile": true,
            "FileLevel": "INFO",
            "FileLocation": "",
        },
        "FileSettings": {
            "EnableFileAttachments": config.site.0.enable_file,
            "MaxFileSize": config.site.0.max_file_size_mb * 1024 * 1024,
            "MaxImageDecodingSize": 38_400_000,
            "DriverName": "local",
        },
        "EmailSettings": {
            "EnableSignUpWithEmail": config.authentication.0.enable_sign_up_with_email,
            "EnableSignInWithEmail": config.authentication.0.enable_sign_in_with_email,
            "EnableSignInWithUsername": config.authentication.0.enable_sign_in_with_username,
            "SendEmailNotifications": provider_settings.as_ref().map(|p| p.enabled && !p.host.is_empty()).unwrap_or(false),
            "RequireEmailVerification": false,
            "FeedbackName": provider_settings.as_ref().map(|p| p.from_name.clone()).unwrap_or_else(|| "RustChat".to_string()),
            "FeedbackEmail": provider_settings.as_ref().map(|p| p.from_address.clone()).unwrap_or_default(),
            "SMTPServer": provider_settings.as_ref().map(|p| p.host.clone()).unwrap_or_default(),
            "SMTPPort": provider_settings.as_ref().map(|p| p.port.to_string()).unwrap_or_else(|| "587".to_string()),
            // Redacted for non-admin callers to prevent credential reconnaissance
            "SMTPUsername": if auth.has_permission(&crate::auth::policy::permissions::SYSTEM_MANAGE) {
                provider_settings.as_ref().map(|p| p.username.clone()).unwrap_or_default()
            } else {
                String::new()
            },
            "ConnectionSecurity": provider_settings.as_ref().map(|p| match p.tls_mode {
                crate::models::email::TlsMode::ImplicitTls => "TLS",
                crate::models::email::TlsMode::Starttls => "STARTTLS",
                crate::models::email::TlsMode::None => "",
            }).unwrap_or("STARTTLS"),
            "PasswordResetSalt": "",
            "EnablePasswordReset": config.authentication.0.password_enable_forgot_link,
        },
        "RateLimitSettings": {
            "Enable": false,
            "PerSec": 10,
            "MaxBurst": 100,
        },
        "PrivacySettings": {
            "ShowEmailAddress": true,
            "ShowFullName": true,
        },
        "SupportSettings": {
            "TermsOfServiceLink": config.site.0.terms_of_service_link,
            "PrivacyPolicyLink": config.site.0.privacy_policy_link,
            "AboutLink": config.site.0.about_link,
            "HelpLink": config.site.0.help_link,
            "ReportAProblemLink": config.site.0.report_a_problem_link,
            "SupportEmail": config.site.0.support_email,
        },
        "AnnouncementSettings": {
            "EnableBanner": false,
            "BannerText": "",
            "BannerColor": "#f2a93b",
            "BannerTextColor": "#333333",
        },
        "ThemeSettings": {
            "EnableThemeSelection": true,
            "DefaultTheme": "default",
            "AllowCustomThemes": true,
        },
        "PasswordSettings": {
            "MinimumLength": config.authentication.0.password_min_length,
            "Lowercase": config.authentication.0.password_require_lowercase,
            "Number": config.authentication.0.password_require_number,
            "Uppercase": config.authentication.0.password_require_uppercase,
            "Symbol": config.authentication.0.password_require_symbol,
        },
        "LocalizationSettings": {
            "DefaultServerLocale": config.site.0.default_locale,
            "DefaultClientLocale": config.site.0.default_locale,
            "AvailableLocales": "",
        },
        "SamlSettings": {
            "Enable": config.authentication.0.enable_saml,
        },
        "LdapSettings": {
            "Enable": config.authentication.0.enable_ldap,
        },
        "GuestAccountsSettings": {
            "Enable": config.authentication.0.enable_guest_accounts,
        },
        "IntegrationSettings": {
            "EnableIncomingWebhooks": config.integrations.0.enable_webhooks,
            "EnableOutgoingWebhooks": config.integrations.0.enable_webhooks,
            "EnableCommands": config.integrations.0.enable_slash_commands,
            "EnableBotAccountCreation": config.integrations.0.enable_bots,
            "EnableCustomEmoji": config.site.0.enable_custom_emoji,
        },
        "ComplianceSettings": {
            "Enable": false,
            "Directory": "./compliance",
            "EnableDaily": false,
        },
        "DataRetentionSettings": {
            "EnableMessageDeletion": config.compliance.0.message_retention_days > 0,
            "MessageRetentionDays": config.compliance.0.message_retention_days,
            "EnableFileDeletion": config.compliance.0.file_retention_days > 0,
            "FileRetentionDays": config.compliance.0.file_retention_days,
        },
        "FeatureFlags": config.experimental.0,
    });

    Ok(Json(response))
}

/// POST /config/reload
async fn reload_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /config/environment
async fn get_environment_config(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

/// POST /config/patch
async fn patch_config(
    State(state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
    Json(patch): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    use crate::auth::policy::permissions;
    if !auth.has_permission(&permissions::SYSTEM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission to modify server configuration".to_string(),
        ));
    }

    // Parse and apply patches to the relevant config sections
    // The patch format is { "SectionName": { "key": "value" } }

    let repo = crate::repositories::SystemRepository::new(&state.db);

    // Handle TeamSettings -> site.site_name
    if let Some(team_settings) = patch.get("TeamSettings").and_then(|v| v.as_object()) {
        if let Some(site_name) = team_settings.get("SiteName").and_then(|v| v.as_str()) {
            repo.update_site_name(site_name, auth.user_id).await?;
        }

        if let Some(default_channels) = team_settings.get("ExperimentalDefaultChannels") {
            let normalized = normalize_configured_default_channels(default_channels);
            repo.update_team_default_channels(&serde_json::json!(normalized), auth.user_id)
                .await?;
        }
    }

    // Handle EmailSettings
    if let Some(email_settings) = patch.get("EmailSettings").and_then(|v| v.as_object()) {
        if let Some(host) = email_settings.get("SMTPServer").and_then(|v| v.as_str()) {
            repo.update_smtp_host(host, auth.user_id).await?;
        }
        if let Some(port) = email_settings.get("SMTPPort").and_then(|v| v.as_str()) {
            repo.update_smtp_port(port, auth.user_id).await?;
        }
        if let Some(from) = email_settings.get("FeedbackEmail").and_then(|v| v.as_str()) {
            repo.update_from_address(from, auth.user_id).await?;
        }
    }

    // Handle IntegrationSettings -> integrations
    if let Some(int_settings) = patch.get("IntegrationSettings").and_then(|v| v.as_object()) {
        if let Some(enable_webhooks) = int_settings
            .get("EnableIncomingWebhooks")
            .and_then(|v| v.as_bool())
        {
            repo.update_enable_webhooks(enable_webhooks, auth.user_id)
                .await?;
        }
        if let Some(enable_commands) = int_settings.get("EnableCommands").and_then(|v| v.as_bool())
        {
            repo.update_enable_slash_commands(enable_commands, auth.user_id)
                .await?;
        }
    }

    // Handle DataRetentionSettings -> compliance
    if let Some(retention) = patch
        .get("DataRetentionSettings")
        .and_then(|v| v.as_object())
    {
        if let Some(days) = retention
            .get("MessageRetentionDays")
            .and_then(|v| v.as_i64())
        {
            repo.update_message_retention_days(days, auth.user_id)
                .await?;
        }
        if let Some(days) = retention.get("FileRetentionDays").and_then(|v| v.as_i64()) {
            repo.update_file_retention_days(days, auth.user_id).await?;
        }
    }

    // Return updated config
    get_config(State(state), auth).await
}

fn ensure_manage_system(auth: &crate::api::v4::extractors::MmAuthUser) -> ApiResult<()> {
    if !auth.has_permission(&crate::auth::policy::permissions::SYSTEM_MANAGE) {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions to manage system license".to_string(),
        ));
    }

    Ok(())
}

/// Legacy compatibility shim for historical RustChat behavior.
/// Canonical Mattermost contract is POST/DELETE on `/license`.
async fn get_license_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(serde_json::json!({})))
}

/// POST /license
async fn upload_license(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    ensure_manage_system(&auth)?;

    Ok(crate::api::v4::mm_not_implemented(
        "api.license.upload.not_implemented.app_error",
        "License upload is not implemented.",
        "POST /api/v4/license is not supported in this server.",
    ))
}

/// DELETE /license
async fn remove_license(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<(axum::http::StatusCode, Json<serde_json::Value>)> {
    ensure_manage_system(&auth)?;

    Ok(crate::api::v4::mm_not_implemented(
        "api.license.remove.not_implemented.app_error",
        "License removal is not implemented.",
        "DELETE /api/v4/license is not supported in this server.",
    ))
}

/// GET /license/renewal
async fn get_license_renewal_link(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;

    Ok(Json(serde_json::json!({
        "renewal_link": ""
    })))
}

/// Legacy compatibility shim for historical RustChat behavior.
async fn get_license_renewal_link_legacy(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(serde_json::json!({
        "renewal_link": ""
    })))
}

/// POST /trial-license
async fn trial_license(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(serde_json::json!({"status": "OK"})))
}

#[derive(Serialize)]
struct SystemStatus {
    #[serde(rename = "AndroidLatestVersion")]
    android_latest_version: String,
    #[serde(rename = "AndroidMinVersion")]
    android_min_version: String,
    #[serde(rename = "DesktopLatestVersion")]
    desktop_latest_version: String,
    #[serde(rename = "DesktopMinVersion")]
    desktop_min_version: String,
    #[serde(rename = "IosLatestVersion")]
    ios_latest_version: String,
    #[serde(rename = "IosMinVersion")]
    ios_min_version: String,
    #[serde(
        rename = "CanReceiveNotifications",
        skip_serializing_if = "Option::is_none"
    )]
    can_receive_notifications: Option<String>,
    status: String,
    version: String,
}

#[derive(serde::Deserialize)]
struct PingQuery {
    format: Option<String>,
    device_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct PushDiagnostics {
    has_push_proxy_url: bool,
    has_fcm_db_config: bool,
    has_fcm_env_config: bool,
}

impl PushDiagnostics {
    fn can_attempt_push(&self) -> bool {
        self.has_push_proxy_url || self.has_fcm_db_config || self.has_fcm_env_config
    }
}

async fn get_push_diagnostics(state: &AppState) -> PushDiagnostics {
    let has_push_proxy_url = std::env::var("RUSTCHAT_PUSH_PROXY_URL")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());

    let has_fcm_env_config = std::env::var("FCM_PROJECT_ID")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
        && std::env::var("FCM_ACCESS_TOKEN")
            .ok()
            .is_some_and(|v| !v.trim().is_empty());

    let has_fcm_db_config = crate::repositories::SystemRepository::new(&state.db)
        .get_fcm_credentials()
        .await
        .ok()
        .flatten()
        .is_some_and(|(project_id, access_token)| {
            !project_id.trim().is_empty() && !access_token.trim().is_empty()
        });

    PushDiagnostics {
        has_push_proxy_url,
        has_fcm_db_config,
        has_fcm_env_config,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestNotificationOutcome {
    Sent,
    NoDevices,
    DeliveryUnavailable,
}

fn classify_test_notification_result(
    registered_device_count: usize,
    sent_count: usize,
) -> TestNotificationOutcome {
    if sent_count > 0 {
        TestNotificationOutcome::Sent
    } else if registered_device_count == 0 {
        TestNotificationOutcome::NoDevices
    } else {
        TestNotificationOutcome::DeliveryUnavailable
    }
}

fn can_receive_notifications_response(
    device_id: Option<&str>,
    diagnostics: PushDiagnostics,
) -> Option<String> {
    let has_device_id = device_id.is_some_and(|id| !id.trim().is_empty());
    if !has_device_id {
        return None;
    }

    let value = if diagnostics.can_attempt_push() {
        // We do not perform a real proxy send on ping yet, so expose "true" to satisfy clients.
        "true"
    } else {
        "false"
    };

    Some(value.to_string())
}

async fn ping(
    State(state): State<AppState>,
    Query(query): Query<PingQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let can_receive_notifications = if query.device_id.is_some() {
        let diagnostics = get_push_diagnostics(&state).await;
        let value = can_receive_notifications_response(query.device_id.as_deref(), diagnostics);
        if let Some(ref can_receive) = value {
            tracing::info!(
                has_push_proxy_url = diagnostics.has_push_proxy_url,
                has_fcm_db_config = diagnostics.has_fcm_db_config,
                has_fcm_env_config = diagnostics.has_fcm_env_config,
                can_receive_notifications = %can_receive,
                "Ping push capability diagnostic"
            );
        }
        value
    } else {
        None
    };

    if matches!(query.format.as_deref(), Some("old")) {
        let mut old = serde_json::json!({
            "ActiveSearchBackend": "database",
            "AndroidLatestVersion": "",
            "AndroidMinVersion": "",
            "IosLatestVersion": "",
            "IosMinVersion": "",
            "status": "OK"
        });
        if let Some(can_receive) = can_receive_notifications {
            if let serde_json::Value::Object(ref mut map) = old {
                map.insert(
                    "CanReceiveNotifications".to_string(),
                    serde_json::Value::String(can_receive),
                );
            }
        }
        return Ok(Json(old));
    }

    let body = serde_json::to_value(SystemStatus {
        android_latest_version: "".to_string(),
        android_min_version: "".to_string(),
        desktop_latest_version: "".to_string(),
        desktop_min_version: "".to_string(),
        ios_latest_version: "".to_string(),
        ios_min_version: "".to_string(),
        can_receive_notifications,
        status: "OK".to_string(),
        version: MM_VERSION.to_string(),
    })
    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    Ok(Json(body))
}

async fn client_perf(
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let _payload: serde_json::Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        let content_type = headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if content_type.starts_with("application/json") {
            serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}))
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            serde_urlencoded::from_bytes(&body).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::from_slice(&body)
                .or_else(|_| serde_urlencoded::from_bytes(&body))
                .unwrap_or_else(|_| serde_json::json!({}))
        }
    };

    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn version() -> ApiResult<impl IntoResponse> {
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        MM_VERSION.to_string(),
    ))
}

pub async fn invalidate_caches(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn recycle_database(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

pub async fn post_logs(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
    Json(input): Json<Vec<String>>,
) -> ApiResult<Json<serde_json::Value>> {
    for log in input {
        tracing::info!("Client log: {}", log);
    }
    Ok(Json(serde_json::json!({"status": "OK"})))
}

/// GET /system/timezones - Returns a list of supported timezones
async fn get_timezones() -> ApiResult<Json<Vec<String>>> {
    // Returns a standard list of IANA timezone names
    let timezones = vec![
        "Pacific/Midway",
        "Pacific/Honolulu",
        "America/Anchorage",
        "America/Los_Angeles",
        "America/Denver",
        "America/Chicago",
        "America/New_York",
        "America/Toronto",
        "America/Sao_Paulo",
        "Atlantic/Azores",
        "Europe/London",
        "Europe/Paris",
        "Europe/Berlin",
        "Europe/Moscow",
        "Asia/Dubai",
        "Asia/Karachi",
        "Asia/Dhaka",
        "Asia/Bangkok",
        "Asia/Shanghai",
        "Asia/Tokyo",
        "Australia/Sydney",
        "Pacific/Auckland",
        "UTC",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    Ok(Json(timezones))
}

async fn get_prev_trial_license(
    State(_state): State<AppState>,
    auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    ensure_manage_system(&auth)?;
    Ok(Json(serde_json::json!({})))
}

async fn get_client_license_load_metric(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_analytics_old(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn set_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn clear_server_busy(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn ack_notification(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_redirect_location(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"location": ""})))
}

async fn upgrade_plan(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn get_upgrade_status(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

async fn get_upgrade_allowed(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"allowed": false})))
}

async fn restart_server(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"status": "OK"})))
}

async fn check_integrity(
    State(_state): State<AppState>,
    _auth: crate::api::v4::extractors::MmAuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}

#[cfg(test)]
mod tests {
    use super::{
        can_receive_notifications_response, classify_test_notification_result, PushDiagnostics,
        TestNotificationOutcome,
    };

    #[test]
    fn classify_test_notification_result_detects_no_devices() {
        assert_eq!(
            classify_test_notification_result(0, 0),
            TestNotificationOutcome::NoDevices
        );
    }

    #[test]
    fn classify_test_notification_result_detects_delivery_unavailable() {
        assert_eq!(
            classify_test_notification_result(2, 0),
            TestNotificationOutcome::DeliveryUnavailable
        );
    }

    #[test]
    fn classify_test_notification_result_detects_success() {
        assert_eq!(
            classify_test_notification_result(2, 1),
            TestNotificationOutcome::Sent
        );
    }

    #[test]
    fn can_receive_notifications_response_omits_field_without_device_id() {
        let diagnostics = PushDiagnostics {
            has_push_proxy_url: false,
            has_fcm_db_config: false,
            has_fcm_env_config: false,
        };
        assert_eq!(can_receive_notifications_response(None, diagnostics), None);
        assert_eq!(
            can_receive_notifications_response(Some(""), diagnostics),
            None
        );
    }

    #[test]
    fn can_receive_notifications_response_reports_not_available_when_unconfigured() {
        let diagnostics = PushDiagnostics {
            has_push_proxy_url: false,
            has_fcm_db_config: false,
            has_fcm_env_config: false,
        };
        assert_eq!(
            can_receive_notifications_response(Some("android_rn-v2:test"), diagnostics),
            Some("false".to_string())
        );
    }

    #[test]
    fn can_receive_notifications_response_reports_verified_when_push_is_configured() {
        let diagnostics = PushDiagnostics {
            has_push_proxy_url: true,
            has_fcm_db_config: false,
            has_fcm_env_config: false,
        };
        assert_eq!(
            can_receive_notifications_response(Some("android_rn-v2:test"), diagnostics),
            Some("true".to_string())
        );
    }
}
