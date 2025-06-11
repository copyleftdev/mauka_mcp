//! Mauka MCP Server Benchmarks
//!
//! This module contains benchmarks for various components of the Mauka MCP Server.
//! The benchmarks are implemented using the Criterion framework, which provides
//! statistical analysis and performance regression detection.
//!
//! To run the benchmarks:
//! ```bash
//! cargo bench --features benchmarking
//! ```

use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkId, Criterion,
    SamplingMode, Throughput,
};
use std::time::Duration;

/// Benchmark the Kahuna Lock-Free Queue
fn bench_kahuna_queue(c: &mut Criterion) {
    use mauka_mcp_lib::data_structures::kahuna_queue::{KahunaQueue, KahunaQueueConfig};
    
    let mut group = c.benchmark_group("kahuna_queue");
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(2));
    group.warm_up_time(Duration::from_secs(1));
    group.sample_size(100);

    // Sequential push performance with different queue sizes
    for size in [100, 1000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential_push", size), 
            size, 
            |b, &size| {
                b.iter(|| {
                    let queue = KahunaQueue::<usize>::with_config(KahunaQueueConfig {
                        max_capacity: size * 2,  // Ensure we don't hit capacity
                        ..Default::default()
                    });
                    
                    for i in 0..size {
                        queue.push(black_box(i));
                    }
                });
            }
        );
    }

    // Sequential pop performance with different queue sizes
    for size in [100, 1000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential_pop", size), 
            size, 
            |b, &size| {
                b.iter_batched(
                    || {
                        let queue = KahunaQueue::<usize>::with_config(KahunaQueueConfig {
                            max_capacity: size * 2,
                            ..Default::default()
                        });
                        for i in 0..size {
                            queue.push(i);
                        }
                        queue
                    },
                    |queue| {
                        for _ in 0..size {
                            black_box(queue.pop());
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            }
        );
    }

    // Mixed operations benchmark
    group.bench_function("mixed_operations", |b| {
        b.iter(|| {
            let queue = KahunaQueue::<usize>::new();
            
            // Alternate push and pop to test interleaved operations
            for i in 0..1000 {
                queue.push(black_box(i));
                
                if i % 2 == 0 {
                    black_box(queue.pop());
                }
            }
            
            // Pop remaining items
            while let Some(item) = queue.pop() {
                black_box(item);
            }
        });
    });

    group.finish();
}

/// Benchmark the Niihau Header Trie
fn bench_niihau_trie(c: &mut Criterion) {
    use mauka_mcp_lib::data_structures::niihau_trie::{NiihauTrie, NiihauTrieConfig};
    
    let mut group = c.benchmark_group("niihau_trie");
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(2));
    group.warm_up_time(Duration::from_secs(1));

    // Insert benchmark with different key sizes
    for key_length in [8, 16, 32, 64].iter() {
        group.bench_with_input(
            BenchmarkId::new("insert", key_length), 
            key_length, 
            |b, &length| {
                let trie = NiihauTrie::new();
                let mut keys = Vec::with_capacity(1000);
                
                // Generate keys of specified length
                for i in 0..1000 {
                    let key = format!("{:0width$}", i, width = length);
                    keys.push(key);
                }
                
                let mut index = 0;
                b.iter(|| {
                    // Cycle through keys to avoid reusing the same key
                    let key = &keys[index % keys.len()];
                    index += 1;
                    black_box(trie.insert(key, format!("value_{}", index)).unwrap());
                });
            }
        );
    }
    
    // Lookup benchmark
    group.bench_function("lookup", |b| {
        let trie = NiihauTrie::new();
        let mut keys = Vec::with_capacity(1000);
        
        // Insert 1000 keys first
        for i in 0..1000 {
            let key = format!("key_{}", i);
            keys.push(key.clone());
            trie.insert(&key, format!("value_{}", i)).unwrap();
        }
        
        let mut index = 0;
        b.iter(|| {
            // Cycle through keys for lookups
            let key = &keys[index % keys.len()];
            index += 1;
            black_box(trie.get(key).unwrap());
        });
    });
    
    // Prefix search benchmark
    group.bench_function("prefix_search", |b| {
        let trie = NiihauTrie::new();
        
        // Create a hierarchy of keys
        for i in 0..100 {
            for j in 0..10 {
                let key = format!("prefix_{}_key_{}", i, j);
                trie.insert(&key, format!("value_{}_{}", i, j)).unwrap();
            }
        }
        
        let mut prefix_index = 0;
        b.iter(|| {
            let prefix = format!("prefix_{}_", prefix_index % 100);
            prefix_index += 1;
            black_box(trie.find_by_prefix(&prefix).unwrap());
        });
    });

    group.finish();
}

/// Benchmark the Kona Bloom Filter
fn bench_kona_bloom_filter(c: &mut Criterion) {
    // This will be implemented once the Kona Bloom Filter is developed
    let mut group = c.benchmark_group("kona_bloom_filter");
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(2));

    // Placeholder for future benchmarks
    // group.bench_function("insert", |b| {});
    // group.bench_function("contains", |b| {});

    group.finish();
}

/// Benchmark the Boyer-Moore Pattern Matcher
fn bench_boyer_moore(c: &mut Criterion) {
    // This will be implemented once the Boyer-Moore Pattern Matcher is developed
    let mut group = c.benchmark_group("boyer_moore");
    group.sampling_mode(SamplingMode::Flat);

    // Placeholder for future benchmarks
    // group.bench_with_input(BenchmarkId::new("search", size), &input, |b, i| {});

    group.finish();
}

/// Benchmark the JSON-RPC protocol handler
fn bench_json_rpc(c: &mut Criterion) {
    // This will be implemented once the JSON-RPC handler is developed
    let mut group = c.benchmark_group("json_rpc");

    // Placeholder for future benchmarks
    // group.bench_function("parse_request", |b| {});
    // group.bench_function("serialize_response", |b| {});

    group.finish();
}

// Group all benchmarks together
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_measurement(WallTime)
        .significance_level(0.01)
        .noise_threshold(0.02)
        .confidence_level(0.99);
    targets = bench_kahuna_queue, bench_niihau_trie, bench_kona_bloom_filter,
             bench_boyer_moore, bench_json_rpc
}

criterion_main!(benches);
