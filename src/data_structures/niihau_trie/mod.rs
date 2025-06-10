//! Niihau Header Trie Implementation
//!
//! This module provides an efficient trie-based data structure for storing
//! and retrieving header key-value pairs with fast prefix lookups.
//! Optimized for MCP protocol handling use cases.

mod node;
mod error;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub use error::NiihauTrieError;
use node::TrieNode;

/// Result type for Niihau Trie operations
pub type NiihauTrieResult<T> = Result<T, NiihauTrieError>;

/// Configuration options for the Niihau Header Trie
#[derive(Debug, Clone)]
pub struct NiihauTrieConfig {
    /// Whether to use case-sensitive keys
    pub case_sensitive: bool,
    
    /// Maximum depth allowed in the trie (prevents stack overflows)
    pub max_depth: usize,
    
    /// Whether to allow duplicate keys with different values
    pub allow_duplicates: bool,
}

impl Default for NiihauTrieConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            max_depth: 64,
            allow_duplicates: false,
        }
    }
}

/// Niihau Header Trie is an efficient trie-based data structure for storing 
/// and retrieving header key-value pairs with fast prefix lookups.
///
/// Key features:
/// * Case-insensitive key lookup (configurable)
/// * Fast prefix matching for HTTP header-like use cases
/// * Thread-safe with fine-grained locking
/// * Memory efficient representation for shared prefixes
#[derive(Debug)]
pub struct NiihauTrie {
    /// The root node of the trie
    root: Arc<RwLock<TrieNode>>,
    
    /// Configuration options
    config: NiihauTrieConfig,
}

impl NiihauTrie {
    /// Creates a new empty `NiihauTrie` with default configuration.
    ///
    /// # Returns
    ///
    /// A new `NiihauTrie` instance.
    pub fn new() -> Self {
        Self::with_config(NiihauTrieConfig::default())
    }

    /// Creates a new empty `NiihauTrie` with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the trie.
    ///
    /// # Returns
    ///
    /// A new `NiihauTrie` instance.
    pub fn with_config(config: NiihauTrieConfig) -> Self {
        Self {
            root: Arc::new(RwLock::new(TrieNode::new())),
            config,
        }
    }

    /// Inserts a key-value pair into the trie.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert.
    /// * `value` - The value to associate with the key.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - `true` if a new key was inserted, `false` if the key was updated.
    /// * `Err(NiihauTrieError)` - If an error occurred during insertion.
    pub fn insert<K, V>(&self, key: K, value: V) -> NiihauTrieResult<bool>
    where
        K: AsRef<str>,
        V: Into<String>,
    {
        let key = key.as_ref();
        if key.is_empty() {
            return Err(NiihauTrieError::EmptyKey);
        }

        let processed_key = if self.config.case_sensitive {
            Cow::Borrowed(key)
        } else {
            Cow::Owned(key.to_lowercase())
        };
        
        let value = value.into();
        
        // Check depth limit before insertion
        if processed_key.len() > self.config.max_depth {
            return Err(NiihauTrieError::KeyTooLong {
                key: processed_key.into_owned(),
                max_depth: self.config.max_depth,
            });
        }

        let chars: Vec<char> = processed_key.chars().collect();
        
        // Traverse the trie, creating nodes as needed
        let mut node = self.root.clone();
        
        // Track nodes we've traversed to prevent re-locking
        let mut traversed_nodes = Vec::with_capacity(chars.len());
        
        for i in 0..chars.len() {
            let c = chars[i];
            traversed_nodes.push(node.clone());
            
            // Use a scope to limit how long we hold the lock
            let next_node = {
                let mut current = match node.write() {
                    Ok(guard) => guard,
                    Err(_) => return Err(NiihauTrieError::LockError),
                };
                
                // Check if node already has this child to avoid unnecessary allocations
                let next = current.children.entry(c).or_insert_with(|| {
                    Arc::new(RwLock::new(TrieNode::new()))
                });
                next.clone()
            };
            
            // Move to next node
            node = next_node;
        }
        
        // Now at the final node, set values
        let mut current = match node.write() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        
        let is_new = !current.is_terminal;
        
        if is_new {
            // This is a new key
            current.is_terminal = true;
            current.values.push(value);
        } else if self.config.allow_duplicates {
            // Key exists but we allow duplicates
            // Check if value already exists to avoid duplicates
            if !current.values.contains(&value) {
                current.values.push(value);
            }
        } else {
            // Key exists and we don't allow duplicates, so replace the value
            current.values.clear();
            current.values.push(value);
        }
        
        // Release the lock by dropping current explicitly
        drop(current);
        
        Ok(is_new)
    }

