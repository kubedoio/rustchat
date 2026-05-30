#![allow(clippy::needless_borrows_for_generic_args)]
use reqwest::StatusCode;
use uuid::Uuid;

use crate::common::spawn_app;

mod common;

#[tokio::test]
async fn member_cannot_create_team() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Permission Org").await;
    let (token, _user_id) =
        register_and_login(&app, org_id, "team_member", "team_member@example.com", None).await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "unauthorized-team",
            "display_name": "Unauthorized Team"
        }))
        .send()
        .await
        .expect("team create request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn org_admin_can_create_team() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Permission Admin Org").await;
    let (token, _user_id) = register_and_login(
        &app,
        org_id,
        "team_admin_user",
        "team_admin_user@example.com",
        Some("org_admin"),
    )
    .await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "approved-team",
            "display_name": "Approved Team"
        }))
        .send()
        .await
        .expect("team create request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("team create response should be JSON");
    assert_eq!(body["name"], "approved-team");
    assert_eq!(body["display_name"], "Approved Team");
}

#[tokio::test]
async fn guest_cannot_create_team() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Guest Permission Org").await;
    let (token, _user_id) = register_and_login(
        &app,
        org_id,
        "guest_team_user",
        "guest_team_user@example.com",
        Some("guest"),
    )
    .await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "guest-team",
            "display_name": "Guest Team"
        }))
        .send()
        .await
        .expect("team create request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_delete_team() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Team Delete Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "team_delete_member",
        "team_delete_member@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "delete-guarded-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v1/teams/{}", app.address, team_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("team delete request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn team_admin_can_delete_team() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Team Delete Success Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "team_delete_admin",
        "team_delete_admin@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "delete-allowed-team").await;
    add_team_member(&app, team_id, user_id, "admin").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v1/teams/{}", app.address, team_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("team delete request should complete");

    assert!(response.status().is_success());

    let still_exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM teams WHERE id = $1")
        .bind(team_id)
        .fetch_optional(&app.db_pool)
        .await
        .expect("team lookup should succeed");
    assert!(still_exists.is_none());
}

#[tokio::test]
async fn member_cannot_update_channel_settings() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Channel Permission Org").await;
    let (_creator_token, creator_id) = register_and_login(
        &app,
        org_id,
        "channel_creator",
        "channel_creator@example.com",
        None,
    )
    .await;
    let (member_token, member_id) = register_and_login(
        &app,
        org_id,
        "channel_member",
        "channel_member@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "permissions-team").await;
    add_team_member(&app, team_id, creator_id, "member").await;
    add_team_member(&app, team_id, member_id, "member").await;

    let channel_id = insert_channel(&app, team_id, creator_id, "permissions-channel").await;
    add_channel_member(&app, channel_id, creator_id, "admin").await;
    add_channel_member(&app, channel_id, member_id, "member").await;

    let response = app
        .api_client
        .put(format!("{}/api/v1/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {member_token}"))
        .json(&serde_json::json!({
            "display_name": "Unauthorized Rename"
        }))
        .send()
        .await
        .expect("channel update request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn guest_cannot_create_standard_channel() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Guest Channel Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "guest_channel_user",
        "guest_channel_user@example.com",
        Some("guest"),
    )
    .await;

    let team_id = insert_team(&app, org_id, "guest-channel-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "team_id": team_id,
            "name": "guest-blocked-channel",
            "display_name": "Guest Blocked Channel",
            "channel_type": "public"
        }))
        .send()
        .await
        .expect("channel create request should complete");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_can_create_standard_channel() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Member Channel Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "member_channel_user",
        "member_channel_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "member-channel-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let response = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "team_id": team_id,
            "name": "member-created-channel",
            "display_name": "Member Created Channel",
            "channel_type": "public"
        }))
        .send()
        .await
        .expect("channel create request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("channel create response should be JSON");
    assert_eq!(body["name"], "member-created-channel");
    assert_eq!(body["display_name"], "Member Created Channel");
}

#[tokio::test]
async fn channel_admin_can_update_channel_settings() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Channel Permission Success Org").await;
    let (_creator_token, creator_id) = register_and_login(
        &app,
        org_id,
        "channel_admin_creator",
        "channel_admin_creator@example.com",
        None,
    )
    .await;
    let (admin_token, admin_id) = register_and_login(
        &app,
        org_id,
        "channel_admin_member",
        "channel_admin_member@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "permissions-team-success").await;
    add_team_member(&app, team_id, creator_id, "member").await;
    add_team_member(&app, team_id, admin_id, "member").await;

    let channel_id = insert_channel(&app, team_id, creator_id, "permissions-admin-channel").await;
    add_channel_member(&app, channel_id, creator_id, "admin").await;
    add_channel_member(&app, channel_id, admin_id, "admin").await;

    let response = app
        .api_client
        .put(format!("{}/api/v1/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "display_name": "Updated By Admin",
            "purpose": "Managed safely"
        }))
        .send()
        .await
        .expect("channel update request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("channel update response should be JSON");
    assert_eq!(body["display_name"], "Updated By Admin");
    assert_eq!(body["purpose"], "Managed safely");
}

