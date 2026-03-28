//! KV Store Benchmark
//!
//! Benchmarks for RocksKvStore operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rustviking::storage::config::StorageConfig;
use rustviking::storage::{KvStore, RocksKvStore};
use tempfile::TempDir;

fn create_kv_store() -> (RocksKvStore, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = StorageConfig {
        path: temp_dir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 1000,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config).expect("Failed to create RocksKvStore");
    (store, temp_dir)
}

fn bench_kv_put(c: &mut Criterion) {
    let (store, _temp_dir) = create_kv_store();

    let mut group = c.benchmark_group("kv_put");
    group.throughput(Throughput::Bytes(100));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = format!("key_{}", counter);
                let value = vec![0u8; 100];
                store.put(key.as_bytes(), &value).expect("Put failed");
                counter += 1;
                black_box(&store);
            });
        });
    }
    group.finish();
}

fn bench_kv_get(c: &mut Criterion) {
    let (store, _temp_dir) = create_kv_store();

    // Pre-populate with data
    for i in 0..10000 {
        let key = format!("key_{}", i);
        let value = vec![i as u8; 100];
        store.put(key.as_bytes(), &value).expect("Put failed");
    }

    let mut group = c.benchmark_group("kv_get");
    group.throughput(Throughput::Bytes(100));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = format!("key_{}", counter % 10000);
                let result = store.get(key.as_bytes()).expect("Get failed");
                black_box(result);
                counter += 1;
            });
        });
    }
    group.finish();
}

fn bench_kv_scan_prefix(c: &mut Criterion) {
    let (store, _temp_dir) = create_kv_store();

    // Pre-populate with data
    for i in 0..5000 {
        let prefix = if i < 2500 { "prefix_a" } else { "prefix_b" };
        let key = format!("{}_{}", prefix, i);
        let value = vec![i as u8; 50];
        store.put(key.as_bytes(), &value).expect("Put failed");
    }

    let mut group = c.benchmark_group("kv_scan_prefix");

    group.bench_function("scan_2500_entries", |b| {
        b.iter(|| {
            let results = store.scan_prefix(b"prefix_a").expect("Scan failed");
            black_box(results);
        });
    });

    group.finish();
}

fn bench_kv_batch(c: &mut Criterion) {
    let (store, _temp_dir) = create_kv_store();

    let mut group = c.benchmark_group("kv_batch");
    group.throughput(Throughput::Bytes(1000));

    group.bench_function("batch_100_puts", |b| {
        b.iter(|| {
            let mut batch = store.batch().expect("Batch failed");
            for i in 0..100 {
                let key = format!("batch_key_{}", i);
                let value = vec![i as u8; 10];
                batch.put(key.into_bytes(), value);
            }
            batch.commit().expect("Commit failed");
            black_box(&store);
        });
    });

    group.finish();
}

criterion_group!(
    kv_benches,
    bench_kv_put,
    bench_kv_get,
    bench_kv_scan_prefix,
    bench_kv_batch,
);

criterion_main!(kv_benches);
