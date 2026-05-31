#![allow(dead_code, unused_imports)]
#![allow(clippy::needless_borrows_for_generic_args)]
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
};
use uuid::Uuid;

use crate::common::spawn_app;

mod common;

type WsClient =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct TestContext {
    app: common::TestApp,
    sender_token: String,
    sender_id: Uuid,
    sender_username: String,
    receiver_token: String,
    receiver_id: Uuid,
    receiver_username: String,
    team_id: Uuid,
    channel_id: Uuid,
}

async fn setup_context() -> TestContext {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Posts Service Test Org")
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");

    let sender_username = format!("sender_{}", &Uuid::new_v4().to_string()[..8]);
    let sender_email = format!("{sender_username}@example.com");
    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&json!({
            "username": sender_username,
            "email": sender_email,
            "password": "Password123!",
            "display_name": "Sender",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("failed to register sender")
        .error_for_status()
        .expect("sender register should succeed");

    let sender_login = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&json!({ "login_id": sender_email, "password": "Password123!" }))
        .send()
        .await
        .expect("failed to login sender")
        .error_for_status()
        .expect("sender login should succeed");

    let sender_token = sender_login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("missing sender token")
        .to_string();

    let sender_me = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {sender_token}"))
        .send()
        .await
        .expect("failed to get sender me")
        .error_for_status()
        .expect("sender me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("sender me should be json");

    let sender_id = sender_me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("sender id should parse");

    let receiver_username = format!("receiver_{}", &Uuid::new_v4().to_string()[..8]);
    let receiver_email = format!("{receiver_username}@example.com");
    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&json!({
            "username": receiver_username,
            "email": receiver_email,
            "password": "Password123!",
            "display_name": "Receiver",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("failed to register receiver")
        .error_for_status()
        .expect("receiver register should succeed");

    let receiver_login = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&json!({ "login_id": receiver_email, "password": "Password123!" }))
        .send()
        .await
        .expect("failed to login receiver")
        .error_for_status()
        .expect("receiver login should succeed");

    let receiver_token = receiver_login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("missing receiver token")
        .to_string();

    let receiver_me = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {receiver_token}"))
        .send()
        .await
        .expect("failed to get receiver me")
        .error_for_status()
        .expect("receiver me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("receiver me should be json");

    let receiver_id = receiver_me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("receiver id should parse");

    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("posts-team-{}", &team_id.to_string()[..8]))
    .bind("Posts Service Test Team")
    .execute(&app.db_pool)
    .await
    .expect("failed to create team");

    for uid in [sender_id, receiver_id] {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(team_id)
            .bind(uid)
            .execute(&app.db_pool)
            .await
            .expect("failed to add team member");
    }

    let channel_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, $3, 'public')")
        .bind(channel_id)
        .bind(team_id)
        .bind(format!("posts-chan-{}", &channel_id.to_string()[..8]))
        .execute(&app.db_pool)
        .await
        .expect("failed to create channel");

    for uid in [sender_id, receiver_id] {
        sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
            .bind(channel_id)
            .bind(uid)
            .execute(&app.db_pool)
            .await
            .expect("failed to add channel member");
    }

    TestContext {
        app,
        sender_token,
        sender_id,
        sender_username,
        receiver_token,
        receiver_id,
        receiver_username,
        team_id,
        channel_id,
    }
}

async fn connect_ws(base_http_url: &str, token: &str) -> WsClient {
    let ws_base = base_http_url.replacen("http://", "ws://", 1);
    let ws_url = format!("{ws_base}/api/v4/websocket");

    let mut request = ws_url
        .into_client_request()
        .expect("websocket request should be valid");
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str(token).expect("valid websocket subprotocol token"),
    );

    let (ws_stream, _) = connect_async(request)
        .await
        .expect("websocket connection should succeed");
    ws_stream
}

async fn wait_for_event(
    ws: &mut WsClient,
    expected_event: &str,
    within: Duration,
) -> serde_json::Value {
    let deadline = Instant::now() + within;

    loop {
        let now = Instant::now();
        assert!(
            now < deadline,
            "timed out waiting for websocket event {expected_event}"
        );

        let timeout_left = deadline.saturating_duration_since(now);
        let message = tokio::time::timeout(timeout_left, ws.next())
            .await
            .expect("timeout while waiting for websocket frame")
            .expect("websocket closed unexpectedly")
            .expect("websocket frame should be valid");

        if let Message::Text(text) = message {
            let parsed: serde_json::Value =
                serde_json::from_str(&text).expect("frame should be valid JSON");
            if parsed["event"] == expected_event {
                return parsed;
            }
        }
    }
}

#[tokio::test]
async fn create_post_broadcasts_to_channel() {
    let ctx = setup_context().await;

    // Receiver connects via WebSocket
    let mut ws = connect_ws(&ctx.app.address, &ctx.receiver_token).await;
    let hello = wait_for_event(&mut ws, "hello", Duration::from_secs(5)).await;
    assert_eq!(hello["event"], "hello");

    // Sender creates a post
    let message = "Broadcast test message";
    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": message
        }))
        .send()
        .await
        .expect("failed to create post")
        .error_for_status()
        .expect("create post should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("create post should be json");

    let post_id = post_res["id"].as_str().expect("post should have id");

    // Receiver should receive a 'posted' websocket event
    let posted_event = wait_for_event(&mut ws, "posted", Duration::from_secs(5)).await;
    let event_data = &posted_event["data"];

    // In Mattermost-compatible WS format, data.post is a JSON string
    let post_data_str = event_data["post"]
        .as_str()
        .expect("posted event data.post should be a JSON string");
    let post_data: serde_json::Value =
        serde_json::from_str(post_data_str).expect("post data should parse as JSON");

    assert_eq!(
        post_data["message"], message,
        "posted event should contain correct message"
    );
    assert_eq!(
        post_data["id"], post_id,
        "posted event should contain correct post id"
    );
    assert_eq!(
        post_data["channel_id"],
        encode_mm_id(ctx.channel_id),
        "posted event should contain correct channel id"
    );

    // Cleanup websocket
    let _ = ws.close(None).await;
}

