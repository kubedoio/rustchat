#![allow(clippy::needless_borrows_for_generic_args)]
//! OAuth2/OIDC integration tests

mod common;

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use common::spawn_app;

/// Helper to create an admin user and return auth token
async fn create_test_admin(app: &common::TestApp) -> (String, Uuid) {
    let org_id = Uuid::new_v4();

    // Create organization
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // Register a user first
    let username = format!(
        "admin_{}",
        Uuid::new_v4().to_string().split('-').next().unwrap()
    );
    let email = format!("{}@test.com", username);

    let user_data = serde_json::json!({
        "username": username,
        "email": email,
        "password": "AdminPass123!",
        "display_name": "Test Admin",
        "org_id": org_id
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register admin");

    assert_eq!(
        200,
        response.status().as_u16(),
        "Failed to register admin user"
    );

    // Update role to system_admin via SQL
    sqlx::query("UPDATE users SET role = 'system_admin' WHERE email = $1")
        .bind(&email)
        .execute(&app.db_pool)
        .await
        .expect("Failed to update role");

    // Login to get token with new role
    let login_data = serde_json::json!({
        "email": email,
        "password": "AdminPass123!"
    });

    let login_response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(
        200,
        login_response.status().as_u16(),
        "Failed to login admin user"
    );

    let body: serde_json::Value = login_response.json().await.unwrap();
    let token = body["token"].as_str().unwrap().to_string();
    (token, org_id)
}

/// Helper to create a regular user and return auth token
async fn create_test_user(app: &common::TestApp, role: &str) -> String {
    let org_id = Uuid::new_v4();

    // Create organization
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // Register a user
    let username = format!(
        "user_{}",
        Uuid::new_v4().to_string().split('-').next().unwrap()
    );
    let email = format!("{}@test.com", username);

    let user_data = serde_json::json!({
        "username": username,
        "email": email,
        "password": "UserPass123!",
        "display_name": "Test User",
        "org_id": org_id
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register user");

    assert_eq!(200, response.status().as_u16(), "Failed to register user");

    // Update role via SQL
    sqlx::query("UPDATE users SET role = $1 WHERE email = $2")
        .bind(role)
        .bind(&email)
        .execute(&app.db_pool)
        .await
        .expect("Failed to update role");

    // Login to get token with new role
    let login_data = serde_json::json!({
        "email": email,
        "password": "UserPass123!"
    });

    let login_response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(
        200,
        login_response.status().as_u16(),
        "Failed to login user"
    );

    let body: serde_json::Value = login_response.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}

/// Helper to create an SSO configuration for testing
async fn create_test_sso_config(
    app: &common::TestApp,
    token: &str,
    provider_type: &str,
    provider_key: &str,
    issuer_url: Option<&str>,
) -> String {
    let mut request = json!({
        "provider_key": provider_key,
        "provider_type": provider_type,
        "display_name": format!("Test {}", provider_key),
        "client_id": "test-client-id",
        "client_secret": "test-client-secret",
        "is_active": true,
        "auto_provision": true,
    });

    if let Some(url) = issuer_url {
        request["issuer_url"] = json!(url);
    }

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Failed to create SSO config: {:?}",
        response.text().await
    );
    let body: serde_json::Value = response.json().await.unwrap();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_oauth_login_redirects_to_provider() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    // Create a test GitHub config (no OIDC discovery needed)
    let _config_id = create_test_sso_config(&app, &token, "github", "test-github", None).await;

    // Test login endpoint
    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/oauth2/test-github/login?redirect_uri=/dashboard",
            app.address
        ))
        .send()
        .await
        .unwrap();

    // Should redirect (302 or 307)
    assert!(
        response.status().is_redirection(),
        "Expected redirect, got {:?}",
        response.status()
    );
}

#[tokio::test]
async fn test_legacy_mobile_login_route_redirects_to_oauth2_login() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let _config_id = create_test_sso_config(
        &app,
        &token,
        "google",
        "test-google",
        Some("https://accounts.google.com"),
    )
    .await;

    let response = app
        .api_client
        .get(format!(
            "{}/oauth/google/mobile_login?redirect_to={}&state=test-state&code_challenge=test-challenge&code_challenge_method=S256",
            app.address,
            urlencoding::encode("mmauth://callback")
        ))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(location.starts_with("/api/v1/oauth2/test-google/login?"));
    assert!(location.contains("mobile=true"));
    assert!(location.contains("redirect_to=mmauth%3A%2F%2Fcallback"));
    assert!(location.contains("state=test-state"));
    assert!(location.contains("code_challenge=test-challenge"));
}

