//! Vector Index Integration Tests
//!
//! End-to-end tests for vector indexing operations.

use rustviking::index::{
    Bitmap, HnswIndex, HnswIndexPersister, HnswParams, IvfIndex, IvfIndexPersister, IvfParams,
    LayeredIndex, MetricType, VectorIndex, LEVEL_L0, LEVEL_L1, LEVEL_L2,
};
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_ivf_end_to_end() {
    let params = IvfParams {
        num_partitions: 4,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 4);

    // Generate training data
    let training_data: Vec<Vec<f32>> = (0..20)
        .map(|i| vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32])
        .collect();
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
        m: 16,
        ef_construction: 200,
        ef_search: 50,
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
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfIndex::new(params, 3));
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

// ========================================
// IVF Persistence Tests
// ========================================

#[test]
fn test_ivf_persist_and_restore() {
    let temp_dir = TempDir::new().unwrap();

    // Create and populate index
    let params = IvfParams {
        num_partitions: 4,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 4);

    // Training data
    let vectors: Vec<Vec<f32>> = (0..20)
        .map(|i| vec![i as f32, (i * 2) as f32, (i * 3) as f32, (i * 4) as f32])
        .collect();
    index.train(&vectors).unwrap();

    // Insert vectors with different levels
    for (i, v) in vectors.iter().enumerate() {
        let level = (i % 3) as u8; // L0, L1, L2
        index.insert(i as u64, v, level).unwrap();
    }

    let original_count = index.count();

    // Persist
    let persister = IvfIndexPersister::new(temp_dir.path()).unwrap();
    persister.persist_index(&index).unwrap();

    // Restore
    let restored = persister.restore_index().unwrap();

    // Verify count
    assert_eq!(restored.count(), original_count);

    // Verify search works with same results
    let query = vec![5.0, 10.0, 15.0, 20.0];
    let original_results = index.search(&query, 5, None).unwrap();
    let restored_results = restored.search(&query, 5, None).unwrap();

    assert_eq!(original_results.len(), restored_results.len());
    // First result should be the same
    assert_eq!(original_results[0].id, restored_results[0].id);
}

#[test]
fn test_ivf_persist_level_filter_after_restore() {
    let temp_dir = TempDir::new().unwrap();

    let params = IvfParams {
        num_partitions: 4,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 3);

    // Train
    let train_data: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32, 0.0, 0.0]).collect();
    index.train(&train_data).unwrap();

    // Insert at different levels
    index.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    index.insert(2, &[2.0, 0.0, 0.0], LEVEL_L1).unwrap();
    index.insert(3, &[3.0, 0.0, 0.0], LEVEL_L2).unwrap();

    // Persist and restore
    let persister = IvfIndexPersister::new(temp_dir.path()).unwrap();
    persister.persist_index(&index).unwrap();
    let restored = persister.restore_index().unwrap();

    // Search with level filter
    let results = restored.search(&[1.0, 0.0, 0.0], 10, Some(LEVEL_L0)).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].level, LEVEL_L0);
}

// ========================================
// HNSW Persistence Tests
// ========================================

#[test]
fn test_hnsw_persist_and_restore() {
    let temp_dir = TempDir::new().unwrap();

    let params = HnswParams {
        m: 16,
        ef_construction: 200,
        ef_search: 50,
        metric: MetricType::L2,
    };
    let index = HnswIndex::new(params, 3);

    // Insert vectors
    index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0, 0.0], 1).unwrap();
    index.insert(3, &[0.0, 0.0, 1.0], 0).unwrap();
    index.insert(4, &[1.0, 1.0, 1.0], 2).unwrap();

    let original_count = index.count();

    // Persist
    let persister = HnswIndexPersister::new(temp_dir.path()).unwrap();
    persister.persist_index(&index).unwrap();

    // Restore
    let restored = persister.restore_index().unwrap();

    // Verify
    assert_eq!(restored.count(), original_count);
    assert_eq!(restored.dimension(), 3);

    // Verify search works
    let results = restored.search(&[1.0, 0.0, 0.0], 2, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, 1);
}

