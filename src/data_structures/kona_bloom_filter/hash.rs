// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Hashing utilities for Kona Bloom Filter.
//! 
//! This module provides specialized high-performance hash functions 
//! optimized for Bloom filter use cases. It uses a combination of hash functions
//! to generate multiple independent hashes from a single input efficiently.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A trait for computing multiple hash values from a single input.
pub(crate) trait MultiHasher {
    /// The type of the value to hash.
    type Value;
    
    /// Compute multiple hash values for the given value.
    /// 
    /// # Arguments
    /// 
    /// * `value` - The value to hash
    /// * `hash_count` - The number of hash values to generate
    /// * `bit_mask` - A bit mask to apply to each hash (typically size of bit array - 1)
    /// 
    /// # Returns
    /// 
    /// A vector of hash values (bit positions)
    fn compute_hashes(&self, value: &Self::Value, hash_count: usize, bit_mask: usize) -> Vec<usize>;
}

/// A multi-hasher implementation using the FNV algorithm combined with a double-hashing technique.
/// This provides a good balance of performance and distribution quality.
pub(crate) struct FnvMultiHasher<T> {
    /// Marker for the type of values this hasher works with
    _marker: PhantomData<fn(T)>,
}

impl<T> Default for FnvMultiHasher<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T> FnvMultiHasher<T> {
    /// Create a new instance of the FNV multi-hasher.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: Hash> MultiHasher for FnvMultiHasher<T> {
    type Value = T;
    
    fn compute_hashes(&self, value: &T, hash_count: usize, bit_mask: usize) -> Vec<usize> {
        // Use double hashing technique to generate multiple hash values efficiently
        let mut result = Vec::with_capacity(hash_count);
        
        // Get two independent hash values using different algorithms
        let h1 = calculate_hash1(value) & bit_mask;
        let h2 = calculate_hash2(value) & bit_mask;
        
        // Generate multiple hashes using the formula: h1 + i*h2 (mod bit_mask)
        // This is more efficient than computing separate hashes
        for i in 0..hash_count {
            let hash = (h1.wrapping_add(i.wrapping_mul(h2))) & bit_mask;
            result.push(hash);
        }
        
        result
    }
}

/// Calculate the first hash value using FNV-1a algorithm.
fn calculate_hash1<T: Hash>(value: &T) -> usize {
    let mut hasher = fnv::FnvHasher::default();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

/// Calculate the second hash value using a different algorithm to ensure independence.
/// This uses a modified FNV with a different prime number.
fn calculate_hash2<T: Hash>(value: &T) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_multi_hasher() {
        let hasher = FnvMultiHasher::<String>::new();
        let test_value = "test_string".to_string();
        
        // Test with 10 hash functions and a bit mask for 1024 bits
        let bit_mask = 1023; // 2^10 - 1
        let hashes = hasher.compute_hashes(&test_value, 10, bit_mask);
        
        // Check that we got the expected number of hashes
        assert_eq!(hashes.len(), 10);
        
        // Check that all hashes are within the bit mask range
        for hash in &hashes {
            assert!(*hash <= bit_mask);
        }
        
        // Check that we have some diversity in the hash values
        let unique_hashes = hashes.iter().collect::<HashSet<_>>();
        assert!(unique_hashes.len() >= 5); // Should have at least half unique values
    }

    #[test]
    fn test_hash_stability() {
        let hasher = FnvMultiHasher::<String>::new();
        let test_value = "stable_hash_test".to_string();
        
        // Generate hashes twice and ensure they're the same
        let first_run = hasher.compute_hashes(&test_value, 5, 1023);
        let second_run = hasher.compute_hashes(&test_value, 5, 1023);
        
        assert_eq!(first_run, second_run);
    }
    
    #[test]
    fn test_different_inputs_produce_different_hashes() {
        let hasher = FnvMultiHasher::<String>::new();
        let value1 = "input1".to_string();
        let value2 = "input2".to_string();
        
        let hashes1 = hasher.compute_hashes(&value1, 5, 1023);
        let hashes2 = hasher.compute_hashes(&value2, 5, 1023);
        
        // The hash sets should be different
        assert_ne!(hashes1, hashes2);
    }
}
