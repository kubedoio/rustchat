#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::models::{UpdateUser, UserResponse};
use serde_json::json;
use sqlx::Executor;

mod common;

#[tokio::test]
async fn test_user_custom_status() {
    let app = spawn_app().await;

    // 1. Register and Login
    let user_data = json!({
        "username": "statususer",
        "email": "status@example.com",
        "password": "Password123!",
        "display_name": "Status User"
    });

    let _reg_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // We can get the user ID from the registration response usually,
    // or login to get it. Let's assume registration returns AuthResponse or UserResponse.
    // If we look at api/auth.rs, register returns AuthResponse.

    // Let's just login to be sure we have the token and user info.
    let login_data = json!({
        "email": "status@example.com",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(200, login_res.status().as_u16());
    let body: serde_json::Value = login_res
        .json()
        .await
        .expect("Failed to parse login response");
    let token = body["token"]
        .as_str()
        .expect("Missing auth token")
        .to_string();
    let user_id = body["user"]["id"].as_str().unwrap().to_string();

    // 2. Update Custom Status
    let status_payload = json!({
        "emoji": "🍔",
        "text": "Out for lunch",
        "expires_at": null
    });

    let update_data = UpdateUser {
        username: None,
        display_name: None,
        avatar_url: None,
        custom_status: Some(status_payload.clone()),
    };

    let update_res = app
        .api_client
        .put(&format!("{}/api/v1/users/{}", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&update_data)
        .send()
        .await
        .expect("Failed to update user");

    assert_eq!(200, update_res.status().as_u16());
    let updated_user: UserResponse = update_res
        .json()
        .await
        .expect("Failed to parse updated user");

    assert_eq!(updated_user.custom_status, Some(status_payload.clone()));

    // 3. Verify Persistence (Get User)
    let get_res = app
        .api_client
        .get(&format!("{}/api/v1/users/{}", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get user");

    assert_eq!(200, get_res.status().as_u16());
    let user: UserResponse = get_res.json().await.expect("Failed to parse user");

    assert_eq!(user.custom_status, Some(status_payload));
}

#[tokio::test]
async fn test_auth_me_normalizes_internal_presigned_avatar_urls() {
    let app = spawn_app().await;

    let register_data = json!({
        "username": "avataruser",
        "email": "avatar@example.com",
        "password": "Password123!",
        "display_name": "Avatar User"
    });

    let register_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&register_data)
        .send()
        .await
        .expect("Failed to register avatar user");

    assert_eq!(200, register_res.status().as_u16());
    let register_body: serde_json::Value = register_res
        .json()
        .await
        .expect("Failed to parse register response");
    let token = register_body["token"]
        .as_str()
        .expect("Missing auth token")
        .to_string();
    let user_id = register_body["user"]["id"]
        .as_str()
        .expect("Missing user id")
        .to_string();

    let stale_url = format!(
        "https://s3.example.com/rustchat-uploads/files/{}/legacy-avatar.jpg?x-id=GetObject&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Date=20260401T232501Z&X-Amz-Expires=3600",
        user_id
    );

    app.db_pool
        .execute(
            sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
                .bind(&stale_url)
                .bind(
                    uuid::Uuid::parse_str(&user_id)
                        .expect("Expected UUID user id for avatar normalization test"),
                ),
        )
        .await
        .expect("Failed to seed stale avatar URL");

    let me_res = app
        .api_client
        .get(&format!("{}/api/v1/auth/me", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to fetch current user");

    assert_eq!(200, me_res.status().as_u16());
    let me: UserResponse = me_res
        .json()
        .await
        .expect("Failed to parse /auth/me response");
    assert_eq!(
        me.avatar_url,
        Some(format!("/api/v4/users/{}/image", user_id))
    );
}
