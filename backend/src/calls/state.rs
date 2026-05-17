//! Call state management for Mattermost Calls plugin
//!
//! Manages active calls, participants, and call metadata in memory.
//! For multi-node deployments, this should be backed by Redis.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;
use uuid::Uuid;

/// Represents an active call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallState {
    pub call_id: Uuid,
    pub channel_id: Uuid,
    pub owner_id: Uuid,
    pub host_id: Uuid,
    pub started_at: i64,
    pub participants: HashMap<Uuid, Participant>,
    pub screen_sharer: Option<Uuid>,
    pub thread_id: Option<Uuid>,
    #[serde(default)]
    pub dismissed_users: HashSet<Uuid>,
}

/// Represents a call participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub joined_at: i64,
    pub muted: bool,
    pub screen_sharing: bool,
    pub hand_raised: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CallStateBackend {
    /// Single-node mode only. State lives in-process.
    Memory,
    /// Redis-backed mode for multi-node. Falls back to in-memory on operation errors.
    Redis,
    /// Prefer Redis when available, otherwise use in-memory.
    #[default]
    Auto,
}

impl CallStateBackend {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "memory" | "in-memory" | "single-node" => Self::Memory,
            "redis" | "multi-node" => Self::Redis,
            "auto" | "" => Self::Auto,
            _ => Self::Auto,
        }
    }
}

/// Manages all active call state
pub struct CallStateManager {
    /// Active calls by call_id
    calls: RwLock<HashMap<Uuid, CallState>>,
    /// Index: channel_id -> call_id for quick lookup
    channel_index: RwLock<HashMap<Uuid, Uuid>>,
    /// Optional Redis backend for cross-node state.
    redis: Option<deadpool_redis::Pool>,
    /// Backend behavior mode.
    backend: CallStateBackend,
}

impl CallStateManager {
    /// Create a new call state manager
    pub fn new() -> Self {
        Self {
            calls: RwLock::new(HashMap::new()),
            channel_index: RwLock::new(HashMap::new()),
            redis: None,
            backend: CallStateBackend::Memory,
        }
    }

    /// Create a call state manager with backend selection.
    pub fn with_backend(redis: Option<deadpool_redis::Pool>, backend: CallStateBackend) -> Self {
        Self {
            calls: RwLock::new(HashMap::new()),
            channel_index: RwLock::new(HashMap::new()),
            redis,
            backend,
        }
    }

    pub fn configured_backend(&self) -> CallStateBackend {
        self.backend
    }

    pub fn active_backend(&self) -> CallStateBackend {
        if self.should_use_redis() {
            CallStateBackend::Redis
        } else {
            CallStateBackend::Memory
        }
    }

    /// Add a new call
    pub async fn add_call(&self, call: CallState) {
        self.persist_call(call).await;
    }

    /// Remove a call
    pub async fn remove_call(&self, call_id: Uuid) {
        let removed_call = self.remove_call_local(call_id).await;

        let Some(mut conn) = self.redis_conn().await else {
            return;
        };

        let channel_id = if let Some(call) = &removed_call {
            Some(call.channel_id)
        } else {
            self.redis_get_call(&mut conn, call_id)
                .await
                .ok()
                .flatten()
                .map(|c| c.channel_id)
        };

        if let Some(channel_id) = channel_id {
            let channel_key = Self::redis_channel_key(channel_id);
            let _: Result<usize, _> = conn.del(channel_key).await;
        }

        let call_key = Self::redis_call_key(call_id);
        let _: Result<usize, _> = conn.del(call_key).await;
        let _: Result<usize, _> = conn
            .srem(Self::redis_active_calls_key(), call_id.to_string())
            .await;
    }

    /// Get call by ID
    pub async fn get_call(&self, call_id: Uuid) -> Option<CallState> {
        if let Some(mut conn) = self.redis_conn().await {
            match self.redis_get_call(&mut conn, call_id).await {
                Ok(Some(call)) => {
                    self.upsert_call_local(call.clone()).await;
                    return Some(call);
                }
                Ok(None) => {}
                Err(err) => {
                    warn!(call_id = %call_id, error = %err, "redis get_call failed; using local call state");
                }
            }
        }

        let calls = self.calls.read().await;
        calls.get(&call_id).cloned()
    }

