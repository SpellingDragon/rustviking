//! Vector Store Benchmark
//!
//! Benchmarks for MemoryVectorStore and RocksDBVectorStore.
//! Compares performance across different data scales and dimensions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::executor::block_on;
use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::rocks::RocksDBVectorStore;
use rustviking::vector_store::traits::VectorStore;
use rustviking::vector_store::types::{IndexParams, VectorPoint};
use serde_json::Value;
use tempfile::TempDir;

/// Fixed seed for reproducible random vector generation
const RANDOM_SEED: u64 = 42;

/// Generate a deterministic random vector using a simple LCG
fn generate_random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut values = Vec::with_capacity(dim);
    let mut state = seed;

    for _ in 0..dim {
        // LCG parameters: a=1664525, c=1013904223, m=2^32
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let normalized = (state as f32) / (u32::MAX as f32);
        // Range [-1, 1]
        values.push(normalized * 2.0 - 1.0);
    }

    values
}

/// Create a test vector point with the given id and vector
fn create_vector_point(id: &str, vector: Vec<f32>, uri: &str) -> VectorPoint {
    let mut payload = serde_json::Map::new();
    payload.insert("id".to_string(), Value::String(id.to_string()));
    payload.insert("uri".to_string(), Value::String(uri.to_string()));
    payload.insert(
        "context_type".to_string(),
        Value::String("resource".to_string()),
    );
    payload.insert("is_leaf".to_string(), Value::Bool(true));
    payload.insert("level".to_string(), Value::Number(0.into()));
    payload.insert(
        "created_at".to_string(),
        Value::String("2024-01-01".to_string()),
    );

    VectorPoint {
        id: id.to_string(),
        vector,
        sparse_vector: None,
        payload: Value::Object(payload),
    }
}

/// Generate a batch of vector points
fn generate_vector_points(count: usize, dimension: usize, start_id: u64) -> Vec<VectorPoint> {
    (0..count)
        .map(|i| {
            let id = format!("vec_{}", start_id + i as u64);
            let uri = format!("/test/vectors/{}_{}", dimension, start_id + i as u64);
            let vector =
                generate_random_vector(dimension, RANDOM_SEED.wrapping_add(start_id + i as u64));
            create_vector_point(&id, vector, &uri)
        })
        .collect()
}

/// Generate a single query vector
fn generate_query_vector(dimension: usize, seed: u64) -> Vec<f32> {
    generate_random_vector(dimension, seed)
}

// =============================================================================
// Upsert Benchmarks
// =============================================================================

fn bench_memory_upsert(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_upsert");

    for (count, dimension) in [(100, 128), (1000, 128), (10000, 128)].iter() {
        group.bench_with_input(
            BenchmarkId::new(format!("dim_{}", dimension), count),
            &(*count, *dimension),
            |b, (count, dimension)| {
                let points = generate_vector_points(*count, *dimension, 0);

                b.iter(|| {
                    let store = MemoryVectorStore::new();
                    let params = IndexParams::default();
                    let collection_name = format!("bench_mem_{}_{}", dimension, count);
                    block_on(store.create_collection(&collection_name, *dimension, params)).unwrap();
                    block_on(store.upsert(&collection_name, points.clone())).unwrap();
                    black_box(&store);
                });
            },
        );
    }

    group.finish();
}

