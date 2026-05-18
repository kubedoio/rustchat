//! SFU (Selective Forwarding Unit) for RustChat Calls
//!
//! Routes audio/video tracks between participants in a call.
//! Each participant sends one stream and receives streams from all other participants.
#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, trace, warn};
use uuid::Uuid;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::api::API;
use webrtc::ice::mdns::MulticastDnsMode;
use webrtc::ice::udp_mux::UDPMux;
use webrtc::ice::udp_network::UDPNetwork;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_remote::TrackRemote;

use crate::config::CallsConfig;

pub mod manager;
pub mod signaling;
pub mod tracks;

pub use manager::SFUManager;
use signaling::{SignalingMessage, SignalingServer};
use tracks::TrackManager;

pub const VOICE_EVENT_CHANNEL_CAPACITY: usize = 1024;
const SIGNALING_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug, Clone)]
pub enum VoiceEvent {
    VoiceOn { call_id: Uuid, session_id: Uuid },
    VoiceOff { call_id: Uuid, session_id: Uuid },
}

/// Represents a participant in the SFU
pub struct Participant {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub audio_track: Option<Arc<TrackLocalStaticRTP>>,
    pub video_track: Option<Arc<TrackLocalStaticRTP>>,
    pub screen_track: Option<Arc<TrackLocalStaticRTP>>,
    pub signaling_tx: mpsc::Sender<SignalingMessage>,
    pub is_screen_sharing: bool,
}

/// SFU manages all peer connections and routes media
#[allow(clippy::upper_case_acronyms)]
pub struct SFU {
    call_id: Uuid,
    config: CallsConfig,
    participants: Arc<RwLock<HashMap<Uuid, Participant>>>,
    track_manager: Arc<TrackManager>,
    signaling: Arc<SignalingServer>,
    pending_ice_candidates: Arc<RwLock<HashMap<Uuid, Vec<RTCIceCandidateInit>>>>,
    voice_event_tx: mpsc::Sender<VoiceEvent>,
    webrtc_api: Arc<API>,
    webrtc_api_fallback: Arc<API>,
}

fn media_stream_id(session_id: Uuid) -> String {
    format!("stream-{session_id}")
}

fn screen_stream_id(session_id: Uuid) -> String {
    format!("screen-stream-{session_id}")
}

impl SFU {
    fn resolve_ice_host_override(ice_host_override: &str) -> Option<IpAddr> {
        if let Ok(ip) = ice_host_override.parse::<IpAddr>() {
            return Some(ip);
        }

        // Allow hostnames by resolving once when SFU is created.
        let addrs: Vec<IpAddr> = (ice_host_override, 0)
            .to_socket_addrs()
            .ok()?
            .map(|addr| addr.ip())
            .collect();

        addrs
            .iter()
            .find(|ip| ip.is_ipv4())
            .copied()
            .or_else(|| addrs.first().copied())
    }

    /// Create a new SFU instance
    pub async fn new(
        call_id: Uuid,
        config: CallsConfig,
        voice_event_tx: mpsc::Sender<VoiceEvent>,
        shared_udp_mux: Option<Arc<dyn UDPMux + Send + Sync>>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let participants = Arc::new(RwLock::new(HashMap::new()));
        let track_manager = Arc::new(TrackManager::new());
        let signaling = Arc::new(SignalingServer::new());
        let pending_ice_candidates = Arc::new(RwLock::new(HashMap::new()));

        // Build primary API (with optional NAT 1:1 override) and fallback API (without NAT override).
        let webrtc_api = Self::build_webrtc_api(call_id, &config, shared_udp_mux.clone(), true)?;
        let webrtc_api_fallback = Self::build_webrtc_api(call_id, &config, shared_udp_mux, false)?;

        Ok(Arc::new(Self {
            call_id,
            config,
            participants,
            track_manager,
            signaling,
            pending_ice_candidates,
            voice_event_tx,
            webrtc_api,
            webrtc_api_fallback,
        }))
    }

