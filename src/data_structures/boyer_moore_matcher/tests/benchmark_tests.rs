// Copyright (c) 2025 Mauka MCP Authors
//
// Licensed under dual license:
// - MIT License (LICENSE-MIT or https://opensource.org/licenses/MIT)
// - Apache License, Version 2.0 (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)

//! Benchmarking tests for the Boyer-Moore Pattern Matcher.

#[cfg(test)]
mod benches {
    use crate::data_structures::boyer_moore_matcher::{BoyerMooreMatcher, MatcherOptions};
    use criterion::{black_box, Criterion, criterion_group, criterion_main};
    
    const TEXT_SIZE_SMALL: usize = 1_000;
    const TEXT_SIZE_MEDIUM: usize = 10_000;
    const TEXT_SIZE_LARGE: usize = 100_000;
    
    // Generation of benchmark text with controlled pattern occurrences
    fn generate_benchmark_text(size: usize, pattern: &str, occurrences: usize) -> String {
        let pattern_len = pattern.len();
        let total_pattern_size = pattern_len * occurrences;
        let filler_size = if size > total_pattern_size { size - total_pattern_size } else { 0 };
        
        let mut text = String::with_capacity(size);
        let filler = "abcdefghijklmnopqrstuvwxyz0123456789";
        
        // Add patterns uniformly distributed in the text
        if occurrences > 0 && filler_size > 0 {
            let segment_size = filler_size / (occurrences + 1);
            let remainder = filler_size % (occurrences + 1);
            
            let mut remaining_filler = remainder;
            
            for _ in 0..occurrences {
                // Add filler segment
                let segment_with_extra = if remaining_filler > 0 {
                    remaining_filler -= 1;
                    segment_size + 1
                } else {
                    segment_size
                };
                
                for _ in 0..segment_with_extra {
                    text.push(filler.chars().nth(fastrand::usize(0..filler.len())).unwrap());
                }
                
                // Add pattern
                text.push_str(pattern);
            }
            
            // Add final filler segment
            for _ in 0..segment_size {
                text.push(filler.chars().nth(fastrand::usize(0..filler.len())).unwrap());
            }
        } else {
            // Edge case: no occurrences or no filler
            for _ in 0..filler_size {
                text.push(filler.chars().nth(fastrand::usize(0..filler.len())).unwrap());
            }
            
            for _ in 0..occurrences {
                text.push_str(pattern);
            }
        }
        
        text
    }
    
    // Benchmark finding a single occurrence
    pub fn bench_find_first(c: &mut Criterion) {
        let mut group = c.benchmark_group("BoyerMoore_FindFirst");
        
        // Short pattern
        let pattern_short = "needle";
        let text_short = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern_short, 1);
        
        group.bench_function("short_pattern", |b| {
            let matcher = BoyerMooreMatcher::new(pattern_short);
            b.iter(|| matcher.find_first(black_box(&text_short)))
        });
        
        // Medium pattern
        let pattern_medium = "medium_length_pattern_for_benchmark";
        let text_medium = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern_medium, 1);
        
        group.bench_function("medium_pattern", |b| {
            let matcher = BoyerMooreMatcher::new(pattern_medium);
            b.iter(|| matcher.find_first(black_box(&text_medium)))
        });
        
        // Long pattern
        let pattern_long = "this_is_a_long_pattern_to_test_boyer_moore_algorithm_performance_with_longer_patterns";
        let text_long = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern_long, 1);
        
        group.bench_function("long_pattern", |b| {
            let matcher = BoyerMooreMatcher::new(pattern_long);
            b.iter(|| matcher.find_first(black_box(&text_long)))
        });
        
        // Case insensitive
        group.bench_function("case_insensitive", |b| {
            let options = MatcherOptions::new().case_insensitive(true);
            let matcher = BoyerMooreMatcher::with_options(pattern_short, options);
            b.iter(|| matcher.find_first(black_box(&text_short)))
        });
        
        group.finish();
    }
    
    // Benchmark finding all occurrences
    pub fn bench_find_all(c: &mut Criterion) {
        let mut group = c.benchmark_group("BoyerMoore_FindAll");
        
        // Few occurrences
        let pattern = "pattern";
        let text_few = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern, 10);
        
        group.bench_function("few_occurrences", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_few)).collect();
                black_box(positions)
            })
        });
        
        // Many occurrences
        let text_many = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern, 100);
        
        group.bench_function("many_occurrences", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_many)).collect();
                black_box(positions)
            })
        });
        
        // Overlapping occurrences
        let overlapping_pattern = "aaa";
        let text_overlapping = generate_benchmark_text(TEXT_SIZE_MEDIUM, "aaaaa", 20);
        
        group.bench_function("overlapping", |b| {
            let options = MatcherOptions::new().allow_overlapping(true);
            let matcher = BoyerMooreMatcher::with_options(overlapping_pattern, options);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_overlapping)).collect();
                black_box(positions)
            })
        });
        
        group.finish();
    }
    
    // Benchmark with different text sizes
    pub fn bench_text_sizes(c: &mut Criterion) {
        let mut group = c.benchmark_group("BoyerMoore_TextSizes");
        
        let pattern = "benchmark";
        
        // Small text
        let text_small = generate_benchmark_text(TEXT_SIZE_SMALL, pattern, 5);
        
        group.bench_function("small_text", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_small)).collect();
                black_box(positions)
            })
        });
        
        // Medium text
        let text_medium = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern, 10);
        
        group.bench_function("medium_text", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_medium)).collect();
                black_box(positions)
            })
        });
        
        // Large text
        let text_large = generate_benchmark_text(TEXT_SIZE_LARGE, pattern, 20);
        
        group.bench_function("large_text", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text_large)).collect();
                black_box(positions)
            })
        });
        
        group.finish();
    }
    
    // Comparison benchmark with standard library
    pub fn bench_vs_standard(c: &mut Criterion) {
        let mut group = c.benchmark_group("BoyerMoore_VsStandard");
        
        let pattern = "comparison";
        let text = generate_benchmark_text(TEXT_SIZE_MEDIUM, pattern, 50);
        
        // Boyer-Moore find_all
        group.bench_function("boyer_moore_find_all", |b| {
            let matcher = BoyerMooreMatcher::new(pattern);
            b.iter(|| {
                let positions: Vec<usize> = matcher.find_all(black_box(&text)).collect();
                black_box(positions)
            })
        });
        
        // Standard library find
        group.bench_function("std_find_all", |b| {
            b.iter(|| {
                let mut positions = Vec::new();
                let mut start = 0;
                
                while let Some(pos) = black_box(&text[start..]).find(black_box(pattern)) {
                    let absolute_pos = start + pos;
                    positions.push(absolute_pos);
                    start = absolute_pos + pattern.len();
                    
                    if start >= text.len() {
                        break;
                    }
                }
                
                black_box(positions)
            })
        });
        
        group.finish();
    }

    criterion_group!(
        benches,
        bench_find_first,
        bench_find_all,
        bench_text_sizes,
        bench_vs_standard
    );
    criterion_main!(benches);
}
