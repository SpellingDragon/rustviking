//! CLI benchmark commands
//!
//! Implements benchmark subcommands for performance testing.

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::cli::{success, CliResponse};
use crate::error::{Result, RustVikingError};
use crate::index::bitmap::Bitmap;
use crate::index::{IvfIndex, IvfParams, MetricType, VectorIndex};
use crate::storage::{KvStore, RocksKvStore, StorageConfig};

/// Latency statistics collector
#[derive(Debug, Default)]
pub struct LatencyStats {
    latencies: Vec<f64>, // in microseconds
}

impl LatencyStats {
    /// Create a new empty stats collector
    pub fn new() -> Self {
        Self {
            latencies: Vec::new(),
        }
    }

    /// Record a duration
    pub fn record(&mut self, duration: Duration) {
        let us = duration.as_nanos() as f64 / 1000.0;
        self.latencies.push(us);
    }

    /// Get count of recorded latencies
    pub fn count(&self) -> usize {
        self.latencies.len()
    }

    /// Get average latency in microseconds
    pub fn avg_us(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        self.latencies.iter().sum::<f64>() / self.latencies.len() as f64
    }

    /// Get p50 latency in microseconds
    pub fn p50_us(&self) -> f64 {
        self.percentile(50.0)
    }

    /// Get p99 latency in microseconds
    pub fn p99_us(&self) -> f64 {
        self.percentile(99.0)
    }

    /// Get min latency in microseconds
    pub fn min_us(&self) -> f64 {
        if self.latencies.is_empty() {
            0.0
        } else {
            self.latencies.iter().copied().fold(f64::MAX, |a, b| a.min(b))
        }
    }

    /// Get max latency in microseconds
    pub fn max_us(&self) -> f64 {
        if self.latencies.is_empty() {
            0.0
        } else {
            self.latencies.iter().copied().fold(f64::MIN, |a, b| a.max(b))
        }
    }

    /// Get QPS (operations per second)
    pub fn qps(&self) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let total_us: f64 = self.latencies.iter().sum();
        if total_us == 0.0 {
            return f64::MAX;
        }
        1_000_000.0 * self.latencies.len() as f64 / total_us
    }

    /// Get total time in milliseconds
    pub fn total_ms(&self) -> f64 {
        self.latencies.iter().sum::<f64>() / 1000.0
    }

    /// Get percentile value
    fn percentile(&self, p: f64) -> f64 {
        if self.latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }
}

/// Benchmark result data
#[derive(Serialize)]
pub struct BenchResult {
    pub test: &'static str,
    pub count: usize,
    pub total_ms: f64,
    pub qps: f64,
    pub avg_us: f64,
    pub p50_us: f64,
    pub p99_us: f64,
    pub min_us: f64,
    pub max_us: f64,
}

/// Temporary directory wrapper for benchmark cleanup
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Result<Self> {
        let base = std::env::temp_dir();
        let dir_name = format!("rustviking_bench_{}", uuid::Uuid::new_v4());
        let path = base.join(dir_name);
        fs::create_dir_all(&path).map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(Self { path })
    }
}

/// Run KV write benchmark
pub fn bench_kv_write(count: usize) -> Result<CliResponse<BenchResult>> {
    let temp_dir = TempDir::new()?;
    let config = StorageConfig {
        path: temp_dir.path.to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 1000,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config)?;
    let mut stats = LatencyStats::new();

    for i in 0..count {
        let key = format!("bench_key_{}", i);
        let value = generate_random_bytes(100, i as u64);

        let start = Instant::now();
        store.put(key.as_bytes(), &value)?;
        stats.record(start.elapsed());
    }

    // Cleanup temp directory
    let _ = fs::remove_dir_all(&temp_dir.path);

    Ok(success(BenchResult {
        test: "kv-write",
        count: stats.count(),
        total_ms: stats.total_ms(),
        qps: stats.qps(),
        avg_us: stats.avg_us(),
        p50_us: stats.p50_us(),
        p99_us: stats.p99_us(),
        min_us: stats.min_us(),
        max_us: stats.max_us(),
    }))
}

/// Run KV read benchmark
pub fn bench_kv_read(count: usize) -> Result<CliResponse<BenchResult>> {
    let temp_dir = TempDir::new()?;
    let config = StorageConfig {
        path: temp_dir.path.to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 1000,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config)?;

    // Pre-populate with data (10x the read count for variety)
    let prefill_count = std::cmp::max(count * 10, 1000);
    for i in 0..prefill_count {
        let key = format!("bench_key_{}", i);
        let value = generate_random_bytes(100, i as u64);
        store.put(key.as_bytes(), &value)?;
    }

    let mut stats = LatencyStats::new();

    // Random reads
    for i in 0..count {
        let key_idx = i % prefill_count;
        let key = format!("bench_key_{}", key_idx);

        let start = Instant::now();
        let _ = store.get(key.as_bytes())?;
        stats.record(start.elapsed());
    }

    // Cleanup temp directory
    let _ = fs::remove_dir_all(&temp_dir.path);

    Ok(success(BenchResult {
        test: "kv-read",
        count: stats.count(),
        total_ms: stats.total_ms(),
        qps: stats.qps(),
        avg_us: stats.avg_us(),
        p50_us: stats.p50_us(),
        p99_us: stats.p99_us(),
        min_us: stats.min_us(),
        max_us: stats.max_us(),
    }))
}

