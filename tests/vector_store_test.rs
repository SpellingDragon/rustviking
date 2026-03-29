//! Vector Store Integration Tests
//!
//! Tests for VectorStore trait and MemoryVectorStore implementation.

use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::*;
use serde_json::json;

// Helper function to create a test vector point
fn create_test_point(id: &str, vector: Vec<f32>, uri: &str) -> VectorPoint {
    let payload = json!({
        "id": id,
        "uri": uri,
        "context_type": "resource",
        "is_leaf": true,
        "level": 0,
        "created_at": "2024-01-01T00:00:00Z"
    });

    VectorPoint {
        id: id.to_string(),
        vector,
        sparse_vector: None,
        payload,
    }
}

// ============================================================================
// Collection Tests
// ============================================================================

#[test]
fn test_create_collection() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("test_collection", 128, params)
        .unwrap();
    let info = store.collection_info("test_collection").unwrap();

    assert_eq!(info.name, "test_collection");
    assert_eq!(info.dimension, 128);
    assert_eq!(info.count, 0);
}

#[test]
fn test_create_duplicate_collection() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("test", 64, params.clone()).unwrap();
    let result = store.create_collection("test", 64, params);

    assert!(result.is_err());
}

#[test]
fn test_collection_info() {
    let store = MemoryVectorStore::new();
    let params = IndexParams {
        index_type: IndexType::Flat,
        distance: DistanceType::L2,
        ..Default::default()
    };

    store.create_collection("info_test", 256, params).unwrap();
    let info = store.collection_info("info_test").unwrap();

    assert_eq!(info.name, "info_test");
    assert_eq!(info.dimension, 256);
    assert_eq!(info.count, 0);
    assert_eq!(info.index_type, IndexType::Flat);
    assert_eq!(info.distance, DistanceType::L2);
}

// ============================================================================
// Upsert and Get Tests
// ============================================================================

#[test]
fn test_upsert_and_get() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("upsert_test", 3, params).unwrap();

    let point = create_test_point("p1", vec![1.0, 2.0, 3.0], "/test/file1");
    store.upsert("upsert_test", vec![point]).unwrap();

    let retrieved = store.get("upsert_test", "p1").unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "p1");
    assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_upsert_overwrite() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("overwrite_test", 3, params)
        .unwrap();

    // Insert initial point
    let point1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
    store.upsert("overwrite_test", vec![point1]).unwrap();

    // Overwrite with same ID
    let point2 = create_test_point("p1", vec![0.0, 1.0, 0.0], "/test/file2");
    store.upsert("overwrite_test", vec![point2]).unwrap();

    let retrieved = store.get("overwrite_test", "p1").unwrap().unwrap();
    assert_eq!(retrieved.vector, vec![0.0, 1.0, 0.0]);

    // Verify URI was updated
    let uri = retrieved.payload.get("uri").unwrap().as_str().unwrap();
    assert_eq!(uri, "/test/file2");
}

#[test]
fn test_upsert_multiple_points() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("multi_test", 3, params).unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3"),
    ];

    store.upsert("multi_test", points).unwrap();

    let info = store.collection_info("multi_test").unwrap();
    assert_eq!(info.count, 3);

    assert!(store.get("multi_test", "p1").unwrap().is_some());
    assert!(store.get("multi_test", "p2").unwrap().is_some());
    assert!(store.get("multi_test", "p3").unwrap().is_some());
}

// ============================================================================
// Search Tests
// ============================================================================

#[test]
fn test_search_basic() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("search_test", 3, params).unwrap();

    // Insert orthogonal vectors
    let points = vec![
        create_test_point("x_axis", vec![1.0, 0.0, 0.0], "/test/x"),
        create_test_point("y_axis", vec![0.0, 1.0, 0.0], "/test/y"),
        create_test_point("z_axis", vec![0.0, 0.0, 1.0], "/test/z"),
    ];
    store.upsert("search_test", points).unwrap();

    // Search for vector closest to [1.0, 0.0, 0.0]
    let results = store
        .search("search_test", &[1.0, 0.0, 0.0], 2, None)
        .unwrap();

    assert_eq!(results.len(), 2);
    // First result should be the x_axis vector (cosine distance 0)
    assert_eq!(results[0].id, "x_axis");
    // Lower score is better (cosine distance)
    assert!(results[0].score < results[1].score);
}

