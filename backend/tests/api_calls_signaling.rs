#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::useless_conversion)]
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use rustchat::mattermost_compat::id::{encode_mm_id, parse_mm_or_uuid};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
};
use uuid::Uuid;

use crate::common::spawn_app;

mod common;

type WsClient =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

#[tokio::test]
async fn calls_lifecycle_events_are_delivered_over_websocket() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Lifecycle Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "caller_a", "caller_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "caller_b", "caller_b@example.com").await;

    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let mut ws_a = connect_ws(&app.address, &token_a).await;
    let mut ws_b = connect_ws(&app.address, &token_b).await;

    wait_for_event(&mut ws_a, "hello", Duration::from_secs(5)).await;
    wait_for_event(&mut ws_b, "hello", Duration::from_secs(5)).await;
    subscribe_channel(&mut ws_a, channel_id).await;
    subscribe_channel(&mut ws_b, channel_id).await;
    let _ = wait_for_event(&mut ws_a, "channel_subscribed", Duration::from_secs(5)).await;
    let _ = wait_for_event(&mut ws_b, "channel_subscribed", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let call_start = wait_for_event(
        &mut ws_b,
        "custom_com.mattermost.calls_call_start",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(call_start["channel_id"], channel_id.to_string());
    let owner_joined = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_joined",
        Duration::from_secs(5),
    )
    .await;
    let owner_session = owner_joined["session_id"]
        .as_str()
        .expect("owner session_id should be present");
    Uuid::parse_str(owner_session).expect("owner session_id should be a raw UUID");

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);

    let user_joined = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_joined",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(user_joined["channel_id"], channel_id.to_string());
    assert_eq!(user_joined["user_id"], encode_mm_id(user_b));
    let joined_session = user_joined["session_id"]
        .as_str()
        .expect("joined session_id should be present");
    Uuid::parse_str(joined_session).expect("joined session_id should be a raw UUID");

    let leave_b = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/leave",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("leave call request failed");
    assert_eq!(leave_b.status(), StatusCode::OK);

    let user_left = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_left",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(user_left["channel_id"], channel_id.to_string());
    assert_eq!(user_left["user_id"], encode_mm_id(user_b));

    let leave_a = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/leave",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("leave call request failed");
    assert_eq!(leave_a.status(), StatusCode::OK);

    let call_end = wait_for_event(
        &mut ws_b,
        "custom_com.mattermost.calls_call_end",
        Duration::from_secs(15),
    )
    .await;
    assert!(
        call_end["channel_id"] == channel_id.to_string()
            || call_end["channel_id"] == encode_mm_id(channel_id)
    );
}

#[tokio::test]
async fn calls_start_in_direct_channel_auto_rings_other_participants() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Direct Ringing Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "direct_ring_a", "direct_ring_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "direct_ring_b", "direct_ring_b@example.com").await;
    let channel_id =
        create_team_and_channel_with_members_of_type(&app, org_id, &[user_a, user_b], "direct")
            .await;

    let mut ws_a = connect_ws(&app.address, &token_a).await;
    let mut ws_b = connect_ws(&app.address, &token_b).await;
    wait_for_event(&mut ws_a, "hello", Duration::from_secs(5)).await;
    wait_for_event(&mut ws_b, "hello", Duration::from_secs(5)).await;
    subscribe_channel(&mut ws_a, channel_id).await;
    subscribe_channel(&mut ws_b, channel_id).await;
    let _ = wait_for_event(&mut ws_a, "channel_subscribed", Duration::from_secs(5)).await;
    let _ = wait_for_event(&mut ws_b, "channel_subscribed", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);
    let start_body: serde_json::Value = start.json().await.expect("start call JSON");

    let ringing_event = wait_for_event(
        &mut ws_b,
        "custom_com.mattermost.calls_ringing",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(ringing_event["sender_id"], encode_mm_id(user_a));
    assert_eq!(ringing_event["call_id"], start_body["id"]);
}

