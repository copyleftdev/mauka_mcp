//! Resource limits configuration module.
//!
//! This module defines limits for various resources used by the MCP server,
//! including memory, CPU, connections, and request rates.

use super::{ConfigResult, Validate};
use crate::error::config::ConfigError;
use serde::{Deserialize, Serialize};

/// Resource limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LimitsConfig {
    /// Memory limits
    pub memory: MemoryLimits,

    /// CPU limits
    pub cpu: CpuLimits,

    /// Connection limits
    pub connection: ConnectionLimits,

    /// Request rate limits
    pub request_rate: RequestRateLimits,
}


impl Validate for LimitsConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.memory.validate()?;
        self.cpu.validate()?;
        self.connection.validate()?;
        self.request_rate.validate()?;
        Ok(())
    }
}

/// Memory limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum heap size in bytes
    pub max_heap_size_bytes: Option<usize>,

    /// Maximum RSS (Resident Set Size) in bytes
    pub max_rss_bytes: Option<usize>,

    /// Memory warning threshold as a ratio of max (0.0 to 1.0)
    pub warning_threshold: f64,

    /// Whether to enable jemalloc memory profiling
    pub enable_profiling: bool,

    /// Whether to use jemalloc as the memory allocator
    pub use_jemalloc: bool,

    /// Background memory purge interval in milliseconds
    pub purge_interval_ms: u64,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_heap_size_bytes: Some(2_000_000_000), // 2 GB
            max_rss_bytes: Some(3_000_000_000),       // 3 GB
            warning_threshold: 0.8,
            enable_profiling: true,
            use_jemalloc: true,
            purge_interval_ms: 60000, // 1 minute
        }
    }
}

impl Validate for MemoryLimits {
    fn validate(&self) -> ConfigResult<()> {
        // Validate warning_threshold
        if self.warning_threshold <= 0.0 || self.warning_threshold >= 1.0 {
            return Err(ConfigError::ValidationError(
                "warning_threshold must be between 0.0 and 1.0 exclusive".to_string(),
            ));
        }

        // Validate purge_interval_ms
        if self.purge_interval_ms == 0 {
            return Err(ConfigError::ValidationError(
                "purge_interval_ms must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// CPU limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    /// Maximum number of worker threads
    pub max_worker_threads: usize,

    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: f64,

    /// Whether to enable CPU affinity for threads
    pub enable_affinity: bool,

    /// Whether to enable thread pinning to specific cores
    pub enable_thread_pinning: bool,
}

impl Default for CpuLimits {
    fn default() -> Self {
        Self {
            max_worker_threads: num_cpus::get(),
            max_cpu_percent: 90.0,
            enable_affinity: true,
            enable_thread_pinning: false,
        }
    }
}

impl Validate for CpuLimits {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_worker_threads
        if self.max_worker_threads == 0 {
            return Err(ConfigError::ValidationError(
                "max_worker_threads must be greater than 0".to_string(),
            ));
        }

        // Validate max_cpu_percent
        if self.max_cpu_percent <= 0.0 || self.max_cpu_percent > 100.0 {
            return Err(ConfigError::ValidationError(
                "max_cpu_percent must be between 0.0 (exclusive) and 100.0 (inclusive)".to_string(),
            ));
        }

        Ok(())
    }
}

/// Connection limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLimits {
    /// Maximum number of concurrent connections
    pub max_concurrent_connections: usize,

    /// Maximum number of connections per IP address
    pub max_connections_per_ip: usize,

    /// Connection idle timeout in milliseconds
    pub idle_timeout_ms: u64,

    /// Maximum backlog size for pending connections
    pub max_backlog: usize,

    /// Maximum number of file descriptors
    pub max_file_descriptors: Option<u64>,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_concurrent_connections: 100_000,
            max_connections_per_ip: 1000,
            idle_timeout_ms: 300_000, // 5 minutes
            max_backlog: 1024,
            max_file_descriptors: Some(100_000),
        }
    }
}

impl Validate for ConnectionLimits {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_concurrent_connections
        if self.max_concurrent_connections == 0 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_connections must be greater than 0".to_string(),
            ));
        }

        // Validate max_connections_per_ip
        if self.max_connections_per_ip == 0 {
            return Err(ConfigError::ValidationError(
                "max_connections_per_ip must be greater than 0".to_string(),
            ));
        }

        // Validate idle_timeout_ms
        if self.idle_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "idle_timeout_ms must be greater than 0".to_string(),
            ));
        }

        // Validate max_backlog
        if self.max_backlog == 0 {
            return Err(ConfigError::ValidationError(
                "max_backlog must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Request rate limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRateLimits {
    /// Maximum requests per second globally
    pub max_rps: f64,

    /// Maximum requests per second per IP address
    pub max_rps_per_ip: f64,

    /// Burst factor for rate limiting (multiple of base rate)
    pub burst_factor: f64,

    /// Time window for rate calculation in milliseconds
    pub window_ms: u64,

    /// Whether to enable adaptive rate limiting
    pub enable_adaptive: bool,
}

impl Default for RequestRateLimits {
    fn default() -> Self {
        Self {
            max_rps: 50_000.0,
            max_rps_per_ip: 1000.0,
            burst_factor: 2.0,
            window_ms: 1000,
            enable_adaptive: true,
        }
    }
}

impl Validate for RequestRateLimits {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_rps
        if self.max_rps <= 0.0 {
            return Err(ConfigError::ValidationError(
                "max_rps must be greater than 0".to_string(),
            ));
        }

        // Validate max_rps_per_ip
        if self.max_rps_per_ip <= 0.0 {
            return Err(ConfigError::ValidationError(
                "max_rps_per_ip must be greater than 0".to_string(),
            ));
        }

        // Validate burst_factor
        if self.burst_factor <= 1.0 {
            return Err(ConfigError::ValidationError(
                "burst_factor must be greater than 1.0".to_string(),
            ));
        }

        // Validate window_ms
        if self.window_ms == 0 {
            return Err(ConfigError::ValidationError(
                "window_ms must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}