#[tokio::test]
async fn create_post_mentions_triggers_notification() {
    let ctx = setup_context().await;

    // Sender creates a post mentioning the receiver by username
    let mention_message = format!("Hey @{} check this out!", ctx.receiver_username);
    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": mention_message
        }))
        .send()
        .await
        .expect("failed to create post")
        .error_for_status()
        .expect("create post should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("create post should be json");

    let post_id = post_res["id"].as_str().expect("post should have id");

    // Verify post props contain mentions metadata
    let props = post_res["props"]
        .as_object()
        .expect("props should be object");
    assert!(
        props.contains_key("mentions"),
        "post props should contain mentions key"
    );
    let mentions = props["mentions"]
        .as_array()
        .expect("mentions should be array");
    assert!(
        mentions
            .iter()
            .any(|m| m.as_str() == Some(&ctx.receiver_username)),
        "post mentions should include receiver username"
    );

    let post_uuid = parse_mm_or_uuid(post_id).expect("post id should parse");

    // Query receiver's activity feed for mention notification
    let activity_res = ctx
        .app
        .api_client
        .get(format!("{}/api/v4/users/me/activity", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.receiver_token))
        .send()
        .await
        .expect("failed to get activity feed")
        .error_for_status()
        .expect("activity feed should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("activity feed should be json");

    let order = activity_res["order"]
        .as_array()
        .expect("order should be array");
    assert!(
        !order.is_empty(),
        "receiver activity feed should contain items after mention"
    );

    let activities = activity_res["activities"]
        .as_object()
        .expect("activities should be keyed by activity id");
    let mention_activity = activities.values().find(|a| {
        let actor_matches = a["actor_id"]
            .as_str()
            .and_then(parse_mm_or_uuid)
            .is_some_and(|id| id == ctx.sender_id);
        let post_matches = a["post_id"]
            .as_str()
            .and_then(parse_mm_or_uuid)
            .is_some_and(|id| id == post_uuid);

        a["type"] == "mention" && actor_matches && post_matches
    });

    assert!(
        mention_activity.is_some(),
        "receiver should have a mention activity for the post"
    );

    // Also verify via direct DB query for robustness
    let activity_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activities WHERE user_id = $1 AND type = 'mention' AND post_id = $2",
    )
    .bind(ctx.receiver_id)
    .bind(post_uuid)
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to count activities");

    assert_eq!(
        activity_count, 1,
        "there should be exactly one mention activity in the database"
    );
}

