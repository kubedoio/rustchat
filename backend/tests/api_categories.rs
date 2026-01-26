use crate::common::spawn_app;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_sidebar_categories() {
    let app = spawn_app().await;

    // 1. Register and Login
    let user_data = json!({
        "username": "catuser",
        "email": "cat@example.com",
        "password": "Password123!",
        "display_name": "Category User"
    });

    app.api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    let login_data = json!({
        "login_id": "catuser",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(&format!("{}/api/v4/users/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(200, login_res.status().as_u16());
    let token = login_res.headers().get("Token").unwrap().to_str().unwrap().to_string();
    let user_info: serde_json::Value = login_res.json().await.unwrap();
    let _user_id = user_info["id"].as_str().unwrap();

    // 2. Create a team (v1)
    let team_data = json!({
        "name": "cat-team",
        "display_name": "Category Team",
        "description": "Test Team"
    });

    let team_res = app.api_client
        .post(&format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&team_data)
        .send()
        .await
        .expect("Failed to create team");

    assert_eq!(200, team_res.status().as_u16());
    let team: serde_json::Value = team_res.json().await.unwrap();
    let team_id = team["id"].as_str().unwrap();

    // 3. Get categories (Default)
    let get_res = app.api_client
        .get(&format!("{}/api/v4/users/me/teams/{}/channels/categories", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get categories");

    assert_eq!(200, get_res.status().as_u16());
    let categories: serde_json::Value = get_res.json().await.unwrap();
    assert!(!categories["categories"].as_array().unwrap().is_empty());
    assert_eq!(categories["categories"][0]["display_name"], "Channels");

    // 4. Create a category
    let create_cat_data = json!({
        "display_name": "My Custom Category",
        "type": "custom"
    });

    let create_res = app.api_client
        .post(&format!("{}/api/v4/users/me/teams/{}/channels/categories", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&create_cat_data)
        .send()
        .await
        .expect("Failed to create category");

    assert_eq!(200, create_res.status().as_u16());
    let new_cat: serde_json::Value = create_res.json().await.unwrap();
    assert_eq!(new_cat["display_name"], "My Custom Category");
    let cat_id = new_cat["id"].as_str().unwrap();

    // 5. Update categories (assign a channel)
    // Create a channel (v1)
    let channel_data = json!({
        "name": "cat-channel",
        "display_name": "Category Channel",
        "type": "public"
    });
    let chan_res = app.api_client
        .post(&format!("{}/api/v1/teams/{}/channels", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&channel_data)
        .send()
        .await
        .unwrap();
    assert_eq!(200, chan_res.status().as_u16());
    let channel: serde_json::Value = chan_res.json().await.unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    let mut updated_cat = new_cat.clone();
    updated_cat["channel_ids"] = json!([channel_id]);

    let update_res = app.api_client
        .put(&format!("{}/api/v4/users/me/teams/{}/channels/categories", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "categories": [updated_cat] }))
        .send()
        .await
        .unwrap();

    assert_eq!(200, update_res.status().as_u16());
    let update_body: serde_json::Value = update_res.json().await.unwrap();
    assert_eq!(update_body[0]["channel_ids"][0], channel_id);

    // 6. Update category order
    let order_data = json!([cat_id]);
    let order_res = app.api_client
        .put(&format!("{}/api/v4/users/me/teams/{}/channels/categories/order", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&order_data)
        .send()
        .await
        .unwrap();

    assert_eq!(200, order_res.status().as_u16());
}