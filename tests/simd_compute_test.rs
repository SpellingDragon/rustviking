//! SIMD and Parallel Computing Integration Tests
//!
//! End-to-end tests for SIMD-accelerated and parallel batch operations.
//! Tests parallel_batch_l2_distances, parallel_batch_dot_products, top_k functions.

use rustviking::compute::distance::DistanceComputer;
use rustviking::compute::simd::{
    batch_dot_products, batch_l2_distances, compute_dot_product, compute_l2_distance,
    parallel_batch_dot_products, parallel_batch_dot_products_with_computer,
    parallel_batch_l2_distances, parallel_batch_l2_distances_with_computer, top_k_largest,
    top_k_smallest, PARALLEL_THRESHOLD,
};

// ============================================================================
// Parallel Batch L2 Distance Tests
// ============================================================================

#[test]
fn test_parallel_batch_l2_distances_basic() {
    let query = vec![1.0, 0.0, 0.0];
    let vectors = vec![
        vec![1.0, 0.0, 0.0], // distance 0
        vec![0.0, 1.0, 0.0], // distance 2
        vec![0.0, 0.0, 1.0], // distance 2
        vec![1.0, 1.0, 0.0], // distance 1
    ];

    let distances = parallel_batch_l2_distances(&query, &vectors);

    assert_eq!(distances.len(), 4);
    assert!((distances[0] - 0.0).abs() < 1e-6);
    assert!((distances[1] - 2.0).abs() < 1e-6);
    assert!((distances[2] - 2.0).abs() < 1e-6);
    assert!((distances[3] - 1.0).abs() < 1e-6);
}

#[test]
fn test_parallel_batch_l2_distances_consistency_with_sequential() {
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..100)
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel_distances = parallel_batch_l2_distances(&query, &vectors);
    let sequential_distances = batch_l2_distances(&query, &vectors);

    assert_eq!(parallel_distances.len(), sequential_distances.len());
    for (p, s) in parallel_distances.iter().zip(sequential_distances.iter()) {
        assert!(
            (p - s).abs() < 1e-5,
            "Parallel and sequential results differ: {} vs {}",
            p,
            s
        );
    }
}

#[test]
fn test_parallel_batch_l2_distances_large_scale() {
    let query: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
    let vectors: Vec<Vec<f32>> = (0..10000)
        .map(|i| (0..128).map(|j| ((i + j) % 100) as f32 * 0.01).collect())
        .collect();

    let distances = parallel_batch_l2_distances(&query, &vectors);

    assert_eq!(distances.len(), 10000);
    // All distances should be non-negative
    for d in &distances {
        assert!(*d >= 0.0, "L2 distance should be non-negative");
    }
}

#[test]
fn test_parallel_batch_l2_distances_with_computer() {
    let computer = DistanceComputer::new(64);
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..500)
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel_distances = parallel_batch_l2_distances_with_computer(&computer, &query, &vectors);
    let sequential_distances = batch_l2_distances(&query, &vectors);

    assert_eq!(parallel_distances.len(), sequential_distances.len());
    for (p, s) in parallel_distances.iter().zip(sequential_distances.iter()) {
        assert!((p - s).abs() < 1e-5);
    }
}

// ============================================================================
// Parallel Batch Dot Product Tests
// ============================================================================

#[test]
fn test_parallel_batch_dot_products_basic() {
    let query = vec![1.0, 2.0, 3.0];
    let vectors = vec![
        vec![1.0, 0.0, 0.0], // dot = 1
        vec![0.0, 1.0, 0.0], // dot = 2
        vec![0.0, 0.0, 1.0], // dot = 3
        vec![1.0, 1.0, 1.0], // dot = 6
    ];

    let dots = parallel_batch_dot_products(&query, &vectors);

    assert_eq!(dots.len(), 4);
    assert!((dots[0] - 1.0).abs() < 1e-6);
    assert!((dots[1] - 2.0).abs() < 1e-6);
    assert!((dots[2] - 3.0).abs() < 1e-6);
    assert!((dots[3] - 6.0).abs() < 1e-6);
}

