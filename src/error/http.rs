//! HTTP client error module.
//!
//! This module defines error types that may occur during HTTP client operations,
//! including connection pooling, request handling, and response processing.

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during HTTP client operations.
#[derive(Error, Debug)]
pub enum HttpError {
    /// Error when creating a connection in the connection pool.
    #[error("Failed to create connection: {0}")]
    ConnectionCreationError(String),

    /// Error when the connection pool is exhausted.
    #[error("Connection pool exhausted")]
    ConnectionPoolExhausted,

    /// Error when a connection is invalid or corrupt.
    #[error("Invalid connection: {0}")]
    InvalidConnection(String),

    /// Error when a request fails due to a timeout.
    #[error("Request timed out after {0:?}")]
    RequestTimeout(Duration),

    /// Error when a request is rejected by the rate limiter.
    #[error("Request rejected by rate limiter: {0}")]
    RateLimited(String),

    /// Error when a request is rejected by the circuit breaker.
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    /// Error when a request fails due to a DNS resolution failure.
    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),

    /// Error when a request fails due to a TLS error.
    #[error("TLS error: {0}")]
    TlsError(String),

    /// Error when a request results in an HTTP error status code.
    #[error("HTTP error status: {status} - {message}")]
    HttpStatus {
        /// The HTTP status code
        status: u16,
        /// The error message
        message: String,
    },

    /// Error when a request fails due to a connect timeout.
    #[error("Connect timeout after {0:?}")]
    ConnectTimeout(Duration),

    /// Error when a response cannot be decoded.
    #[error("Response decode error: {0}")]
    ResponseDecodeError(String),

    /// Error when a response fails content validation.
    #[error("Content validation error: {0}")]
    ContentValidationError(String),

    /// Error when a request is invalid.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Error when the URL is invalid or blocked.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Error when robots.txt disallows access.
    #[error("Access disallowed by robots.txt: {0}")]
    RobotsDisallowed(String),

    /// Error when a content security policy is violated.
    #[error("Content security policy violation: {0}")]
    CspViolation(String),

    /// Other HTTP client errors.
    #[error("HTTP client error: {0}")]
    Other(String),
}
