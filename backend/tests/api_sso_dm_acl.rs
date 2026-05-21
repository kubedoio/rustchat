#![allow(clippy::needless_borrows_for_generic_args)]
mod common;

use axum::http::StatusCode;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use deadpool_redis::redis::AsyncCommands;
use serde_json::json;
use uuid::Uuid;

use common::{spawn_app, spawn_app_with_config, test_config, TestApp};
use rustchat::services::oauth_token_exchange::{
    create_exchange_code_with_sso, ExchangeCodePayload, SsoExchangeChallenge,
};
use sha2::{Digest, Sha256};

fn unique_slug(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4().simple())
}

fn s256_challenge(code_verifier: &str) -> String {
    let hash = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn get_test_password() -> String {
    format!("Str0ng{}0rd!", "Passw")
}

fn get_admin_password() -> String {
    format!("Admin{}123!", "Pass")
}

async fn create_org(app: &TestApp, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind(name)
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");
    org_id
}

async fn register_user(app: &TestApp, org_id: Uuid, email: &str, password: &str) {
    let username = unique_slug("user");
    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
            "display_name": username,
            "org_id": org_id,
        }))
        .send()
        .await
        .expect("Failed to register user");

    assert_eq!(response.status(), StatusCode::OK);
}

async fn login_v1(app: &TestApp, email: &str, password: &str) -> reqwest::Response {
    app.api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to call v1 login")
}

async fn login_v4(app: &TestApp, login_id: &str, password: &str) -> reqwest::Response {
    app.api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&json!({
            "login_id": login_id,
            "password": password,
        }))
        .send()
        .await
        .expect("Failed to call v4 login")
}

async fn user_id_by_email(app: &TestApp, email: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to load user id")
}

async fn create_admin_token(app: &TestApp) -> String {
    let org_id = create_org(app, "Admin Org").await;
    let email = format!("{}@example.com", unique_slug("admin"));
    let password = &get_admin_password();

    register_user(app, org_id, &email, password).await;

    sqlx::query("UPDATE users SET role = 'system_admin' WHERE email = $1")
        .bind(&email)
        .execute(&app.db_pool)
        .await
        .expect("Failed to promote admin");

    let response = login_v1(app, &email, password).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.expect("Invalid login response");
    body["token"].as_str().expect("Missing token").to_string()
}

async fn patch_authentication_config(
    app: &TestApp,
    admin_token: &str,
    payload: serde_json::Value,
) -> reqwest::Response {
    app.api_client
        .patch(format!(
            "{}/api/v1/admin/config/authentication",
            app.address
        ))
        .bearer_auth(admin_token)
        .json(&payload)
        .send()
        .await
        .expect("Failed to patch auth config")
}

async fn ensure_team_membership(app: &TestApp, org_id: Uuid, user_ids: &[Uuid]) {
    let team_id = Uuid::new_v4();
    let team_name = unique_slug("dm-team");

    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(&team_name)
    .bind("DM Team")
    .execute(&app.db_pool)
    .await
    .expect("Failed to create team");

    for user_id in user_ids {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(team_id)
            .bind(*user_id)
            .execute(&app.db_pool)
            .await
            .expect("Failed to create team membership");
    }
}

async fn create_direct_channel(app: &TestApp, token: &str, user_ids: &[Uuid]) -> reqwest::Response {
    let ids: Vec<String> = user_ids.iter().map(|id| id.to_string()).collect();

    app.api_client
        .post(format!("{}/api/v4/channels/direct", app.address))
        .bearer_auth(token)
        .json(&ids)
        .send()
        .await
        .expect("Failed to create direct channel")
}

#[tokio::test]
async fn require_sso_blocks_password_login_and_break_glass_allows_it() {
    let app = spawn_app().await;

    let admin_token = create_admin_token(&app).await;

    let org_id = create_org(&app, "SSO Org").await;
    let blocked_email = format!("{}@example.com", unique_slug("blocked"));
    let break_glass_email = format!("{}@example.com", unique_slug("breakglass"));
    let password = &get_test_password();

    register_user(&app, org_id, &blocked_email, password).await;
    register_user(&app, org_id, &break_glass_email, password).await;

    let patch_response = patch_authentication_config(
        &app,
        &admin_token,
        json!({
            "enable_email_password": true,
            "enable_sso": true,
            "require_sso": true,
            "sso_break_glass_emails": [break_glass_email.clone()],
        }),
    )
    .await;
    assert_eq!(patch_response.status(), StatusCode::OK);

    let blocked_v1 = login_v1(&app, &blocked_email, password).await;
    assert_eq!(blocked_v1.status(), StatusCode::BAD_REQUEST);

    let blocked_v4 = login_v4(&app, &blocked_email, password).await;
    assert_eq!(blocked_v4.status(), StatusCode::BAD_REQUEST);

    let allowed_v1 = login_v1(&app, &break_glass_email, password).await;
    assert_eq!(allowed_v1.status(), StatusCode::OK);

    let allowed_v4 = login_v4(&app, &break_glass_email, password).await;
    assert_eq!(allowed_v4.status(), StatusCode::OK);
}

