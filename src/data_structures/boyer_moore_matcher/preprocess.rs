// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Pattern preprocessing for the Boyer-Moore algorithm.
//!
//! This module contains functions for preprocessing the pattern before searching,
//! which is a key part of the Boyer-Moore algorithm's efficiency. The preprocessing
//! step analyzes the pattern to generate lookup tables that enable fast skipping
//! during the search phase.

use super::error::{BoyerMooreError, Result};
use super::tables::{BadCharTable, GoodSuffixTable};

/// Maximum allowed pattern length to prevent excessive memory usage
const MAX_PATTERN_LENGTH: usize = 1024 * 32;  // 32KiB

/// Result of the preprocessing step containing all necessary lookup tables.
#[derive(Debug)]
pub struct PreprocessedPattern {
    /// The pattern being searched for
    pub pattern: String,
    
    /// Lowercase version of the pattern (for case-insensitive search)
    pub pattern_lower: Option<String>,
    
    /// Bad character rule table
    pub bad_char_table: BadCharTable,
    
    /// Good suffix rule table
    pub good_suffix_table: GoodSuffixTable,
    
    /// Whether the search is case-insensitive
    pub case_insensitive: bool,
    
    /// The length of the pattern in characters
    pub pattern_len: usize,
}

impl PreprocessedPattern {
    /// Preprocesses a pattern for use in the Boyer-Moore algorithm.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to preprocess.
    /// * `case_insensitive` - Whether to perform case-insensitive matching.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `PreprocessedPattern` or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern is empty or exceeds the maximum allowed length.
    pub fn new(pattern: &str, case_insensitive: bool) -> Result<Self> {
        if pattern.is_empty() {
            return Err(BoyerMooreError::EmptyPattern);
        }
        
        let pattern_len = pattern.chars().count();
        if pattern_len > MAX_PATTERN_LENGTH {
            return Err(BoyerMooreError::PatternTooLarge);
        }
        
        // Create lowercase version for case-insensitive matching
        let pattern_lower = if case_insensitive {
            Some(pattern.to_lowercase())
        } else {
            None
        };
        
        // Create the lookup tables
        let bad_char_table = BadCharTable::new(pattern, case_insensitive);
        let good_suffix_table = GoodSuffixTable::new(pattern);
        
        Ok(Self {
            pattern: pattern.to_string(),
            pattern_lower,
            bad_char_table,
            good_suffix_table,
            case_insensitive,
            pattern_len,
        })
    }
    
    /// Gets the effective pattern to match against.
    ///
    /// When case-insensitive matching is enabled, returns the lowercase pattern.
    /// Otherwise, returns the original pattern.
    pub fn effective_pattern(&self) -> &str {
        if self.case_insensitive {
            self.pattern_lower.as_ref().unwrap_or(&self.pattern)
        } else {
            &self.pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_preprocess_valid_pattern() {
        let result = PreprocessedPattern::new("PATTERN", false);
        assert!(result.is_ok());
        
        let processed = result.unwrap();
        assert_eq!(processed.pattern, "PATTERN");
        assert_eq!(processed.pattern_len, 7);
        assert_eq!(processed.case_insensitive, false);
        assert!(processed.pattern_lower.is_none());
    }
    
    #[test]
    fn test_preprocess_case_insensitive() {
        let result = PreprocessedPattern::new("Pattern", true);
        assert!(result.is_ok());
        
        let processed = result.unwrap();
        assert_eq!(processed.pattern, "Pattern");
        assert_eq!(processed.case_insensitive, true);
        assert_eq!(processed.pattern_lower, Some("pattern".to_string()));
        assert_eq!(processed.effective_pattern(), "pattern");
    }
    
    #[test]
    fn test_preprocess_empty_pattern() {
        let result = PreprocessedPattern::new("", false);
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert_eq!(err, BoyerMooreError::EmptyPattern);
        }
    }
    
    #[test]
    fn test_preprocess_pattern_too_large() {
        // Create a pattern that exceeds the maximum allowed length
        let large_pattern = "a".repeat(MAX_PATTERN_LENGTH + 1);
        let result = PreprocessedPattern::new(&large_pattern, false);
        assert!(result.is_err());
        
        if let Err(err) = result {
            assert_eq!(err, BoyerMooreError::PatternTooLarge);
        }
    }
}