#[tokio::test]
async fn test_legacy_mobile_login_route_rejects_invalid_custom_scheme() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let _config_id = create_test_sso_config(
        &app,
        &token,
        "google",
        "test-google-2",
        Some("https://accounts.google.com"),
    )
    .await;

    let response = app
        .api_client
        .get(format!(
            "{}/oauth/google/mobile_login?redirect_to={}",
            app.address,
            urlencoding::encode("javascript:alert('hello')")
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_legacy_mobile_login_route_rejects_unconfigured_custom_scheme() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let _config_id = create_test_sso_config(
        &app,
        &token,
        "google",
        "test-google-3",
        Some("https://accounts.google.com"),
    )
    .await;

    let response = app
        .api_client
        .get(format!(
            "{}/oauth/google/mobile_login?redirect_to={}",
            app.address,
            urlencoding::encode("customscheme://callback")
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_legacy_mobile_login_route_accepts_configured_custom_scheme() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let _config_id = create_test_sso_config(
        &app,
        &token,
        "google",
        "test-google-4",
        Some("https://accounts.google.com"),
    )
    .await;

    sqlx::query(
        "UPDATE server_config
         SET site = jsonb_set(
             COALESCE(site, '{}'::jsonb),
             '{app_custom_url_schemes}',
             $1::jsonb,
             true
         )
         WHERE id = 'default'",
    )
    .bind(serde_json::json!(["customscheme://"]))
    .execute(&app.db_pool)
    .await
    .unwrap();

    let response = app
        .api_client
        .get(format!(
            "{}/oauth/google/mobile_login?redirect_to={}",
            app.address,
            urlencoding::encode("customscheme://callback")
        ))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(location.contains("redirect_to=customscheme%3A%2F%2Fcallback"));
}

#[tokio::test]
async fn test_legacy_login_route_redirects_to_oauth2_login() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let _config_id = create_test_sso_config(&app, &token, "github", "legacy-github", None).await;

    let response = app
        .api_client
        .get(format!(
            "{}/oauth/legacy-github/login?redirect_to=/channels/town-square",
            app.address
        ))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_redirection());
    let location = response
        .headers()
        .get("location")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(location.starts_with("/api/v1/oauth2/legacy-github/login"));
    assert!(location.contains("redirect_uri=%2Fchannels%2Ftown-square"));
}

#[tokio::test]
async fn test_oauth_callback_invalid_state_returns_error() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    // Create a test GitHub config (no OIDC discovery needed)
    let _config_id = create_test_sso_config(&app, &token, "github", "test-github", None).await;

    // Test callback with invalid state
    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/oauth2/test-github/callback?code=123&state=invalid-state",
            app.address
        ))
        .send()
        .await
        .unwrap();

    // Invalid state should return 400 or redirect to login
    // Depending on implementation, either is acceptable
    let status = response.status();
    assert!(
        status.is_redirection() || status == StatusCode::BAD_REQUEST,
        "Expected redirect or 400, got {:?}",
        status
    );

    if status.is_redirection() {
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.contains("/login") || location.contains("error"));
    }
}

