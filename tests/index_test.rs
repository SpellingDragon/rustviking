//! Vector Index Integration Tests
//!
//! End-to-end tests for vector indexing operations.

use rustviking::index::{
    VectorIndex, IvfPqIndex, IvfPqParams, HnswIndex, HnswParams,
    MetricType, LayeredIndex, LEVEL_L0, LEVEL_L1, LEVEL_L2,
    Bitmap,
};
use std::sync::Arc;

#[test]
fn test_ivf_pq_end_to_end() {
    let params = IvfPqParams {
        num_partitions: 4,
        num_sub_vectors: 2,
        pq_bits: 8,
        metric: MetricType::L2,
    };
    let index = IvfPqIndex::new(params, 4);
    
    // Generate training data
    let training_data: Vec<Vec<f32>> = (0..20).map(|i| {
        vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32]
    }).collect();
    index.train(&training_data).unwrap();
    
    // Insert vectors
    for (i, v) in training_data.iter().enumerate() {
        index.insert(i as u64, v, 2).unwrap();
    }
    assert_eq!(index.count(), 20);
    
    // Search
    let query = vec![5.0, 10.0, 15.0, 20.0];
    let results = index.search(&query, 5, None).unwrap();
    assert!(!results.is_empty());
    assert!(results.len() <= 5);
    
    // The closest result should be id=5 (exact match)
    assert_eq!(results[0].id, 5);
    assert!(results[0].score < 0.001); // near-zero distance
}

#[test]
fn test_hnsw_end_to_end() {
    let params = HnswParams {
        m: 8,
        ef_construction: 32,
        ef_search: 16,
        metric: MetricType::L2,
    };
    let index = HnswIndex::new(params, 3);
    
    // Insert some vectors
    index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0, 0.0], 2).unwrap();
    index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap();
    index.insert(4, &[1.0, 1.0, 0.0], 2).unwrap();
    index.insert(5, &[0.5, 0.5, 0.5], 2).unwrap();
    
    // Search for [1,0,0] - should find id=1 first
    let results = index.search(&[1.0, 0.0, 0.0], 3, None).unwrap();
    assert_eq!(results[0].id, 1);
    assert!(results[0].score < 0.001);
}

#[test]
fn test_layered_index() {
    let params = IvfPqParams {
        num_partitions: 2,
        num_sub_vectors: 1,
        pq_bits: 8,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfPqIndex::new(params, 3));
    let layered = LayeredIndex::new(inner);
    
    // Insert vectors at different levels
    layered.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    layered.insert(2, &[0.0, 1.0, 0.0], LEVEL_L1).unwrap();
    layered.insert(3, &[0.0, 0.0, 1.0], LEVEL_L2).unwrap();
    layered.insert(4, &[1.0, 1.0, 0.0], LEVEL_L2).unwrap();
    
    assert_eq!(layered.count(), 4);
    
    // Search only L0
    let l0_results = layered.search_abstract(&[1.0, 0.0, 0.0], 10).unwrap();
    assert!(l0_results.iter().all(|r| r.level == LEVEL_L0));
    
    // Search only L2
    let l2_results = layered.search_detail(&[0.0, 0.0, 1.0], 10).unwrap();
    assert!(l2_results.iter().all(|r| r.level == LEVEL_L2));
}

#[test]
fn test_bitmap_complex_operations() {
    let users_premium = Bitmap::from_ids(&[1, 2, 5, 8, 10]);
    let users_active = Bitmap::from_ids(&[2, 3, 5, 7, 8, 11]);
    let users_new = Bitmap::from_ids(&[7, 8, 10, 11, 12]);
    
    // Premium AND active
    let premium_active = users_premium.intersection(&users_active);
    assert_eq!(premium_active.to_vec(), vec![2, 5, 8]);
    
    // Active OR new
    let active_or_new = users_active.union(&users_new);
    assert_eq!(active_or_new.cardinality(), 8);
    
    // Premium but not new
    let premium_not_new = users_premium.difference(&users_new);
    assert_eq!(premium_not_new.to_vec(), vec![1, 2, 5]);
}
