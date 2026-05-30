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
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')",
    )
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

async fn create_post(ctx: &TestContext, channel_id: Uuid, message: &str) -> String {
    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": message }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    post_body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn mm_update_post_route_put_updates_post_message() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let post_id = create_post(&ctx, channel_id, "before update").await;

    let update_res = ctx
        .app
        .api_client
        .put(format!("{}/api/v4/posts/{}", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "id": post_id,
            "message": "after update"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(200, update_res.status().as_u16());
    let update_body: serde_json::Value = update_res.json().await.unwrap();
    assert_eq!(update_body["id"], post_id);
    assert_eq!(update_body["message"], "after update");

    let stored_message: String = sqlx::query_scalar("SELECT message FROM posts WHERE id = $1")
        .bind(parse_mm_or_uuid(&post_id).unwrap())
        .fetch_one(&ctx.app.db_pool)
        .await
        .unwrap();
    assert_eq!(stored_message, "after update");
}

#[tokio::test]
async fn mm_update_post_route_put_requires_matching_body_id() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let post_id = create_post(&ctx, channel_id, "before mismatch").await;

    let mismatch_id = encode_mm_id(Uuid::new_v4());
    let update_res = ctx
        .app
        .api_client
        .put(format!("{}/api/v4/posts/{}", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "id": mismatch_id,
            "message": "should fail"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(400, update_res.status().as_u16());
}

#[tokio::test]
async fn mm_update_post_route_put_rejects_non_author() {
    let ctx = setup_mm_user().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;
    let post_id = create_post(&ctx, channel_id, "author message").await;

    let intruder_data = json!({
        "username": "mmroutes_intruder",
        "email": "mmroutes_intruder@example.com",
        "password": "Password123!",
        "display_name": "MM Routes Intruder",
        "org_id": ctx.org_id
    });
    ctx.app
        .api_client
        .post(format!("{}/api/v1/auth/register", &ctx.app.address))
        .json(&intruder_data)
        .send()
        .await
        .unwrap();

    let intruder_login = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/users/login", &ctx.app.address))
        .json(&json!({
            "login_id": "mmroutes_intruder@example.com",
            "password": "Password123!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, intruder_login.status().as_u16());
    let intruder_token = intruder_login
        .headers()
        .get("Token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let intruder_me = ctx
        .app
        .api_client
        .get(format!("{}/api/v4/users/me", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", intruder_token))
        .send()
        .await
        .unwrap();
    let intruder_me_body: serde_json::Value = intruder_me.json().await.unwrap();
    let intruder_user_uuid = parse_mm_or_uuid(intruder_me_body["id"].as_str().unwrap()).unwrap();

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(intruder_user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
        .bind(channel_id)
        .bind(intruder_user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    let intruder_update = ctx
        .app
        .api_client
        .put(format!("{}/api/v4/posts/{}", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", intruder_token))
        .json(&json!({
            "id": post_id,
            "message": "intruder edit"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(403, intruder_update.status().as_u16());
}

#[tokio::test]
async fn mm_burn_on_read_routes_return_explicit_not_implemented() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let post_id = create_post(&ctx, channel_id, "burn on read route parity").await;

    let reveal_get = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/posts/{}/reveal",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(501, reveal_get.status().as_u16());

    let burn_delete = ctx
        .app
        .api_client
        .delete(format!(
            "{}/api/v4/posts/{}/burn",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(501, burn_delete.status().as_u16());

    let reveal_post = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/posts/{}/reveal",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(501, reveal_post.status().as_u16());

    let burn_post = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/posts/{}/burn",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(501, burn_post.status().as_u16());
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
    let upload_status = upload_res.status().as_u16();
    assert!(
        upload_status == 200 || upload_status == 201,
        "unexpected upload status: {}",
        upload_status
    );
    let upload_body: serde_json::Value = upload_res.json().await.unwrap();
    let file_id = upload_body["file_infos"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

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
        .get(format!(
            "{}/api/v4/posts/{}/files/info",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, info_res.status().as_u16());
    let info_body: serde_json::Value = info_res.json().await.unwrap();
    assert_eq!(info_body.as_array().unwrap().len(), 1);

    let link_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/files/{}/link",
            &ctx.app.address, file_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, link_res.status().as_u16());
    let link_body: serde_json::Value = link_res.json().await.unwrap();
    let link = link_body["link"].as_str().unwrap();
    assert!(link.starts_with("/api/v4/files/"));
    assert!(!link.contains("X-Amz-"));
    assert!(!link.starts_with("http://"));
    assert!(!link.starts_with("https://"));
}

#[tokio::test]
async fn native_file_upload_rejects_non_member_channel_association() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let intruder_data = json!({
        "username": "nativefileintruder",
        "email": "nativefileintruder@example.com",
        "password": "Password123!",
        "display_name": "Native File Intruder",
        "org_id": ctx.org_id
    });
    ctx.app
        .api_client
        .post(format!("{}/api/v1/auth/register", &ctx.app.address))
        .json(&intruder_data)
        .send()
        .await
        .unwrap();

    let login_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/users/login", &ctx.app.address))
        .json(&json!({
            "login_id": "nativefileintruder@example.com",
            "password": "Password123!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, login_res.status().as_u16());
    let intruder_token = login_res
        .headers()
        .get("Token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let part = reqwest::multipart::Part::bytes(b"hello world".to_vec())
        .file_name("private.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let upload_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v1/files?channel_id={}",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", intruder_token))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(403, upload_res.status().as_u16());

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE channel_id = $1 AND name = $2")
            .bind(channel_id)
            .bind("private.txt")
            .fetch_one(&ctx.app.db_pool)
            .await
            .unwrap();
    assert_eq!(0, count);
}

#[tokio::test]
async fn native_file_upload_returns_authenticated_api_url() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let part = reqwest::multipart::Part::bytes(b"hello member".to_vec())
        .file_name("member.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let upload_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v1/files?channel_id={}",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(200, upload_res.status().as_u16());
    let upload_body: serde_json::Value = upload_res.json().await.unwrap();
    let url = upload_body["url"].as_str().unwrap();
    assert!(url.starts_with("/api/v1/files/"));
    assert!(url.ends_with("/content"));
    assert!(!url.contains("X-Amz-"));
    assert!(!url.starts_with("http://"));
    assert!(!url.starts_with("https://"));

    let file_id = upload_body["id"].as_str().unwrap();
    let download_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v1/files/{}/download",
            &ctx.app.address, file_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, download_res.status().as_u16());
    let download_body: serde_json::Value = download_res.json().await.unwrap();
    assert_eq!(url, download_body["url"].as_str().unwrap());
}

#[tokio::test]
async fn native_presign_endpoint_is_explicitly_unsupported() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let presign_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v1/files/presign", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "filename": "direct.txt",
            "content_type": "text/plain",
            "channel_id": channel_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(501, presign_res.status().as_u16());
    let body: serde_json::Value = presign_res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "NOT_IMPLEMENTED");
    assert!(body.get("upload_url").is_none());
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

    let _first_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Before" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, _first_res.status().as_u16());

    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Anchor" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap().to_string();
    let post_uuid = parse_mm_or_uuid(&post_id).unwrap();

    let _third_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "After" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, _third_res.status().as_u16());

    let seq: i64 = sqlx::query_scalar("SELECT seq FROM posts WHERE id = $1 AND deleted_at IS NULL")
        .bind(post_uuid)
        .fetch_one(&ctx.app.db_pool)
        .await
        .unwrap_or(0);

    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM posts WHERE channel_id = $1 AND deleted_at IS NULL",
    )
    .bind(channel_id)
    .fetch_one(&ctx.app.db_pool)
    .await
    .unwrap_or(0);
    let unread_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM posts WHERE channel_id = $1 AND deleted_at IS NULL AND seq > $2",
    )
    .bind(channel_id)
    .bind((seq - 1).max(0))
    .fetch_one(&ctx.app.db_pool)
    .await
    .unwrap_or(0);
    let expected_read_position = (total_count - unread_count).max(0);

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
    assert_eq!(unread_body["msg_count"], expected_read_position);
}

#[tokio::test]
async fn mm_set_unread_reply_when_crt_disabled_zeros_root_unread_and_updates_thread_membership() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    sqlx::query(
        r#"
        INSERT INTO mattermost_preferences (user_id, category, name, value)
        VALUES ($1, 'display_settings', 'collapsed_reply_threads', 'off')
        ON CONFLICT (user_id, category, name) DO UPDATE SET value = 'off'
        "#,
    )
    .bind(ctx.user_uuid)
    .execute(&ctx.app.db_pool)
    .await
    .unwrap();

    let root_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "root" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, root_res.status().as_u16());
    let root_body: serde_json::Value = root_res.json().await.unwrap();
    let root_id = root_body["id"].as_str().unwrap().to_string();
    let root_uuid = parse_mm_or_uuid(&root_id).unwrap();

    let reply1_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "reply one",
            "root_id": root_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, reply1_res.status().as_u16());
    let reply1_body: serde_json::Value = reply1_res.json().await.unwrap();
    let reply1_id = reply1_body["id"].as_str().unwrap().to_string();

    let reply2_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "reply two",
            "root_id": root_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, reply2_res.status().as_u16());

    let unread_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/users/{}/posts/{}/set_unread",
            &ctx.app.address, ctx.user_id, reply1_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "collapsed_threads_supported": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, unread_res.status().as_u16());
    let unread_body: serde_json::Value = unread_res.json().await.unwrap();

    assert_eq!(unread_body["mention_count_root"], 0);
    assert_eq!(unread_body["urgent_mention_count"], 0);
    assert_eq!(unread_body["msg_count_root"], 1);

    let membership_row: (bool, i32, i32) = sqlx::query_as(
        r#"
        SELECT following, mention_count, unread_replies_count
        FROM thread_memberships
        WHERE user_id = $1 AND post_id = $2
        "#,
    )
    .bind(ctx.user_uuid)
    .bind(root_uuid)
    .fetch_one(&ctx.app.db_pool)
    .await
    .unwrap();

    assert!(membership_row.0);
    assert_eq!(membership_row.1, 0);
    assert_eq!(membership_row.2, 2);
}

#[tokio::test]
async fn mm_channel_posts_include_file_metadata_for_mobile_history() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let png_1x1: Vec<u8> = vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 255, 255, 63, 0,
        5, 254, 2, 254, 167, 100, 129, 165, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let part = reqwest::multipart::Part::bytes(png_1x1)
        .file_name("photo.png")
        .mime_str("image/png")
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
    assert!(upload_res.status().is_success());
    let upload_body: serde_json::Value = upload_res.json().await.unwrap();
    let file_id = upload_body["file_infos"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "Image post",
            "file_ids": [file_id]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap().to_string();

    let reaction_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/reactions", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "user_id": ctx.user_id,
            "post_id": post_id,
            "emoji_name": "thumbsup"
        }))
        .send()
        .await
        .unwrap();
    assert!(
        reaction_res.status().as_u16() == 200 || reaction_res.status().as_u16() == 201,
        "unexpected reaction status: {}",
        reaction_res.status()
    );

    let posts_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/posts?page=0&per_page=30",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, posts_res.status().as_u16());

    let posts_body: serde_json::Value = posts_res.json().await.unwrap();
    let post_entry = &posts_body["posts"][post_id.as_str()];
    let files = post_entry["metadata"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["id"], upload_body["file_infos"][0]["id"]);

    let reactions = post_entry["metadata"]["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji_name"], "thumbsup");
}
