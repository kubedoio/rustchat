use crate::common::spawn_app;
use rustchat::models::{CommandResponse, CreateSlashCommand, ExecuteCommand, SlashCommand, Team};
use serde_json::Value;

mod common;

#[tokio::test]
async fn test_slash_command_lifecycle() {
    let app = spawn_app().await;

    // 1. Register and Login
    let user_data = serde_json::json!({
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

    let login_data = serde_json::json!({
        "email": "cmd@example.com",
        "password": "Password123!"
    });

    app.api_client
        .post(format!("{}/api/v1/auth/login", &app.address))
        .json(&login_data)
        .send()
        .await
        .expect("Failed to login");

    // 2. Create Team
    let team_data = serde_json::json!({
        "name": "cmdteam",
        "display_name": "Command Team",
        "description": "Team for testing commands"
    });

    let team_res = app.api_client
        .post(format!("{}/api/v1/teams", &app.address))
        .json(&team_data)
        .send()
        .await
        .expect("Failed to create team");

    assert_eq!(200, team_res.status().as_u16());
    let team: Team = team_res.json().await.expect("Failed to parse team");

    // 3. Get Channels to find a channel ID
    let channels_res = app.api_client
        .get(format!("{}/api/v1/teams/{}/channels", &app.address, team.id))
        .send()
        .await
        .expect("Failed to list channels");

    // Note: create_team might not create default channels in the current implementation,
    // but usually there's a General channel or similar.
    // If not, we might need to create one.
    // Let's check if channels are returned.
    let channels: Vec<Value> = channels_res.json().await.expect("Failed to parse channels");

    let channel_id = if channels.is_empty() {
        // Create a channel
        let channel_data = serde_json::json!({
            "team_id": team.id,
            "name": "general",
            "display_name": "General",
            "type": "public"
        });
        let c_res = app.api_client
            .post(format!("{}/api/v1/channels", &app.address))
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

    // 4. Test Built-in Command (/echo)
    let echo_cmd = ExecuteCommand {
        command: "/echo Hello World".to_string(),
        channel_id: channel_uuid,
        team_id: Some(team.id),
    };

    let echo_res = app.api_client
        .post(format!("{}/api/v1/integrations/commands/execute", &app.address))
        .json(&echo_cmd)
        .send()
        .await
        .expect("Failed to execute echo");

    assert_eq!(200, echo_res.status().as_u16());
    let echo_body: CommandResponse = echo_res.json().await.expect("Failed to parse echo response");
    assert_eq!(echo_body.text, "Echo: Hello World");

    // 5. Create Custom Slash Command
    let new_cmd = CreateSlashCommand {
        trigger: "/custom".to_string(),
        url: "http://localhost:12345/hook".to_string(),
        method: "POST".to_string(),
        display_name: Some("Custom Cmd".to_string()),
        description: Some("A test command".to_string()),
        hint: Some("args".to_string()),
    };

    let create_res = app.api_client
        .post(format!("{}/api/v1/integrations/commands?team_id={}", &app.address, team.id))
        .json(&new_cmd)
        .send()
        .await
        .expect("Failed to create command");

    assert_eq!(200, create_res.status().as_u16());
    let created_cmd: SlashCommand = create_res.json().await.expect("Failed to parse created command");
    assert_eq!(created_cmd.trigger, "custom");

    // 6. Execute Custom Command
    // This is expected to fail (internal error) because the URL is unreachable,
    // but we verify that it attempts to execute it (i.e. not "Command not found").
    let custom_exec = ExecuteCommand {
        command: "/custom some args".to_string(),
        channel_id: channel_uuid,
        team_id: Some(team.id),
    };

    let exec_res = app.api_client
        .post(format!("{}/api/v1/integrations/commands/execute", &app.address))
        .json(&custom_exec)
        .send()
        .await
        .expect("Failed to execute custom command");

    // Since reqwest error mapping in backend returns 500 on connection error
    // check if we got a response (could be 500 or 200 with error text depending on implementation)
    // api/integrations.rs: map_err(|e| AppError::Internal(...)) -> 500
    assert_eq!(500, exec_res.status().as_u16());
}
