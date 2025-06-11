// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Error types for the Boyer-Moore Pattern Matcher.

/// Error types for Boyer-Moore Pattern Matcher operations
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum BoyerMooreError {
    /// Empty pattern provided
    #[error("Pattern cannot be empty")]
    EmptyPattern,
    
    /// Pattern is too large
    #[error("Pattern exceeds maximum allowed length")]
    PatternTooLarge,
    
    /// Invalid UTF-8 sequence
    #[error("Invalid UTF-8 sequence in pattern or text")]
    InvalidUtf8,
    
    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

/// Result type for Boyer-Moore Pattern Matcher operations
pub type Result<T> = std::result::Result<T, BoyerMooreError>;