#[tokio::test]
async fn create_post_reply_increments_reply_count_and_creates_activities() {
    let ctx = setup_context().await;

    // 1. Create a root post
    let root_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": "Root post for reply test"
        }))
        .send()
        .await
        .expect("failed to create root post")
        .error_for_status()
        .expect("create root post should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("root post should be json");

    let root_id = root_res["id"].as_str().expect("root post should have id");
    let root_uuid = parse_mm_or_uuid(root_id).expect("root id should parse");

    // 2. Create a reply that mentions the receiver
    let reply_message = format!("Hey @{} replying to thread!", ctx.receiver_username);
    let reply_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": reply_message,
            "root_id": root_id
        }))
        .send()
        .await
        .expect("failed to create reply")
        .error_for_status()
        .expect("create reply should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("reply should be json");

    let reply_id = reply_res["id"].as_str().expect("reply should have id");
    let reply_uuid = parse_mm_or_uuid(reply_id).expect("reply id should parse");

    // 3. Verify root post reply_count was incremented
    let root_post_db: serde_json::Value = sqlx::query_as::<_, (i32,)>(
        "SELECT reply_count FROM posts WHERE id = $1"
    )
    .bind(root_uuid)
    .fetch_one(&ctx.app.db_pool)
    .await
    .map(|(count,)| serde_json::json!({"reply_count": count}))
    .expect("root post should exist in db");

    assert_eq!(
        root_post_db["reply_count"].as_i64(),
        Some(1),
        "root post reply_count should be incremented"
    );

    // 4. Verify reply activity was created for receiver (parent post author is sender, but reply activity goes to parent author)
    // Actually the parent author is the sender, so no reply activity for receiver.
    // Instead, verify mention activity was created.
    let mention_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activities WHERE user_id = $1 AND type = 'mention' AND post_id = $2"
    )
    .bind(ctx.receiver_id)
    .bind(reply_uuid)
    .fetch_one(&ctx.app.db_pool)
    .await
    .expect("failed to count mention activities");

    assert_eq!(
        mention_count, 1,
        "there should be exactly one mention activity for the reply"
    );

    // 5. Verify channel_reads was created for author (sender)
    let author_read: Option<i64> = sqlx::query_scalar(
        "SELECT last_read_message_id FROM channel_reads WHERE user_id = $1 AND channel_id = $2"
    )
    .bind(ctx.sender_id)
    .bind(ctx.channel_id)
    .fetch_optional(&ctx.app.db_pool)
    .await
    .expect("failed to query channel_reads");

    assert!(
        author_read.is_some(),
        "author channel_reads should exist after post creation"
    );
}

