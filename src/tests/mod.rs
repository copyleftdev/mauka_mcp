//! Test modules for the Mauka MCP Server.
//!
//! This module contains all testing infrastructure, including:
//! - Unit tests for each component
//! - Integration tests for cross-component functionality
//! - Property-based tests using proptest
//! - Test fixtures and utilities
//! - Performance regression tests
//!
//! The test philosophy follows the project standards:
//! - Test-driven development for all features
//! - Minimum 90% code coverage for all components
//! - Testing all error paths and edge cases
//! - Property-based testing for input validation
//! - Fuzzing for parsing components

pub mod config_tests;
pub mod error_tests;
pub mod kahuna_queue_tests;
pub mod test_utils;

// Re-export commonly used testing tools to simplify imports in test modules
pub use test_utils::{
    create_test_dir, duration_strategy, jsonrpc_method_strategy, string_strategy, MockProvider,
    TestFixture,
};

/// Run a suite of property-based tests using the given configuration, strategy, and test function.
#[macro_export]
macro_rules! run_proptest {
    ($strategy:expr, $test_fn:expr) => {
        proptest::proptest! {
            #![proptest_config(proptest::test_runner::Config::with_cases(100))]
            |value in $strategy| {
                $test_fn(value)?;
            }
        }
    };
}

/// Marker trait for integration tests that may affect global state.
///
/// Tests implementing this trait should be run serially to prevent
/// interference between tests.
pub trait GlobalStateTest {
    /// Set up any required global state before the test.
    fn setup(&self);

    /// Clean up global state after the test.
    fn teardown(&self);
}
