// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Configuration options for the Puka Cuckoo Hash table.

/// Configuration for the Puka Cuckoo Hash table.
#[derive(Debug, Clone)]
pub struct PukaCuckooHashConfig {
    /// Initial capacity of the hash table (number of slots per subtable).
    /// The total number of slots will be this value multiplied by the number of hash functions.
    pub initial_capacity: usize,
    
    /// Maximum load factor before triggering a resize.
    /// Cuckoo hashing performs best with lower load factors (0.4-0.7).
    pub max_load_factor: f64,
    
    /// Number of hash functions to use.
    /// More hash functions increase the maximum achievable load factor but increase lookup costs.
    pub hash_function_count: usize,
    
    /// Maximum number of evictions during an insert before giving up and resizing.
    /// This prevents infinite loops during insertion.
    pub max_eviction_attempts: usize,
    
    /// Whether to use a thread-safe implementation.
    /// Set to false for single-threaded scenarios to improve performance.
    pub thread_safe: bool,
}

impl PukaCuckooHashConfig {
    /// Creates a new configuration with default values.
    ///
    /// # Returns
    ///
    /// A new `PukaCuckooHashConfig` instance with default settings.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Sets the initial capacity of the hash table.
    ///
    /// # Arguments
    ///
    /// * `initial_capacity` - The initial number of slots per subtable.
    ///
    /// # Returns
    ///
    /// Self with the updated configuration.
    pub fn with_initial_capacity(mut self, initial_capacity: usize) -> Self {
        self.initial_capacity = initial_capacity;
        self
    }
    
    /// Sets the maximum load factor before triggering a resize.
    ///
    /// # Arguments
    ///
    /// * `max_load_factor` - The maximum load factor (0.0 to 1.0).
    ///
    /// # Returns
    ///
    /// Self with the updated configuration.
    pub fn with_max_load_factor(mut self, max_load_factor: f64) -> Self {
        self.max_load_factor = max_load_factor.clamp(0.1, 0.95);
        self
    }
    
    /// Sets the number of hash functions to use.
    ///
    /// # Arguments
    ///
    /// * `hash_function_count` - The number of hash functions (2 to 8).
    ///
    /// # Returns
    ///
    /// Self with the updated configuration.
    pub fn with_hash_function_count(mut self, hash_function_count: usize) -> Self {
        self.hash_function_count = hash_function_count.clamp(2, 8);
        self
    }
    
    /// Sets the maximum number of eviction attempts.
    ///
    /// # Arguments
    ///
    /// * `max_eviction_attempts` - The maximum number of evictions.
    ///
    /// # Returns
    ///
    /// Self with the updated configuration.
    pub fn with_max_eviction_attempts(mut self, max_eviction_attempts: usize) -> Self {
        self.max_eviction_attempts = max_eviction_attempts;
        self
    }
    
    /// Sets whether to use a thread-safe implementation.
    ///
    /// # Arguments
    ///
    /// * `thread_safe` - Whether to use a thread-safe implementation.
    ///
    /// # Returns
    ///
    /// Self with the updated configuration.
    pub fn with_thread_safety(mut self, thread_safe: bool) -> Self {
        self.thread_safe = thread_safe;
        self
    }
}

impl Default for PukaCuckooHashConfig {
    fn default() -> Self {
        Self {
            initial_capacity: 1_024,  // 1K slots per subtable
            max_load_factor: 0.5,     // 50% load factor is typically good for cuckoo hashing
            hash_function_count: 2,    // Standard cuckoo hashing uses 2 hash functions
            max_eviction_attempts: 500, // Prevent pathological cases
            thread_safe: true,        // Thread-safety by default for MCP server use case
        }
    }
}