/// Vector dimension for benchmarks
const VECTOR_DIM: usize = 128;

/// Run vector search benchmark
pub fn bench_vector_search(count: usize) -> Result<CliResponse<BenchResult>> {
    let params = IvfParams {
        num_partitions: 8,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, VECTOR_DIM);

    // Generate training data and train the index
    let training_data: Vec<Vec<f32>> = (0..100)
        .map(|i| generate_random_vector(VECTOR_DIM, i as u64))
        .collect();
    index.train(&training_data)?;

    // Pre-populate with vectors (10x the search count)
    let prefill_count = std::cmp::max(count * 10, 1000);
    for i in 0..prefill_count {
        let vector = generate_random_vector(VECTOR_DIM, i as u64);
        index.insert(i as u64, &vector, 2)?;
    }

    let mut stats = LatencyStats::new();

    // Run searches
    for i in 0..count {
        let query = generate_random_vector(VECTOR_DIM, (i + prefill_count) as u64);

        let start = Instant::now();
        let _ = index.search(&query, 10, None)?;
        stats.record(start.elapsed());
    }

    Ok(success(BenchResult {
        test: "vector-search",
        count: stats.count(),
        total_ms: stats.total_ms(),
        qps: stats.qps(),
        avg_us: stats.avg_us(),
        p50_us: stats.p50_us(),
        p99_us: stats.p99_us(),
        min_us: stats.min_us(),
        max_us: stats.max_us(),
    }))
}

/// Run bitmap operations benchmark
pub fn bench_bitmap_ops(count: usize) -> Result<CliResponse<BenchResult>> {
    let mut stats = LatencyStats::new();

    // Pre-create bitmaps for intersection/union tests
    let bm1 = create_bitmap_with_ids(count as u64);
    let bm2 = create_sparse_bitmap(count as u64 / 2, 2);

    // Alternate between intersection and union
    for i in 0..count {
        let start = Instant::now();
        if i % 2 == 0 {
            let _ = bm1.intersection(&bm2);
        } else {
            let _ = bm1.union(&bm2);
        }
        stats.record(start.elapsed());
    }

    Ok(success(BenchResult {
        test: "bitmap-ops",
        count: stats.count(),
        total_ms: stats.total_ms(),
        qps: stats.qps(),
        avg_us: stats.avg_us(),
        p50_us: stats.p50_us(),
        p99_us: stats.p99_us(),
        min_us: stats.min_us(),
        max_us: stats.max_us(),
    }))
}

// Helper functions

/// Generate random bytes for testing
fn generate_random_bytes(size: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let mut result = Vec::with_capacity(size);
    for _ in 0..size {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        result.push((state >> 40) as u8);
    }
    result
}

/// Generate a random vector with fixed seed for reproducibility
fn generate_random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..dim)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 40) as f32) / ((1u64 << 24) as f32) * 2.0 - 1.0 // Range [-1, 1]
        })
        .collect()
}

/// Create a bitmap with sequential IDs
fn create_bitmap_with_ids(count: u64) -> Bitmap {
    let ids: Vec<u64> = (0..count).collect();
    Bitmap::from_ids(&ids)
}

/// Create a sparse bitmap with step
fn create_sparse_bitmap(count: u64, step: u64) -> Bitmap {
    let ids: Vec<u64> = (0..count).map(|i| i * step).collect();
    Bitmap::from_ids(&ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_stats() {
        let mut stats = LatencyStats::new();

        stats.record(Duration::from_micros(10));
        stats.record(Duration::from_micros(20));
        stats.record(Duration::from_micros(30));

        assert_eq!(stats.count(), 3);
        assert!((stats.avg_us() - 20.0).abs() < 0.01);
        assert!((stats.p50_us() - 20.0).abs() < 0.01);
        assert!((stats.min_us() - 10.0).abs() < 0.01);
        assert!((stats.max_us() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_latency_stats_empty() {
        let stats = LatencyStats::new();
        assert_eq!(stats.count(), 0);
        assert_eq!(stats.avg_us(), 0.0);
        assert_eq!(stats.qps(), 0.0);
    }
}
