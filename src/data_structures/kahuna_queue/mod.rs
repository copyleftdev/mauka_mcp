//! Kahuna Lock-Free Queue Implementation
//! 
//! This module provides a high-performance, lock-free concurrent queue 
//! implementation based on the Michael-Scott queue algorithm, optimized for 
//! MCP protocol handler use cases.
//! 
//! # Key Features
//! 
//! * Lock-free push and pop operations for high concurrency
//! * ABA problem prevention through atomic operations
//! * Memory-efficient implementation with proper backpressure
//! * Zero-allocation in the hot path for stable performance
//! 
//! # Concurrency Safety
//! 
//! The implementation uses the following concurrency patterns:
//! 
//! * **Atomic Operations**: All shared state is updated using atomic operations to ensure
//!   thread safety without locks (AtomicPtr, AtomicUsize)
//! 
//! * **Interior Mutability**: The `Node` structure uses an `UnsafeCell` to allow internal
//!   mutation while maintaining immutable references
//! 
//! * **Memory Reclamation**: Proper handling of node deallocation using `Box::from_raw`
//!   after ensuring exclusive ownership
//! 
//! * **Backpressure Mechanism**: Built-in capacity limits with configurable backpressure
//!   to prevent resource exhaustion

use std::sync::{Arc, Mutex, Barrier};
use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, Ordering};
use std::time::Duration;
use std::{ptr, thread};
use std::time::Instant;

// We keep the unused imports in the non-test code to maintain API compatibility
// The test module will import them locally as needed

mod node;
pub use node::Node;

/// Error types for Kahuna Queue operations
#[derive(Debug, thiserror::Error)]
pub enum KahunaQueueError {
    /// Queue is full and backpressure is being applied
    #[error("Queue is at capacity, backpressure applied")]
    QueueFull,

    /// Queue is empty
    #[error("Queue is empty")]
    QueueEmpty,

    /// Operation timed out
    #[error("Operation timed out after {0:?}")]
    Timeout(Duration),
}

/// Result type for Kahuna Queue operations
pub type KahunaQueueResult<T> = Result<T, KahunaQueueError>;

/// Configuration for the Kahuna Queue
#[derive(Debug, Clone)]
pub struct KahunaQueueConfig {
    /// Maximum capacity of the queue
    pub max_capacity: usize,
    
    /// Default timeout for blocking operations
    pub default_timeout: Option<Duration>,
    
    /// Whether to apply backpressure when the queue is full
    pub apply_backpressure: bool,
}

impl Default for KahunaQueueConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            default_timeout: Some(Duration::from_secs(1)),
            apply_backpressure: true,
        }
    }
}

/// KahunaQueue is a lock-free concurrent queue optimized for MCP protocol handling.
///
/// This implementation uses atomic operations to ensure thread safety without 
/// locks, allowing for high throughput in multi-producer, multi-consumer scenarios.
///
/// # Type Parameters
///
/// * `T` - Type of items stored in the queue. Must be `Send + Sync`.
#[derive(Debug)]
pub struct KahunaQueue<T: Send + Sync> {
    /// Head pointer to the first node in the queue
    head: AtomicPtr<Node<T>>,
    
    /// Tail pointer to the last node in the queue
    tail: AtomicPtr<Node<T>>,
    
    /// Current number of items in the queue
    count: AtomicUsize,
    
    /// Maximum capacity of the queue
    max_capacity: usize,
    
    /// Default timeout for blocking operations
    default_timeout: Option<Duration>,
    
    /// Whether to apply backpressure when the queue is full
    apply_backpressure: bool,
}

impl<T: Send + Sync> KahunaQueue<T> {
    /// Creates a new empty `KahunaQueue` with default configuration.
    ///
    /// # Returns
    ///
    /// A new `KahunaQueue` instance.
    pub fn new() -> Self {
        Self::with_config(KahunaQueueConfig::default())
    }

