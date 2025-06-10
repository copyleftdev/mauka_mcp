//! Node implementation for the Niihau Header Trie.
//!
//! This module provides the TrieNode structure used in the Niihau Trie implementation.
//! Nodes are the fundamental building blocks of the trie, each containing
//! values and references to child nodes.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A node in the Niihau Header Trie.
///
/// Each node represents a character in a key path. Terminal nodes contain values
/// associated with complete keys.
#[derive(Debug)]
pub struct TrieNode {
    /// Map of characters to child nodes
    pub children: HashMap<char, Arc<RwLock<TrieNode>>>,
    
    /// Whether this node represents the end of a key
    pub is_terminal: bool,
    
    /// Values associated with this key (if it's a terminal node)
    pub values: Vec<String>,
}

impl TrieNode {
    /// Creates a new empty trie node.
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_terminal: false,
            values: Vec::new(),
        }
    }
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::new()
    }
}