#[tokio::test]
async fn test_admin_sso_crud_operations() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    // Create
    let create_response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "my-oidc",
            "provider_type": "oidc",
            "display_name": "My OIDC Provider",
            "issuer_url": "https://auth.example.com",
            "client_id": "my-client-id",
            "client_secret": "my-client-secret",
            "scopes": ["openid", "profile", "email"],
            "is_active": true,
            "auto_provision": true,
            "default_role": "member",
            "groups_claim": "groups",
            "role_mappings": {"admins": "system_admin", "users": "member"},
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);
    let created: serde_json::Value = create_response.json().await.unwrap();
    let config_id = created["id"].as_str().unwrap();
    assert_eq!(created["provider_key"], "my-oidc");
    assert_eq!(created["provider_type"], "oidc");
    assert_eq!(created["issuer_url"], "https://auth.example.com");
    // Secret should NOT be returned
    assert!(!created.as_object().unwrap().contains_key("client_secret"));
    assert!(!created
        .as_object()
        .unwrap()
        .contains_key("client_secret_encrypted"));

    // List
    let list_response = app
        .api_client
        .get(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = list_response.json().await.unwrap();
    assert!(configs.iter().any(|c| c["provider_key"] == "my-oidc"));

    // Get
    let get_response = app
        .api_client
        .get(&format!("{}/api/v1/admin/sso/{}", app.address, config_id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let fetched: serde_json::Value = get_response.json().await.unwrap();
    assert_eq!(fetched["provider_key"], "my-oidc");

    // Update
    let update_response = app
        .api_client
        .put(&format!("{}/api/v1/admin/sso/{}", app.address, config_id))
        .bearer_auth(&token)
        .json(&json!({
            "display_name": "Updated Name",
            "is_active": false,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: serde_json::Value = update_response.json().await.unwrap();
    assert_eq!(updated["display_name"], "Updated Name");
    assert_eq!(updated["is_active"], false);

    // Delete
    let delete_response = app
        .api_client
        .delete(&format!("{}/api/v1/admin/sso/{}", app.address, config_id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify deletion
    let get_response = app
        .api_client
        .get(&format!("{}/api/v1/admin/sso/{}", app.address, config_id))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_sso_validation_errors() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    // Missing required fields for OIDC
    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "invalid-oidc",
            "provider_type": "oidc",
            // Missing issuer_url, client_id, client_secret
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Invalid provider type
    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "test",
            "provider_type": "invalid",
            "client_id": "test",
            "client_secret": "test",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Invalid provider key (uppercase)
    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "InvalidKey",
            "provider_type": "github",
            "client_id": "test",
            "client_secret": "test",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Duplicate provider key
    let _ = create_test_sso_config(&app, &token, "github", "duplicate-key", None).await;

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "duplicate-key",
            "provider_type": "github",
            "client_id": "test",
            "client_secret": "test",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_sso_non_admin_denied() {
    let app = spawn_app().await;
    let user_token = create_test_user(&app, "member").await;

    let response = app
        .api_client
        .get(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_sso_config_response_excludes_secrets() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "secure-test",
            "provider_type": "oidc",
            "issuer_url": "https://secure.example.com",
            "client_id": "public-id",
            "client_secret": "super-secret-value-that-should-not-leak",
            "is_active": true,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();

    // Verify secret is NOT in response
    let body_str = body.to_string();
    assert!(!body_str.contains("super-secret"));
    assert!(!body.as_object().unwrap().contains_key("client_secret"));
    assert!(!body
        .as_object()
        .unwrap()
        .contains_key("client_secret_encrypted"));

    // Verify public fields are present
    assert_eq!(body["client_id"], "public-id");
    assert_eq!(body["provider_key"], "secure-test");
}

#[tokio::test]
async fn test_github_sso_config() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "github-enterprise",
            "provider_type": "github",
            "display_name": "GitHub Enterprise",
            "client_id": "github-client-id",
            "client_secret": "github-secret",
            "github_org": "mycompany",
            "github_team": "engineering",
            "is_active": true,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["provider_type"], "github");
    assert_eq!(body["github_org"], "mycompany");
    assert_eq!(body["github_team"], "engineering");
}

#[tokio::test]
async fn test_google_sso_config() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "google-workspace",
            "provider_type": "google",
            "display_name": "Google Workspace",
            "issuer_url": "https://accounts.google.com",
            "client_id": "google-client-id",
            "client_secret": "google-secret",
            "allow_domains": ["company.com", "subsidiary.com"],
            "is_active": true,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["provider_type"], "google");
    assert_eq!(body["issuer_url"], "https://accounts.google.com");
    let domains = body["allow_domains"].as_array().unwrap();
    assert!(domains.contains(&json!("company.com")));
    assert!(domains.contains(&json!("subsidiary.com")));
}

#[tokio::test]
async fn test_oidc_scopes_must_include_openid() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    let response = app
        .api_client
        .post(&format!("{}/api/v1/admin/sso", app.address))
        .bearer_auth(&token)
        .json(&json!({
            "provider_key": "bad-oidc",
            "provider_type": "oidc",
            "issuer_url": "https://example.com",
            "client_id": "test",
            "client_secret": "test",
            "scopes": ["profile", "email"], // Missing openid!
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_sso_list_includes_login_url() {
    let app = spawn_app().await;
    let (token, _org_id) = create_test_admin(&app).await;

    // Create a provider
    let _ = create_test_sso_config(&app, &token, "github", "my-github", None).await;

    // Enable SSO
    app.api_client
        .patch(&format!(
            "{}/api/v1/admin/config/authentication",
            app.address
        ))
        .bearer_auth(&token)
        .json(&json!({
            "enable_email_password": true,
            "enable_sso": true,
            "require_sso": false,
        }))
        .send()
        .await
        .unwrap();

    // Query public providers endpoint
    let response = app
        .api_client
        .get(&format!("{}/api/v1/oauth2/providers", app.address))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let providers: Vec<serde_json::Value> = response.json().await.unwrap();

    assert!(!providers.is_empty());
    let provider = providers
        .iter()
        .find(|p| p["provider_key"] == "my-github")
        .unwrap();
    assert!(provider["login_url"]
        .as_str()
        .unwrap()
        .contains("/oauth2/my-github/login"));
}
