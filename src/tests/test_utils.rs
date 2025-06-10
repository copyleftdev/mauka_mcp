//! Test utilities and fixtures for the Mauka MCP Server.
//!
//! This module provides reusable test components, fixtures, and helpers
//! to facilitate property-based testing, integration testing, and performance
//! regression testing as required by project standards.

use mockall::predicate::*;
use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy};
use std::time::Duration;
use tempfile::TempDir;

/// Maximum string length for generated test data.
const MAX_STRING_LENGTH: usize = 1000;

/// Maximum vector length for generated test data.
const MAX_VECTOR_LENGTH: usize = 100;

/// Maximum duration for timeouts in milliseconds.
const MAX_TIMEOUT_MS: u64 = 30000;

/// Create a temporary directory for test files.
///
/// # Returns
///
/// A result containing the temporary directory or an error if creation fails.
pub fn create_test_dir() -> std::io::Result<TempDir> {
    tempfile::tempdir()
}

/// Generate a strategy for random string generation.
///
/// This uses proptest to generate strings with specific constraints
/// for property-based testing.
///
/// # Parameters
///
/// * `max_length` - The maximum length of the generated strings.
///
/// # Returns
///
/// A boxed strategy that generates random strings.
pub fn string_strategy(max_length: usize) -> BoxedStrategy<String> {
    let length = 0..max_length;
    proptest::collection::vec(proptest::char::any(), length)
        .prop_map(|chars| chars.into_iter().collect::<String>())
        .boxed()
}

/// Generate a strategy for random JSON-RPC method names.
///
/// # Returns
///
/// A boxed strategy that generates valid JSON-RPC method names.
pub fn jsonrpc_method_strategy() -> BoxedStrategy<String> {
    r"[a-zA-Z][a-zA-Z0-9_]+"
        .prop_map(|s| s)
        .prop_filter("Method too long", |s| s.len() < 30)
        .boxed()
}

/// Generate a strategy for random durations within specified bounds.
///
/// # Returns
///
/// A boxed strategy that generates random Duration values.
pub fn duration_strategy() -> BoxedStrategy<Duration> {
    (0..MAX_TIMEOUT_MS)
        .prop_map(|ms| Duration::from_millis(ms))
        .boxed()
}

/// Test fixture for integration tests requiring fully initialized components.
///
/// This struct helps with setting up and tearing down complex test environments
/// in a consistent way.
pub struct TestFixture {
    /// Temporary directory for test files
    pub temp_dir: TempDir,
    /// Vector of environment variables to cleanup after tests
    env_vars: Vec<String>,
}

impl TestFixture {
    /// Create a new test fixture.
    ///
    /// # Returns
    ///
    /// A result containing the new fixture or an error.
    pub fn new() -> std::io::Result<Self> {
        let temp_dir = create_test_dir()?;
        Ok(Self {
            temp_dir,
            env_vars: Vec::new(),
        })
    }

    /// Set an environment variable for this test.
    ///
    /// The variable will be cleaned up when the fixture is dropped.
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the environment variable.
    /// * `value` - The value to set.
    pub fn set_env<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        let key_str = key.into();
        std::env::set_var(&key_str, value.into());
        self.env_vars.push(key_str);
    }

    /// Create a temporary file within the fixture directory.
    ///
    /// # Parameters
    ///
    /// * `contents` - The contents to write to the file.
    /// * `extension` - The file extension to use.
    ///
    /// # Returns
    ///
    /// A result containing the path to the file or an error.
    pub fn create_file<C: AsRef<[u8]>>(
        &self,
        contents: C,
        extension: &str,
    ) -> std::io::Result<std::path::PathBuf> {
        let mut file = tempfile::Builder::new()
            .suffix(extension)
            .tempfile_in(&self.temp_dir)?;
        std::io::Write::write_all(&mut file, contents.as_ref())?;
        Ok(file.path().to_path_buf())
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Clean up any environment variables we set
        for key in &self.env_vars {
            std::env::remove_var(key);
        }
    }
}

/// A trait for test helpers that provide mock responses.
pub trait MockProvider {
    /// The type of mock response this provider generates.
    type MockOutput;

    /// Generate a mock response.
    ///
    /// # Returns
    ///
    /// A mock response of the specified type.
    fn generate_mock(&self) -> Self::MockOutput;
}

/// Generate test cases for property-based tests.
///
/// # Parameters
///
/// * `proptest_config` - Configuration for the property-based test.
/// * `strategy` - The strategy to use for generating test cases.
/// * `test_fn` - The function to run for each test case.
///
/// # Returns
///
/// A test result.
#[macro_export]
macro_rules! proptest_suite {
    ($proptest_config:expr, $strategy:expr, $test_fn:expr) => {
        proptest!($proptest_config, |value in $strategy| {
            $test_fn(value)?;
            Ok(())
        });
    };
}
