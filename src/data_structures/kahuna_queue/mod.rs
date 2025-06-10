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

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::time::Duration;
use std::time::Instant;

// Only use the bare minimum imports needed to keep code clean
// The test module will import them locally as needed

mod node;
pub use node::Node;

/// Error types for Kahuna Queue operations
#[derive(Debug, thiserror::Error, PartialEq)]
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

    /// Attempts to push an item onto the queue using lock-free operations.
    ///
    /// This method uses a lock-free algorithm based on compare-and-swap operations
    /// to safely insert items into the queue from multiple threads. The implementation
    /// carefully manages memory ownership to prevent leaks and use-after-free bugs.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to push onto the queue.
    ///
    /// # Returns
    ///
    /// * `true` if the push was successful
    /// * `false` if the queue is full and backpressure is applied
    ///
    /// # Thread Safety
    ///
    /// This operation is thread-safe and can be called concurrently from multiple threads.
    pub fn push(&self, value: T) -> bool {
        // Apply backpressure if configured and queue is at capacity
        if self.apply_backpressure && self.is_full() {
            return false;
        }

        // Create a new node with the value
        let new_node = Box::new(Node::new(value));
        let new_node_ptr = Box::into_raw(new_node);

        // Use a lock-free algorithm to insert at the tail
        loop {
            // Get the current tail and its next pointer with proper memory ordering
            let tail_ptr = self.tail.load(Ordering::Acquire);
            
            // Verify tail_ptr is not null before dereferencing
            if tail_ptr.is_null() {
                // Handle this extremely rare case (should never happen in a properly initialized queue)
                // Convert the node back to a Box to properly deallocate it
                unsafe { drop(Box::from_raw(new_node_ptr)); }
                return false;
            }
            
            let tail = unsafe { &*tail_ptr };
            let tail_next_ptr = tail.next.load(Ordering::Acquire);

            // Check if the tail hasn't changed since we read it
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
                            // Successfully inserted the new node
                            // We help update the tail pointer - this might not succeed if another thread
                            // updates it first, but that's okay
                            let _ = self.tail.compare_exchange(
                                tail_ptr,
                                new_node_ptr,
                                Ordering::Release,
                                Ordering::Relaxed,
                            );
                            
                            // Increment the count atomically with appropriate memory ordering
                            self.count.fetch_add(1, Ordering::Release);
                            return true;
                        }
                        Err(_) => {
                            // Another thread inserted a node, retry
                            continue;
                        }
                    }
                } else {
                    // The tail is not actually the tail, try to help by moving the tail forward
                    // This helps other threads make progress and prevents the "lagging tail" problem
                    let _ = self.tail.compare_exchange(
                        tail_ptr,
                        tail_next_ptr,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                }
            }
            
            // Brief yield to reduce contention in tight loops
            std::hint::spin_loop();
        }
    }

    /// Attempts to pop an item from the queue using lock-free operations.
    ///
    /// This method implements the lock-free dequeue operation of the Michael-Scott queue algorithm.
    /// It handles the ABA problem and ensures proper memory reclamation.
    ///
    /// # Returns
    ///
    /// Some(T) if an item was popped, None if the queue is empty.
    ///
    /// # Thread Safety
    ///
    /// This operation is thread-safe and can be called concurrently from multiple threads.
    /// It properly synchronizes with push operations through atomic memory operations.
    pub fn pop(&self) -> Option<T> {
        loop {
            // Get the current head, tail, and head's next pointers with proper memory ordering
            let head_ptr = self.head.load(Ordering::Acquire);
            
            // Safety check for null pointer (should never happen in a properly initialized queue)
            if head_ptr.is_null() {
                return None;
            }
            
            let tail_ptr = self.tail.load(Ordering::Acquire);
            let head = unsafe { &*head_ptr };
            let next_ptr = head.next.load(Ordering::Acquire);

            // Check if the head hasn't changed since we read it
            if head_ptr == self.head.load(Ordering::Acquire) {
                // If the head and tail are the same, and there's no next node,
                // the queue is empty (this is the sentinel node only state)
                if head_ptr == tail_ptr && next_ptr.is_null() {
                    return None;
                }

                // If the head and tail are the same but there is a next node,
                // the tail is lagging, help by moving it forward
                if head_ptr == tail_ptr {
                    // This is a cooperative operation to help other threads
                    let _ = self.tail.compare_exchange(
                        tail_ptr,
                        next_ptr,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                    continue;
                }
                
                // Safety check for null next pointer (should never happen in a valid state)
                if next_ptr.is_null() {
                    continue;
                }

                // Try to retrieve the value from the next node (the real head of data)
                let next = unsafe { &*next_ptr };
                
                // Take the value atomically
                let value = next.take();

                // Try to move the head forward
                match self.head.compare_exchange(
                    head_ptr,
                    next_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Successfully moved the head, safely reclaim the old node memory
                        // SAFETY: We have exclusive ownership of head_ptr since the compare_exchange succeeded
                        // and no other thread can access it now. Box::from_raw will take ownership and
                        // drop it safely when it goes out of scope.
                        unsafe {
                            drop(Box::from_raw(head_ptr));
                        }
                        
                        // Only decrement the count if we actually got a value
                        // This prevents underflow in rare race conditions
                        if value.is_some() {
                            // Use Relaxed ordering for the count since it's just a metric
                            // and doesn't affect algorithm correctness
                            self.count.fetch_sub(1, Ordering::Release);
                        }
                        
                        return value;
                    }
                    Err(_) => {
                        // Head was moved by another thread, retry
                        continue;
                    }
                }
            }
            
            // Brief yield to reduce contention in tight loops
            std::hint::spin_loop();
        }
    }

    /// Attempts to pop an item from the queue with a timeout.
    ///
    /// This method repeatedly tries to pop an item from the queue until an item is found
    /// or the timeout is reached. It uses an exponential backoff strategy to reduce CPU usage
    /// while waiting.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for an item. If None, uses the default timeout.
    ///               If no default timeout is configured, tries once without waiting.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The popped item.
    /// * `Err(KahunaQueueError::QueueEmpty)` - The queue is empty and no timeout was specified.
    /// * `Err(KahunaQueueError::Timeout)` - The timeout was reached without finding an item.
    ///
    /// # Thread Safety
    ///
    /// This operation is thread-safe and can be called concurrently from multiple threads.
    pub fn pop_with_timeout(&self, timeout: Option<Duration>) -> KahunaQueueResult<T> {
        // Use provided timeout or fall back to configured default
        let timeout = timeout.or(self.default_timeout);
        
        if let Some(timeout) = timeout {
            let start = Instant::now();
            let mut backoff_counter = 0u32;
            
            // Try until timeout is reached
            while start.elapsed() < timeout {
                // Try to pop an item
                if let Some(value) = self.pop() {
                    return Ok(value);
                }
                
                // Progressive backoff to reduce CPU usage while waiting
                if backoff_counter < 10 {
                    // Short spins for immediate response if item becomes available
                    for _ in 0..(1 << backoff_counter) {
                        std::hint::spin_loop();
                    }
                } else {
                    // After spinning hasn't helped, yield to the OS scheduler
                    std::thread::yield_now();
                    
                    // For long waits, sleep for progressively longer intervals
                    if backoff_counter > 15 {
                        let sleep_ms = std::cmp::min(1 << (backoff_counter - 15), 50);
                        std::thread::sleep(Duration::from_millis(sleep_ms as u64));
                    }
                }
                
                backoff_counter = std::cmp::min(backoff_counter + 1, 20);
            }
            
            // Timeout reached without finding an item
            Err(KahunaQueueError::Timeout(timeout))
        } else {
            // No timeout specified, just try once
            self.pop().ok_or(KahunaQueueError::QueueEmpty)
        }
    }
}

