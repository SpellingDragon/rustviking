//! Vector Index Benchmark
//!
//! Benchmarks for IvfPqIndex vector operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustviking::index::{IvfPqIndex, IvfPqParams, MetricType, VectorIndex};

const DIMENSION: usize = 128;

fn create_index() -> IvfPqIndex {
    let params = IvfPqParams {
        num_partitions: 8,
        num_sub_vectors: 8,
        pq_bits: 8,
        metric: MetricType::L2,
    };
    IvfPqIndex::new(params, DIMENSION)
}

fn generate_random_vector(dim: usize, seed: u64) -> Vec<f32> {
    // Simple deterministic pseudo-random vector generation
    (0..dim)
        .map(|i| {
            let x = (seed.wrapping_mul(i as u64 + 1)) as f32;
            (x % 2.0) - 1.0 // Range [-1, 1]
        })
        .collect()
}

fn generate_training_data(count: usize, dim: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| generate_random_vector(dim, i as u64))
        .collect()
}

fn bench_vector_insert(c: &mut Criterion) {
    let index = create_index();

    // Train with some data
    let training_data = generate_training_data(100, DIMENSION);
    index.train(&training_data).expect("Train failed");

    let mut group = c.benchmark_group("vector_insert");

    for count in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("insert", count), count, |b, &count| {
            let mut id_counter = 0u64;
            b.iter(|| {
                let vector = generate_random_vector(DIMENSION, id_counter);
                index.insert(id_counter, &vector, 2).expect("Insert failed");
                id_counter += 1;
                if id_counter >= count {
                    id_counter = 0;
                }
                black_box(&index);
            });
        });
    }

    group.finish();
}

fn bench_vector_insert_batch(c: &mut Criterion) {
    let index = create_index();

    // Train with some data
    let training_data = generate_training_data(100, DIMENSION);
    index.train(&training_data).expect("Train failed");

    let mut group = c.benchmark_group("vector_insert_batch");

    group.bench_function("batch_100_vectors", |b| {
        let mut batch_counter = 0u64;
        b.iter(|| {
            let vectors: Vec<(u64, Vec<f32>, u8)> = (0..100)
                .map(|i| {
                    let id = batch_counter * 100 + i as u64;
                    let vector = generate_random_vector(DIMENSION, id);
                    (id, vector, 2)
                })
                .collect();
            index.insert_batch(&vectors).expect("Batch insert failed");
            batch_counter += 1;
            black_box(&index);
        });
    });

    group.finish();
}

fn bench_vector_search(c: &mut Criterion) {
    let index = create_index();

    // Train and populate with data
    let training_data = generate_training_data(100, DIMENSION);
    index.train(&training_data).expect("Train failed");

    for i in 0..1000 {
        let vector = generate_random_vector(DIMENSION, i);
        index.insert(i, &vector, 2).expect("Insert failed");
    }

    let mut group = c.benchmark_group("vector_search");

    for k in [1, 10, 50].iter() {
        group.bench_with_input(BenchmarkId::new("k", k), k, |b, &k| {
            let mut query_counter = 0u64;
            b.iter(|| {
                let query = generate_random_vector(DIMENSION, query_counter);
                let results = index.search(&query, k, None).expect("Search failed");
                query_counter += 1;
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_vector_search_with_filter(c: &mut Criterion) {
    let index = create_index();

    // Train and populate with data at different levels
    let training_data = generate_training_data(100, DIMENSION);
    index.train(&training_data).expect("Train failed");

    for i in 0..1000 {
        let vector = generate_random_vector(DIMENSION, i);
        let level = (i % 3) as u8; // Levels 0, 1, 2
        index.insert(i, &vector, level).expect("Insert failed");
    }

    let mut group = c.benchmark_group("vector_search_with_filter");

    group.bench_function("search_level_filter", |b| {
        let mut query_counter = 0u64;
        b.iter(|| {
            let query = generate_random_vector(DIMENSION, query_counter);
            let results = index.search(&query, 10, Some(2)).expect("Search failed");
            query_counter += 1;
            black_box(results);
        });
    });

    group.finish();
}

fn bench_vector_get(c: &mut Criterion) {
    let index = create_index();

    // Train and populate with data
    let training_data = generate_training_data(100, DIMENSION);
    index.train(&training_data).expect("Train failed");

    for i in 0..1000 {
        let vector = generate_random_vector(DIMENSION, i);
        index.insert(i, &vector, 2).expect("Insert failed");
    }

    let mut group = c.benchmark_group("vector_get");

    group.bench_function("get_by_id", |b| {
        let mut id_counter = 0u64;
        b.iter(|| {
            let result = index.get(id_counter % 1000).expect("Get failed");
            id_counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    vector_benches,
    bench_vector_insert,
    bench_vector_insert_batch,
    bench_vector_search,
    bench_vector_search_with_filter,
    bench_vector_get,
);

criterion_main!(vector_benches);
