//! Data structures for the Mauka MCP Server.
//!
//! This module contains specialized data structures optimized for
//! high-performance concurrent operations in the Mauka MCP Server.
//! All implementations adhere to the strict project requirements:
//! - No unsafe code (except where absolutely necessary and thoroughly documented)
//! - Lock-free concurrency patterns
//! - Zero-copy operations where possible
//! - Cache-aware implementations

pub mod kahuna_queue;
pub mod niihau_trie;

// Re-export common data structures
pub use kahuna_queue::KahunaQueue;
pub use niihau_trie::{NiihauTrie, NiihauTrieError, NiihauTrieResult};
