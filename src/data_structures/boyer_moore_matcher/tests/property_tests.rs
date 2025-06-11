// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Property-based tests for Boyer-Moore Pattern Matcher.

use proptest::prelude::*;
use std::collections::HashSet;

use crate::data_structures::boyer_moore_matcher::{BoyerMooreMatcher, MatcherOptions};

// Strategy for generating valid pattern strings (non-empty, reasonable length)
fn pattern_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_\\-]{1,50}").unwrap()
}

// Strategy for generating text corpus
fn text_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_\\- ,.!?]{10,500}").unwrap()
}

// Strategy for generating options
fn options_strategy() -> impl Strategy<Value = MatcherOptions> {
    (prop::bool::ANY, prop::bool::ANY).prop_map(|(case_insensitive, allow_overlapping)| {
        MatcherOptions::new()
            .case_insensitive(case_insensitive)
            .allow_overlapping(allow_overlapping)
    })
}

proptest! {
    // Property: find_first should return Some(position) only if the pattern exists at that position
    #[test]
    fn prop_find_first_valid_position(pattern in pattern_strategy(), text in text_strategy()) {
        let matcher = BoyerMooreMatcher::new(&pattern);
        
        if let Some(pos) = matcher.find_first(&text) {
            // The pattern must exist at the found position
            let substring = &text[pos..std::cmp::min(text.len(), pos + pattern.len())];
            prop_assert_eq!(substring, pattern);
        }
    }
    
    // Property: find_first with case insensitive option should work regardless of case
    #[test]
    fn prop_find_first_case_insensitive(pattern in pattern_strategy(), text in text_strategy()) {
        let options = MatcherOptions::new().case_insensitive(true);
        let matcher = BoyerMooreMatcher::with_options(&pattern, options);
        
        // Convert pattern to uppercase to test case insensitivity
        let upper_pattern = pattern.to_uppercase();
        let mut modified_text = text.clone();
        
        // If the original pattern exists in the text, replace it with uppercase
        if let Some(pos) = text.find(&pattern) {
            modified_text.replace_range(pos..(pos + pattern.len()), &upper_pattern);
            
            // The matcher should find the uppercase version
            let result = matcher.find_first(&modified_text);
            prop_assert!(result.is_some());
            
            if let Some(found_pos) = result {
                let found = &modified_text[found_pos..(found_pos + upper_pattern.len())];
                prop_assert_eq!(found.to_lowercase(), pattern.to_lowercase());
            }
        }
    }
    
    // Property: find_all should find all occurrences of the pattern
    #[test]
    fn prop_find_all_finds_all_occurrences(
        pattern in pattern_strategy(),
        text_fragments in prop::collection::vec(text_strategy(), 1..10)
    ) {
        // Create text with known pattern positions by joining text fragments with the pattern
        let text = text_fragments.join(&pattern);
        
        // Calculate the expected positions
        let mut expected_positions = Vec::new();
        let mut pos = 0;
        for fragment in &text_fragments[0..text_fragments.len() - 1] {
            pos += fragment.len();
            expected_positions.push(pos);
            pos += pattern.len();
        }
        
        // Find all occurrences using the matcher
        let matcher = BoyerMooreMatcher::new(&pattern);
        let found_positions: Vec<usize> = matcher.find_all(&text).collect();
        
        // All expected positions should be found
        for expected_pos in &expected_positions {
            prop_assert!(found_positions.contains(expected_pos));
        }
        
        // There should be exactly as many matches as expected positions
        prop_assert_eq!(found_positions.len(), expected_positions.len());
    }
    
    // Property: find_all with overlapping should find overlapping patterns
    #[test]
    fn prop_find_all_overlapping(pattern in "[a-z]{2,5}".prop_map(String::from)) {
        if pattern.len() < 2 {
            return Ok(());
        }
        
        // Create an overlapping pattern (e.g., "abcabc" contains overlapping "abcab")
        let overlapping = format!("{}{}", pattern, &pattern[0..pattern.len() - 1]);
        
        // With overlapping enabled
        let options_overlap = MatcherOptions::new().allow_overlapping(true);
        let matcher_overlap = BoyerMooreMatcher::with_options(&pattern, options_overlap);
        let positions_overlap: Vec<usize> = matcher_overlap.find_all(&overlapping).collect();
        
        // Without overlapping
        let matcher_no_overlap = BoyerMooreMatcher::new(&pattern);
        let positions_no_overlap: Vec<usize> = matcher_no_overlap.find_all(&overlapping).collect();
        
        // With overlapping enabled, we should find more matches or the same number
        prop_assert!(positions_overlap.len() >= positions_no_overlap.len());
        
        // First match should be at position 0
        if !positions_overlap.is_empty() {
            prop_assert_eq!(positions_overlap[0], 0);
        }
    }
    
    // Property: correctness against standard library
    #[test]
    fn prop_matches_standard_library(
        pattern in pattern_strategy(),
        text in text_strategy(),
        options in options_strategy()
    ) {
        // Skip empty patterns as they're not supported
        if pattern.is_empty() {
            return Ok(());
        }
        
        let matcher = BoyerMooreMatcher::with_options(&pattern, options.clone());
        
        // Get all positions using our matcher
        let our_positions: HashSet<usize> = matcher.find_all(&text).collect();
        
        // Get all positions using standard library
        let mut std_positions = HashSet::new();
        let mut start_pos = 0;
        
        if options.case_insensitive {
            let text_lower = text.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            
            while let Some(pos) = text_lower[start_pos..].find(&pattern_lower) {
                let absolute_pos = start_pos + pos;
                std_positions.insert(absolute_pos);
                
                if options.allow_overlapping {
                    start_pos = absolute_pos + 1;
                } else {
                    start_pos = absolute_pos + pattern_lower.len();
                }
                
                if start_pos >= text_lower.len() {
                    break;
                }
            }
        } else {
            while let Some(pos) = text[start_pos..].find(&pattern) {
                let absolute_pos = start_pos + pos;
                std_positions.insert(absolute_pos);
                
                if options.allow_overlapping {
                    start_pos = absolute_pos + 1;
                } else {
                    start_pos = absolute_pos + pattern.len();
                }
                
                if start_pos >= text.len() {
                    break;
                }
            }
        }
        
        // Our results should match the standard library's results
        prop_assert_eq!(our_positions, std_positions);
    }
}
