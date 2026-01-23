//! Telemetry module for rustchat
//!
//! Provides structured logging and tracing setup.

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize telemetry (logging and tracing)
pub fn init(log_level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Create a tracing span for a request
#[macro_export]
macro_rules! request_span {
    ($request_id:expr) => {
        tracing::info_span!("request", request_id = %$request_id)
    };
}