#[test]
fn test_parallel_batch_dot_products_consistency_with_sequential() {
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..100)
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel_dots = parallel_batch_dot_products(&query, &vectors);
    let sequential_dots = batch_dot_products(&query, &vectors);

    assert_eq!(parallel_dots.len(), sequential_dots.len());
    for (p, s) in parallel_dots.iter().zip(sequential_dots.iter()) {
        assert!(
            (p - s).abs() < 1e-5,
            "Parallel and sequential results differ: {} vs {}",
            p,
            s
        );
    }
}

#[test]
fn test_parallel_batch_dot_products_large_scale() {
    let query: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
    let vectors: Vec<Vec<f32>> = (0..10000)
        .map(|i| (0..128).map(|j| ((i + j) % 100) as f32 * 0.01).collect())
        .collect();

    let dots = parallel_batch_dot_products(&query, &vectors);

    assert_eq!(dots.len(), 10000);
}

#[test]
fn test_parallel_batch_dot_products_with_computer() {
    let computer = DistanceComputer::new(64);
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..500)
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel_dots = parallel_batch_dot_products_with_computer(&computer, &query, &vectors);
    let sequential_dots = batch_dot_products(&query, &vectors);

    assert_eq!(parallel_dots.len(), sequential_dots.len());
    for (p, s) in parallel_dots.iter().zip(sequential_dots.iter()) {
        assert!((p - s).abs() < 1e-5);
    }
}

// ============================================================================
// Top-K Smallest Tests
// ============================================================================

#[test]
fn test_top_k_smallest_basic() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top3 = top_k_smallest(&values, 3);

    assert_eq!(top3.len(), 3);
    assert_eq!(top3[0].1, 1.0);
    assert_eq!(top3[1].1, 2.0);
    assert_eq!(top3[2].1, 3.0);
}

#[test]
fn test_top_k_smallest_with_indices() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top3 = top_k_smallest(&values, 3);

    assert_eq!(top3[0], (1, 1.0)); // index 1, value 1.0
    assert_eq!(top3[1], (3, 2.0)); // index 3, value 2.0
    assert_eq!(top3[2], (2, 3.0)); // index 2, value 3.0
}

#[test]
fn test_top_k_smallest_sorted_order() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top_k = top_k_smallest(&values, 5);

    // Should be sorted by value
    for i in 1..top_k.len() {
        assert!(top_k[i - 1].1 <= top_k[i].1);
    }
}

#[test]
fn test_top_k_smallest_k_larger_than_n() {
    let values = vec![1.0, 2.0, 3.0];
    let result = top_k_smallest(&values, 10);

    assert_eq!(result.len(), 3);
}

#[test]
fn test_top_k_smallest_k_zero() {
    let values = vec![1.0, 2.0, 3.0];
    let result = top_k_smallest(&values, 0);

    assert!(result.is_empty());
}

#[test]
fn test_top_k_smallest_empty() {
    let values: Vec<f32> = vec![];
    let result = top_k_smallest(&values, 5);

    assert!(result.is_empty());
}

#[test]
fn test_top_k_smallest_large_scale() {
    let values: Vec<f32> = (0..10000).map(|i| (i % 100) as f32).collect();
    let top100 = top_k_smallest(&values, 100);

    assert_eq!(top100.len(), 100);
    // All top 100 should be 0.0 since we have many zeros
    for (_, val) in &top100 {
        assert!(*val <= 1.0);
    }
}

#[test]
fn test_top_k_smallest_with_duplicates() {
    let values = vec![3.0, 1.0, 2.0, 1.0, 3.0, 2.0, 1.0];
    let top3 = top_k_smallest(&values, 3);

    assert_eq!(top3.len(), 3);
    // All should be 1.0 (the smallest value)
    for (_, val) in &top3 {
        assert!((val - 1.0).abs() < 1e-6);
    }
}

// ============================================================================
// Top-K Largest Tests
// ============================================================================

#[test]
fn test_top_k_largest_basic() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top3 = top_k_largest(&values, 3);

    assert_eq!(top3.len(), 3);
    assert_eq!(top3[0].1, 5.0);
    assert_eq!(top3[1].1, 4.0);
    assert_eq!(top3[2].1, 3.0);
}

