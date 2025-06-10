//! Configuration module for the Mauka MCP Server.
//!
//! This module provides a comprehensive configuration system that can load settings
//! from files (TOML, YAML, JSON) and override them with environment variables.
//! All configuration values are validated for correctness before use.

use std::path::{Path, PathBuf};
use std::sync::Arc;
// Removed unused import
use crate::error::config::ConfigError;
use config::{Config, ConfigError as ExternalConfigError, Environment, File};
use serde::{Deserialize, Serialize};

pub mod cache;
pub mod http;
pub mod limits;
pub mod security;
pub mod server;

/// Result type for configuration operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// A trait for types that can be validated.
pub trait Validate {
    /// Validates that the configuration is correct.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the configuration is valid
    /// * `Err(ConfigError)` if the configuration is invalid
    fn validate(&self) -> ConfigResult<()>;
}

/// Main configuration for the Mauka MCP Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct MaukaConfig {
    /// Server configuration
    pub server: server::ServerConfig,

    /// HTTP client configuration
    pub http: http::HttpConfig,

    /// Cache configuration
    pub cache: cache::CacheConfig,

    /// Security configuration
    pub security: security::SecurityConfig,

    /// Resource limits configuration
    pub limits: limits::LimitsConfig,

    /// Log configuration
    pub log: LogConfig,
}


impl Validate for MaukaConfig {
    fn validate(&self) -> ConfigResult<()> {
        self.server.validate()?;
        self.http.validate()?;
        self.cache.validate()?;
        self.security.validate()?;
        self.limits.validate()?;
        self.log.validate()?;
        Ok(())
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Whether to log in JSON format
    pub json: bool,

    /// Whether to include source code locations in logs
    pub source_location: bool,

    /// Log file path (None for stdout)
    pub file: Option<PathBuf>,

    /// Maximum log file size in megabytes before rotation
    pub max_size_mb: u64,

    /// Maximum number of rotated log files to keep
    pub max_files: u8,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
            source_location: true,
            file: None,
            max_size_mb: 100,
            max_files: 5,
        }
    }
}

impl Validate for LogConfig {
    fn validate(&self) -> ConfigResult<()> {
        // Validate log level
        match self.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid log level: {}",
                    self.level
                )))
            }
        }

        // Validate max_size_mb
        if self.max_size_mb == 0 {
            return Err(ConfigError::ValidationError(
                "max_size_mb must be greater than 0".to_string(),
            ));
        }

        // Validate max_files
        if self.max_files == 0 {
            return Err(ConfigError::ValidationError(
                "max_files must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Configuration loader for the Mauka MCP Server.
#[derive(Debug)]
pub struct ConfigLoader {
    config_path: Option<PathBuf>,
    env_prefix: String,
}

impl ConfigLoader {
    /// Creates a new configuration loader.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional path to the configuration file
    /// * `env_prefix` - Prefix for environment variables that override configuration values
    pub fn new<P: AsRef<Path>>(config_path: Option<P>, env_prefix: &str) -> Self {
        Self {
            config_path: config_path.map(|p| p.as_ref().to_path_buf()),
            env_prefix: env_prefix.to_string(),
        }
    }

    /// Loads the configuration from a file and environment variables.
    ///
    /// # Returns
    ///
    /// * `Ok(MaukaConfig)` if the configuration was loaded successfully
    /// * `Err(ConfigError)` if there was an error loading the configuration
    pub fn load(&self) -> ConfigResult<MaukaConfig> {
        let mut builder = Config::builder();

        // Add default configuration values
        builder = builder.add_source(
            Config::try_from(&MaukaConfig::default())
                .map_err(|e| ConfigError::ParseError(e.to_string()))?,
        );

        // Add configuration from file if provided
        if let Some(path) = &self.config_path {
            if !path.exists() {
                return Err(ConfigError::FileNotFound(path.clone()));
            }

            builder = match path.extension().and_then(|ext| ext.to_str()) {
                Some("toml") => builder.add_source(File::with_name(path.to_str().unwrap())),
                Some("json") => builder.add_source(
                    File::with_name(path.to_str().unwrap()).format(config::FileFormat::Json),
                ),
                Some("yaml" | "yml") => builder.add_source(
                    File::with_name(path.to_str().unwrap()).format(config::FileFormat::Yaml),
                ),
                _ => {
                    return Err(ConfigError::ParseError(format!(
                        "Unsupported file extension for: {path:?}"
                    )))
                }
            };
        }

        // Add environment variables with prefix
        builder = builder.add_source(
            Environment::with_prefix(&self.env_prefix)
                .separator("__")
                .try_parsing(true),
        );

        // Build the configuration
        let config = builder.build().map_err(|e| match e {
            ExternalConfigError::NotFound(path) => ConfigError::FileNotFound(PathBuf::from(path)),
            ExternalConfigError::PathParse(path) => {
                ConfigError::ParseError(format!("Invalid path: {path:?}"))
            }
            ExternalConfigError::FileParse { .. } => {
                ConfigError::ParseError("Error parsing config file".to_string())
            }
            ExternalConfigError::Foreign(err) => ConfigError::ParseError(err.to_string()),
            ExternalConfigError::Frozen => {
                ConfigError::ParseError("Configuration is frozen".to_string())
            }
            ExternalConfigError::Message(msg) => ConfigError::ParseError(msg),
            ExternalConfigError::Type { .. } => {
                ConfigError::ParseError("Type conversion error".to_string())
            }
        })?;

        // Deserialize the configuration
        let mauka_config: MaukaConfig = config
            .try_deserialize()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Validate the configuration
        mauka_config.validate()?;

        Ok(mauka_config)
    }
}

/// Global configuration accessor.
#[derive(Debug, Clone)]
pub struct GlobalConfig {
    config: Arc<MaukaConfig>,
}

impl GlobalConfig {
    /// Creates a new global configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to use
    pub fn new(config: MaukaConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Returns a reference to the configuration.
    pub fn get(&self) -> &MaukaConfig {
        &self.config
    }
}

/// Global configuration instance.
static mut GLOBAL_CONFIG: Option<GlobalConfig> = None;

/// Initializes the global configuration.
///
/// # Arguments
///
/// * `config` - The configuration to use
///
/// # Safety
///
/// This function is not thread-safe and should only be called during application initialization.
pub fn init_global_config(config: MaukaConfig) {
    unsafe {
        GLOBAL_CONFIG = Some(GlobalConfig::new(config));
    }
}

/// Returns a reference to the global configuration.
///
/// # Panics
///
/// Panics if the global configuration has not been initialized.
pub fn get_global_config() -> &'static GlobalConfig {
    unsafe {
        GLOBAL_CONFIG
            .as_ref()
            .expect("Global configuration not initialized")
    }
}
