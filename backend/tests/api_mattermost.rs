use crate::common::spawn_app;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn mm_login_success() {
    let app = spawn_app().await;

    // Register a user via V1 first (or DB insert)
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = serde_json::json!({
        "username": "mmuser",
        "email": "mm@example.com",
        "password": "Password123!",
        "display_name": "MM User",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    // MM Login
    let login_data = serde_json::json!({
        "login_id": "mm@example.com",
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

    // Check Token header
    let token = response
        .headers()
        .get("Token")
        .expect("Missing Token header");
    let token_str = token.to_str().expect("Invalid Token header").to_string();
    assert!(!token_str.is_empty());

    let body: serde_json::Value = response.json().await.expect("Failed to read JSON");
    assert_eq!(body["username"], "mmuser");

    // Use token to get me
    let me_response = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {}", token_str))
        .send()
        .await
        .expect("Failed to get me");

    assert_eq!(200, me_response.status().as_u16());
    let me_body: serde_json::Value = me_response.json().await.expect("Failed to read JSON");
    assert_eq!(me_body["username"], "mmuser");

    // Check config
    let config_response = app
        .api_client
        .get(format!("{}/api/v4/config/client", &app.address))
        .send()
        .await
        .expect("Failed to get config");
    assert_eq!(200, config_response.status().as_u16());
    let config_body: serde_json::Value = config_response.json().await.expect("Failed to read JSON");
    assert_eq!(config_body["Version"], "10.0.0-rustchat");
}

#[tokio::test]
async fn mm_posts_flow() {
    let app = spawn_app().await;

    // Setup: Create Org, User, Team, Channel via DB or V1
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org 2")
        .execute(&app.db_pool)
        .await
        .unwrap();

    let user_data = serde_json::json!({
        "username": "mmuser2",
        "email": "mm2@example.com",
        "password": "Password123!",
        "display_name": "MM User 2",
        "org_id": org_id
    });

    // Register
    let reg_res = app
        .api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .unwrap();
    let reg_body: serde_json::Value = reg_res.json().await.unwrap();
    let user_id = reg_body["user"]["id"].as_str().unwrap();
    let token = reg_body["token"].as_str().unwrap(); // V1 token works too if we support Bearer

    // Create Team
    let team_id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'mmteam', 'MM Team', true)")
        .bind(team_id).bind(org_id).execute(&app.db_pool).await.unwrap();

    // Add user to team
    sqlx::query(
        "INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2::uuid, 'member')",
    )
    .bind(team_id)
    .bind(user_id)
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Create Channel
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')",
    )
    .bind(channel_id)
    .bind(team_id)
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Add user to channel
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2::uuid, 'member', '{}')")
        .bind(channel_id).bind(user_id).execute(&app.db_pool).await.unwrap();

    // Check my teams
    let teams_res = app
        .api_client
        .get(format!("{}/api/v4/users/me/teams", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, teams_res.status().as_u16());
    let teams_body: serde_json::Value = teams_res.json().await.unwrap();
    assert!(teams_body.as_array().unwrap().len() > 0);
    assert_eq!(teams_body[0]["id"], team_id.to_string());

    // Create Post via V4
    let post_data = serde_json::json!({
        "channel_id": channel_id.to_string(),
        "message": "Hello from MM Mobile"
    });

    let post_res = app
        .api_client
        .post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&post_data)
        .send()
        .await
        .unwrap();

    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    assert_eq!(post_body["message"], "Hello from MM Mobile");
    let post_id = post_body["id"].as_str().unwrap();

    // Fetch posts
    let posts_res = app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/posts",
            &app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(200, posts_res.status().as_u16());
    let posts_body: serde_json::Value = posts_res.json().await.unwrap();

    let order = posts_body["order"].as_array().unwrap();
    assert!(order.contains(&serde_json::Value::String(post_id.to_string())));
    assert_eq!(
        posts_body["posts"][post_id]["message"],
        "Hello from MM Mobile"
    );
}
