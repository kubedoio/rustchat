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
