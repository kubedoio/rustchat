//! Reliability middleware for external service calls
//!
//! Provides retry logic, circuit breakers, and timeouts for external
//! dependencies like OIDC, S3, and email providers.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tracing::{debug, error, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Failure threshold reached - requests are rejected
    Open,
    /// Testing if service has recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to wait before attempting recovery
    pub recovery_timeout: Duration,
    /// Number of successes required to close circuit
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
        }
    }
}

/// Circuit breaker for protecting external calls
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<CircuitState>,
    failures: Mutex<u32>,
    successes: Mutex<u32>,
    last_failure: Mutex<Option<Instant>>,
    name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            state: Mutex::new(CircuitState::Closed),
            failures: Mutex::new(0),
            successes: Mutex::new(0),
            last_failure: Mutex::new(None),
            name: name.into(),
        })
    }

    /// Create with default config
    pub fn default_config(name: impl Into<String>) -> Arc<Self> {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Execute a function with circuit breaker protection
    pub async fn execute<T, E, F, Fut>(self: &Arc<Self>, f: F) -> Result<T, CircuitError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        // Check if circuit allows the request
        if let Err(CircuitError::Open) = self.check_state().await {
            return Err(CircuitError::Open);
        }

        // Execute the request
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(err) => {
                self.on_failure().await;
                Err(CircuitError::Inner(err))
            }
        }
    }

    /// Check current circuit state and transition if needed
    pub async fn check_state(&self) -> Result<(), CircuitError<()>> {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if recovery timeout has passed
                let should_attempt = {
                    let last = self.last_failure.lock().await;
                    last.map(|t| t.elapsed() >= self.config.recovery_timeout)
                        .unwrap_or(true)
                };

                if should_attempt {
                    info!(
                        circuit = %self.name,
                        "Circuit breaker entering half-open state"
                    );
                    *state = CircuitState::HalfOpen;
                    *self.successes.lock().await = 0;
                    Ok(())
                } else {
                    Err(CircuitError::Open)
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful request
    pub async fn on_success(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::HalfOpen => {
                let mut successes = self.successes.lock().await;
                *successes += 1;

                if *successes >= self.config.success_threshold {
                    info!(
                        circuit = %self.name,
                        "Circuit breaker closed after recovery"
                    );
                    *state = CircuitState::Closed;
                    *self.failures.lock().await = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failures on success in closed state
                let mut failures = self.failures.lock().await;
                if *failures > 0 {
                    *failures = 0;
                }
            }
            _ => {}
        }
    }

    /// Record a failed request
    pub async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        let mut failures = self.failures.lock().await;
        *failures += 1;
        *self.last_failure.lock().await = Some(Instant::now());

        match *state {
            CircuitState::Closed if *failures >= self.config.failure_threshold => {
                warn!(
                    circuit = %self.name,
                    failures = *failures,
                    "Circuit breaker opened due to failures"
                );
                *state = CircuitState::Open;
            }
            CircuitState::HalfOpen => {
                // Failed in half-open, go back to open
                warn!(
                    circuit = %self.name,
                    "Circuit breaker re-opened after failed recovery attempt"
                );
                *state = CircuitState::Open;
            }
            _ => {}
        }
    }

    /// Get current state (for monitoring)
    pub async fn state(&self) -> CircuitState {
        *self.state.lock().await
    }
}

/// Circuit breaker errors
#[derive(Debug, Clone)]
pub enum CircuitError<E> {
    /// Circuit is open
    Open,
    /// Inner error from the protected function
    Inner(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitError::Open => write!(f, "Circuit breaker is open"),
            CircuitError::Inner(e) => write!(f, "Inner error: {}", e),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for CircuitError<E> {}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to retry on specific errors
    pub retry_if: RetryCondition,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            retry_if: RetryCondition::Default,
        }
    }
}

/// Retry condition
#[derive(Debug, Clone, Copy)]
pub enum RetryCondition {
    /// Always retry
    Always,
    /// Never retry
    Never,
    /// Default condition (retry on transient errors)
    Default,
}

impl RetryCondition {
    fn should_retry<E>(&self, _error: &E) -> bool {
        match self {
            RetryCondition::Always => true,
            RetryCondition::Never => false,
            RetryCondition::Default => {
                // Default: retry on network errors, 5xx responses, etc.
                // In a real implementation, check error type
                true
            }
        }
    }
}

