use crate::common::spawn_app;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use uuid::Uuid;

mod common;

async fn setup_mm_user() -> (common::TestApp, String, String, Uuid, Uuid) {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = json!({
        "username": "mmthreads",
        "email": "mmthreads@example.com",
        "password": "Password123!",
        "display_name": "MM Threads",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "mmthreads@example.com",
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
    let token = response
        .headers()
        .get("Token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let me_res = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    let me_body: serde_json::Value = me_res.json().await.unwrap();
    let user_id = me_body["id"].as_str().unwrap().to_string();
    let user_uuid = parse_mm_or_uuid(&user_id).unwrap();

    (app, token, user_id, user_uuid, org_id)
}

#[tokio::test]
async fn mm_threads_endpoints_smoke() {
    let (app, token, user_id, user_uuid, org_id) = setup_mm_user().await;

    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'mmteam', 'MM Team', true)",
    )
    .bind(team_id)
    .bind(org_id)
    .execute(&app.db_pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(user_uuid)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let channel_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')")
        .bind(channel_id)
        .bind(team_id)
        .execute(&app.db_pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
        .bind(channel_id)
        .bind(user_uuid)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let post_res = app
        .api_client
        .post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Root" }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let root_post_id = post_body["id"].as_str().unwrap().to_string();

    let reply_res = app
        .api_client
        .post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "channel_id": channel_id.to_string(), "message": "Reply", "root_id": root_post_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(200, reply_res.status().as_u16());
    let reply_body: serde_json::Value = reply_res.json().await.unwrap();
    let reply_id = reply_body["id"].as_str().unwrap().to_string();

    let follow_res = app
        .api_client
        .put(format!(
            "{}/api/v4/users/{}/teams/{}/threads/{}/following",
            &app.address, user_id, team_id, root_post_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, follow_res.status().as_u16());

    sqlx::query("UPDATE thread_memberships SET mention_count = 2 WHERE user_id = $1 AND post_id = $2")
        .bind(user_uuid)
        .bind(parse_mm_or_uuid(&root_post_id).unwrap())
        .execute(&app.db_pool)
        .await
        .unwrap();

    let list_res = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/teams/{}/threads",
            &app.address, user_id, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, list_res.status().as_u16());
    let list_body: serde_json::Value = list_res.json().await.unwrap();
    let threads = list_body["threads"].as_array().unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["id"], encode_mm_id(parse_mm_or_uuid(&root_post_id).unwrap()));

    let mention_res = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/teams/{}/threads/mention_counts",
            &app.address, user_id, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, mention_res.status().as_u16());
    let mention_body: serde_json::Value = mention_res.json().await.unwrap();
    let channel_key = encode_mm_id(channel_id);
    assert_eq!(mention_body[&channel_key], 2);

    let unread_res = app
        .api_client
        .post(format!(
            "{}/api/v4/users/{}/teams/{}/threads/{}/set_unread/{}",
            &app.address, user_id, team_id, root_post_id, reply_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, unread_res.status().as_u16());
    let unread_body: serde_json::Value = unread_res.json().await.unwrap();
    assert!(unread_body["unread_replies"].as_i64().unwrap_or(0) >= 1);

    let read_all_res = app
        .api_client
        .put(format!(
            "{}/api/v4/users/{}/teams/{}/threads/read",
            &app.address, user_id, team_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, read_all_res.status().as_u16());
    let read_all_body: serde_json::Value = read_all_res.json().await.unwrap();
    assert_eq!(read_all_body["status"], "OK");
}

#[tokio::test]
async fn mm_preferences_endpoints_smoke() {
    let (app, token, user_id, _user_uuid, _org_id) = setup_mm_user().await;

    let pref_put = app
        .api_client
        .put(format!("{}/api/v4/users/{}/preferences", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!([
            { "user_id": user_id, "category": "display", "name": "theme", "value": "dark" },
            { "user_id": user_id, "category": "tutorial", "name": "step", "value": "1" }
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(200, pref_put.status().as_u16());

    let pref_get = app
        .api_client
        .get(format!("{}/api/v4/users/{}/preferences", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let prefs: serde_json::Value = pref_get.json().await.unwrap();
    assert_eq!(prefs.as_array().unwrap().len(), 2);

    let pref_cat = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/preferences/display",
            &app.address, user_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let cat_body: serde_json::Value = pref_cat.json().await.unwrap();
    assert_eq!(cat_body.as_array().unwrap().len(), 1);
    assert_eq!(cat_body[0]["name"], "theme");

    let pref_name = app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/preferences/display/name/theme",
            &app.address, user_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let name_body: serde_json::Value = pref_name.json().await.unwrap();
    assert_eq!(name_body["value"], "dark");

    let pref_delete = app
        .api_client
        .post(format!("{}/api/v4/users/{}/preferences/delete", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!([
            { "user_id": user_id, "category": "display", "name": "theme", "value": "dark" }
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(200, pref_delete.status().as_u16());

    let pref_get_after = app
        .api_client
        .get(format!("{}/api/v4/users/{}/preferences", &app.address, user_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    let prefs_after: serde_json::Value = pref_get_after.json().await.unwrap();
    assert_eq!(prefs_after.as_array().unwrap().len(), 1);
}
