#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::models::{CommandResponse, CreateSlashCommand, ExecuteCommand, SlashCommand, Team};
use serde_json::Value;

mod common;

#[tokio::test]
async fn test_slash_command_lifecycle() {
    let app = spawn_app().await;

    // 1. Register
    let user_data = serde_json::json!({
        "username": "cmduser",
        "email": "cmd@example.com",
        "password": "Password123!",
        "display_name": "Cmd User"
    });

    app.api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // 2. Promote to org_admin BEFORE login
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE username = 'cmduser'")
        .execute(&app.db_pool)
        .await
        .expect("Failed to update user role");

    // 3. Login
    let login_data = serde_json::json!({
        "email": "cmd@example.com",
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
    let login_body: serde_json::Value = login_res
        .json()
        .await
        .expect("Failed to parse login response");
    let token = login_body["token"]
        .as_str()
        .expect("Missing auth token")
        .to_string();
    assert_eq!(login_body["user"]["role"], "org_admin");

    // 4. Create Team
    let team_data = serde_json::json!({
        "name": "cmdteam",
        "display_name": "Command Team",
        "description": "Team for testing commands"
    });

    let team_res = app
        .api_client
        .post(&format!("{}/api/v1/teams", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&team_data)
        .send()
        .await
        .expect("Failed to create team");

    assert_eq!(200, team_res.status().as_u16());
    let team: Team = team_res.json().await.expect("Failed to parse team");

    // 5. Get Channels to find a channel ID
    let channels_res = app
        .api_client
        .get(&format!(
            "{}/api/v1/teams/{}/channels",
            &app.address, team.id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list channels");

    let channels: Vec<Value> = channels_res.json().await.expect("Failed to parse channels");

    let channel_id = if channels.is_empty() {
        // Create a channel
        let channel_data = serde_json::json!({
            "team_id": team.id,
            "name": "general",
            "display_name": "General",
            "type": "public"
        });
        let c_res = app
            .api_client
            .post(&format!("{}/api/v1/channels", &app.address))
            .header("Authorization", format!("Bearer {}", token))
            .json(&channel_data)
            .send()
            .await
            .expect("Failed to create channel");
        let c: Value = c_res.json().await.expect("Failed to parse channel");
        c["id"].as_str().unwrap().to_string()
    } else {
        channels[0]["id"].as_str().unwrap().to_string()
    };

    let channel_uuid = uuid::Uuid::parse_str(&channel_id).unwrap();

    // 6. Test Built-in Command (/echo)
    let echo_cmd = ExecuteCommand {
        command: "/echo Hello World".to_string(),
        channel_id: channel_uuid,
        team_id: Some(team.id),
    };

    let echo_res = app
        .api_client
        .post(&format!("{}/api/v1/commands/execute", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&echo_cmd)
        .send()
        .await
        .expect("Failed to execute echo");

    assert_eq!(200, echo_res.status().as_u16());
    let echo_body: CommandResponse = echo_res
        .json()
        .await
        .expect("Failed to parse echo response");
    assert_eq!(echo_body.text, "Echo: Hello World");

    // 7. Create Custom Slash Command
    let new_cmd = CreateSlashCommand {
        trigger: "/custom".to_string(),
        url: "http://localhost:12345/hook".to_string(),
        method: "POST".to_string(),
        display_name: Some("Custom Cmd".to_string()),
        description: Some("A test command".to_string()),
        hint: Some("args".to_string()),
    };

    let create_res = app
        .api_client
        .post(&format!(
            "{}/api/v1/commands?team_id={}",
            &app.address, team.id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&new_cmd)
        .send()
        .await
        .expect("Failed to create command");

    assert_eq!(200, create_res.status().as_u16());
    let created_cmd: SlashCommand = create_res
        .json()
        .await
        .expect("Failed to parse created command");
    assert_eq!(created_cmd.trigger, "custom");

    // 8. Execute Custom Command
    let custom_exec = ExecuteCommand {
        command: "/custom some args".to_string(),
        channel_id: channel_uuid,
        team_id: Some(team.id),
    };

    let exec_res = app
        .api_client
        .post(&format!("{}/api/v1/commands/execute", &app.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&custom_exec)
        .send()
        .await
        .expect("Failed to execute custom command");

    assert_eq!(200, exec_res.status().as_u16());
    let exec_body: CommandResponse = exec_res
        .json()
        .await
        .expect("Failed to parse custom command response");
    assert_eq!(exec_body.response_type, "ephemeral");
    assert_eq!(
        exec_body.text,
        "Command URL is not valid or points to an internal address"
    );
}