    /// Retrieves values associated with a key from the trie.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - The values associated with the key, or empty if the key was not found.
    /// * `Err(NiihauTrieError)` - If an error occurred during retrieval.
    pub fn get<K>(&self, key: K) -> NiihauTrieResult<Vec<String>>
    where
        K: AsRef<str>,
    {
        let key = key.as_ref();
        if key.is_empty() {
            return Err(NiihauTrieError::EmptyKey);
        }

        let processed_key = if self.config.case_sensitive {
            Cow::Borrowed(key)
        } else {
            Cow::Owned(key.to_lowercase())
        };

        let chars: Vec<char> = processed_key.chars().collect();
        let mut node = self.root.clone();
        
        // We'll acquire read locks one at a time to minimize contention
        // and avoid deadlocks by ensuring consistent lock acquisition order
        for &c in chars.iter() {
            // Acquire a read lock on the current node
            let current = match node.read() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            
            // Check if the character exists in this node's children
            match current.children.get(&c) {
                Some(next) => {
                    // Clone the Arc to the next node before releasing this lock
                    let next_node = next.clone();
                    
                    // Release the read lock by dropping the guard
                    drop(current);
                    
                    // Move to the next node
                    node = next_node;
                },
                None => {
                    // Character not found, key doesn't exist
                    return Ok(Vec::new());
                }
            }
        }

        // Reached the end of the key path, acquire final read lock
        let current = match node.read() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        
        // Check if this is a terminal node (key exists)
        let result = if current.is_terminal {
            current.values.clone()
        } else {
            Vec::new()
        };
        
        // Release the lock explicitly
        drop(current);
        
        Ok(result)
    }

    /// Checks if a key exists in the trie.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - `true` if the key exists, `false` otherwise.
    /// * `Err(NiihauTrieError)` - If an error occurred during check.
    pub fn contains<K>(&self, key: K) -> NiihauTrieResult<bool>
    where
        K: AsRef<str>,
    {
        let values = self.get(key)?;
        Ok(!values.is_empty())
    }

    /// Removes a key from the trie with proper concurrency safety.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - `true` if the key was removed, `false` if it wasn't found.
    /// * `Err(NiihauTrieError)` - If an error occurred during removal.
    pub fn remove<K>(&self, key: K) -> NiihauTrieResult<bool>
    where
        K: AsRef<str>,
    {
        let key = key.as_ref();
        if key.is_empty() {
            return Err(NiihauTrieError::EmptyKey);
        }

        let processed_key = if self.config.case_sensitive {
            Cow::Borrowed(key)
        } else {
            Cow::Owned(key.to_lowercase())
        };

        let chars: Vec<char> = processed_key.chars().collect();
        
        // Use a retry mechanism in case of lock contention
        const MAX_RETRIES: usize = 3;
        let mut retry_count = 0;
        
        while retry_count < MAX_RETRIES {
            match self.remove_recursive(&chars, 0, self.root.clone()) {
                Ok(result) => return Ok(result),
                Err(NiihauTrieError::LockError) if retry_count < MAX_RETRIES - 1 => {
                    // Brief backoff before retry to reduce contention
                    std::thread::yield_now();
                    retry_count += 1;
                    continue;
                },
                Err(e) => return Err(e),
            }
        }
        
        // If we've exhausted retries, report lock error
        Err(NiihauTrieError::LockError)
    }

