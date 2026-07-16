#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn register_user_success() {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Test Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = serde_json::json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "Password123!",
        "display_name": "Test User",
        "org_id": org_id
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn register_user_success_sets_auth_cookie() {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Cookie Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = serde_json::json!({
        "username": "cookieuser",
        "email": "cookie@example.com",
        "password": "Password123!",
        "display_name": "Cookie User",
        "org_id": org_id
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let set_cookie = response
        .headers()
        .get("set-cookie")
        .expect("Register auto-login response should include a Set-Cookie header");
    let cookie_str = set_cookie
        .to_str()
        .expect("Set-Cookie should be valid ASCII");
    assert!(
        cookie_str.contains("MMAUTHTOKEN="),
        "Set-Cookie header should set the MMAUTHTOKEN cookie: {}",
        cookie_str
    );
    assert!(
        cookie_str.contains("HttpOnly"),
        "MMAUTHTOKEN cookie should be HttpOnly: {}",
        cookie_str
    );

    let body: serde_json::Value = response.json().await.expect("Failed to read JSON");
    assert!(body.get("token").is_some());
    assert_eq!(
        body.get("requires_password_setup"),
        Some(&serde_json::json!(false))
    );
}

#[tokio::test]
async fn login_user_success() {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Login Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    // Register first
    let user_data = serde_json::json!({
        "username": "loginuser",
        "email": "login@example.com",
        "password": "Password123!",
        "display_name": "Login User",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    // Login
    let login_data = serde_json::json!({
        "email": "login@example.com",
        "password": "Password123!"
    });

    let response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    let body: serde_json::Value = response.json().await.expect("Failed to read JSON");
    assert!(body.get("token").is_some());
}