#[test]
fn test_hnsw_persist_level_filter_after_restore() {
    let temp_dir = TempDir::new().unwrap();

    let params = HnswParams::default();
    let index = HnswIndex::new(params, 3);

    index.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    index.insert(2, &[0.0, 1.0, 0.0], LEVEL_L1).unwrap();
    index.insert(3, &[0.0, 0.0, 1.0], LEVEL_L2).unwrap();

    // Persist and restore
    let persister = HnswIndexPersister::new(temp_dir.path()).unwrap();
    persister.persist_index(&index).unwrap();
    let restored = persister.restore_index().unwrap();

    // Verify level filter
    let results = restored.search(&[1.0, 0.0, 0.0], 10, Some(LEVEL_L0)).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].level, LEVEL_L0);
}

// ========================================
// Large Scale Insertion Tests
// ========================================

#[test]
fn test_ivf_large_scale_insertion() {
    let params = IvfParams {
        num_partitions: 16,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 16);

    // Generate and insert 1000+ vectors
    let vectors: Vec<Vec<f32>> = (0..1000)
        .map(|i| (0..16).map(|j| ((i * j) % 100) as f32 / 100.0).collect())
        .collect();

    // Train first
    index.train(&vectors).unwrap();

    // Insert all
    for (i, v) in vectors.iter().enumerate() {
        index.insert(i as u64, v, 2).unwrap();
    }

    assert_eq!(index.count(), 1000);

    // Search
    let query: Vec<f32> = (0..16).map(|i| (i % 10) as f32 / 10.0).collect();
    let results = index.search(&query, 10, None).unwrap();
    assert!(!results.is_empty());
    assert!(results.len() <= 10);
}

#[test]
fn test_hnsw_large_scale_insertion() {
    let params = HnswParams {
        m: 16,
        ef_construction: 200,
        ef_search: 100,
        metric: MetricType::L2,
    };
    let index = HnswIndex::new(params, 32);

    // Insert 1000+ vectors
    for i in 0..1000 {
        let vector: Vec<f32> = (0..32).map(|j| ((i * j) % 100) as f32 / 100.0).collect();
        index.insert(i, &vector, 2).unwrap();
    }

    assert_eq!(index.count(), 1000);

    // Search for zero vector
    let query: Vec<f32> = vec![0.0; 32];
    let results = index.search(&query, 10, None).unwrap();
    assert!(!results.is_empty());
}

// ========================================
// Delete and Search Correctness Tests
// ========================================

#[test]
fn test_ivf_delete_and_search() {
    let params = IvfParams {
        num_partitions: 2,  // Use fewer partitions for better coverage
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 3);

    // Train
    let train_data: Vec<Vec<f32>> = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
        vec![1.0, 1.0, 0.0],
    ];
    index.train(&train_data).unwrap();

    // Insert vectors
    index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0, 0.0], 2).unwrap();
    index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap();

    // Search before delete - just verify search works
    let results_before = index.search(&[1.0, 0.0, 0.0], 3, None).unwrap();
    assert!(!results_before.is_empty(), "Search should return results");

    // Delete id=1
    index.delete(1).unwrap();
    assert_eq!(index.count(), 2);

    // Search after delete
    let results_after = index.search(&[1.0, 0.0, 0.0], 3, None).unwrap();

    // Verify deleted id is not in results
    assert!(
        !results_after.iter().any(|r| r.id == 1),
        "Deleted id should not appear in search results"
    );
}

#[test]
fn test_hnsw_delete_and_search() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 3);

    index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0, 0.0], 2).unwrap();
    index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap();

    // Delete id=1
    index.delete(1).unwrap();
    assert_eq!(index.count(), 2);

    // Verify deleted id is not in results
    let results = index.search(&[1.0, 0.0, 0.0], 3, None).unwrap();
    assert!(
        !results.iter().any(|r| r.id == 1),
        "Deleted id should not appear in search results"
    );
}

