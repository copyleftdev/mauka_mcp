//! Error types for Niihau Header Trie.
//!
//! This module defines the error types that can occur during Niihau Trie operations.

/// Errors that can occur in Niihau Trie operations.
#[derive(Debug, thiserror::Error)]
pub enum NiihauTrieError {
    /// Error when an empty key is provided.
    #[error("Empty key not allowed")]
    EmptyKey,
    
    /// Error when a key exceeds the maximum depth allowed.
    #[error("Key '{key}' exceeds maximum trie depth of {max_depth}")]
    KeyTooLong {
        /// The key that was too long.
        key: String,
        /// The maximum allowed depth.
        max_depth: usize,
    },
    
    /// Error when a RwLock operation fails (poisoned lock).
    #[error("Failed to acquire lock (poisoned)")]
    LockError,
    
    /// Error when an operation is performed on a node that doesn't exist.
    #[error("Node not found for key: {0}")]
    NodeNotFound(String),
}

// Display implementation is automatically provided by thiserror

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = NiihauTrieError::EmptyKey;
        assert_eq!(err.to_string(), "Empty key not allowed");

        let err = NiihauTrieError::KeyTooLong {
            key: "test".to_string(),
            max_depth: 10,
        };
        assert_eq!(err.to_string(), "Key 'test' exceeds maximum trie depth of 10");

        let err = NiihauTrieError::LockError;
        assert_eq!(err.to_string(), "Failed to acquire lock (poisoned)");

        let err = NiihauTrieError::NodeNotFound("test".to_string());
        assert_eq!(err.to_string(), "Node not found for key: test");
    }
}
