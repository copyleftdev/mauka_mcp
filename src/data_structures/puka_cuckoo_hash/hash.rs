// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Hash functions for the Puka Cuckoo Hash table.
//!
//! This module provides specialized hash functions for cuckoo hashing to minimize
//! collisions and ensure good distribution across the hash tables.

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// The number of seeds used for hash functions.
/// These prime numbers are chosen to provide good hash distribution.
#[allow(clippy::unreadable_literal)]
const HASH_SEEDS: [u64; 8] = [
    0x517cc1b727220a95, 0x83588256c732eb1f, 0xabe33b1c9b32d199, 0x4cf18d443988208f,
    0xd5c5778faf2a1ef1, 0xa22d34e45c79d3b5, 0xb3e52f89793f0af5, 0x9d1d10f8dd66cbcb,
];

/// Computes a hash value for the given key with the specified seed.
///
/// # Type Parameters
///
/// * `K` - The type of key being hashed.
///
/// # Arguments
///
/// * `key` - The key to hash.
/// * `seed` - The seed to use in the hash function.
///
/// # Returns
///
/// The computed hash value.
pub fn hash_with_seed<K: Hash + ?Sized>(key: &K, seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    key.hash(&mut hasher);
    hasher.finish()
}

/// Returns a set of hash values for the given key.
///
/// # Type Parameters
///
/// * `K` - The type of key being hashed.
///
/// # Arguments
///
/// * `key` - The key to hash.
/// * `hash_count` - The number of hash values to generate.
/// * `table_size` - The size of the hash table (used for modulo operation).
///
/// # Returns
///
/// The key is used to generate hash indices for the buckets in the hash table.
pub fn get_hash_indices<K: Hash + ?Sized>(key: &K, hash_count: usize, table_size: usize) -> Vec<usize> {
    (0..hash_count)
        .map(|i| {
            let hash = hash_with_seed(key, HASH_SEEDS[i]) as usize;
            hash % table_size
        })
        .collect()
}

/// Implements a uniform hashing strategy for cuckoo hashing.
///
/// This implementation provides multiple hash functions with good distribution
/// properties and minimal correlation to ensure effective cuckoo hashing.
#[derive(Debug)]
pub struct CuckooHasher {
    // Implementation details
    hash_count: usize,
    table_size: usize,
}

impl CuckooHasher {
    /// Creates a new CuckooHasher with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `hash_count` - The number of hash functions to use.
    /// * `table_size` - The size of each subtable.
    ///
    /// # Returns
    ///
    /// A new `CuckooHasher` instance.
    pub fn new(hash_count: usize, table_size: usize) -> Self {
        let hash_count = hash_count.clamp(1, HASH_SEEDS.len());
        Self {
            hash_count,
            table_size,
        }
    }

    /// Gets the hash indices for a key.
    ///
    /// # Type Parameters
    ///
    /// * `K` - The type of key being hashed.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to hash.
    ///
    /// # Returns
    ///
    /// A vector of hash indices for each subtable.
    pub fn get_indices<K: Hash + ?Sized>(&self, key: &K) -> Vec<usize> {
        get_hash_indices(key, self.hash_count, self.table_size)
    }

    /// Updates the table size (e.g., after a resize).
    ///
    /// # Arguments
    ///
    /// * `new_table_size` - The new size of each subtable.
    pub fn update_table_size(&mut self, new_table_size: usize) {
        self.table_size = new_table_size;
    }

    /// Gets the number of hash functions being used.
    ///
    /// # Returns
    ///
    /// The number of hash functions.
    pub fn hash_function_count(&self) -> usize {
        self.hash_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_indices_generation() {
        let key = "test_key".to_string();
        let indices = get_hash_indices(&key, 2, 1000);
        
        assert_eq!(indices.len(), 2);
        // Indices should be in range [0, 999]
        assert!(indices[0] < 1000);
        assert!(indices[1] < 1000);
    }

    #[test]
    fn test_hasher() {
        let hasher = CuckooHasher::new(3, 1000);
        let key = "another_key".to_string();
        let indices = hasher.get_indices(&key);
        
        assert_eq!(indices.len(), 3);
        // All indices should be different for a good hash function
        assert!(!(indices[0] == indices[1] && indices[0] == indices[2]));
    }
}