#[test]
fn test_delete_nonexistent_vector() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 2);

    // Try to delete non-existent vector
    let result = index.delete(999);
    assert!(result.is_err(), "Deleting non-existent vector should fail");
}

// ========================================
// Boundary Condition Tests
// ========================================

#[test]
fn test_ivf_zero_vector() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 3);

    // Train with some data
    let train_data: Vec<Vec<f32>> = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
    ];
    index.train(&train_data).unwrap();

    // Insert zero vector
    index.insert(0, &[0.0, 0.0, 0.0], 2).unwrap();

    // Search with zero vector
    let results = index.search(&[0.0, 0.0, 0.0], 1, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, 0);
}

#[test]
fn test_hnsw_zero_vector() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 3);

    // Insert zero vector
    index.insert(0, &[0.0, 0.0, 0.0], 2).unwrap();

    // Search with zero vector
    let results = index.search(&[0.0, 0.0, 0.0], 1, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, 0);
}

#[test]
fn test_ivf_duplicate_id() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 2);

    index.insert(1, &[1.0, 0.0], 2).unwrap();

    // IVF allows duplicate IDs (it just appends)
    // Let's verify the count increases
    index.insert(1, &[0.0, 1.0], 2).unwrap();
    // Count should be 2 since IVF doesn't enforce unique IDs
    assert_eq!(index.count(), 2);
}

#[test]
fn test_hnsw_duplicate_id_error() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 2);

    index.insert(1, &[1.0, 0.0], 2).unwrap();

    // HNSW should reject duplicate IDs
    let result = index.insert(1, &[0.0, 1.0], 2);
    assert!(result.is_err(), "HNSW should reject duplicate ID");
}

#[test]
fn test_ivf_dimension_mismatch() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 4);

    // Wrong dimension
    let result = index.insert(1, &[1.0, 0.0], 2);
    assert!(
        result.is_err(),
        "Should fail with wrong dimension"
    );
}

#[test]
fn test_hnsw_dimension_mismatch() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 4);

    // Wrong dimension
    let result = index.insert(1, &[1.0, 0.0], 2);
    assert!(
        result.is_err(),
        "Should fail with wrong dimension"
    );
}

#[test]
fn test_ivf_k_greater_than_total() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 2);

    // Train
    let train_data: Vec<Vec<f32>> = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
    index.train(&train_data).unwrap();

    index.insert(1, &[1.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0], 2).unwrap();

    // k > total count
    let results = index.search(&[0.5, 0.5], 100, None).unwrap();
    assert!(results.len() <= 2, "Should return at most total count");
}

#[test]
fn test_hnsw_k_greater_than_total() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 2);

    index.insert(1, &[1.0, 0.0], 2).unwrap();
    index.insert(2, &[0.0, 1.0], 2).unwrap();

    // k > total count
    let results = index.search(&[0.5, 0.5], 100, None).unwrap();
    assert!(results.len() <= 2, "Should return at most total count");
}

#[test]
fn test_ivf_search_empty_index() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let index = IvfIndex::new(params, 2);

    // Train with dummy data
    let train_data: Vec<Vec<f32>> = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
    index.train(&train_data).unwrap();

    // Search empty index
    let results = index.search(&[0.5, 0.5], 5, None).unwrap();
    assert!(results.is_empty(), "Empty index should return empty results");
}

#[test]
fn test_hnsw_search_empty_index() {
    let params = HnswParams::default();
    let index = HnswIndex::new(params, 2);

    // Search empty index
    let results = index.search(&[0.5, 0.5], 5, None).unwrap();
    assert!(results.is_empty(), "Empty index should return empty results");
}

// ========================================
// L0/L1/L2 Layered Filtering Tests
// ========================================

