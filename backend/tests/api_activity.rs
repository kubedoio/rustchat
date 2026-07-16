#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use serde_json::Value;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_get_activity_feed_requires_auth() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .get(format!("{}/api/v4/users/some-id/activity", app.address))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Expected 401 Unauthorized, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_get_activity_feed_returns_activities() {
    let app = spawn_app().await;

    // 1. Create Organization
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org Activity")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // 2. Register User1 (the activity recipient)
    let user1_data = serde_json::json!({
        "username": format!("actuser1_{}", Uuid::new_v4().to_string().replace('-', "")[..8].to_string()),
        "email": format!("actuser1_{}@example.com", Uuid::new_v4()),
        "password": "Password123!",
        "display_name": "Activity User 1",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user1_data)
        .send()
        .await
        .expect("Failed to register user1");

    let login1_data = serde_json::json!({
        "email": user1_data["email"],
        "password": "Password123!"
    });

    let login1_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login1_data)
        .send()
        .await
        .expect("Failed to login user1");

    let login1_body: Value = login1_res.json().await.unwrap();
    let token1 = login1_body["token"].as_str().unwrap().to_string();
    let user1_id_str = login1_body["user"]["id"].as_str().unwrap().to_string();
    let user1_uuid = Uuid::parse_str(&user1_id_str).unwrap();

    // 3. Register User2 (the actor/mention sender)
    let user2_data = serde_json::json!({
        "username": format!("actuser2_{}", Uuid::new_v4().to_string().replace('-', "")[..8].to_string()),
        "email": format!("actuser2_{}@example.com", Uuid::new_v4()),
        "password": "Password123!",
        "display_name": "Activity User 2",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user2_data)
        .send()
        .await
        .expect("Failed to register user2");

    let login2_data = serde_json::json!({
        "email": user2_data["email"],
        "password": "Password123!"
    });

    let login2_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login2_data)
        .send()
        .await
        .expect("Failed to login user2");

    let login2_body: Value = login2_res.json().await.unwrap();
    let user2_uuid = Uuid::parse_str(login2_body["user"]["id"].as_str().unwrap()).unwrap();

    // 4. Create Team
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("act-team-{}", Uuid::new_v4()))
    .bind("Activity Test Team")
    .bind(true)
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert team");

    // Add both users to team
    for uid in [user1_uuid, user2_uuid] {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(team_id)
            .bind(uid)
            .bind("member")
            .execute(&app.db_pool)
            .await
            .expect("Failed to add user to team");
    }

    // 5. Create Channel
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, $5::channel_type, $6)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(format!("act-channel-{}", Uuid::new_v4()))
    .bind("Activity Test Channel")
    .bind("public")
    .bind(user1_uuid)
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert channel");

    // Add both users to channel
    for uid in [user1_uuid, user2_uuid] {
        sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(channel_id)
            .bind(uid)
            .bind("member")
            .execute(&app.db_pool)
            .await
            .expect("Failed to add user to channel");
    }

    // 6. Insert a fake post (from user2)
    let post_id = Uuid::new_v4();
    sqlx::query("INSERT INTO posts (id, channel_id, user_id, message) VALUES ($1, $2, $3, $4)")
        .bind(post_id)
        .bind(channel_id)
        .bind(user2_uuid)
        .bind("Hello @user1, check this out!")
        .execute(&app.db_pool)
        .await
        .expect("Failed to insert post");

    // 7. Insert activity directly (mention of user1 by user2)
    sqlx::query(
        "INSERT INTO activities (user_id, type, actor_id, channel_id, team_id, post_id, message_text, read) \
         VALUES ($1, 'mention', $2, $3, $4, $5, $6, false)",
    )
    .bind(user1_uuid)
    .bind(user2_uuid)
    .bind(channel_id)
    .bind(team_id)
    .bind(post_id)
    .bind("Hello @user1, check this out!")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert activity");

    // 8. GET /api/v4/users/{user1_id}/activity as user1
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/activity",
            app.address, user1_uuid
        ))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .expect("Failed to get activity feed");

    assert_eq!(
        response.status().as_u16(),
        200,
        "Expected 200 OK, got {}",
        response.status()
    );

    let body: Value = response.json().await.unwrap();
    let order = body["order"].as_array().unwrap();
    assert!(!order.is_empty(), "Expected non-empty order array");

    let unread_count = body["unread_count"].as_i64().unwrap_or(0);
    assert!(
        unread_count > 0,
        "Expected unread_count > 0, got {}",
        unread_count
    );
}

#[tokio::test]
async fn test_cannot_view_other_users_activity() {
    let app = spawn_app().await;

    // 1. Create Organization
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org ACL")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // 2. Register User1
    let user1_data = serde_json::json!({
        "username": format!("acluser1_{}", Uuid::new_v4().to_string().replace('-', "")[..8].to_string()),
        "email": format!("acluser1_{}@example.com", Uuid::new_v4()),
        "password": "Password123!",
        "display_name": "ACL User 1",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user1_data)
        .send()
        .await
        .expect("Failed to register user1");

    let login1_data = serde_json::json!({
        "email": user1_data["email"],
        "password": "Password123!"
    });

    let login1_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login1_data)
        .send()
        .await
        .expect("Failed to login user1");

    let login1_body: Value = login1_res.json().await.unwrap();
    let token1 = login1_body["token"].as_str().unwrap().to_string();

    // 3. Register User2
    let user2_data = serde_json::json!({
        "username": format!("acluser2_{}", Uuid::new_v4().to_string().replace('-', "")[..8].to_string()),
        "email": format!("acluser2_{}@example.com", Uuid::new_v4()),
        "password": "Password123!",
        "display_name": "ACL User 2",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user2_data)
        .send()
        .await
        .expect("Failed to register user2");

    let login2_data = serde_json::json!({
        "email": user2_data["email"],
        "password": "Password123!"
    });

    let login2_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login2_data)
        .send()
        .await
        .expect("Failed to login user2");

    let login2_body: Value = login2_res.json().await.unwrap();
    let user2_uuid = Uuid::parse_str(login2_body["user"]["id"].as_str().unwrap()).unwrap();

    // 4. Try to access user2's activity feed as user1 - should be 403
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/activity",
            app.address, user2_uuid
        ))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(
        response.status().as_u16(),
        403,
        "Expected 403 Forbidden, got {}",
        response.status()
    );
}
