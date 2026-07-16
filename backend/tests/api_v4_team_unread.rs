#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::{spawn_app_with_config, test_config};
use rustchat::mattermost_compat::id::parse_mm_or_uuid;
use serde_json::json;
use uuid::Uuid;

mod common;

struct TestContext {
    app: common::TestApp,
    token: String,
    user_id: String,
    user_uuid: Uuid,
    org_id: Uuid,
}

async fn setup_mm_user_with_priority_enabled() -> TestContext {
    let mut config = test_config();
    config.unread.post_priority_enabled = true;
    config.unread.collapsed_threads_enabled = true;
    let app = spawn_app_with_config(config).await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");

    let user_data = json!({
        "username": "urgentreader",
        "email": "urgentreader@example.com",
        "password": "Password123!",
        "display_name": "Urgent Reader",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("failed to register user");

    let login_data = json!({
        "login_id": "urgentreader@example.com",
        "password": "Password123!"
    });

    let response = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("failed to login");
    assert_eq!(200, response.status().as_u16());

    let token = response
        .headers()
        .get("Token")
        .expect("missing token header")
        .to_str()
        .expect("invalid token header")
        .to_string();

    let me_res = app
        .api_client
        .get(format!("{}/api/v4/users/me", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("failed to fetch /users/me");
    let me_body: serde_json::Value = me_res.json().await.expect("invalid /users/me body");
    let user_id = me_body["id"].as_str().expect("missing user id").to_string();
    let user_uuid = parse_mm_or_uuid(&user_id).expect("user id must parse");

    TestContext {
        app,
        token,
        user_id,
        user_uuid,
        org_id,
    }
}

async fn setup_team_channel(ctx: &TestContext) -> (Uuid, Uuid) {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'urgent-team', 'Urgent Team', true)",
    )
    .bind(team_id)
    .bind(ctx.org_id)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create team");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .expect("failed to add team member");

    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'urgent-channel', 'public')",
    )
    .bind(channel_id)
    .bind(team_id)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create channel");
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
        .bind(channel_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .expect("failed to add channel member");

    (team_id, channel_id)
}

#[tokio::test]
async fn team_unread_includes_thread_urgent_mentions_when_enabled() {
    let ctx = setup_mm_user_with_priority_enabled().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;

    let root_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "Thread root",
        }))
        .send()
        .await
        .expect("failed to create root post");
    assert_eq!(200, root_res.status().as_u16());
    let root_body: serde_json::Value = root_res.json().await.expect("invalid root post body");
    let root_id = root_body["id"]
        .as_str()
        .expect("missing root id")
        .to_string();
    let root_uuid = parse_mm_or_uuid(&root_id).expect("root id should parse");

    let root_created_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT created_at FROM posts WHERE id = $1")
            .bind(root_uuid)
            .fetch_one(&ctx.app.db_pool)
            .await
            .expect("failed to fetch root created_at");

    let reply_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "root_id": root_id,
            "message": "@urgentreader @here urgent thread mention",
        }))
        .send()
        .await
        .expect("failed to create reply post");
    assert_eq!(200, reply_res.status().as_u16());

    sqlx::query(
        r#"
        INSERT INTO thread_memberships (user_id, post_id, following, last_read_at, mention_count, unread_replies_count)
        VALUES ($1, $2, true, $3, 1, 1)
        ON CONFLICT (user_id, post_id) DO UPDATE SET
            following = true,
            last_read_at = EXCLUDED.last_read_at,
            mention_count = EXCLUDED.mention_count,
            unread_replies_count = EXCLUDED.unread_replies_count,
            updated_at = NOW()
        "#,
    )
    .bind(ctx.user_uuid)
    .bind(root_uuid)
    .bind(root_created_at)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to upsert thread membership");

    let unread_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/teams/{}/unread?include_collapsed_threads=true",
            ctx.app.address, ctx.user_id, team_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .expect("failed to fetch team unread");
    assert_eq!(200, unread_res.status().as_u16());

    let unread_body: serde_json::Value = unread_res.json().await.expect("invalid unread body");
    assert_eq!(unread_body["thread_count"], 1);
    assert_eq!(unread_body["thread_mention_count"], 1);
    assert_eq!(unread_body["thread_urgent_mention_count"], 1);
}
