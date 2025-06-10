//! HTTP client configuration module.
//!
//! This module defines configuration for the HTTP client core, including
//! connection pooling, rate limiting, and circuit breaker settings.

// Duration is used in config values but imported via Serde
use super::{ConfigResult, Validate};
use crate::error::config::ConfigError;
use serde::{Deserialize, Serialize};

/// HTTP client configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpConfig {
    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,

    /// Rate limiter configuration
    pub rate_limiter: RateLimiterConfig,

    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,

    /// General HTTP client settings
    pub client: HttpClientConfig,
}

impl Validate for HttpConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.connection_pool.validate()?;
        self.rate_limiter.validate()?;
        self.circuit_breaker.validate()?;
        self.client.validate()?;
        Ok(())
    }
}

/// Connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections per host
    pub max_connections_per_host: usize,

    /// Keep-alive timeout in seconds
    pub keep_alive_sec: u64,

    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,

    /// Maximum number of idle connections
    pub max_idle_connections: usize,

    /// Maximum idle time for connections in seconds
    pub max_idle_time_sec: u64,

    /// Connection health check interval in seconds
    pub health_check_interval_sec: u64,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 32,
            keep_alive_sec: 60,
            connect_timeout_ms: 5000,
            max_idle_connections: 100,
            max_idle_time_sec: 90,
            health_check_interval_sec: 30,
        }
    }
}

impl Validate for ConnectionPoolConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_connections_per_host
        if self.max_connections_per_host == 0 {
            return Err(ConfigError::ValidationError(
                "max_connections_per_host must be greater than 0".to_string(),
            ));
        }

        // Validate keep_alive_sec
        if self.keep_alive_sec == 0 {
            return Err(ConfigError::ValidationError(
                "keep_alive_sec must be greater than 0".to_string(),
            ));
        }

        // Validate connect_timeout_ms
        if self.connect_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "connect_timeout_ms must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Rate limiter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    /// Whether the rate limiter is enabled
    pub enabled: bool,

    /// Initial rate limit in requests per second
    pub initial_rate: f64,

    /// Maximum rate limit in requests per second
    pub max_rate: f64,

    /// Minimum rate limit in requests per second
    pub min_rate: f64,

    /// Increase multiplier for the MIMD algorithm
    pub increase_factor: f64,

    /// Decrease multiplier for the MIMD algorithm
    pub decrease_factor: f64,

    /// Rate update interval in milliseconds
    pub update_interval_ms: u64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_rate: 50.0,
            max_rate: 500.0,
            min_rate: 1.0,
            increase_factor: 1.1,
            decrease_factor: 0.5,
            update_interval_ms: 1000,
        }
    }
}

impl Validate for RateLimiterConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate initial_rate
        if self.initial_rate <= 0.0 {
            return Err(ConfigError::ValidationError(
                "initial_rate must be greater than 0".to_string(),
            ));
        }

        // Validate max_rate
        if self.max_rate < self.initial_rate {
            return Err(ConfigError::ValidationError(format!(
                "max_rate ({}) must be >= initial_rate ({})",
                self.max_rate, self.initial_rate
            )));
        }

        // Validate min_rate
        if self.min_rate <= 0.0 {
            return Err(ConfigError::ValidationError(
                "min_rate must be greater than 0".to_string(),
            ));
        }

        if self.min_rate > self.initial_rate {
            return Err(ConfigError::ValidationError(format!(
                "min_rate ({}) must be <= initial_rate ({})",
                self.min_rate, self.initial_rate
            )));
        }

        // Validate increase_factor
        if self.increase_factor <= 1.0 {
            return Err(ConfigError::ValidationError(
                "increase_factor must be greater than 1.0".to_string(),
            ));
        }

        // Validate decrease_factor
        if self.decrease_factor >= 1.0 || self.decrease_factor <= 0.0 {
            return Err(ConfigError::ValidationError(
                "decrease_factor must be between 0.0 and 1.0 exclusive".to_string(),
            ));
        }

        Ok(())
    }
}

/// Circuit breaker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Whether the circuit breaker is enabled
    pub enabled: bool,

    /// Window size for error rate calculation
    pub window_size: usize,

    /// Error threshold ratio to trip the circuit breaker
    pub error_threshold_ratio: f64,

    /// Minimum number of requests before tripping
    pub minimum_request_threshold: usize,

    /// Reset timeout in milliseconds
    pub reset_timeout_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            window_size: 100,
            error_threshold_ratio: 0.5,
            minimum_request_threshold: 20,
            reset_timeout_ms: 30000,
        }
    }
}

impl Validate for CircuitBreakerConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate window_size
        if self.window_size == 0 {
            return Err(ConfigError::ValidationError(
                "window_size must be greater than 0".to_string(),
            ));
        }

        // Validate error_threshold_ratio
        if self.error_threshold_ratio <= 0.0 || self.error_threshold_ratio > 1.0 {
            return Err(ConfigError::ValidationError(
                "error_threshold_ratio must be between 0.0 (exclusive) and 1.0 (inclusive)"
                    .to_string(),
            ));
        }

        // Validate reset_timeout_ms
        if self.reset_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "reset_timeout_ms must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// General HTTP client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    /// User agent string
    pub user_agent: String,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,

    /// Whether to follow redirects
    pub follow_redirects: bool,

    /// Maximum number of redirects to follow
    pub max_redirects: usize,

    /// Whether to enable HTTP/2
    pub http2_enabled: bool,

    /// Maximum idle HTTP/2 streams per connection
    pub http2_max_idle_streams: u32,

    /// Maximum concurrent HTTP/2 streams per connection
    pub http2_max_concurrent_streams: u32,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            user_agent: "Mauka-MCP/1.0".to_string(),
            request_timeout_ms: 30000,
            follow_redirects: true,
            max_redirects: 10,
            http2_enabled: true,
            http2_max_idle_streams: 100,
            http2_max_concurrent_streams: 250,
        }
    }
}

impl Validate for HttpClientConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate user_agent
        if self.user_agent.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "user_agent cannot be empty".to_string(),
            ));
        }

        // Validate request_timeout_ms
        if self.request_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "request_timeout_ms must be greater than 0".to_string(),
            ));
        }

        // Validate max_redirects
        if self.follow_redirects && self.max_redirects == 0 {
            return Err(ConfigError::ValidationError(
                "max_redirects must be greater than 0 when follow_redirects is enabled".to_string(),
            ));
        }

        Ok(())
    }
}