#[test]
fn test_top_k_largest_with_indices() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top3 = top_k_largest(&values, 3);

    assert_eq!(top3[0], (0, 5.0)); // index 0, value 5.0
    assert_eq!(top3[1], (4, 4.0)); // index 4, value 4.0
    assert_eq!(top3[2], (2, 3.0)); // index 2, value 3.0
}

#[test]
fn test_top_k_largest_sorted_order() {
    let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
    let top_k = top_k_largest(&values, 5);

    // Should be sorted by value (descending)
    for i in 1..top_k.len() {
        assert!(top_k[i - 1].1 >= top_k[i].1);
    }
}

#[test]
fn test_top_k_largest_k_larger_than_n() {
    let values = vec![1.0, 2.0, 3.0];
    let result = top_k_largest(&values, 10);

    assert_eq!(result.len(), 3);
}

#[test]
fn test_top_k_largest_k_zero() {
    let values = vec![1.0, 2.0, 3.0];
    let result = top_k_largest(&values, 0);

    assert!(result.is_empty());
}

#[test]
fn test_top_k_largest_empty() {
    let values: Vec<f32> = vec![];
    let result = top_k_largest(&values, 5);

    assert!(result.is_empty());
}

#[test]
fn test_top_k_largest_large_scale() {
    let values: Vec<f32> = (0..10000).map(|i| (i % 100) as f32).collect();
    let top100 = top_k_largest(&values, 100);

    assert_eq!(top100.len(), 100);
    // All top 100 should be 99.0 (the largest value)
    for (_, val) in &top100 {
        assert!(*val >= 98.0);
    }
}

#[test]
fn test_top_k_largest_with_duplicates() {
    let values = vec![1.0, 3.0, 2.0, 3.0, 1.0, 2.0, 3.0];
    let top3 = top_k_largest(&values, 3);

    assert_eq!(top3.len(), 3);
    // All should be 3.0 (the largest value)
    for (_, val) in &top3 {
        assert!((val - 3.0).abs() < 1e-6);
    }
}

// ============================================================================
// SIMD Compute Functions Tests
// ============================================================================

#[test]
fn test_compute_dot_product_basic() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0, 6.0];

    let result = compute_dot_product(&a, &b);
    let expected = 1.0 * 4.0 + 2.0 * 5.0 + 3.0 * 6.0; // 32.0

    assert!((result - expected).abs() < 1e-6);
}

#[test]
fn test_compute_dot_product_simd_length() {
    // Test with vector length that exercises SIMD (multiple of 4 for NEON, 8 for AVX2)
    let a: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let b: Vec<f32> = (0..64).map(|i| i as f32 * 0.2).collect();

    let expected: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let result = compute_dot_product(&a, &b);

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

#[test]
fn test_compute_l2_distance_basic() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0, 6.0];

    let result = compute_l2_distance(&a, &b);
    let expected = 3.0f32.powi(2) + 3.0f32.powi(2) + 3.0f32.powi(2); // 27.0

    assert!((result - expected).abs() < 1e-6);
}

#[test]
fn test_compute_l2_distance_simd_length() {
    // Test with vector length that exercises SIMD
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

#[test]
fn test_compute_l2_distance_non_negative() {
    let a: Vec<f32> = (0..100).map(|i| i as f32).collect();
    let b: Vec<f32> = (0..100).map(|i| (100 - i) as f32).collect();

    let result = compute_l2_distance(&a, &b);
    assert!(result >= 0.0, "L2 distance should be non-negative");
}

// ============================================================================
// Batch Operations Tests
// ============================================================================

#[test]
fn test_batch_l2_distances_basic() {
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
fn test_batch_dot_products_basic() {
    let query = vec![1.0, 2.0];
    let vectors = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];

    let dots = batch_dot_products(&query, &vectors);

    assert!((dots[0] - 1.0).abs() < 1e-6);
    assert!((dots[1] - 2.0).abs() < 1e-6);
    assert!((dots[2] - 3.0).abs() < 1e-6);
}

// ============================================================================
// Parallel Threshold Tests
// ============================================================================

#[test]
fn test_parallel_threshold_value() {
    // Verify the threshold constant
    assert_eq!(PARALLEL_THRESHOLD, 1000);
}

#[test]
fn test_parallel_vs_sequential_small_batch() {
    // Small batch should use sequential computation
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..100) // Less than PARALLEL_THRESHOLD
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel = parallel_batch_l2_distances(&query, &vectors);
    let sequential = batch_l2_distances(&query, &vectors);

    assert_eq!(parallel.len(), sequential.len());
    for (p, s) in parallel.iter().zip(sequential.iter()) {
        assert!((p - s).abs() < 1e-5);
    }
}

