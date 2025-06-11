// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Error types for the Puka Cuckoo Hash table.

/// Error types for Puka Cuckoo Hash operations
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PukaCuckooHashError {
    /// Table is at capacity and cannot accommodate more elements
    #[error("Cuckoo hash table is at capacity, resize required")]
    TableFull,
    
    /// Rehashing failed after too many attempts
    #[error("Rehashing failed after maximum attempts, table may be too full")]
    RehashingFailed,
    
    /// Key already exists in the table (deduplication case)
    #[error("Key already exists in the table")]
    KeyExists,
    
    /// Configuration error
    #[error("Invalid configuration: {0}")]
    ConfigurationError(String),
    
    /// Concurrent operation conflict
    #[error("Concurrent operation conflict, retry required")]
    ConcurrencyConflict,
    
    /// Lock acquisition failure
    #[error("Failed to acquire lock, possible deadlock or contention")]
    LockError,
}

/// Result type for Puka Cuckoo Hash operations
pub type Result<T> = std::result::Result<T, PukaCuckooHashError>;
