//! Error module for the Mauka MCP Server.
//!
//! This module provides a comprehensive error handling framework for the entire application,
//! following Rust's idiomatic error handling patterns with explicit error types,
//! proper error propagation, and helpful context information.

use std::fmt::{Display, Formatter};
use std::sync::Arc;
use thiserror::Error;

pub mod config;
pub mod http;
pub mod protocol;
pub mod transport;

/// Result type alias used throughout the Mauka MCP Server.
pub type MaukaResult<T> = Result<T, MaukaError>;

/// Core error enum for the Mauka MCP Server.
#[derive(Error, Debug)]
pub enum MaukaError {
    /// Errors occurring during configuration loading or validation.
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// Errors related to the MCP protocol handling.
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::ProtocolError),

    /// Errors related to transport mechanisms (WebSocket, Stdio).
    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    /// Errors related to HTTP client operations.
    #[error("HTTP client error: {0}")]
    Http(#[from] http::HttpError),

    /// IO errors that may occur during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/Deserialization errors.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Custom error with message for cases where specific error types are not defined.
    #[error("{0}")]
    Custom(String),
}

/// Error reporting structure to provide context and debugging information.
#[derive(Debug)]
pub struct ErrorContext {
    /// The original error that occurred.
    pub error: MaukaError,

    /// The component where the error occurred.
    pub component: String,

    /// Additional context information to help with debugging.
    pub details: Option<String>,

    /// Stack trace information if available.
    pub trace: Option<String>,
}

impl ErrorContext {
    /// Creates a new error context with the given error and component.
    ///
    /// # Arguments
    ///
    /// * `error` - The error that occurred
    /// * `component` - The component where the error occurred
    pub fn new<S: Into<String>>(error: MaukaError, component: S) -> Self {
        Self {
            error,
            component: component.into(),
            details: None,
            trace: None,
        }
    }

    /// Adds detail information to the error context.
    ///
    /// # Arguments
    ///
    /// * `details` - Additional context information to help with debugging
    pub fn with_details<S: Into<String>>(mut self, details: S) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Adds stack trace information to the error context.
    ///
    /// # Arguments
    ///
    /// * `trace` - Stack trace as a string
    pub fn with_trace<S: Into<String>>(mut self, trace: S) -> Self {
        self.trace = Some(trace.into());
        self
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in {}: {}", self.component, self.error)?;
        if let Some(details) = &self.details {
            write!(f, "\nDetails: {details}")?;
        }
        Ok(())
    }
}

/// Error reporter trait for reporting errors to various sinks.
pub trait ErrorReporter: Send + Sync + std::fmt::Debug {
    /// Report an error with context.
    ///
    /// # Arguments
    ///
    /// * `context` - The error context to report
    fn report(&self, context: ErrorContext);
}

/// A simple error reporter implementation that logs errors using the tracing framework.
#[derive(Default, Debug)]
pub struct TracingErrorReporter;

impl ErrorReporter for TracingErrorReporter {
    fn report(&self, context: ErrorContext) {
        tracing::error!(
            error = %context.error,
            component = %context.component,
            details = context.details.as_deref().unwrap_or("None"),
            trace = context.trace.as_deref().unwrap_or("None"),
            "Error reported"
        );
    }
}

/// Global error reporter accessor.
#[derive(Debug, Default)]
pub struct ErrorReporting {
    reporter: Option<Arc<dyn ErrorReporter>>,
}

impl ErrorReporting {
    /// Set the global error reporter.
    ///
    /// # Arguments
    ///
    /// * `reporter` - The error reporter to use
    pub fn set_reporter(&mut self, reporter: Arc<dyn ErrorReporter>) {
        self.reporter = Some(reporter);
    }

    /// Report an error with context.
    ///
    /// # Arguments
    ///
    /// * `context` - The error context to report
    pub fn report(&self, context: ErrorContext) {
        if let Some(reporter) = &self.reporter {
            reporter.report(context);
        } else {
            // Fallback to standard error output if no reporter is configured
            eprintln!("Error: {context}");
        }
    }
}

/// Error reporting singleton instance.
static mut ERROR_REPORTING: ErrorReporting = ErrorReporting { reporter: None };

/// Get the global error reporting instance.
pub fn get_error_reporting() -> &'static ErrorReporting {
    unsafe { &ERROR_REPORTING }
}

/// Set the global error reporter.
///
/// # Arguments
///
/// * `reporter` - The error reporter to use
pub fn set_error_reporter(reporter: Arc<dyn ErrorReporter>) {
    unsafe {
        ERROR_REPORTING.set_reporter(reporter);
    }
}
