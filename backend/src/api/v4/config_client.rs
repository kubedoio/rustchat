use crate::api::AppState;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::mattermost_compat::{id::encode_mm_id, MM_VERSION};
use crate::models::server_config::{AuthConfig, SiteConfig};
use axum::{extract::{Query, State}, response::IntoResponse, routing::get, Json, Router};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/config/client", get(get_client_config))
        .route("/license/client", get(get_client_license))
}

#[derive(Deserialize)]
pub struct LicenseQuery {
    pub format: Option<String>,
}

pub async fn get_client_config(
    State(state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    if !matches!(query.format.as_deref(), Some("old")) {
        return Ok((
            axum::http::StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "id": "api.config.client.old_format.app_error",
                "message": "The new format for client config is not supported yet. Please provide \"format=old\" in the request.",
                "detailed_error": "",
                "request_id": "",
                "status_code": 501
            })),
        ));
    }

    let (site, auth) = sqlx::query_as::<_, (sqlx::types::Json<SiteConfig>, sqlx::types::Json<AuthConfig>)>(
        "SELECT site, authentication FROM server_config WHERE id = 'default'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|row| (row.0 .0, row.1 .0))
    .unwrap_or_else(|| (SiteConfig::default(), AuthConfig::default()));

    let diagnostic_id = diagnostic_id(&site);
    Ok((
        axum::http::StatusCode::OK,
        Json(legacy_config(&site, &auth, &diagnostic_id)),
    ))
}

pub async fn get_client_license(
    State(_state): State<AppState>,
    Query(query): Query<LicenseQuery>,
) -> ApiResult<impl IntoResponse> {
    let body = if matches!(query.format.as_deref(), Some("old")) {
        serde_json::json!({
            "IsLicensed": "false"
        })
    } else {
        serde_json::to_value(mm::License {
            is_licensed: false,
            issued_at: 0,
            starts_at: 0,
            expires_at: 0,
        })
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    };

    Ok(Json(body))
}

