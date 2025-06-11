// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

use mauka_mcp_lib::data_structures::{KonaBloomFilter, KonaBloomFilterConfig};
use proptest::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

// Constants for testing
const EXPECTED_ITEMS: usize = 10_000;
const FALSE_POSITIVE_RATE: f64 = 0.01;
const THREAD_COUNT: usize = 8;
const ITEMS_PER_THREAD: usize = 1_000;

/// Setup a test filter with specific configuration
fn setup_test_filter<T>() -> KonaBloomFilter<T> 
where 
    T: std::hash::Hash + Eq + Send + Sync + 'static,
{
    let config = KonaBloomFilterConfig::new()
        .with_expected_items(EXPECTED_ITEMS)
        .with_false_positive_rate(FALSE_POSITIVE_RATE);
    
    KonaBloomFilter::with_config(config)
}

/// Test basic insertion and lookup operations
#[test]
fn test_basic_operations() {
    let filter = setup_test_filter::<String>();
    
    let values = vec![
        "test1".to_string(), 
        "test2".to_string(), 
        "test3".to_string()
    ];
    
    // Check that no values are initially present
    for val in &values {
        assert!(!filter.check(val.clone()), "Value should not be in filter initially");
    }
    
    // Insert values and verify they are present
    for val in &values {
        filter.insert(val.clone());
        assert!(filter.check(val.clone()), "Value should be in filter after insertion");
    }
    
    // Check that uninserted values are not found
    assert!(!filter.check("not_inserted".to_string()), 
           "Uninserted value should not be in filter");
}

/// Test the filter's clear method
#[test]
fn test_clear() {
    let filter = setup_test_filter::<String>();
    
    // Insert some values
    filter.insert("value1".to_string());
    filter.insert("value2".to_string());
    
    // Verify they are present
    assert!(filter.check("value1".to_string()));
    assert!(filter.check("value2".to_string()));
    
    // Clear the filter
    filter.clear();
    
    // Verify values are no longer present
    assert!(!filter.check("value1".to_string()));
    assert!(!filter.check("value2".to_string()));
}

/// Test the filter's fill ratio
#[test]
fn test_fill_ratio() {
    let filter = setup_test_filter::<u32>();
    
    // Initially the fill ratio should be 0
    assert_eq!(filter.fill_ratio(), 0.0);
    
    // Insert many values to fill the filter somewhat
    for i in 0..1000 {
        filter.insert(i);
    }
    
    // Fill ratio should be greater than 0 but less than 1
    let ratio = filter.fill_ratio();
    assert!(ratio > 0.0);
    assert!(ratio < 1.0);
}

/// Test that generational rotation works properly
#[test]
fn test_generations() {
    // Create a filter with generations enabled
    let config = KonaBloomFilterConfig::new()
        .with_expected_items(1_000)
        .with_generations(true)
        .with_generation_count(3)
        .with_generation_duration(Duration::from_millis(50));
        
    let filter = KonaBloomFilter::<String>::with_config(config);
    
    // Insert a value
    filter.insert("gen0".to_string());
    
    // Sleep to allow generation rotation
    thread::sleep(Duration::from_millis(60));
    
    // Insert another value in the new generation
    filter.insert("gen1".to_string());
    
    // Both values should still be in the filter
    assert!(filter.check("gen0".to_string()));
    assert!(filter.check("gen1".to_string()));
    
    // Sleep to allow two more generation rotations
    thread::sleep(Duration::from_millis(120));
    
    // This access should trigger a rotation
    filter.check("trigger".to_string());
    
    // Insert a new value
    filter.insert("gen3".to_string());
    
    // The oldest value might be gone now since generations wrapped around
    assert!(filter.check("gen1".to_string()));
    assert!(filter.check("gen3".to_string()));
}

/// Test that the false positive rate is approximately as expected
#[test]
fn test_false_positive_rate() {
    // Create a filter with known parameters
    let config = KonaBloomFilterConfig::new()
        .with_expected_items(10_000)
        .with_false_positive_rate(FALSE_POSITIVE_RATE);
        
    let filter = KonaBloomFilter::<u32>::with_config(config);
    
    // Insert expected number of items
    for i in 0..10_000 {
        filter.insert(i);
    }
    
    // Test with 10,000 values that were NOT inserted
    let mut false_positive_count = 0;
    for i in 10_000..20_000 {
        if filter.check(i) {
            false_positive_count += 1;
        }
    }
    
    // Calculate the observed false positive rate
    let observed_rate = false_positive_count as f64 / 10_000.0;
    
    // The observed rate should be within a reasonable factor of the expected rate
    // (using a factor of 2x to account for statistical variation)
    assert!(observed_rate < FALSE_POSITIVE_RATE * 2.0, 
           "False positive rate too high: {}", observed_rate);
}