    fn build_webrtc_api(
        call_id: Uuid,
        config: &CallsConfig,
        shared_udp_mux: Option<Arc<dyn UDPMux + Send + Sync>>,
        use_nat_override: bool,
    ) -> Result<Arc<API>, Box<dyn std::error::Error + Send + Sync>> {
        // Create media engine with codec support
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;

        // Create interceptor registry
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)?;

        // Configure ICE transport behavior for deployment networking.
        let mut setting_engine = SettingEngine::default();

        if let Some(udp_mux) = shared_udp_mux {
            setting_engine.set_udp_network(UDPNetwork::Muxed(udp_mux));
            info!(
                call_id = %call_id,
                udp_port = config.udp_port,
                "SFU configured with shared UDP mux"
            );
        }

        if use_nat_override {
            if let Some(ice_host_override) = config
                .ice_host_override
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                if let Some(resolved_ip) = Self::resolve_ice_host_override(ice_host_override) {
                    setting_engine.set_ice_multicast_dns_mode(MulticastDnsMode::Disabled);
                    setting_engine
                        .set_nat_1to1_ips(vec![resolved_ip.to_string()], RTCIceCandidateType::Host);
                    info!(
                        call_id = %call_id,
                        ice_host_override = %ice_host_override,
                        resolved_ip = %resolved_ip,
                        "SFU configured with ICE host override"
                    );
                } else {
                    warn!(
                        call_id = %call_id,
                        ice_host_override = %ice_host_override,
                        "Ignoring invalid RUSTCHAT_CALLS_ICE_HOST_OVERRIDE: expected IP or resolvable hostname"
                    );
                }
            }
        }

        Ok(Arc::new(
            APIBuilder::new()
                .with_setting_engine(setting_engine)
                .with_media_engine(m)
                .with_interceptor_registry(registry)
                .build(),
        ))
    }

    /// Add a new participant to the SFU
    pub async fn add_participant(
        &self,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<
        (Arc<RTCPeerConnection>, mpsc::Receiver<SignalingMessage>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Create ICE servers configuration
        let ice_servers = self.build_ice_servers();

        // Create peer connection configuration
        let rtc_config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        // Create peer connection (with fallback if NAT mapping is misconfigured).
        let peer_connection = match self
            .webrtc_api
            .new_peer_connection(rtc_config.clone())
            .await
        {
            Ok(pc) => Arc::new(pc),
            Err(primary_err) => {
                let err_text = primary_err.to_string();
                if err_text.to_ascii_lowercase().contains("1:1 nat ip mapping") {
                    warn!(
                        session_id = %session_id,
                        error = %err_text,
                        "Primary WebRTC API failed due to NAT mapping; retrying with fallback API"
                    );
                    Arc::new(
                        self.webrtc_api_fallback
                            .new_peer_connection(rtc_config)
                            .await?,
                    )
                } else {
                    return Err(Box::new(primary_err));
                }
            }
        };

        // Create signaling channel
        let (signaling_tx, signaling_rx) = mpsc::channel(SIGNALING_CHANNEL_CAPACITY);

        // Set up track handling
        self.setup_track_handlers(&peer_connection, user_id, session_id)
            .await?;

        // Set up ICE handling
        self.setup_ice_handlers(&peer_connection, user_id, signaling_tx.clone())
            .await?;

        // Create outgoing tracks for this participant
        // These tracks will receive media from all other participants
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "audio/opus".to_string(),
                ..Default::default()
            },
            format!("audio-{}", session_id),
            media_stream_id(session_id),
        ));

        let video_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "video/vp8".to_string(),
                ..Default::default()
            },
            format!("video-{}", session_id),
            media_stream_id(session_id),
        ));

        let screen_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "video/vp8".to_string(),
                ..Default::default()
            },
            format!("screen-{}", session_id),
            screen_stream_id(session_id),
        ));

        // Add tracks to peer connection
        info!(
            "Adding tracks to peer connection for session {}",
            session_id
        );
        peer_connection
            .add_track(audio_track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await?;
        peer_connection
            .add_track(video_track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await?;
        peer_connection
            .add_track(screen_track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await?;
        info!("Tracks added successfully for session {}", session_id);

        // Store participant
        let participant = Participant {
            user_id,
            session_id,
            peer_connection: peer_connection.clone(),
            audio_track: Some(audio_track),
            video_track: Some(video_track),
            screen_track: Some(screen_track),
            signaling_tx: signaling_tx.clone(),
            is_screen_sharing: false,
        };

        self.participants
            .write()
            .await
            .insert(session_id, participant);
        self.signaling
            .register_channel(session_id, signaling_tx.clone())
            .await;

        Ok((peer_connection, signaling_rx))
    }

    /// Remove a participant from the SFU
    pub async fn remove_participant(
        &self,
        session_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut participants = self.participants.write().await;

        if let Some(participant) = participants.remove(&session_id) {
            // Close peer connection
            participant.peer_connection.close().await?;

            // Remove tracks from manager
            self.track_manager
                .remove_participant_tracks(session_id)
                .await;
        }
        self.signaling.unregister_channel(session_id).await;
        self.pending_ice_candidates
            .write()
            .await
            .remove(&session_id);

        Ok(())
    }

    /// Recreate a participant's PeerConnection.
    ///
    /// This tears down the old (possibly dead) PeerConnection and builds
    /// a fresh one while keeping the same session_id.  Returns the new
    /// signaling receiver so the caller can spawn a new forwarder.
    pub async fn recreate_participant(
        &self,
        user_id: Uuid,
        session_id: Uuid,
    ) -> Result<
        (Arc<RTCPeerConnection>, mpsc::Receiver<SignalingMessage>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        info!(session_id = %session_id, "Recreating participant PeerConnection");

        // Close and remove the old participant if present
        {
            let mut participants = self.participants.write().await;
            if let Some(old) = participants.remove(&session_id) {
                let _ = old.peer_connection.close().await;
            }
        }
        self.signaling.unregister_channel(session_id).await;
        self.track_manager
            .remove_participant_tracks(session_id)
            .await;
        self.pending_ice_candidates
            .write()
            .await
            .remove(&session_id);

        // Re-add with fresh PeerConnection (reuses add_participant logic)
        self.add_participant(user_id, session_id).await
    }

    /// Check if a participant session is present in this SFU.
    pub async fn has_participant(&self, session_id: Uuid) -> bool {
        self.participants.read().await.contains_key(&session_id)
    }

    /// Handle offer from client
    pub async fn handle_offer(
        &self,
        session_id: Uuid,
        offer: RTCSessionDescription,
    ) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
        info!(session_id = %session_id, "SFU handle_offer start");

        // Clone the Arc<PeerConnection> so we can release the lock before the
        // 500ms ICE-gathering sleep.
        let pc = {
            let participants = self.participants.read().await;
            let participant = participants
                .get(&session_id)
                .ok_or("Participant not found")?;
            participant.peer_connection.clone()
        }; // read lock released here

        // Set remote description (the offer)
        info!(session_id = %session_id, "Setting remote description");
        pc.set_remote_description(offer).await?;

        info!(session_id = %session_id, "Flushing pending ICE candidates");
        self.flush_pending_ice_candidates(session_id, &pc).await?;

        // Create answer
        info!(session_id = %session_id, "Creating answer");
        let answer = pc.create_answer(None).await?;

        // Set local description
        info!(session_id = %session_id, "Setting local description");
        pc.set_local_description(answer.clone()).await?;

        // Wait for ICE gathering to complete (or timeout)
        // We wait longer (2 seconds) to allow STUN/TURN candidates to be gathered
        info!(session_id = %session_id, "Waiting for ICE gathering (2 seconds)");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        // Get the final answer with ICE candidates
        info!(session_id = %session_id, "Getting final answer");
        let final_answer = pc.local_description().await.ok_or("No local description")?;

        info!(
            session_id = %session_id,
            sdp_length = final_answer.sdp.len(),
            "Answer SDP generated"
        );

        info!(session_id = %session_id, "SFU handle_offer success");
        Ok(final_answer)
    }

    /// Handle ICE candidate from client
    /// In an SFU architecture, each client only connects to the SFU, not to other clients.
    /// The client's ICE candidate should only be added to the SFU's peer connection for that client.
    /// The SFU generates its own candidates (via on_ice_candidate handler) and sends them to the client.
    pub async fn handle_ice_candidate(
        &self,
        session_id: Uuid,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let candidate_init =
            Self::parse_client_ice_candidate(candidate.clone(), sdp_mid.clone(), sdp_mline_index)?;

        let participants = self.participants.read().await;
        let participant = participants
            .get(&session_id)
            .ok_or("Participant not found")?;

        if participant
            .peer_connection
            .remote_description()
            .await
            .is_none()
        {
            // Remote description not set yet, queue the candidate
            drop(participants);
            self.pending_ice_candidates
                .write()
                .await
                .entry(session_id)
                .or_default()
                .push(candidate_init);
            info!(
                session_id = %session_id,
                candidate_len = candidate.len(),
                "ICE candidate queued (remote description not set yet)"
            );
        } else {
            // Add candidate to the SFU's peer connection for this client
            participant
                .peer_connection
                .add_ice_candidate(candidate_init)
                .await?;
            info!(
                session_id = %session_id,
                candidate_len = candidate.len(),
                "ICE candidate added to peer connection"
            );
        }

        Ok(())
    }

    /// Build ICE servers for the SFU's server-side PeerConnection.
    ///
    /// In containerized/Docker environments, the SFU needs STUN servers to discover
    /// its public IP address (srflx candidates). Without this, clients outside the
    /// container network cannot connect to the SFU's internal host candidates.
    ///
    /// We also add TURN servers as a fallback for relay when direct connectivity fails.
    fn build_ice_servers(&self) -> Vec<RTCIceServer> {
        let mut servers = vec![];

        // Add STUN servers so the SFU can discover its public IP address
        // This is critical in Docker/containerized environments where the container
        // has internal IPs that are not reachable from outside
        if !self.config.stun_servers.is_empty() {
            for stun_url in &self.config.stun_servers {
                if !stun_url.trim().is_empty() {
                    servers.push(RTCIceServer {
                        urls: vec![stun_url.clone()],
                        ..Default::default()
                    });
                }
            }
        } else {
            // Default STUN servers if none configured
            servers.push(RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            });
            servers.push(RTCIceServer {
                urls: vec!["stun:stun1.l.google.com:19302".to_string()],
                ..Default::default()
            });
        }

        // Add TURN server if enabled (needed for relay through NAT/firewall)
        if self.config.turn_server_enabled
            && !self.config.turn_server_url.trim().is_empty()
            && !self.config.turn_server_username.trim().is_empty()
            && !self.config.turn_server_credential.trim().is_empty()
        {
            #[allow(clippy::needless_update)]
            servers.push(RTCIceServer {
                urls: vec![self.config.turn_server_url.clone()],
                username: self.config.turn_server_username.clone(),
                credential: self.config.turn_server_credential.clone(),
                ..Default::default()
            });
        }

        info!(
            stun_count = servers
                .len()
                .saturating_sub(if self.config.turn_server_enabled {
                    1
                } else {
                    0
                }),
            turn_enabled = self.config.turn_server_enabled,
            "ICE servers configured for SFU"
        );

        servers
    }

    /// Set up track handlers for a peer connection
    async fn setup_track_handlers(
        &self,
        peer_connection: &Arc<RTCPeerConnection>,
        _user_id: Uuid,
        session_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let track_manager = self.track_manager.clone();
        let participants = self.participants.clone();
        let call_id = self.call_id;
        let voice_event_tx = self.voice_event_tx.clone();

        // Handle incoming tracks
        peer_connection.on_track(Box::new(
            move |track: Arc<TrackRemote>,
                  _receiver: Arc<RTCRtpReceiver>,
                  _transceiver: Arc<RTCRtpTransceiver>| {
                let track_manager = track_manager.clone();
                let participants = participants.clone();
                let session_id = session_id;
                let call_id = call_id;
                let voice_event_tx = voice_event_tx.clone();

                tokio::spawn(async move {
                    info!(
                        session_id = %session_id,
                        track_id = %track.id(),
                        track_kind = ?track.kind(),
                        stream_id = %track.stream_id(),
                        "New track received from participant"
                    );

                    // Register the track
                    track_manager
                        .register_track(session_id, track.clone())
                        .await;

                    // Forward track to other participants
                    Self::forward_track(
                        call_id,
                        track,
                        track_manager,
                        participants,
                        session_id,
                        voice_event_tx,
                    )
                    .await;
                });

                Box::pin(async {})
            },
        ));

        Ok(())
    }

    /// Forward a track to all other participants
    async fn forward_track(
        call_id: Uuid,
        track: Arc<TrackRemote>,
        track_manager: Arc<TrackManager>,
        participants: Arc<RwLock<HashMap<Uuid, Participant>>>,
        sender_session_id: Uuid,
        voice_event_tx: mpsc::Sender<VoiceEvent>,
    ) {
        // Read RTP packets from the track
        let mut rtp_buffer = vec![0u8; 1500];
        let mut packet_count = 0u64;
        let mut last_log_at = tokio::time::Instant::now();

        let mut voice_on = false;
        let mut last_packet_at = tokio::time::Instant::now();

        // Check if sender is screen sharing - this is set via the /screen-share API endpoint
        let sender_is_screen_sharing = participants
            .read()
            .await
            .get(&sender_session_id)
            .map(|p| p.is_screen_sharing)
            .unwrap_or(false);

        // Also check stream_id/track_label as fallback for older clients.
        // This is currently debug-oriented because forwarding for video now targets both tracks.
        let stream_id_raw = track.stream_id();
        let track_id_raw = track.id();
        let stream_id_lower = stream_id_raw.to_lowercase();
        let track_id_lower = track_id_raw.to_lowercase();
        let stream_id_contains_screen = stream_id_lower.contains("screen")
            || stream_id_lower.contains("display")
            || stream_id_lower.contains("share");
        let track_label_contains_screen = track_id_lower.contains("screen")
            || track_id_lower.contains("display")
            || track_id_lower.contains("share");

        // Determine if this is a screen share track
        // Priority: API state > stream_id detection > track_label detection
        let is_screen_share =
            if track.kind() == webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video {
                sender_is_screen_sharing || stream_id_contains_screen || track_label_contains_screen
            } else {
                false
            };

        info!(
            call_id = %call_id,
            sender_session_id = %sender_session_id,
            track_id = %track_id_raw,
            track_kind = ?track.kind(),
            stream_id = %stream_id_raw,
            sender_is_screen_sharing = sender_is_screen_sharing,
            stream_id_contains_screen = stream_id_contains_screen,
            track_label_contains_screen = track_label_contains_screen,
            is_screen_share = is_screen_share,
            "Starting track forwarding loop"
        );

        loop {
            // Read RTP packet
            match track.read(&mut rtp_buffer).await {
                Ok((packet, _)) => {
                    packet_count += 1;

                    // Log packet stats periodically
                    if last_log_at.elapsed() > tokio::time::Duration::from_secs(10) {
                        info!(
                            call_id = %call_id,
                            sender_session_id = %sender_session_id,
                            track_id = %track.id(),
                            packet_count = packet_count,
                            "Track forwarding stats"
                        );
                        last_log_at = tokio::time::Instant::now();
                    }

                    // Update voice activity if it's an audio track
                    if track.kind() == webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio {
                        last_packet_at = tokio::time::Instant::now();
                        if !voice_on {
                            voice_on = true;
                            try_send_voice_event(
                                &voice_event_tx,
                                VoiceEvent::VoiceOn {
                                    call_id,
                                    session_id: sender_session_id,
                                },
                            );
                        }
                    }
                    // Get the packet data length
                    let n = packet.payload.len();
                    if n == 0 {
                        continue;
                    }

                    // Forward to other participants
                    let participants_guard = participants.read().await;
                    let other_participant_count = participants_guard.len().saturating_sub(1);

                    if other_participant_count > 0 {
                        for (session_id, participant) in participants_guard.iter() {
                            if *session_id == sender_session_id {
                                continue; // Don't send back to sender
                            }

                            // Forward based on track kind
                            match track.kind() {
                                webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio => {
                                    if let Some(audio_track) = &participant.audio_track {
                                        if let Err(e) = audio_track.write_rtp(&packet).await {
                                            trace!(
                                                session_id = %session_id,
                                                error = %e,
                                                "Failed to write audio RTP packet"
                                            );
                                        }
                                    }
                                }
                                webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video => {
                                    // Race-safe fallback: duplicate all video RTP packets to both destinations.
                                    // This avoids misrouting if /screen-share state arrives after the track.
                                    if packet_count % 100 == 1 {
                                        info!(
                                            sender_session_id = %sender_session_id,
                                            receiver_session_id = %session_id,
                                            stream_id = %stream_id_raw,
                                            track_id = %track_id_raw,
                                            sender_is_screen_sharing = sender_is_screen_sharing,
                                            has_video_track = participant.video_track.is_some(),
                                            has_screen_track = participant.screen_track.is_some(),
                                            "Forwarding video RTP to all available receiver video/screen tracks"
                                        );
                                    }

                                    if let Some(video_track) = &participant.video_track {
                                        if let Err(e) = video_track.write_rtp(&packet).await {
                                            trace!(
                                                session_id = %session_id,
                                                error = %e,
                                                "Failed to write video RTP packet"
                                            );
                                        }
                                    }

                                    if let Some(screen_track) = &participant.screen_track {
                                        if let Err(e) = screen_track.write_rtp(&packet).await {
                                            trace!(
                                                session_id = %session_id,
                                                error = %e,
                                                "Failed to write screen RTP packet"
                                            );
                                        }
                                    }

                                    if participant.video_track.is_none()
                                        && participant.screen_track.is_none()
                                        && packet_count % 100 == 1
                                    {
                                        warn!(
                                            receiver_session_id = %session_id,
                                            "No receiver video/screen tracks available for video RTP forwarding"
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(_) => {
                    // Track closed or error
                    break;
                }
            }

            // Check for voice silence (no packets for 500ms)
            if voice_on
                && track.kind() == webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio
                && last_packet_at.elapsed() > tokio::time::Duration::from_millis(500)
            {
                voice_on = false;
                try_send_voice_event(
                    &voice_event_tx,
                    VoiceEvent::VoiceOff {
                        call_id,
                        session_id: sender_session_id,
                    },
                );
            }
        }

        // Unregister track when done
        track_manager
            .unregister_track(sender_session_id, &track.id())
            .await;

        info!(
            call_id = %call_id,
            sender_session_id = %sender_session_id,
            track_id = %track.id(),
            total_packets = packet_count,
            "Track forwarding ended"
        );
    }

    /// Set up ICE handlers for a peer connection
    async fn setup_ice_handlers(
        &self,
        peer_connection: &Arc<RTCPeerConnection>,
        _user_id: Uuid,
        signaling_tx: mpsc::Sender<SignalingMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Clone signaling_tx for each handler since it will be moved into closures
        let signaling_tx_ice = signaling_tx.clone();
        let signaling_tx_ice_state = signaling_tx.clone();
        let signaling_tx_conn_state = signaling_tx;

        // Handle ICE candidates
        peer_connection.on_ice_candidate(Box::new(
            move |candidate: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| {
                if let Some(candidate) = candidate {
                    // Send ICE candidate to client via signaling
                    let candidate_json = candidate.to_json().ok();
                    let candidate_str = candidate_json
                        .as_ref()
                        .map(|j| j.candidate.clone())
                        .unwrap_or_default();

                    info!(
                        candidate = %candidate_str.chars().take(80).collect::<String>(),
                        candidate_len = candidate_str.len(),
                        "SFU generated ICE candidate - sending to client"
                    );

                    try_send_signaling(
                        &signaling_tx_ice,
                        SignalingMessage::IceCandidate {
                            candidate: candidate_str,
                            sdp_mid: candidate_json.as_ref().and_then(|j| j.sdp_mid.clone()),
                            sdp_mline_index: candidate_json
                                .as_ref()
                                .and_then(|j| j.sdp_mline_index),
                            username_fragment: candidate_json
                                .as_ref()
                                .and_then(|j| j.username_fragment.clone()),
                        },
                    );
                } else {
                    info!("ICE candidate gathering completed (null candidate received)");
                }

                Box::pin(async {})
            },
        ));

        // Handle ICE connection state changes
        peer_connection.on_ice_connection_state_change(Box::new(
            move |state: webrtc::ice_transport::ice_connection_state::RTCIceConnectionState| {
                try_send_signaling(
                    &signaling_tx_ice_state,
                    SignalingMessage::IceConnectionState {
                        state: format!("{:?}", state),
                    },
                );
                Box::pin(async {})
            },
        ));

        // Handle peer connection state changes (using ICE connection state as proxy)
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState| {
                try_send_signaling(
                    &signaling_tx_conn_state,
                    SignalingMessage::ConnectionState {
                        state: format!("{:?}", state),
                    },
                );
                Box::pin(async {})
            },
        ));

        Ok(())
    }

    /// Get all participant sessions in the SFU
    pub async fn get_participant_sessions(&self) -> Vec<Uuid> {
        self.participants.read().await.keys().cloned().collect()
    }

    /// Get participant count
    pub async fn get_participant_count(&self) -> usize {
        self.participants.read().await.len()
    }

    /// Set screen sharing state for a participant
    pub async fn set_screen_sharing(&self, session_id: Uuid, is_sharing: bool) {
        let mut participants = self.participants.write().await;
        if let Some(participant) = participants.get_mut(&session_id) {
            participant.is_screen_sharing = is_sharing;
            info!(
                session_id = %session_id,
                is_sharing = is_sharing,
                "Screen sharing state updated"
            );
        }
    }

    /// Get screen sharing state for a participant
    pub async fn is_screen_sharing(&self, session_id: Uuid) -> bool {
        self.participants
            .read()
            .await
            .get(&session_id)
            .map(|p| p.is_screen_sharing)
            .unwrap_or(false)
    }

    /// Get a new signaling receiver for an existing participant.
    /// This is used to spawn a signaling forwarder for HTTP-based clients
    /// that need to receive ICE candidates via WebSocket.
    pub async fn get_signaling_receiver(
        &self,
        session_id: Uuid,
    ) -> Option<mpsc::Receiver<SignalingMessage>> {
        let participants = self.participants.read().await;

        if let Some(_participant) = participants.get(&session_id) {
            // Create a new signaling channel
            let (tx, rx) = mpsc::channel(SIGNALING_CHANNEL_CAPACITY);

            // Register the new channel with the signaling server
            // Drop the read lock first to avoid deadlock
            drop(participants);
            self.signaling.register_channel(session_id, tx).await;

            info!(
                session_id = %session_id,
                "Created new signaling channel for existing participant"
            );

            Some(rx)
        } else {
            None
        }
    }

    async fn flush_pending_ice_candidates(
        &self,
        session_id: Uuid,
        peer_connection: &Arc<RTCPeerConnection>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pending = self
            .pending_ice_candidates
            .write()
            .await
            .remove(&session_id);
        if let Some(candidates) = pending {
            for candidate in candidates {
                peer_connection.add_ice_candidate(candidate).await?;
            }
        }
        Ok(())
    }

    fn parse_client_ice_candidate(
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Result<RTCIceCandidateInit, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Debug, Deserialize)]
        struct BrowserIceCandidate {
            candidate: String,
            #[serde(rename = "sdpMid")]
            sdp_mid: Option<String>,
            #[serde(rename = "sdpMLineIndex")]
            sdp_mline_index: Option<u16>,
            #[serde(rename = "usernameFragment")]
            username_fragment: Option<String>,
        }

        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            return Err("ICE candidate cannot be empty".into());
        }

        if trimmed.starts_with('{') {
            let parsed: BrowserIceCandidate = serde_json::from_str(trimmed)?;
            return Ok(RTCIceCandidateInit {
                candidate: parsed.candidate,
                sdp_mid: parsed.sdp_mid,
                sdp_mline_index: parsed.sdp_mline_index,
                username_fragment: parsed.username_fragment,
            });
        }

        Ok(RTCIceCandidateInit {
            candidate: trimmed.to_string(),
            sdp_mid,
            sdp_mline_index,
            username_fragment: None,
        })
    }
}

impl Drop for SFU {
    fn drop(&mut self) {
        // Cleanup when SFU is dropped
        // In production, you'd want to properly close all peer connections
    }
}

fn try_send_signaling(tx: &mpsc::Sender<SignalingMessage>, message: SignalingMessage) {
    match tx.try_send(message) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!("Signaling channel full; dropping signaling message");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            warn!("Signaling channel closed; dropping signaling message");
        }
    }
}

