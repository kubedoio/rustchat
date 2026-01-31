use crate::common::spawn_app;
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
        "username": "mmpostextra",
        "email": "mmpostextra@example.com",
        "password": "Password123!",
        "display_name": "MM Post Extra",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "mmpostextra@example.com",
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

async fn create_post(ctx: &TestContext, channel_id: Uuid) -> String {
    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Hello" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    post_body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn mm_posts_extra_routes() {
    let ctx = setup_mm_user().await;
    let (_team_id, channel_id) = setup_team_channel(&ctx).await;
    let post_id = create_post(&ctx, channel_id).await;

    let ids_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/ids", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!([post_id]))
        .send()
        .await
        .unwrap();
    assert_eq!(200, ids_res.status().as_u16());
    let ids_body: serde_json::Value = ids_res.json().await.unwrap();
    assert_eq!(ids_body.as_array().unwrap().len(), 1);

    let reaction_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/reactions", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "user_id": ctx.user_id, "post_id": post_id, "emoji_name": "thumbsup" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, reaction_res.status().as_u16());

    let bulk_reactions = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/ids/reactions", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!([post_id]))
        .send()
        .await
        .unwrap();
    assert_eq!(200, bulk_reactions.status().as_u16());
    let bulk_body: serde_json::Value = bulk_reactions.json().await.unwrap();
    assert!(bulk_body.as_object().unwrap().len() >= 1);

    let pin_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/{}/pin", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, pin_res.status().as_u16());

    let is_pinned: bool = sqlx::query_scalar("SELECT is_pinned FROM posts WHERE id = $1")
        .bind(parse_mm_or_uuid(&post_id).unwrap())
        .fetch_one(&ctx.app.db_pool)
        .await
        .unwrap_or(false);
    assert!(is_pinned);

    let unpin_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/{}/unpin", &ctx.app.address, post_id))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, unpin_res.status().as_u16());

    let action_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/posts/{}/actions/123",
            &ctx.app.address, post_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "data": "ok" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, action_res.status().as_u16());

    let rewrite_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/rewrite", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({ "message": "hello", "agent_id": "", "action": "summarize" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, rewrite_res.status().as_u16());
    let rewrite_body: serde_json::Value = rewrite_res.json().await.unwrap();
    assert_eq!(rewrite_body["rewritten_text"], "hello");

    let scheduled_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts/schedule", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "scheduled",
            "scheduled_at": chrono::Utc::now().timestamp_millis() + 60000
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, scheduled_res.status().as_u16());
    let scheduled_body: serde_json::Value = scheduled_res.json().await.unwrap();
    let scheduled_id = scheduled_body["id"].as_str().unwrap();

    let update_res = ctx
        .app
        .api_client
        .put(format!(
            "{}/api/v4/posts/schedule/{}",
            &ctx.app.address, scheduled_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .json(&json!({
            "id": scheduled_id,
            "channel_id": channel_id.to_string(),
            "user_id": ctx.user_id,
            "message": "scheduled update",
            "scheduled_at": chrono::Utc::now().timestamp_millis() + 120000
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, update_res.status().as_u16());
}
