//! Cluster-wide WebSocket broadcast via Redis Pub/Sub
//!
//! Enables multi-node deployments by forwarding WebSocket events
//! between nodes using Redis as the message backbone.

use std::sync::Arc;

use deadpool_redis::redis::{AsyncCommands, Msg};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::realtime::{WsEnvelope, WsHub};

/// Redis channel for cluster-wide WebSocket broadcasts
const WS_CLUSTER_CHANNEL: &str = "rustchat:cluster:ws:broadcast";

/// Messages sent between nodes for cluster coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Broadcast a WebSocket event to all nodes
    Broadcast {
        /// The envelope to broadcast
        envelope: WsEnvelope,
        /// Origin node ID (to avoid echo)
        origin_node: String,
    },
    /// Node heartbeat for presence tracking
    Heartbeat {
        node_id: String,
        timestamp: i64,
        connection_count: usize,
    },
}

/// Cluster broadcast manager
pub struct ClusterBroadcast {
    /// Unique node identifier
    node_id: String,
    /// Local hub reference
    hub: Arc<WsHub>,
    /// Redis connection pool
    redis: deadpool_redis::Pool,
    /// Redis URL for dedicated pub/sub connection
    redis_url: String,
}

impl ClusterBroadcast {
    /// Create a new cluster broadcast manager
    pub fn new(redis: deadpool_redis::Pool, redis_url: String, hub: Arc<WsHub>) -> Arc<Self> {
        let node_id = format!(
            "{}-{}",
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            Uuid::new_v4()
        );

        Arc::new(Self {
            node_id,
            hub,
            redis,
            redis_url,
        })
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Start the cluster broadcast subscriber
    ///
    /// Note: This spawns a background task that maintains a dedicated pub/sub connection.
    /// The task will automatically reconnect on errors and exit when `shutdown` is cancelled.
    pub async fn start(self: &Arc<Self>, shutdown: CancellationToken) -> anyhow::Result<()> {
        let redis_url = self.redis_url.clone();
        let hub = self.hub.clone();
        let node_id = self.node_id.clone();

        info!(
            node_id = %node_id,
            channel = WS_CLUSTER_CHANNEL,
            "Starting cluster broadcast subscriber"
        );

        // Spawn subscriber in a background task
        let _handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => {
                        info!("Cluster broadcast subscriber shutting down");
                        break;
                    }
                    result = Self::run_subscriber(&redis_url, &hub, &node_id) => {
                        match result {
                            Ok(_) => {
                                info!("Cluster subscriber ended normally");
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "Cluster subscriber error, reconnecting in 5s...");
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
            }
        });

        info!("Cluster broadcast subscriber started");
        Ok(())
    }

    /// Run the subscriber loop (reconnects on failure)
    async fn run_subscriber(redis_url: &str, hub: &WsHub, node_id: &str) -> anyhow::Result<()> {
        // Create a dedicated connection for pub/sub
        // Note: deadpool_redis doesn't have native pub/sub support in the pool
        // We use a separate connection that we manage ourselves
        let client = redis::Client::open(redis_url)?;
        let mut pubsub = client.get_async_pubsub().await?;

        pubsub.subscribe(WS_CLUSTER_CHANNEL).await?;

        info!("Subscribed to cluster channel");

        let mut msg_stream = pubsub.on_message();

        loop {
            match msg_stream.next().await {
                Some(msg) => {
                    if let Err(e) = Self::handle_cluster_message(hub, node_id, msg).await {
                        error!(error = %e, "Failed to handle cluster message");
                    }
                }
                None => {
                    return Err(anyhow::anyhow!("Pub/sub stream ended"));
                }
            }
        }
    }

    /// Handle an incoming cluster message
    async fn handle_cluster_message(
        hub: &WsHub,
        local_node_id: &str,
        msg: Msg,
    ) -> anyhow::Result<()> {
        let payload: Vec<u8> = msg
            .get_payload()
            .map_err(|e| anyhow::anyhow!("Failed to get message payload: {}", e))?;

        let message: ClusterMessage = serde_json::from_slice(&payload)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize cluster message: {}", e))?;

        match message {
            ClusterMessage::Broadcast {
                envelope,
                origin_node,
            } => {
                // Don't echo back messages from this node
                if origin_node == local_node_id {
                    return Ok(());
                }

                debug!(
                    event = %envelope.event,
                    origin = %origin_node,
                    "Received cluster broadcast"
                );

                // Broadcast only locally to avoid rebroadcast loops.
                hub.broadcast_local(envelope).await;
            }
            ClusterMessage::Heartbeat {
                node_id,
                timestamp,
                connection_count,
            } => {
                debug!(
                    node = %node_id,
                    timestamp = timestamp,
                    connections = connection_count,
                    "Received cluster heartbeat"
                );
                // Heartbeats are used for monitoring, not functional logic
            }
        }

        Ok(())
    }

    /// Broadcast a message to all nodes in the cluster
    pub async fn broadcast_to_cluster(&self, envelope: WsEnvelope) -> anyhow::Result<()> {
        let message = ClusterMessage::Broadcast {
            envelope,
            origin_node: self.node_id.clone(),
        };

        let payload = serde_json::to_vec(&message)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cluster message: {}", e))?;

        let mut conn = self
            .redis
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Redis connection: {}", e))?;

        let _: () = conn
            .publish(WS_CLUSTER_CHANNEL, payload)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish to cluster: {}", e))?;

        Ok(())
    }

    /// Send a heartbeat to the cluster
    pub async fn send_heartbeat(&self) -> anyhow::Result<()> {
        let connection_count = self.hub.count_connections().await;

        let message = ClusterMessage::Heartbeat {
            node_id: self.node_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            connection_count,
        };

        let payload = serde_json::to_vec(&message)
            .map_err(|e| anyhow::anyhow!("Failed to serialize heartbeat: {}", e))?;

        let mut conn = self
            .redis
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Redis connection: {}", e))?;

        let _: () = conn
            .publish(WS_CLUSTER_CHANNEL, payload)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish heartbeat: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_message_serialization() {
        let envelope = WsEnvelope::event(
            crate::realtime::EventType::UserTyping,
            serde_json::json!({"user_id": "test"}),
            Some(Uuid::new_v4()),
        );

        let message = ClusterMessage::Broadcast {
            envelope,
            origin_node: "test-node".to_string(),
        };

        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: ClusterMessage = serde_json::from_slice(&serialized).unwrap();

        match deserialized {
            ClusterMessage::Broadcast { origin_node, .. } => {
                assert_eq!(origin_node, "test-node");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
