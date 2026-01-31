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
        "username": "mmroutes",
        "email": "mmroutes@example.com",
        "password": "Password123!",
        "display_name": "MM Routes",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "mmroutes@example.com",
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
    sqlx::query("INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')")
        .bind(channel_id)
        .bind(team_id)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
        .bind(channel_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    (team_id, channel_id)
}

#[tokio::test]
async fn mm_post_files_info_returns_files() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let part = reqwest::multipart::Part::bytes(b"hello world".to_vec())
        .file_name("test.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new()
        .part("files", part)
        .text("channel_id", channel_id.to_string());

    let upload_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/files", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(200, upload_res.status().as_u16());
    let upload_body: serde_json::Value = upload_res.json().await.unwrap();
    let file_id = upload_body["file_infos"][0]["id"].as_str().unwrap().to_string();

    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "Post with file",
            "file_ids": [file_id]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap();

    let info_res = ctx
        .app
        .api_client
        .get(format!("{}/api/v4/posts/{}/files/info", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, info_res.status().as_u16());
    let info_body: serde_json::Value = info_res.json().await.unwrap();
    assert_eq!(info_body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn mm_flagged_posts_returns_saved_posts() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Saved" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap().to_string();
    let post_uuid = parse_mm_or_uuid(&post_id).unwrap();

    sqlx::query("INSERT INTO saved_posts (user_id, post_id) VALUES ($1, $2)")
        .bind(ctx.user_uuid)
        .bind(post_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    let flagged_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/posts/flagged",
            &ctx.app.address, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, flagged_res.status().as_u16());
    let flagged_body: serde_json::Value = flagged_res.json().await.unwrap();
    assert_eq!(flagged_body["order"].as_array().unwrap().len(), 1);
    let post_key = encode_mm_id(post_uuid);
    assert!(flagged_body["posts"].get(&post_key).is_some());
}

#[tokio::test]
async fn mm_set_unread_returns_channel_unread_at() {
    let ctx = setup_mm_user().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;

    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Unread" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap().to_string();
    let post_uuid = parse_mm_or_uuid(&post_id).unwrap();

    let seq: i64 = sqlx::query_scalar("SELECT seq FROM posts WHERE id = $1")
        .bind(post_uuid)
        .fetch_one(&ctx.app.db_pool)
        .await
        .unwrap_or(0);

    let unread_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/users/{}/posts/{}/set_unread",
            &ctx.app.address, ctx.user_id, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, unread_res.status().as_u16());
    let unread_body: serde_json::Value = unread_res.json().await.unwrap();
    assert_eq!(unread_body["channel_id"], encode_mm_id(channel_id));
    assert_eq!(unread_body["team_id"], encode_mm_id(team_id));
    assert_eq!(unread_body["msg_count"], (seq - 1).max(0));
}