#[test]
fn test_search_with_filter() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("filter_test", 3, params).unwrap();

    // Create points with different context types
    let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    p1.payload = json!({
        "id": "p1",
        "uri": "/test/1",
        "context_type": "resource",
        "is_leaf": true,
        "level": 0
    });

    let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2");
    p2.payload = json!({
        "id": "p2",
        "uri": "/test/2",
        "context_type": "memory",
        "is_leaf": true,
        "level": 0
    });

    let mut p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3");
    p3.payload = json!({
        "id": "p3",
        "uri": "/test/3",
        "context_type": "resource",
        "is_leaf": true,
        "level": 0
    });

    store.upsert("filter_test", vec![p1, p2, p3]).unwrap();

    // Filter for context_type == "resource"
    let filter = Filter::Eq("context_type".to_string(), json!("resource"));
    let results = store
        .search("filter_test", &[1.0, 0.0, 0.0], 10, Some(filter))
        .unwrap();

    assert_eq!(results.len(), 2);
    for result in &results {
        assert_ne!(result.id, "p2");
    }
}

#[test]
fn test_search_with_in_filter() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("in_filter_test", 3, params)
        .unwrap();

    let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    p1.payload = json!({"id": "p1", "uri": "/test/1", "level": 0});

    let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2");
    p2.payload = json!({"id": "p2", "uri": "/test/2", "level": 1});

    let mut p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3");
    p3.payload = json!({"id": "p3", "uri": "/test/3", "level": 2});

    store.upsert("in_filter_test", vec![p1, p2, p3]).unwrap();

    // Filter for level in [0, 1]
    let filter = Filter::In("level".to_string(), vec![json!(0), json!(1)]);
    let results = store
        .search("in_filter_test", &[1.0, 0.0, 0.0], 10, Some(filter))
        .unwrap();

    assert_eq!(results.len(), 2);
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"p1"));
    assert!(ids.contains(&"p2"));
    assert!(!ids.contains(&"p3"));
}

#[test]
fn test_search_nonexistent_collection() {
    let store = MemoryVectorStore::new();

    let result = store.search("nonexistent", &[1.0, 0.0, 0.0], 10, None);
    assert!(result.is_err());
}

#[test]
fn test_search_with_limit() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("limit_test", 3, params).unwrap();

    let points: Vec<VectorPoint> = (0..10)
        .map(|i| {
            let v = i as f32 / 10.0;
            create_test_point(&format!("p{}", i), vec![v, v, v], &format!("/test/{}", i))
        })
        .collect();

    store.upsert("limit_test", points).unwrap();

    // Request only 3 results
    let results = store
        .search("limit_test", &[0.5, 0.5, 0.5], 3, None)
        .unwrap();
    assert_eq!(results.len(), 3);
}

// ============================================================================
// Delete Tests
// ============================================================================

#[test]
fn test_delete() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("delete_test", 3, params).unwrap();

    let point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    store.upsert("delete_test", vec![point]).unwrap();

    // Verify exists
    assert!(store.get("delete_test", "p1").unwrap().is_some());

    // Delete
    store.delete("delete_test", "p1").unwrap();

    // Verify deleted
    assert!(store.get("delete_test", "p1").unwrap().is_none());

    // Verify count updated
    let info = store.collection_info("delete_test").unwrap();
    assert_eq!(info.count, 0);
}

#[test]
fn test_delete_by_uri_prefix() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("uri_delete_test", 3, params)
        .unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/docs/file1.txt"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/docs/subdir/file2.txt"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/other/file3.txt"),
    ];

    store.upsert("uri_delete_test", points).unwrap();

    // Delete by URI prefix
    store
        .delete_by_uri_prefix("uri_delete_test", "/docs")
        .unwrap();

    // p1 and p2 should be deleted
    assert!(store.get("uri_delete_test", "p1").unwrap().is_none());
    assert!(store.get("uri_delete_test", "p2").unwrap().is_none());

    // p3 should remain
    assert!(store.get("uri_delete_test", "p3").unwrap().is_some());

    let info = store.collection_info("uri_delete_test").unwrap();
    assert_eq!(info.count, 1);
}