#[tokio::test]
async fn ring_endpoint_requires_channel_membership() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Ring Permission Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "ring_member_a", "ring_member_a@example.com").await;
    let (_token_b, user_b) =
        register_and_login(&app, org_id, "ring_member_b", "ring_member_b@example.com").await;
    let (token_outsider, _user_outsider) =
        register_and_login(&app, org_id, "ring_outsider", "ring_outsider@example.com").await;

    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let ring = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/ring",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_outsider}"))
        .send()
        .await
        .expect("ring request failed");
    assert_eq!(ring.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn offer_generates_server_signaling_event_over_websocket() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Signaling Org").await;
    let (token, user_id) =
        register_and_login(&app, org_id, "signal_user", "signal_user@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_id]).await;

    let mut ws = connect_ws(&app.address, &token).await;
    wait_for_event(&mut ws, "hello", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let signal_event = wait_for_event(
        &mut ws,
        "custom_com.mattermost.calls_signal",
        Duration::from_secs(8),
    )
    .await;
    assert_eq!(signal_event["channel_id_raw"], channel_id.to_string());
    assert_eq!(signal_event["signal"]["type"], "connection-state");
    assert_eq!(signal_event["signal"]["state"], "ready");
    let conn_id = signal_event["connID"]
        .as_str()
        .expect("calls_signal should include connID");
    assert!(!conn_id.is_empty(), "connID should not be empty");
    let serialized_signal = signal_event["data"]
        .as_str()
        .expect("calls_signal should include serialized data field");
    let parsed_signal: serde_json::Value =
        serde_json::from_str(serialized_signal).expect("serialized signal should parse as JSON");
    assert_eq!(parsed_signal["type"], signal_event["signal"]["type"]);
}

#[tokio::test]
async fn calls_mobile_channel_state_and_end_route_are_compatible() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Mobile REST Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "mobile_rest_a", "mobile_rest_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "mobile_rest_b", "mobile_rest_b@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    // Promote user_a to channel admin so they can manage calls in this channel.
    sqlx::query(
        "UPDATE channel_members SET role = 'channel_admin' WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id)
    .bind(user_a)
    .execute(&app.db_pool)
    .await
    .expect("failed to promote user to channel admin");

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);

    let channels = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/channels?mobilev2=true",
            app.address
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get channels request failed");
    assert_eq!(channels.status(), StatusCode::OK);
    let channels_body: serde_json::Value = channels.json().await.expect("channels JSON");
    let channel_state = channels_body
        .as_array()
        .expect("channels should be array")
        .iter()
        .find(|entry| entry["channel_id"] == encode_mm_id(channel_id))
        .expect("channel state should exist");
    assert_eq!(channel_state["enabled"], true);
    assert!(channel_state["call"].is_object());
    assert!(channel_state["call"]["sessions"].is_object());
    let list_thread_id = channel_state["call"]["thread_id"]
        .as_str()
        .expect("channel list call state should include thread_id");
    assert!(!list_thread_id.is_empty(), "thread_id should not be empty");

    let get_channel_state = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}?mobilev2=true",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get channel state request failed");
    assert_eq!(get_channel_state.status(), StatusCode::OK);
    let get_channel_state_body: serde_json::Value =
        get_channel_state.json().await.expect("channel state JSON");
    assert_eq!(get_channel_state_body["enabled"], true);
    assert!(get_channel_state_body["call"].is_object());
    let thread_id = get_channel_state_body["call"]["thread_id"]
        .as_str()
        .expect("mobile channel state should include thread_id")
        .to_string();
    assert!(!thread_id.is_empty(), "thread_id should not be empty");

    let thread = app
        .api_client
        .get(format!("{}/api/v4/posts/{}/thread", app.address, thread_id))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get call thread request failed");
    assert_eq!(thread.status(), StatusCode::OK);

    let config = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/config",
            app.address
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("calls config request failed");
    assert_eq!(config.status(), StatusCode::OK);
    let config_body: serde_json::Value = config.json().await.expect("config JSON");
    assert_eq!(config_body["EnableRinging"], true);
    assert_eq!(config_body["HostControlsAllowed"], true);
    assert_eq!(config_body["MaxCallParticipants"], 0);
    assert_eq!(config_body["AllowScreenSharing"], true);
    assert_eq!(config_body["EnableSimulcast"], false);
    assert_eq!(config_body["EnableAV1"], false);
    assert_eq!(config_body["MaxRecordingDuration"], 60);
    assert_eq!(config_body["TranscribeAPI"], "whisper.cpp");
    assert_eq!(config_body["sku_short_name"], "starter");
    assert_eq!(config_body["EnableDCSignaling"], false);
    assert_eq!(config_body["EnableTranscriptions"], false);
    assert_eq!(config_body["EnableLiveCaptions"], false);

    let recording_start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/recording/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("recording start request failed");
    assert_ne!(recording_start.status(), StatusCode::NOT_FOUND);

    let disable_calls = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("disable calls request failed");
    assert_eq!(disable_calls.status(), StatusCode::OK);
    let disable_body: serde_json::Value = disable_calls.json().await.expect("disable JSON");
    assert_eq!(disable_body["enabled"], false);

    let end_forbidden = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/end",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("end call (non-host) request failed");
    assert_eq!(end_forbidden.status(), StatusCode::FORBIDDEN);

    let end_ok = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/end",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("end call request failed");
    assert_eq!(end_ok.status(), StatusCode::OK);
    let end_body: serde_json::Value = end_ok.json().await.expect("end JSON");
    assert_eq!(end_body["status"], "OK");

    let ended_thread = app
        .api_client
        .get(format!("{}/api/v4/posts/{}/thread", app.address, thread_id))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get ended call thread request failed");
    assert_eq!(ended_thread.status(), StatusCode::OK);
    let ended_thread_body: serde_json::Value =
        ended_thread.json().await.expect("ended thread JSON");
    let ended_post = ended_thread_body["posts"]
        .get(&thread_id)
        .expect("thread root post should exist after call end");
    let ended_at = ended_post["props"]["end_at"]
        .as_i64()
        .expect("call thread post should contain numeric end_at");
    assert!(
        ended_at > 0,
        "call thread post end_at should be set once the call ends"
    );

    let idle_state = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}?mobilev2=true",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("idle channel state request failed");
    assert_eq!(idle_state.status(), StatusCode::OK);
    let idle_body: serde_json::Value = idle_state.json().await.expect("idle JSON");
    assert_eq!(idle_body["call"], serde_json::Value::Null);
}

