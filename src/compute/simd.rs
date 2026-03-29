//! SIMD optimization utilities
//!
//! Provides optimized batch operations using SIMD acceleration.
//!
//! # SIMD Acceleration
//!
//! This module uses platform-specific SIMD intrinsics:
//! - **ARM64 (Apple Silicon, etc.)**: Uses NEON intrinsics
//! - **x86_64**: Uses AVX2/FMA intrinsics when available

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use rayon::prelude::*;

use super::distance::DistanceComputer;

// ============================================================================
// NEON implementations for ARM64
// ============================================================================

/// NEON-accelerated dot product (single vector pair)
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn dot_product_neon(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 4;
    let mut sum = vdupq_n_f32(0.0);

    for i in 0..chunks {
        let va = vld1q_f32(a.as_ptr().add(i * 4));
        let vb = vld1q_f32(b.as_ptr().add(i * 4));
        sum = vfmaq_f32(sum, va, vb);
    }

    let mut result = vaddvq_f32(sum);

    for i in (chunks * 4)..n {
        result += a[i] * b[i];
    }
    result
}

/// NEON-accelerated L2 squared distance (single vector pair)
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn l2_distance_neon(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 4;
    let mut sum = vdupq_n_f32(0.0);

    for i in 0..chunks {
        let va = vld1q_f32(a.as_ptr().add(i * 4));
        let vb = vld1q_f32(b.as_ptr().add(i * 4));
        let diff = vsubq_f32(va, vb);
        sum = vfmaq_f32(sum, diff, diff);
    }

    let mut result = vaddvq_f32(sum);

    for i in (chunks * 4)..n {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    result
}

// ============================================================================
// AVX2 implementations for x86_64
// ============================================================================

/// AVX2-accelerated dot product (single vector pair)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
#[inline]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 8;
    let mut sum = _mm256_setzero_ps();

    for i in 0..chunks {
        let va = _mm256_loadu_ps(a.as_ptr().add(i * 8));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i * 8));
        sum = _mm256_fmadd_ps(va, vb, sum);
    }

    let hi = _mm256_extractf128_ps(sum, 1);
    let lo = _mm256_castps256_ps128(sum);
    let sum128 = _mm_add_ps(lo, hi);
    let sum64 = _mm_add_ps(sum128, _mm_movehl_ps(sum128, sum128));
    let sum32 = _mm_add_ss(sum64, _mm_shuffle_ps(sum64, sum64, 1));
    let mut result = _mm_cvtss_f32(sum32);

    for i in (chunks * 8)..n {
        result += a[i] * b[i];
    }
    result
}

/// AVX2-accelerated L2 squared distance (single vector pair)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2", enable = "fma")]
#[inline]
unsafe fn l2_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let chunks = n / 8;
    let mut sum = _mm256_setzero_ps();

    for i in 0..chunks {
        let va = _mm256_loadu_ps(a.as_ptr().add(i * 8));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i * 8));
        let diff = _mm256_sub_ps(va, vb);
        sum = _mm256_fmadd_ps(diff, diff, sum);
    }

    let hi = _mm256_extractf128_ps(sum, 1);
    let lo = _mm256_castps256_ps128(sum);
    let sum128 = _mm_add_ps(lo, hi);
    let sum64 = _mm_add_ps(sum128, _mm_movehl_ps(sum128, sum128));
    let sum32 = _mm_add_ss(sum64, _mm_shuffle_ps(sum64, sum64, 1));
    let mut result = _mm_cvtss_f32(sum32);

    for i in (chunks * 8)..n {
        let diff = a[i] - b[i];
        result += diff * diff;
    }
    result
}

// ============================================================================
// Scalar fallback implementations
// ============================================================================

#[cfg(any(
    not(any(target_arch = "aarch64", target_arch = "x86_64")),
    target_arch = "x86_64"
))]
#[inline]
fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(any(
    not(any(target_arch = "aarch64", target_arch = "x86_64")),
    target_arch = "x86_64"
))]
#[inline]
fn l2_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

// ============================================================================
// Public batch operations
// ============================================================================

/// Computes a single distance between two vectors using SIMD when available.
///
/// This is the core primitive used by batch operations.
#[inline]
pub fn compute_dot_product(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    #[cfg(target_arch = "aarch64")]
    {
        unsafe { dot_product_neon(a, b) }
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            unsafe { dot_product_avx2(a, b) }
        } else {
            dot_product_scalar(a, b)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        dot_product_scalar(a, b)
    }
}

/// Computes L2 squared distance using SIMD when available.
#[inline]
pub fn compute_l2_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    #[cfg(target_arch = "aarch64")]
    {
        unsafe { l2_distance_neon(a, b) }
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            unsafe { l2_distance_avx2(a, b) }
        } else {
            l2_distance_scalar(a, b)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        l2_distance_scalar(a, b)
    }
}

