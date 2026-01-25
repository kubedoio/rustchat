use crate::common::spawn_app;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_admin_health_check() {
    let app = spawn_app().await;

    // 1. Register a user
    let user_data = json!({
        "username": "adminuser",
        "email": "admin@example.com",
        "password": "Password123!",
        "display_name": "Admin User"
    });

    let reg_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // We expect success or failure if already exists?
    // Since it's a fresh DB per test (random name), it should succeed.
    assert!(reg_res.status().is_success());

    // 2. Promote user to system_admin
    // We need the user ID or we can just update by email
    sqlx::query("UPDATE users SET role = 'system_admin' WHERE email = $1")
        .bind("admin@example.com")
        .execute(&app.db_pool)
        .await
        .expect("Failed to promote user");

    // 3. Login to get session/cookies
    let login_data = json!({
        "email": "admin@example.com",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert!(login_res.status().is_success());
    let login_body: serde_json::Value = login_res
        .json()
        .await
        .expect("Failed to parse login response");
    let token = login_body["token"]
        .as_str()
        .expect("Token missing in response");

    // 4. Check admin health endpoint
    let health_res = app
        .api_client
        .get(&format!("{}/api/v1/admin/health", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get health");

    assert_eq!(200, health_res.status().as_u16());

    let body: serde_json::Value = health_res
        .json()
        .await
        .expect("Failed to parse health response");

    // 5. Verify structure
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["websocket"]["active_connections"], 0);
    assert_eq!(body["storage"]["connected"], true);
    assert!(body["database"]["connected"].as_bool().unwrap());
}
