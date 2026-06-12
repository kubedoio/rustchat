//! Security regression tests for /api/v4/files upload authorization.
//!
//! Ensures that channel membership is validated before any file chunk is
//! buffered to disk, so unauthorized requests cannot force temp-file I/O.

use crate::common::spawn_app;
use rustchat::mattermost_compat::id::parse_mm_or_uuid;
use serde_json::json;
use uuid::Uuid;

mod common;

struct TestContext {
    app: common::TestApp,
    token: String,
    user_uuid: Uuid,
    org_id: Uuid,
}

async fn setup_user() -> TestContext {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("File Security Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = json!({
        "username": "filesecurity",
        "email": "filesecurity@example.com",
        "password": "Password123!",
        "display_name": "File Security",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "filesecurity@example.com",
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
    let user_uuid = parse_mm_or_uuid(me_body["id"].as_str().unwrap()).unwrap();

    TestContext {
        app,
        token,
        user_uuid,
        org_id,
    }
}

async fn setup_team_channel(ctx: &TestContext) -> (Uuid, Uuid) {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'secteam', 'Security Team', true)",
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
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'secchannel', 'public')",
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

async fn register_intruder(ctx: &TestContext) -> String {
    let intruder_data = json!({
        "username": "fileintruder",
        "email": "fileintruder@example.com",
        "password": "Password123!",
        "display_name": "File Intruder",
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
            "login_id": "fileintruder@example.com",
            "password": "Password123!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, login_res.status().as_u16());

    login_res
        .headers()
        .get("Token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn file_field_before_channel_id_form_field_does_not_buffer_for_non_member() {
    let ctx = setup_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let intruder_token = register_intruder(&ctx).await;

    let part = reqwest::multipart::Part::bytes(b"sensitive data".to_vec())
        .file_name("secret.txt")
        .mime_str("text/plain")
        .unwrap();

    // Place the file field before channel_id in the multipart stream.
    let form = reqwest::multipart::Form::new()
        .part("files", part)
        .text("channel_id", channel_id.to_string());

    let upload_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/files", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", intruder_token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // Must fail before buffering any file chunks to disk.
    assert_eq!(400, upload_res.status().as_u16());

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE channel_id = $1 AND name = $2")
            .bind(channel_id)
            .bind("secret.txt")
            .fetch_one(&ctx.app.db_pool)
            .await
            .unwrap();
    assert_eq!(0, count);
}

#[tokio::test]
async fn upload_without_channel_id_is_rejected() {
    let ctx = setup_user().await;

    let part = reqwest::multipart::Part::bytes(b"orphan data".to_vec())
        .file_name("orphan.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("files", part);

    let upload_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/files", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(400, upload_res.status().as_u16());

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE name = $1")
        .bind("orphan.txt")
        .fetch_one(&ctx.app.db_pool)
        .await
        .unwrap();
    assert_eq!(0, count);
}

#[tokio::test]
async fn member_upload_with_channel_id_query_param_still_works() {
    let ctx = setup_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;

    let part = reqwest::multipart::Part::bytes(b"hello member".to_vec())
        .file_name("member.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("files", part);

    let upload_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/files?channel_id={}",
            &ctx.app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    let status = upload_res.status().as_u16();
    assert!(
        status == 200 || status == 201,
        "unexpected upload status: {}",
        status
    );

    let body: serde_json::Value = upload_res.json().await.unwrap();
    let file_id = body["file_infos"][0]["id"].as_str().unwrap();

    let info_res = ctx
        .app
        .api_client
        .get(format!("{}/api/v4/files/{}", &ctx.app.address, file_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert!(info_res.status().is_redirection() || info_res.status().is_success());
}

#[tokio::test]
async fn non_member_upload_with_channel_id_query_param_is_rejected() {
    let ctx = setup_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let intruder_token = register_intruder(&ctx).await;

    let part = reqwest::multipart::Part::bytes(b"private data".to_vec())
        .file_name("private.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("files", part);

    let upload_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/files?channel_id={}",
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