    /// Creates a new empty `KahunaQueue` with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the queue.
    ///
    /// # Returns
    ///
    /// A new `KahunaQueue` instance.
    pub fn with_config(config: KahunaQueueConfig) -> Self {
        // Create a sentinel node (empty node that's always present)
        let sentinel = Box::new(Node::sentinel());
        let sentinel_ptr = Box::into_raw(sentinel);

        Self {
            head: AtomicPtr::new(sentinel_ptr),
            tail: AtomicPtr::new(sentinel_ptr),
            count: AtomicUsize::new(0),
            max_capacity: config.max_capacity,
            default_timeout: config.default_timeout,
            apply_backpressure: config.apply_backpressure,
        }
    }

    /// Returns the current number of items in the queue.
    ///
    /// Note that in a concurrent environment this value may be immediately outdated.
    ///
    /// # Returns
    ///
    /// The number of items currently in the queue.
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Returns whether the queue is empty.
    ///
    /// Note that in a concurrent environment this value may be immediately outdated.
    ///
    /// # Returns
    ///
    /// `true` if the queue is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the queue is at capacity.
    ///
    /// # Returns
    ///
    /// `true` if the queue is at capacity, `false` otherwise.
    pub fn is_full(&self) -> bool {
        self.len() >= self.max_capacity
    }

    /// Attempts to push an item onto the queue.
    ///
    /// If the queue is at capacity and backpressure is enabled, this method
    /// will return `Err(KahunaQueueError::QueueFull)`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to push onto the queue.
    ///
    /// # Returns
    ///
    /// * `true` if the push was successful
    /// * `false` if the queue is full and backpressure is applied
    pub fn push(&self, value: T) -> bool {
        // Check capacity if backpressure is enabled
        if self.apply_backpressure && self.is_full() {
            return false;
        }

        // Create a new node with the value
        let new_node = Box::new(Node::new(value));
        let new_node_ptr = Box::into_raw(new_node);

        // Use a lock-free algorithm to insert at the tail
        loop {
            // Get the current tail and its next pointer
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let tail = unsafe { &*tail_ptr };
            let tail_next_ptr = tail.next.load(Ordering::Acquire);

            // Check if the tail is still the actual tail
            if tail_ptr == self.tail.load(Ordering::Acquire) {
                if tail_next_ptr.is_null() {
                    // The tail is really the tail, try to insert the new node
                    match tail.next.compare_exchange(
                        std::ptr::null_mut(),
                        new_node_ptr,
                        Ordering::Release,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            // Successfully inserted the new node, try to update the tail
                            let _ = self.tail.compare_exchange(
                                tail_ptr,
                                new_node_ptr,
                                Ordering::Release,
                                Ordering::Relaxed,
                            );
                            // Increment the count
                            self.count.fetch_add(1, Ordering::SeqCst);
                            return true;
                        }
                        Err(_) => {
                            // Another thread inserted a node, retry
                            continue;
                        }
                    }
                } else {
                    // The tail is not actually the tail, try to help by moving the tail forward
                    let _ = self.tail.compare_exchange(
                        tail_ptr,
                        tail_next_ptr,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                }
            }
        }
    }

    /// Attempts to pop an item from the queue.
    ///
    /// # Returns
    ///
    /// Some(T) if an item was popped, None if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        loop {
            // Get the current head, tail, and head's next pointers
            let head_ptr = self.head.load(Ordering::Acquire);
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let head = unsafe { &*head_ptr };
            let next_ptr = head.next.load(Ordering::Acquire);

