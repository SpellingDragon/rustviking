//! Compute Benchmark
//!
//! Benchmarks for distance computation, SIMD operations, and embedding generation.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::executor::block_on;
use rustviking::compute::distance::DistanceComputer;
use rustviking::compute::simd::{
    batch_dot_products, batch_l2_distances, parallel_batch_dot_products,
    parallel_batch_l2_distances, top_k_smallest,
};
use rustviking::embedding::mock::MockEmbeddingProvider;
use rustviking::embedding::traits::EmbeddingProvider;
use rustviking::embedding::types::EmbeddingRequest;

// ============================================================================
// Helper functions for generating test data
// ============================================================================

/// Generate a random vector with fixed seed for reproducibility
fn generate_random_vector(dim: usize, seed: u64) -> Vec<f32> {
    // Simple LCG RNG for reproducibility
    let mut state = seed;
    (0..dim)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 40) as f32) / ((1u64 << 24) as f32)
        })
        .collect()
}

/// Generate multiple random vectors
fn generate_random_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| generate_random_vector(dim, seed.wrapping_add(i as u64)))
        .collect()
}

// ============================================================================
// Distance computation benchmarks
// ============================================================================

fn bench_distance_computer(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_computer");

    // Benchmark L2 distance for different dimensions
    for dim in [128, 256, 512, 1024].iter() {
        let computer = DistanceComputer::new(*dim);
        let vec_a = generate_random_vector(*dim, 12345);
        let vec_b = generate_random_vector(*dim, 67890);

        group.bench_with_input(BenchmarkId::new("l2_distance", dim), dim, |b, _| {
            b.iter(|| {
                let result = computer.l2_distance(&vec_a, &vec_b);
                black_box(result);
            });
        });

        group.bench_with_input(BenchmarkId::new("dot_product", dim), dim, |b, _| {
            b.iter(|| {
                let result = computer.dot_product(&vec_a, &vec_b);
                black_box(result);
            });
        });

        group.bench_with_input(BenchmarkId::new("cosine_distance", dim), dim, |b, _| {
            b.iter(|| {
                let result = computer.cosine_distance(&vec_a, &vec_b);
                black_box(result);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Batch computation benchmarks
// ============================================================================

fn bench_batch_distances(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_distances");

    let dim = 128;
    let query = generate_random_vector(dim, 12345);

    // Benchmark different collection sizes
    for count in [100, 1000, 10000].iter() {
        let vectors = generate_random_vectors(*count, dim, 67890);

        // Sequential batch L2
        group.bench_with_input(
            BenchmarkId::new("batch_l2_sequential", count),
            count,
            |b, _| {
                b.iter(|| {
                    let results = batch_l2_distances(&query, &vectors);
                    black_box(results);
                });
            },
        );

        // Parallel batch L2
        group.bench_with_input(
            BenchmarkId::new("batch_l2_parallel", count),
            count,
            |b, _| {
                b.iter(|| {
                    let results = parallel_batch_l2_distances(&query, &vectors);
                    black_box(results);
                });
            },
        );

        // Sequential batch dot product
        group.bench_with_input(
            BenchmarkId::new("batch_dot_sequential", count),
            count,
            |b, _| {
                b.iter(|| {
                    let results = batch_dot_products(&query, &vectors);
                    black_box(results);
                });
            },
        );

        // Parallel batch dot product
        group.bench_with_input(
            BenchmarkId::new("batch_dot_parallel", count),
            count,
            |b, _| {
                b.iter(|| {
                    let results = parallel_batch_dot_products(&query, &vectors);
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

// Benchmark batch operations with different dimensions
fn bench_batch_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_dimensions");

    let count = 1000;

    for dim in [128, 256, 512, 1024].iter() {
        let query = generate_random_vector(*dim, 12345);
        let vectors = generate_random_vectors(count, *dim, 67890);

        group.bench_with_input(BenchmarkId::new("l2_dim", dim), dim, |b, _| {
            b.iter(|| {
                let results = parallel_batch_l2_distances(&query, &vectors);
                black_box(results);
            });
        });

        group.bench_with_input(BenchmarkId::new("dot_dim", dim), dim, |b, _| {
            b.iter(|| {
                let results = parallel_batch_dot_products(&query, &vectors);
                black_box(results);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Top-K benchmarks
// ============================================================================

fn bench_top_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("top_k");

    // Benchmark different data sizes and k values
    for data_size in [1000, 10000, 100000].iter() {
        let values: Vec<f32> = (0..*data_size).map(|i| i as f32).collect();

        for k in [10, 50, 100].iter() {
            let bench_id = format!("size_{}_k_{}", data_size, k);
            group.bench_with_input(
                BenchmarkId::new("top_k_smallest", &bench_id),
                &(*data_size, *k),
                |b, (_, k)| {
                    b.iter(|| {
                        let results = top_k_smallest(&values, *k);
                        black_box(results);
                    });
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Embedding generation benchmarks
// ============================================================================

fn bench_embedding_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("embedding_generation");

    // Benchmark single embed with different text lengths
    for text_len in [10, 100, 1000].iter() {
        let provider = MockEmbeddingProvider::new(512);
        let text = "a".repeat(*text_len);
        let request = EmbeddingRequest {
            texts: vec![text],
            model: None,
            normalize: false,
        };

        group.bench_with_input(
            BenchmarkId::new("embed_single", text_len),
            text_len,
            |b, _| {
                b.iter(|| {
                    let result = block_on(provider.embed(request.clone())).unwrap();
                    black_box(result);
                });
            },
        );
    }

    // Benchmark embed with different dimensions
    for dim in [128, 256, 512, 1024].iter() {
        let provider = MockEmbeddingProvider::new(*dim);
        let request = EmbeddingRequest {
            texts: vec!["benchmark test text".to_string()],
            model: None,
            normalize: false,
        };

        group.bench_with_input(BenchmarkId::new("embed_dim", dim), dim, |b, _| {
            b.iter(|| {
                let result = block_on(provider.embed(request.clone())).unwrap();
                black_box(result);
            });
        });
    }

    // Benchmark normalized vs non-normalized
    let provider_norm = MockEmbeddingProvider::new(512);
    let request_norm = EmbeddingRequest {
        texts: vec!["benchmark test text".to_string()],
        model: None,
        normalize: true,
    };
    let request_no_norm = EmbeddingRequest {
        texts: vec!["benchmark test text".to_string()],
        model: None,
        normalize: false,
    };

    group.bench_function("embed_normalized", |b| {
        b.iter(|| {
            let result = block_on(provider_norm.embed(request_norm.clone())).unwrap();
            black_box(result);
        });
    });

    group.bench_function("embed_not_normalized", |b| {
        b.iter(|| {
            let result = block_on(provider_norm.embed(request_no_norm.clone())).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_embedding_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("embedding_batch");

    let provider = MockEmbeddingProvider::new(512);

    // Benchmark embed_batch with different batch sizes
    for batch_size in [10, 50, 100].iter() {
        let requests: Vec<EmbeddingRequest> = (0..*batch_size)
            .map(|i| EmbeddingRequest {
                texts: vec![format!("text {}", i)],
                model: None,
                normalize: false,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("embed_batch", batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    let results = block_on(provider.embed_batch(requests.clone(), 4)).unwrap();
                    black_box(results);
                });
            },
        );
    }

    // Benchmark embed_batch with different texts per request
    for texts_per_req in [1, 5, 10].iter() {
        let requests: Vec<EmbeddingRequest> = (0..10)
            .map(|i| EmbeddingRequest {
                texts: (0..*texts_per_req)
                    .map(|j| format!("request {} text {}", i, j))
                    .collect(),
                model: None,
                normalize: false,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("embed_batch_texts_per_req", texts_per_req),
            texts_per_req,
            |b, _| {
                b.iter(|| {
                    let results = block_on(provider.embed_batch(requests.clone(), 4)).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// SIMD vs Scalar comparison benchmarks
// ============================================================================

fn bench_simd_vs_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_vs_scalar");

    // Note: The DistanceComputer automatically uses SIMD when available
    // We benchmark it to show the performance characteristics

    for dim in [128, 256, 512, 1024].iter() {
        let computer = DistanceComputer::new(*dim);
        let vec_a = generate_random_vector(*dim, 12345);
        let vec_b = generate_random_vector(*dim, 67890);

        // L2 distance with SIMD (automatic selection)
        group.bench_with_input(BenchmarkId::new("simd_l2", dim), dim, |b, _| {
            b.iter(|| {
                let result = computer.l2_distance(&vec_a, &vec_b);
                black_box(result);
            });
        });

        // Dot product with SIMD (automatic selection)
        group.bench_with_input(BenchmarkId::new("simd_dot", dim), dim, |b, _| {
            b.iter(|| {
                let result = computer.dot_product(&vec_a, &vec_b);
                black_box(result);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Criterion groups and main
// ============================================================================

criterion_group!(
    compute_benches,
    bench_distance_computer,
    bench_batch_distances,
    bench_batch_dimensions,
    bench_top_k,
    bench_embedding_generation,
    bench_embedding_batch,
    bench_simd_vs_scalar,
);

criterion_main!(compute_benches);
