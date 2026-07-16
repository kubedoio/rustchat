#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use uuid::Uuid;

mod common;

struct AuthSession {
    token: String,
    user_uuid: Uuid,
}

struct TestContext {
    app: common::TestApp,
    org_id: Uuid,
    admin: AuthSession,
    member: AuthSession,
}

async fn register_and_login_user(
    app: &common::TestApp,
    org_id: Uuid,
    username: &str,
    email: &str,
    role: &str,
) -> AuthSession {
    let user_data = json!({
        "username": username,
        "email": email,
        "password": "Password123!",
        "display_name": username,
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("failed to register user");

    let user_uuid: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to lookup registered user");

    if role != "member" {
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_uuid)
            .execute(&app.db_pool)
            .await
            .expect("failed to set custom role");
    }

    let login_res = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&json!({
            "login_id": email,
            "password": "Password123!"
        }))
        .send()
        .await
        .expect("failed to login");

    assert_eq!(200, login_res.status().as_u16());
    let token = login_res
        .headers()
        .get("Token")
        .expect("token header missing")
        .to_str()
        .expect("token header invalid")
        .to_string();

    let me_res = app
        .api_client
        .get(format!("{}/api/v4/users/me", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to get /users/me");
    assert_eq!(200, me_res.status().as_u16());

    let me_body: serde_json::Value = me_res.json().await.expect("invalid /users/me body");
    let user_id = me_body["id"].as_str().expect("missing /users/me id");
    let user_uuid = parse_mm_or_uuid(user_id).expect("invalid mattermost-compatible user id");

    AuthSession { token, user_uuid }
}

async fn setup_context() -> TestContext {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");

    let admin = register_and_login_user(
        &app,
        org_id,
        "mmchannelsadmin",
        "mmchannelsadmin@example.com",
        "system_admin",
    )
    .await;
    let member = register_and_login_user(
        &app,
        org_id,
        "mmchannelsmember",
        "mmchannelsmember@example.com",
        "member",
    )
    .await;

    TestContext {
        app,
        org_id,
        admin,
        member,
    }
}

async fn setup_team_with_channels(ctx: &TestContext) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'platform', 'Platform Team', true)",
    )
    .bind(team_id)
    .bind(ctx.org_id)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create team");

    for user_id in [ctx.admin.user_uuid, ctx.member.user_uuid] {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(team_id)
            .bind(user_id)
            .execute(&ctx.app.db_pool)
            .await
            .expect("failed to add team member");
    }

    let town_square = insert_channel(ctx, team_id, "town-square", false).await;
    let off_topic = insert_channel(ctx, team_id, "off-topic", false).await;
    let engineering = insert_channel(ctx, team_id, "engineering", false).await;
    let archived_private = insert_channel(ctx, team_id, "archived-private", true).await;

    (
        team_id,
        town_square,
        off_topic,
        engineering,
        archived_private,
    )
}

async fn insert_channel(ctx: &TestContext, team_id: Uuid, name: &str, is_archived: bool) -> Uuid {
    let channel_id = Uuid::new_v4();
    let display_name = name.replace('-', " ");
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, is_archived, creator_id) VALUES ($1, $2, $3, $4, $5::channel_type, $6, $7)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(name)
    .bind(display_name)
    .bind("public")
    .bind(is_archived)
    .bind(ctx.admin.user_uuid)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create channel");

    for user_id in [ctx.admin.user_uuid, ctx.member.user_uuid] {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')",
        )
        .bind(channel_id)
        .bind(user_id)
        .execute(&ctx.app.db_pool)
        .await
        .expect("failed to add channel member");
    }

    channel_id
}

#[tokio::test]
async fn mm_get_all_channels_requires_system_manage() {
    let ctx = setup_context().await;
    let _ = setup_team_with_channels(&ctx).await;

    let response = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels?page=0&per_page=20",
            ctx.app.address
        ))
        .header("Authorization", format!("Bearer {}", ctx.member.token))
        .send()
        .await
        .expect("request failed");

    assert_eq!(403, response.status().as_u16());
}

