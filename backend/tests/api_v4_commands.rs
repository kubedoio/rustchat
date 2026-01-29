use crate::common::spawn_app;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_v4_commands() {
    let app = spawn_app().await;

    // 1. Register & Login to get Token
    let user_data = json!({
        "username": "cmduser",
        "email": "cmd@example.com",
        "password": "Password123!",
        "display_name": "Cmd User"
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    let login_v4_res = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&json!({
            "login_id": "cmd@example.com",
            "password": "Password123!"
        }))
        .send()
        .await
        .expect("Failed to login v4");

    assert_eq!(login_v4_res.status().as_u16(), 200);
    let token = login_v4_res
        .headers()
        .get("Token")
        .expect("No Token header in v4 login")
        .to_str()
        .unwrap()
        .to_string();

    // 2. Test Autocomplete
    let list_res = app
        .api_client
        .get(format!("{}/api/v4/commands/autocomplete", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list commands");

    assert_eq!(list_res.status().as_u16(), 200);
    let commands: Vec<serde_json::Value> = list_res.json().await.unwrap();
    assert!(!commands.is_empty());
    assert!(commands.iter().any(|c| c["trigger"] == "shrug"));
    assert!(commands.iter().any(|c| c["trigger"] == "call"));

    // 3. Test Execute /shrug
    let exec_res = app
        .api_client
        .post(format!("{}/api/v4/commands/execute", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "command": "/shrug",
            "channel_id": "some_channel_id",
            "team_id": "some_team_id"
        }))
        .send()
        .await
        .expect("Failed to execute shrug");

    assert_eq!(exec_res.status().as_u16(), 200);
    let resp: serde_json::Value = exec_res.json().await.unwrap();
    assert_eq!(resp["response_type"], "in_channel");
    assert!(resp["text"].as_str().unwrap().contains("¯\\_(ツ)_/¯"));

    // 4. Test Execute /call (Default Disabled)
    let exec_call_res = app
        .api_client
        .post(format!("{}/api/v4/commands/execute", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({
            "command": "/call",
            "channel_id": "some_channel_id"
        }))
        .send()
        .await
        .expect("Failed to execute call");

    assert_eq!(exec_call_res.status().as_u16(), 200);
    let call_resp: serde_json::Value = exec_call_res.json().await.unwrap();

    // Expect disabled message because default config is disabled
    assert_eq!(call_resp["response_type"], "ephemeral");
    assert_eq!(call_resp["text"], "MiroTalk integration is disabled.");
}
