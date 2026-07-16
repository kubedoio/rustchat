#![allow(dead_code)]
#![allow(clippy::needless_borrows_for_generic_args)]
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

mod common;

struct TestContext {
    app: common::TestApp,
    sender_token: String,
    sender_id: Uuid,
    receiver_token: String,
    receiver_id: Uuid,
    receiver_username: String,
    observer_token: String,
    observer_id: Uuid,
    observer_username: String,
    team_id: Uuid,
    channel_id: Uuid,
}

async fn setup_context() -> TestContext {
    let mut config = common::test_config();
    config.unread.post_priority_enabled = true;
    let app = common::spawn_app_with_config(config).await;

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Unread Test Org")
        .execute(&app.db_pool)
        .await
        .expect("failed to create organization");

    // Register sender
    let sender_email = format!("sender_{}@example.com", Uuid::new_v4());
    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": format!("sender_{}", &sender_email[..8]),
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
        .post(format!("{}/api/v4/users/login", app.address))
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
        .get(format!("{}/api/v4/users/me", app.address))
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

    // Register receiver
    let receiver_email = format!("receiver_{}@example.com", Uuid::new_v4());
    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": format!("receiver_{}", &receiver_email[..8]),
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
        .post(format!("{}/api/v4/users/login", app.address))
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
        .get(format!("{}/api/v4/users/me", app.address))
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
    let receiver_username = receiver_me["username"]
        .as_str()
        .expect("receiver username should be string")
        .to_string();

    // Register observer
    let observer_email = format!("observer_{}@example.com", Uuid::new_v4());
    app.api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": format!("observer_{}", &observer_email[..8]),
            "email": observer_email,
            "password": "Password123!",
            "display_name": "Observer",
            "org_id": org_id
        }))
        .send()
        .await
        .expect("failed to register observer")
        .error_for_status()
        .expect("observer register should succeed");

    let observer_login = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&json!({ "login_id": observer_email, "password": "Password123!" }))
        .send()
        .await
        .expect("failed to login observer")
        .error_for_status()
        .expect("observer login should succeed");

    let observer_token = observer_login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("missing observer token")
        .to_string();

    let observer_me = app
        .api_client
        .get(format!("{}/api/v4/users/me", app.address))
        .header("Authorization", format!("Bearer {observer_token}"))
        .send()
        .await
        .expect("failed to get observer me")
        .error_for_status()
        .expect("observer me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("observer me should be json");

    let observer_id = observer_me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("observer id should parse");
    let observer_username = observer_me["username"]
        .as_str()
        .expect("observer username should be string")
        .to_string();

    // Create team and channel
    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("unread-team-{}", &team_id.to_string()[..8]))
    .bind("Unread Test Team")
    .execute(&app.db_pool)
    .await
    .expect("failed to create team");

    for uid in [sender_id, receiver_id, observer_id] {
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
        .bind(format!("unread-chan-{}", &channel_id.to_string()[..8]))
        .execute(&app.db_pool)
        .await
        .expect("failed to create channel");

    for uid in [sender_id, receiver_id, observer_id] {
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
        receiver_token,
        receiver_id,
        receiver_username,
        observer_token,
        observer_id,
        observer_username,
        team_id,
        channel_id,
    }
}