/// Batch L2 distance computation with SIMD acceleration.
///
/// Returns distances from query to each vector in the batch.
///
/// # Performance
///
/// Uses SIMD intrinsics for the inner distance computation,
/// making this efficient for high-dimensional vectors.
pub fn batch_l2_distances(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| compute_l2_distance(query, v))
        .collect()
}

/// Batch dot product computation with SIMD acceleration.
///
/// Returns dot products from query to each vector in the batch.
pub fn batch_dot_products(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| compute_dot_product(query, v))
        .collect()
}

/// Batch L2 distances using DistanceComputer (for API compatibility).
///
/// This is an alternative API that uses `DistanceComputer` internally.
pub fn batch_l2_distances_with_computer(
    computer: &DistanceComputer,
    query: &[f32],
    vectors: &[Vec<f32>],
) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| computer.l2_distance(query, v))
        .collect()
}

/// Batch dot products using DistanceComputer (for API compatibility).
pub fn batch_dot_products_with_computer(
    computer: &DistanceComputer,
    query: &[f32],
    vectors: &[Vec<f32>],
) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| computer.dot_product(query, v))
        .collect()
}

// ============================================================================
// Parallel batch operations using rayon
// ============================================================================

/// Threshold for using parallel computation (vectors above this count use rayon)
pub const PARALLEL_THRESHOLD: usize = 1000;

/// Parallel batch L2 distance computation using rayon.
///
/// Returns distances from query to each vector in the batch.
/// Uses parallel iteration when the number of vectors exceeds PARALLEL_THRESHOLD.
///
/// # Performance
///
/// - For small batches (< 1000 vectors): uses sequential iteration
/// - For large batches (>= 1000 vectors): uses rayon parallel iteration
pub fn parallel_batch_l2_distances(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    if vectors.len() < PARALLEL_THRESHOLD {
        // Small batch: use sequential computation
        batch_l2_distances(query, vectors)
    } else {
        // Large batch: use parallel computation
        vectors
            .par_iter()
            .map(|v| compute_l2_distance(query, v))
            .collect()
    }
}

/// Parallel batch dot product computation using rayon.
///
/// Returns dot products from query to each vector in the batch.
/// Uses parallel iteration when the number of vectors exceeds PARALLEL_THRESHOLD.
pub fn parallel_batch_dot_products(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    if vectors.len() < PARALLEL_THRESHOLD {
        // Small batch: use sequential computation
        batch_dot_products(query, vectors)
    } else {
        // Large batch: use parallel computation
        vectors
            .par_iter()
            .map(|v| compute_dot_product(query, v))
            .collect()
    }
}

/// Parallel batch L2 distances using DistanceComputer.
///
/// This is an alternative API that uses `DistanceComputer` internally
/// with rayon parallelization for large batches.
pub fn parallel_batch_l2_distances_with_computer(
    computer: &DistanceComputer,
    query: &[f32],
    vectors: &[Vec<f32>],
) -> Vec<f32> {
    if vectors.len() < PARALLEL_THRESHOLD {
        // Small batch: use sequential computation
        batch_l2_distances_with_computer(computer, query, vectors)
    } else {
        // Large batch: use parallel computation
        vectors
            .par_iter()
            .map(|v| computer.l2_distance(query, v))
            .collect()
    }
}

/// Parallel batch dot products using DistanceComputer.
///
/// This is an alternative API that uses `DistanceComputer` internally
/// with rayon parallelization for large batches.
pub fn parallel_batch_dot_products_with_computer(
    computer: &DistanceComputer,
    query: &[f32],
    vectors: &[Vec<f32>],
) -> Vec<f32> {
    if vectors.len() < PARALLEL_THRESHOLD {
        // Small batch: use sequential computation
        batch_dot_products_with_computer(computer, query, vectors)
    } else {
        // Large batch: use parallel computation
        vectors
            .par_iter()
            .map(|v| computer.dot_product(query, v))
            .collect()
    }
}

// ============================================================================
// Top-K implementations
// ============================================================================

/// Internal helper for heap-based top-k
/// Wrapper to make BinaryHeap a min-heap for f32
struct MinFloat {
    idx: usize,
    val: f32,
}

impl PartialEq for MinFloat {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl Eq for MinFloat {}

impl PartialOrd for MinFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other.val.partial_cmp(&self.val).unwrap_or(Ordering::Equal)
    }
}

/// Wrapper to make BinaryHeap a max-heap for f32
struct MaxFloat {
    idx: usize,
    val: f32,
}

impl PartialEq for MaxFloat {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl Eq for MaxFloat {}

impl PartialOrd for MaxFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MaxFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap behavior
        self.val.partial_cmp(&other.val).unwrap_or(Ordering::Equal)
    }
}

