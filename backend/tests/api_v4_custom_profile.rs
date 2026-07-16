#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::mattermost_compat::id::encode_mm_id;
use serde_json::{json, Value};
use uuid::Uuid;

mod common;

#[tokio::test]
async fn custom_profile_fields_returns_200_and_mattermost_shape() {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Custom Profile Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = json!({
        "username": "cpuser",
        "email": "cpuser@example.com",
        "password": "Password123!",
        "display_name": "CP User",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    let login_data = json!({
        "login_id": "cpuser@example.com",
        "password": "Password123!"
    });

    let login_response = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");
    assert_eq!(200, login_response.status().as_u16());

    let token = login_response
        .headers()
        .get("Token")
        .expect("Missing Token header")
        .to_str()
        .expect("Invalid token header")
        .to_string();

    let field_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO custom_profile_fields (id, group_id, name, field_type, attrs, target_id, target_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(field_id)
    .bind("")
    .bind("Department")
    .bind("text")
    .bind(json!({"sort_order": 1}))
    .bind("")
    .bind("user")
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert custom profile field");

    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/custom_profile_attributes/fields",
            app.address
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to fetch custom profile fields");

    assert_eq!(
        200,
        response.status().as_u16(),
        "Expected 200 OK, got {}",
        response.status()
    );

    let body: Value = response
        .json()
        .await
        .expect("Failed to decode response body");
    let fields = body.as_array().expect("Expected array response");
    assert!(
        !fields.is_empty(),
        "Expected at least one custom profile field"
    );

    let expected_id = encode_mm_id(field_id);
    let field = fields
        .iter()
        .find(|f| f["id"] == expected_id)
        .expect("Inserted field not returned");

    assert_eq!(field["name"], "Department");
    assert_eq!(field["type"], "text");
    assert_eq!(field["target_type"], "user");
    assert!(field["create_at"].is_number());
    assert!(field["update_at"].is_number());
    assert_eq!(field["delete_at"], 0);
}