fn try_send_voice_event(tx: &mpsc::Sender<VoiceEvent>, event: VoiceEvent) {
    match tx.try_send(event) {
        Ok(()) => {}
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!("Voice event channel full; dropping voice activity event");
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            warn!("Voice event channel closed; dropping voice activity event");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{media_stream_id, screen_stream_id, SFU};
    use uuid::Uuid;

    #[test]
    fn parses_browser_json_ice_candidate() {
        let candidate = r#"{"candidate":"candidate:1 1 UDP 1 127.0.0.1 5000 typ host","sdpMid":"0","sdpMLineIndex":0}"#;
        let parsed = SFU::parse_client_ice_candidate(candidate.to_string(), None, None)
            .expect("candidate should parse");

        assert_eq!(
            parsed.candidate,
            "candidate:1 1 UDP 1 127.0.0.1 5000 typ host"
        );
        assert_eq!(parsed.sdp_mid.as_deref(), Some("0"));
        assert_eq!(parsed.sdp_mline_index, Some(0));
    }

    #[test]
    fn parses_plain_ice_candidate_with_explicit_indexes() {
        let parsed = SFU::parse_client_ice_candidate(
            "candidate:2 1 UDP 1 192.168.1.10 6000 typ host".to_string(),
            Some("audio".to_string()),
            Some(1),
        )
        .expect("candidate should parse");

        assert_eq!(
            parsed.candidate,
            "candidate:2 1 UDP 1 192.168.1.10 6000 typ host"
        );
        assert_eq!(parsed.sdp_mid.as_deref(), Some("audio"));
        assert_eq!(parsed.sdp_mline_index, Some(1));
    }

    #[test]
    fn screen_stream_id_is_distinct_from_media_stream_id() {
        let session_id = Uuid::new_v4();
        assert_ne!(media_stream_id(session_id), screen_stream_id(session_id));
    }
}