fn legacy_config(site: &SiteConfig, auth: &AuthConfig, diagnostic_id: &str) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut map = Map::new();
    let insert = |map: &mut Map<String, Value>, key: &str, value: &str| {
        map.insert(key.to_string(), Value::String(value.to_string()));
    };

    insert(&mut map, "AboutLink", &site.about_link);
    insert(&mut map, "AllowDownloadLogs", bool_str(site.allow_download_logs));
    insert(&mut map, "AndroidAppDownloadLink", &site.android_app_download_link);
    insert(&mut map, "AndroidLatestVersion", "");
    insert(&mut map, "AndroidMinVersion", "");
    insert(&mut map, "AppDownloadLink", &site.app_download_link);
    insert(&mut map, "AppsPluginEnabled", "true");
    insert(&mut map, "AsymmetricSigningPublicKey", "");
    insert(&mut map, "BuildDate", "");
    insert(&mut map, "BuildEnterpriseReady", "false");
    insert(&mut map, "BuildHash", "");
    insert(&mut map, "BuildHashEnterprise", "none");
    insert(&mut map, "BuildNumber", MM_VERSION);
    insert(&mut map, "CWSURL", "");
    insert(&mut map, "CustomBrandText", &site.custom_brand_text);
    insert(&mut map, "CustomDescriptionText", &site.custom_description_text);
    insert(&mut map, "DefaultClientLocale", &site.default_locale);
    insert(&mut map, "SiteName", &site.site_name);
    insert(&mut map, "SiteURL", &site.site_url);
    insert(&mut map, "Version", MM_VERSION);
    insert(&mut map, "DiagnosticId", diagnostic_id);
    insert(&mut map, "EnableCustomBrand", bool_str(site.enable_custom_brand));
    insert(&mut map, "EnableCustomEmoji", bool_str(site.enable_custom_emoji));
    insert(&mut map, "EnableFile", bool_str(site.enable_file));
    insert(&mut map, "EnableUserStatuses", bool_str(site.enable_user_statuses));
    insert(&mut map, "EnableAskCommunityLink", "true");
    insert(&mut map, "EnableBotAccountCreation", "true");
    insert(&mut map, "EnableClientMetrics", "true");
    insert(&mut map, "EnableComplianceExport", "false");
    insert(&mut map, "EnableDesktopLandingPage", "true");
    insert(&mut map, "EnableDiagnostics", bool_str(site.diagnostics_enabled));
    insert(&mut map, "DiagnosticsEnabled", bool_str(site.diagnostics_enabled));
    insert(&mut map, "EnableGuestAccounts", bool_str(auth.enable_guest_accounts));
    insert(&mut map, "EnableJoinLeaveMessageByDefault", "true");
    insert(&mut map, "EnableLdap", bool_str(auth.enable_ldap));
    insert(&mut map, "EnableMultifactorAuthentication", bool_str(auth.enable_multifactor_authentication));
    insert(&mut map, "EnableOpenServer", bool_str(auth.enable_open_server));
    insert(&mut map, "EnableSaml", bool_str(auth.enable_saml));
    insert(
        &mut map,
        "EnableSignInWithEmail",
        bool_str(auth.enable_email_password && auth.enable_sign_in_with_email),
    );
    insert(
        &mut map,
        "EnableSignInWithUsername",
        bool_str(auth.enable_email_password && auth.enable_sign_in_with_username),
    );
    insert(
        &mut map,
        "EnableSignUpWithEmail",
        bool_str(auth.allow_registration && auth.enable_sign_up_with_email),
    );
    insert(&mut map, "EnableSignUpWithGitLab", bool_str(auth.enable_sign_up_with_gitlab));
    insert(&mut map, "EnableSignUpWithGoogle", bool_str(auth.enable_sign_up_with_google));
    insert(&mut map, "EnableSignUpWithOffice365", bool_str(auth.enable_sign_up_with_office365));
    insert(&mut map, "EnableSignUpWithOpenId", bool_str(auth.enable_sign_up_with_openid));
    insert(
        &mut map,
        "EnableUserCreation",
        bool_str(auth.allow_registration && auth.enable_user_creation),
    );
    insert(&mut map, "EnforceMultifactorAuthentication", bool_str(auth.enforce_multifactor_authentication));
    insert(&mut map, "FeatureFlagAppsEnabled", "false");
    insert(&mut map, "FeatureFlagAttributeBasedAccessControl", "true");
    insert(&mut map, "FeatureFlagChannelBookmarks", "true");
    insert(&mut map, "FeatureFlagCloudAnnualRenewals", "false");
    insert(&mut map, "FeatureFlagCloudDedicatedExportUI", "false");
    insert(&mut map, "FeatureFlagCloudIPFiltering", "false");
    insert(&mut map, "FeatureFlagConsumePostHook", "false");
    insert(&mut map, "FeatureFlagContentFlagging", "false");
    insert(&mut map, "FeatureFlagCustomProfileAttributes", "true");
    insert(&mut map, "FeatureFlagDeprecateCloudFree", "false");
    insert(&mut map, "FeatureFlagEnableExportDirectDownload", "false");
    insert(&mut map, "FeatureFlagEnableRemoteClusterService", "false");
    insert(&mut map, "FeatureFlagEnableSharedChannelsDMs", "false");
    insert(&mut map, "FeatureFlagEnableSharedChannelsMemberSync", "false");
    insert(&mut map, "FeatureFlagEnableSharedChannelsPlugins", "true");
    insert(&mut map, "FeatureFlagEnableSyncAllUsersForRemoteCluster", "false");
    insert(&mut map, "FeatureFlagExperimentalAuditSettingsSystemConsoleUI", "true");
    insert(&mut map, "FeatureFlagMobileSSOCodeExchange", "true");
    insert(&mut map, "FeatureFlagMoveThreadsEnabled", "false");
    insert(&mut map, "FeatureFlagNormalizeLdapDNs", "false");
    insert(&mut map, "FeatureFlagNotificationMonitoring", "true");
    insert(&mut map, "FeatureFlagOnboardingTourTips", "true");
    insert(&mut map, "FeatureFlagPermalinkPreviews", "false");
    insert(&mut map, "FeatureFlagStreamlinedMarketplace", "true");
    insert(&mut map, "FeatureFlagTestBoolFeature", "false");
    insert(&mut map, "FeatureFlagTestFeature", "off");
    insert(&mut map, "FeatureFlagWebSocketEventScope", "true");
    insert(&mut map, "FeatureFlagWysiwygEditor", "false");
    insert(&mut map, "FileLevel", "INFO");
    insert(&mut map, "ForgotPasswordLink", "");
    insert(&mut map, "GitLabButtonColor", "");
    insert(&mut map, "GitLabButtonText", "");
    insert(&mut map, "GuestAccountsEnforceMultifactorAuthentication", bool_str(auth.enforce_multifactor_authentication));
    insert(&mut map, "HasImageProxy", "false");
    insert(&mut map, "HelpLink", &site.help_link);
    insert(&mut map, "HideGuestTags", "false");
    insert(&mut map, "IosAppDownloadLink", &site.ios_app_download_link);
    insert(&mut map, "IosLatestVersion", "");
    insert(&mut map, "IosMinVersion", "");
    insert(&mut map, "LdapLoginButtonBorderColor", "");
    insert(&mut map, "LdapLoginButtonColor", "");
    insert(&mut map, "LdapLoginButtonTextColor", "");
    insert(&mut map, "LdapLoginFieldName", "");
    insert(&mut map, "MobileExternalBrowser", "false");
    insert(&mut map, "PasswordMinimumLength", &auth.password_min_length.to_string());
    insert(&mut map, "PasswordEnableForgotLink", bool_str(auth.password_enable_forgot_link));
    insert(&mut map, "PasswordRequireLowercase", bool_str(auth.password_require_lowercase));
    insert(&mut map, "PasswordRequireNumber", bool_str(auth.password_require_number));
    insert(&mut map, "PasswordRequireSymbol", bool_str(auth.password_require_symbol));
    insert(&mut map, "PasswordRequireUppercase", bool_str(auth.password_require_uppercase));
    insert(&mut map, "PluginsEnabled", "true");
    insert(&mut map, "PrivacyPolicyLink", &site.privacy_policy_link);
    insert(&mut map, "ReportAProblemLink", &site.report_a_problem_link);
    insert(&mut map, "ReportAProblemMail", "");
    insert(&mut map, "ReportAProblemType", "default");
    insert(&mut map, "SamlLoginButtonBorderColor", "");
    insert(&mut map, "SamlLoginButtonColor", "");
    insert(&mut map, "SamlLoginButtonText", "");
    insert(&mut map, "SamlLoginButtonTextColor", "");
    insert(&mut map, "ServiceEnvironment", &site.service_environment);
    insert(&mut map, "SupportEmail", &site.support_email);
    insert(&mut map, "TelemetryId", diagnostic_id);
    insert(&mut map, "TermsOfServiceLink", &site.terms_of_service_link);
    insert(&mut map, "WebsocketPort", "80");
    insert(&mut map, "WebsocketSecurePort", "443");
    insert(&mut map, "WebsocketURL", "");
    
    // Add essential fields for mobile
    insert(&mut map, "EnableMobileFileDownload", bool_str(site.enable_mobile_file_download));
    insert(&mut map, "EnableMobileFileUpload", bool_str(site.enable_mobile_file_upload));
    insert(&mut map, "NoAccounts", bool_str(!auth.allow_registration));
    insert(&mut map, "EmailLoginButtonBorderColor", "#2389D7");
    insert(&mut map, "EmailLoginButtonColor", "#0000");
    insert(&mut map, "EmailLoginButtonTextColor", "#2389D7");
    insert(&mut map, "OpenIdButtonColor", "");
    insert(&mut map, "OpenIdButtonText", "");

    Value::Object(map)
}

fn diagnostic_id(site: &SiteConfig) -> String {
    let seed = if !site.site_url.is_empty() {
        site.site_url.as_bytes()
    } else if !site.site_name.is_empty() {
        site.site_name.as_bytes()
    } else {
        b"rustchat"
    };

    let mut hasher = Sha256::new();
    hasher.update(seed);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);

    Uuid::from_slice(&bytes)
        .map(encode_mm_id)
        .unwrap_or_else(|_| encode_mm_id(Uuid::new_v4()))
}

fn bool_str(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