async fn insert_org(app: &common::TestApp, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind(name)
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");
    org_id
}

async fn register_and_login(
    app: &common::TestApp,
    org_id: Uuid,
    username: &str,
    email: &str,
    role: Option<&str>,
) -> (String, Uuid) {
    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&serde_json::json!({
            "username": username,
            "email": email,
            "password": "Password123!",
            "display_name": username,
            "org_id": org_id,
        }))
        .send()
        .await
        .expect("register request failed")
        .error_for_status()
        .expect("register should succeed");

    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&app.db_pool)
        .await
        .expect("registered user should exist");

    if let Some(role) = role {
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&app.db_pool)
            .await
            .expect("failed to update user role");
    }

    let login = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&serde_json::json!({
            "login_id": email,
            "password": "Password123!",
        }))
        .send()
        .await
        .expect("login request failed")
        .error_for_status()
        .expect("login should succeed");

    let token = login
        .headers()
        .get("Token")
        .and_then(|value| value.to_str().ok())
        .expect("token header missing")
        .to_string();

    (token, user_id)
}

async fn insert_team(app: &common::TestApp, org_id: Uuid, name: &str) -> Uuid {
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(name)
    .bind(name)
    .execute(&app.db_pool)
    .await
    .expect("failed to create team");
    team_id
}

async fn add_team_member(app: &common::TestApp, team_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(&app.db_pool)
        .await
        .expect("failed to add team member");
}

async fn insert_channel(
    app: &common::TestApp,
    team_id: Uuid,
    creator_id: Uuid,
    name: &str,
) -> Uuid {
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, 'public', $5)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(name)
    .bind(name)
    .bind(creator_id)
    .execute(&app.db_pool)
    .await
    .expect("failed to create channel");
    channel_id
}

async fn add_channel_member(app: &common::TestApp, channel_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, $3, '{}'::jsonb)",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(role)
    .execute(&app.db_pool)
    .await
    .expect("failed to add channel member");
}

#[tokio::test]
async fn update_channel_duplicate_name_returns_conflict() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Conflict Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "conflict_user",
        "conflict_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "conflict-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel1_id = insert_channel(&app, team_id, user_id, "channel-one").await;
    let channel2_id = insert_channel(&app, team_id, user_id, "channel-two").await;
    add_channel_member(&app, channel1_id, user_id, "admin").await;
    add_channel_member(&app, channel2_id, user_id, "admin").await;

    let response = app
        .api_client
        .put(format!("{}/api/v1/channels/{}", app.address, channel1_id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "channel-two"
        }))
        .send()
        .await
        .expect("channel update request should complete");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn v4_update_channel_duplicate_name_returns_conflict() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "V4 Conflict Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "v4_conflict_user",
        "v4_conflict_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "v4-conflict-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel1_id = insert_channel(&app, team_id, user_id, "v4-channel-one").await;
    let channel2_id = insert_channel(&app, team_id, user_id, "v4-channel-two").await;
    add_channel_member(&app, channel1_id, user_id, "admin").await;
    add_channel_member(&app, channel2_id, user_id, "admin").await;

    let response = app
        .api_client
        .put(format!("{}/api/v4/channels/{}", app.address, channel1_id))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "v4-channel-two"
        }))
        .send()
        .await
        .expect("v4 channel update request should complete");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn v4_patch_channel_duplicate_name_returns_conflict() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "V4 Patch Conflict Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "v4_patch_conflict_user",
        "v4_patch_conflict_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "v4-patch-conflict-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel1_id = insert_channel(&app, team_id, user_id, "v4-patch-channel-one").await;
    let channel2_id = insert_channel(&app, team_id, user_id, "v4-patch-channel-two").await;
    add_channel_member(&app, channel1_id, user_id, "admin").await;
    add_channel_member(&app, channel2_id, user_id, "admin").await;

    let response = app
        .api_client
        .put(format!(
            "{}/api/v4/channels/{}/patch",
            app.address, channel1_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "v4-patch-channel-two"
        }))
        .send()
        .await
        .expect("v4 channel patch request should complete");

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn v4_delete_channel_archives_and_list_deleted_includes_it() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Archive Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "archive_user",
        "archive_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "archive-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "archive-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // Archive the channel
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    // List deleted channels should include it
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/teams/{}/channels/deleted",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("list deleted channels request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let channels: Vec<serde_json::Value> = response
        .json()
        .await
        .expect("list deleted channels should be JSON");

    let found = channels
        .iter()
        .any(|c| c["id"] == rustchat::mattermost_compat::id::encode_mm_id(channel_id));
    assert!(
        found,
        "archived channel should appear in deleted channels list"
    );

    // Verify delete_at is set in the MM response
    let archived = channels
        .iter()
        .find(|c| c["id"] == rustchat::mattermost_compat::id::encode_mm_id(channel_id))
        .expect("archived channel should be present");
    assert!(
        archived["delete_at"].as_i64().unwrap_or(0) > 0,
        "delete_at should be greater than 0 for archived channel"
    );
}

