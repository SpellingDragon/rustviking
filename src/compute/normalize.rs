//! Vector normalization utilities

/// Normalize a vector to unit length (L2 norm)
pub fn l2_normalize(vector: &[f32]) -> Vec<f32> {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return vector.to_vec();
    }
    vector.iter().map(|x| x / norm).collect()
}

/// Normalize a vector in-place
pub fn l2_normalize_inplace(vector: &mut [f32]) {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }
    for x in vector.iter_mut() {
        *x /= norm;
    }
}

/// Compute L2 norm
pub fn l2_norm(vector: &[f32]) -> f32 {
    vector.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let v = vec![3.0, 4.0];
        let n = l2_normalize(&v);
        assert!((n[0] - 0.6).abs() < 1e-6);
        assert!((n[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_unit() {
        let v = vec![1.0, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert!((n[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero() {
        let v = vec![0.0, 0.0, 0.0];
        let n = l2_normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_l2_norm() {
        let v = vec![3.0, 4.0];
        assert!((l2_norm(&v) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_inplace() {
        let mut v = vec![3.0, 4.0];
        l2_normalize_inplace(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }
}