#[tokio::test]
async fn calls_mobile_event_names_and_payloads_are_compatible() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Mobile WS Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "mobile_ws_a", "mobile_ws_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "mobile_ws_b", "mobile_ws_b@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let mut ws_a = connect_ws(&app.address, &token_a).await;
    let mut ws_b = connect_ws(&app.address, &token_b).await;
    wait_for_event(&mut ws_a, "hello", Duration::from_secs(5)).await;
    wait_for_event(&mut ws_b, "hello", Duration::from_secs(5)).await;
    subscribe_channel(&mut ws_a, channel_id).await;
    subscribe_channel(&mut ws_b, channel_id).await;
    let _ = wait_for_event(&mut ws_a, "channel_subscribed", Duration::from_secs(5)).await;
    let _ = wait_for_event(&mut ws_b, "channel_subscribed", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);
    let _ = wait_for_event(
        &mut ws_b,
        "custom_com.mattermost.calls_call_start",
        Duration::from_secs(5),
    )
    .await;
    let owner_joined = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_joined",
        Duration::from_secs(5),
    )
    .await;
    let owner_session = owner_joined["session_id"]
        .as_str()
        .expect("owner session_id should be present");
    Uuid::parse_str(owner_session).expect("owner session_id should be raw UUID");

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);
    let b_joined_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_joined",
        Duration::from_secs(5),
    )
    .await;
    let b_joined_session = b_joined_event["session_id"]
        .as_str()
        .expect("joined participant session_id should be present");
    Uuid::parse_str(b_joined_session).expect("joined participant session_id should be raw UUID");
    let call_state_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_call_state",
        Duration::from_secs(5),
    )
    .await;
    let call_state_payload = call_state_event["call"]
        .as_str()
        .expect("call state payload should include call JSON");
    let call_state_json: serde_json::Value =
        serde_json::from_str(call_state_payload).expect("call state payload should parse as JSON");
    assert_eq!(call_state_json["channel_id"], encode_mm_id(channel_id));

    let channel_state = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}?mobilev2=true",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get channel state request failed");
    assert_eq!(channel_state.status(), StatusCode::OK);
    let channel_state_body: serde_json::Value = channel_state.json().await.expect("state JSON");
    let b_session_id = channel_state_body["call"]["sessions"]
        .as_object()
        .expect("sessions should be object")
        .values()
        .find(|session| session["user_id"] == encode_mm_id(user_b))
        .and_then(|session| session["session_id"].as_str())
        .expect("target session id should exist")
        .to_string();
    assert_eq!(b_session_id, b_joined_session);

    let raise = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/raise-hand",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("raise hand request failed");
    assert_eq!(raise.status(), StatusCode::OK);
    let raised_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_raise_hand",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        raised_event["session_id"].as_str(),
        Some(b_session_id.as_str())
    );
    assert!(raised_event["raised_hand"].as_i64().unwrap_or_default() > 0);

    let lower = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/lower-hand",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("lower hand request failed");
    assert_eq!(lower.status(), StatusCode::OK);
    let lowered_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_unraise_hand",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        lowered_event["session_id"].as_str(),
        Some(b_session_id.as_str())
    );
    assert_eq!(lowered_event["raised_hand"], 0);

    let screen_on = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/screen-share",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("screen share request failed");
    assert_eq!(screen_on.status(), StatusCode::OK);
    let screen_on_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_screen_on",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        screen_on_event["session_id"].as_str(),
        Some(b_session_id.as_str())
    );

    let host_screen_off = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/host/screen-off",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "session_id": b_session_id.clone() }))
        .send()
        .await
        .expect("host screen-off request failed");
    assert_eq!(host_screen_off.status(), StatusCode::OK);
    let screen_off_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_screen_off",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        screen_off_event["session_id"].as_str(),
        Some(b_session_id.as_str())
    );

    let host_make = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/host/make",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "new_host_id": encode_mm_id(user_b) }))
        .send()
        .await
        .expect("host make request failed");
    assert_eq!(host_make.status(), StatusCode::OK);
    let host_changed_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_call_host_changed",
        Duration::from_secs(5),
    )
    .await;
    let expected_host = encode_mm_id(user_b);
    assert_eq!(
        host_changed_event["hostID"].as_str(),
        Some(expected_host.as_str())
    );
    let _ = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_call_state",
        Duration::from_secs(5),
    )
    .await;

    let dismiss = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/dismiss-notification",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("dismiss notification request failed");
    assert_eq!(dismiss.status(), StatusCode::OK);
    let dismissed_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_dismissed_notification",
        Duration::from_secs(5),
    )
    .await;
    let expected_user = encode_mm_id(user_b);
    assert_eq!(
        dismissed_event["userID"].as_str(),
        Some(expected_user.as_str())
    );
    assert!(dismissed_event["callID"].as_str().is_some());

    let dismissed_state_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_call_state",
        Duration::from_secs(5),
    )
    .await;
    let dismissed_payload = dismissed_state_event["call"]
        .as_str()
        .expect("dismissed call state event should include call JSON");
    let dismissed_json: serde_json::Value =
        serde_json::from_str(dismissed_payload).expect("dismissed call JSON should parse");
    let dismissed_user_id = encode_mm_id(user_b);
    assert_eq!(
        dismissed_json["dismissed_notification"][dismissed_user_id.as_str()],
        true
    );

    let call_state_resp = app
        .api_client
        .get(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("get call state request failed");
    assert_eq!(call_state_resp.status(), StatusCode::OK);
    let call_state_body: serde_json::Value = call_state_resp.json().await.expect("call state JSON");
    let dismissed_user_id = encode_mm_id(user_b);
    assert_eq!(
        call_state_body["dismissed_notification"][dismissed_user_id.as_str()],
        true
    );
}