    /// Get call by channel ID
    pub async fn get_call_by_channel(&self, channel_id: &Uuid) -> Option<CallState> {
        if let Some(mut conn) = self.redis_conn().await {
            match self.redis_get_call_by_channel(&mut conn, *channel_id).await {
                Ok(Some(call)) => {
                    self.upsert_call_local(call.clone()).await;
                    return Some(call);
                }
                Ok(None) => {}
                Err(err) => {
                    warn!(channel_id = %channel_id, error = %err, "redis get_call_by_channel failed; using local call state");
                }
            }
        }

        let index = self.channel_index.read().await;
        let call_id = index.get(channel_id)?;

        let calls = self.calls.read().await;
        calls.get(call_id).cloned()
    }

    /// Add a participant to a call
    pub async fn add_participant(&self, call_id: Uuid, participant: Participant) {
        self.mutate_call(call_id, |call| {
            call.participants.insert(participant.user_id, participant);
        })
        .await;
    }

    /// Remove a participant from a call
    pub async fn remove_participant(&self, call_id: Uuid, user_id: Uuid) {
        self.mutate_call(call_id, |call| {
            call.participants.remove(&user_id);

            // If screen sharer left, clear screen sharer
            if call.screen_sharer == Some(user_id) {
                call.screen_sharer = None;
            }
        })
        .await;
    }

    /// Get a participant
    pub async fn get_participant(&self, call_id: Uuid, user_id: Uuid) -> Option<Participant> {
        self.get_call(call_id)
            .await
            .as_ref()
            .and_then(|call| call.participants.get(&user_id).cloned())
    }