#[test]
fn test_parallel_vs_sequential_large_batch() {
    // Large batch should use parallel computation
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let vectors: Vec<Vec<f32>> = (0..2000) // More than PARALLEL_THRESHOLD
        .map(|i| (0..64).map(|j| (i * j) as f32 * 0.01).collect())
        .collect();

    let parallel = parallel_batch_l2_distances(&query, &vectors);
    let sequential = batch_l2_distances(&query, &vectors);

    assert_eq!(parallel.len(), sequential.len());
    for (p, s) in parallel.iter().zip(sequential.iter()) {
        assert!((p - s).abs() < 1e-5);
    }
}

// ============================================================================
// Large Scale Parallel Search Tests
// ============================================================================

#[test]
fn test_large_scale_parallel_search_10000_vectors() {
    let query: Vec<f32> = (0..128).map(|i| i as f32 * 0.01).collect();
    let vectors: Vec<Vec<f32>> = (0..10000)
        .map(|i| (0..128).map(|j| ((i + j) % 100) as f32 * 0.01).collect())
        .collect();

    // Compute distances in parallel
    let distances = parallel_batch_l2_distances(&query, &vectors);

    // Find top 10 smallest distances
    let top10 = top_k_smallest(&distances, 10);

    assert_eq!(top10.len(), 10);

    // Verify top 10 are actually the smallest
    let mut sorted_distances = distances.clone();
    sorted_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for i in 0..10 {
        assert!(
            (top10[i].1 - sorted_distances[i]).abs() < 1e-5,
            "Top {} mismatch: got {}, expected {}",
            i,
            top10[i].1,
            sorted_distances[i]
        );
    }
}

#[test]
fn test_large_scale_parallel_search_50000_vectors() {
    let query: Vec<f32> = (0..64).map(|i| i as f32 * 0.01).collect();
    let vectors: Vec<Vec<f32>> = (0..50000)
        .map(|i| (0..64).map(|j| ((i * j) % 100) as f32 * 0.01).collect())
        .collect();

    // Compute distances in parallel
    let distances = parallel_batch_l2_distances(&query, &vectors);
    assert_eq!(distances.len(), 50000);

    // Compute dot products in parallel
    let dots = parallel_batch_dot_products(&query, &vectors);
    assert_eq!(dots.len(), 50000);

    // Find top 100 smallest distances
    let top100 = top_k_smallest(&distances, 100);
    assert_eq!(top100.len(), 100);

    // Find top 100 largest dot products
    let top100_dots = top_k_largest(&dots, 100);
    assert_eq!(top100_dots.len(), 100);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_parallel_batch_empty_vectors() {
    let query = vec![1.0, 2.0, 3.0];
    let vectors: Vec<Vec<f32>> = vec![];

    let distances = parallel_batch_l2_distances(&query, &vectors);
    assert!(distances.is_empty());

    let dots = parallel_batch_dot_products(&query, &vectors);
    assert!(dots.is_empty());
}

#[test]
fn test_parallel_batch_single_vector() {
    let query = vec![1.0, 2.0, 3.0];
    let vectors = vec![vec![4.0, 5.0, 6.0]];

    let distances = parallel_batch_l2_distances(&query, &vectors);
    assert_eq!(distances.len(), 1);

    let dots = parallel_batch_dot_products(&query, &vectors);
    assert_eq!(dots.len(), 1);
}

#[test]
fn test_parallel_batch_zero_vectors() {
    let query = vec![1.0, 2.0, 3.0];
    let vectors = vec![vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0]];

    let distances = parallel_batch_l2_distances(&query, &vectors);
    assert_eq!(distances.len(), 2);
    // Distance from [1,2,3] to [0,0,0] = 14
    assert!((distances[0] - 14.0).abs() < 1e-6);
    assert!((distances[1] - 14.0).abs() < 1e-6);

    let dots = parallel_batch_dot_products(&query, &vectors);
    assert_eq!(dots.len(), 2);
    assert!((dots[0] - 0.0).abs() < 1e-6);
    assert!((dots[1] - 0.0).abs() < 1e-6);
}