#[test]
fn test_delete_by_uri_prefix_no_match() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("no_match_test", 3, params).unwrap();

    let point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1.txt");
    store.upsert("no_match_test", vec![point]).unwrap();

    // Delete with non-matching prefix
    store
        .delete_by_uri_prefix("no_match_test", "/nonexistent")
        .unwrap();

    // Point should still exist
    assert!(store.get("no_match_test", "p1").unwrap().is_some());
}

// ============================================================================
// Update URI Tests
// ============================================================================

#[test]
fn test_update_uri() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("update_uri_test", 3, params)
        .unwrap();

    let mut point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file.txt");
    point.payload = json!({
        "id": "p1",
        "uri": "/old/path/file.txt",
        "parent_uri": "/old/path",
        "context_type": "resource"
    });

    store.upsert("update_uri_test", vec![point]).unwrap();

    // Update URI
    store
        .update_uri("update_uri_test", "/old/path", "/new/path")
        .unwrap();

    let updated = store.get("update_uri_test", "p1").unwrap().unwrap();
    let uri = updated.payload.get("uri").unwrap().as_str().unwrap();
    assert_eq!(uri, "/new/path/file.txt");

    let parent_uri = updated.payload.get("parent_uri").unwrap().as_str().unwrap();
    assert_eq!(parent_uri, "/new/path");
}

#[test]
fn test_update_uri_partial_match() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("partial_test", 3, params).unwrap();

    let mut point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file.txt");
    point.payload = json!({
        "id": "p1",
        "uri": "/old/path/file.txt",
    });

    store.upsert("partial_test", vec![point]).unwrap();

    // Update with a different old_uri (no match)
    store
        .update_uri("partial_test", "/different/path", "/new/path")
        .unwrap();

    // URI should remain unchanged
    let unchanged = store.get("partial_test", "p1").unwrap().unwrap();
    let uri = unchanged.payload.get("uri").unwrap().as_str().unwrap();
    assert_eq!(uri, "/old/path/file.txt");
}

// ============================================================================
// Provider Info Tests
// ============================================================================

#[test]
fn test_provider_name_and_version() {
    let store = MemoryVectorStore::new();
    assert_eq!(store.name(), "memory");
    assert_eq!(store.version(), "0.1.0");
}

#[test]
fn test_initialize() {
    let store = MemoryVectorStore::new();
    let result = store.initialize(&json!({}));
    assert!(result.is_ok());
}

// ============================================================================
// Distance Type Tests
// ============================================================================

#[test]
fn test_search_cosine_distance() {
    let store = MemoryVectorStore::new();
    let params = IndexParams {
        distance: DistanceType::Cosine,
        ..Default::default()
    };

    store.create_collection("cosine_test", 3, params).unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1"),
        create_test_point("p2", vec![0.5, 0.5, 0.0], "/test/2"),
    ];
    store.upsert("cosine_test", points).unwrap();

    // Search with [1, 0, 0] - should find p1 first
    let results = store
        .search("cosine_test", &[1.0, 0.0, 0.0], 2, None)
        .unwrap();
    assert_eq!(results[0].id, "p1");
}

#[test]
fn test_search_l2_distance() {
    let store = MemoryVectorStore::new();
    let params = IndexParams {
        distance: DistanceType::L2,
        ..Default::default()
    };

    store.create_collection("l2_test", 3, params).unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3"),
    ];
    store.upsert("l2_test", points).unwrap();

    // Search with [1, 0, 0] - should find p1 first (L2 distance = 0)
    let results = store.search("l2_test", &[1.0, 0.0, 0.0], 1, None).unwrap();
    assert_eq!(results[0].id, "p1");
}

