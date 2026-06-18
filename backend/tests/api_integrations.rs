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
        url: "http://rustchat-test.invalid/hook".to_string(),
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

#[tokio::test]
async fn test_slash_command_creation_rejects_invalid_urls() {
    let app = spawn_app().await;

    // 1. Register
    let user_data = serde_json::json!({
        "username": "cmduser2",
        "email": "cmd2@example.com",
        "password": "Password123!",
        "display_name": "Cmd User 2"
    });

    app.api_client
        .post(&format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register");

    // 2. Promote to org_admin BEFORE login
    sqlx::query("UPDATE users SET role = 'org_admin' WHERE username = 'cmduser2'")
        .execute(&app.db_pool)
        .await
        .expect("Failed to update user role");

    // 3. Login
    let login_data = serde_json::json!({
        "email": "cmd2@example.com",
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

    // 4. Create Team
    let team_data = serde_json::json!({
        "name": "cmdteam2",
        "display_name": "Command Team 2",
        "description": "Team for testing command URL validation"
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

    let invalid_urls = [
        ("localhost", "http://localhost:12345/hook"),
        ("loopback_v4", "http://127.0.0.1:12345/hook"),
        ("loopback_v6", "http://[::1]:12345/hook"),
        ("private_ip", "http://10.0.0.1/hook"),
        ("metadata_ip", "http://169.254.169.254/hook"),
        ("internal_host", "http://metadata.google.internal/hook"),
        ("non_http_scheme", "ftp://example.com/hook"),
        ("not_a_url", "not a valid url"),
    ];

    for (idx, (label, url)) in invalid_urls.iter().enumerate() {
        let new_cmd = CreateSlashCommand {
            trigger: format!("/invalid-{}", label),
            url: url.to_string(),
            method: "POST".to_string(),
            display_name: Some(format!("Invalid {} command", label)),
            description: Some("Should be rejected".to_string()),
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
            .unwrap_or_else(|_| panic!("Failed to send create command request for {}", label));

        assert_eq!(
            422,
            create_res.status().as_u16(),
            "Expected URL '{}' (case {}) to be rejected at creation time",
            url,
            idx
        );
    }

    // A plain public http(s) URL should be accepted at creation time.
    let valid_cmd = CreateSlashCommand {
        trigger: "/valid".to_string(),
        url: "https://example.com/hook".to_string(),
        method: "POST".to_string(),
        display_name: Some("Valid Cmd".to_string()),
        description: Some("Should be accepted".to_string()),
        hint: Some("args".to_string()),
    };

    let valid_res = app
        .api_client
        .post(&format!(
            "{}/api/v1/commands?team_id={}",
            &app.address, team.id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&valid_cmd)
        .send()
        .await
        .expect("Failed to create valid command");

    assert_eq!(200, valid_res.status().as_u16());
}