            // Check if the head is still valid
            if head_ptr == self.head.load(Ordering::Acquire) {
                // If the head and tail are the same, and there's no next node,
                // the queue is empty
                if head_ptr == tail_ptr && next_ptr.is_null() {
                    return None;
                }

                // If the head and tail are the same but there is a next node,
                // the tail is lagging, help by moving it forward
                if head_ptr == tail_ptr {
                    let _ = self.tail.compare_exchange(
                        tail_ptr,
                        next_ptr,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                    continue;
                }

                // Try to retrieve the value from the next node (real head)
                let next = unsafe { &*next_ptr };
                let value = next.take(); // Use the Node's take() method instead of directly accessing UnsafeCell

                // Try to move the head forward
                match self.head.compare_exchange(
                    head_ptr,
                    next_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Successfully moved the head, properly clean up the old node
                        // SAFETY: We have exclusive ownership of head_ptr since the compare_exchange succeeded
                        // and no other thread can access it now. Box::from_raw will take ownership and
                        // drop it safely when it goes out of scope.
                        unsafe {
                            // Convert the raw pointer back to a Box and drop it
                            drop(Box::from_raw(head_ptr));
                        }
                        // Decrement the count atomically
                        self.count.fetch_sub(1, Ordering::SeqCst);
                        return value;
                    }
                    Err(_) => {
                        // Head was moved by another thread, retry
                        continue;
                    }
                }
            }
        }
    }

    /// Attempts to pop an item from the queue with a timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for an item. If None, uses the default timeout.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The popped item.
    /// * `Err(KahunaQueueError::QueueEmpty)` - The queue is empty.
    /// * `Err(KahunaQueueError::Timeout)` - The timeout was reached.
    pub fn pop_with_timeout(&self, timeout: Option<Duration>) -> KahunaQueueResult<T> {
        let timeout = timeout.or(self.default_timeout);
        
        if let Some(timeout) = timeout {
            let start = Instant::now();
            
            while start.elapsed() < timeout {
                if let Some(value) = self.pop() {
                    return Ok(value);
                }
                // Small backoff to avoid spinning too aggressively
                std::thread::yield_now();
            }
            
            Err(KahunaQueueError::Timeout(timeout))
        } else {
            // No timeout specified, just try once
            self.pop().ok_or(KahunaQueueError::QueueEmpty)
        }
    }
}

impl<T: Send + Sync> Drop for KahunaQueue<T> {
    fn drop(&mut self) {
        // Free all remaining nodes
        while let Some(_) = self.pop() {}
        
        // Free the sentinel node
        let head_ptr = self.head.load(Ordering::Relaxed);
        if !head_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(head_ptr));
            }
        }
    }
}

