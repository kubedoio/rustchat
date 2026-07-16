#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use chrono::Utc;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_sidebar_categories() {
    let app = spawn_app().await;

    // 1. Register
    let user_data = json!({
        "username": "catuser",
        "email": "cat@example.com",
        "password": "Password123!",
        "display_name": "Category User"
    });

    app.api_client
        .post(&format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // 2. Update user role to org_admin to allow team creation
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE username = 'catuser'")
        .execute(&app.db_pool)
        .await
        .expect("Failed to update user role");

    // 3. Login (v1)
    let login_data = json!({
        "email": "cat@example.com",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(&format!("{}/api/v1/auth/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(200, login_res.status().as_u16());
    let login_body: serde_json::Value = login_res.json().await.unwrap();
    let token = login_body["token"]
        .as_str()
        .expect("missing token")
        .to_string();
    let _user_id = login_body["user"]["id"].as_str().expect("missing user id");
    assert_eq!(login_body["user"]["role"], "org_admin");

    // 4. Create a team (v1)
    let team_data = json!({
        "name": "cat-team",
        "display_name": "Category Team",
        "description": "Test Team"
    });

    let team_res = app
        .api_client
        .post(&format!("{}/api/v1/teams", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&team_data)
        .send()
        .await
        .expect("Failed to create team");

    assert_eq!(200, team_res.status().as_u16());
    let team: serde_json::Value = team_res.json().await.unwrap();
    let team_id = team["id"].as_str().unwrap();

    // 5. Get categories (Default)
    let get_res = app
        .api_client
        .get(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get categories");

    assert_eq!(200, get_res.status().as_u16());
    let categories: serde_json::Value = get_res.json().await.unwrap();
    assert!(!categories["categories"].as_array().unwrap().is_empty());
    assert_eq!(categories["categories"][0]["display_name"], "Channels");

    // 6. Create a category
    let create_cat_data = json!({
        "display_name": "My Custom Category",
        "type": "custom"
    });

    let create_res = app
        .api_client
        .post(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&create_cat_data)
        .send()
        .await
        .expect("Failed to create category");

    assert_eq!(200, create_res.status().as_u16());
    let new_cat: serde_json::Value = create_res.json().await.unwrap();
    assert_eq!(new_cat["display_name"], "My Custom Category");
    let cat_id = new_cat["id"].as_str().unwrap();

    // 7. Update categories (assign a channel)
    // Create a channel (v1)
    let channel_data = json!({
        "name": "cat-channel",
        "display_name": "Category Channel",
        "type": "public"
    });
    let mut channel_data_with_team = channel_data.as_object().unwrap().clone();
    channel_data_with_team.insert("team_id".to_string(), json!(team_id));

    let chan_res = app
        .api_client
        .post(&format!("{}/api/v1/channels", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&channel_data_with_team)
        .send()
        .await
        .unwrap();
    assert_eq!(200, chan_res.status().as_u16());
    let channel: serde_json::Value = chan_res.json().await.unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    let mut updated_cat = new_cat.clone();
    updated_cat["channel_ids"] = json!([channel_id]);

    let update_res = app
        .api_client
        .put(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!([updated_cat]))
        .send()
        .await
        .unwrap();

    assert_eq!(200, update_res.status().as_u16());
    let update_body: serde_json::Value = update_res.json().await.unwrap();
    let expected_channel_id = encode_mm_id(Uuid::parse_str(channel_id).unwrap());
    assert_eq!(update_body[0]["channel_ids"][0], expected_channel_id);

    let mut wrapped_cat = new_cat.clone();
    wrapped_cat["channel_ids"] = json!([channel_id]);
    let wrapped_update_res = app
        .api_client
        .put(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "categories": [wrapped_cat] }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, wrapped_update_res.status().as_u16());

    // 8. Update category order
    let order_data = json!([cat_id]);
    let order_res = app
        .api_client
        .put(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories/order",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&order_data)
        .send()
        .await
        .unwrap();

    assert_eq!(200, order_res.status().as_u16());

    let order_get_res = app
        .api_client
        .get(&format!(
            "{}/api/v4/users/me/teams/{}/channels/categories/order",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, order_get_res.status().as_u16());
    let order_get_body: serde_json::Value = order_get_res.json().await.unwrap();
    assert!(order_get_body.as_array().is_some());
}

#[tokio::test]
async fn get_categories_backfills_orphaned_channels() {
    let app = spawn_app().await;

    // 1. Register
    let user_data = json!({
        "username": "catorphan",
        "email": "catorphan@example.com",
        "password": "Password123!",
        "display_name": "Category Orphan"
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // 2. Update user role to org_admin
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE username = 'catorphan'")
        .execute(&app.db_pool)
        .await
        .expect("Failed to update user role");

    // 3. Login
    let login_data = json!({
        "email": "catorphan@example.com",
        "password": "Password123!"
    });

    let login_res = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    assert_eq!(200, login_res.status().as_u16());
    let login_body: serde_json::Value = login_res.json().await.expect("invalid login body");
    let token = login_body["token"]
        .as_str()
        .expect("missing token")
        .to_string();
    let user_id = login_body["user"]["id"].as_str().expect("missing user id");
    let user_uuid = parse_mm_or_uuid(user_id).expect("invalid user id");

    // 4. Create Team
    let team_data = json!({
        "name": "cat-orphan-team",
        "display_name": "Category Orphan Team",
        "description": "Test Team"
    });
    let team_res = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&team_data)
        .send()
        .await
        .expect("Failed to create team");
    assert_eq!(200, team_res.status().as_u16());
    let team: serde_json::Value = team_res.json().await.expect("invalid team body");
    let team_id = team["id"].as_str().expect("missing team id");
    let team_uuid = parse_mm_or_uuid(team_id).expect("invalid team id");

    // 5. Create Channel
    let channel_data = json!({
        "name": "cat-orphan-channel",
        "display_name": "Category Orphan Channel",
        "type": "public",
        "team_id": team_id
    });
    let channel_res = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&channel_data)
        .send()
        .await
        .expect("Failed to create channel");
    assert_eq!(200, channel_res.status().as_u16());
    let channel: serde_json::Value = channel_res.json().await.expect("invalid channel body");
    let channel_id = channel["id"].as_str().expect("missing channel id");
    let channel_mm_id = encode_mm_id(Uuid::parse_str(channel_id).expect("invalid raw channel id"));

    // 6. Persist a broken category row
    let category_id = Uuid::new_v4();
    let now = Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO channel_categories (
            id, team_id, user_id, type, display_name, sorting, muted, collapsed, sort_order, create_at, update_at, delete_at
        ) VALUES ($1, $2, $3, 'channels', 'Channels', 'alpha', false, false, 0, $4, $4, 0)
        "#,
    )
    .bind(category_id)
    .bind(team_uuid)
    .bind(user_uuid)
    .bind(now)
    .execute(&app.db_pool)
    .await
    .expect("Failed to insert category");

    // 7. Verify backfill
    let get_res = app
        .api_client
        .get(format!(
            "{}/api/v4/users/me/teams/{}/channels/categories",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get categories");
    assert_eq!(200, get_res.status().as_u16());
    let body: serde_json::Value = get_res.json().await.expect("invalid categories body");

    let categories = body["categories"]
        .as_array()
        .expect("categories should be an array");
    let broken_category = categories
        .iter()
        .find(|cat| cat["id"] == encode_mm_id(category_id))
        .expect("expected existing broken category");
    let channel_ids = broken_category["channel_ids"]
        .as_array()
        .expect("channel_ids should be array")
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect::<Vec<String>>();
    assert!(
        channel_ids.contains(&channel_mm_id),
        "orphaned team channel should be backfilled into category"
    );
}