async fn get_channel_unread(
    app: &common::TestApp,
    token: &str,
    channel_id: Uuid,
) -> serde_json::Value {
    let res = app
        .api_client
        .get(format!(
            "{}/api/v4/channels/{}/unread",
            app.address,
            encode_mm_id(channel_id)
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("failed to get channel unread")
        .error_for_status()
        .expect("channel unread should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("channel unread should be json");
    res
}

async fn get_redis_channel_unread(
    app: &common::TestApp,
    user_id: Uuid,
    channel_id: Uuid,
) -> HashMap<String, i64> {
    let key = format!("rc:unread:v2:uc:{}:{}", user_id, channel_id);
    let mut conn = app.redis_pool.get().await.expect("redis connection");

    let values: Vec<Option<String>> = redis::cmd("HMGET")
        .arg(&key)
        .arg(&[
            "msg_count",
            "msg_count_root",
            "mention_count",
            "mention_count_root",
            "urgent_mention_count",
        ])
        .query_async(&mut conn)
        .await
        .expect("hmget should succeed");

    let fields = [
        "msg_count",
        "msg_count_root",
        "mention_count",
        "mention_count_root",
        "urgent_mention_count",
    ];
    fields
        .iter()
        .zip(values.iter())
        .map(|(field, value)| {
            let parsed = value
                .as_ref()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);
            (field.to_string(), parsed)
        })
        .collect()
}

async fn create_post(
    app: &common::TestApp,
    token: &str,
    channel_id: Uuid,
    message: &str,
) -> serde_json::Value {
    let res = app
        .api_client
        .post(format!("{}/api/v4/posts", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "channel_id": channel_id.to_string(),
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
    res
}

#[tokio::test]
async fn increment_unreads_updates_counts() {
    let ctx = setup_context().await;

    // Verify receiver starts with 0 unreads (no posts yet)
    let before = get_channel_unread(&ctx.app, &ctx.receiver_token, ctx.channel_id).await;
    assert_eq!(
        before["msg_count"], 0,
        "receiver should start with 0 unreads"
    );

    // Sender creates a post
    create_post(&ctx.app, &ctx.sender_token, ctx.channel_id, "Hello world").await;

    // Receiver should now have 1 unread
    let after = get_channel_unread(&ctx.app, &ctx.receiver_token, ctx.channel_id).await;
    assert_eq!(
        after["msg_count"], 1,
        "receiver unread count should increment after post"
    );
}

#[tokio::test]
async fn increment_unreads_skips_sender() {
    let ctx = setup_context().await;

    // Verify sender starts with 0 unreads
    let before = get_channel_unread(&ctx.app, &ctx.sender_token, ctx.channel_id).await;
    assert_eq!(before["msg_count"], 0, "sender should start with 0 unreads");

    // Sender creates a post
    create_post(&ctx.app, &ctx.sender_token, ctx.channel_id, "Hello world").await;

    // Sender should still have 0 unreads (their own post should not increment)
    let after = get_channel_unread(&ctx.app, &ctx.sender_token, ctx.channel_id).await;
    assert_eq!(
        after["msg_count"], 0,
        "sender unread count should remain 0 after their own post"
    );

    // Verify via channel_reads table that sender was advanced to their post seq
    let last_read: Option<i64> = sqlx::query_scalar(
        "SELECT last_read_message_id FROM channel_reads WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(ctx.channel_id)
    .bind(ctx.sender_id)
    .fetch_optional(&ctx.app.db_pool)
    .await
    .expect("failed to query channel_reads");

    assert!(
        last_read.is_some() && last_read.unwrap() > 0,
        "sender should have a channel_reads entry with positive seq"
    );
}

#[tokio::test]
async fn get_unread_counts_returns_correct_values() {
    let ctx = setup_context().await;

    // Create 3 posts
    for i in 1..=3 {
        create_post(
            &ctx.app,
            &ctx.sender_token,
            ctx.channel_id,
            &format!("Message {i}"),
        )
        .await;
    }

    // Receiver should have exactly 3 unreads
    let receiver_unread = get_channel_unread(&ctx.app, &ctx.receiver_token, ctx.channel_id).await;
    assert_eq!(
        receiver_unread["msg_count"], 3,
        "receiver should have exactly 3 unreads"
    );
    assert_eq!(
        receiver_unread["mention_count"], 0,
        "receiver should have 0 mentions"
    );

    // Sender should have 0 unreads
    let sender_unread = get_channel_unread(&ctx.app, &ctx.sender_token, ctx.channel_id).await;
    assert_eq!(
        sender_unread["msg_count"], 0,
        "sender should have 0 unreads after reading their own posts"
    );

    // Mark channel as read for receiver
    let mark_read_res = ctx
        .app
        .api_client
        .post(format!(
            "{}/api/v4/channels/{}/members/me/read",
            ctx.app.address,
            encode_mm_id(ctx.channel_id)
        ))
        .header("Authorization", format!("Bearer {}", ctx.receiver_token))
        .send()
        .await
        .expect("failed to mark channel read")
        .error_for_status()
        .expect("mark read should succeed");

    let mark_read_body: serde_json::Value = mark_read_res.json().await.expect("should be json");
    assert_eq!(mark_read_body["status"], "OK");

    // Receiver should now have 0 unreads
    let after_read = get_channel_unread(&ctx.app, &ctx.receiver_token, ctx.channel_id).await;
    assert_eq!(
        after_read["msg_count"], 0,
        "receiver should have 0 unreads after marking channel read"
    );
}

#[tokio::test]
async fn plain_message_increments_only_msg_counts_in_redis() {
    let ctx = setup_context().await;

    create_post(&ctx.app, &ctx.sender_token, ctx.channel_id, "Hello world").await;

    for user_id in [ctx.receiver_id, ctx.observer_id] {
        let counts = get_redis_channel_unread(&ctx.app, user_id, ctx.channel_id).await;
        assert_eq!(counts["msg_count"], 1, "msg_count should increment");
        assert_eq!(
            counts["msg_count_root"], 1,
            "msg_count_root should increment"
        );
        assert_eq!(
            counts["mention_count"], 0,
            "mention_count should stay 0 for plain message"
        );
        assert_eq!(
            counts["mention_count_root"], 0,
            "mention_count_root should stay 0 for plain message"
        );
        assert_eq!(
            counts["urgent_mention_count"], 0,
            "urgent_mention_count should stay 0 for plain message"
        );
    }
}

#[tokio::test]
async fn user_mention_increments_mention_counts_for_that_user_only() {
    let ctx = setup_context().await;

    create_post(
        &ctx.app,
        &ctx.sender_token,
        ctx.channel_id,
        &format!("hey @{} check this", ctx.receiver_username),
    )
    .await;

    let receiver_counts = get_redis_channel_unread(&ctx.app, ctx.receiver_id, ctx.channel_id).await;
    assert_eq!(receiver_counts["msg_count"], 1);
    assert_eq!(receiver_counts["mention_count"], 1);
    assert_eq!(receiver_counts["mention_count_root"], 1);
    assert_eq!(receiver_counts["urgent_mention_count"], 0);

    let observer_counts = get_redis_channel_unread(&ctx.app, ctx.observer_id, ctx.channel_id).await;
    assert_eq!(observer_counts["msg_count"], 1);
    assert_eq!(
        observer_counts["mention_count"], 0,
        "observer should not be mentioned"
    );
    assert_eq!(
        observer_counts["mention_count_root"], 0,
        "observer should not be mentioned"
    );
    assert_eq!(observer_counts["urgent_mention_count"], 0);
}

#[tokio::test]
async fn here_mention_increments_urgent_count_for_channel_members() {
    let ctx = setup_context().await;

    create_post(&ctx.app, &ctx.sender_token, ctx.channel_id, "@here urgent").await;

    for user_id in [ctx.receiver_id, ctx.observer_id] {
        let counts = get_redis_channel_unread(&ctx.app, user_id, ctx.channel_id).await;
        assert_eq!(counts["msg_count"], 1);
        assert_eq!(
            counts["mention_count"], 0,
            "@here alone is not a regular mention"
        );
        assert_eq!(
            counts["mention_count_root"], 0,
            "@here alone is not a regular mention"
        );
        assert_eq!(
            counts["urgent_mention_count"], 1,
            "@here should increment urgent count"
        );
    }
}
