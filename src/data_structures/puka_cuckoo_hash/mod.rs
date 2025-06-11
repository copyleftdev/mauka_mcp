// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Puka Cuckoo Hash implementation for efficient deduplication.
//!
//! A high-performance concurrent cuckoo hash table optimized for deduplication
//! workloads in the Mauka MCP Server. This implementation provides strong guarantees
//! about uniqueness while maintaining excellent insert and lookup performance.
//!
//! # Features
//!
//! - Thread-safe implementation suitable for concurrent environments
//! - Configurable load factor and rehashing parameters
//! - Optimized for high-throughput deduplication operations
//! - Efficient memory usage for practical deduplication scenarios
//! - Zero unsafe code for enhanced security
//!
//! # Example
//!
//! ```
//! use mauka_mcp_lib::data_structures::puka_cuckoo_hash::{PukaCuckooHash, PukaCuckooHashConfig};
//!
//! // Create a cuckoo hash table with default configuration
//! let table = PukaCuckooHash::<String, u32>::new();
//!
//! // Insert a value with no conflicts
//! assert!(table.insert("hello".to_string(), 42));
//!
//! // Retrieving a value
//! assert_eq!(table.get(&"hello".to_string()), Some(&42));
//!
//! // Failed lookup for non-existent key
//! assert_eq!(table.get(&"world".to_string()), None);
//!
//! // Deduplication in action - trying to insert the same key again fails
//! assert!(!table.insert("hello".to_string(), 100));
//! assert_eq!(table.get(&"hello".to_string()), Some(&42)); // Original value remains
//! ```
//!
//! # Deduplication Strategy
//!
//! The Puka Cuckoo Hash table is designed specifically for deduplication workloads:
//!
//! 1. It uses a cuckoo hashing strategy with multiple hash functions to resolve collisions
//! 2. New values can only be inserted if they don't already exist in the table
//! 3. Fast negative lookups are optimized for high-throughput filtering
//! 4. Memory efficiency is maintained even at high load factors
//!
//! Example deduplication workflow:
//!
//! ```
//! use mauka_mcp_lib::data_structures::puka_cuckoo_hash::PukaCuckooHash;
//!
//! struct DedupFilter<K, V> {
//!     filter: PukaCuckooHash<K, V>,
//! }
//!
//! impl<K: std::hash::Hash + Eq + Clone, V: Clone> DedupFilter<K, V> {
//!     pub fn process(&self, key: K, value: V) -> Option<V> {
//!         // Try to insert the value
//!         if self.filter.insert(key.clone(), value.clone()) {
//!             // First time seeing this key, process the value
//!             Some(value)
//!         } else {
//!             // Duplicate, skip processing
//!             None
//!         }
//!     }
//! }
//! ```

// Module declarations
mod config;
mod error;
mod table;
mod hash;

// Re-exports
pub use config::PukaCuckooHashConfig;
pub use error::{PukaCuckooHashError, Result};
pub use table::PukaCuckooHash;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_operations() {
        let table = PukaCuckooHash::<String, usize>::new();
        
        // Insert some values
        assert!(table.insert("hello".to_string(), 1));
        assert!(table.insert("world".to_string(), 2));
        
        // Verify lookups
        assert_eq!(table.get(&"hello".to_string()), Some(1));
        assert_eq!(table.get(&"world".to_string()), Some(2));
        assert_eq!(table.get(&"test".to_string()), None);
        
        // Check deduplication (no duplicate inserts)
        assert!(!table.insert("hello".to_string(), 100));
        assert_eq!(table.get(&"hello".to_string()), Some(1)); // Original value remains
    }
    
    #[test]
    fn test_custom_configuration() {
        let config = PukaCuckooHashConfig::new()
            .with_initial_capacity(1_000)
            .with_max_load_factor(0.7);
            
        let table = PukaCuckooHash::<String, u32>::with_config(config);
        
        assert!(table.insert("test-config".to_string(), 42));
        assert_eq!(table.get(&"test-config".to_string()), Some(42));
    }
}
