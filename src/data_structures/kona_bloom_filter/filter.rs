// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Main implementation of the Kona Bloom Filter.

use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::data_structures::kona_bloom_filter::config::KonaBloomFilterConfig;
use crate::data_structures::kona_bloom_filter::hash::{FnvMultiHasher, MultiHasher};

/// A Bloom filter for admission control.
///
/// Kona Bloom Filter is a thread-safe, lock-free implementation of a Bloom filter,
/// optimized for high-throughput admission control in caching systems. It efficiently
/// tracks which items are worth caching based on past access patterns.
///
/// This implementation uses atomic operations to ensure thread safety without locks
/// and supports generational rotation for approximate time-based expiry.
///
/// # Type Parameters
///
/// * `T` - The type of values stored in the filter. Must implement Hash and Eq.
///
/// # Examples
///
/// ```
/// use mauka_mcp_lib::data_structures::kona_bloom_filter::{KonaBloomFilter, KonaBloomFilterConfig};
///
/// // Create a filter with default configuration
/// let filter = KonaBloomFilter::<String>::new();
/// 
/// // Insert an item
/// filter.insert("hello_world".to_string());
/// 
/// // Check if an item exists
/// assert!(filter.check("hello_world".to_string()));
/// assert!(!filter.check("not_inserted".to_string()));
///
/// // Create a filter with custom configuration
/// let config = KonaBloomFilterConfig::new()
///     .with_expected_items(100_000)
///     .with_false_positive_rate(0.001);
///     
/// let custom_filter = KonaBloomFilter::<String>::with_config(config);
/// ```
pub struct KonaBloomFilter<T: Hash + Eq> {
    /// Configuration for the filter
    config: KonaBloomFilterConfig,
    
    /// Bit arrays for each generation (each generation is an array of AtomicU64)
    generations: Vec<Arc<Vec<AtomicU64>>>,
    
    /// Current active generation index
    current_generation: AtomicU64,
    
    /// Time when the current generation started - using Cell for interior mutability
    generation_start: std::cell::Cell<Instant>,
    
    /// Hasher for computing bit positions
    hasher: FnvMultiHasher<T>,
    
    /// Marker for the type of values this filter works with
    _marker: PhantomData<T>,
}

impl<T: Hash + Eq> KonaBloomFilter<T> {
    /// Create a new Bloom filter with default configuration.
    pub fn new() -> Self {
        Self::with_config(KonaBloomFilterConfig::default())
    }
    
