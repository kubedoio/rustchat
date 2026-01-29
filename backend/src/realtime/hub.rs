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

/// Connection entry with unique ID
#[derive(Debug)]
struct ConnectionEntry {
    sender: broadcast::Sender<String>,
    conn_id: Uuid,
}

/// WebSocket Hub manages all active connections
pub struct WsHub {
    /// Active connections: user_id -> list of (connection_id, sender)
    connections: RwLock<HashMap<Uuid, Vec<ConnectionEntry>>>,
    /// User subscriptions to channels
    channel_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>, // channel_id -> user_ids
    /// User subscriptions to teams
    team_subscriptions: RwLock<HashMap<Uuid, Vec<Uuid>>>, // team_id -> user_ids
    /// User presence status
    presence: RwLock<HashMap<Uuid, String>>,
    /// Usernames cache
    usernames: RwLock<HashMap<Uuid, String>>,
    /// Maximum simultaneous connections per user (default: 5)
    max_connections_per_user: RwLock<usize>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            team_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
            max_connections_per_user: RwLock::new(5), // Default: 5 connections per user
        })
    }

    /// Set maximum connections per user (admin setting)
    pub async fn set_max_connections_per_user(&self, max: usize) {
        let mut max_conn = self.max_connections_per_user.write().await;
        *max_conn = max;
    }

    /// Get maximum connections per user
    pub async fn get_max_connections_per_user(&self) -> usize {
        *self.max_connections_per_user.read().await
    }

    /// Add a new connection
    /// Returns (connection_id, receiver) or None if max connections exceeded
    pub async fn add_connection(
        &self,
        user_id: Uuid,
        username: String,
    ) -> Option<(Uuid, broadcast::Receiver<String>)> {
        let max_conn = *self.max_connections_per_user.read().await;
        
        let mut connections = self.connections.write().await;
        
        // Check if user has reached max connections
        let user_conns = connections.entry(user_id).or_insert_with(Vec::new);
        if user_conns.len() >= max_conn {
            tracing::warn!(
                "User {} has reached max connections limit ({}). Rejecting new connection.",
                user_id,
                max_conn
            );
            return None;
        }
        
        let (tx, rx) = broadcast::channel(100);
        let conn_id = Uuid::new_v4();
        
        user_conns.push(ConnectionEntry {
            sender: tx,
            conn_id,
        });
        
        tracing::info!(
            "User {} connected with connection {}. Total connections: {}",
            user_id,
            conn_id,
            user_conns.len()
        );
        
        // Only update presence/username if this is the first connection
        if user_conns.len() == 1 {
            let mut presence = self.presence.write().await;
            presence.insert(user_id, "online".to_string());
            
            let mut usernames = self.usernames.write().await;
            usernames.insert(user_id, username);
        }
        
        Some((conn_id, rx))
    }

    /// Remove a specific connection
    pub async fn remove_connection(&self, user_id: Uuid, conn_id: Uuid) {
        let mut connections = self.connections.write().await;
        
        if let Some(user_conns) = connections.get_mut(&user_id) {
            // Find and remove the specific connection
            if let Some(pos) = user_conns.iter().position(|c| c.conn_id == conn_id) {
                user_conns.remove(pos);
                tracing::info!(
                    "Connection {} removed for user {}. Remaining connections: {}",
                    conn_id,
                    user_id,
                    user_conns.len()
                );
            }
            
            // If no more connections for this user, clean up
            if user_conns.is_empty() {
                connections.remove(&user_id);
                
                let mut presence = self.presence.write().await;
                presence.remove(&user_id);
                
                let mut usernames = self.usernames.write().await;
                usernames.remove(&user_id);
                
                tracing::info!("User {} disconnected (no more active connections)", user_id);
            }
        }
        
        // Note: We don't eagerly remove from subscriptions here as it requires scanning all maps.
        // Lazy cleanup happens if we implement a periodic cleaner or just rely on 'connections' check.
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

    /// Subscribe user to a team
    pub async fn subscribe_team(&self, user_id: Uuid, team_id: Uuid) {
        let mut subs = self.team_subscriptions.write().await;
        subs.entry(team_id).or_insert_with(Vec::new).push(user_id);
    }

    /// Unsubscribe user from a team
    pub async fn unsubscribe_team(&self, user_id: Uuid, team_id: Uuid) {
        let mut subs = self.team_subscriptions.write().await;
        if let Some(users) = subs.get_mut(&team_id) {
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

                        // Send to ALL connections for this user
                        if let Some(user_conns) = connections.get(user_id) {
                            for conn in user_conns {
                                let _ = conn.sender.send(message.clone());
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

                        // Send to ALL connections for this user
                        if let Some(user_conns) = connections.get(user_id) {
                            for conn in user_conns {
                                let _ = conn.sender.send(message.clone());
                            }
                        }
                    }
                }
            } else if let Some(user_id) = broadcast.user_id {
                // Direct message to specific user - send to ALL their connections
                if let Some(user_conns) = connections.get(&user_id) {
                    for conn in user_conns {
                        let _ = conn.sender.send(message.clone());
                    }
                }
            }
        } else {
            // Broadcast to all connections
            for user_conns in connections.values() {
                for conn in user_conns {
                    let _ = conn.sender.send(message.clone());
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

    /// Get total number of active connections across all users
    pub async fn count_connections(&self) -> usize {
        let connections = self.connections.read().await;
        connections.values().map(|v| v.len()).sum()
    }

    /// Get number of active connections for a specific user
    pub async fn count_user_connections(&self, user_id: Uuid) -> usize {
        let connections = self.connections.read().await;
        connections.get(&user_id).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            channel_subscriptions: RwLock::new(HashMap::new()),
            team_subscriptions: RwLock::new(HashMap::new()),
            presence: RwLock::new(HashMap::new()),
            usernames: RwLock::new(HashMap::new()),
            max_connections_per_user: RwLock::new(5),
        }
    }
}
