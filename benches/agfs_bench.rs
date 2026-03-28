//! AGFS Benchmark
//!
//! Benchmarks for AGFS filesystem operations using MemoryPlugin.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rustviking::agfs::{FileSystem, WriteFlag};
use rustviking::plugins::memory::MemoryPlugin;

fn create_memory_fs() -> MemoryPlugin {
    MemoryPlugin::new()
}

fn bench_agfs_write(c: &mut Criterion) {
    let fs = create_memory_fs();

    let mut group = c.benchmark_group("agfs_write");
    group.throughput(Throughput::Bytes(1024));

    group.bench_function("write_1kb", |b| {
        let mut counter = 0u64;
        let data = vec![0xABu8; 1024];
        b.iter(|| {
            let path = format!("/file_{}", counter);
            fs.write(&path, &data, 0, WriteFlag::CREATE)
                .expect("Write failed");
            counter += 1;
            black_box(&fs);
        });
    });

    group.finish();
}

fn bench_agfs_read(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Pre-populate files
    let data = vec![0xCDu8; 1024];
    for i in 0..1000 {
        let path = format!("/file_{}", i);
        fs.write(&path, &data, 0, WriteFlag::CREATE)
            .expect("Write failed");
    }

    let mut group = c.benchmark_group("agfs_read");
    group.throughput(Throughput::Bytes(1024));

    group.bench_function("read_1kb", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/file_{}", counter % 1000);
            let result = fs.read(&path, 0, 0).expect("Read failed");
            counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

fn bench_agfs_mkdir(c: &mut Criterion) {
    let fs = create_memory_fs();

    let mut group = c.benchmark_group("agfs_mkdir");

    group.bench_function("mkdir", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/dir_{}", counter);
            fs.mkdir(&path, 0o755).expect("Mkdir failed");
            counter += 1;
            black_box(&fs);
        });
    });

    group.finish();
}

fn bench_agfs_read_dir(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Pre-populate directory with files
    fs.mkdir("/bench_dir", 0o755).expect("Mkdir failed");
    for i in 0..100 {
        let path = format!("/bench_dir/file_{}", i);
        fs.write(&path, b"data", 0, WriteFlag::CREATE)
            .expect("Write failed");
    }

    let mut group = c.benchmark_group("agfs_read_dir");

    group.bench_function("read_dir_100_files", |b| {
        b.iter(|| {
            let result = fs.read_dir("/bench_dir").expect("Read dir failed");
            black_box(result);
        });
    });

    group.finish();
}

fn bench_agfs_stat(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Pre-populate files
    for i in 0..1000 {
        let path = format!("/stat_file_{}", i);
        fs.write(&path, b"test data", 0, WriteFlag::CREATE)
            .expect("Write failed");
    }

    let mut group = c.benchmark_group("agfs_stat");

    group.bench_function("stat", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/stat_file_{}", counter % 1000);
            let result = fs.stat(&path).expect("Stat failed");
            counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

fn bench_agfs_exists(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Pre-populate some files
    for i in 0..500 {
        let path = format!("/exists_file_{}", i);
        fs.write(&path, b"x", 0, WriteFlag::CREATE)
            .expect("Write failed");
    }

    let mut group = c.benchmark_group("agfs_exists");

    group.bench_function("exists_present", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/exists_file_{}", counter % 500);
            let result = fs.exists(&path);
            counter += 1;
            black_box(result);
        });
    });

    group.bench_function("exists_missing", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/missing_file_{}", counter);
            let result = fs.exists(&path);
            counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

fn bench_agfs_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("agfs_remove");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("remove", size), size, |b, _| {
            let mut counter = 0u64;
            b.iter(|| {
                let fs = create_memory_fs();
                // Create files to remove
                for i in 0..*size {
                    let path = format!("/remove_file_{}", i);
                    fs.write(&path, b"data", 0, WriteFlag::CREATE)
                        .expect("Write failed");
                }

                // Remove one file
                let path = format!("/remove_file_{}", counter % *size);
                fs.remove(&path).expect("Remove failed");
                counter += 1;
                black_box(&fs);
            });
        });
    }

    group.finish();
}

fn bench_agfs_append(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Create initial file
    fs.write("/append_test", b"initial", 0, WriteFlag::CREATE)
        .expect("Write failed");

    let mut group = c.benchmark_group("agfs_append");
    group.throughput(Throughput::Bytes(100));

    group.bench_function("append_100_bytes", |b| {
        let data = vec![0xEFu8; 100];
        b.iter(|| {
            fs.write("/append_test", &data, 0, WriteFlag::APPEND)
                .expect("Append failed");
            black_box(&fs);
        });
    });

    group.finish();
}

fn bench_agfs_size(c: &mut Criterion) {
    let fs = create_memory_fs();

    // Create files of various sizes
    for i in 0..100 {
        let path = format!("/size_file_{}", i);
        let data = vec![0u8; (i + 1) * 100];
        fs.write(&path, &data, 0, WriteFlag::CREATE)
            .expect("Write failed");
    }

    let mut group = c.benchmark_group("agfs_size");

    group.bench_function("size", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let path = format!("/size_file_{}", counter % 100);
            let result = fs.size(&path).expect("Size failed");
            counter += 1;
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    agfs_benches,
    bench_agfs_write,
    bench_agfs_read,
    bench_agfs_mkdir,
    bench_agfs_read_dir,
    bench_agfs_stat,
    bench_agfs_exists,
    bench_agfs_remove,
    bench_agfs_append,
    bench_agfs_size,
);

criterion_main!(agfs_benches);