/// Find top-k smallest values and their indices using a max-heap.
///
/// # Algorithm
///
/// Uses a max-heap of size k, achieving O(n log k) time complexity
/// instead of O(n log n) for full sorting.
///
/// # Performance
///
/// For k << n, this is significantly faster than sorting the entire array.
pub fn top_k_smallest(values: &[f32], k: usize) -> Vec<(usize, f32)> {
    if k == 0 || values.is_empty() {
        return Vec::new();
    }

    let k = k.min(values.len());

    // Use max-heap of size k - keeps k smallest elements
    // When we have > k elements, pop the largest (the one we don't want)
    let mut heap: BinaryHeap<MaxFloat> = BinaryHeap::with_capacity(k + 1);

    for (idx, &val) in values.iter().enumerate() {
        heap.push(MaxFloat { idx, val });
        if heap.len() > k {
            heap.pop();
        }
    }

    let mut result: Vec<_> = heap.into_iter().map(|mf| (mf.idx, mf.val)).collect();
    result.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    result
}

/// Find top-k largest values and their indices using a min-heap.
///
/// # Algorithm
///
/// Uses a min-heap of size k, achieving O(n log k) time complexity
/// instead of O(n log n) for full sorting.
pub fn top_k_largest(values: &[f32], k: usize) -> Vec<(usize, f32)> {
    if k == 0 || values.is_empty() {
        return Vec::new();
    }

    let k = k.min(values.len());

    // Use min-heap of size k - keeps k largest elements
    // When we have > k elements, pop the smallest (the one we don't want)
    let mut heap: BinaryHeap<MinFloat> = BinaryHeap::with_capacity(k + 1);

    for (idx, &val) in values.iter().enumerate() {
        heap.push(MinFloat { idx, val });
        if heap.len() > k {
            heap.pop();
        }
    }

    let mut result: Vec<_> = heap.into_iter().map(|mf| (mf.idx, mf.val)).collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_l2_distances() {
        let query = vec![1.0, 0.0];
        let vectors = vec![
            vec![1.0, 0.0], // distance 0
            vec![0.0, 1.0], // distance 2
            vec![1.0, 1.0], // distance 1
        ];
        let distances = batch_l2_distances(&query, &vectors);
        assert!((distances[0] - 0.0).abs() < 1e-6);
        assert!((distances[1] - 2.0).abs() < 1e-6);
        assert!((distances[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_batch_dot_products() {
        let query = vec![1.0, 2.0];
        let vectors = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let dots = batch_dot_products(&query, &vectors);
        assert!((dots[0] - 1.0).abs() < 1e-6);
        assert!((dots[1] - 2.0).abs() < 1e-6);
        assert!((dots[2] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_k_smallest() {
        let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        let top3 = top_k_smallest(&values, 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0], (1, 1.0));
        assert_eq!(top3[1], (3, 2.0));
        assert_eq!(top3[2], (2, 3.0));
    }

    #[test]
    fn test_top_k_largest() {
        let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        let top3 = top_k_largest(&values, 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0], (0, 5.0));
        assert_eq!(top3[1], (4, 4.0));
        assert_eq!(top3[2], (2, 3.0));
    }

    #[test]
    fn test_top_k_smallest_empty() {
        let values: Vec<f32> = vec![];
        let result = top_k_smallest(&values, 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_top_k_smallest_k_larger_than_n() {
        let values = vec![1.0, 2.0];
        let result = top_k_smallest(&values, 5);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_top_k_zero() {
        let values = vec![1.0, 2.0, 3.0];
        let result = top_k_smallest(&values, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_dot_product_simd() {
        // Test with vector length that exercises SIMD
        let a: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
        let b: Vec<f32> = (0..64).map(|i| i as f32 * 0.2).collect();

        let expected: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let result = compute_dot_product(&a, &b);

        // SIMD FMA may have slightly different rounding than scalar
        // Use relative error for robustness
        let abs_diff = (result - expected).abs();
        let rel_diff = if expected.abs() > 1e-6 {
            abs_diff / expected.abs()
        } else {
            abs_diff
        };
        assert!(
            rel_diff < 1e-3 || abs_diff < 1e-3,
            "expected: {}, result: {}",
            expected,
            result
        );
    }

    #[test]
    fn test_compute_l2_distance_simd() {
        let a: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
        let b: Vec<f32> = (0..64).map(|i| i as f32 * 0.2).collect();

        let expected: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
        let result = compute_l2_distance(&a, &b);

        // SIMD FMA may have slightly different rounding than scalar
        let abs_diff = (result - expected).abs();
        let rel_diff = if expected.abs() > 1e-6 {
            abs_diff / expected.abs()
        } else {
            abs_diff
        };
        assert!(
            rel_diff < 1e-3 || abs_diff < 1e-3,
            "expected: {}, result: {}",
            expected,
            result
        );
    }
}
