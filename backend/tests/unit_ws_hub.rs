use rustchat::realtime::{WsHub, WsEnvelope, EventType, WsBroadcast};
use uuid::Uuid;

#[tokio::test]
async fn test_ws_hub_multiple_connections() {
    let hub = WsHub::new();
    let user_id = Uuid::new_v4();

    // 1. First connection
    let mut rx1 = hub.add_connection(user_id, "user1".to_string()).await;

    // 2. Second connection
    let mut rx2 = hub.add_connection(user_id, "user1".to_string()).await;

    // 3. Broadcast
    let env = WsEnvelope::event(EventType::Hello, "test", None).with_broadcast(
        WsBroadcast {
            channel_id: None,
            team_id: None,
            user_id: Some(user_id),
            exclude_user_id: None,
        }
    );
    hub.broadcast(env).await;

    // 4. Verify rx2 receives
    let msg2 = rx2.recv().await;
    assert!(msg2.is_ok(), "Second connection should receive message");

    // 5. Verify rx1 receives (Should pass if fixed, fail if broken)
    let msg1 = rx1.recv().await;
    assert!(msg1.is_ok(), "First connection should ALSO receive message. Result: {:?}", msg1);
}