#[test]
fn test_compute_functions_zero_vectors() {
    let a = vec![0.0; 64];
    let b = vec![0.0; 64];

    let dot = compute_dot_product(&a, &b);
    assert!((dot - 0.0).abs() < 1e-6);

    let l2 = compute_l2_distance(&a, &b);
    assert!((l2 - 0.0).abs() < 1e-6);
}

#[test]
fn test_compute_functions_orthogonal_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];

    let dot = compute_dot_product(&a, &b);
    assert!((dot - 0.0).abs() < 1e-6);

    let l2 = compute_l2_distance(&a, &b);
    assert!((l2 - 2.0).abs() < 1e-6);
}

#[test]
fn test_compute_functions_identical_vectors() {
    let a: Vec<f32> = (0..64).map(|i| i as f32 * 0.1).collect();
    let b = a.clone();

    let dot = compute_dot_product(&a, &b);
    let expected_dot: f32 = a.iter().map(|x| x * x).sum();
    assert!((dot - expected_dot).abs() < 1e-3);

    let l2 = compute_l2_distance(&a, &b);
    assert!((l2 - 0.0).abs() < 1e-5);
}

#[test]
fn test_top_k_with_negative_values() {
    let values = vec![-5.0, -1.0, -3.0, -2.0, -4.0];

    let top3_smallest = top_k_smallest(&values, 3);
    assert_eq!(top3_smallest[0].1, -5.0);
    assert_eq!(top3_smallest[1].1, -4.0);
    assert_eq!(top3_smallest[2].1, -3.0);

    let top3_largest = top_k_largest(&values, 3);
    assert_eq!(top3_largest[0].1, -1.0);
    assert_eq!(top3_largest[1].1, -2.0);
    assert_eq!(top3_largest[2].1, -3.0);
}

#[test]
fn test_top_k_with_mixed_values() {
    let values = vec![-5.0, 1.0, -3.0, 2.0, 0.0];

    let top3_smallest = top_k_smallest(&values, 3);
    assert_eq!(top3_smallest[0].1, -5.0);
    assert_eq!(top3_smallest[1].1, -3.0);
    assert_eq!(top3_smallest[2].1, 0.0);

    let top3_largest = top_k_largest(&values, 3);
    assert_eq!(top3_largest[0].1, 2.0);
    assert_eq!(top3_largest[1].1, 1.0);
    assert_eq!(top3_largest[2].1, 0.0);
}

#[test]
fn test_top_k_with_nan_values() {
    let values = vec![1.0, f32::NAN, 3.0, 2.0];

    // NaN handling - depends on implementation
    let top3 = top_k_smallest(&values, 3);
    assert_eq!(top3.len(), 3);
}

#[test]
fn test_top_k_with_infinity() {
    let values = vec![1.0, f32::INFINITY, 3.0, f32::NEG_INFINITY];

    let top3_smallest = top_k_smallest(&values, 3);
    assert_eq!(top3_smallest[0].1, f32::NEG_INFINITY);

    let top3_largest = top_k_largest(&values, 3);
    assert_eq!(top3_largest[0].1, f32::INFINITY);
}

#[test]
fn test_parallel_batch_different_dimensions() {
    for dim in [32, 64, 128, 256, 512, 768, 1024] {
        let query: Vec<f32> = (0..dim).map(|i| i as f32 * 0.01).collect();
        let vectors: Vec<Vec<f32>> = (0..100)
            .map(|i| (0..dim).map(|j| ((i + j) % 100) as f32 * 0.01).collect())
            .collect();

        let distances = parallel_batch_l2_distances(&query, &vectors);
        assert_eq!(distances.len(), 100);

        let dots = parallel_batch_dot_products(&query, &vectors);
        assert_eq!(dots.len(), 100);
    }
}
