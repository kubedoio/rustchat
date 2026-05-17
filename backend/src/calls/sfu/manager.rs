//! SFU Manager
//!
//! Manages SFU instances for active calls. Each call has its own SFU
//! to isolate media traffic between different calls.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;
use webrtc::ice::udp_mux::{UDPMux, UDPMuxDefault, UDPMuxParams};

use super::{VoiceEvent, SFU, VOICE_EVENT_CHANNEL_CAPACITY};
use crate::config::CallsConfig;
use tokio::sync::mpsc;

/// Manages SFU instances for all active calls
pub struct SFUManager {
    /// Active SFUs: call_id -> SFU instance
    sfus: Arc<RwLock<HashMap<Uuid, Arc<SFU>>>>,
    /// Calls configuration
    config: CallsConfig,
    /// Voice events channel
    voice_event_tx: mpsc::Sender<VoiceEvent>,
    /// Shared UDP mux for ICE (single UDP port across all SFUs).
    shared_udp_mux: Arc<RwLock<Option<Arc<dyn UDPMux + Send + Sync>>>>,
}

impl SFUManager {
    /// Create a new SFU manager
    pub fn new(config: CallsConfig, voice_event_tx: mpsc::Sender<VoiceEvent>) -> Arc<Self> {
        Arc::new(Self {
            sfus: Arc::new(RwLock::new(HashMap::new())),
            config,
            voice_event_tx,
            shared_udp_mux: Arc::new(RwLock::new(None)),
        })
    }

    /// Create or get an SFU for a call
    pub async fn get_or_create_sfu(
        &self,
        call_id: Uuid,
    ) -> Result<Arc<SFU>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if SFU already exists
        {
            let sfus = self.sfus.read().await;
            if let Some(sfu) = sfus.get(&call_id) {
                return Ok(sfu.clone());
            }
        }

        let shared_udp_mux = self.get_or_create_shared_udp_mux().await?;

        // Create new SFU
        let sfu = SFU::new(
            call_id,
            self.config.clone(),
            self.voice_event_tx.clone(),
            shared_udp_mux,
        )
        .await?;

        // Store it
        self.sfus.write().await.insert(call_id, sfu.clone());

        Ok(sfu)
    }

    /// Get an existing SFU for a call
    pub async fn get_sfu(&self, call_id: Uuid) -> Option<Arc<SFU>> {
        self.sfus.read().await.get(&call_id).cloned()
    }

    /// Remove an SFU (when call ends)
    pub async fn remove_sfu(&self, call_id: Uuid) {
        self.sfus.write().await.remove(&call_id);
    }

    /// Check if an SFU exists for a call
    pub async fn has_sfu(&self, call_id: Uuid) -> bool {
        self.sfus.read().await.contains_key(&call_id)
    }

    /// Get count of active SFUs
    pub async fn active_sfu_count(&self) -> usize {
        self.sfus.read().await.len()
    }

    /// Get all active call IDs
    pub async fn active_call_ids(&self) -> Vec<Uuid> {
        self.sfus.read().await.keys().cloned().collect()
    }

    /// Clean up all SFUs (for shutdown)
    pub async fn cleanup_all(&self) {
        self.sfus.write().await.clear();
    }

    async fn get_or_create_shared_udp_mux(
        &self,
    ) -> Result<Option<Arc<dyn UDPMux + Send + Sync>>, Box<dyn std::error::Error + Send + Sync>>
    {
        if self.config.udp_port == 0 {
            return Ok(None);
        }

        {
            let guard = self.shared_udp_mux.read().await;
            if let Some(mux) = guard.as_ref() {
                return Ok(Some(mux.clone()));
            }
        }

        let mut guard = self.shared_udp_mux.write().await;
        if let Some(mux) = guard.as_ref() {
            return Ok(Some(mux.clone()));
        }

        let bind_addr = format!("0.0.0.0:{}", self.config.udp_port);
        let udp_socket = match UdpSocket::bind(&bind_addr).await {
            Ok(socket) => socket,
            Err(err) => {
                tracing::warn!(
                    udp_port = self.config.udp_port,
                    bind_addr = %bind_addr,
                    error = %err,
                    "Failed to initialize shared UDP mux; falling back to default ICE UDP behavior"
                );
                return Ok(None);
            }
        };
        let udp_mux: Arc<dyn UDPMux + Send + Sync> =
            UDPMuxDefault::new(UDPMuxParams::new(udp_socket));

        info!(
            udp_port = self.config.udp_port,
            bind_addr = %bind_addr,
            "Initialized shared UDP mux for SFU ICE traffic"
        );

        *guard = Some(udp_mux.clone());
        Ok(Some(udp_mux))
    }
}

impl Default for SFUManager {
    fn default() -> Self {
        let (tx, _) = mpsc::channel(VOICE_EVENT_CHANNEL_CAPACITY);
        // Create with empty config - this is mainly for testing
        Self {
            sfus: Arc::new(RwLock::new(HashMap::new())),
            config: CallsConfig::default(),
            voice_event_tx: tx,
            shared_udp_mux: Arc::new(RwLock::new(None)),
        }
    }
}