    /// Helper function for recursive removal of nodes with proper concurrency safety.
    ///
    /// This method implements a depth-first traversal to remove the specified key.
    /// It uses fine-grained locking to minimize contention during concurrent removal operations.
    fn remove_recursive(&self, chars: &[char], depth: usize, node: Arc<RwLock<TrieNode>>) -> NiihauTrieResult<bool> {
        // Check if we've reached maximum recursion depth
        if depth > self.config.max_depth {
            return Err(NiihauTrieError::KeyTooLong {
                key: chars.iter().collect::<String>(),
                max_depth: self.config.max_depth,
            });
        }

        if depth == chars.len() {
            // We've reached the end of the key, remove values if this is a terminal node
            let mut current = match node.write() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            
            // Check if this is a terminal node
            if current.is_terminal {
                current.is_terminal = false;
                current.values.clear();
                drop(current); // Explicitly release lock
                return Ok(true);
            } else {
                drop(current); // Explicitly release lock
                return Ok(false);
            }
        }
        
        // Get the current character
        let c = chars[depth];
        
        // Get the child node for this character
        let child_node_opt = {
            let current = match node.read() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            
            // Clone the child node if it exists
            let result = current.children.get(&c).cloned();
            drop(current); // Explicitly release lock
            result
        };
        
        let child_node = match child_node_opt {
            Some(child) => child,
            None => return Ok(false), // Character not found, key doesn't exist
        };
        
        // Recursively remove from child node
        let removed = self.remove_recursive(chars, depth + 1, child_node.clone())?;
        
        // If we removed something and the child node is now empty, we can remove it
        // from the parent to save space
        if removed {
            let child_empty = {
                let child = match child_node.read() {
                    Ok(guard) => guard,
                    Err(_) => return Err(NiihauTrieError::LockError),
                };
                
                let is_empty = !child.is_terminal && child.children.is_empty();
                drop(child); // Explicitly release lock
                is_empty
            };
            
            if child_empty {
                let mut current = match node.write() {
                    Ok(guard) => guard,
                    Err(_) => return Err(NiihauTrieError::LockError),
                };
                
                current.children.remove(&c);
                // Lock is released when current goes out of scope
            }
        }
        
        Ok(removed)
    }

    /// Finds all keys and values with a given prefix with proper concurrency safety.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<String>>)` - A map of keys to their values for all keys with the given prefix.
    /// * `Err(NiihauTrieError)` - If an error occurred during search.
    pub fn find_by_prefix<P>(&self, prefix: P) -> NiihauTrieResult<HashMap<String, Vec<String>>>
    where
        P: AsRef<str>,
    {
        let prefix = prefix.as_ref();

        let processed_prefix = if self.config.case_sensitive {
            Cow::Borrowed(prefix)
        } else {
            Cow::Owned(prefix.to_lowercase())
        };

        let mut result = HashMap::new();
                
        // Handle empty prefix - collect all keys
        if processed_prefix.is_empty() {
            let root_node = match self.root.read() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            self.collect_keys_with_prefix(&root_node, "", String::new(), &mut result)?;
            return Ok(result);
        }

        // Traverse to the prefix node
        let mut node = self.root.clone();
        for c in processed_prefix.chars() {
            let next_opt = {
                let current = match node.read() {
                    Ok(guard) => guard,
                    Err(_) => return Err(NiihauTrieError::LockError),
                };
                let next = current.children.get(&c).cloned();
                drop(current); // Explicitly release lock
                next
            };
            
            match next_opt {
                Some(next) => node = next,
                None => return Ok(HashMap::new()), // Prefix not found
            }
        }

        // Collect all keys from this prefix node
        let node_ref = match node.read() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        
        self.collect_keys_with_prefix(&node_ref, &processed_prefix, String::new(), &mut result)?;
        drop(node_ref); // Explicitly release lock
        
        Ok(result)
    }
    
    /// Helper function to recursively collect all keys from a node with proper concurrency safety.
    fn collect_keys_with_prefix(
        &self,
        node: &TrieNode,
        prefix: &str,
        current_suffix: String,
        result: &mut HashMap<String, Vec<String>>,
    ) -> NiihauTrieResult<()> {
        // If this is a terminal node, add it to the results
        if node.is_terminal {
            let full_key = format!("{}{}", prefix, current_suffix);
            result.insert(full_key, node.values.clone());
        }
        
        // Recurse for all children
        for (c, child) in &node.children {
            // Use a block to ensure lock is released after cloning values
            let child_node = match child.read() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            
            let mut new_suffix = current_suffix.clone();
            new_suffix.push(*c);
            
            self.collect_keys_with_prefix(&child_node, prefix, new_suffix, result)?;
        }
        
        Ok(())
    }
    
    /// Returns the number of keys in the trie with proper concurrency safety.
    ///
    /// This requires traversing the entire trie, so it's an O(n) operation.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of keys in the trie.
    /// * `Err(NiihauTrieError)` - If an error occurred during counting.
    pub fn len(&self) -> NiihauTrieResult<usize> {
        let root = match self.root.read() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        let count = self.count_keys(&root)?;
        // Explicitly release lock
        drop(root);
        Ok(count)
    }
    