#[tokio::test]
async fn mm_get_all_channels_supports_filters_and_total_count() {
    let ctx = setup_context().await;
    let (_team_id, _town_square, _off_topic, _engineering, _archived_private) =
        setup_team_with_channels(&ctx).await;

    let base_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels?page=0&per_page=100",
            ctx.app.address
        ))
        .header("Authorization", format!("Bearer {}", ctx.admin.token))
        .send()
        .await
        .expect("request failed");
    assert_eq!(200, base_res.status().as_u16());
    let base_body: serde_json::Value = base_res.json().await.expect("invalid response body");
    let base_channels = base_body.as_array().expect("expected channels array");
    assert_eq!(
        3,
        base_channels.len(),
        "archived channel should be excluded"
    );

    for channel in base_channels {
        assert!(channel["team_name"].is_string());
        assert!(channel["team_display_name"].is_string());
        assert!(channel["team_update_at"].is_number());
    }

    let filtered_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels?include_deleted=true&exclude_default_channels=true&include_total_count=true&page=0&per_page=100",
            ctx.app.address
        ))
        .header("Authorization", format!("Bearer {}", ctx.admin.token))
        .send()
        .await
        .expect("request failed");
    assert_eq!(200, filtered_res.status().as_u16());

    let filtered_body: serde_json::Value = filtered_res.json().await.expect("invalid response");
    let filtered_channels = filtered_body["channels"]
        .as_array()
        .expect("channels field should be an array");
    let total_count = filtered_body["total_count"]
        .as_i64()
        .expect("total_count should be integer");

    assert_eq!(2, filtered_channels.len());
    assert_eq!(2, total_count);

    let names: Vec<String> = filtered_channels
        .iter()
        .map(|c| c["name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(names.contains(&"engineering".to_string()));
    assert!(names.contains(&"archived-private".to_string()));
    assert!(!names.contains(&"town-square".to_string()));
    assert!(!names.contains(&"off-topic".to_string()));
}

async fn setup_empty_team(ctx: &TestContext) -> Uuid {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'empty-team', 'Empty Team', true)",
    )
    .bind(team_id)
    .bind(ctx.org_id)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create team");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(ctx.admin.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .expect("failed to add team member");

    team_id
}

#[tokio::test]
async fn create_channel_rejects_invalid_names() {
    let ctx = setup_context().await;
    let team_id = setup_empty_team(&ctx).await;

    let base_url = format!("{}/api/v4/channels", ctx.app.address);

    let send_create = |name: &str| {
        ctx.app
            .api_client
            .post(&base_url)
            .header("Authorization", format!("Bearer {}", ctx.admin.token))
            .json(&json!({
                "team_id": encode_mm_id(team_id),
                "name": name,
                "display_name": "Display Name",
                "type": "O"
            }))
            .send()
    };

    // Empty name
    let res = send_create("").await.expect("request failed");
    assert_eq!(400, res.status().as_u16(), "empty name should be rejected");

    // Whitespace-only name
    let res = send_create("   ").await.expect("request failed");
    assert_eq!(
        400,
        res.status().as_u16(),
        "whitespace-only name should be rejected"
    );

    // Name exceeding 64 characters
    let long_name = "a".repeat(65);
    let res = send_create(&long_name).await.expect("request failed");
    assert_eq!(
        400,
        res.status().as_u16(),
        "name exceeding 64 characters should be rejected"
    );

    // Name with invalid characters
    let res = send_create("bad name!").await.expect("request failed");
    assert_eq!(
        400,
        res.status().as_u16(),
        "name with invalid characters should be rejected"
    );

    // Valid name should succeed
    let res = send_create("valid-channel").await.expect("request failed");
    assert_eq!(
        200,
        res.status().as_u16(),
        "valid channel name should be accepted"
    );

    // Whitespace-padded valid name should succeed and be stored trimmed
    let res = send_create("  trimmed-channel  ")
        .await
        .expect("request failed");
    assert_eq!(
        200,
        res.status().as_u16(),
        "whitespace-padded valid channel name should be accepted"
    );
    let body = res
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse response");
    assert_eq!(
        Some("trimmed-channel"),
        body["name"].as_str(),
        "stored channel name should be trimmed"
    );
}