#[tokio::test]
async fn calls_host_transfers_when_original_host_leaves() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Host Transfer Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "host_leave_a", "host_leave_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "host_leave_b", "host_leave_b@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let mut ws_b = connect_ws(&app.address, &token_b).await;
    wait_for_event(&mut ws_b, "hello", Duration::from_secs(5)).await;
    subscribe_channel(&mut ws_b, channel_id).await;
    let _ = wait_for_event(&mut ws_b, "channel_subscribed", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);

    let leave_host = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/leave",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("host leave request failed");
    assert_eq!(leave_host.status(), StatusCode::OK);

    let host_changed = wait_for_event(
        &mut ws_b,
        "custom_com.mattermost.calls_call_host_changed",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(host_changed["hostID"], encode_mm_id(user_b));

    let end_by_new_host = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/end",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("end call request failed");
    assert_eq!(end_by_new_host.status(), StatusCode::OK);
}

#[tokio::test]
async fn system_admin_can_end_call_even_when_not_host() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Admin End Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "admin_end_a", "admin_end_a@example.com").await;
    let (_token_b, user_b) =
        register_and_login(&app, org_id, "admin_end_b", "admin_end_b@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    sqlx::query("UPDATE users SET role = 'system_admin' WHERE id = $1")
        .bind(user_b)
        .execute(&app.db_pool)
        .await
        .expect("failed to promote user to system_admin");
    let token_b = login_and_get_token(&app, "admin_end_b@example.com", "Password123!").await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);

    let end = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/end",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("end call request failed");
    assert_eq!(end.status(), StatusCode::OK);
}

