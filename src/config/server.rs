//! Server configuration module.
//!
//! This module defines configuration related to the MCP server itself,
//! including transport options, worker threads, and basic server settings.

use super::ConfigResult;
use super::Validate;
use crate::error::config::ConfigError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Transport type for the MCP server.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    /// WebSocket transport
    WebSocket,
    /// Standard I/O transport
    Stdio,
    /// Support both transports
    Both,
}

impl Default for TransportType {
    fn default() -> Self {
        Self::Both
    }
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Name of the server (used in logs and metrics)
    pub name: String,

    /// Transport to use for communication
    pub transport: TransportType,

    /// Address to bind to for WebSocket transport
    pub address: SocketAddr,

    /// Number of worker threads for request processing
    pub worker_threads: usize,

    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,

    /// Default request timeout in milliseconds
    pub default_timeout_ms: u64,

    /// Path to state directory for persistent storage
    pub state_dir: PathBuf,

    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "mauka-mcp".to_string(),
            transport: TransportType::default(),
            address: "127.0.0.1:8765".parse().unwrap(),
            worker_threads: num_cpus::get(),
            max_concurrent_requests: 1000,
            default_timeout_ms: 30000,
            state_dir: PathBuf::from("/var/lib/mauka-mcp"),
            max_message_size: 10 * 1024 * 1024, // 10 MiB
        }
    }
}

impl Validate for ServerConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate server name
        if self.name.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "Server name cannot be empty".to_string(),
            ));
        }

        // Validate worker threads
        if self.worker_threads == 0 {
            return Err(ConfigError::ValidationError(
                "worker_threads must be greater than 0".to_string(),
            ));
        }

        // Validate max_concurrent_requests
        if self.max_concurrent_requests == 0 {
            return Err(ConfigError::ValidationError(
                "max_concurrent_requests must be greater than 0".to_string(),
            ));
        }

        // Validate default_timeout_ms
        if self.default_timeout_ms == 0 {
            return Err(ConfigError::ValidationError(
                "default_timeout_ms must be greater than 0".to_string(),
            ));
        }

        // Validate max_message_size
        if self.max_message_size == 0 {
            return Err(ConfigError::ValidationError(
                "max_message_size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}
