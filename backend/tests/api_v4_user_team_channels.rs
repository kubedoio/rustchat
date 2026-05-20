#![allow(clippy::needless_borrows_for_generic_args)]
use crate::common::spawn_app;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use uuid::Uuid;

mod common;

struct TestContext {
    app: common::TestApp,
    token: String,
    user_id: String,
    user_uuid: Uuid,
    org_id: Uuid,
}

async fn setup_mm_user() -> TestContext {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("MM Org")
        .execute(&app.db_pool)
        .await
        .expect("Failed to create organization");

    let user_data = json!({
        "username": "mmuserteams",
        "email": "mmuserteams@example.com",
        "password": "Password123!",
        "display_name": "MM User Teams",
        "org_id": org_id
    });

    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to register.");

    let login_data = json!({
        "login_id": "mmuserteams@example.com",
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

    TestContext {
        app,
        token,
        user_id,
        user_uuid,
        org_id,
    }
}

async fn setup_team_channel(ctx: &TestContext) -> (Uuid, Uuid) {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, 'mmteam', 'MM Team', true)",
    )
    .bind(team_id)
    .bind(ctx.org_id)
    .execute(&ctx.app.db_pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, 'mmchannel', 'public')",
    )
    .bind(channel_id)
    .bind(team_id)
    .execute(&ctx.app.db_pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
        .bind(channel_id)
        .bind(ctx.user_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    (team_id, channel_id)
}

#[tokio::test]
async fn mm_user_team_and_channel_routes() {
    let ctx = setup_mm_user().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;

    let teams_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/teams",
            &ctx.app.address, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, teams_res.status().as_u16());
    let teams_body: serde_json::Value = teams_res.json().await.unwrap();
    assert_eq!(teams_body.as_array().unwrap().len(), 1);
    assert_eq!(teams_body[0]["id"], encode_mm_id(team_id));

    let channels_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/channels",
            &ctx.app.address, ctx.user_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, channels_res.status().as_u16());
    let channels_body: serde_json::Value = channels_res.json().await.unwrap();
    assert_eq!(channels_body.as_array().unwrap().len(), 1);

    let team_channels_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users/{}/teams/{}/channels",
            &ctx.app.address, ctx.user_id, team_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();
    assert_eq!(200, team_channels_res.status().as_u16());
    let team_channels_body: serde_json::Value = team_channels_res.json().await.unwrap();
    assert_eq!(team_channels_body.as_array().unwrap().len(), 1);
    assert_eq!(team_channels_body[0]["id"], encode_mm_id(channel_id));
}

#[tokio::test]
async fn mm_list_users_team_filters() {
    let ctx = setup_mm_user().await;
    let (team_id, channel_id) = setup_team_channel(&ctx).await;

    // Register User B under the same organization
    let user_data_b = json!({
        "username": "mmuserteamsb",
        "email": "mmuserteamsb@example.com",
        "password": "Password123!",
        "display_name": "MM User Teams B",
        "org_id": ctx.org_id
    });

    ctx.app
        .api_client
        .post(format!("{}/api/v1/auth/register", &ctx.app.address))
        .json(&user_data_b)
        .send()
        .await
        .expect("Failed to register user B.");

    let user_b_uuid: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'mmuserteamsb'")
            .fetch_one(&ctx.app.db_pool)
            .await
            .unwrap();

    // Add User B to the team
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(user_b_uuid)
        .execute(&ctx.app.db_pool)
        .await
        .unwrap();

    // 1. Query with in_team filter. Should return both mmuserteams and mmuserteamsb
    let list_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users?in_team={}",
            &ctx.app.address, team_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();

    assert_eq!(200, list_res.status().as_u16());
    let list_body: serde_json::Value = list_res.json().await.unwrap();
    let users = list_body.as_array().expect("Expected JSON array response");

    assert!(
        users.len() >= 2,
        "Expected at least 2 team members, got: {:?}",
        users
    );

    let usernames: Vec<&str> = users
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert!(usernames.contains(&"mmuserteams"));
    assert!(usernames.contains(&"mmuserteamsb"));

    // 2. Query with in_team and not_in_channel filters.
    // User A (mmuserteams / ctx.user_uuid) is inside channel_id, but User B (mmuserteamsb) is not.
    // Therefore, searching not_in_channel should return User B but NOT User A.
    let list_not_in_chan_res = ctx
        .app
        .api_client
        .get(format!(
            "{}/api/v4/users?in_team={}&not_in_channel={}",
            &ctx.app.address, team_id, channel_id
        ))
        .header("Authorization", format!("Bearer {}", ctx.token))
        .send()
        .await
        .unwrap();

    assert_eq!(200, list_not_in_chan_res.status().as_u16());
    let not_in_chan_body: serde_json::Value = list_not_in_chan_res.json().await.unwrap();
    let users_not_in_chan = not_in_chan_body
        .as_array()
        .expect("Expected JSON array response");

    let usernames_not_in_chan: Vec<&str> = users_not_in_chan
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert!(!usernames_not_in_chan.contains(&"mmuserteams"));
    assert!(usernames_not_in_chan.contains(&"mmuserteamsb"));
}