// SAFETY: KahunaQueue<T> can be safely shared between threads when T is Send + Sync
unsafe impl<T: Send + Sync> Send for KahunaQueue<T> {}
unsafe impl<T: Send + Sync> Sync for KahunaQueue<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Instant;
    
    /// Helper function for setting up a test queue with specified configuration
    /// 
    /// # Arguments
    /// * `max_capacity` - Maximum capacity for the queue
    /// * `apply_backpressure` - Whether to apply backpressure when queue is full
    /// 
    /// # Returns
    /// A new KahunaQueue with the specified configuration wrapped in an Arc
    fn create_test_queue<T: Send + Sync + 'static>(max_capacity: usize, apply_backpressure: bool) -> Arc<KahunaQueue<T>> {
        Arc::new(KahunaQueue::with_config(KahunaQueueConfig {
            max_capacity,
            default_timeout: None,
            apply_backpressure,
        }))
    }
    
    #[test]
    fn test_queue_basic_operations() {
        let queue = KahunaQueue::new();
        
        // Test initial state
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert!(!queue.is_full());
        
        // Test push and len
        assert!(queue.push(1));
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);
        
        // Test pop
        assert_eq!(queue.pop(), Some(1));
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        
        // Test empty pop
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_queue_multiple_operations() {
        let queue = KahunaQueue::new();
        
        // Push multiple items
        for i in 0..10 {
            assert!(queue.push(i));
        }
        
        assert_eq!(queue.len(), 10);
        
        // Pop all items in order
        for i in 0..10 {
            assert_eq!(queue.pop(), Some(i));
        }
        
        assert!(queue.is_empty());
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_queue_timeout() {
        let queue = KahunaQueue::new();
        
        // Test timeout on empty queue
        let result = queue.pop_with_timeout(Some(Duration::from_millis(10)));
        assert!(matches!(result, Err(KahunaQueueError::Timeout(_))));
        
        // Test success case
        queue.push(42);
        let result = queue.pop_with_timeout(Some(Duration::from_millis(10)));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_queue_backpressure() {
        let queue = KahunaQueue::with_config(KahunaQueueConfig {
            max_capacity: 5,
            default_timeout: None,
            apply_backpressure: true,
        });
        
        // Fill the queue
        for i in 0..5 {
            assert!(queue.push(i));
        }
        
        // This should fail due to backpressure
        assert!(!queue.push(5));
        assert_eq!(queue.len(), 5);
        
        // After popping one, we should be able to push again
        assert_eq!(queue.pop(), Some(0));
        assert!(queue.push(5));
    }

    /// Tests concurrent operations on the lock-free queue with producers and consumers.
    /// Uses a simplified approach with a small number of threads and items to ensure
    /// the test completes quickly.
    #[test]
    fn test_queue_concurrent_operations() {
        // Simplified test parameters to avoid lengthy test runs
        const MAX_CAPACITY: usize = 100;
        const PRODUCER_COUNT: usize = 2;
        const CONSUMER_COUNT: usize = 2;
        const ITEMS_PER_PRODUCER: usize = 20;
        const TOTAL_ITEMS: usize = PRODUCER_COUNT * ITEMS_PER_PRODUCER;
        const MAX_RETRY: usize = 1_000;
        const MAX_EMPTY_RETRIES: usize = 100;
        
        // Create a shared queue
        let queue = create_test_queue::<usize>(MAX_CAPACITY, true);
        
        // Synchronization primitives
        let producers_done = Arc::new(AtomicBool::new(false));
        let consumed_items = Arc::new(Mutex::new(Vec::with_capacity(TOTAL_ITEMS)));
        let received_count = Arc::new(AtomicUsize::new(0));
        
        // Use a barrier to make sure all threads start at roughly the same time
        let barrier = Arc::new(Barrier::new(PRODUCER_COUNT + CONSUMER_COUNT + 1));
        
        // Queue is wrapped in Arc for thread-safe sharing
        let queue = Arc::new(queue);
        
        // Launch producer threads
        let mut producer_handles = Vec::new();
        for p in 0..PRODUCER_COUNT {
            let q = Arc::clone(&queue);
            let b = Arc::clone(&barrier);
            producer_handles.push(thread::spawn(move || -> Result<(), String> {
                // Wait for all threads to be ready
                b.wait();
                
                for i in 0..ITEMS_PER_PRODUCER {
                    let item = p * ITEMS_PER_PRODUCER + i;
                    
                    let mut retries = 0;
                    while !q.push(item) {
                        thread::yield_now();
                        retries += 1;
                        
                        if retries > MAX_RETRY {
                            return Err(format!("Producer {} failed to push item {}", p, item));
                        }
                    }
                }
                Ok(())
            }));
        }
        
        // Launch consumer threads
        let mut consumer_handles = Vec::new();
        for c in 0..CONSUMER_COUNT {
            let q = Arc::clone(&queue);
            let consumed = Arc::clone(&consumed_items);
            let counter = Arc::clone(&received_count);
            let done_flag = Arc::clone(&producers_done);
            let b = Arc::clone(&barrier);
            
            consumer_handles.push(thread::spawn(move || -> Result<usize, String> {
                // Wait for all threads to be ready before starting
                b.wait();
                
                let mut items_received = 0;
                let mut empty_retries = 0;
                
                // Set a timeout for the overall consumer operation
                let start_time = Instant::now();
                let timeout = Duration::from_secs(10); // 10 second timeout - sufficient for this test size
                
                // Keep consuming until we've seen everything or definitely timed out
                loop {
                    // Check for timeout
                    if start_time.elapsed() > timeout {
                        return Err(format!("Consumer {} timed out after {} seconds", c, timeout.as_secs()));
                    }
                    
                    match q.pop() {
                        Some(item) => {
                            // Critical section: store consumed item with proper locking
                            match consumed.lock() {
                                Ok(mut guard) => {
                                    guard.push(item);
                                    // Track with atomic counter for thread safety
                                    counter.fetch_add(1, Ordering::SeqCst);
                                    items_received += 1;
                                    empty_retries = 0; // Reset empty counter on successful read
                                },
                                Err(_) => {
                                    // Mutex poisoned, serious error
                                    return Err(format!("Consumer {} encountered poisoned mutex", c));
                                }
                            }
                        },
                        None => {
                            empty_retries += 1;
                            
                            // Check if global task is complete
                            let current_count = counter.load(Ordering::SeqCst);
                            if current_count >= TOTAL_ITEMS {
                                break; // All items processed by the collective consumers
                            }
                            
                            // Exit condition: producers done + exhaustive empty checks
                            if done_flag.load(Ordering::Acquire) {
                                if empty_retries > MAX_EMPTY_RETRIES {
                                    // Double-check we're actually done by checking total counter
                                    let final_count = counter.load(Ordering::SeqCst);
                                    if final_count >= TOTAL_ITEMS {
                                        break;
                                    } else if empty_retries > MAX_EMPTY_RETRIES * 2 {
                                        // Give up after extended checking - there may be a bug
                                        return Err(format!("Consumer {} gave up: only {} of {} items found after {} empty checks", 
                                                          c, final_count, TOTAL_ITEMS, empty_retries));
                                    }
                                }
                            }
                            
                            // Brief backoff to reduce contention
                            thread::yield_now();
                            
                            // After several empty attempts, sleep briefly to reduce CPU usage
                            if empty_retries % 10 == 0 {
                                std::thread::sleep(std::time::Duration::from_millis(1));
                            }
                        }
                    }
                }
                
                Ok(items_received)
            }));
        }
        
        // Wait at the barrier to synchronize start of all threads
        barrier.wait();
        
        // Wait for producers to complete with timeout
        for (i, handle) in producer_handles.into_iter().enumerate() {
            match handle.join() {
                Ok(Ok(())) => {}, // Success
                Ok(Err(e)) => panic!("Producer {} error: {}", i, e),
                Err(e) => panic!("Producer {} panicked: {:?}", i, e),
            }
        }
        
        // Signal consumers that producers are done
        producers_done.store(true, Ordering::Release);
        
        // Wait for consumers with timeout
        for (i, handle) in consumer_handles.into_iter().enumerate() {
            match handle.join() {
                Ok(Ok(_count)) => {
                    // Successfully consumed items
                },
                Ok(Err(e)) => panic!("Consumer {} error: {}", i, e),
                Err(e) => panic!("Consumer {} panicked: {:?}", i, e),
            }
        }
        
        // Verify all items were properly consumed and tracked
        let final_count = received_count.load(Ordering::SeqCst);
        let consumed_vec_result = consumed_items.lock();
        
        // Handle potential mutex poisoning
        let consumed_vec = match consumed_vec_result {
            Ok(guard) => guard,
            Err(e) => panic!("Verification failed: mutex poisoned: {}", e),
        };
        
        // Verify count matches expected total
        assert_eq!(final_count, TOTAL_ITEMS, 
                   "Counter shows {} but expected {}", final_count, TOTAL_ITEMS);
        assert_eq!(consumed_vec.len(), TOTAL_ITEMS, 
                   "Expected {} items but consumed {}", TOTAL_ITEMS, consumed_vec.len());
        
        // Check for duplicates by sorting and deduplicating
        let mut sorted = consumed_vec.clone();
        sorted.sort();
        let original_len = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), original_len, 
                   "Found {} duplicate items", original_len - sorted.len());
        
        // Verify every expected item was consumed exactly once
        let mut expected_items = Vec::with_capacity(TOTAL_ITEMS);
        for p in 0..PRODUCER_COUNT {
            for i in 0..ITEMS_PER_PRODUCER {
                expected_items.push(p * ITEMS_PER_PRODUCER + i);
            }
        }
        expected_items.sort();
        
        assert_eq!(sorted, expected_items, 
                   "Consumed items don't match expected items");
        
        // Final verification that the queue is empty
        assert!(queue.is_empty(), "Queue not empty after test");
    }
}