#[test]
fn test_search_dot_product() {
    let store = MemoryVectorStore::new();
    let params = IndexParams {
        distance: DistanceType::DotProduct,
        ..Default::default()
    };

    store.create_collection("dot_test", 3, params).unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2"),
    ];
    store.upsert("dot_test", points).unwrap();

    // For dot product, higher is better (lower "distance")
    let results = store.search("dot_test", &[1.0, 0.0, 0.0], 1, None).unwrap();
    assert_eq!(results[0].id, "p1");
}

// ============================================================================
// Index Type Tests
// ============================================================================

#[test]
fn test_index_types() {
    let store = MemoryVectorStore::new();

    let params = IndexParams {
        index_type: IndexType::Flat,
        ..Default::default()
    };
    store.create_collection("flat_test", 64, params).unwrap();
    let info = store.collection_info("flat_test").unwrap();
    assert_eq!(info.index_type, IndexType::Flat);

    // HNSW index
    let params = IndexParams {
        index_type: IndexType::Hnsw,
        ..Default::default()
    };
    store.create_collection("hnsw_test", 64, params).unwrap();
    let info = store.collection_info("hnsw_test").unwrap();
    assert_eq!(info.index_type, IndexType::Hnsw);

    // IVF index
    let params = IndexParams {
        index_type: IndexType::Ivf,
        ..Default::default()
    };
    store.create_collection("ivf_test", 64, params).unwrap();
    let info = store.collection_info("ivf_test").unwrap();
    assert_eq!(info.index_type, IndexType::Ivf);

    // FlatHybrid index
    let params = IndexParams {
        index_type: IndexType::FlatHybrid,
        ..Default::default()
    };
    store.create_collection("hybrid_test", 64, params).unwrap();
    let info = store.collection_info("hybrid_test").unwrap();
    assert_eq!(info.index_type, IndexType::FlatHybrid);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_get_nonexistent_point() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("empty_test", 3, params).unwrap();

    let result = store.get("empty_test", "nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_delete_nonexistent_point() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("delete_test", 3, params).unwrap();

    // Deleting non-existent point should succeed (no-op)
    let result = store.delete("delete_test", "nonexistent");
    assert!(result.is_ok());
}

#[test]
fn test_collection_info_nonexistent() {
    let store = MemoryVectorStore::new();

    let result = store.collection_info("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_upsert_wrong_dimension() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("dim_test", 3, params).unwrap();

    // Try to insert vector with wrong dimension
    let point = create_test_point("p1", vec![1.0, 2.0], "/test/1"); // 2D instead of 3D
    let result = store.upsert("dim_test", vec![point]);

    assert!(result.is_err());
}

#[test]
fn test_search_wrong_dimension() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store
        .create_collection("dim_search_test", 3, params)
        .unwrap();

    // Try to search with wrong dimension
    let result = store.search("dim_search_test", &[1.0, 2.0], 10, None);
    assert!(result.is_err());
}

// ============================================================================
// Sparse Vector Tests
// ============================================================================

#[test]
fn test_sparse_vector_storage() {
    let store = MemoryVectorStore::new();
    let params = IndexParams::default();

    store.create_collection("sparse_test", 3, params).unwrap();

    let mut sparse = std::collections::HashMap::new();
    sparse.insert(0, 1.0);
    sparse.insert(2, 0.5);

    let point = VectorPoint {
        id: "sparse_p1".to_string(),
        vector: vec![1.0, 0.0, 0.5],
        sparse_vector: Some(sparse.clone()),
        payload: json!({"id": "sparse_p1", "uri": "/test/sparse"}),
    };

    store.upsert("sparse_test", vec![point]).unwrap();

    let retrieved = store.get("sparse_test", "sparse_p1").unwrap().unwrap();
    assert!(retrieved.sparse_vector.is_some());

    let retrieved_sparse = retrieved.sparse_vector.unwrap();
    assert_eq!(retrieved_sparse.get(&0), Some(&1.0));
    assert_eq!(retrieved_sparse.get(&2), Some(&0.5));
}
