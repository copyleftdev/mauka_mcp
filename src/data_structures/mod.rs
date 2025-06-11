//! Data structures for the Mauka MCP Server.
//!
//! This module contains specialized data structures optimized for
//! high-performance concurrent operations in the Mauka MCP Server.
//! All implementations adhere to the strict project requirements:
//! - No unsafe code (except where absolutely necessary and thoroughly documented)
//! - Lock-free concurrency patterns
//! - Zero-copy operations where possible
//! - Cache-aware implementations

pub mod boyer_moore_matcher;
pub mod kahuna_queue;
pub mod kona_bloom_filter;
pub mod niihau_trie;
pub mod puka_cuckoo_hash;

// Re-export common data structures
pub use boyer_moore_matcher::{BoyerMooreMatcher, BoyerMooreError, MatcherOptions};
pub use kahuna_queue::KahunaQueue;
pub use kona_bloom_filter::{KonaBloomFilter, KonaBloomFilterConfig, KonaBloomFilterError};
pub use niihau_trie::{NiihauTrie, NiihauTrieError, NiihauTrieResult};
pub use puka_cuckoo_hash::{PukaCuckooHash, PukaCuckooHashConfig, PukaCuckooHashError};