#[tokio::test]
async fn v4_delete_already_archived_channel_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Double Archive Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "double_archive_user",
        "double_archive_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "double-archive-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "double-archive-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // First archive
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("first channel delete request should complete");
    assert_eq!(response.status(), StatusCode::OK);

    // Second archive should fail
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("second channel delete request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_restore_channel_clears_archive_state() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Restore Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "restore_user",
        "restore_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "restore-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "restore-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // Archive the channel
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");
    assert_eq!(response.status(), StatusCode::OK);

    // Verify it's in deleted list
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/teams/{}/channels/deleted",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("list deleted channels request should complete");
    let channels: Vec<serde_json::Value> = response.json().await.unwrap();
    assert!(channels
        .iter()
        .any(|c| c["id"] == rustchat::mattermost_compat::id::encode_mm_id(channel_id)));

    // Restore the channel
    let response = app
        .api_client
        .post(format!(
            "{}/api/v4/channels/{}/restore",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel restore request should complete");

    assert_eq!(response.status(), StatusCode::OK);
    let restored: serde_json::Value = response.json().await.unwrap();
    assert_eq!(restored["delete_at"], 0);

    // Verify it's no longer in deleted list
    let response = app
        .api_client
        .get(format!(
            "{}/api/v4/teams/{}/channels/deleted",
            app.address, team_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("list deleted channels request should complete");
    let channels: Vec<serde_json::Value> = response.json().await.unwrap();
    assert!(
        !channels
            .iter()
            .any(|c| c["id"] == rustchat::mattermost_compat::id::encode_mm_id(channel_id)),
        "restored channel should not appear in deleted channels list"
    );
}

#[tokio::test]
async fn v4_restore_non_archived_channel_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Restore Non-Archived Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "restore_na_user",
        "restore_na_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "restore-na-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "restore-na-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // Try to restore a channel that was never archived
    let response = app
        .api_client
        .post(format!(
            "{}/api/v4/channels/{}/restore",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel restore request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_delete_town_square_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Town Square Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "townsquare_user",
        "townsquare_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "townsquare-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "town-square").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_delete_off_topic_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Off Topic Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "offtopic_user",
        "offtopic_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "offtopic-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "off-topic").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_delete_direct_channel_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "DM Archive Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "dm_archive_user",
        "dm_archive_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "dm-archive-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, 'direct', $5)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(format!("{user_id}_{user_id}"))
    .bind("DM")
    .bind(user_id)
    .execute(&app.db_pool)
    .await
    .expect("failed to create direct channel");

    add_channel_member(&app, channel_id, user_id, "admin").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_delete_group_channel_returns_bad_request() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "GM Archive Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "gm_archive_user",
        "gm_archive_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "gm-archive-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, 'group', $5)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(format!("{user_id}_{user_id}_{user_id}"))
    .bind("GM")
    .bind(user_id)
    .execute(&app.db_pool)
    .await
    .expect("failed to create group channel");

    add_channel_member(&app, channel_id, user_id, "admin").await;

    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn v4_archive_channel_creates_system_message() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "System Msg Org").await;
    let (token, user_id) =
        register_and_login(&app, org_id, "sysmsg_user", "sysmsg_user@example.com", None).await;

    let team_id = insert_team(&app, org_id, "sysmsg-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "sysmsg-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // Archive the channel
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    // Verify a system post was created
    let post_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM posts
        WHERE channel_id = $1
          AND props->>'type' = 'system_channel_archived'
        "#,
    )
    .bind(channel_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("should count posts");

    assert_eq!(post_count, 1, "archive system message should be created");
}

#[tokio::test]
async fn v4_restore_channel_creates_system_message() {
    let app = spawn_app().await;
    let org_id = insert_org(&app, "Restore SysMsg Org").await;
    let (token, user_id) = register_and_login(
        &app,
        org_id,
        "restore_sysmsg_user",
        "restore_sysmsg_user@example.com",
        None,
    )
    .await;

    let team_id = insert_team(&app, org_id, "restore-sysmsg-team").await;
    add_team_member(&app, team_id, user_id, "member").await;

    let channel_id = insert_channel(&app, team_id, user_id, "restore-sysmsg-channel").await;
    add_channel_member(&app, channel_id, user_id, "admin").await;

    // Archive the channel
    let response = app
        .api_client
        .delete(format!("{}/api/v4/channels/{}", app.address, channel_id))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel delete request should complete");
    assert_eq!(response.status(), StatusCode::OK);

    // Restore the channel
    let response = app
        .api_client
        .post(format!(
            "{}/api/v4/channels/{}/restore",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("channel restore request should complete");
    assert_eq!(response.status(), StatusCode::OK);

    // Verify a system post was created
    let post_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM posts
        WHERE channel_id = $1
          AND props->>'type' = 'system_channel_restored'
        "#,
    )
    .bind(channel_id)
    .fetch_one(&app.db_pool)
    .await
    .expect("should count posts");

    assert_eq!(post_count, 1, "restore system message should be created");
}
