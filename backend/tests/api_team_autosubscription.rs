#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::await_holding_lock)]
use crate::common::spawn_app;
use rustchat::models::Team;
use std::sync::Mutex;
use uuid::Uuid;

mod common;

// ponytail: use standard library LazyLock instead of once_cell
static TEST_MUTEX: std::sync::LazyLock<Mutex<()>> = std::sync::LazyLock::new(|| Mutex::new(()));

async fn register_user(app: &common::TestApp, username: &str, email: &str, password: &str) {
    let payload = serde_json::json!({
        "username": username,
        "email": email,
        "password": password,
        "display_name": username
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&payload)
        .send()
        .await
        .expect("register request failed");

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    assert_eq!(
        status, 200,
        "register should succeed, got status {} body {}",
        status, body
    );
}

async fn login_token(app: &common::TestApp, email: &str, password: &str) -> String {
    let payload = serde_json::json!({
        "email": email,
        "password": password
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", &app.address))
        .json(&payload)
        .send()
        .await
        .expect("login request failed");

    assert_eq!(response.status().as_u16(), 200, "login should succeed");

    let body: serde_json::Value = response.json().await.expect("invalid login response");
    body["token"].as_str().expect("missing token").to_string()
}

#[tokio::test]
async fn add_team_member_succeeds_when_default_channel_autojoin_fails() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(&app, "owner_user", "owner_user@example.com", "Password123!").await;
    register_user(
        &app,
        "member_user",
        "member_user@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE email = 'owner_user@example.com'")
        .execute(&app.db_pool)
        .await
        .expect("failed to elevate owner");

    let owner_token = login_token(&app, "owner_user@example.com", "Password123!").await;
    let member_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("member_user@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch member user id");

    let team_payload = serde_json::json!({
        "name": "softfail-team",
        "display_name": "Soft Fail Team",
        "description": "team for soft fail test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");

    sqlx::query(
        "UPDATE server_config
         SET experimental = jsonb_set(experimental, '{test_force_default_channel_join_failure}', 'true'::jsonb, true)
         WHERE id = 'default'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to enable default-channel auto-join failpoint");

    let add_member_payload = serde_json::json!({
        "user_id": member_id,
        "role": "member"
    });
    let add_member_response = app
        .api_client
        .post(format!("{}/api/v1/teams/{}/members", &app.address, team.id))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&add_member_payload)
        .send()
        .await
        .expect("add member request failed");

    assert_eq!(
        add_member_response.status().as_u16(),
        200,
        "team membership should succeed even when default channel auto-join fails"
    );

    let is_team_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to check team membership");
    assert!(
        is_team_member,
        "expected user to be a persisted team member"
    );

    let joined_channel_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to count channel memberships");
    assert_eq!(
        joined_channel_count, 0,
        "forced failure should prevent auto-joined channels while preserving team membership"
    );
}

#[tokio::test]
async fn v4_add_team_member_succeeds_when_default_channel_autojoin_fails() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(
        &app,
        "owner_user_v4",
        "owner_user_v4@example.com",
        "Password123!",
    )
    .await;
    register_user(
        &app,
        "member_user_v4",
        "member_user_v4@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE email = 'owner_user_v4@example.com'")
        .execute(&app.db_pool)
        .await
        .expect("failed to elevate owner");

    let owner_token = login_token(&app, "owner_user_v4@example.com", "Password123!").await;
    let member_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("member_user_v4@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch member user id");

    let team_payload = serde_json::json!({
        "name": "softfail-team-v4",
        "display_name": "Soft Fail Team v4",
        "description": "team for soft fail v4 test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");

    sqlx::query(
        "UPDATE server_config
         SET experimental = jsonb_set(experimental, '{test_force_default_channel_join_failure}', 'true'::jsonb, true)
         WHERE id = 'default'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to enable default-channel auto-join failpoint");

    let add_member_payload = serde_json::json!({
        "user_id": member_id.to_string()
    });
    let add_member_response = app
        .api_client
        .post(format!("{}/api/v4/teams/{}/members", &app.address, team.id))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&add_member_payload)
        .send()
        .await
        .expect("v4 add member request failed");

    assert_eq!(
        add_member_response.status().as_u16(),
        200,
        "v4 team membership should succeed even when default channel auto-join fails"
    );

    let is_team_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to check v4 team membership");
    assert!(
        is_team_member,
        "expected user to be a persisted v4 team member"
    );

    let joined_channel_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to count v4 channel memberships");
    assert_eq!(
        joined_channel_count, 0,
        "forced failure should prevent v4 auto-joined channels while preserving team membership"
    );
}

#[tokio::test]
async fn v4_add_team_member_by_invite_succeeds_when_default_channel_autojoin_fails() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(
        &app,
        "owner_user_v4_invite",
        "owner_user_v4_invite@example.com",
        "Password123!",
    )
    .await;
    register_user(
        &app,
        "member_user_v4_invite",
        "member_user_v4_invite@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query(
        "UPDATE users SET role = 'org_admin' WHERE email = 'owner_user_v4_invite@example.com'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to elevate owner");

    let owner_token = login_token(&app, "owner_user_v4_invite@example.com", "Password123!").await;
    let member_token = login_token(&app, "member_user_v4_invite@example.com", "Password123!").await;

    let member_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("member_user_v4_invite@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch member user id");

    let team_payload = serde_json::json!({
        "name": "softfail-team-v4-invite",
        "display_name": "Soft Fail Team v4 Invite",
        "description": "team for soft fail v4 invite test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");
    let team_invite_id: String = sqlx::query_scalar("SELECT invite_id FROM teams WHERE id = $1")
        .bind(team.id)
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch team invite_id");

    sqlx::query(
        "UPDATE server_config
         SET experimental = jsonb_set(experimental, '{test_force_default_channel_join_failure}', 'true'::jsonb, true)
         WHERE id = 'default'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to enable default-channel auto-join failpoint");

    let invite_join_response = app
        .api_client
        .post(format!(
            "{}/api/v4/teams/members/invite?invite_id={}",
            &app.address, team_invite_id
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .expect("v4 add member by invite request failed");

    assert_eq!(
        invite_join_response.status().as_u16(),
        201,
        "v4 invite-based team membership should succeed even when default channel auto-join fails"
    );

    let is_team_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to check v4 invite team membership");
    assert!(
        is_team_member,
        "expected user to be a persisted v4 invite team member"
    );

    let joined_channel_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE c.team_id = $1 AND cm.user_id = $2
        "#,
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to count v4 invite channel memberships");
    assert_eq!(
        joined_channel_count, 0,
        "forced failure should prevent v4 invite auto-joined channels while preserving team membership"
    );
}

#[tokio::test]
async fn v4_add_team_member_by_token_uses_one_time_token() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(
        &app,
        "owner_user_v4_token",
        "owner_user_v4_token@example.com",
        "Password123!",
    )
    .await;
    register_user(
        &app,
        "member_user_v4_token",
        "member_user_v4_token@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query(
        "UPDATE users SET role = 'org_admin' WHERE email = 'owner_user_v4_token@example.com'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to elevate owner");

    let owner_token = login_token(&app, "owner_user_v4_token@example.com", "Password123!").await;
    let member_token = login_token(&app, "member_user_v4_token@example.com", "Password123!").await;

    let member_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("member_user_v4_token@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch member user id");

    let team_payload = serde_json::json!({
        "name": "team-v4-token",
        "display_name": "Team v4 Token",
        "description": "team for v4 token invite test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");

    sqlx::query("UPDATE teams SET allow_open_invite = false WHERE id = $1")
        .bind(team.id)
        .execute(&app.db_pool)
        .await
        .expect("failed to make team closed");

    let invite_token = format!("invite_token_{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO team_invite_tokens (token, team_id, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
    )
    .bind(&invite_token)
    .bind(team.id)
    .execute(&app.db_pool)
    .await
    .expect("failed to create invite token");

    let invite_join_response = app
        .api_client
        .post(format!(
            "{}/api/v4/teams/members/invite?token={}",
            &app.address, invite_token
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .expect("v4 add member by token request failed");

    assert_eq!(
        invite_join_response.status().as_u16(),
        201,
        "v4 token-based team membership should succeed"
    );

    let is_team_member: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team.id)
    .bind(member_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to check v4 token team membership");
    assert!(
        is_team_member,
        "expected user to be a persisted v4 token team member"
    );

    let token_used_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT used_at FROM team_invite_tokens WHERE token = $1")
            .bind(&invite_token)
            .fetch_one(&app.db_pool)
            .await
            .expect("failed to load token used_at");
    assert!(token_used_at.is_some(), "token should be marked as used");

    let second_attempt_response = app
        .api_client
        .post(format!(
            "{}/api/v4/teams/members/invite?token={}",
            &app.address, invite_token
        ))
        .header("Authorization", format!("Bearer {}", member_token))
        .send()
        .await
        .expect("second v4 add member by token request failed");
    assert_eq!(
        second_attempt_response.status().as_u16(),
        400,
        "used invite token should be rejected"
    );
}

#[tokio::test]
async fn create_team_bootstraps_fallback_default_channels_and_joins_creator() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(
        &app,
        "owner_user_bootstrap",
        "owner_user_bootstrap@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query(
        "UPDATE users SET role = 'org_admin' WHERE email = 'owner_user_bootstrap@example.com'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to elevate owner");

    let owner_token = login_token(&app, "owner_user_bootstrap@example.com", "Password123!").await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("owner_user_bootstrap@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch owner id");

    sqlx::query(
        "UPDATE server_config
         SET experimental = jsonb_set(experimental, '{team_default_channels}', '[]'::jsonb, true)
         WHERE id = 'default'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to clear configured default channels");

    let team_payload = serde_json::json!({
        "name": "bootstrap-team",
        "display_name": "Bootstrap Team",
        "description": "team bootstrap test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");

    let public_channel_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM channels WHERE team_id = $1 AND type = 'public'::channel_type ORDER BY name",
    )
    .bind(team.id)
    .fetch_all(&app.db_pool)
    .await
    .expect("failed to fetch team public channels");
    assert!(
        public_channel_names
            .iter()
            .any(|name| name == "town-square"),
        "town-square channel should exist"
    );
    assert!(
        public_channel_names.iter().any(|name| name == "off-topic"),
        "off-topic channel should exist when default-channel config is empty"
    );

    let creator_default_memberships: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE c.team_id = $1
          AND cm.user_id = $2
          AND c.name = ANY($3::text[])
        "#,
    )
    .bind(team.id)
    .bind(owner_id)
    .bind(vec!["town-square", "off-topic"])
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to count creator default-channel memberships");
    assert_eq!(
        creator_default_memberships, 2,
        "creator should be auto-joined to fallback default channels"
    );
}

#[tokio::test]
async fn create_team_bootstraps_configured_default_channels_and_joins_creator() {
    let _guard = TEST_MUTEX.lock().expect("test mutex poisoned");
    let app = spawn_app().await;

    register_user(
        &app,
        "owner_user_custom_defaults",
        "owner_user_custom_defaults@example.com",
        "Password123!",
    )
    .await;

    // Promote to org_admin BEFORE login
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE email = 'owner_user_custom_defaults@example.com'")
        .execute(&app.db_pool)
        .await
        .expect("failed to elevate owner");

    let owner_token = login_token(
        &app,
        "owner_user_custom_defaults@example.com",
        "Password123!",
    )
    .await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind("owner_user_custom_defaults@example.com")
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to fetch owner id");

    sqlx::query(
        "UPDATE server_config
         SET experimental = jsonb_set(experimental, '{team_default_channels}', '[\"engineering\"]'::jsonb, true)
         WHERE id = 'default'",
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to configure custom default channels");

    let team_payload = serde_json::json!({
        "name": "bootstrap-team-custom-defaults",
        "display_name": "Bootstrap Team Custom Defaults",
        "description": "team bootstrap custom defaults test"
    });
    let team_response = app
        .api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", owner_token))
        .json(&team_payload)
        .send()
        .await
        .expect("team create request failed");
    assert_eq!(
        team_response.status().as_u16(),
        200,
        "team create should succeed"
    );
    let team: Team = team_response.json().await.expect("invalid team response");

    let public_channel_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM channels WHERE team_id = $1 AND type = 'public'::channel_type ORDER BY name",
    )
    .bind(team.id)
    .fetch_all(&app.db_pool)
    .await
    .expect("failed to fetch team public channels");
    assert!(
        public_channel_names
            .iter()
            .any(|name| name == "town-square"),
        "town-square channel should exist for configured defaults"
    );
    assert!(
        public_channel_names
            .iter()
            .any(|name| name == "engineering"),
        "configured default channel should be created"
    );
    assert!(
        !public_channel_names.iter().any(|name| name == "off-topic"),
        "off-topic should not be created when explicit defaults are configured without it"
    );

    let creator_default_memberships: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM channel_members cm
        JOIN channels c ON c.id = cm.channel_id
        WHERE c.team_id = $1
          AND cm.user_id = $2
          AND c.name = ANY($3::text[])
        "#,
    )
    .bind(team.id)
    .bind(owner_id)
    .bind(vec!["town-square", "engineering"])
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to count creator configured default-channel memberships");
    assert_eq!(
        creator_default_memberships, 2,
        "creator should be auto-joined to configured default channels"
    );
}
