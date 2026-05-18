//! Track Management for SFU
//!
//! Manages audio/video tracks from participants and handles track forwarding.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use webrtc::track::track_remote::TrackRemote;

/// Track information
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_id: String,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub kind: TrackKind,
    pub track: Arc<TrackRemote>,
}

/// Track kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Audio,
    Video,
    Screen,
}

impl From<webrtc::rtp_transceiver::rtp_codec::RTPCodecType> for TrackKind {
    fn from(kind: webrtc::rtp_transceiver::rtp_codec::RTPCodecType) -> Self {
        match kind {
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio => TrackKind::Audio,
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video => TrackKind::Video,
            _ => TrackKind::Video, // Default to video for unknown types
        }
    }
}

/// Track manager handles all tracks in the SFU
pub struct TrackManager {
    /// Active tracks: track_id -> TrackInfo
    tracks: Arc<RwLock<HashMap<String, TrackInfo>>>,

    /// Tracks by session: session_id -> [track_ids]
    session_tracks: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,
}

impl TrackManager {
    /// Create a new track manager
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(RwLock::new(HashMap::new())),
            session_tracks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new track
    pub async fn register_track(&self, session_id: Uuid, track: Arc<TrackRemote>) {
        let track_id = track.id();

        // Determine track kind from the track stream ID or codec
        let kind = if track.stream_id().contains("screen") {
            TrackKind::Screen
        } else {
            TrackKind::from(track.kind())
        };

        let track_info = TrackInfo {
            track_id: track_id.clone(),
            session_id,
            user_id: session_id, // Using session_id as user_id for now
            kind,
            track: track.clone(),
        };

        // Add to tracks map
        self.tracks
            .write()
            .await
            .insert(track_id.clone(), track_info);

        // Add to session tracks
        self.session_tracks
            .write()
            .await
            .entry(session_id)
            .or_insert_with(Vec::new)
            .push(track_id);
    }

    /// Unregister a track
    pub async fn unregister_track(&self, session_id: Uuid, track_id: &str) {
        // Remove from tracks map
        self.tracks.write().await.remove(track_id);

        // Remove from session tracks
        if let Some(tracks) = self.session_tracks.write().await.get_mut(&session_id) {
            tracks.retain(|id| id != track_id);
        }
    }

    /// Remove all tracks for a participant
    pub async fn remove_participant_tracks(&self, session_id: Uuid) {
        // Get all track IDs for this session
        let track_ids = {
            let session_tracks = self.session_tracks.read().await;
            session_tracks.get(&session_id).cloned().unwrap_or_default()
        };

        // Remove all tracks
        let mut tracks = self.tracks.write().await;
        for track_id in &track_ids {
            tracks.remove(track_id);
        }

        // Remove session entry
        self.session_tracks.write().await.remove(&session_id);
    }

    /// Get all tracks
    pub async fn get_all_tracks(&self) -> Vec<TrackInfo> {
        self.tracks.read().await.values().cloned().collect()
    }

    /// Get tracks by session
    pub async fn get_tracks_by_session(&self, session_id: Uuid) -> Vec<TrackInfo> {
        let track_ids = {
            self.session_tracks
                .read()
                .await
                .get(&session_id)
                .cloned()
                .unwrap_or_default()
        };

        let tracks = self.tracks.read().await;
        track_ids
            .iter()
            .filter_map(|id| tracks.get(id).cloned())
            .collect()
    }

    /// Get tracks excluding a session (for forwarding)
    pub async fn get_tracks_excluding_session(&self, exclude_session_id: Uuid) -> Vec<TrackInfo> {
        self.tracks
            .read()
            .await
            .values()
            .filter(|t| t.session_id != exclude_session_id)
            .cloned()
            .collect()
    }

    /// Get track count
    pub async fn get_track_count(&self) -> usize {
        self.tracks.read().await.len()
    }

    /// Get participant count (by unique sessions)
    pub async fn get_participant_count(&self) -> usize {
        self.session_tracks.read().await.len()
    }
}

impl Default for TrackManager {
    fn default() -> Self {
        Self::new()
    }
}