    /// Helper function to count all terminal nodes (keys) in the trie with proper concurrency safety.
    fn count_keys(&self, node: &TrieNode) -> NiihauTrieResult<usize> {
        let mut count = if node.is_terminal { 1 } else { 0 };
        
        for child in node.children.values() {
            let child_node = match child.read() {
                Ok(guard) => guard,
                Err(_) => return Err(NiihauTrieError::LockError),
            };
            
            count += self.count_keys(&child_node)?;
            // Lock is automatically released when child_node goes out of scope
        }
        
        Ok(count)
    }
    /// Checks if the trie is empty with proper concurrency safety.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - `true` if the trie is empty, `false` otherwise.
    /// * `Err(NiihauTrieError)` - If an error occurred during check.
    pub fn is_empty(&self) -> NiihauTrieResult<bool> {
        let root = match self.root.read() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        
        let result = root.children.is_empty() && !root.is_terminal;
        // Explicitly release lock
        drop(root);
        
        Ok(result)
    }

    /// Clears all entries from the trie with proper concurrency safety.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the trie was successfully cleared.
    /// * `Err(NiihauTrieError)` - If an error occurred during clearing.
    pub fn clear(&self) -> NiihauTrieResult<()> {
        let mut root = match self.root.write() {
            Ok(guard) => guard,
            Err(_) => return Err(NiihauTrieError::LockError),
        };
        
        *root = TrieNode::new();
        // Lock is automatically released when root goes out of scope
        
        Ok(())
    }
}

impl Default for NiihauTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_trie_basic_operations() {
        let trie = NiihauTrie::new();
        
        // Test initial state
        assert!(trie.is_empty().unwrap());
        
        // Test insertion
        assert!(trie.insert("hello", "world").unwrap());
        assert_eq!(trie.len().unwrap(), 1);
        assert!(!trie.is_empty().unwrap());
        
        // Test retrieval
        assert_eq!(trie.get("hello").unwrap(), vec!["world".to_string()]);
        assert!(trie.contains("hello").unwrap());
        assert!(trie.get("nonexistent").unwrap().is_empty());
        assert!(!trie.contains("nonexistent").unwrap());
        
        // Test case-insensitivity
        assert_eq!(trie.get("HELLO").unwrap(), vec!["world".to_string()]);
        
        // Test update
        assert!(!trie.insert("hello", "planet").unwrap());
        assert_eq!(trie.get("hello").unwrap(), vec!["planet".to_string()]);
        