#[test]
fn test_layered_index_l0_only() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfIndex::new(params, 3));
    let layered = LayeredIndex::new(inner.clone());

    // Train
    let train_data: Vec<Vec<f32>> = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    inner.train(&train_data).unwrap();

    // Insert at different levels
    layered.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    layered.insert(2, &[0.5, 0.5, 0.0], LEVEL_L1).unwrap();
    layered.insert(3, &[0.0, 0.0, 1.0], LEVEL_L2).unwrap();

    // Search L0 only
    let results = layered.search_abstract(&[1.0, 0.0, 0.0], 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].level, LEVEL_L0);
}

#[test]
fn test_layered_index_l1_only() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfIndex::new(params, 3));
    let layered = LayeredIndex::new(inner.clone());

    // Train
    let train_data: Vec<Vec<f32>> = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    inner.train(&train_data).unwrap();

    layered.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    layered.insert(2, &[0.5, 0.5, 0.0], LEVEL_L1).unwrap();
    layered.insert(3, &[0.0, 0.0, 1.0], LEVEL_L2).unwrap();

    // Search L1 only
    let results = layered.search_overview(&[0.5, 0.5, 0.0], 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 2);
    assert_eq!(results[0].level, LEVEL_L1);
}

#[test]
fn test_layered_index_l2_only() {
    let params = IvfParams {
        num_partitions: 2,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfIndex::new(params, 3));
    let layered = LayeredIndex::new(inner.clone());

    // Train
    let train_data: Vec<Vec<f32>> = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    inner.train(&train_data).unwrap();

    layered.insert(1, &[1.0, 0.0, 0.0], LEVEL_L0).unwrap();
    layered.insert(2, &[0.5, 0.5, 0.0], LEVEL_L1).unwrap();
    layered.insert(3, &[0.0, 0.0, 1.0], LEVEL_L2).unwrap();

    // Search L2 only
    let results = layered.search_detail(&[0.0, 0.0, 1.0], 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 3);
    assert_eq!(results[0].level, LEVEL_L2);
}

#[test]
fn test_layered_index_hierarchical_search() {
    let params = IvfParams {
        num_partitions: 4,
        metric: MetricType::L2,
    };
    let inner = Arc::new(IvfIndex::new(params, 3));
    let layered = LayeredIndex::new(inner.clone());

    // Train
    let train_data: Vec<Vec<f32>> = (0..10)
        .map(|i| vec![i as f32 / 10.0, 0.0, 0.0])
        .collect();
    inner.train(&train_data).unwrap();

    // Insert vectors at all levels
    for i in 0..3 {
        layered.insert(i, &[i as f32 / 10.0, 0.0, 0.0], i as u8).unwrap();
    }
    for i in 3..6 {
        layered.insert(i, &[i as f32 / 10.0, 0.0, 0.0], LEVEL_L1).unwrap();
    }
    for i in 6..10 {
        layered.insert(i, &[i as f32 / 10.0, 0.0, 0.0], LEVEL_L2).unwrap();
    }

    // Hierarchical search should return results
    let results = layered.hierarchical_search(&[0.5, 0.0, 0.0], 5).unwrap();
    assert!(!results.is_empty());
}

// ========================================
// Bitmap Index Tests
// ========================================

#[test]
fn test_bitmap_from_ids() {
    let bitmap = Bitmap::from_ids(&[1, 2, 3, 5, 8, 13]);

    assert!(bitmap.contains(1));
    assert!(bitmap.contains(5));
    assert!(bitmap.contains(13));
    assert!(!bitmap.contains(4));
    assert!(!bitmap.contains(100));
}

#[test]
fn test_bitmap_add_and_remove() {
    let mut bitmap = Bitmap::new();

    bitmap.add(10);
    bitmap.add(20);
    bitmap.add(30);

    assert!(bitmap.contains(10));
    assert!(bitmap.contains(20));
    assert!(bitmap.contains(30));
    assert_eq!(bitmap.cardinality(), 3);

    bitmap.remove(20);
    assert!(!bitmap.contains(20));
    assert_eq!(bitmap.cardinality(), 2);
}

