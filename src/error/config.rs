//! Configuration error module.
//!
//! This module defines error types that may occur during configuration loading,
//! parsing, and validation operations.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration operations.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Error when the configuration file is missing.
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    /// Error when the configuration file cannot be read.
    #[error("Failed to read configuration file: {0}")]
    FileReadError(String),

    /// Error when parsing the configuration file.
    #[error("Failed to parse configuration file: {0}")]
    ParseError(String),

    /// Error when validating the configuration.
    #[error("Configuration validation error: {0}")]
    ValidationError(String),

    /// Error when a required configuration value is missing.
    #[error("Missing required configuration value: {0}")]
    MissingValue(String),

    /// Error when a configuration value has an invalid type.
    #[error("Invalid configuration value type for {key}: expected {expected}, got {actual}")]
    InvalidValueType {
        /// The key of the invalid value
        key: String,
        /// The expected type
        expected: String,
        /// The actual type
        actual: String,
    },

    /// Error when a configuration value is out of the valid range.
    #[error("Configuration value {key} is out of valid range: {message}")]
    ValueOutOfRange {
        /// The key of the invalid value
        key: String,
        /// Description of the valid range
        message: String,
    },

    /// Other configuration errors.
    #[error("Configuration error: {0}")]
    Other(String),
}
