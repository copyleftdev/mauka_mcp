//! Tests for the Kahuna Lock-Free Queue implementation.
//!
//! This module contains unit tests, property-based tests, and
//! performance tests for the Kahuna Queue.

use crate::data_structures::kahuna_queue::{KahunaQueue, KahunaQueueConfig};
use proptest::prelude::*;
use std::sync::{Arc, Barrier};
use std::thread;

/// Test basic queue operations (push/pop) in a single-threaded context
#[test]
fn test_basic_operations() {
    let queue = KahunaQueue::new();

    // Test simple push and pop
    assert!(queue.push(1));
    assert!(queue.push(2));
    assert!(queue.push(3));

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), None);

    // Test interleaved push and pop
    assert!(queue.push(10));
    assert_eq!(queue.pop(), Some(10));
    assert!(queue.push(20));
    assert!(queue.push(30));
    assert_eq!(queue.pop(), Some(20));
    assert!(queue.push(40));
    assert_eq!(queue.pop(), Some(30));
    assert_eq!(queue.pop(), Some(40));
    assert_eq!(queue.pop(), None);
}

/// Test queue behavior at capacity limits
#[test]
fn test_capacity_limits() {
    // Create a queue with small capacity for testing
    let queue = KahunaQueue::with_config(KahunaQueueConfig {
        max_capacity: 3,
        default_timeout: None,
        apply_backpressure: true,
    });

    // Fill to capacity
    assert!(queue.push(1));
    assert!(queue.push(2));
    assert!(queue.push(3));

    // Should fail to push when full
    assert!(!queue.push(4));

    // After popping, we can push again
    assert_eq!(queue.pop(), Some(1));
    assert!(queue.push(4));

    // Verify contents
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(4));
}

/// Test multithreaded push operations
#[test]
fn test_concurrent_push() {
    let queue = Arc::new(KahunaQueue::with_config(KahunaQueueConfig {
        max_capacity: 1000,
        default_timeout: None,
        apply_backpressure: false,
    }));
    let thread_count = 10;
    let items_per_thread = 100;

    let barrier = Arc::new(Barrier::new(thread_count));
    let mut handles = Vec::with_capacity(thread_count);

    // Spawn threads that will all push concurrently
    for i in 0..thread_count {
        let queue_clone: Arc<KahunaQueue<i32>> = Arc::clone(&queue);
        let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            let base = i * items_per_thread;
            barrier_clone.wait(); // Wait for all threads to be ready

            for j in 0..items_per_thread {
                let value = base + j;
                while !queue_clone.push(value as i32) {
                    thread::yield_now(); // Back off if queue is full
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Count items and check no duplicates
    let mut count = 0;
    let mut seen = std::collections::HashSet::<i32>::new();

    while let Some(item) = queue.pop() {
        count += 1;
        assert!(seen.insert(item), "Duplicate item found: {}", item);
    }

    assert_eq!(
        count,
        thread_count * items_per_thread,
        "Expected {} items but found {}",
        thread_count * items_per_thread,
        count
    );
}

/// Test multithreaded push and pop operations
#[test]
fn test_concurrent_push_pop() {
    let queue = Arc::new(KahunaQueue::with_config(KahunaQueueConfig {
        max_capacity: 100,
        default_timeout: None,
        apply_backpressure: false,
    }));
    let producer_count = 5;
    let consumer_count = 5;
    let items_per_producer = 1000;

    let total_expected_items = producer_count * items_per_producer;
    let barrier = Arc::new(Barrier::new(producer_count + consumer_count));

    // Track popped values
    let result = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Spawn producer threads
    let mut handles = Vec::new();
    for i in 0..producer_count {
        let queue_clone: Arc<KahunaQueue<i32>> = Arc::clone(&queue);
        let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            let base = i * items_per_producer;
            barrier_clone.wait();

            for j in 0..items_per_producer {
                let value = base + j;
                while !queue_clone.push(value as i32) {
                    thread::yield_now();
                }
            }
        });
        handles.push(handle);
    }

    // Spawn consumer threads
    for _ in 0..consumer_count {
        let queue_clone: Arc<KahunaQueue<i32>> = Arc::clone(&queue);
        let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
        let result_clone: Arc<std::sync::Mutex<Vec<i32>>> = Arc::clone(&result);

        let handle = thread::spawn(move || {
            let mut local_result = Vec::new();
            barrier_clone.wait();

            // Each consumer tries to get items until the expected total is reached
            loop {
                if let Some(item) = queue_clone.pop() {
                    local_result.push(item);
                } else {
                    thread::yield_now();
                }

                // Check globally if we're done
                let result = result_clone.lock().unwrap();
                let total_popped = result.len() + local_result.len();
                if total_popped >= total_expected_items {
                    break;
                }
                drop(result);
            }

            // Add our local results to the global collection
            let mut result = result_clone.lock().unwrap();
            result.append(&mut local_result);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify results
    let result = Arc::try_unwrap(result)
        .expect("Arc still has multiple owners")
        .into_inner()
        .expect("Mutex poisoned");

    assert_eq!(
        result.len(),
        total_expected_items,
        "Expected {} items but found {}",
        total_expected_items,
        result.len()
    );

    // Check for duplicates and missing values
    let mut seen = std::collections::HashSet::<i32>::new();
    for &item in &result {
        assert!(seen.insert(item), "Duplicate item found: {}", item);
    }

    for i in 0..producer_count {
        for j in 0..items_per_producer {
            let value = i * items_per_producer + j;
            assert!(seen.contains(&(value as i32)), "Missing value: {}", value);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn proptest_push_pop_sequence(operations in prop::collection::vec(prop::bool::ANY, 1..100)) {
        let queue = KahunaQueue::<i32>::new();
        let mut values = Vec::new();
        let mut next_value = 0;

        for &op_is_push in &operations {
            if op_is_push {
                // Push operation
                assert!(queue.push(next_value));
                values.push(next_value);
                next_value += 1;
            } else if !values.is_empty() {
                // Pop operation (only if we expect something in the queue)
                let expected = values.remove(0);
                assert_eq!(queue.pop(), Some(expected));
            }
        }

        // Drain remaining items
        for expected in values {
            assert_eq!(queue.pop(), Some(expected));
        }

        // Queue should be empty now
        assert_eq!(queue.pop(), None);
    }
}
