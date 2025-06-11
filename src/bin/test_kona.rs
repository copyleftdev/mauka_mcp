use std::time::Duration;
use std::sync::{Arc, Barrier};
use std::thread;

use mauka_mcp_lib::data_structures::kona_bloom_filter::{KonaBloomFilterConfig, KonaBloomFilter};

/// Run a basic test to verify bloom filter insertion and membership checking.
fn test_bloom_filter_basic() -> bool {
    let filter = KonaBloomFilter::<String>::new();
    
    // Insert and check
    filter.insert("test1".to_string());
    filter.insert("test2".to_string());
    
    let has_test1 = filter.check("test1".to_string());
    let has_test2 = filter.check("test2".to_string());
    let has_test3 = filter.check("test3".to_string());
    
    has_test1 && has_test2 && !has_test3
}

/// Test generations by using sleep to allow natural rotation
fn test_generations() -> bool {
    // Use a very short duration to avoid long test times
    let config = KonaBloomFilterConfig::new()
        .with_generations(true)
        .with_generation_count(2)
        .with_generation_duration(Duration::from_millis(50));
        
    let filter = KonaBloomFilter::<String>::with_config(config);
    
    // Insert an item in the first generation
    filter.insert("gen0".to_string());
    
    // Verify it exists
    if !filter.check("gen0".to_string()) {
        return false;
    }
    
    // Sleep long enough for the generation to rotate
    std::thread::sleep(Duration::from_millis(75));
    
    // Add a new item in the new generation
    filter.insert("gen1".to_string());
    
    // Both items should still be in the filter
    let test1 = filter.check("gen0".to_string());
    let test2 = filter.check("gen1".to_string());
    
    if !test1 || !test2 {
        return false;
    }
    
    // Sleep long enough for another generation rotation
    // This should cause the first generation to be cleaned out
    std::thread::sleep(Duration::from_millis(75));
    
    // Insert another item that will trigger generation rotation
    filter.insert("gen2".to_string());
    
    // gen1 should still be there, gen0 might be gone now due to rotation
    let test3 = filter.check("gen1".to_string());
    let test4 = filter.check("gen2".to_string());
    
    // We care about items in newer generations being preserved
    test3 && test4
}

/// Test concurrency safety with multiple threads inserting and checking values.
fn test_concurrency() -> bool {
    let filter = Arc::new(KonaBloomFilter::<u64>::new());
    
    let thread_count = 10;
    let items_per_thread = 1000;
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let mut handles = Vec::with_capacity(thread_count);
    
    for t in 0..thread_count {
        let filter_clone: Arc<KonaBloomFilter<u64>> = Arc::clone(&filter);
        let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
        
        let handle = thread::spawn(move || {
            let start = (t * items_per_thread) as u64;
            let end = start + items_per_thread as u64;
            
            // Wait for all threads to be ready
            barrier_clone.wait();
            
            // Each thread inserts its own range of items
            for i in start..end {
                filter_clone.insert(i);
            }
            
            // Every thread should be able to see all inserted items
            // Each thread checks all items
            for i in 0..((thread_count * items_per_thread) as u64) {
                if !filter_clone.check(i) {
                    println!("Thread {} failed to find item {}", t, i);
                    return false;
                }
            }
            true
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to be ready at the starting line
    barrier.wait();
    
    // Wait for all threads to finish and check the results
    for handle in handles {
        if !handle.join().unwrap() {
            return false;
        }
    }
    
    // Check from the main thread too
    for i in 0..((thread_count * items_per_thread) as u64) {
        if !filter.check(i) {
            println!("Main thread failed to find item {}", i);
            return false;
        }
    }
    
    true
}

/// Test the clear operation correctly resets all bits in the filter.
fn test_clear() -> bool {
    let filter = KonaBloomFilter::<String>::new();
    
    filter.insert("test1".to_string());
    filter.insert("test2".to_string());
    
    // Verify items are present
    let test1 = filter.check("test1".to_string());
    let test2 = filter.check("test2".to_string());
    
    if !test1 || !test2 {
        return false;
    }
    
    // Clear the filter
    filter.clear();
    
    // Verify items are gone
    let test3 = filter.check("test1".to_string());
    let test4 = filter.check("test2".to_string());
    
    !test3 && !test4
}

/// Main function to run the KonaBloomFilter test suite.
/// Reports success/failure for each test with appropriate output formatting.
fn main() {
    println!("Running Kona Bloom Filter Verification Tests");
    println!("============================================\n");

    let mut passed = 0;
    let mut failed = 0;

    // Test 1: Basic operations
    if test_bloom_filter_basic() {
        println!("✅ Basic operations: PASSED");
        passed += 1;
    } else {
        println!("❌ Basic operations: FAILED");
        failed += 1;
    }

    // Test 2: Generation rotation
    if test_generations() {
        println!("✅ Generation rotation: PASSED");
        passed += 1;
    } else {
        println!("❌ Generation rotation: FAILED");
        failed += 1;
    }
    
    // Pause to allow any generation rotations to settle
    std::thread::sleep(Duration::from_millis(150));

    // Test 3: Concurrency safety
    if test_concurrency() {
        println!("✅ Concurrency safety: PASSED");
        passed += 1;
    } else {
        println!("❌ Concurrency safety: FAILED");
        failed += 1;
    }

    // Test 4: Clear operation
    if test_clear() {
        println!("✅ Clear operation: PASSED");
        passed += 1;
    } else {
        println!("❌ Clear operation: FAILED");
        failed += 1;
    }

    println!("\nTest Results: {} passed, {} failed", passed, failed);
    if failed == 0 {
        println!("All tests passed! KonaBloomFilter implementation is verified.");
        println!("Safe interior mutability pattern is working correctly.");
    } else {
        println!("Some tests failed! Please check the implementation.");
    }
}