#[test]
fn test_bitmap_intersection() {
    let a = Bitmap::from_ids(&[1, 2, 3, 4, 5]);
    let b = Bitmap::from_ids(&[3, 4, 5, 6, 7]);

    let c = a.intersection(&b);

    assert_eq!(c.cardinality(), 3);
    assert!(c.contains(3));
    assert!(c.contains(4));
    assert!(c.contains(5));
    assert!(!c.contains(1));
    assert!(!c.contains(7));
}

#[test]
fn test_bitmap_union() {
    let a = Bitmap::from_ids(&[1, 2, 3]);
    let b = Bitmap::from_ids(&[3, 4, 5]);

    let c = a.union(&b);

    assert_eq!(c.cardinality(), 5);
    assert!(c.contains(1));
    assert!(c.contains(2));
    assert!(c.contains(3));
    assert!(c.contains(4));
    assert!(c.contains(5));
}

#[test]
fn test_bitmap_difference() {
    let a = Bitmap::from_ids(&[1, 2, 3, 4, 5]);
    let b = Bitmap::from_ids(&[3, 4, 5, 6, 7]);

    let c = a.difference(&b);

    assert_eq!(c.cardinality(), 2);
    assert!(c.contains(1));
    assert!(c.contains(2));
    assert!(!c.contains(3));
}

#[test]
fn test_bitmap_add_range() {
    let mut bitmap = Bitmap::new();
    bitmap.add_range(10, 20);

    assert_eq!(bitmap.cardinality(), 10);
    for i in 10..20 {
        assert!(bitmap.contains(i), "Should contain {}", i);
    }
    assert!(!bitmap.contains(9));
    assert!(!bitmap.contains(20));
}

#[test]
fn test_bitmap_cardinality_and_empty() {
    let mut bitmap = Bitmap::new();
    assert!(bitmap.is_empty());
    assert_eq!(bitmap.cardinality(), 0);

    bitmap.add(1);
    assert!(!bitmap.is_empty());
    assert_eq!(bitmap.cardinality(), 1);

    bitmap.add_range(10, 15);
    assert_eq!(bitmap.cardinality(), 6);
}

#[test]
fn test_bitmap_to_vec() {
    let bitmap = Bitmap::from_ids(&[5, 2, 8, 1, 9]);
    let vec = bitmap.to_vec();

    // Should be sorted
    assert_eq!(vec, vec![1, 2, 5, 8, 9]);
}

#[test]
fn test_bitmap_serialize_deserialize() {
    let original = Bitmap::from_ids(&[1, 5, 10, 100, 1000]);
    let serialized = original.serialize();
    let restored = Bitmap::deserialize(&serialized).unwrap();

    assert_eq!(original.cardinality(), restored.cardinality());
    assert_eq!(original.to_vec(), restored.to_vec());
}

#[test]
fn test_bitmap_large_ids() {
    // Bitmap only supports u32 IDs internally
    let mut bitmap = Bitmap::new();

    // u32::MAX should work
    bitmap.add(u32::MAX as u64);
    assert!(bitmap.contains(u32::MAX as u64));

    // IDs > u32::MAX are silently ignored
    bitmap.add((u32::MAX as u64) + 1);
    assert!(!bitmap.contains((u32::MAX as u64) + 1));
}

#[test]
fn test_bitmap_empty_operations() {
    let a = Bitmap::new();
    let b = Bitmap::from_ids(&[1, 2, 3]);

    // Empty intersect non-empty
    let c = a.intersection(&b);
    assert!(c.is_empty());

    // Empty union non-empty
    let d = a.union(&b);
    assert_eq!(d.cardinality(), 3);

    // Non-empty difference empty
    let e = b.difference(&a);
    assert_eq!(e.cardinality(), 3);
}
