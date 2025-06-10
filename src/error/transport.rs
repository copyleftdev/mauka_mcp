//! Transport error module.
//!
//! This module defines error types that may occur in WebSocket and Stdio
//! transport implementations.

use std::io;
use thiserror::Error;

/// Errors that can occur during transport operations.
#[derive(Error, Debug)]
pub enum TransportError {
    /// Error when connecting to a WebSocket.
    #[error("WebSocket connection error: {0}")]
    WebSocketConnectionError(String),

    /// Error when sending a message over WebSocket.
    #[error("WebSocket send error: {0}")]
    WebSocketSendError(String),

    /// Error when receiving a message over WebSocket.
    #[error("WebSocket receive error: {0}")]
    WebSocketReceiveError(String),

    /// Error when the WebSocket connection is closed unexpectedly.
    #[error("WebSocket connection closed unexpectedly: {0}")]
    WebSocketConnectionClosed(String),

    /// Error when reading from standard input.
    #[error("Standard input read error: {0}")]
    StdioReadError(#[from] io::Error),

    /// Error when writing to standard output.
    #[error("Standard output write error: {0}")]
    StdioWriteError(String),

    /// Error when the transport is not initialized.
    #[error("Transport not initialized")]
    NotInitialized,

    /// Error when the transport is already initialized.
    #[error("Transport already initialized")]
    AlreadyInitialized,

    /// Error when the transport is closed.
    #[error("Transport closed")]
    Closed,

    /// Error when a timeout occurs during transport operations.
    #[error("Transport timeout after {0} milliseconds")]
    Timeout(u64),

    /// Other transport errors.
    #[error("Transport error: {0}")]
    Other(String),
}
