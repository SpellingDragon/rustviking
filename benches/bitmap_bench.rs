//! Bitmap Benchmark
//!
//! Benchmarks for Bitmap set operations using public API.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustviking::index::bitmap::Bitmap;

fn create_bitmap_with_ids(count: u64) -> Bitmap {
    let ids: Vec<u64> = (0..count).collect();
    Bitmap::from_ids(&ids)
}

fn create_sparse_bitmap(count: u64, step: u64) -> Bitmap {
    let ids: Vec<u64> = (0..count).map(|i| i * step).collect();
    Bitmap::from_ids(&ids)
}

fn bench_bitmap_intersection(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_intersection");

    for size in [100, 1000, 10000].iter() {
        let bm1 = create_bitmap_with_ids(*size as u64);
        let bm2 = create_sparse_bitmap(*size as u64 / 2, 2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result = bm1.intersection(&bm2);
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_bitmap_union(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_union");

    for size in [100, 1000, 10000].iter() {
        let bm1 = create_bitmap_with_ids(*size as u64);
        let bm2 = create_sparse_bitmap(*size as u64 / 2, 3);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result = bm1.union(&bm2);
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_bitmap_difference(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_difference");

    for size in [100, 1000, 10000].iter() {
        let bm1 = create_bitmap_with_ids(*size as u64);
        let bm2 = create_sparse_bitmap(*size as u64 / 2, 2);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result = bm1.difference(&bm2);
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_bitmap_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_add");

    group.bench_function("add_sequential", |b| {
        let mut bm = Bitmap::new();
        let mut counter = 0u64;
        b.iter(|| {
            bm.add(counter);
            counter += 1;
            black_box(&bm);
        });
    });

    group.finish();
}

fn bench_bitmap_add_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_add_range");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut bm = Bitmap::new();
                bm.add_range(0, size as u64);
                black_box(bm);
            });
        });
    }

    group.finish();
}

fn bench_bitmap_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_contains");

    let bm = create_bitmap_with_ids(10000);

    group.bench_function("contains_existing", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let result = bm.contains(counter % 10000);
            counter += 1;
            black_box(result);
        });
    });

    group.bench_function("contains_missing", |b| {
        let mut counter = 10000u64;
        b.iter(|| {
            let result = bm.contains(counter);
            counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

fn bench_bitmap_cardinality(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_cardinality");

    let bm = create_bitmap_with_ids(10000);

    group.bench_function("cardinality", |b| {
        b.iter(|| {
            let count = bm.cardinality();
            black_box(count);
        });
    });

    group.finish();
}

fn bench_bitmap_to_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_to_vec");

    for size in [100, 1000, 10000].iter() {
        let bm = create_bitmap_with_ids(*size as u64);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let vec = bm.to_vec();
                black_box(vec);
            });
        });
    }

    group.finish();
}

fn bench_bitmap_combined_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_combined");

    let bm1 = create_bitmap_with_ids(5000);
    let bm2 = create_sparse_bitmap(2500, 2);
    let bm3 = create_sparse_bitmap(1250, 4);

    group.bench_function("intersection_union_chain", |b| {
        b.iter(|| {
            let result = bm1.intersection(&bm2).union(&bm3);
            black_box(result);
        });
    });

    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let mut bm = Bitmap::new();
            bm.add_range(0, 1000);
            let other = Bitmap::from_ids(&(500..1500).collect::<Vec<_>>());
            let intersection = bm.intersection(&other);
            let union = intersection.union(&bm3);
            let _final = union.difference(&bm2);
            black_box(union);
        });
    });

    group.finish();
}

criterion_group!(
    bitmap_benches,
    bench_bitmap_intersection,
    bench_bitmap_union,
    bench_bitmap_difference,
    bench_bitmap_add,
    bench_bitmap_add_range,
    bench_bitmap_contains,
    bench_bitmap_cardinality,
    bench_bitmap_to_vec,
    bench_bitmap_combined_operations,
);

criterion_main!(bitmap_benches);
