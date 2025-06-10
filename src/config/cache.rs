//! Cache configuration module.
//!
//! This module defines configuration for the caching system, including memory cache,
//! persistent storage, and cache policies.

use super::{ConfigResult, Validate};
use crate::error::config::ConfigError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,

    /// Memory cache configuration
    pub memory: MemoryCacheConfig,

    /// Persistent storage configuration
    pub persistent: PersistentCacheConfig,

    /// Cache policy configuration
    pub policy: CachePolicyConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            persistent: PersistentCacheConfig::default(),
            policy: CachePolicyConfig::default(),
        }
    }
}

impl Validate for CacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.memory.validate()?;
        self.persistent.validate()?;
        self.policy.validate()?;
        Ok(())
    }
}

/// Memory cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    /// Whether the memory cache is enabled
    pub enabled: bool,

    /// Maximum memory cache size in bytes
    pub max_size_bytes: usize,

    /// ARC cache part sizes in ratio (p value)
    pub p_value: f64,

    /// Whether to use Bloom filter for admission control
    pub use_bloom_filter: bool,

    /// Bloom filter false positive rate
    pub bloom_false_positive_rate: f64,

    /// Bloom filter capacity
    pub bloom_capacity: usize,

    /// Whether to use deduplication
    pub use_deduplication: bool,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_bytes: 1_000_000_000, // 1 GB
            p_value: 0.5,
            use_bloom_filter: true,
            bloom_false_positive_rate: 0.01,
            bloom_capacity: 1_000_000,
            use_deduplication: true,
        }
    }
}

impl Validate for MemoryCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate max_size_bytes
        if self.max_size_bytes == 0 {
            return Err(ConfigError::ValidationError(
                "max_size_bytes must be greater than 0".to_string(),
            ));
        }

        // Validate p_value
        if self.p_value <= 0.0 || self.p_value >= 1.0 {
            return Err(ConfigError::ValidationError(
                "p_value must be between 0.0 and 1.0 exclusive".to_string(),
            ));
        }

        // Validate bloom_false_positive_rate
        if self.use_bloom_filter
            && (self.bloom_false_positive_rate <= 0.0 || self.bloom_false_positive_rate >= 1.0)
        {
            return Err(ConfigError::ValidationError(
                "bloom_false_positive_rate must be between 0.0 and 1.0 exclusive".to_string(),
            ));
        }

        // Validate bloom_capacity
        if self.use_bloom_filter && self.bloom_capacity == 0 {
            return Err(ConfigError::ValidationError(
                "bloom_capacity must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Persistent cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentCacheConfig {
    /// Whether persistent caching is enabled
    pub enabled: bool,

    /// Path to store persistent cache data
    pub path: PathBuf,

    /// Maximum disk space to use in bytes
    pub max_size_bytes: u64,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Maximum number of open files
    pub max_open_files: i32,

    /// Whether to use compression for stored data
    pub use_compression: bool,
}

impl Default for PersistentCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("/var/lib/mauka-mcp/cache"),
            max_size_bytes: 10_000_000_000, // 10 GB
            flush_interval_ms: 5000,
            max_open_files: 1000,
            use_compression: true,
        }
    }
}

impl Validate for PersistentCacheConfig {
    fn validate(&self) -> ConfigResult<()> {
        if self.enabled {
            // Validate max_size_bytes
            if self.max_size_bytes == 0 {
                return Err(ConfigError::ValidationError(
                    "max_size_bytes must be greater than 0".to_string(),
                ));
            }

            // Validate flush_interval_ms
            if self.flush_interval_ms == 0 {
                return Err(ConfigError::ValidationError(
                    "flush_interval_ms must be greater than 0".to_string(),
                ));
            }

            // Validate max_open_files
            if self.max_open_files <= 0 {
                return Err(ConfigError::ValidationError(
                    "max_open_files must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Cache policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicyConfig {
    /// Default Time-To-Live for cache entries in seconds
    pub default_ttl_sec: u64,

    /// Whether to respect Cache-Control headers
    pub respect_cache_control: bool,

    /// Whether to cache responses with errors (e.g. 4xx, 5xx)
    pub cache_errors: bool,

    /// Minimum size in bytes for content to be cached
    pub min_size_bytes: usize,

    /// Maximum size in bytes for content to be cached
    pub max_size_bytes: usize,

    /// Content types to cache (empty means all)
    pub cacheable_content_types: Vec<String>,
}

impl Default for CachePolicyConfig {
    fn default() -> Self {
        Self {
            default_ttl_sec: 3600, // 1 hour
            respect_cache_control: true,
            cache_errors: false,
            min_size_bytes: 0,
            max_size_bytes: 10_000_000, // 10 MB
            cacheable_content_types: vec![
                "text/html".to_string(),
                "text/plain".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "application/javascript".to_string(),
                "text/css".to_string(),
                "image/".to_string(),
            ],
        }
    }
}

impl Validate for CachePolicyConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate default_ttl_sec
        if self.default_ttl_sec == 0 {
            return Err(ConfigError::ValidationError(
                "default_ttl_sec must be greater than 0".to_string(),
            ));
        }

        // Validate max_size_bytes
        if self.max_size_bytes == 0 {
            return Err(ConfigError::ValidationError(
                "max_size_bytes must be greater than 0".to_string(),
            ));
        }

        // Validate min_size_bytes <= max_size_bytes
        if self.min_size_bytes > self.max_size_bytes {
            return Err(ConfigError::ValidationError(format!(
                "min_size_bytes ({}) must be less than or equal to max_size_bytes ({})",
                self.min_size_bytes, self.max_size_bytes
            )));
        }

        Ok(())
    }
}