    /// Get all participants in a call
    pub async fn get_participants(&self, call_id: Uuid) -> Vec<Participant> {
        self.get_call(call_id)
            .await
            .as_ref()
            .map(|call| call.participants.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Set participant muted status
    pub async fn set_muted(&self, call_id: Uuid, user_id: Uuid, muted: bool) {
        self.mutate_call(call_id, |call| {
            if let Some(participant) = call.participants.get_mut(&user_id) {
                participant.muted = muted;
            }
        })
        .await;
    }

    /// Set participant screen sharing status
    pub async fn set_screen_sharing(&self, call_id: Uuid, user_id: Uuid, sharing: bool) {
        self.mutate_call(call_id, |call| {
            if let Some(participant) = call.participants.get_mut(&user_id) {
                participant.screen_sharing = sharing;
            }
        })
        .await;
    }

    /// Set call screen sharer
    pub async fn set_screen_sharer(&self, call_id: Uuid, user_id: Option<Uuid>) {
        self.mutate_call(call_id, |call| {
            call.screen_sharer = user_id;
        })
        .await;
    }

    /// Set participant hand raised status
    pub async fn set_hand_raised(&self, call_id: Uuid, user_id: Uuid, raised: bool) {
        self.mutate_call(call_id, |call| {
            if let Some(participant) = call.participants.get_mut(&user_id) {
                participant.hand_raised = raised;
            }
        })
        .await;
    }

    /// Set call host
    pub async fn set_host(&self, call_id: Uuid, host_id: Uuid) {
        self.mutate_call(call_id, |call| {
            call.host_id = host_id;
        })
        .await;
    }

    /// Set call thread root post ID.
    pub async fn set_thread_id(&self, call_id: Uuid, thread_id: Option<Uuid>) {
        self.mutate_call(call_id, |call| {
            call.thread_id = thread_id;
        })
        .await;
    }

    /// Get call thread root post ID.
    pub async fn get_thread_id(&self, call_id: Uuid) -> Option<Uuid> {
        self.get_call(call_id).await.and_then(|call| call.thread_id)
    }

    /// Mark a user as having dismissed incoming call notifications.
    pub async fn dismiss_user_notification(&self, call_id: Uuid, user_id: Uuid) {
        self.mutate_call(call_id, |call| {
            call.dismissed_users.insert(user_id);
        })
        .await;
    }

    /// Check if a user has dismissed notifications for a call.
    pub async fn is_notification_dismissed(&self, call_id: Uuid, user_id: Uuid) -> bool {
        let calls = self.calls.read().await;
        calls
            .get(&call_id)
            .map(|call| call.dismissed_users.contains(&user_id))
            .unwrap_or(false)
    }

    /// Get all active calls (for cleanup/debugging)
    pub async fn get_all_calls(&self) -> Vec<CallState> {
        if let Some(mut conn) = self.redis_conn().await {
            match self.redis_get_all_calls(&mut conn).await {
                Ok(calls) => {
                    for call in &calls {
                        self.upsert_call_local(call.clone()).await;
                    }
                    if !calls.is_empty() {
                        return calls;
                    }
                }
                Err(err) => {
                    warn!(error = %err, "redis get_all_calls failed; using local call state");
                }
            }
        }

        let calls = self.calls.read().await;
        calls.values().cloned().collect()
    }

    /// Get participant count for a call
    pub async fn get_participant_count(&self, call_id: Uuid) -> usize {
        let calls = self.calls.read().await;
        calls
            .get(&call_id)
            .map(|call| call.participants.len())
            .unwrap_or(0)
    }

    /// End all calls (for shutdown)
    pub async fn end_all_calls(&self) {
        let mut calls = self.calls.write().await;
        let mut index = self.channel_index.write().await;

        calls.clear();
        index.clear();

        let Some(mut conn) = self.redis_conn().await else {
            return;
        };

        if let Ok(call_ids) = conn
            .smembers::<_, Vec<String>>(Self::redis_active_calls_key())
            .await
        {
            for call_id in call_ids {
                if let Ok(call_uuid) = Uuid::from_str(&call_id) {
                    if let Ok(Some(call_state)) = self.redis_get_call(&mut conn, call_uuid).await {
                        let _: Result<usize, _> = conn
                            .del(Self::redis_channel_key(call_state.channel_id))
                            .await;
                    }

                    let _: Result<usize, _> = conn.del(Self::redis_call_key(call_uuid)).await;
                }
            }
        }

        let _: Result<usize, _> = conn.del(Self::redis_active_calls_key()).await;
    }

    async fn mutate_call<F>(&self, call_id: Uuid, mutator: F)
    where
        F: FnOnce(&mut CallState),
    {
        if let Some(mut call) = self.get_call(call_id).await {
            mutator(&mut call);
            self.persist_call(call).await;
        }
    }

    async fn persist_call(&self, call: CallState) {
        let call_for_local = call.clone();
        self.upsert_call_local(call_for_local).await;

        let Some(mut conn) = self.redis_conn().await else {
            return;
        };

        if let Err(err) = self.redis_set_call(&mut conn, &call).await {
            warn!(
                call_id = %call.call_id,
                channel_id = %call.channel_id,
                error = %err,
                "redis persist_call failed; call state remains local"
            );
        }
    }

    async fn upsert_call_local(&self, call: CallState) {
        let mut calls = self.calls.write().await;
        let mut index = self.channel_index.write().await;
        index.insert(call.channel_id, call.call_id);
        calls.insert(call.call_id, call);
    }

    async fn remove_call_local(&self, call_id: Uuid) -> Option<CallState> {
        let mut calls = self.calls.write().await;
        let mut index = self.channel_index.write().await;
        let removed = calls.remove(&call_id);
        if let Some(call) = &removed {
            index.remove(&call.channel_id);
        }
        removed
    }

    async fn redis_conn(&self) -> Option<deadpool_redis::Connection> {
        if !self.should_use_redis() {
            return None;
        }

        let Some(pool) = &self.redis else {
            return None;
        };

        match pool.get().await {
            Ok(conn) => Some(conn),
            Err(err) => {
                warn!(error = %err, "redis pool unavailable; falling back to local call state");
                None
            }
        }
    }

    fn should_use_redis(&self) -> bool {
        match self.backend {
            CallStateBackend::Memory => false,
            CallStateBackend::Redis | CallStateBackend::Auto => self.redis.is_some(),
        }
    }

    async fn redis_set_call(
        &self,
        conn: &mut deadpool_redis::Connection,
        call: &CallState,
    ) -> Result<(), String> {
        let payload = serde_json::to_string(call).map_err(|e| e.to_string())?;

        let _: () = conn
            .set(Self::redis_call_key(call.call_id), payload)
            .await
            .map_err(|e| e.to_string())?;
        let _: () = conn
            .set(
                Self::redis_channel_key(call.channel_id),
                call.call_id.to_string(),
            )
            .await
            .map_err(|e| e.to_string())?;
        let _: () = conn
            .sadd(Self::redis_active_calls_key(), call.call_id.to_string())
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn redis_get_call(
        &self,
        conn: &mut deadpool_redis::Connection,
        call_id: Uuid,
    ) -> Result<Option<CallState>, String> {
        let payload: Option<String> = conn
            .get(Self::redis_call_key(call_id))
            .await
            .map_err(|e| e.to_string())?;
        payload
            .map(|value| serde_json::from_str::<CallState>(&value).map_err(|e| e.to_string()))
            .transpose()
    }

    async fn redis_get_call_by_channel(
        &self,
        conn: &mut deadpool_redis::Connection,
        channel_id: Uuid,
    ) -> Result<Option<CallState>, String> {
        let call_id: Option<String> = conn
            .get(Self::redis_channel_key(channel_id))
            .await
            .map_err(|e| e.to_string())?;
        let Some(call_id) = call_id else {
            return Ok(None);
        };
        let call_id = Uuid::from_str(&call_id).map_err(|e| e.to_string())?;
        self.redis_get_call(conn, call_id).await
    }

    async fn redis_get_all_calls(
        &self,
        conn: &mut deadpool_redis::Connection,
    ) -> Result<Vec<CallState>, String> {
        let call_ids: Vec<String> = conn
            .smembers(Self::redis_active_calls_key())
            .await
            .map_err(|e| e.to_string())?;
        let mut calls = Vec::new();
        for call_id in call_ids {
            let Ok(call_uuid) = Uuid::from_str(&call_id) else {
                continue;
            };
            if let Some(call) = self.redis_get_call(conn, call_uuid).await? {
                calls.push(call);
            }
        }
        Ok(calls)
    }

    fn redis_call_key(call_id: Uuid) -> String {
        format!("rustchat:calls:state:{call_id}")
    }

    fn redis_channel_key(channel_id: Uuid) -> String {
        format!("rustchat:calls:channel:{channel_id}")
    }

    fn redis_active_calls_key() -> &'static str {
        "rustchat:calls:active"
    }
}

impl Default for CallStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_backend_stores_and_reads_calls() {
        let manager = CallStateManager::new();

        let call_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        manager
            .add_call(CallState {
                call_id,
                channel_id,
                owner_id,
                host_id: owner_id,
                started_at: 1,
                participants: HashMap::new(),
                screen_sharer: None,
                thread_id: None,
                dismissed_users: HashSet::new(),
            })
            .await;

        let call = manager.get_call(call_id).await.expect("call should exist");
        assert_eq!(call.channel_id, channel_id);
        assert_eq!(manager.active_backend(), CallStateBackend::Memory);
    }

    #[tokio::test]
    async fn auto_backend_without_redis_falls_back_to_memory() {
        let manager = CallStateManager::with_backend(None, CallStateBackend::Auto);
        assert_eq!(manager.active_backend(), CallStateBackend::Memory);

        let call_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();
        manager
            .add_call(CallState {
                call_id,
                channel_id,
                owner_id: Uuid::new_v4(),
                host_id: Uuid::new_v4(),
                started_at: 2,
                participants: HashMap::new(),
                screen_sharer: None,
                thread_id: None,
                dismissed_users: HashSet::new(),
            })
            .await;

        assert!(manager.get_call(call_id).await.is_some());
    }

    #[test]
    fn parse_backend_mode_variants() {
        assert_eq!(CallStateBackend::parse("memory"), CallStateBackend::Memory);
        assert_eq!(CallStateBackend::parse("redis"), CallStateBackend::Redis);
        assert_eq!(CallStateBackend::parse("auto"), CallStateBackend::Auto);
        assert_eq!(CallStateBackend::parse("unknown"), CallStateBackend::Auto);
    }
}