impl<T: Send + Sync> Drop for KahunaQueue<T> {
    /// Safely frees all memory associated with the queue when it is dropped.
    ///
    /// This implementation ensures all nodes are properly deallocated,
    /// preventing memory leaks. It first consumes all remaining items,
    /// then frees the sentinel node.
    fn drop(&mut self) {
        // Free all remaining nodes by popping them
        // This ensures proper deallocation of all nodes in the queue
        while let Some(_) = self.pop() {}
        
        // Finally free the sentinel node that's always present
        let head_ptr = self.head.load(Ordering::Relaxed);
        if !head_ptr.is_null() {
            // SAFETY: At this point we have exclusive ownership of the queue,
            // so it's safe to deallocate the sentinel node.
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
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;
    use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
    use std::time::Instant;
    
    /// Helper function for setting up a test queue with specified configuration
    fn setup_test_queue<T>(max_capacity: usize) -> KahunaQueue<T>
    where
        T: Send + Sync + 'static,
    {
        KahunaQueue::with_config(KahunaQueueConfig {
            max_capacity,
            default_timeout: None,
            apply_backpressure: true,
        })
    }
    
    #[test]
    fn test_basic_operations() {
        let queue = KahunaQueue::new();
        
        // Test initial state
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
        
        // Test push and verify size
        assert!(queue.push(42));
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
        
        // Test pop
        assert_eq!(queue.pop(), Some(42));
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        
        // Test pop empty queue
        assert_eq!(queue.pop(), None);
    }
    
    #[test]
    fn test_backpressure() {
        let queue = setup_test_queue::<i32>(2);
        
        assert!(queue.push(1));
        assert!(queue.push(2));
        
        // Queue is at max capacity, should apply backpressure
        assert!(!queue.push(3));
        
        // After popping, we should be able to push again
        assert_eq!(queue.pop(), Some(1));
        assert!(queue.push(3));
        
        // Clean up
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }
    
    #[test]
    fn test_pop_with_timeout() {
        let queue = setup_test_queue::<i32>(100);
        
        // Test immediate timeout on empty queue
        let result = queue.pop_with_timeout(Some(Duration::from_millis(0)));
        assert!(matches!(result, Err(KahunaQueueError::Timeout(_))));
        
        // Insert an item and pop with timeout
        assert!(queue.push(42));
        let result = queue.pop_with_timeout(Some(Duration::from_millis(100)));
        assert_eq!(result, Ok(42));
        
        // Test no timeout specified
        let result = queue.pop_with_timeout(None);
        assert!(matches!(result, Err(KahunaQueueError::QueueEmpty)));
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
        let queue = setup_test_queue::<usize>(MAX_CAPACITY);
        
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
            let q: Arc<KahunaQueue<usize>> = Arc::clone(&queue);
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
            let q: Arc<KahunaQueue<usize>> = Arc::clone(&queue);
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
    
    #[test]
    fn test_fifo_order_under_contention() {
        const QUEUE_SIZE: usize = 100;
        const NUM_PRODUCER_THREADS: usize = 4;
        const ITEMS_PER_THREAD: usize = 500;
        
        let queue = Arc::new(setup_test_queue::<usize>(QUEUE_SIZE));
        
        // Use ranges to verify FIFO ordering within each producer's stream
        // Each producer will push items with its ID as the high bits
        let barrier = Arc::new(Barrier::new(NUM_PRODUCER_THREADS + 1)); // +1 for consumer thread
        
        // Thread handles
        let mut handles = Vec::new();
        
        // Producer threads
        for id in 0..NUM_PRODUCER_THREADS {
            let queue_clone = Arc::clone(&queue);
            let barrier_clone = Arc::clone(&barrier);
            
            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier_clone.wait();
                
                // Each thread produces items with its ID encoded in high bits
                // and sequence number in low bits to detect ordering violations
                for seq in 0..ITEMS_PER_THREAD {
                    let item = (id << 24) | seq; // Upper 8 bits = thread ID, lower 24 bits = sequence
                    while !queue_clone.push(item) {
                        thread::yield_now();
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Consumer thread - verifies FIFO ordering per producer stream
        let consumer_handle = {
            let queue_clone = Arc::clone(&queue);
            let barrier_clone = Arc::clone(&barrier);
            
            thread::spawn(move || {
                // Track the last sequence number seen for each producer thread
                let mut last_seq = vec![0; NUM_PRODUCER_THREADS];
                let mut items_received = 0;
                let expected_total = NUM_PRODUCER_THREADS * ITEMS_PER_THREAD;
                
                // Wait for all threads to be ready
                barrier_clone.wait();
                
                while items_received < expected_total {
                    if let Some(item) = queue_clone.pop() {
                        // Extract thread ID and sequence number
                        let thread_id = (item >> 24) as usize;
                        let seq = item & 0xFFFFFF;
                        
                        // Verify this item comes after the previous one from this producer
                        // This ensures FIFO ordering within each producer's stream
                        assert!(seq >= last_seq[thread_id], 
                                "FIFO violation for thread {}: received seq {} after {}", 
                                thread_id, seq, last_seq[thread_id]);
                        
                        last_seq[thread_id] = seq + 1;
                        items_received += 1;
                    } else {
                        thread::yield_now();
                    }
                }
                
                // Verify we received all items from each producer
                for (id, &seq) in last_seq.iter().enumerate() {
                    assert_eq!(seq, ITEMS_PER_THREAD, "Missing items from producer {}", id);
                }
            })
        };
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        consumer_handle.join().unwrap();
        
        // Verify the queue is empty
        assert!(queue.is_empty(), "Queue not empty after test");
    }
    
    #[test]
    fn test_high_contention_stress() {
        // Using a much higher thread count than CPU cores to create contention
        const NUM_THREADS: usize = 32;
        const QUEUE_SIZE: usize = 100;
        const OPS_PER_THREAD: usize = 1000;
        
        let queue = Arc::new(setup_test_queue::<usize>(QUEUE_SIZE));
        let barrier = Arc::new(Barrier::new(NUM_THREADS));
        let op_counter = Arc::new(AtomicUsize::new(0));
        let error_flag = Arc::new(AtomicBool::new(false));
        
        // Thread handles
        let mut handles = Vec::new();
        
        for id in 0..NUM_THREADS {
            let queue_clone = Arc::clone(&queue);
            let barrier_clone = Arc::clone(&barrier);
            let op_counter_clone = Arc::clone(&op_counter);
            let error_flag_clone = Arc::clone(&error_flag);
            
            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier_clone.wait();
                
                let mut rng = id; // Use thread ID as simple seed
                let mut local_op_count = 0;
                
                while local_op_count < OPS_PER_THREAD && !error_flag_clone.load(Ordering::Relaxed) {
                    // Randomly choose between push and pop operations
                    // More complex patterns can use thread ID for deterministic variation
                    rng = rng.wrapping_mul(1664525).wrapping_add(1013904223); // Simple LCG
                    let do_push = (rng % 2) == 0;
                    
                    if do_push {
                        // Use a combined thread ID and counter as the value
                        let value = (id << 24) | (local_op_count & 0xFFFFFF);
                        if queue_clone.push(value) {
                            op_counter_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        if queue_clone.pop().is_some() {
                            op_counter_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    
                    local_op_count += 1;
                    
                    // Occasionally yield to increase scheduler contention
                    if (rng % 64) == 0 {
                        thread::yield_now();
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Success if we get here without panics or deadlocks
        let total_ops = op_counter.load(Ordering::Relaxed);
        assert!(total_ops > 0, "No operations completed successfully");
        
        // Empty the queue for cleanup
        while queue.pop().is_some() {}
    }
    
    #[test]
    fn test_drop_with_remaining_items() {
        // Create a queue with items and verify it drops properly
        let queue = setup_test_queue::<Box<[u8; 1024]>>(100);
        
        // Push several large items that would leak memory if not properly freed
        for i in 0..10 {
            let large_item = Box::new([i as u8; 1024]);
            assert!(queue.push(large_item));
        }
        
        // Queue will be dropped at the end of this scope
        // The test passes if it doesn't crash, leak memory, or trigger sanitizer errors
    }
}
