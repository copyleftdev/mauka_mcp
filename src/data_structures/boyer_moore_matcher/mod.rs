// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Boyer-Moore Pattern Matcher for efficient string searching.
//!
//! This module provides an optimized implementation of the Boyer-Moore string search
//! algorithm, which is particularly efficient for searching longer patterns in large
//! text corpuses. The implementation includes both the bad character rule and the good
//! suffix rule for maximum performance.
//!
//! # Features
//!
//! - Efficient string search with sublinear performance on average
//! - Support for case-sensitive and case-insensitive matching
//! - Works with UTF-8 strings while maintaining correct character boundaries
//! - Multiple match support with iterator interface
//! - Zero allocation in the search hot path
//!
//! # Example
//!
//! ```
//! use mauka_mcp::data_structures::boyer_moore_matcher::{BoyerMooreMatcher, MatcherOptions};
//!
//! // Create a matcher with default options
//! let pattern = "needle";
//! let matcher = BoyerMooreMatcher::new(pattern);
//!
//! // Search for the pattern in a text
//! let text = "Finding a needle in a haystack is hard, but finding another needle is easier.";
//! let matches = matcher.find_all(text).collect::<Vec<_>>();
//!
//! // Should find two matches
//! assert_eq!(matches, vec![10, 47]);
//!
//! // Create a case-insensitive matcher
//! let options = MatcherOptions::new().case_insensitive(true);
//! let matcher = BoyerMooreMatcher::with_options("NEEDLE", options);
//!
//! // Will find both "needle" and "NEEDLE" regardless of case
//! assert!(matcher.find_first("There is a needle here.").is_some());
//! ```
//!
//! # Performance Characteristics
//!
//! The Boyer-Moore algorithm has the following performance characteristics:
//!
//! - Preprocessing time: O(m + σ) where m is the pattern length and σ is the alphabet size
//! - Space complexity: O(m + σ)
//! - Best case: O(n/m) comparisons (where n is the text length)
//! - Worst case: O(n*m) comparisons, but this is rare in practice
//! - Average case: O(n)
//!
//! This implementation optimizes for the average case while ensuring the worst
//! case does not become pathological.

mod error;
mod matcher;
mod preprocess;
mod tables;

// Re-exports
pub use error::{BoyerMooreError, Result};
pub use matcher::{BoyerMooreMatcher, MatcherOptions, Match, MatchIterator};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_matching() {
        let matcher = BoyerMooreMatcher::new("pattern");
        
        // Test with exact match
        let text = "Here is a pattern to find.";
        let result = matcher.find_first(text);
        assert_eq!(result, Some(10));
        
        // Test with no match
        let text = "This text does not contain what we're looking for.";
        let result = matcher.find_first(text);
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_find_all() {
        let matcher = BoyerMooreMatcher::new("test");
        
        let text = "This is a test. Another test. Final test.";
        let matches: Vec<usize> = matcher.find_all(text).collect();
        
        assert_eq!(matches, vec![10, 23, 35]);
    }
    
    #[test]
    fn test_case_insensitive() {
        let options = MatcherOptions::new().case_insensitive(true);
        let matcher = BoyerMooreMatcher::with_options("CASE", &options);
        
        let text = "Testing Case insensitive case matching CASE";
        let matches: Vec<usize> = matcher.find_all(text).collect();
        
        assert_eq!(matches, vec![8, 28, 42]);
    }
    
    #[test]
    fn test_overlapping_matches() {
        let options = MatcherOptions::new().allow_overlapping(true);
        let matcher = BoyerMooreMatcher::with_options("aaa", &options);
        
        let text = "aaaaa";  // Should match at positions 0, 1, 2
        let matches: Vec<usize> = matcher.find_all(text).collect();
        
        assert_eq!(matches, vec![0, 1, 2]);
    }
    
    #[test]
    fn test_utf8_handling() {
        // Test with Unicode characters
        let matcher = BoyerMooreMatcher::new("café");
        
        let text = "Welcome to the café!";
        let result = matcher.find_first(text);
        assert_eq!(result, Some(15));
        
        // Ensure proper boundary handling
        let matcher = BoyerMooreMatcher::new("café");
        let text = "café";
        let result = matcher.find_first(text);
        assert_eq!(result, Some(0));
    }
}
