//! Mauka MCP Server Library
//!
//! This library contains the core components of the Mauka MCP Server,
//! including the protocol handlers, transport layers, and utilities.
//! The library is designed to be used by the binary crate, but can also
//! be used as a dependency by other projects.
//!
//! # Architecture
//!
//! The Mauka MCP Server is designed with the following principles in mind:
//! - Strict component boundaries
//! - Dependency injection for testability
//! - Async-first approach for scalability
//! - Comprehensive error handling and propagation
//! - Zero-copy optimizations where possible
//! - Lock-free concurrency for high throughput

// Re-export public modules
pub mod config;
pub mod data_structures;
pub mod error;
pub mod protocol;
pub mod utils;

// Internal modules that are not part of the public API
#[cfg(test)]
pub(crate) mod tests;

// Feature-gated modules
#[cfg(feature = "benchmarking")]
pub mod bench;

/// Version information for the Mauka MCP Server.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library initialization function
pub fn init() -> error::MaukaResult<()> {
    // Set up global error reporter with tracing
    let reporter = error::TracingErrorReporter::new();
    error::set_error_reporter(std::sync::Arc::new(std::sync::Mutex::new(reporter)));

    // Initialize default configuration
    config::init_default_config()?;

    Ok(())
}