#[tokio::test]
async fn calls_reaction_event_contains_mobile_fields() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Reaction Payload Org").await;
    let (token_a, user_a) =
        register_and_login(&app, org_id, "reaction_a", "reaction_a@example.com").await;
    let (token_b, user_b) =
        register_and_login(&app, org_id, "reaction_b", "reaction_b@example.com").await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let mut ws_a = connect_ws(&app.address, &token_a).await;
    let mut ws_b = connect_ws(&app.address, &token_b).await;
    wait_for_event(&mut ws_a, "hello", Duration::from_secs(5)).await;
    wait_for_event(&mut ws_b, "hello", Duration::from_secs(5)).await;
    subscribe_channel(&mut ws_a, channel_id).await;
    subscribe_channel(&mut ws_b, channel_id).await;
    let _ = wait_for_event(&mut ws_a, "channel_subscribed", Duration::from_secs(5)).await;
    let _ = wait_for_event(&mut ws_b, "channel_subscribed", Duration::from_secs(5)).await;

    let start = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/start",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .send()
        .await
        .expect("start call request failed");
    assert_eq!(start.status(), StatusCode::OK);

    let join = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/calls/{}/join",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .send()
        .await
        .expect("join call request failed");
    assert_eq!(join.status(), StatusCode::OK);

    let react_action = serde_json::json!({
        "action": "custom_com.mattermost.calls_react",
        "seq": 1,
        "data": {
            "data": "{\"name\":\"+1\",\"literal\":\"👍\"}"
        }
    });
    ws_b.send(Message::Text(react_action.to_string().into()))
        .await
        .expect("reaction websocket action should be sent");

    let reacted_event = wait_for_event(
        &mut ws_a,
        "custom_com.mattermost.calls_user_reacted",
        Duration::from_secs(5),
    )
    .await;
    assert!(reacted_event["session_id"].as_str().is_some());
    assert!(reacted_event["timestamp"].as_i64().unwrap_or_default() > 0);
    assert_eq!(reacted_event["emoji"]["name"], "+1");
    assert_eq!(reacted_event["reaction"], "👍");
}

