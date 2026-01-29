use crate::common::spawn_app;
use rustchat::mattermost_compat::id::encode_mm_id;
use serde_json::Value;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn get_channel_posts_returns_200() {
    let app = spawn_app().await;

    // 1. Create Organization
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // 2. Register & Login User
    let user_data = serde_json::json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "Password123!",
        "display_name": "Test User",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = serde_json::json!({
        "email": "test@example.com",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login.");

    let login_body: Value = login_res.json().await.unwrap();
    let token = login_body["token"].as_str().unwrap();
    let user_id = login_body["user"]["id"].as_str().unwrap();

    // 3. Create Team
    let team_id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, name, display_name, type, allow_open_invite, organization_id) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(team_id)
        .bind("test-team")
        .bind("Test Team")
        .bind("O")
        .bind(true)
        .bind(org_id)
        .execute(&app.db_pool)
        .await
        .expect("Failed to insert team");

    // Add user to team
    sqlx::query("INSERT INTO team_members (team_id, user_id, roles) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(Uuid::parse_str(user_id).unwrap())
        .bind("team_user")
        .execute(&app.db_pool)
        .await
        .expect("Failed to add user to team");

    // 4. Create Channel
    let channel_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(channel_id)
        .bind(team_id)
        .bind("test-channel")
        .bind("Test Channel")
        .bind("O")
        .bind(Uuid::parse_str(user_id).unwrap())
        .execute(&app.db_pool)
        .await
        .expect("Failed to insert channel");

    // Add user to channel
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, roles) VALUES ($1, $2, $3)")
        .bind(channel_id)
        .bind(Uuid::parse_str(user_id).unwrap())
        .bind("channel_user")
        .execute(&app.db_pool)
        .await
        .expect("Failed to add user to channel");

    // 5. Create Post
    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts (id, channel_id, user_id, message) VALUES ($1, $2, $3, $4)")
        .bind(post_id)
        .bind(channel_id)
        .bind(Uuid::parse_str(user_id).unwrap())
        .bind("Hello World")
        .execute(&app.db_pool)
        .await
        .expect("Failed to insert post");

    // 6. Call GET /api/v4/channels/{channel_id}/posts
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/posts?page=0&per_page=30",
            &app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get posts");

    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK, got {}",
        response.status()
    );

    let body: Value = response.json().await.unwrap();
    let posts = body["posts"].as_object().unwrap();
    let post_key = encode_mm_id(post_id);
    assert!(posts.contains_key(&post_key), "Post not found in response");

    let post = &posts[&post_key];
    let reply_count = post["reply_count"]
        .as_i64()
        .expect("reply_count should be a number");
    assert_eq!(reply_count, 0);
}
