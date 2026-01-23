//! WebSocket connection hub

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::events::WsEnvelope;

/// Connection info for a WebSocket client
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub user_id: Uuid,
    pub channels: Vec<Uuid>,
    pub teams: Vec<Uuid>,
}

/// WebSocket Hub manages all active connections
pub struct WsHub {
    /// Active connections: user_id -> sender
    connections: RwLock<HashMap<Uuid, broadcast::Sender<String>>>,
    /// User subscriptions to channels
    channel_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>, // channel_id -> user_ids
    /// User presence status
    presence: RwLock<HashMap<Uuid, String>>,
    /// Usernames cache
    usernames: RwLock<HashMap<Uuid, String>>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
        })
    }

    /// Add a new connection
    pub async fn add_connection(&self, user_id: Uuid, username: String) -> broadcast::Receiver<String> {
        let (tx, rx) = broadcast::channel(100);
        
        let mut connections = self.connections.write().await;
        connections.insert(user_id, tx);
        
        let mut presence = self.presence.write().await;
        presence.insert(user_id, "online".to_string());
        
        let mut usernames = self.usernames.write().await;
        usernames.insert(user_id, username);
        
        rx
    }

    /// Remove a connection
    pub async fn remove_connection(&self, user_id: Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(&user_id);
        
        let mut presence = self.presence.write().await;
        presence.remove(&user_id);
        
        let mut usernames = self.usernames.write().await;
        usernames.remove(&user_id);
    }

    /// Subscribe user to a channel
    pub async fn subscribe_channel(&self, user_id: Uuid, channel_id: Uuid) {
        let mut subs = self.channel_subscriptions.write().await;
        subs.entry(channel_id)
            .or_insert_with(Vec::new)
            .push(user_id);
    }

    /// Unsubscribe user from a channel
    pub async fn unsubscribe_channel(&self, user_id: Uuid, channel_id: Uuid) {
        let mut subs = self.channel_subscriptions.write().await;
        if let Some(users) = subs.get_mut(&channel_id) {
            users.retain(|&id| id != user_id);
        }
    }

    /// Broadcast event to specific targets
    pub async fn broadcast(&self, envelope: WsEnvelope) {
        let message = match serde_json::to_string(&envelope) {
            Ok(m) => m,
            Err(_) => return,
        };

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

                        if let Some(tx) = connections.get(user_id) {
                            let _ = tx.send(message.clone());
                        }
                    }
                }
            } else if let Some(user_id) = broadcast.user_id {
                // Direct message to specific user
                if let Some(tx) = connections.get(&user_id) {
                    let _ = tx.send(message);
                }
            }
        } else {
            // Broadcast to all (rare, mainly for system messages)
            for tx in connections.values() {
                let _ = tx.send(message.clone());
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
}

impl Default for WsHub {
    fn default() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
        }
    }
}
