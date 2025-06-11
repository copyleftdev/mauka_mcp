// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Configuration for the Kona Bloom Filter.

use std::time::Duration;

/// Configuration for the Kona Bloom Filter.
///
/// This struct provides configuration options for tuning the Bloom filter's 
/// performance characteristics, including size, false positive rate, and
/// cache admission policies.
#[derive(Debug, Clone)]
pub struct KonaBloomFilterConfig {
    /// Expected number of items that will be inserted into the filter
    /// Used to calculate optimal bit array size
    expected_items: usize,
    
    /// Desired probability of false positives (0.0 to 1.0)
    /// Lower values increase accuracy but require more memory
    false_positive_rate: f64,
    
    /// Optional maximum size of the bit array in bytes
    /// If provided, will constrain the filter size regardless of other parameters
    max_size_bytes: Option<usize>,
    
    /// Number of hash functions to use
    /// If None, an optimal number will be calculated based on other parameters
    hash_functions: Option<usize>,
    
    /// Whether to use generational rotation of filters to handle entry expiry
    use_generations: bool,
    
    /// How often to rotate to a new generation (only if use_generations is true)
    generation_duration: Duration,
    
    /// Number of generations to maintain (only if use_generations is true)
    generation_count: usize,
}

impl KonaBloomFilterConfig {
    /// Create a new default configuration.
    ///
    /// Default values:
    /// - expected_items: 100,000
    /// - false_positive_rate: 0.01 (1%)
    /// - max_size_bytes: None (unconstrained)
    /// - hash_functions: None (auto-calculated)
    /// - use_generations: false
    /// - generation_duration: 1 hour
    /// - generation_count: 4
    pub fn new() -> Self {
        Self {
            expected_items: 100_000,
            false_positive_rate: 0.01,
            max_size_bytes: None,
            hash_functions: None,
            use_generations: false,
            generation_duration: Duration::from_secs(60 * 60), // 1 hour
            generation_count: 4,
        }
    }
    
    /// Set the expected number of items to be inserted into the filter.
    ///
    /// This value helps optimize the size of the bit array to achieve the 
    /// desired false positive rate. Setting an accurate value improves efficiency.
    pub fn with_expected_items(mut self, expected_items: usize) -> Self {
        self.expected_items = expected_items;
        self
    }
    
    /// Set the desired false positive rate (between 0.0 and 1.0).
    ///
    /// Lower values increase accuracy but require more memory.
    /// Typical values range from 0.01 (1%) to 0.001 (0.1%).
    pub fn with_false_positive_rate(mut self, false_positive_rate: f64) -> Self {
        if false_positive_rate <= 0.0 || false_positive_rate >= 1.0 {
            panic!("False positive rate must be between 0.0 and 1.0 exclusive");
        }
        self.false_positive_rate = false_positive_rate;
        self
    }
    
    /// Set a maximum size limit (in bytes) for the filter.
    ///
    /// This can be used to limit memory usage regardless of other parameters.
    /// If set, the filter will optimize within this constraint.
    pub fn with_max_size_bytes(mut self, max_size_bytes: usize) -> Self {
        self.max_size_bytes = Some(max_size_bytes);
        self
    }
    
    /// Explicitly set the number of hash functions.
    ///
    /// By default, the optimal number is calculated based on the false positive rate.
    /// Only use this if you have specific requirements.
    pub fn with_hash_functions(mut self, hash_functions: usize) -> Self {
        if hash_functions == 0 {
            panic!("Number of hash functions must be greater than 0");
        }
        self.hash_functions = Some(hash_functions);
        self
    }
    
    /// Enable generational rotation for handling entry expiry.
    ///
    /// When enabled, the filter maintains multiple generations and rotates them
    /// periodically, allowing entries to effectively "expire" over time.
    pub fn with_generations(mut self, use_generations: bool) -> Self {
        self.use_generations = use_generations;
        self
    }
    
    /// Set how often to rotate generations (if enabled).
    pub fn with_generation_duration(mut self, duration: Duration) -> Self {
        self.generation_duration = duration;
        self
    }
    
    /// Set how many generations to maintain (if enabled).
    pub fn with_generation_count(mut self, count: usize) -> Self {
        if count < 2 {
            panic!("Generation count must be at least 2");
        }
        self.generation_count = count;
        self
    }
    