#[tokio::test]
async fn create_post_dm_remembers_removed_user() {
    let ctx = setup_context().await;

    // Create a DM channel between sender and receiver
    let dm_channel_id = Uuid::new_v4();
    let dm_name = format!("{}__{}", ctx.sender_id, ctx.receiver_id);
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, $3, 'direct')"
    )
    .bind(dm_channel_id)
    .bind(ctx.team_id)
    .bind(&dm_name)
    .execute(&ctx.app.db_pool)
    .await
    .expect("failed to create dm channel");

    // Add both users as members initially
    for uid in [ctx.sender_id, ctx.receiver_id] {
        sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
            .bind(dm_channel_id)
            .bind(uid)
            .execute(&ctx.app.db_pool)
            .await
            .expect("failed to add dm member");
    }

    // Remove receiver from DM
    sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
        .bind(dm_channel_id)
        .bind(ctx.receiver_id)
        .execute(&ctx.app.db_pool)
        .await
        .expect("failed to remove receiver from dm");

    // Verify receiver is not a member
    let member_before: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(dm_channel_id)
    .bind(ctx.receiver_id)
    .fetch_optional(&ctx.app.db_pool)
    .await
    .expect("failed to check membership before");
    assert!(member_before.is_none(), "receiver should not be a dm member before post");

    // Sender creates a post in the DM
    let post_res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": dm_channel_id.to_string(),
            "message": "DM resurrection test"
        }))
        .send()
        .await
        .expect("failed to create dm post")
        .error_for_status()
        .expect("create dm post should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("dm post should be json");

    let post_id = post_res["id"].as_str().expect("post should have id");
    assert!(!post_id.is_empty(), "post should be created successfully");

    // Verify receiver was re-added to DM
    let member_after: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM channel_members WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(dm_channel_id)
    .bind(ctx.receiver_id)
    .fetch_optional(&ctx.app.db_pool)
    .await
    .expect("failed to check membership after");
    assert!(
        member_after.is_some(),
        "receiver should be re-added to dm after post creation"
    );
}


