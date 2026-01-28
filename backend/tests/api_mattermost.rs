use crate::common::spawn_app;
use uuid::Uuid;
use rustchat::mattermost_compat::id::parse_mm_or_uuid;

mod common;

#[tokio::test]
async fn mm_login_and_features() {
    let app = spawn_app().await;

    // --- Setup: Create Org, User ---
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

    // --- Login ---
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
    let token = response.headers().get("Token").unwrap().to_str().unwrap().to_string();

    // --- Get Me ---
    let me_res = app.api_client.get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let me_body: serde_json::Value = me_res.json().await.unwrap();
    let user_id = me_body["id"].as_str().unwrap();
    let user_uuid = parse_mm_or_uuid(user_id).unwrap();

    // --- Device ---
    let device_res = app.api_client
        .post(format!("{}/api/v4/users/sessions/device", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "device_id": "d1", "token": "push1", "platform": "ios" }))
        .send().await.unwrap();
    assert_eq!(200, device_res.status().as_u16());

    // --- Preferences ---
    let pref_put = app.api_client
        .put(format!("{}/api/v4/users/me/preferences", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!([
            { "user_id": user_id, "category": "display", "name": "theme", "value": "dark" }
        ]))
        .send().await.unwrap();
    assert_eq!(200, pref_put.status().as_u16());

    let pref_get = app.api_client
        .get(format!("{}/api/v4/users/me/preferences", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let prefs: serde_json::Value = pref_get.json().await.unwrap();
    assert_eq!(prefs[0]["value"], "dark");

    // --- Status ---
    let status_put = app.api_client
        .put(format!("{}/api/v4/users/me/status", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "user_id": user_id, "status": "dnd" }))
        .send().await.unwrap();
    assert_eq!(200, status_put.status().as_u16());
    let status_body: serde_json::Value = status_put.json().await.unwrap();
    assert_eq!(status_body["status"], "dnd");

    // --- Team & Channel Setup ---
    let team_id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'mmteam', 'MM Team', true)")
        .bind(team_id).bind(org_id).execute(&app.db_pool).await.unwrap();
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2::uuid, 'member')")
        .bind(team_id).bind(user_uuid).execute(&app.db_pool).await.unwrap();

    let channel_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')")
        .bind(channel_id).bind(team_id).execute(&app.db_pool).await.unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2::uuid, 'member', '{}')")
        .bind(channel_id).bind(user_uuid).execute(&app.db_pool).await.unwrap();

    // --- Get Team ---
    let team_res = app.api_client.get(format!("{}/api/v4/teams/{}", &app.address, team_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, team_res.status().as_u16());

    // --- Get Channel ---
    let chan_res = app.api_client.get(format!("{}/api/v4/channels/{}", &app.address, channel_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, chan_res.status().as_u16());

    // --- View Channel ---
    let view_res = app.api_client.post(format!("{}/api/v4/channels/members/me/view", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "channel_id": channel_id.to_string() }))
        .send().await.unwrap();
    assert_eq!(200, view_res.status().as_u16());

    // --- Posts & Threads ---
    // Create Post
    let post_res = app.api_client.post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "channel_id": channel_id.to_string(), "message": "Test Post" }))
        .send().await.unwrap();
    assert_eq!(200, post_res.status().as_u16());
    let post_body: serde_json::Value = post_res.json().await.unwrap();
    let post_id = post_body["id"].as_str().unwrap();

    // Create Reply
    let reply_res = app.api_client.post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "channel_id": channel_id.to_string(), "message": "Reply", "root_id": post_id }))
        .send().await.unwrap();
    assert_eq!(200, reply_res.status().as_u16());

    // Get Thread
    let thread_res = app.api_client.get(format!("{}/api/v4/posts/{}/thread", &app.address, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, thread_res.status().as_u16());
    let thread_body: serde_json::Value = thread_res.json().await.unwrap();
    assert!(thread_body["order"].as_array().unwrap().len() >= 2);

    // Patch Post
    let patch_res = app.api_client.put(format!("{}/api/v4/posts/{}/patch", &app.address, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "message": "Edited Post" }))
        .send().await.unwrap();
    assert_eq!(200, patch_res.status().as_u16());
    let patch_body: serde_json::Value = patch_res.json().await.unwrap();
    assert_eq!(patch_body["message"], "Edited Post");

    // Add Reaction
    let react_res = app.api_client.post(format!("{}/api/v4/reactions", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "user_id": user_id, "post_id": post_id, "emoji_name": "smile" }))
        .send().await.unwrap();
    assert_eq!(200, react_res.status().as_u16());

    // Get Reactions
    let get_react_res = app.api_client.get(format!("{}/api/v4/posts/{}/reactions", &app.address, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let reactions: serde_json::Value = get_react_res.json().await.unwrap();
    assert_eq!(reactions[0]["emoji_name"], "smile");

    // Remove Reaction
    let del_react_res = app.api_client.delete(format!("{}/api/v4/users/me/posts/{}/reactions/smile", &app.address, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, del_react_res.status().as_u16());

    // Delete Post
    let del_res = app.api_client.delete(format!("{}/api/v4/posts/{}", &app.address, post_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, del_res.status().as_u16());

    // Typing
    let typing_res = app.api_client.post(format!("{}/api/v4/users/{}/channels/{}/typing", &app.address, user_id, channel_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(200, typing_res.status().as_u16());
}

#[tokio::test]
async fn mm_files_upload() {
    let app = spawn_app().await;

    // Register User
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)").bind(org_id).bind("OrgFile").execute(&app.db_pool).await.unwrap();
    let user_data = serde_json::json!({ "username": "fuser", "email": "f@x.com", "password": "P!", "org_id": org_id });
    let reg_res = app.api_client.post(format!("{}/api/v1/auth/register", &app.address)).json(&user_data).send().await.unwrap();
    let token = reg_res.json::<serde_json::Value>().await.unwrap()["token"].as_str().unwrap().to_string();

    // Upload
    let part = reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name("test.txt").mime_str("text/plain").unwrap();
    let form = reqwest::multipart::Form::new().part("files", part);

    let upload_res = app.api_client.post(format!("{}/api/v4/files", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send().await.unwrap();

    assert_eq!(200, upload_res.status().as_u16());
    let body: serde_json::Value = upload_res.json().await.unwrap();
    let file_id = body["file_infos"][0]["id"].as_str().unwrap();

    // Get File Info
    let info_res = app.api_client.get(format!("{}/api/v4/files/{}", &app.address, file_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();

    // It redirects to S3.
    assert!(info_res.status().is_redirection() || info_res.status().is_success());
}
