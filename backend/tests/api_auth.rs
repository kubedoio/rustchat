mod common;
use common::spawn_app;
use serde_json::json;

#[tokio::test]
async fn register_user_success() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // We need a unique email to avoid collision if DB isn't cleaned
    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());

    let body = json!({
        "username": "testuser",
        "email": email,
        "password": "password123"
    });

    let response = client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(201, response.status().as_u16());
}

#[tokio::test]
async fn login_success() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let email = format!("login_{}@example.com", uuid::Uuid::new_v4());
    let password = "password123";

    // 1. Register first
    let register_body = json!({
        "username": "loginuser",
        "email": email,
        "password": password
    });

    client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&register_body)
        .send()
        .await
        .expect("Failed to register user");

    // 2. Login
    let login_body = json!({
        "email": email,
        "password": password
    });

    let response = client
        .post(&format!("{}/api/v1/auth/login", &app.address))
        .json(&login_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    // Verify we got a token
    let json: serde_json::Value = response.json().await.expect("Failed to read JSON");
    assert!(json.get("token").is_some());
}
