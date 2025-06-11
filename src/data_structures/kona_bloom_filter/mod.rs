// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Kona Bloom Filter for admission control.
//!
//! A space-efficient probabilistic data structure designed for cache admission control.
//! The Kona Bloom Filter tracks which keys are likely to be accessed frequently
//! enough to warrant caching, while maintaining excellent performance characteristics
//! and a configurable, bounded memory footprint.
//!
//! # Features
//!
//! - Thread-safe, lock-free implementation suitable for high-concurrency environments.
//! - Configurable false positive rate and memory usage.
//! - Optimal hash function selection based on desired properties.
//! - Optional generational design for approximate time-based expiry.
//! - Zero unsafe code for enhanced security.
//!
//! # Example
//!
//! ```
//! use mauka_mcp_lib::data_structures::kona_bloom_filter::{KonaBloomFilter, KonaBloomFilterConfig};
//!
//! // Create a bloom filter with default configuration
//! let filter = KonaBloomFilter::<String>::new();
//!
//! // Insert an item
//! filter.insert("hello".to_string());
//!
//! // Check if an item exists
//! assert!(filter.check("hello".to_string()));
//! assert!(!filter.check("world".to_string()));
//! ```
//!
//! # Cache Admission Control
//!
//! One of the primary use cases for Bloom filters in caching is admission control,
//! deciding which items should be admitted to a capacity-constrained cache.
//! The Kona Bloom Filter helps identify items that have been accessed multiple times
//! and are therefore likely to benefit from caching.
//!
//! The classic "cache on second hit" strategy can be implemented as:
//!
//! ```
//! use mauka_mcp_lib::data_structures::kona_bloom_filter::KonaBloomFilter;
//!
//! struct CacheWithAdmissionControl<K, V> {
//!     filter: KonaBloomFilter<K>,
//!     // ... other cache fields ...
//! }
//!
//! impl<K: std::hash::Hash + Eq + Clone, V> CacheWithAdmissionControl<K, V> {
//!     pub fn get(&mut self, key: K) -> Option<&V> {
//!         // If item is in the cache, return it
//!         // if let Some(value) = self.cache.get(&key) {
//!         //     return Some(value);
//!         // }
//!
//!         // Track access in the filter
//!         let was_present = self.filter.check(key.clone());
//!         self.filter.insert(key.clone());
//!
//!         // Only cache if this is at least the second access
//!         if was_present {
//!             // Retrieve value and insert into cache
//!             // let value = fetch_value(key);
//!             // self.cache.insert(key, value);
//!             // return Some(self.cache.get(&key).unwrap());
//!         }
//!
//!         None
//!     }
//! }
//! ```

// Module declarations
mod config;
mod error;
mod filter;
mod hash;

// Re-exports
pub use config::KonaBloomFilterConfig;
pub use error::{KonaBloomFilterError, Result};
pub use filter::KonaBloomFilter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let filter = KonaBloomFilter::<String>::new();
        
        // Insert some values
        filter.insert("hello".to_string());
        filter.insert("world".to_string());
        
        // Check membership
        assert!(filter.check("hello".to_string()));
        assert!(filter.check("world".to_string()));
        assert!(!filter.check("test".to_string()));
    }
    
    #[test]
    fn test_custom_configuration() {
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(1_000)
            .with_false_positive_rate(0.001);
            
        let filter = KonaBloomFilter::<String>::with_config(config);
        
        filter.insert("test-config".to_string());
        assert!(filter.check("test-config".to_string()));
    }
}