/// Test concurrent insertions from multiple threads
#[test]
fn test_concurrent_insertions() {
    let filter = Arc::new(setup_test_filter::<usize>());
    let barrier = Arc::new(Barrier::new(THREAD_COUNT + 1));
    let mut handles = Vec::with_capacity(THREAD_COUNT);
    
    // Spawn threads to insert values
    for t in 0..THREAD_COUNT {
        let filter = Arc::clone(&filter);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            let start = t * ITEMS_PER_THREAD;
            let end = start + ITEMS_PER_THREAD;
            
            // Wait for all threads to be ready
            barrier.wait();
            
            // Insert values
            for i in start..end {
                filter.insert(i);
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
    
    // Verify all items were added successfully
    for i in 0..(THREAD_COUNT * ITEMS_PER_THREAD) {
        assert!(filter.check(i), "Value {} should be in the filter", i);
    }
}

/// Test concurrent insertions and lookups from multiple threads
#[test]
fn test_concurrent_insertions_and_lookups() {
    let filter = Arc::new(setup_test_filter::<usize>());
    let barrier = Arc::new(Barrier::new(THREAD_COUNT * 2 + 1)); // Writers + readers + main
    let inserted_count = Arc::new(AtomicUsize::new(0));
    let found_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    
    // Spawn writer threads
    for t in 0..THREAD_COUNT {
        let filter = Arc::clone(&filter);
        let barrier = Arc::clone(&barrier);
        let inserted_count = Arc::clone(&inserted_count);
        
        let handle = thread::spawn(move || {
            let start = t * ITEMS_PER_THREAD;
            let end = start + ITEMS_PER_THREAD;
            
            // Wait for all threads to be ready
            barrier.wait();
            
            // Insert values
            for i in start..end {
                if filter.insert(i) {
                    inserted_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Spawn reader threads
    for t in 0..THREAD_COUNT {
        let filter = Arc::clone(&filter);
        let barrier = Arc::clone(&barrier);
        let found_count = Arc::clone(&found_count);
        
        let handle = thread::spawn(move || {
            let start = t * ITEMS_PER_THREAD;
            let end = start + ITEMS_PER_THREAD;
            
            // Wait for all threads to be ready
            barrier.wait();
            
            // Check for values (may see some, may not, depending on timing)
            for i in start..end {
                if filter.check(i) {
                    found_count.fetch_add(1, Ordering::Relaxed);
                }
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
    
    // We can't make strong assertions about exactly how many items were found
    // since there's a race between insertion and lookup, but we can make some
    // basic sanity checks
    println!("Inserted count: {}", inserted_count.load(Ordering::Relaxed));
    println!("Found count: {}", found_count.load(Ordering::Relaxed));
    
    // After all threads complete, all items should be in the filter
    for i in 0..(THREAD_COUNT * ITEMS_PER_THREAD) {
        assert!(filter.check(i), "Value {} should be in the filter after all threads complete", i);
    }
}

/// Test high contention scenario with many threads
#[test]
fn test_high_contention() {
    let filter = Arc::new(setup_test_filter::<u32>());
    let barrier = Arc::new(Barrier::new(THREAD_COUNT * 2 + 1)); // Writers + readers + main
    let mut handles = Vec::new();
    
    const SHARED_VALUE_COUNT: u32 = 100;
    
    // Spawn writer threads that all insert the same values
    for _ in 0..THREAD_COUNT {
        let filter = Arc::clone(&filter);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            // All threads insert the same values
            for i in 0..SHARED_VALUE_COUNT {
                filter.insert(i);
            }
        });
        
        handles.push(handle);
    }
    
    // Spawn reader threads that check the same values
    for _ in 0..THREAD_COUNT {
        let filter = Arc::clone(&filter);
        let barrier = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            barrier.wait();
            
            // All threads check the same values repeatedly
            for _ in 0..10 {
                for i in 0..SHARED_VALUE_COUNT {
                    let _ = filter.check(i);
                }
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
    
    // Verify all shared values are in the filter
    for i in 0..SHARED_VALUE_COUNT {
        assert!(filter.check(i));
    }
}

/// Test performance under moderate load
#[test]
fn test_performance() {
    let filter = setup_test_filter::<usize>();
    
    // Measure insertion time
    let start = Instant::now();
    for i in 0..10_000 {
        filter.insert(i);
    }
    let insertion_time = start.elapsed();
    
    // Measure lookup time
    let start = Instant::now();
    for i in 0..10_000 {
        filter.check(i);
    }
    let lookup_time = start.elapsed();
    
    // Print performance metrics
    println!("Insertion time for 10,000 items: {:?}", insertion_time);
    println!("Lookup time for 10,000 items: {:?}", lookup_time);
    println!("Average insertion time: {:?} per item", insertion_time / 10_000);
    println!("Average lookup time: {:?} per item", lookup_time / 10_000);
    
    // Basic performance assertions
    assert!(insertion_time.as_micros() < 1_000_000, "Insertions should complete in under 1 second");
    assert!(lookup_time.as_micros() < 1_000_000, "Lookups should complete in under 1 second");
}

// Property-based tests
proptest! {
    /// Test that any inserted item is always found
    #[test]
    fn proptest_insert_then_find(values in prop::collection::vec(any::<u64>(), 1..100)) {
        let filter = setup_test_filter::<u64>();
        
        for val in &values {
            filter.insert(*val);
        }
        
        for val in &values {
            prop_assert!(filter.check(*val));
        }
    }
    
    /// Test that clearing works for any set of values
    #[test]
    fn proptest_clear_removes_all(values in prop::collection::vec(any::<u64>(), 1..100)) {
        let filter = setup_test_filter::<u64>();
        
        for val in &values {
            filter.insert(*val);
        }
        
        filter.clear();
        
        for val in &values {
            prop_assert!(!filter.check(*val));
        }
    }
}