fn bench_rocksdb_upsert(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_upsert");

    for (count, dimension) in [(100, 128), (1000, 128), (10000, 128)].iter() {
        group.bench_with_input(
            BenchmarkId::new(format!("dim_{}", dimension), count),
            &(*count, *dimension),
            |b, (count, dimension)| {
                let points = generate_vector_points(*count, *dimension, 0);

                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let store =
                        RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
                    let params = IndexParams::default();
                    let collection_name = format!("bench_rocks_{}_{}", dimension, count);
                    block_on(store.create_collection(&collection_name, *dimension, params)).unwrap();
                    block_on(store.upsert(&collection_name, points.clone())).unwrap();
                    black_box(&store);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Search Benchmarks
// =============================================================================

fn bench_memory_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_search");

    for (data_size, dimension) in [
        (1000, 128),
        (10000, 128),
        (1000, 256),
        (10000, 256),
        (1000, 512),
        (10000, 512),
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::new(format!("dim_{}_size_{}", dimension, data_size), "top10"),
            &(*data_size, *dimension),
            |b, (data_size, dimension)| {
                let points = generate_vector_points(*data_size, *dimension, 0);
                let query = generate_query_vector(*dimension, RANDOM_SEED);

                b.iter(|| {
                    let store = MemoryVectorStore::new();
                    let params = IndexParams::default();
                    let collection_name = format!("bench_mem_search_{}_{}", dimension, data_size);
                    block_on(store.create_collection(&collection_name, *dimension, params)).unwrap();
                    block_on(store.upsert(&collection_name, points.clone())).unwrap();
                    let results = block_on(store.search(&collection_name, &query, 10, None)).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

fn bench_rocksdb_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_search");

    for (data_size, dimension) in [
        (1000, 128),
        (10000, 128),
        (1000, 256),
        (10000, 256),
        (1000, 512),
        (10000, 512),
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::new(format!("dim_{}_size_{}", dimension, data_size), "top10"),
            &(*data_size, *dimension),
            |b, (data_size, dimension)| {
                let points = generate_vector_points(*data_size, *dimension, 0);
                let query = generate_query_vector(*dimension, RANDOM_SEED);

                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let store =
                        RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
                    let params = IndexParams::default();
                    let collection_name = format!("bench_rocks_search_{}_{}", dimension, data_size);
                    block_on(store.create_collection(&collection_name, *dimension, params)).unwrap();
                    block_on(store.upsert(&collection_name, points.clone())).unwrap();
                    let results = block_on(store.search(&collection_name, &query, 10, None)).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Delete Benchmarks
// =============================================================================

fn bench_memory_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_delete");

    // Single delete benchmark
    group.bench_function("single_delete", |b| {
        let points = generate_vector_points(1000, 128, 0);

        b.iter(|| {
            let store = MemoryVectorStore::new();
            let params = IndexParams::default();
            block_on(store.create_collection("del_test", 128, params)).unwrap();
            block_on(store.upsert("del_test", points.clone())).unwrap();

            let mut counter = 0u64;
            let id = format!("vec_{}", counter % 1000);
            block_on(store.delete("del_test", &id)).unwrap();
            counter += 1;

            // Re-insert to maintain data for next iteration
            let vector = generate_random_vector(128, counter);
            let point = create_vector_point(&id, vector, &format!("/test/{}", id));
            block_on(store.upsert("del_test", vec![point])).unwrap();

            black_box(&store);
        });
    });

    // Delete by URI prefix benchmark
    group.bench_function("delete_by_uri_prefix", |b| {
        // Pre-populate with prefixed URIs
        let points: Vec<VectorPoint> = (0..1000)
            .map(|i| {
                let id = format!("vec_{}", i);
                let prefix = if i < 500 { "/docs/" } else { "/other/" };
                let uri = format!("{}{}", prefix, i);
                let vector = generate_random_vector(128, i as u64);
                create_vector_point(&id, vector, &uri)
            })
            .collect();

        b.iter(|| {
            let store = MemoryVectorStore::new();
            let params = IndexParams::default();
            block_on(store.create_collection("del_prefix_test", 128, params)).unwrap();
            block_on(store.upsert("del_prefix_test", points.clone())).unwrap();

            // Delete docs prefix (will delete 500 items)
            block_on(store.delete_by_uri_prefix("del_prefix_test", "/docs")).unwrap();

            // Re-populate docs for next iteration
            let new_points: Vec<VectorPoint> = (0..500)
                .map(|i| {
                    let id = format!("vec_{}", i);
                    let uri = format!("/docs/{}", i);
                    let vector = generate_random_vector(128, i as u64);
                    create_vector_point(&id, vector, &uri)
                })
                .collect();
            block_on(store.upsert("del_prefix_test", new_points)).unwrap();

            black_box(&store);
        });
    });

    group.finish();
}

fn bench_rocksdb_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_delete");

    // Single delete benchmark
    group.bench_function("single_delete", |b| {
        let points = generate_vector_points(1000, 128, 0);

        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let store =
                RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
            let params = IndexParams::default();
            block_on(store.create_collection("del_test", 128, params)).unwrap();
            block_on(store.upsert("del_test", points.clone())).unwrap();

            let mut counter = 0u64;
            let id = format!("vec_{}", counter % 1000);
            block_on(store.delete("del_test", &id)).unwrap();
            counter += 1;

            // Re-insert to maintain data for next iteration
            let vector = generate_random_vector(128, counter);
            let point = create_vector_point(&id, vector, &format!("/test/{}", id));
            block_on(store.upsert("del_test", vec![point])).unwrap();

            black_box(&store);
        });
    });

    // Delete by URI prefix benchmark
    group.bench_function("delete_by_uri_prefix", |b| {
        // Pre-populate with prefixed URIs
        let points: Vec<VectorPoint> = (0..1000)
            .map(|i| {
                let id = format!("vec_{}", i);
                let prefix = if i < 500 { "/docs/" } else { "/other/" };
                let uri = format!("{}{}", prefix, i);
                let vector = generate_random_vector(128, i as u64);
                create_vector_point(&id, vector, &uri)
            })
            .collect();

        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let store =
                RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
            let params = IndexParams::default();
            block_on(store.create_collection("del_prefix_test", 128, params)).unwrap();
            block_on(store.upsert("del_prefix_test", points.clone())).unwrap();

            // Delete docs prefix (will delete 500 items)
            block_on(store.delete_by_uri_prefix("del_prefix_test", "/docs")).unwrap();

            // Re-populate docs for next iteration
            let new_points: Vec<VectorPoint> = (0..500)
                .map(|i| {
                    let id = format!("vec_{}", i);
                    let uri = format!("/docs/{}", i);
                    let vector = generate_random_vector(128, i as u64);
                    create_vector_point(&id, vector, &uri)
                })
                .collect();
            block_on(store.upsert("del_prefix_test", new_points)).unwrap();

            black_box(&store);
        });
    });

    group.finish();
}

// =============================================================================
// Comparison Benchmarks (Memory vs RocksDB)
// =============================================================================

fn bench_upsert_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("upsert_comparison");

    for count in [100, 1000, 10000].iter() {
        let dimension = 128;
        let points = generate_vector_points(*count, dimension, 0);

        // MemoryVectorStore
        group.bench_with_input(BenchmarkId::new("memory", count), count, |b, &count| {
            b.iter(|| {
                let store = MemoryVectorStore::new();
                let params = IndexParams::default();
                block_on(store.create_collection("comp_mem", dimension, params)).unwrap();
                block_on(store.upsert("comp_mem", points.clone())).unwrap();
                black_box(&store);
            });
        });

        // RocksDBVectorStore
        group.bench_with_input(BenchmarkId::new("rocksdb", count), count, |b, &count| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let store =
                    RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
                let params = IndexParams::default();
                block_on(store.create_collection("comp_rocks", dimension, params)).unwrap();
                block_on(store.upsert("comp_rocks", points.clone())).unwrap();
                black_box(&store);
            });
        });
    }

    group.finish();
}

