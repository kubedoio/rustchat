#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
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

async fn setup_mm_user() -> TestContext {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = json!({
        "username": "mmchannelmember",
        "email": "mmchannelmember@example.com",
        "password": "Password123!",
        "display_name": "MM Channel Member",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "mmchannelmember@example.com",
        "password": "Password123!"
    });

    let response = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());
    let token = response
        .headers()
        .get("Token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let me_res = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let me_body: serde_json::Value = me_res.json().await.unwrap();
    let user_id = me_body["id"].as_str().unwrap().to_string();
    let user_uuid = parse_mm_or_uuid(&user_id).unwrap();

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
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'mmteam', 'MM Team', true)",
    )
    .bind(team_id)
    .bind(ctx.org_id)
    .execute(&ctx.app.db_pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')",
    )
    .bind(channel_id)
    .bind(team_id)
    .execute(&ctx.app.db_pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'channel_admin', '{}')")
        .bind(channel_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    (team_id, channel_id)
}

#[tokio::test]
async fn mm_channel_member_routes() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let ids_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/channels/{}/members/ids",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "user_ids": [ctx.user_id.clone()] }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, ids_res.status().as_u16());
    let ids_body: serde_json::Value = ids_res.json().await.unwrap();
    assert_eq!(ids_body.as_array().unwrap().len(), 1);

    let member_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/members/{}",
            &ctx.app.address, channel_id, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, member_res.status().as_u16());
    let member_body: serde_json::Value = member_res.json().await.unwrap();
    assert_eq!(member_body["channel_id"], encode_mm_id(channel_id));

    let roles_res = ctx
        .app
        .api_client
        .put(format!(
            "{}/api/v4/channels/{}/members/{}/roles",
            &ctx.app.address, channel_id, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "roles": "channel_admin channel_user" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, roles_res.status().as_u16());
    let roles_body: serde_json::Value = roles_res.json().await.unwrap();
    assert!(roles_body["roles"]
        .as_str()
        .unwrap()
        .contains("channel_admin"));

    let notify_res = ctx
        .app
        .api_client
        .put(format!(
            "{}/api/v4/channels/{}/members/{}/notify_props",
            &ctx.app.address, channel_id, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "desktop": "mention", "mark_unread": "all" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, notify_res.status().as_u16());
    let notify_body: serde_json::Value = notify_res.json().await.unwrap();
    assert_eq!(notify_body["status"], "OK");
}

#[tokio::test]
async fn mm_channel_member_routes_return_computed_counts() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let first_seq: i64 = sqlx::query_scalar(
        "INSERT INTO posts (channel_id, user_id, message, props) VALUES ($1, $2, $3, '{}'::jsonb) RETURNING seq",
    )
    .bind(channel_id)
    .bind(ctx.user_uuid)
    .bind("seed message")
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to insert first post");

    let _second_seq: i64 = sqlx::query_scalar(
        "INSERT INTO posts (channel_id, user_id, message, props) VALUES ($1, $2, $3, '{}'::jsonb) RETURNING seq",
    )
    .bind(channel_id)
    .bind(ctx.user_uuid)
    .bind("@mmchannelmember @here unread mention")
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to insert second post");

    sqlx::query(
        r#"
        INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, channel_id)
        DO UPDATE SET last_read_message_id = EXCLUDED.last_read_message_id, last_read_at = NOW()
        "#,
    )
    .bind(ctx.user_uuid)
    .bind(channel_id)
    .bind(first_seq)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to set read cursor");

    let member_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/members/{}",
            &ctx.app.address, channel_id, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, member_res.status().as_u16());
    let member_body: serde_json::Value = member_res.json().await.unwrap();
    assert_eq!(member_body["msg_count"], 1);
    assert_eq!(member_body["msg_count_root"], 1);
    assert_eq!(member_body["mention_count"], 1);
    assert_eq!(member_body["mention_count_root"], 1);
}

#[tokio::test]
async fn mm_channel_unread_returns_root_and_team_fields() {
    let ctx = setup_mm_user().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;

    let first_seq: i64 = sqlx::query_scalar(
        "INSERT INTO posts (channel_id, user_id, message, props) VALUES ($1, $2, $3, '{}'::jsonb) RETURNING seq",
    )
    .bind(channel_id)
    .bind(ctx.user_uuid)
    .bind("seed message")
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to insert first post");

    let _second_seq: i64 = sqlx::query_scalar(
        "INSERT INTO posts (channel_id, user_id, message, props) VALUES ($1, $2, $3, '{}'::jsonb) RETURNING seq",
    )
    .bind(channel_id)
    .bind(ctx.user_uuid)
    .bind("@mmchannelmember @here unread mention")
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to insert second post");

    sqlx::query(
        r#"
        INSERT INTO channel_reads (user_id, channel_id, last_read_message_id, last_read_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, channel_id)
        DO UPDATE SET last_read_message_id = EXCLUDED.last_read_message_id, last_read_at = NOW()
        "#,
    )
    .bind(ctx.user_uuid)
    .bind(channel_id)
    .bind(first_seq)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to set read cursor");

    let unread_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/unread",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, unread_res.status().as_u16());
    let unread_body: serde_json::Value = unread_res.json().await.unwrap();
    assert_eq!(unread_body["team_id"], encode_mm_id(team_id));
    assert_eq!(unread_body["channel_id"], encode_mm_id(channel_id));
    assert_eq!(unread_body["msg_count"], 1);
    assert_eq!(unread_body["msg_count_root"], 1);
    assert_eq!(unread_body["mention_count"], 1);
    assert_eq!(unread_body["mention_count_root"], 1);
}

#[tokio::test]
async fn mm_channel_member_leave_with_me() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    // Verify user is a member
    let member_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/members/me",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, member_res.status().as_u16());

    // Leave the channel using "me" placeholder
    let leave_res = ctx
        .app
        .api_client
        .delete(format!(
            "{}/api/v4/channels/{}/members/me",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, leave_res.status().as_u16());

    // Verify user is no longer a member
    let member_res_after = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/members/me",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(403, member_res_after.status().as_u16());
}
