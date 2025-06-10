//! Tests for the error module.
//!
//! This module contains tests for error handling and error types.

use crate::error::{
    get_error_reporting, set_error_reporter, ErrorContext, ErrorReporter, MaukaError,
    TracingErrorReporter,
};
use std::sync::Arc;

/// Test that error context can be created and displayed properly.
#[test]
fn test_error_context_display() {
    let error = MaukaError::Custom("test error".to_string());
    let context = ErrorContext::new(error, "test_component").with_details("additional details");

    let display_string = format!("{context}");
    assert!(display_string.contains("test error"));
    assert!(display_string.contains("test_component"));
    assert!(display_string.contains("additional details"));
}

/// Test that nested errors work correctly.
#[test]
fn test_nested_errors() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mauka_error = MaukaError::Io(io_error);

    let error_string = format!("{mauka_error}");
    assert!(error_string.contains("file not found"));
}

/// Mock error reporter for testing.
#[derive(Debug)]
struct MockErrorReporter {
    reported_count: std::sync::atomic::AtomicUsize,
}

impl MockErrorReporter {
    fn new() -> Self {
        Self {
            reported_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn reported_count(&self) -> usize {
        self.reported_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ErrorReporter for MockErrorReporter {
    fn report(&self, _context: ErrorContext) {
        self.reported_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Test that the global error reporter works correctly.
///
/// Note: This test should be run in isolation because it modifies global state.
#[test]
fn test_global_error_reporter() {
    let reporter = Arc::new(MockErrorReporter::new());
    set_error_reporter(reporter.clone());

    let error = MaukaError::Custom("test error".to_string());
    let context = ErrorContext::new(error, "test_component");

    get_error_reporting().report(context);

    assert_eq!(reporter.reported_count(), 1);
}

/// Test that the default tracing error reporter can be created.
#[test]
fn test_tracing_error_reporter() {
    let reporter = TracingErrorReporter;
    let error = MaukaError::Custom("test error".to_string());
    let context = ErrorContext::new(error, "test_component");

    // Just make sure this doesn't panic
    reporter.report(context);
}
