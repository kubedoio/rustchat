//! Per-agent rate limiter using in-memory sliding window.

use std::time::{Duration, Instant};
use dashmap::DashMap;
use uuid::Uuid;

/// Rate limit check result.
pub enum RateLimitResult {
    Allowed,
    Throttled { retry_after_secs: u64 },
}

pub struct AgentRateLimiter {
    /// agent_id -> (window_start, request_count)
    windows: DashMap<Uuid, (Instant, u32)>,
    max_requests_per_minute: u32,
    max_tokens_per_hour: u32,
    /// agent_id -> (hour_start, token_count)
    token_windows: DashMap<Uuid, (Instant, u32)>,
}

impl AgentRateLimiter {
    pub fn new(max_requests_per_minute: u32, max_tokens_per_hour: u32) -> Self {
        Self {
            windows: DashMap::new(),
            max_requests_per_minute,
            max_tokens_per_hour,
            token_windows: DashMap::new(),
        }
    }

    pub fn check_request(&self, agent_id: Uuid) -> RateLimitResult {
        let now = Instant::now();
        let window = Duration::from_secs(60);

        let mut entry = self.windows.entry(agent_id).or_insert((now, 0));
        let (start, count) = *entry.value();

        if now.duration_since(start) > window {
            // Window expired, reset
            *entry.value_mut() = (now, 1);
            RateLimitResult::Allowed
        } else if count >= self.max_requests_per_minute {
            let retry_after = window.as_secs() - now.duration_since(start).as_secs();
            RateLimitResult::Throttled { retry_after_secs: retry_after }
        } else {
            entry.value_mut().1 += 1;
            RateLimitResult::Allowed
        }
    }

    pub fn record_tokens(&self, agent_id: Uuid, tokens: u32) {
        let now = Instant::now();
        let window = Duration::from_secs(3600);

        let mut entry = self.token_windows.entry(agent_id).or_insert((now, 0));
        let (start, _count) = *entry.value();

        if now.duration_since(start) > window {
            *entry.value_mut() = (now, tokens);
        } else {
            entry.value_mut().1 += tokens;
        }
    }

    pub fn check_tokens(&self, agent_id: Uuid) -> RateLimitResult {
        let now = Instant::now();
        let window = Duration::from_secs(3600);

        if let Some(entry) = self.token_windows.get(&agent_id) {
            let (start, count) = *entry.value();
            if now.duration_since(start) <= window && count >= self.max_tokens_per_hour {
                let retry_after = window.as_secs() - now.duration_since(start).as_secs();
                return RateLimitResult::Throttled { retry_after_secs: retry_after };
            }
        }
        RateLimitResult::Allowed
    }
}