        // Test removal
        assert!(trie.remove("hello").unwrap());
        assert!(trie.is_empty().unwrap());
        assert!(!trie.remove("hello").unwrap());
    }

    #[test]
    fn test_trie_prefix_search() {
        let trie = NiihauTrie::new();
        
        // Insert some keys with common prefixes
        trie.insert("apple", "fruit").unwrap();
        trie.insert("application", "software").unwrap();
        trie.insert("apply", "verb").unwrap();
        trie.insert("banana", "yellow").unwrap();
        
        // Test prefix search
        let results = trie.find_by_prefix("app").unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.contains_key("apple"));
        assert!(results.contains_key("application"));
        assert!(results.contains_key("apply"));
        
        // Test with no matches
        let results = trie.find_by_prefix("orange").unwrap();
        assert!(results.is_empty());
    }

    /// Tests concurrent operations on the trie with multiple threads inserting,
    /// retrieving, and removing values simultaneously to verify thread safety.
    #[test]
    fn test_trie_concurrency() {
        // Test configuration
        const THREAD_COUNT: usize = 8;
        const OPS_PER_THREAD: usize = 50;
        const TOTAL_KEYS: usize = THREAD_COUNT * OPS_PER_THREAD;
        
        // Create shared data structures with proper synchronization
        let trie = Arc::new(NiihauTrie::new());
        let start_barrier = Arc::new(std::sync::Barrier::new(THREAD_COUNT + 1)); // +1 for main thread
        let completion_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        
        // Track all keys and values for verification
        let all_keys = Arc::new(std::sync::Mutex::new(Vec::with_capacity(TOTAL_KEYS)));
        
        // Spawn worker threads
        let mut handles = Vec::with_capacity(THREAD_COUNT);
        
        for thread_id in 0..THREAD_COUNT {
            let trie_ref = Arc::clone(&trie);
            let barrier = Arc::clone(&start_barrier);
            let counter = Arc::clone(&completion_counter);
            let keys_tracker = Arc::clone(&all_keys);
            
            let handle = thread::spawn(move || -> Result<usize, String> {
                // Wait for all threads to be ready before starting
                barrier.wait();
                
                let mut thread_keys = Vec::with_capacity(OPS_PER_THREAD);
                let mut successful_ops = 0;
                
                // Phase 1: Insert operations
                for j in 0..OPS_PER_THREAD {
                    let key = format!("key_{}_{}", thread_id, j);
                    let value = format!("value_{}_{}", thread_id, j);
                    
                    // Track keys for verification
                    thread_keys.push(key.clone());
                    
                    match trie_ref.insert(&key, &value) {
                        Ok(_) => {
                            // Immediately verify insertion
                            match trie_ref.get(&key) {
                                Ok(values) => {
                                    if !values.contains(&value) {
                                        return Err(format!(
                                            "Thread {} failed to verify key {}: expected value not found", 
                                            thread_id, key
                                        ));
                                    }
                                    successful_ops += 1;
                                },
                                Err(e) => return Err(format!(
                                    "Thread {} get operation failed for key {}: {}", 
                                    thread_id, key, e
                                )),
                            }
                        },
                        Err(e) => return Err(format!(
                            "Thread {} insert operation failed for key {}: {}", 
                            thread_id, key, e
                        )),
                    }
                }
                
                // Add thread keys to global key tracker for verification
                if let Ok(mut keys) = keys_tracker.lock() {
                    keys.extend(thread_keys);
                } else {
                    return Err(format!("Thread {} couldn't acquire keys mutex", thread_id));
                }
                
                // Update completion counter
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                
                Ok(successful_ops)
            });
            
            handles.push(handle);
        }
        
        // Wait at barrier to synchronize start of all worker threads
        start_barrier.wait();
        
        // Wait for all threads to complete with proper error handling
        let mut _total_ops = 0;
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.join() {
                Ok(Ok(ops)) => { _total_ops += ops; },
                Ok(Err(e)) => panic!("Thread {} reported error: {}", i, e),
                Err(e) => panic!("Thread {} panicked: {:?}", i, e),
            }
        }
        
        // Verify all threads completed successfully
        assert_eq!(
            completion_counter.load(std::sync::atomic::Ordering::SeqCst), 
            THREAD_COUNT,
            "Not all threads completed successfully"
        );
        
        // Verify the final state of the trie
        let trie_size = match trie.len() {
            Ok(size) => size,
            Err(e) => panic!("Failed to get trie size: {}", e),
        };
        
        assert_eq!(
            trie_size, 
            TOTAL_KEYS,
            "Expected {} keys in trie, but found {}", 
            TOTAL_KEYS, trie_size
        );
        
        // Verify every key is retrievable and contains the expected value
        let all_inserted_keys = match all_keys.lock() {
            Ok(keys) => keys.clone(),
            Err(e) => panic!("Failed to acquire keys mutex: {}", e),
        };
        
        for key in all_inserted_keys.iter() {
            // Extract the thread ID and operation index from the key
            let parts: Vec<&str> = key.split('_').collect();
            if parts.len() != 3 {
                panic!("Invalid key format: {}", key);
            }
            
            let expected_value = format!("value_{}_{}" ,parts[1], parts[2]);
            
            match trie.get(key) {
                Ok(values) => {
                    assert!(
                        values.contains(&expected_value),
                        "Key '{}' doesn't contain expected value '{}', found: {:?}",
                        key, expected_value, values
                    );
                },
                Err(e) => panic!("Failed to get key '{}': {}", key, e),
            }
        }
        
        // Additional concurrency test: remove half the keys while reading the other half
        let remove_keys = all_inserted_keys.iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, k)| k.clone())
            .collect::<Vec<String>>();
        
        let read_keys = all_inserted_keys.iter()
            .enumerate()
            .filter(|(i, _)| i % 2 != 0)
            .map(|(_, k)| k.clone())
            .collect::<Vec<String>>();
        
        // Create threads for concurrent reads and removes
        let trie_for_remove = Arc::clone(&trie);
        let trie_for_read = Arc::clone(&trie);
        
        let remove_thread = thread::spawn(move || {
            for key in remove_keys {
                match trie_for_remove.remove(&key) {
                    Ok(true) => {}, // Successfully removed
                    Ok(false) => panic!("Failed to remove key: {}", key),
                    Err(e) => panic!("Error removing key {}: {}", key, e),
                }
            }
        });
        
        let read_thread = thread::spawn(move || {
            for key in read_keys {
                let _ = trie_for_read.get(&key); // Ignore errors, just testing concurrency safety
            }
        });
        
        // Wait for both operations to complete
        remove_thread.join().unwrap();
        read_thread.join().unwrap();
        
        // Final verification: trie should have half the original keys
        assert_eq!(trie.len().unwrap(), TOTAL_KEYS / 2);
    }
}
