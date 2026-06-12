//! WebSocket connection hub

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::timeout;
use uuid::Uuid;

use super::cluster_broadcast::ClusterBroadcast;
use super::events::WsEnvelope;
use super::websocket_actor::WsCommand;
use crate::telemetry::metrics;

/// Connection info for a WebSocket client
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub user_id: Uuid,
    pub channels: Vec<Uuid>,
    pub teams: Vec<Uuid>,
}

/// Connection handles stored per socket.
struct ConnectionHandles {
    broadcast_tx: broadcast::Sender<String>,
    cmd_tx: mpsc::Sender<WsCommand>,
}

/// WebSocket Hub manages all active connections
pub struct WsHub {
    /// Active connections: user_id -> connection_id -> handles
    connections: RwLock<HashMap<Uuid, HashMap<Uuid, ConnectionHandles>>>,
    /// User subscriptions to channels
    channel_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>, // channel_id -> user_ids
    /// User subscriptions to teams
    team_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>, // team_id -> user_ids
    /// Reverse index: user_id -> channel_ids
    user_channel_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Reverse index: user_id -> team_ids
    user_team_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// User presence status
    presence: RwLock<HashMap<Uuid, String>>,
    /// Usernames cache
    usernames: RwLock<HashMap<Uuid, String>>,
    /// Optional cluster broadcaster for multi-node fan-out
    cluster_broadcast: RwLock<Option<Arc<ClusterBroadcast>>>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            team_subscriptions: RwLock::new(HashMap::new()),
            user_channel_subscriptions: RwLock::new(HashMap::new()),
            user_team_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
            cluster_broadcast: RwLock::new(None),
        })
    }

    pub async fn set_cluster_broadcast(&self, cluster: Arc<ClusterBroadcast>) {
        let mut slot = self.cluster_broadcast.write().await;
        *slot = Some(cluster);
    }

    /// Add a new connection
    pub async fn add_connection(
        &self,
        user_id: Uuid,
        username: String,
        cmd_tx: mpsc::Sender<WsCommand>,
    ) -> (Uuid, broadcast::Receiver<String>) {
        let (tx, rx) = broadcast::channel(100);
        let connection_id = Uuid::new_v4();
        let handles = ConnectionHandles {
            broadcast_tx: tx,
            cmd_tx,
        };

        let mut connections = self.connections.write().await;
        connections
            .entry(user_id)
            .or_insert_with(HashMap::new)
            .insert(connection_id, handles);

        let mut presence = self.presence.write().await;
        presence.insert(user_id, "online".to_string());

        let mut usernames = self.usernames.write().await;
        usernames.insert(user_id, username);

        metrics::record_ws_connection();

        (connection_id, rx)
    }

    /// Remove a connection
    pub async fn remove_connection(&self, user_id: Uuid, connection_id: Uuid) {
        let mut connections = self.connections.write().await;
        let mut should_clear_presence = false;

        if let Some(user_connections) = connections.get_mut(&user_id) {
            user_connections.remove(&connection_id);
            metrics::record_ws_disconnection();
            if user_connections.is_empty() {
                connections.remove(&user_id);
                should_clear_presence = true;
            }
        }

        drop(connections);

        if should_clear_presence {
            let mut presence = self.presence.write().await;
            presence.remove(&user_id);

            let mut usernames = self.usernames.write().await;
            usernames.remove(&user_id);

            drop(presence);
            drop(usernames);

            // Clean up channel subscriptions using reverse index
            let user_channels: Vec<Uuid> = {
                let mut rev = self.user_channel_subscriptions.write().await;
                rev.remove(&user_id).unwrap_or_default()
            };

            if !user_channels.is_empty() {
                let mut subs = self.channel_subscriptions.write().await;
                for channel_id in user_channels {
                    if let Some(users) = subs.get_mut(&channel_id) {
                        users.retain(|&id| id != user_id);
                        if users.is_empty() {
                            subs.remove(&channel_id);
                        }
                    }
                }
            }

            // Clean up team subscriptions using reverse index
            let user_teams: Vec<Uuid> = {
                let mut rev = self.user_team_subscriptions.write().await;
                rev.remove(&user_id).unwrap_or_default()
            };

            if !user_teams.is_empty() {
                let mut subs = self.team_subscriptions.write().await;
                for team_id in user_teams {
                    if let Some(users) = subs.get_mut(&team_id) {
                        users.retain(|&id| id != user_id);
                        if users.is_empty() {
                            subs.remove(&team_id);
                        }
                    }
                }
            }
        }
    }

    /// Subscribe user to a channel
    pub async fn subscribe_channel(&self, user_id: Uuid, channel_id: Uuid) {
        {
            let mut subs = self.channel_subscriptions.write().await;
            let users = subs.entry(channel_id).or_insert_with(Vec::new);
            if !users.contains(&user_id) {
                users.push(user_id);
            }
        }
        {
            let mut rev = self.user_channel_subscriptions.write().await;
            let channels = rev.entry(user_id).or_insert_with(Vec::new);
            if !channels.contains(&channel_id) {
                channels.push(channel_id);
            }
        }
    }

    /// Unsubscribe user from a channel
    pub async fn unsubscribe_channel(&self, user_id: Uuid, channel_id: Uuid) {
        {
            let mut subs = self.channel_subscriptions.write().await;
            if let Some(users) = subs.get_mut(&channel_id) {
                users.retain(|&id| id != user_id);
                if users.is_empty() {
                    subs.remove(&channel_id);
                }
            }
        }
        {
            let mut rev = self.user_channel_subscriptions.write().await;
            if let Some(channels) = rev.get_mut(&user_id) {
                channels.retain(|&id| id != channel_id);
                if channels.is_empty() {
                    rev.remove(&user_id);
                }
            }
        }
    }

    /// Subscribe user to a team
    pub async fn subscribe_team(&self, user_id: Uuid, team_id: Uuid) {
        {
            let mut subs = self.team_subscriptions.write().await;
            let users = subs.entry(team_id).or_insert_with(Vec::new);
            if !users.contains(&user_id) {
                users.push(user_id);
            }
        }
        {
            let mut rev = self.user_team_subscriptions.write().await;
            let teams = rev.entry(user_id).or_insert_with(Vec::new);
            if !teams.contains(&team_id) {
                teams.push(team_id);
            }
        }
    }

    /// Unsubscribe user from a team
    pub async fn unsubscribe_team(&self, user_id: Uuid, team_id: Uuid) {
        {
            let mut subs = self.team_subscriptions.write().await;
            if let Some(users) = subs.get_mut(&team_id) {
                users.retain(|&id| id != user_id);
                if users.is_empty() {
                    subs.remove(&team_id);
                }
            }
        }
        {
            let mut rev = self.user_team_subscriptions.write().await;
            if let Some(teams) = rev.get_mut(&user_id) {
                teams.retain(|&id| id != team_id);
                if teams.is_empty() {
                    rev.remove(&user_id);
                }
            }
        }
    }

    /// Broadcast event to specific targets
    pub async fn broadcast(&self, envelope: WsEnvelope) {
        self.broadcast_local(envelope.clone()).await;

        let cluster = { self.cluster_broadcast.read().await.clone() };
        if let Some(cluster) = cluster {
            if let Err(err) = cluster.broadcast_to_cluster(envelope).await {
                tracing::warn!(error = %err, "Failed to fan out websocket event to cluster");
            }
        }
    }

    /// Broadcast only to local in-process subscribers.
    /// Used by cluster subscribers to avoid rebroadcast loops.
    pub async fn broadcast_local(&self, envelope: WsEnvelope) {
        let _timer = metrics::BroadcastTimer::new();
        let message = match serde_json::to_string(&envelope) {
            Ok(m) => m,
            Err(_) => return,
        };

        // Debug logging for important events
        if envelope.event == "typing"
            || envelope.event == "stop_typing"
            || envelope.event == "status_change"
        {
            tracing::debug!(
                event = %envelope.event,
                has_broadcast = envelope.broadcast.is_some(),
                "Broadcasting WebSocket event"
            );
        }

        let connections = self.connections.read().await;

        if let Some(broadcast) = &envelope.broadcast {
            // Targeted broadcast
            if let Some(channel_id) = broadcast.channel_id {
                // Broadcast to channel subscribers
                let subs = self.channel_subscriptions.read().await;
                if let Some(user_ids) = subs.get(&channel_id) {
                    for user_id in user_ids {
                        // Check exclusions
                        if let Some(exclude) = broadcast.exclude_user_id {
                            if *user_id == exclude {
                                continue;
                            }
                        }

                        if let Some(user_connections) = connections.get(user_id) {
                            for handles in user_connections.values() {
                                let _ = handles.broadcast_tx.send(message.clone());
                            }
                        }
                    }
                }
            } else if let Some(team_id) = broadcast.team_id {
                // Broadcast to team subscribers
                let subs = self.team_subscriptions.read().await;
                if let Some(user_ids) = subs.get(&team_id) {
                    for user_id in user_ids {
                        // Check exclusions
                        if let Some(exclude) = broadcast.exclude_user_id {
                            if *user_id == exclude {
                                continue;
                            }
                        }

                        if let Some(user_connections) = connections.get(user_id) {
                            for tx in user_connections.values() {
                                let _ = tx.broadcast_tx.send(message.clone());
                            }
                        }
                    }
                }
            } else if let Some(user_id) = broadcast.user_id {
                // Direct message to specific user
                if let Some(user_connections) = connections.get(&user_id) {
                    for handles in user_connections.values() {
                        let _ = handles.broadcast_tx.send(message.clone());
                    }
                }
            }
        } else {
            // Broadcast to all (rare, mainly for system messages)
            for user_connections in connections.values() {
                for handles in user_connections.values() {
                    let _ = handles.broadcast_tx.send(message.clone());
                }
            }
        }
    }

    /// Update user presence
    pub async fn set_presence(&self, user_id: Uuid, status: String) {
        let mut presence = self.presence.write().await;
        presence.insert(user_id, status);
    }

    /// Get user presence
    pub async fn get_presence(&self, user_id: Uuid) -> Option<String> {
        let presence = self.presence.read().await;
        presence.get(&user_id).cloned()
    }

    /// Get all online users
    pub async fn online_users(&self) -> Vec<Uuid> {
        let presence = self.presence.read().await;
        presence
            .iter()
            .filter(|(_, status)| *status == "online")
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get cached username
    pub async fn get_username(&self, user_id: Uuid) -> Option<String> {
        let usernames = self.usernames.read().await;
        usernames.get(&user_id).cloned()
    }

    /// Get number of active connections
    pub async fn count_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections
            .values()
            .map(|user_connections| user_connections.len())
            .sum()
    }

    /// Get number of active connections for a user
    pub async fn user_connection_count(&self, user_id: Uuid) -> usize {
        let connections = self.connections.read().await;
        connections
            .get(&user_id)
            .map(|user_connections| user_connections.len())
            .unwrap_or(0)
    }

    /// Send a close command to every active connection, waiting for each
    /// command queue to accept it rather than silently dropping on a full queue.
    ///
    /// The read lock is dropped before spawning close tasks so that connection
    /// additions and removals are not blocked during shutdown.
    pub async fn close_all_with_code(&self, code: u16, reason: &str) {
        let cmd_txs: Vec<mpsc::Sender<WsCommand>> = {
            let connections = self.connections.read().await;
            connections
                .values()
                .flat_map(|user_connections| {
                    user_connections
                        .values()
                        .map(|handles| handles.cmd_tx.clone())
                })
                .collect()
        };

        let reason = reason.to_string();
        let mut tasks = Vec::new();
        for cmd_tx in cmd_txs {
            let reason = reason.clone();
            tasks.push(tokio::spawn(async move {
                let _ = timeout(
                    Duration::from_secs(5),
                    cmd_tx.send(WsCommand::Close(code, reason)),
                )
                .await;
            }));
        }
        // Best-effort wait for all close commands to be enqueued.
        for task in tasks {
            let _ = task.await;
        }
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            team_subscriptions: RwLock::new(HashMap::new()),
            user_channel_subscriptions: RwLock::new(HashMap::new()),
            user_team_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
            cluster_broadcast: RwLock::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::realtime::{EventType, WsBroadcast, WsEnvelope};

    fn dummy_cmd_tx() -> mpsc::Sender<WsCommand> {
        let (tx, _rx) = mpsc::channel(1);
        tx
    }

    #[tokio::test]
    async fn channel_broadcast_respects_exclude_user() {
        let hub = WsHub::new();

        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let user_c = Uuid::new_v4();
        let channel_id = Uuid::new_v4();

        let (_conn_a, mut rx_a) = hub
            .add_connection(user_a, "user-a".to_string(), dummy_cmd_tx())
            .await;
        let (_conn_b, mut rx_b) = hub
            .add_connection(user_b, "user-b".to_string(), dummy_cmd_tx())
            .await;
        let (_conn_c, mut rx_c) = hub
            .add_connection(user_c, "user-c".to_string(), dummy_cmd_tx())
            .await;

        hub.subscribe_channel(user_a, channel_id).await;
        hub.subscribe_channel(user_b, channel_id).await;

        let envelope = WsEnvelope::event(
            EventType::UserTyping,
            serde_json::json!({"channel_id": channel_id}),
            Some(channel_id),
        )
        .with_broadcast(WsBroadcast {
            channel_id: Some(channel_id),
            team_id: None,
            user_id: None,
            exclude_user_id: Some(user_a),
        });

        hub.broadcast(envelope).await;

        let b_msg = timeout(Duration::from_millis(250), rx_b.recv()).await;
        assert!(b_msg.is_ok(), "channel subscriber should receive broadcast");

        let a_msg = timeout(Duration::from_millis(150), rx_a.recv()).await;
        assert!(a_msg.is_err(), "excluded user must not receive broadcast");

        let c_msg = timeout(Duration::from_millis(150), rx_c.recv()).await;
        assert!(
            c_msg.is_err(),
            "non-subscriber should not receive channel broadcast"
        );
    }

    #[tokio::test]
    async fn direct_user_broadcast_targets_only_user() {
        let hub = WsHub::new();

        let target = Uuid::new_v4();
        let other = Uuid::new_v4();

        let (_target_conn, mut target_rx) = hub
            .add_connection(target, "target".to_string(), dummy_cmd_tx())
            .await;
        let (_other_conn, mut other_rx) = hub
            .add_connection(other, "other".to_string(), dummy_cmd_tx())
            .await;

        let envelope = WsEnvelope::event(
            EventType::ChannelSubscribed,
            serde_json::json!({"ok": true}),
            None,
        )
        .with_broadcast(WsBroadcast {
            user_id: Some(target),
            channel_id: None,
            team_id: None,
            exclude_user_id: None,
        });

        hub.broadcast(envelope).await;

        let target_msg = timeout(Duration::from_millis(250), target_rx.recv()).await;
        assert!(
            target_msg.is_ok(),
            "target user should receive direct message"
        );

        let other_msg = timeout(Duration::from_millis(150), other_rx.recv()).await;
        assert!(
            other_msg.is_err(),
            "other users should not receive direct message"
        );
    }

    #[tokio::test]
    async fn channel_subscription_cleaned_on_last_disconnect() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();
        let channel = Uuid::new_v4();

        let (_conn, _rx) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        hub.subscribe_channel(user, channel).await;
        hub.remove_connection(user, _conn).await;

        let subs = hub.channel_subscriptions.read().await;
        assert!(!subs.contains_key(&channel));
    }

    #[tokio::test]
    async fn channel_subscription_not_cleaned_when_other_connections_remain() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();
        let channel = Uuid::new_v4();

        let (conn1, _rx1) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        let (conn2, _rx2) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        hub.subscribe_channel(user, channel).await;

        hub.remove_connection(user, conn1).await;
        {
            let subs = hub.channel_subscriptions.read().await;
            assert!(subs.get(&channel).unwrap().contains(&user));
        }

        hub.remove_connection(user, conn2).await;
        let subs = hub.channel_subscriptions.read().await;
        assert!(!subs.contains_key(&channel));
    }

    #[tokio::test]
    async fn unsubscribe_channel_removes_empty_entry() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();
        let channel = Uuid::new_v4();

        let (_conn, _rx) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        hub.subscribe_channel(user, channel).await;
        hub.unsubscribe_channel(user, channel).await;

        let subs = hub.channel_subscriptions.read().await;
        assert!(!subs.contains_key(&channel));

        let rev = hub.user_channel_subscriptions.read().await;
        assert!(!rev.contains_key(&user));
    }

    #[tokio::test]
    async fn team_subscription_cleaned_on_last_disconnect() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();
        let team = Uuid::new_v4();

        let (_conn, _rx) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        hub.subscribe_team(user, team).await;
        hub.remove_connection(user, _conn).await;

        let subs = hub.team_subscriptions.read().await;
        assert!(!subs.contains_key(&team));
    }

    #[tokio::test]
    async fn unsubscribe_team_removes_empty_entry() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();
        let team = Uuid::new_v4();

        let (_conn, _rx) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        hub.subscribe_team(user, team).await;
        hub.unsubscribe_team(user, team).await;

        let subs = hub.team_subscriptions.read().await;
        assert!(!subs.contains_key(&team));

        let rev = hub.user_team_subscriptions.read().await;
        assert!(!rev.contains_key(&user));
    }

    #[tokio::test]
    async fn close_all_with_code_does_not_hold_connections_lock() {
        let hub = WsHub::new();
        let user = Uuid::new_v4();

        // Add a normal connection and a slow connection whose command queue
        // will block, keeping the close tasks alive long enough to test lock
        // behaviour.
        let (normal_conn, _rx) = hub
            .add_connection(user, "u".to_string(), dummy_cmd_tx())
            .await;
        let (blocking_tx, _blocking_rx) = mpsc::channel(1);
        blocking_tx
            .try_send(WsCommand::Close(0, "block".to_string()))
            .ok();
        let (slow_conn, _rx2) = hub.add_connection(user, "u".to_string(), blocking_tx).await;

        let hub_clone = hub.clone();
        let close_fut = tokio::spawn(async move {
            hub_clone.close_all_with_code(1012, "shutdown").await;
        });

        // Try to add a new connection while close_all_with_code is in progress.
        // If the connections lock were still held, this would time out.
        let add_result = tokio::time::timeout(
            Duration::from_millis(500),
            hub.add_connection(Uuid::new_v4(), "new".to_string(), dummy_cmd_tx()),
        )
        .await;

        assert!(
            add_result.is_ok(),
            "add_connection should not be blocked by close_all_with_code"
        );

        let _ = close_fut.await;

        hub.remove_connection(user, normal_conn).await;
        hub.remove_connection(user, slow_conn).await;
    }
}
