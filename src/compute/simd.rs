//! SIMD optimization utilities
//!
//! Provides optimized batch operations using auto-vectorization.

/// Batch L2 distance computation
/// Returns distances from query to each vector in the batch
pub fn batch_l2_distances(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| {
            query
                .iter()
                .zip(v.iter())
                .map(|(a, b)| {
                    let diff = a - b;
                    diff * diff
                })
                .sum()
        })
        .collect()
}

/// Batch dot product computation
pub fn batch_dot_products(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| query.iter().zip(v.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

/// Find top-k smallest values and their indices
pub fn top_k_smallest(values: &[f32], k: usize) -> Vec<(usize, f32)> {
    let mut indexed: Vec<(usize, f32)> = values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(k);
    indexed
}

/// Find top-k largest values and their indices
pub fn top_k_largest(values: &[f32], k: usize) -> Vec<(usize, f32)> {
    let mut indexed: Vec<(usize, f32)> = values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(k);
    indexed
}

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
}