    /// Calculate the optimal bit array size based on the expected items and false positive rate.
    ///
    /// This uses the formula: m = -n*ln(p)/(ln(2)^2) where:
    /// - m = bit array size
    /// - n = expected number of items
    /// - p = false positive probability
    pub fn calculate_optimal_bit_size(&self) -> usize {
        let n = self.expected_items as f64;
        let p = self.false_positive_rate;
        let m = -n * p.ln() / (std::f64::consts::LN_2 * std::f64::consts::LN_2);
        
        // Round up to the next byte boundary
        let bits = m.ceil() as usize;
        let bytes = (bits + 7) / 8;
        
        // Apply size constraint if set
        if let Some(max_bytes) = self.max_size_bytes {
            bytes.min(max_bytes)
        } else {
            bytes
        }
    }
    
    /// Calculate the optimal number of hash functions.
    ///
    /// This uses the formula: k = (m/n)*ln(2) where:
    /// - k = number of hash functions
    /// - m = bit array size in bits
    /// - n = expected number of items
    pub fn calculate_optimal_hash_functions(&self) -> usize {
        let byte_size = self.calculate_optimal_bit_size();
        let bit_size = byte_size * 8;
        
        let m = bit_size as f64;
        let n = self.expected_items as f64;
        let k = (m / n) * std::f64::consts::LN_2;
        
        // Always use at least one hash function, but cap at a reasonable maximum
        k.round().max(1.0).min(20.0) as usize
    }
    
    /// Get the number of hash functions to use
    pub fn get_hash_functions(&self) -> usize {
        self.hash_functions.unwrap_or_else(|| self.calculate_optimal_hash_functions())
    }
    
    /// Get the size of the bit array in bytes
    pub fn get_bit_array_size_bytes(&self) -> usize {
        self.calculate_optimal_bit_size()
    }
    
    /// Get whether generational filters are enabled
    pub fn get_use_generations(&self) -> bool {
        self.use_generations
    }
    
    /// Get the generation duration
    pub fn get_generation_duration(&self) -> Duration {
        self.generation_duration
    }
    
    /// Get the number of generations to maintain
    pub fn get_generation_count(&self) -> usize {
        self.generation_count
    }
}

impl Default for KonaBloomFilterConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KonaBloomFilterConfig::default();
        assert_eq!(config.expected_items, 100_000);
        assert_eq!(config.false_positive_rate, 0.01);
        assert_eq!(config.max_size_bytes, None);
        assert_eq!(config.hash_functions, None);
        assert!(!config.use_generations);
    }

    #[test]
    fn test_config_builder() {
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(50_000)
            .with_false_positive_rate(0.001)
            .with_max_size_bytes(4096)
            .with_hash_functions(5)
            .with_generations(true)
            .with_generation_duration(Duration::from_secs(300))
            .with_generation_count(3);
        
        assert_eq!(config.expected_items, 50_000);
        assert_eq!(config.false_positive_rate, 0.001);
        assert_eq!(config.max_size_bytes, Some(4096));
        assert_eq!(config.hash_functions, Some(5));
        assert!(config.use_generations);
        assert_eq!(config.generation_duration, Duration::from_secs(300));
        assert_eq!(config.generation_count, 3);
    }
    
    #[test]
    fn test_optimal_bit_size() {
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(10_000)
            .with_false_positive_rate(0.01);
        
        // Calculate expected bit size using the formula
        // m = -n*ln(p)/(ln(2)^2)
        let expected_bits = (-10_000.0 * 0.01f64.ln() / (f64::ln(2.0) * f64::ln(2.0))).ceil() as usize;
        let expected_bytes = (expected_bits + 7) / 8;
        
        assert_eq!(config.calculate_optimal_bit_size(), expected_bytes);
    }
    
    #[test]
    fn test_optimal_hash_functions() {
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(10_000)
            .with_false_positive_rate(0.01);
            
        // Get the bit size first
        let byte_size = config.calculate_optimal_bit_size();
        let bit_size = byte_size * 8;
        
        // Calculate expected hash functions using the formula
        // k = (m/n)*ln(2)
        let expected_k = ((bit_size as f64 / 10_000.0) * f64::ln(2.0)).round() as usize;
        
        assert_eq!(config.calculate_optimal_hash_functions(), expected_k);
    }
    
    #[test]
    #[should_panic(expected = "False positive rate must be between 0.0 and 1.0")]
    fn test_invalid_false_positive_rate() {
        let _config = KonaBloomFilterConfig::new()
            .with_false_positive_rate(1.5);
    }
    
    #[test]
    #[should_panic(expected = "Number of hash functions must be greater than 0")]
    fn test_invalid_hash_functions() {
        let _config = KonaBloomFilterConfig::new()
            .with_hash_functions(0);
    }
    
    #[test]
    #[should_panic(expected = "Generation count must be at least 2")]
    fn test_invalid_generation_count() {
        let _config = KonaBloomFilterConfig::new()
            .with_generation_count(1);
    }
}