/// Execute a function with retry logic
pub async fn with_retry<T, E, F, Fut>(config: &RetryConfig, f: F) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut attempts = 0;
    let mut delay = config.initial_delay;

    loop {
        attempts += 1;

        match f().await {
            Ok(result) => {
                if attempts > 1 {
                    debug!(attempts = attempts, "Request succeeded after retries");
                }
                return Ok(result);
            }
            Err(err) => {
                if attempts >= config.max_attempts {
                    error!(
                        attempts = attempts,
                        error = ?err,
                        "Request failed after all retry attempts"
                    );
                    return Err(err);
                }

                if !config.retry_if.should_retry(&err) {
                    return Err(err);
                }

                warn!(
                    attempt = attempts,
                    max_attempts = config.max_attempts,
                    delay = ?delay,
                    error = ?err,
                    "Request failed, retrying..."
                );

                sleep(delay).await;

                // Exponential backoff with jitter could be added here
                delay = std::cmp::min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * config.backoff_multiplier) as u64,
                    ),
                    config.max_delay,
                );
            }
        }
    }
}

/// Execute a reqwest request builder with retry semantics.
///
/// The request builder must be cloneable for full retry behavior.
/// If cloning is not possible, this will execute a single attempt.
pub async fn send_reqwest_with_retry<E, MapError, CloneError>(
    request: reqwest::RequestBuilder,
    config: &RetryConfig,
    map_error: MapError,
    clone_error: CloneError,
) -> Result<reqwest::Response, E>
where
    E: std::fmt::Debug,
    MapError: Fn(reqwest::Error) -> E + Clone,
    CloneError: Fn() -> E + Clone,
{
    if let Some(template) = request.try_clone() {
        return with_retry(config, move || {
            let request = template.try_clone().ok_or_else(|| clone_error.clone()());
            let map_error = map_error.clone();
            async move {
                let request = request?;
                request.send().await.map_err(map_error)
            }
        })
        .await;
    }

    request.send().await.map_err(map_error)
}

/// Combined retry + circuit breaker wrapper
pub async fn with_resilience<T, E, F, Fut>(
    circuit: &Arc<CircuitBreaker>,
    retry_config: &RetryConfig,
    f: F,
) -> Result<T, CircuitError<E>>
where
    F: Fn() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send,
    E: std::fmt::Debug + Send + 'static,
    T: Send + 'static,
{
    // First check if circuit allows the request
    if let Err(CircuitError::Open) = circuit.check_state().await {
        return Err(CircuitError::Open);
    }

    // Try the request with retries
    match with_retry(retry_config, f).await {
        Ok(result) => {
            circuit.on_success().await;
            Ok(result)
        }
        Err(err) => {
            circuit.on_failure().await;
            Err(CircuitError::Inner(err))
        }
    }
}

/// Global circuit breakers for external services
pub struct ServiceCircuitBreakers {
    pub oidc: Arc<CircuitBreaker>,
    pub s3: Arc<CircuitBreaker>,
    pub email: Arc<CircuitBreaker>,
    pub turnstile: Arc<CircuitBreaker>,
}

impl ServiceCircuitBreakers {
    /// Create all circuit breakers with default config
    pub fn new() -> Self {
        Self {
            oidc: CircuitBreaker::default_config("oidc"),
            s3: CircuitBreaker::default_config("s3"),
            email: CircuitBreaker::default_config("email"),
            turnstile: CircuitBreaker::default_config("turnstile"),
        }
    }

    /// Create with custom configs
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            oidc: CircuitBreaker::new("oidc", config.clone()),
            s3: CircuitBreaker::new("s3", config.clone()),
            email: CircuitBreaker::new("email", config.clone()),
            turnstile: CircuitBreaker::new("turnstile", config),
        }
    }
}

impl Default for ServiceCircuitBreakers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 3,
                recovery_timeout: Duration::from_secs(1),
                success_threshold: 1,
            },
        );

        // Initial state is closed
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Fail 3 times
        for _ in 0..3 {
            let _ = cb.execute(|| async { Err::<(), ()>(()) }).await;
        }

        // Circuit should be open
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_retry_succeeds_eventually() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            retry_if: RetryCondition::Always,
        };

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_retry = attempts.clone();
        let result = with_retry(&config, move || {
            let attempts = attempts_for_retry.clone();
            async move {
                let current = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 3 {
                    Err::<i32, ()>(())
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
