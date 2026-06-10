//! Prometheus metrics for rustchat
//!
//! Provides metrics for monitoring system health, performance,
//! and business-critical operations.

use prometheus::{
    register_counter, register_gauge, register_histogram, register_int_counter_vec,
    register_int_gauge, Counter, Gauge, Histogram, IntCounterVec, IntGauge,
};
use std::sync::LazyLock;
use std::time::Instant;

// ==================== HTTP Metrics ====================

/// HTTP request counter by method, path, and status
pub static HTTP_REQUESTS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_http_requests_total",
        "Total HTTP requests",
        &["method", "path", "status"]
    )
    .expect("metric can be created")
});

/// HTTP request duration histogram
pub static HTTP_REQUEST_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_http_request_duration_seconds",
        "HTTP request duration in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("metric can be created")
});

/// Active HTTP connections
pub static HTTP_ACTIVE_CONNECTIONS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustchat_http_active_connections",
        "Number of active HTTP connections"
    )
    .expect("metric can be created")
});

// ==================== WebSocket Metrics ====================

/// WebSocket connections total
pub static WS_CONNECTIONS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!(
        "rustchat_websocket_connections_total",
        "Total WebSocket connections established"
    )
    .expect("metric can be created")
});

/// Active WebSocket connections
pub static WS_ACTIVE_CONNECTIONS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustchat_websocket_active_connections",
        "Number of active WebSocket connections"
    )
    .expect("metric can be created")
});

/// WebSocket connections by user
pub static WS_CONNECTIONS_BY_USER: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustchat_websocket_connections_by_user",
        "Number of users with active WebSocket connections"
    )
    .expect("metric can be created")
});

/// WebSocket messages sent/received
pub static WS_MESSAGES_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_websocket_messages_total",
        "Total WebSocket messages",
        &["direction", "event_type"]
    )
    .expect("metric can be created")
});

/// WebSocket dropped/lagged message counter
pub static WS_DROPPED_MESSAGES_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_websocket_dropped_messages_total",
        "Total dropped websocket messages",
        &["reason"]
    )
    .expect("metric can be created")
});

/// WebSocket broadcast duration
pub static WS_BROADCAST_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_websocket_broadcast_duration_seconds",
        "WebSocket broadcast duration",
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    )
    .expect("metric can be created")
});

// ==================== Database Metrics ====================

/// Database query duration
pub static DB_QUERY_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_db_query_duration_seconds",
        "Database query duration",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("metric can be created")
});

/// Database connections in use
pub static DB_CONNECTIONS_ACTIVE: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustchat_db_connections_active",
        "Number of active database connections"
    )
    .expect("metric can be created")
});

/// Database connection pool saturation
pub static DB_POOL_SATURATION: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "rustchat_db_pool_saturation_ratio",
        "Database pool saturation (0-1)"
    )
    .expect("metric can be created")
});

// ==================== Redis Metrics ====================

/// Redis operation duration
pub static REDIS_OP_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_redis_operation_duration_seconds",
        "Redis operation duration",
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05]
    )
    .expect("metric can be created")
});

/// Redis connection errors
pub static REDIS_ERRORS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!(
        "rustchat_redis_errors_total",
        "Total Redis connection errors"
    )
    .expect("metric can be created")
});

// ==================== Authentication Metrics ====================

/// Authentication attempts
pub static AUTH_ATTEMPTS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_auth_attempts_total",
        "Total authentication attempts",
        &["method", "result"]
    )
    .expect("metric can be created")
});

/// Rate limit hits
pub static RATE_LIMIT_HITS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_rate_limit_hits_total",
        "Total rate limit hits",
        &["endpoint", "limit_type"]
    )
    .expect("metric can be created")
});

// ==================== Business Metrics ====================

/// Messages sent
pub static MESSAGES_SENT_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!("rustchat_messages_sent_total", "Total messages sent")
        .expect("metric can be created")
});

/// Files uploaded
pub static FILES_UPLOADED_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!("rustchat_files_uploaded_total", "Total files uploaded")
        .expect("metric can be created")
});

/// Active users (guage)
pub static ACTIVE_USERS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustchat_active_users", "Number of active users")
        .expect("metric can be created")
});

// ==================== Circuit Breaker Metrics ====================

/// Circuit breaker state changes
pub static CIRCUIT_BREAKER_STATE: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_circuit_breaker_state_changes_total",
        "Circuit breaker state changes",
        &["service", "state"]
    )
    .expect("metric can be created")
});

/// Circuit breaker failures
pub static CIRCUIT_BREAKER_FAILURES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "rustchat_circuit_breaker_failures_total",
        "Circuit breaker recorded failures",
        &["service"]
    )
    .expect("metric can be created")
});

// ==================== RAG / Knowledge Base Metrics ====================

/// Document indexing duration
pub static RAG_INDEXING_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_rag_indexing_duration_seconds",
        "Time spent indexing a document",
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("metric can be created")
});

/// Total documents indexed
pub static RAG_INDEXING_DOCUMENTS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!(
        "rustchat_rag_indexing_documents_total",
        "Total documents indexed"
    )
    .expect("metric can be created")
});

/// Total chunks indexed
pub static RAG_INDEXING_CHUNKS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!("rustchat_rag_indexing_chunks_total", "Total chunks indexed")
        .expect("metric can be created")
});

/// RAG semantic search duration
pub static RAG_SEARCH_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustchat_rag_search_duration_seconds",
        "Time spent on semantic search",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("metric can be created")
});

// ==================== Helper Functions ====================

/// Record an HTTP request metric
pub fn record_http_request(method: &str, path: &str, status: u16, duration: std::time::Duration) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION.observe(duration.as_secs_f64());
}

/// Record WebSocket connection
pub fn record_ws_connection() {
    WS_CONNECTIONS_TOTAL.inc();
    WS_ACTIVE_CONNECTIONS.inc();
}

/// Record WebSocket disconnection
pub fn record_ws_disconnection() {
    WS_ACTIVE_CONNECTIONS.dec();
}

/// Record WebSocket message
pub fn record_ws_message(direction: &str, event_type: &str) {
    WS_MESSAGES_TOTAL
        .with_label_values(&[direction, event_type])
        .inc();
}

/// Record dropped websocket messages
pub fn record_ws_dropped(reason: &str, count: u64) {
    WS_DROPPED_MESSAGES_TOTAL
        .with_label_values(&[reason])
        .inc_by(count);
}

/// Record authentication attempt
pub fn record_auth_attempt(method: &str, success: bool) {
    let result = if success { "success" } else { "failure" };
    AUTH_ATTEMPTS_TOTAL
        .with_label_values(&[method, result])
        .inc();
}

/// Record rate limit hit
pub fn record_rate_limit_hit(endpoint: &str, limit_type: &str) {
    RATE_LIMIT_HITS_TOTAL
        .with_label_values(&[endpoint, limit_type])
        .inc();
}

/// Create a timer for database queries
pub struct QueryTimer {
    start: Instant,
}

impl Default for QueryTimer {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl QueryTimer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Drop for QueryTimer {
    fn drop(&mut self) {
        DB_QUERY_DURATION.observe(self.start.elapsed().as_secs_f64());
    }
}

/// Create a timer for WebSocket broadcasts
pub struct BroadcastTimer {
    start: Instant,
}

impl Default for BroadcastTimer {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl BroadcastTimer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Drop for BroadcastTimer {
    fn drop(&mut self) {
        WS_BROADCAST_DURATION.observe(self.start.elapsed().as_secs_f64());
    }
}