#[tokio::test]
async fn tx_rolls_back_all_side_effects_on_late_failure() {
    let app = spawn_app().await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Rollback Test Org")
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");

    let user_a_username = format!("user_a_{}", &Uuid::new_v4().to_string()[..8]);
    let user_a_email = format!("{user_a_username}@example.com");
    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&json!({
            "username": user_a_username,
            "email": user_a_email,
            "password": "Password123!",
            "display_name": "User A",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("failed to register user_a")
        .error_for_status()
        .expect("user_a register should succeed");

    let user_a_login = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&json!({ "login_id": user_a_email, "password": "Password123!" }))
        .send()
        .await
        .expect("failed to login user_a")
        .error_for_status()
        .expect("user_a login should succeed");

    let user_a_token = user_a_login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("missing user_a token")
        .to_string();

    let user_a_me = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {user_a_token}"))
        .send()
        .await
        .expect("failed to get user_a me")
        .error_for_status()
        .expect("user_a me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("user_a me should be json");

    let user_a_id = user_a_me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("user_a id should parse");

    // Create team and channel
    let team_id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, org_id, name, display_name) VALUES ($1, $2, $3, $4)")
        .bind(team_id)
        .bind(org_id)
        .bind("rollback-team")
        .bind("Rollback Team")
        .execute(&app.db_pool)
        .await
        .expect("failed to create team");

    let channel_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, 'public', $5)")
        .bind(channel_id)
        .bind(team_id)
        .bind("rollback-channel")
        .bind("Rollback Channel")
        .bind(user_a_id)
        .execute(&app.db_pool)
        .await
        .expect("failed to create channel");

    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(channel_id)
        .bind(user_a_id)
        .execute(&app.db_pool)
        .await
        .expect("failed to add member");

    // Create a parent post by user_a
    let parent_post = app
        .api_client
        .post(format!("{}/api/v4/posts", &app.address))
        .header("Authorization", format!("Bearer {user_a_token}"))
        .json(&json!({
            "channel_id": channel_id.to_string(),
            "message": "parent post"
        }))
        .send()
        .await
        .expect("failed to create parent post")
        .error_for_status()
        .expect("parent post should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("parent post should be json");

    let parent_id = parent_post["id"].as_str().expect("parent should have id");
    let parent_uuid = parse_mm_or_uuid(parent_id).expect("parent id should parse");

    // Verify initial reply_count is 0
    let initial_reply_count: i32 = sqlx::query_scalar("SELECT reply_count FROM posts WHERE id = $1")
        .bind(parent_uuid)
        .fetch_one(&app.db_pool)
        .await
        .expect("failed to get initial reply count");
    assert_eq!(initial_reply_count, 0);

    // Register user_b who will reply
    let user_b_username = format!("user_b_{}", &Uuid::new_v4().to_string()[..8]);
    let user_b_email = format!("{user_b_username}@example.com");
    app.api_client
        .post(format!("{}/api/v1/auth/register", &app.address))
        .json(&json!({
            "username": user_b_username,
            "email": user_b_email,
            "password": "Password123!",
            "display_name": "User B",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("failed to register user_b")
        .error_for_status()
        .expect("user_b register should succeed");

    let user_b_login = app
        .api_client
        .post(format!("{}/api/v4/users/login", &app.address))
        .json(&json!({ "login_id": user_b_email, "password": "Password123!" }))
        .send()
        .await
        .expect("failed to login user_b")
        .error_for_status()
        .expect("user_b login should succeed");

    let user_b_token = user_b_login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("missing user_b token")
        .to_string();

    let user_b_me = app
        .api_client
        .get(format!("{}/api/v4/users/me", &app.address))
        .header("Authorization", format!("Bearer {user_b_token}"))
        .send()
        .await
        .expect("failed to get user_b me")
        .error_for_status()
        .expect("user_b me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("user_b me should be json");

    let user_b_id = user_b_me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("user_b id should parse");

    // Add user_b to channel
    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(channel_id)
        .bind(user_b_id)
        .execute(&app.db_pool)
        .await
        .expect("failed to add user_b member");

    // Start transaction and try to create reply post + activity
    let mut tx = app.db_pool.begin().await.expect("tx should start");

    let post_repo = rustchat::repositories::PostRepository::new(app.db_pool.clone());
    let post = post_repo
        .create_post_in_tx(&mut tx, channel_id, user_b_id, Some(parent_uuid), "reply", serde_json::json!({}), &[])
        .await
        .expect("post insert should succeed");

    // Use a non-existent team_id to trigger FK violation on activities.team_id
    let fake_team_id = Uuid::new_v4();

    // This should fail because the team does not exist (FK on activities.team_id)
    let activity_result = rustchat::services::activity::create_activity_in_tx(
        &mut tx,
        user_a_id,
        rustchat::models::ActivityType::Reply,
        user_b_id,
        channel_id,
        fake_team_id, // non-existent team
        post.id,
        Some(parent_uuid),
        Some("reply".to_string()),
        None,
    ).await;

    assert!(activity_result.is_err(), "activity should fail due to non-existent team FK");

    // Drop tx without commit = rollback
    drop(tx);

    // Verify no post was created
    let post_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE id = $1")
        .bind(post.id)
        .fetch_one(&app.db_pool)
        .await
        .expect("should query post count");
    assert_eq!(post_count, 0, "post should be rolled back");

    // Verify reply_count was NOT incremented
    let reply_count: i32 = sqlx::query_scalar("SELECT reply_count FROM posts WHERE id = $1")
        .bind(parent_uuid)
        .fetch_one(&app.db_pool)
        .await
        .expect("should query reply count");
    assert_eq!(reply_count, 0, "reply_count should be rolled back");
}

#[tokio::test]
async fn create_post_rejects_oversized_message() {
    let ctx = setup_context().await;
    let long_message = "a".repeat(4001);
    let res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": long_message
        }))
        .send()
        .await
        .expect("failed to send request");
    assert_eq!(
        res.status(),
        422,
        "oversized message should be rejected"
    );
}

#[tokio::test]
async fn create_post_rejects_too_many_files() {
    let ctx = setup_context().await;
    let file_ids: Vec<String> = (0..11).map(|_| Uuid::new_v4().to_string()).collect();
    let res = ctx
        .app
        .api_client
        .post(format!("{}/api/v4/posts", &ctx.app.address))
        .header("Authorization", format!("Bearer {}", ctx.sender_token))
        .json(&json!({
            "channel_id": ctx.channel_id.to_string(),
            "message": "message with too many files",
            "file_ids": file_ids
        }))
        .send()
        .await
        .expect("failed to send request");
    assert_eq!(
        res.status(),
        422,
        "too many files should be rejected"
    );
}
