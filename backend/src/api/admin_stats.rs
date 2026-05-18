//! Admin stats and health endpoints

use axum::{extract::State, routing::get, Json, Router};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::api::admin::require_admin;
use crate::api::AppState;
use crate::auth::AuthUser;
use crate::error::ApiResult;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/stats", get(get_stats))
        .route("/admin/health", get(get_health))
}

// ============ Stats & Health ============

#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_teams: i64,
    pub total_channels: i64,
    pub messages_24h: i64,
    pub files_count: i64,
}

pub async fn get_stats(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<SystemStats>> {
    require_admin(&auth)?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let active_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_teams: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let total_channels: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM channels")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));
    let messages_24h: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE created_at > NOW() - INTERVAL '24 hours'")
            .fetch_one(&state.db)
            .await
            .unwrap_or((0,));
    let files_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Ok(Json(SystemStats {
        total_users: total_users.0,
        active_users: active_users.0,
        total_teams: total_teams.0,
        total_channels: total_channels.0,
        messages_24h: messages_24h.0,
        files_count: files_count.0,
    }))
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub database: DatabaseHealth,
    pub storage: StorageHealth,
    pub redis: RedisHealth,
    pub disk: DiskHealth,
    pub websocket: WebSocketHealth,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub latency_ms: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct StorageHealth {
    pub connected: bool,
    #[serde(rename = "type")]
    pub storage_type: String,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct RedisHealth {
    pub connected: bool,
    pub latency_ms: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct DiskHealth {
    pub connected: bool,
    pub used_percent: u64,
    pub available_mb: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct WebSocketHealth {
    pub active_connections: u64,
}

struct HealthCache {
    timestamp: Instant,
    status: HealthStatus,
}

static HEALTH_CACHE: Mutex<Option<HealthCache>> = Mutex::new(None);
const HEALTH_CACHE_TTL: Duration = Duration::from_secs(30);

pub async fn get_health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<HealthStatus>> {
    require_admin(&auth)?;

    // Return cached result if still fresh
    {
        let cache = HEALTH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *cache {
            if cached.timestamp.elapsed() < HEALTH_CACHE_TTL {
                return Ok(Json(cached.status.clone()));
            }
        }
    }

    // Check DB
    let db_start = std::time::Instant::now();
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let db_latency = db_start.elapsed().as_millis() as u64;

    // Check Redis
    let redis_start = std::time::Instant::now();
    let redis_ok = check_redis_admin(&state.redis).await;
    let redis_latency = redis_start.elapsed().as_millis() as u64;

    // Check S3
    let s3_ok = state.s3_client.health_check().await;

    // Check disk (root mount)
    let disk = check_disk("/");

    let all_ok = db_ok && redis_ok && s3_ok && disk.connected;

    let status = HealthStatus {
        status: if all_ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        database: DatabaseHealth {
            connected: db_ok,
            latency_ms: db_latency,
        },
        storage: StorageHealth {
            connected: s3_ok,
            storage_type: "s3".to_string(),
        },
        redis: RedisHealth {
            connected: redis_ok,
            latency_ms: redis_latency,
        },
        disk,
        websocket: WebSocketHealth {
            active_connections: state.ws_hub.count_connections().await as u64,
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    // Update cache
    {
        let mut cache = HEALTH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        *cache = Some(HealthCache {
            timestamp: Instant::now(),
            status: status.clone(),
        });
    }

    Ok(Json(status))
}

async fn check_redis_admin(redis: &deadpool_redis::Pool) -> bool {
    crate::api::health::check_redis(redis).await
}

fn check_disk(path: &str) -> DiskHealth {
    #[cfg(unix)]
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let c_path = match std::ffi::CString::new(path) {
            Ok(p) => p,
            Err(_) => {
                return DiskHealth {
                    connected: false,
                    used_percent: 0,
                    available_mb: 0,
                }
            }
        };
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            // Cast needed for cross-platform compatibility (types vary by OS/arch)
            #[allow(clippy::unnecessary_cast)]
            let block_size = stat.f_frsize as u64;
            #[allow(clippy::unnecessary_cast)]
            let total = stat.f_blocks as u64 * block_size;
            #[allow(clippy::unnecessary_cast)]
            let available = stat.f_bavail as u64 * block_size;
            let used = total.saturating_sub(available);
            let used_percent = if total > 0 {
                ((used as f64 / total as f64) * 100.0) as u64
            } else {
                0
            };
            DiskHealth {
                connected: true,
                used_percent,
                available_mb: available / (1024u64 * 1024u64),
            }
        } else {
            DiskHealth {
                connected: false,
                used_percent: 0,
                available_mb: 0,
            }
        }
    }
    #[cfg(not(unix))]
    {
        // Non-Unix fallback: verify we can write a temp file
        let temp_path = std::env::temp_dir().join("rustchat_disk_check.tmp");
        let writable = std::fs::write(&temp_path, b"check").is_ok()
            && std::fs::remove_file(&temp_path).is_ok();
        DiskHealth {
            connected: writable,
            used_percent: 0,
            available_mb: 0,
        }
    }
}