#[tokio::test]
async fn v4_sso_code_exchange_supports_success_replay_invalid_and_expired() {
    let app = spawn_app().await;
    let state = unique_slug("state");
    let code_verifier = unique_slug("verifier");
    let challenge = s256_challenge(&code_verifier);

    let code = create_exchange_code_with_sso(
        &app.redis_pool,
        Uuid::new_v4(),
        "v4-exchange@example.com".to_string(),
        "member".to_string(),
        Some(Uuid::new_v4()),
        Some(SsoExchangeChallenge {
            expected_state: state.clone(),
            code_challenge: challenge.clone(),
            code_challenge_method: "S256".to_string(),
        }),
    )
    .await
    .expect("Failed to create exchange code");

    let first = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": code.clone(),
            "code_verifier": code_verifier,
            "state": state,
        }))
        .send()
        .await
        .expect("Failed first code exchange");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body: serde_json::Value = first.json().await.expect("Invalid response body");
    assert!(first_body["token"].as_str().is_some());
    assert_eq!(first_body["csrf"], json!(""));

    let replay = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": code,
            "code_verifier": code_verifier,
            "state": state,
        }))
        .send()
        .await
        .expect("Failed replay request");
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);

    let mismatch_state_code = create_exchange_code_with_sso(
        &app.redis_pool,
        Uuid::new_v4(),
        "v4-exchange-state-mismatch@example.com".to_string(),
        "member".to_string(),
        Some(Uuid::new_v4()),
        Some(SsoExchangeChallenge {
            expected_state: "expected-state".to_string(),
            code_challenge: s256_challenge("expected-verifier"),
            code_challenge_method: "S256".to_string(),
        }),
    )
    .await
    .expect("Failed to create mismatch-state exchange code");

    let mismatch_state = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": mismatch_state_code,
            "code_verifier": "expected-verifier",
            "state": "wrong-state",
        }))
        .send()
        .await
        .expect("Failed mismatch-state request");
    assert_eq!(mismatch_state.status(), StatusCode::BAD_REQUEST);

    let invalid = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": "invalid-code",
            "code_verifier": "some-verifier",
            "state": "some-state",
        }))
        .send()
        .await
        .expect("Failed invalid-code request");
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let expired_code = format!("expired-{}", Uuid::new_v4().simple());
    let expired_payload = ExchangeCodePayload {
        user_id: Uuid::new_v4(),
        email: "expired-v4@example.com".to_string(),
        role: "member".to_string(),
        org_id: Some(Uuid::new_v4()),
        created_at: chrono::Utc::now().timestamp() - 120,
        expected_state: Some("expired-state".to_string()),
        code_challenge: Some(s256_challenge("expired-verifier")),
        code_challenge_method: Some("S256".to_string()),
    };

    let mut redis_conn = app
        .redis_pool
        .get()
        .await
        .expect("Failed to get Redis connection");
    let _: () = redis_conn
        .set_ex(
            format!("rustchat:oauth:code:{expired_code}"),
            serde_json::to_string(&expired_payload).expect("Failed to serialize payload"),
            60u64,
        )
        .await
        .expect("Failed to seed expired exchange code");

    let expired = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": expired_code,
            "code_verifier": "expired-verifier",
            "state": "expired-state",
        }))
        .send()
        .await
        .expect("Failed expired-code request");
    assert_eq!(expired.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_sso_code_exchange_feature_disabled_returns_bad_request() {
    let mut cfg = test_config();
    cfg.compatibility.mobile_sso_code_exchange = false;
    let app = spawn_app_with_config(cfg).await;

    let response = app
        .api_client
        .post(format!(
            "{}/api/v4/users/login/sso/code-exchange",
            app.address
        ))
        .json(&json!({
            "login_code": "any-code",
            "code_verifier": "any-verifier",
            "state": "any-state",
        }))
        .send()
        .await
        .expect("Failed feature-disabled request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn dm_acl_disabled_allows_direct_channel_without_shared_keycloak_group() {
    let app = spawn_app().await;

    let org_id = create_org(&app, "DM ACL Disabled Org").await;

    let creator_email = format!("{}@example.com", unique_slug("creator"));
    let other_email = format!("{}@example.com", unique_slug("other"));
    let password = &get_test_password();

    register_user(&app, org_id, &creator_email, password).await;
    register_user(&app, org_id, &other_email, password).await;

    let creator_id = user_id_by_email(&app, &creator_email).await;
    let other_id = user_id_by_email(&app, &other_email).await;

    ensure_team_membership(&app, org_id, &[creator_id, other_id]).await;

    let creator_login = login_v1(&app, &creator_email, password).await;
    assert_eq!(creator_login.status(), StatusCode::OK);
    let creator_body: serde_json::Value = creator_login.json().await.expect("Invalid login body");
    let creator_token = creator_body["token"].as_str().expect("Missing token");

    let response = create_direct_channel(&app, creator_token, &[creator_id, other_id]).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dm_acl_enabled_blocks_direct_channel_without_shared_keycloak_group() {
    let mut cfg = test_config();
    cfg.messaging.dm_acl_enabled = true;
    let app = spawn_app_with_config(cfg).await;

    let org_id = create_org(&app, "DM ACL Block Org").await;

    let creator_email = format!("{}@example.com", unique_slug("creator"));
    let other_email = format!("{}@example.com", unique_slug("other"));
    let password = &get_test_password();

    register_user(&app, org_id, &creator_email, password).await;
    register_user(&app, org_id, &other_email, password).await;

    let creator_id = user_id_by_email(&app, &creator_email).await;
    let other_id = user_id_by_email(&app, &other_email).await;

    ensure_team_membership(&app, org_id, &[creator_id]).await;
    ensure_team_membership(&app, org_id, &[other_id]).await;

    let creator_login = login_v1(&app, &creator_email, password).await;
    assert_eq!(creator_login.status(), StatusCode::OK);
    let creator_body: serde_json::Value = creator_login.json().await.expect("Invalid login body");
    let creator_token = creator_body["token"].as_str().expect("Missing token");

    let response = create_direct_channel(&app, creator_token, &[creator_id, other_id]).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn dm_acl_enabled_allows_direct_channel_with_shared_keycloak_group() {
    let mut cfg = test_config();
    cfg.messaging.dm_acl_enabled = true;
    let app = spawn_app_with_config(cfg).await;

    let org_id = create_org(&app, "DM ACL Allow Org").await;

    let creator_email = format!("{}@example.com", unique_slug("creator"));
    let other_email = format!("{}@example.com", unique_slug("other"));
    let password = &get_test_password();

    register_user(&app, org_id, &creator_email, password).await;
    register_user(&app, org_id, &other_email, password).await;

    let creator_id = user_id_by_email(&app, &creator_email).await;
    let other_id = user_id_by_email(&app, &other_email).await;

    ensure_team_membership(&app, org_id, &[creator_id, other_id]).await;

    let group_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO groups (name, display_name, description, source, remote_id, allow_reference)
        VALUES (NULL, 'Keycloak DM ACL Group', '', 'plugin_keycloak', $1, TRUE)
        RETURNING id
        "#,
    )
    .bind(format!("kc-{}", Uuid::new_v4().simple()))
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to create Keycloak group");

    sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1, $2), ($1, $3)")
        .bind(group_id)
        .bind(creator_id)
        .bind(other_id)
        .execute(&app.db_pool)
        .await
        .expect("Failed to add group members");

    sqlx::query(
        "INSERT INTO group_dm_acl_flags (group_id, enabled, updated_at) VALUES ($1, TRUE, NOW())",
    )
    .bind(group_id)
    .execute(&app.db_pool)
    .await
    .expect("Failed to enable DM ACL for group");

    let creator_login = login_v1(&app, &creator_email, password).await;
    assert_eq!(creator_login.status(), StatusCode::OK);
    let creator_body: serde_json::Value = creator_login.json().await.expect("Invalid login body");
    let creator_token = creator_body["token"].as_str().expect("Missing token");

    let response = create_direct_channel(&app, creator_token, &[creator_id, other_id]).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn dm_acl_enabled_allows_direct_channel_with_shared_team_membership() {
    let mut cfg = test_config();
    cfg.messaging.dm_acl_enabled = true;
    let app = spawn_app_with_config(cfg).await;

    let org_id = create_org(&app, "DM ACL Team Allow Org").await;

    let creator_email = format!("{}@example.com", unique_slug("creator"));
    let other_email = format!("{}@example.com", unique_slug("other"));
    let password = &get_test_password();

    register_user(&app, org_id, &creator_email, password).await;
    register_user(&app, org_id, &other_email, password).await;

    let creator_id = user_id_by_email(&app, &creator_email).await;
    let other_id = user_id_by_email(&app, &other_email).await;

    ensure_team_membership(&app, org_id, &[creator_id, other_id]).await;

    let creator_login = login_v1(&app, &creator_email, password).await;
    assert_eq!(creator_login.status(), StatusCode::OK);
    let creator_body: serde_json::Value = creator_login.json().await.expect("Invalid login body");
    let creator_token = creator_body["token"].as_str().expect("Missing token");

    let response = create_direct_channel(&app, creator_token, &[creator_id, other_id]).await;
    assert_eq!(response.status(), StatusCode::OK);
}