#[tokio::test]
async fn regular_member_cannot_toggle_channel_calls() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Toggle Forbidden Org").await;
    let (_token_a, user_a) = register_and_login(
        &app,
        org_id,
        "toggle_forbidden_a",
        "toggle_forbidden_a@example.com",
    )
    .await;
    let (token_b, user_b) = register_and_login(
        &app,
        org_id,
        "toggle_forbidden_b",
        "toggle_forbidden_b@example.com",
    )
    .await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    let resp = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("toggle calls request failed");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn channel_admin_can_toggle_channel_calls() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Toggle Channel Admin Org").await;
    let (token_a, user_a) = register_and_login(
        &app,
        org_id,
        "toggle_channel_admin_a",
        "toggle_channel_admin_a@example.com",
    )
    .await;
    let (token_b, user_b) = register_and_login(
        &app,
        org_id,
        "toggle_channel_admin_b",
        "toggle_channel_admin_b@example.com",
    )
    .await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    sqlx::query(
        "UPDATE channel_members SET role = 'channel_admin' WHERE channel_id = $1 AND user_id = $2",
    )
    .bind(channel_id)
    .bind(user_a)
    .execute(&app.db_pool)
    .await
    .expect("failed to promote user to channel admin");

    let resp = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("toggle calls request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("toggle calls JSON");
    assert_eq!(body["enabled"], false);

    // Regular members still cannot re-enable calls.
    let resp_b = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_b}"))
        .json(&serde_json::json!({ "enabled": true }))
        .send()
        .await
        .expect("toggle calls request failed");
    assert_eq!(resp_b.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn system_admin_can_toggle_channel_calls() {
    let app = spawn_app().await;

    let org_id = insert_org(&app, "Calls Toggle System Admin Org").await;
    let (_token_a, user_a) = register_and_login(
        &app,
        org_id,
        "toggle_system_admin_a",
        "toggle_system_admin_a@example.com",
    )
    .await;
    let (_token_b, user_b) = register_and_login(
        &app,
        org_id,
        "toggle_system_admin_b",
        "toggle_system_admin_b@example.com",
    )
    .await;
    let channel_id = create_team_and_channel_with_members(&app, org_id, &[user_a, user_b]).await;

    sqlx::query("UPDATE users SET role = 'system_admin' WHERE id = $1")
        .bind(user_a)
        .execute(&app.db_pool)
        .await
        .expect("failed to promote user to system_admin");
    let token_a =
        login_and_get_token(&app, "toggle_system_admin_a@example.com", "Password123!").await;

    let resp = app
        .api_client
        .post(format!(
            "{}/api/v4/plugins/com.mattermost.calls/{}",
            app.address, channel_id
        ))
        .header("Authorization", format!("Bearer {token_a}"))
        .json(&serde_json::json!({ "enabled": false }))
        .send()
        .await
        .expect("toggle calls request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("toggle calls JSON");
    assert_eq!(body["enabled"], false);
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
        .and_then(|v| v.to_str().ok())
        .expect("token header missing")
        .to_string();

    let me = app
        .api_client
        .get(format!("{}/api/v4/users/me", app.address))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("me request failed")
        .error_for_status()
        .expect("me should succeed")
        .json::<serde_json::Value>()
        .await
        .expect("me response should be JSON");

    let user_id = me["id"]
        .as_str()
        .and_then(parse_mm_or_uuid)
        .expect("user id should parse");

    (token, user_id)
}

async fn login_and_get_token(app: &common::TestApp, email: &str, password: &str) -> String {
    let login = app
        .api_client
        .post(format!("{}/api/v4/users/login", app.address))
        .json(&serde_json::json!({
            "login_id": email,
            "password": password,
        }))
        .send()
        .await
        .expect("login request failed")
        .error_for_status()
        .expect("login should succeed");

    login
        .headers()
        .get("Token")
        .and_then(|v| v.to_str().ok())
        .expect("token header missing")
        .to_string()
}

async fn create_team_and_channel_with_members(
    app: &common::TestApp,
    org_id: Uuid,
    users: &[Uuid],
) -> Uuid {
    create_team_and_channel_with_members_of_type(app, org_id, users, "public").await
}

async fn create_team_and_channel_with_members_of_type(
    app: &common::TestApp,
    org_id: Uuid,
    users: &[Uuid],
    channel_type: &str,
) -> Uuid {
    let suffix = Uuid::new_v4().to_string().replace('-', "");
    let team_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("team_{suffix}"))
    .bind(format!("Team {suffix}"))
    .execute(&app.db_pool)
    .await
    .expect("failed to create team");

    sqlx::query(
        "INSERT INTO channels (id, team_id, name, type) VALUES ($1, $2, $3, $4::channel_type)",
    )
    .bind(channel_id)
    .bind(team_id)
    .bind(format!("channel_{suffix}"))
    .bind(channel_type)
    .execute(&app.db_pool)
    .await
    .expect("failed to create channel");

    for user_id in users {
        sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
            .bind(team_id)
            .bind(user_id)
            .execute(&app.db_pool)
            .await
            .expect("failed to add team member");

        sqlx::query("INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}')")
            .bind(channel_id)
            .bind(user_id)
            .execute(&app.db_pool)
            .await
            .expect("failed to add channel member");
    }

    channel_id
}

async fn connect_ws(base_http_url: &str, token: &str) -> WsClient {
    let ws_base = base_http_url.replacen("http://", "ws://", 1);
    let ws_url = format!("{ws_base}/api/v1/ws");
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
                return parsed["data"].clone();
            }
        }
    }
}

async fn subscribe_channel(ws: &mut WsClient, channel_id: Uuid) {
    let command = serde_json::json!({
        "type": "command",
        "event": "subscribe_channel",
        "channel_id": channel_id,
        "data": {},
    });
    ws.send(Message::Text(command.to_string().into()))
        .await
        .expect("subscribe command should be sent");
}