fn bench_search_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_comparison");

    for data_size in [1000, 10000].iter() {
        let dimension = 128;
        let query = generate_query_vector(dimension, RANDOM_SEED);
        let points = generate_vector_points(*data_size, dimension, 0);

        // MemoryVectorStore
        group.bench_with_input(
            BenchmarkId::new("memory", data_size),
            data_size,
            |b, &data_size| {
                b.iter(|| {
                    let store = MemoryVectorStore::new();
                    let params = IndexParams::default();
                    block_on(store.create_collection("search_comp_mem", dimension, params)).unwrap();
                    block_on(store.upsert("search_comp_mem", points.clone())).unwrap();
                    let results = block_on(store.search("search_comp_mem", &query, 10, None)).unwrap();
                    black_box(results);
                });
            },
        );

        // RocksDBVectorStore
        group.bench_with_input(
            BenchmarkId::new("rocksdb", data_size),
            data_size,
            |b, &data_size| {
                b.iter(|| {
                    let temp_dir = TempDir::new().unwrap();
                    let store =
                        RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
                    let params = IndexParams::default();
                    block_on(store.create_collection("search_comp_rocks", dimension, params)).unwrap();
                    block_on(store.upsert("search_comp_rocks", points.clone())).unwrap();
                    let results = block_on(store.search("search_comp_rocks", &query, 10, None)).unwrap();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Criterion Groups and Main
// =============================================================================

criterion_group!(
    vector_store_benches,
    bench_memory_upsert,
    bench_rocksdb_upsert,
    bench_memory_search,
    bench_rocksdb_search,
    bench_memory_delete,
    bench_rocksdb_delete,
    bench_upsert_comparison,
    bench_search_comparison,
);

criterion_main!(vector_store_benches);
