//! Regression tests for v1 posts endpoints.

mod common;
use common::spawn_app;
use serde_json::Value;
use uuid::Uuid;

#[tokio::test]
async fn list_channel_posts_returns_messages_without_500() {
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
    let user_uuid = Uuid::parse_str(user_id).unwrap();

    // 3. Create Team
    let team_id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, $5)")
        .bind(team_id)
        .bind(org_id)
        .bind("test-team")
        .bind("Test Team")
        .bind(true)
        .execute(&app.db_pool)
        .await
        .expect("Failed to insert team");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(user_uuid)
        .bind("member")
        .execute(&app.db_pool)
        .await
        .expect("Failed to add user to team");

    // 4. Create Channel
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, $5::channel_type, $6)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind("test-channel")
    .bind("Test Channel")
    .bind("public")
    .bind(user_uuid)
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert channel");

    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, $3)")
        .bind(channel_id)
        .bind(user_uuid)
        .bind("member")
        .execute(&app.db_pool)
        .await
        .expect("Failed to add user to channel");

    // 5. Create two posts via API
    for i in 0..2 {
        let post_data = serde_json::json!({
            "message": format!("Message {}", i)
        });

        let post_res = app
            .api_client
            .post(format!(
                "{}/api/v1/channels/{}/posts",
                &app.address, channel_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .json(&post_data)
            .send()
            .await
            .expect("Failed to create post");

        assert_eq!(post_res.status().as_u16(), 200);
    }

    // 6. List posts (initial load)
    let list_res = app
        .api_client
        .get(format!(
            "{}/api/v1/channels/{}/posts",
            &app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list posts");

    assert_eq!(
        list_res.status().as_u16(),
        200,
        "list posts should succeed, not return 500"
    );

    let list_body: Value = list_res.json().await.unwrap();
    let messages = list_body["messages"]
        .as_array()
        .expect("Expected messages array");
    assert_eq!(messages.len(), 2, "Expected both posts in response");

    // 7. Load older posts via before cursor (regression guard for the same SELECT)
    let oldest_id = messages
        .last()
        .and_then(|m| m["id"].as_str())
        .expect("Expected oldest post id");

    let older_res = app
        .api_client
        .get(format!(
            "{}/api/v1/channels/{}/posts?before={}",
            &app.address, channel_id, oldest_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list older posts");

    assert_eq!(
        older_res.status().as_u16(),
        200,
        "older posts cursor should succeed, not return 500"
    );
}