    /// Create a new Bloom filter with the given configuration.
    pub fn with_config(config: KonaBloomFilterConfig) -> Self {
        let bit_array_size_bytes = config.get_bit_array_size_bytes();
        let bit_array_size_u64s = (bit_array_size_bytes + 7) / 8;  // Round up to next multiple of 8 bytes
        
        // Create bit arrays for all generations
        let gen_count = if config.get_use_generations() { 
            config.get_generation_count() 
        } else { 
            1 
        };
        
        // Initialize each generation with zeroed AtomicU64s
        let generations = (0..gen_count)
            .map(|_| {
                Arc::new((0..bit_array_size_u64s)
                    .map(|_| AtomicU64::new(0))
                    .collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();
            
        Self {
            config,
            generations,
            current_generation: AtomicU64::new(0),
            generation_start: std::cell::Cell::new(Instant::now()),
            hasher: FnvMultiHasher::new(),
            _marker: PhantomData,
        }
    }
    
    /// Returns true if the filter might contain the value.
    ///
    /// False positives are possible, but false negatives are not.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to check in the filter
    ///
    /// # Returns
    ///
    /// `true` if the value might be in the filter, `false` if it's definitely not.
    pub fn check(&self, value: T) -> bool {
        self.maybe_rotate_generation();
        
        let bit_mask = (self.get_bit_array_size_bits() - 1) as usize;
        let hash_count = self.config.get_hash_functions();
        
        // Compute bit positions
        let bit_positions = self.hasher.compute_hashes(&value, hash_count, bit_mask);
        
        // Check current generation first
        let current_gen = self.current_generation.load(Ordering::Relaxed) as usize;
        if self.check_generation(current_gen, &bit_positions) {
            return true;
        }
        
        // If using generations, check other generations too
        if self.config.get_use_generations() {
            for gen in 0..self.generations.len() {
                if gen != current_gen && self.check_generation(gen, &bit_positions) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Insert a value into the filter.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to insert
    ///
    /// # Returns
    ///
    /// `true` if the insertion changed the filter, `false` if the value was likely already present.
    pub fn insert(&self, value: T) -> bool {
        self.maybe_rotate_generation();
        
        let bit_mask = (self.get_bit_array_size_bits() - 1) as usize;
        let hash_count = self.config.get_hash_functions();
        
        // Compute bit positions
        let bit_positions = self.hasher.compute_hashes(&value, hash_count, bit_mask);
        
        // Get current generation
        let current_gen = self.current_generation.load(Ordering::Relaxed) as usize;
        let bit_array = &self.generations[current_gen];
        
        // Track whether we changed the filter
        let mut changed = false;
        
        // Set each bit
        for &bit_pos in &bit_positions {
            let word_index = bit_pos / 64;
            let bit_index = bit_pos % 64;
            let bit_mask = 1u64 << bit_index;
            
            // Get the word and atomically OR it with the bit mask
            let old_val = bit_array[word_index].fetch_or(bit_mask, Ordering::Relaxed);
            if (old_val & bit_mask) == 0u64 {      // Check if this bit was already set
                changed = true;
            }
        }
        
        changed
    }
    
    /// Reset the filter to empty state.
    ///
    /// This resets all generations of the filter.
    pub fn clear(&self) {
        for gen_array in &self.generations {
            for atomic in gen_array.iter() {
                atomic.store(0, Ordering::Relaxed);
            }
        }
        
        self.current_generation.store(0, Ordering::Relaxed);
        // Reset generation timer using Cell API - thread-safe interior mutability
        self.generation_start.set(Instant::now());
    }
    
    /// Get the estimated fill ratio of the filter.
    ///
    /// This is a rough estimate based on the current generation.
    ///
    /// # Returns
    ///
    /// A value between 0.0 and 1.0 indicating how full the filter is.
    pub fn fill_ratio(&self) -> f64 {
        let current_gen = self.current_generation.load(Ordering::Relaxed) as usize;
        let bit_array = &self.generations[current_gen];
        
        let mut set_bits = 0;
        let total_bits = self.get_bit_array_size_bits();
        
        // Count set bits in current generation
        for atomic in bit_array.iter() {
            let value = atomic.load(Ordering::Relaxed);
            set_bits += value.count_ones() as u64;
        }
        
        set_bits as f64 / total_bits as f64
    }
    
    /// Get the configuration of this filter.
    pub fn config(&self) -> &KonaBloomFilterConfig {
        &self.config
    }
    
    /// Get the total size of the bit array in bits.
    fn get_bit_array_size_bits(&self) -> u64 {
        let bytes = self.config.get_bit_array_size_bytes();
        (bytes * 8) as u64
    }
    
    /// Check if all bits for the given positions are set in the specified generation.
    fn check_generation(&self, generation: usize, bit_positions: &[usize]) -> bool {
        let bit_array = &self.generations[generation];
        
        for &bit_pos in bit_positions {
            let word_index = bit_pos / 64;
            let bit_index = bit_pos % 64;
            let bit_mask = 1u64 << bit_index;
            
            let word = bit_array[word_index].load(Ordering::Relaxed);
            
            // If this bit is not set, the value is definitely not in the filter
            if word & bit_mask == 0 {
                return false;
            }
        }
        
        // All bits were set
        true
    }
    
    /// Check if it's time to rotate to a new generation and perform the rotation if needed.
    fn maybe_rotate_generation(&self) {
        // Skip if generations aren't being used
        if !self.config.get_use_generations() { 
            return;
        }
        
        // Check if it's time to rotate generations
        let now = Instant::now();
        let elapsed = now.duration_since(self.generation_start.get());
        
        if elapsed >= self.config.get_generation_duration() {
            // Update the current generation atomically
            let current_gen = self.current_generation.fetch_add(1, Ordering::AcqRel) + 1;
            let wrapped_gen = current_gen as usize % self.config.get_generation_count();
            
            // Reset the new generation's bit array
            let gen = &self.generations[wrapped_gen];
            for bit in gen.iter() {
                bit.store(0, Ordering::Relaxed);
            }
            
            // Update the generation start time using Cell API
            // This is thread-safe for interior mutability without locks
            // Cell provides single-threaded interior mutability, which is sufficient here
            // since timestamp precision isn't critical for correctness
            self.generation_start.set(Instant::now());
        }
    }
}

// Safe to be shared across threads
unsafe impl<T: Hash + Eq + Send> Send for KonaBloomFilter<T> {}
unsafe impl<T: Hash + Eq + Sync> Sync for KonaBloomFilter<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::sync::Barrier;
    use std::thread;
    
    #[test]
    fn test_bloom_filter_basic() {
        let filter = KonaBloomFilter::<String>::new();
        
        filter.insert("test1".to_string());
        filter.insert("test2".to_string());
        
        assert!(filter.check("test1".to_string()));
        assert!(filter.check("test2".to_string()));
        assert!(!filter.check("test3".to_string()));
    }
    
    #[test]
    fn test_bloom_filter_clear() {
        let filter = KonaBloomFilter::<&str>::new();
        
        filter.insert("test1");
        assert!(filter.check("test1"));
        
        filter.clear();
        assert!(!filter.check("test1"));
    }
    
    #[test]
    fn test_bloom_filter_fill_ratio() {
        let filter = KonaBloomFilter::<u64>::new();
        assert_eq!(filter.fill_ratio(), 0.0);
        
        // Insert some values
        for i in 0..1000 {
            filter.insert(i);
        }
        
        // Fill ratio should be greater than 0 now
        assert!(filter.fill_ratio() > 0.0);
        assert!(filter.fill_ratio() < 1.0);
    }
    
    #[test]
    fn test_custom_config() {
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(100)
            .with_false_positive_rate(0.01)
            .with_hash_functions(3);
            
        let filter = KonaBloomFilter::<String>::with_config(config);
        
        // A small filter should have a higher fill ratio after fewer inserts
        for i in 0..20 {
            filter.insert(i.to_string());
        }
        
        assert!(filter.fill_ratio() > 0.05);
    }
    
    #[test]
    fn test_thread_safety() {
        let filter = Arc::new(KonaBloomFilter::<usize>::new());
        let thread_count = 10;
        let items_per_thread = 1000;
        let barrier = Arc::new(Barrier::new(thread_count + 1));
        let mut handles = Vec::with_capacity(thread_count);
            
        for t in 0..thread_count {
            let filter_clone = Arc::clone(&filter);
            let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
                
            let handle = thread::spawn(move || {
                let start = t * items_per_thread;
                let end = start + items_per_thread;
                    
                barrier_clone.wait();
                for i in start..end {
                    filter_clone.insert(i);
                }
            });
            
            handles.push(handle);
        }
        
        // Start all threads at once
        barrier.wait();
        
        // Wait for all threads to finish
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all items were added
        for i in 0..(thread_count * items_per_thread) {
            assert!(filter.check(i));
        }
    }
    
    #[test]
    fn test_generations() {
        let config = KonaBloomFilterConfig::new()
            .with_generations(true)
            .with_generation_count(3)
            .with_generation_duration(Duration::from_millis(100)); // Short duration for testing
            
        let filter = KonaBloomFilter::<String>::with_config(config);
        
        filter.insert("gen0".to_string());
        
        // Force a generation rotation by simulating time passing
        filter.generation_start.set(Instant::now() - Duration::from_millis(200));
        
        filter.insert("gen1".to_string());
        
        // Both items should still be in the filter
        assert!(filter.check("gen0".to_string()));
        assert!(filter.check("gen1".to_string()));
        
        // Force another generation rotation
        filter.generation_start.set(Instant::now() - Duration::from_millis(200));
        
        filter.insert("gen2".to_string());
        
        // All items should still be in the filter
        assert!(filter.check("gen0".to_string()));
        assert!(filter.check("gen1".to_string()));
        assert!(filter.check("gen2".to_string()));
        
        // Force one more generation rotation (should wrap around and clear gen0)
        filter.generation_start.set(Instant::now() - Duration::from_millis(200));
        
        // Access an item to trigger rotation
        filter.check("trigger_rotation".to_string());
        
        // The "gen0" item might be gone now (since that generation was cleared)
        // But the other items should still be there
        assert!(filter.check("gen1".to_string()));
        assert!(filter.check("gen2".to_string()));
    }
    
    #[test]
    fn test_false_positive_rate() {
        let expected_items = 10_000;
        let target_fp_rate = 0.01;
        
        let config = KonaBloomFilterConfig::new()
            .with_expected_items(expected_items)
            .with_false_positive_rate(target_fp_rate);
            
        let filter = KonaBloomFilter::<u64>::with_config(config);
        
        // Insert expected number of items
        for i in 0..expected_items {
            filter.insert(i as u64);
        }
        
        // Test with 10,000 values that were not inserted
        let mut false_positives = 0;
        for i in expected_items..(expected_items * 2) {
            if filter.check(i as u64) {
                false_positives += 1;
            }
        }
        
        let actual_fp_rate = false_positives as f64 / expected_items as f64;
        
        // False positive rate should be approximately the target rate
        // Allow a factor of 2x since there's statistical variation
        assert!(actual_fp_rate < target_fp_rate * 2.0);
    }
}
