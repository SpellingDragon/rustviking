//! Distance computation
//!
//! Implements L2, Cosine, and DotProduct distance metrics.

/// Distance computer with dimension info
pub struct DistanceComputer {
    dimension: usize,
}

impl DistanceComputer {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// L2 squared distance (no sqrt for performance)
    pub fn l2_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| {
                let diff = x - y;
                diff * diff
            })
            .sum()
    }

    /// Dot product (inner product)
    pub fn dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len());
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Cosine similarity (returns 1 - cosine_sim as distance)
    pub fn cosine_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot = self.dot_product(a, b);
        let norm_a = self.dot_product(a, a).sqrt();
        let norm_b = self.dot_product(b, b).sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0; // max distance for zero vectors
        }

        1.0 - (dot / (norm_a * norm_b))
    }

    /// Cosine similarity (not distance)
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        1.0 - self.cosine_distance(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_distance_identical() {
        let c = DistanceComputer::new(3);
        let v = vec![1.0, 2.0, 3.0];
        assert_eq!(c.l2_distance(&v, &v), 0.0);
    }

    #[test]
    fn test_l2_distance() {
        let c = DistanceComputer::new(3);
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((c.l2_distance(&a, &b) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let c = DistanceComputer::new(3);
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((c.dot_product(&a, &b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_identical() {
        let c = DistanceComputer::new(3);
        let v = vec![1.0, 2.0, 3.0];
        assert!(c.cosine_distance(&v, &v).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let c = DistanceComputer::new(2);
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((c.cosine_distance(&a, &b) - 1.0).abs() < 1e-6);
    }
}
