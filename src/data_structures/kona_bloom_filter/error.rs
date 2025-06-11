// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Error types for the Kona Bloom Filter.

/// Errors that can occur in Kona Bloom Filter operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum KonaBloomFilterError {
    /// The filter is at capacity and can't accept more items
    #[error("Bloom filter is at capacity")]
    AtCapacity,

    /// The filter has an invalid configuration
    #[error("Invalid filter configuration: {0}")]
    InvalidConfiguration(String),

    /// Hash calculation error
    #[error("Hash calculation error: {0}")]
    HashError(String),

    /// Serialization or deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Storage error when persisting filter state
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Generational rotation error
    #[error("Generation rotation error: {0}")]
    GenerationError(String),
}

/// Result type for Kona Bloom Filter operations
pub type Result<T> = std::result::Result<T, KonaBloomFilterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KonaBloomFilterError::AtCapacity;
        assert_eq!(err.to_string(), "Bloom filter is at capacity");

        let err = KonaBloomFilterError::InvalidConfiguration("Too small".to_string());
        assert_eq!(err.to_string(), "Invalid filter configuration: Too small");
    }

    #[test]
    fn test_error_equality() {
        let err1 = KonaBloomFilterError::HashError("bad seed".to_string());
        let err2 = KonaBloomFilterError::HashError("bad seed".to_string());
        let err3 = KonaBloomFilterError::StorageError("IO error".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
