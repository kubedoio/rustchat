#![allow(clippy::needless_borrows_for_generic_args)]

use reqwest::StatusCode;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::common::spawn_app;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};

mod common;

#[tokio::test]
async fn websocket_connect_receives_hello() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "WS Hello Org").await;
    let (token, _user_id) =
        register_and_login(&app, org_id, "ws_hello_user", "ws_hello_user@example.com").await;

    let mut ws = app.connect_ws_v4(&token).await;
    let hello = app.wait_for_event(&mut ws, "hello", 5000).await;
    assert!(!hello.is_null(), "hello event should contain data");

    let _ = ws.close(None).await;
}

#[tokio::test]
async fn websocket_typing_event_broadcast() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "WS Typing Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "ws_typing_a", "ws_typing_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "ws_typing_b", "ws_typing_b@example.com").await;

    let channel_id = create_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let mut ws_a = app.connect_ws_v4(&token_a).await;
    let mut ws_b = app.connect_ws_v4(&token_b).await;

    app.wait_for_event(&mut ws_a, "hello", 5000).await;
    app.wait_for_event(&mut ws_b, "hello", 5000).await;

    app.send_ws_command(
        &mut ws_a,
        "user_typing",
        json!({
            "channel_id": channel_id,
            "parent_id": ""
        }),
    )
    .await;

    let typing = app.wait_for_event(&mut ws_b, "typing", 5000).await;
    assert_eq!(typing["user_id"], encode_mm_id(user_a));
    assert!(
        typing["display_name"].as_str().is_some(),
        "typing event should include display_name"
    );

    let _ = ws_a.close(None).await;
    let _ = ws_b.close(None).await;
}

#[tokio::test]
async fn websocket_disconnect_presence_offline() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "WS Disconnect Org").await;
    let (token, _user_id) = register_and_login(
        &app,
        org_id,
        "ws_disconnect_user",
        "ws_disconnect_user@example.com",
    )
    .await;

    let mut ws = app.connect_ws_v4(&token).await;
    app.wait_for_event(&mut ws, "hello", 5000).await;

    let status = poll_my_status(&app, &token, Duration::from_secs(3)).await;
    assert_eq!(status["status"], "online");

    ws.close(None)
        .await
        .expect("websocket close frame should be sent");
    drop(ws);

    let offline_status =
        wait_for_status(&app, &token, "offline", false, Duration::from_secs(8)).await;
    assert_eq!(offline_status["status"], "offline");
    assert_eq!(offline_status["manual"], false);
}

async fn insert_org(app: &common::TestApp, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind(name)
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");
    org_id
}

async fn register_and_login(
    app: &common::TestApp,
    org_id: Uuid,
    username: &str,
    email: &str,
) -> (String, Uuid) {
    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": username,
            "email": email,
            "password": "Password123!",
            "display_name": username,
            "org_id": org_id,
        }))
        .send()
        .await
        .expect("register request failed")
        .error_for_status()
        .expect("register should succeed");

    let login = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&json!({
            "login_id": email,
            "password": "Password123!",
        }))
        .send()
        .await
        .expect("login request failed")
        .error_for_status()
        .expect("login should succeed");

    let token = login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("token header missing")
        .to_string();

    let me = app
        .api_client
        .get(format!("{}/api/v4/users/me", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("me request failed")
        .error_for_status()
        .expect("me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("me response should be JSON");

    let user_id = me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("user id should parse");

    (token, user_id)
}

async fn create_channel_with_members(app: &common::TestApp, org_id: Uuid, users: &[Uuid]) -> Uuid {
    let suffix = Uuid::new_v4().to_string().replace('-', "");
    let team_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("team_{suffix}"))
    .bind(format!("Team {suffix}"))
    .execute(&app.db_pool)
    .await
    .expect("failed to create team");

    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, $3, 'public'::channel_type)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(format!("channel_{suffix}"))
    .execute(&app.db_pool)
    .await
    .expect("failed to create channel");

    for user_id in users {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(team_id)
            .bind(*user_id)
            .execute(&app.db_pool)
            .await
            .expect("failed to add team member");

        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')",
        )
        .bind(channel_id)
        .bind(*user_id)
        .execute(&app.db_pool)
        .await
        .expect("failed to add channel member");
    }

    channel_id
}

async fn poll_my_status(app: &common::TestApp, token: &str, within: Duration) -> serde_json::Value {
    let deadline = std::time::Instant::now() + within;
    loop {
        let res = app
            .api_client
            .get(format!("{}/api/v4/users/me/status", app.address))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("status request should succeed");
        if res.status() == StatusCode::OK {
            return res
                .json::<serde_json::Value>()
                .await
                .expect("status response should be valid json");
        }

        assert!(
            std::time::Instant::now() < deadline,
            "timed out polling my status endpoint"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_for_status(
    app: &common::TestApp,
    token: &str,
    expected_status: &str,
    expected_manual: bool,
    within: Duration,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + within;

    loop {
        let status = poll_my_status(app, token, Duration::from_secs(2)).await;
        let status_value = status["status"].as_str().unwrap_or_default();
        let manual_value = status["manual"].as_bool().unwrap_or(false);
        if status_value == expected_status && manual_value == expected_manual {
            return status;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for status={expected_status} manual={expected_manual}; got status={status_value} manual={manual_value}"
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
