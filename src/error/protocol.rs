//! Protocol error module.
//!
//! This module defines error types that may occur during MCP protocol operations,
//! including JSON-RPC handling and validation.

use thiserror::Error;

/// Errors that can occur during protocol operations.
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// Error when the JSON-RPC message is invalid.
    #[error("Invalid JSON-RPC message: {0}")]
    InvalidMessage(String),

    /// Error when the JSON-RPC request has an invalid method.
    #[error("Invalid method: {0}")]
    InvalidMethod(String),

    /// Error when the JSON-RPC request has invalid parameters.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Error when a message with a duplicate ID is received.
    #[error("Duplicate message ID: {0}")]
    DuplicateId(String),

    /// Error when the protocol version is unsupported.
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// Error when a required field is missing from the message.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Error when the message exceeds the maximum allowed size.
    #[error("Message size exceeds maximum allowed: {size} > {max_size}")]
    MessageTooLarge {
        /// The actual size of the message in bytes
        size: usize,
        /// The maximum allowed size in bytes
        max_size: usize,
    },

    /// Error when the response cannot be correlated with a request.
    #[error("Cannot correlate response to request: {0}")]
    CorrelationError(String),

    /// Error during protocol initialization.
    #[error("Initialization error: {0}")]
    InitializationError(String),

    /// Error in the tool discovery process.
    #[error("Tool discovery error: {0}")]
    ToolDiscoveryError(String),

    /// Other protocol errors.
    #[error("Protocol error: {0}")]
    Other(String),
}
