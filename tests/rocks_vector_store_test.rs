//! RocksDB VectorStore Integration Tests
//!
//! End-to-end tests for RocksDB-backed persistent vector store.
//! Uses tempdir to ensure tests don't pollute the filesystem.

use rustviking::vector_store::rocks::RocksDBVectorStore;
use rustviking::vector_store::traits::VectorStore;
use rustviking::vector_store::types::{Filter, IndexParams, VectorPoint};
use serde_json::json;
use tempfile::TempDir;

// Helper function to create a test vector point with full payload
fn create_test_point(id: &str, vector: Vec<f32>, uri: &str) -> VectorPoint {
    let payload = json!({
        "id": id,
        "uri": uri,
        "context_type": "resource",
        "is_leaf": true,
        "level": 0,
        "created_at": "2024-01-01T00:00:00Z",
        "abstract_text": format!("Content for {}", id),
        "name": format!("Name {}", id),
        "description": format!("Description for {}", id),
        "active_count": 1
    });

    VectorPoint {
        id: id.to_string(),
        vector,
        sparse_vector: None,
        payload,
    }
}

// Helper to create a test store with temp directory
fn create_test_store() -> (RocksDBVectorStore, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
    (store, temp_dir)
}

// ============================================================================
// Basic CRUD Operations
// ============================================================================

#[tokio::test]
async fn test_create_collection() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("test_collection", 128, params)
        .await
        .unwrap();

    let info = store.collection_info("test_collection").await.unwrap();
    assert_eq!(info.name, "test_collection");
    assert_eq!(info.dimension, 128);
    assert_eq!(info.count, 0);
}

#[tokio::test]
async fn test_upsert_and_get() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("test", 3, params).await.unwrap();

    let point = create_test_point("p1", vec![1.0, 2.0, 3.0], "/test/file1");
    store.upsert("test", vec![point.clone()]).await.unwrap();

    let retrieved = store.get("test", "p1").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "p1");
    assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);

    // Verify count updated
    let info = store.collection_info("test").await.unwrap();
    assert_eq!(info.count, 1);
}

#[tokio::test]
async fn test_upsert_multiple_points() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("multi_test", 3, params).await.unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3"),
    ];

    store.upsert("multi_test", points).await.unwrap();

    let info = store.collection_info("multi_test").await.unwrap();
    assert_eq!(info.count, 3);

    assert!(store.get("multi_test", "p1").await.unwrap().is_some());
    assert!(store.get("multi_test", "p2").await.unwrap().is_some());
    assert!(store.get("multi_test", "p3").await.unwrap().is_some());
}

#[tokio::test]
async fn test_search() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("search_test", 3, params).await.unwrap();

    // Insert orthogonal vectors
    let points = vec![
        create_test_point("x_axis", vec![1.0, 0.0, 0.0], "/test/x"),
        create_test_point("y_axis", vec![0.0, 1.0, 0.0], "/test/y"),
        create_test_point("z_axis", vec![0.0, 0.0, 1.0], "/test/z"),
    ];
    store.upsert("search_test", points).await.unwrap();

    // Search for vector closest to [1.0, 0.0, 0.0]
    let results = store
        .search("search_test", &[1.0, 0.0, 0.0], 2, None)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    // First result should be the x_axis vector (cosine distance 0)
    assert_eq!(results[0].id, "x_axis");
    // Lower score is better (cosine distance)
    assert!(results[0].score < results[1].score);
}

#[tokio::test]
async fn test_delete() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("delete_test", 3, params).await.unwrap();

    let point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    store.upsert("delete_test", vec![point]).await.unwrap();

    // Verify exists
    assert!(store.get("delete_test", "p1").await.unwrap().is_some());

    // Delete
    store.delete("delete_test", "p1").await.unwrap();

    // Verify deleted
    assert!(store.get("delete_test", "p1").await.unwrap().is_none());

    // Verify count updated
    let info = store.collection_info("delete_test").await.unwrap();
    assert_eq!(info.count, 0);
}

// ============================================================================
// Delete by URI Prefix Tests
// ============================================================================

#[tokio::test]
async fn test_delete_by_uri_prefix() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("uri_delete_test", 3, params)
        .await
        .unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/docs/file1.txt"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/docs/subdir/file2.txt"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/other/file3.txt"),
    ];

    store.upsert("uri_delete_test", points).await.unwrap();

    // Delete by URI prefix
    store
        .delete_by_uri_prefix("uri_delete_test", "/docs")
        .await
        .unwrap();

    // p1 and p2 should be deleted
    assert!(store.get("uri_delete_test", "p1").await.unwrap().is_none());
    assert!(store.get("uri_delete_test", "p2").await.unwrap().is_none());

    // p3 should remain
    assert!(store.get("uri_delete_test", "p3").await.unwrap().is_some());

    let info = store.collection_info("uri_delete_test").await.unwrap();
    assert_eq!(info.count, 1);
}

#[tokio::test]
async fn test_delete_by_uri_prefix_nested() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("nested_test", 3, params).await.unwrap();

    let points = vec![
        create_test_point("p1", vec![1.0, 0.0, 0.0], "/a/b/c/file1.txt"),
        create_test_point("p2", vec![0.0, 1.0, 0.0], "/a/b/file2.txt"),
        create_test_point("p3", vec![0.0, 0.0, 1.0], "/a/file3.txt"),
        create_test_point("p4", vec![1.0, 1.0, 0.0], "/x/y/file4.txt"),
    ];

    store.upsert("nested_test", points).await.unwrap();

    // Delete only /a/b prefix
    store.delete_by_uri_prefix("nested_test", "/a/b").await.unwrap();

    // p1 and p2 should be deleted
    assert!(store.get("nested_test", "p1").await.unwrap().is_none());
    assert!(store.get("nested_test", "p2").await.unwrap().is_none());

    // p3 and p4 should remain
    assert!(store.get("nested_test", "p3").await.unwrap().is_some());
    assert!(store.get("nested_test", "p4").await.unwrap().is_some());

    let info = store.collection_info("nested_test").await.unwrap();
    assert_eq!(info.count, 2);
}

// ============================================================================
// Update URI Tests
// ============================================================================

#[tokio::test]
async fn test_update_uri() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("update_uri_test", 3, params)
        .await
        .unwrap();

    let mut point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file.txt");
    point.payload = json!({
        "id": "p1",
        "uri": "/old/path/file.txt",
        "parent_uri": "/old/path",
        "context_type": "resource"
    });

    store.upsert("update_uri_test", vec![point]).await.unwrap();

    // Update URI
    store
        .update_uri("update_uri_test", "/old/path", "/new/path")
        .await
        .unwrap();

    let updated = store.get("update_uri_test", "p1").await.unwrap().unwrap();
    let uri = updated.payload.get("uri").unwrap().as_str().unwrap();
    assert_eq!(uri, "/new/path/file.txt");

    let parent_uri = updated.payload.get("parent_uri").unwrap().as_str().unwrap();
    assert_eq!(parent_uri, "/new/path");
}

#[tokio::test]
async fn test_update_uri_multiple_files() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("multi_update_test", 3, params)
        .await
        .unwrap();

    let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file1.txt");
    p1.payload = json!({
        "id": "p1",
        "uri": "/old/path/file1.txt",
        "parent_uri": "/old/path",
        "context_type": "resource"
    });

    let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/old/path/subdir/file2.txt");
    p2.payload = json!({
        "id": "p2",
        "uri": "/old/path/subdir/file2.txt",
        "parent_uri": "/old/path/subdir",
        "context_type": "memory"
    });

    let mut p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/other/path/file3.txt");
    p3.payload = json!({
        "id": "p3",
        "uri": "/other/path/file3.txt",
        "parent_uri": "/other/path",
        "context_type": "resource"
    });

    store.upsert("multi_update_test", vec![p1, p2, p3]).await.unwrap();

    // Update URI prefix
    store
        .update_uri("multi_update_test", "/old/path", "/new/path")
        .await
        .unwrap();

    // Check p1
    let updated_p1 = store.get("multi_update_test", "p1").await.unwrap().unwrap();
    assert_eq!(
        updated_p1.payload.get("uri").unwrap().as_str().unwrap(),
        "/new/path/file1.txt"
    );
    assert_eq!(
        updated_p1
            .payload
            .get("parent_uri")
            .unwrap()
            .as_str()
            .unwrap(),
        "/new/path"
    );

    // Check p2 (nested)
    let updated_p2 = store.get("multi_update_test", "p2").await.unwrap().unwrap();
    assert_eq!(
        updated_p2.payload.get("uri").unwrap().as_str().unwrap(),
        "/new/path/subdir/file2.txt"
    );
    assert_eq!(
        updated_p2
            .payload
            .get("parent_uri")
            .unwrap()
            .as_str()
            .unwrap(),
        "/new/path/subdir"
    );

    // Check p3 (should be unchanged)
    let unchanged_p3 = store.get("multi_update_test", "p3").await.unwrap().unwrap();
    assert_eq!(
        unchanged_p3.payload.get("uri").unwrap().as_str().unwrap(),
        "/other/path/file3.txt"
    );
}

// ============================================================================
// Collection Info Tests
// ============================================================================

#[tokio::test]
async fn test_collection_info() {
    let (store, _temp) = create_test_store();
    let params = IndexParams {
        index_type: rustviking::vector_store::types::IndexType::Flat,
        distance: rustviking::vector_store::types::DistanceType::L2,
        ..Default::default()
    };

    store.create_collection("info_test", 256, params).await.unwrap();

    let info = store.collection_info("info_test").await.unwrap();
    assert_eq!(info.name, "info_test");
    assert_eq!(info.dimension, 256);
    assert_eq!(info.count, 0);
    assert_eq!(
        info.index_type,
        rustviking::vector_store::types::IndexType::Flat
    );
    assert_eq!(
        info.distance,
        rustviking::vector_store::types::DistanceType::L2
    );
}

#[tokio::test]
async fn test_collection_info_with_data() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("data_test", 64, params).await.unwrap();

    // Insert 100 vectors
    let points: Vec<VectorPoint> = (0..100)
        .map(|i| {
            create_test_point(
                &format!("p{}", i),
                vec![i as f32; 64],
                &format!("/test/{}", i),
            )
        })
        .collect();

    store.upsert("data_test", points).await.unwrap();

    let info = store.collection_info("data_test").await.unwrap();
    assert_eq!(info.count, 100);
    assert_eq!(info.dimension, 64);
}

// ============================================================================
// Large Scale Data Tests
// ============================================================================

#[tokio::test]
async fn test_large_scale_insert_and_search() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("large_test", 128, params).await.unwrap();

    // Insert 1000+ vectors
    let points: Vec<VectorPoint> = (0..1000)
        .map(|i| {
            let vector: Vec<f32> = (0..128).map(|j| ((i * j) % 100) as f32 / 100.0).collect();
            create_test_point(&format!("p{}", i), vector, &format!("/docs/file{}", i))
        })
        .collect();

    store.upsert("large_test", points).await.unwrap();

    let info = store.collection_info("large_test").await.unwrap();
    assert_eq!(info.count, 1000);

    // Search
    let query: Vec<f32> = (0..128).map(|j| (j % 100) as f32 / 100.0).collect();
    let results = store.search("large_test", &query, 10, None).await.unwrap();

    assert_eq!(results.len(), 10);
    // Results should be sorted by score (lower is better)
    for i in 1..results.len() {
        assert!(results[i - 1].score <= results[i].score);
    }
}

#[tokio::test]
async fn test_large_scale_batch_insert() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("batch_test", 64, params).await.unwrap();

    // Insert vectors in batches
    for batch in 0..10 {
        let points: Vec<VectorPoint> = (0..100)
            .map(|i| {
                let idx = batch * 100 + i;
                create_test_point(
                    &format!("p{}", idx),
                    vec![idx as f32; 64],
                    &format!("/batch{}/{}", batch, i),
                )
            })
            .collect();

        store.upsert("batch_test", points).await.unwrap();
    }

    let info = store.collection_info("batch_test").await.unwrap();
    assert_eq!(info.count, 1000);
}

// ============================================================================
// Persistence Tests
// ============================================================================

#[tokio::test]
async fn test_persistence_basic() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Create store and add data
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();
        let params = IndexParams::default();
        store.create_collection("persist_test", 3, params).await.unwrap();

        let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
        let p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/file2");
        store.upsert("persist_test", vec![p1, p2]).await.unwrap();
    }

    // Reopen store and verify data persists
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();

        // Collection should exist
        let info = store.collection_info("persist_test").await.unwrap();
        assert_eq!(info.count, 2);
        assert_eq!(info.dimension, 3);

        // Points should exist
        let retrieved_p1 = store.get("persist_test", "p1").await.unwrap();
        assert!(retrieved_p1.is_some());
        assert_eq!(retrieved_p1.unwrap().vector, vec![1.0, 0.0, 0.0]);

        let retrieved_p2 = store.get("persist_test", "p2").await.unwrap();
        assert!(retrieved_p2.is_some());
        assert_eq!(retrieved_p2.unwrap().vector, vec![0.0, 1.0, 0.0]);
    }
}

#[tokio::test]
async fn test_persistence_search_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Create store and add data
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();
        let params = IndexParams::default();
        store
            .create_collection("search_persist", 3, params)
            .await
            .unwrap();

        let points = vec![
            create_test_point("p1", vec![1.0, 0.0, 0.0], "/docs/a"),
            create_test_point("p2", vec![0.0, 1.0, 0.0], "/docs/b"),
            create_test_point("p3", vec![0.0, 0.0, 1.0], "/docs/c"),
        ];
        store.upsert("search_persist", points).await.unwrap();
    }

    // Reopen and search
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();

        let results = store
            .search("search_persist", &[1.0, 0.0, 0.0], 2, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "p1");
    }
}

#[tokio::test]
async fn test_persistence_delete_by_prefix_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Create store and add data
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();
        let params = IndexParams::default();
        store
            .create_collection("delete_persist", 3, params)
            .await
            .unwrap();

        let points = vec![
            create_test_point("p1", vec![1.0, 0.0, 0.0], "/docs/file1"),
            create_test_point("p2", vec![0.0, 1.0, 0.0], "/docs/file2"),
            create_test_point("p3", vec![0.0, 0.0, 1.0], "/other/file3"),
        ];
        store.upsert("delete_persist", points).await.unwrap();
    }

    // Reopen and delete by prefix
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();

        store
            .delete_by_uri_prefix("delete_persist", "/docs")
            .await
            .unwrap();

        let info = store.collection_info("delete_persist").await.unwrap();
        assert_eq!(info.count, 1);

        assert!(store.get("delete_persist", "p1").await.unwrap().is_none());
        assert!(store.get("delete_persist", "p2").await.unwrap().is_none());
        assert!(store.get("delete_persist", "p3").await.unwrap().is_some());
    }
}

#[tokio::test]
async fn test_persistence_update_uri_after_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Create store and add data
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();
        let params = IndexParams::default();
        store
            .create_collection("update_persist", 3, params)
            .await
            .unwrap();

        let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/file1");
        p1.payload = json!({
            "id": "p1",
            "uri": "/old/file1",
            "parent_uri": "/old",
            "context_type": "resource"
        });
        store.upsert("update_persist", vec![p1]).await.unwrap();
    }

    // Reopen and update URI
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();

        store.update_uri("update_persist", "/old", "/new").await.unwrap();

        let updated = store.get("update_persist", "p1").await.unwrap().unwrap();
        assert_eq!(
            updated.payload.get("uri").unwrap().as_str().unwrap(),
            "/new/file1"
        );
        assert_eq!(
            updated.payload.get("parent_uri").unwrap().as_str().unwrap(),
            "/new"
        );
    }
}

#[tokio::test]
async fn test_persistence_large_dataset() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_str().unwrap();

    // Create store with large dataset
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();
        let params = IndexParams::default();
        store
            .create_collection("large_persist", 64, params)
            .await
            .unwrap();

        let points: Vec<VectorPoint> = (0..500)
            .map(|i| {
                let vector: Vec<f32> = (0..64).map(|j| ((i * j) % 100) as f32 / 100.0).collect();
                create_test_point(&format!("p{}", i), vector, &format!("/data/file{}", i))
            })
            .collect();

        store.upsert("large_persist", points).await.unwrap();

        let info = store.collection_info("large_persist").await.unwrap();
        assert_eq!(info.count, 500);
    }

    // Reopen and verify
    {
        let store = RocksDBVectorStore::with_path(path).unwrap();

        let info = store.collection_info("large_persist").await.unwrap();
        assert_eq!(info.count, 500);

        // Verify some random points
        for i in [0, 100, 250, 499] {
            let point = store.get("large_persist", &format!("p{}", i)).await.unwrap();
            assert!(point.is_some(), "Point p{} should exist", i);
        }

        // Search should work
        let query: Vec<f32> = (0..64).map(|j| (j % 100) as f32 / 100.0).collect();
        let results = store.search("large_persist", &query, 10, None).await.unwrap();
        assert_eq!(results.len(), 10);
    }
}

// ============================================================================
// Search with Filters Tests
// ============================================================================

#[tokio::test]
async fn test_search_with_filter() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("filter_test", 3, params).await.unwrap();

    let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    p1.payload = json!({
        "id": "p1",
        "uri": "/test/1",
        "context_type": "resource",
        "level": 0
    });

    let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2");
    p2.payload = json!({
        "id": "p2",
        "uri": "/test/2",
        "context_type": "memory",
        "level": 1
    });

    let mut p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3");
    p3.payload = json!({
        "id": "p3",
        "uri": "/test/3",
        "context_type": "resource",
        "level": 2
    });

    store.upsert("filter_test", vec![p1, p2, p3]).await.unwrap();

    // Filter for context_type == "resource"
    let filter = Filter::Eq("context_type".to_string(), json!("resource"));
    let results = store
        .search("filter_test", &[1.0, 0.0, 0.0], 10, Some(filter))
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    for result in &results {
        assert_ne!(result.id, "p2");
    }
}

#[tokio::test]
async fn test_search_with_in_filter() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("in_filter_test", 3, params)
        .await
        .unwrap();

    let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/1");
    p1.payload = json!({"id": "p1", "uri": "/test/1", "level": 0});

    let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/2");
    p2.payload = json!({"id": "p2", "uri": "/test/2", "level": 1});

    let mut p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/3");
    p3.payload = json!({"id": "p3", "uri": "/test/3", "level": 2});

    store.upsert("in_filter_test", vec![p1, p2, p3]).await.unwrap();

    // Filter for level in [0, 1]
    let filter = Filter::In("level".to_string(), vec![json!(0), json!(1)]);
    let results = store
        .search("in_filter_test", &[1.0, 0.0, 0.0], 10, Some(filter))
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"p1"));
    assert!(ids.contains(&"p2"));
    assert!(!ids.contains(&"p3"));
}

// ============================================================================
// Provider Info Tests
// ============================================================================

#[tokio::test]
async fn test_provider_name_and_version() {
    let (store, _temp) = create_test_store();
    assert_eq!(store.name(), "rocksdb");
    assert_eq!(store.version(), "0.1.0");
}

#[tokio::test]
async fn test_initialize() {
    let (store, _temp) = create_test_store();
    assert!(store.initialize(&json!({})).await.is_ok());
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_create_duplicate_collection() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("test", 3, params.clone()).await.unwrap();
    let result = store.create_collection("test", 3, params).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_upsert_wrong_dimension() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("dim_test", 3, params).await.unwrap();

    // Try to insert vector with wrong dimension
    let point = create_test_point("p1", vec![1.0, 2.0], "/test/1");
    let result = store.upsert("dim_test", vec![point]).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_wrong_dimension() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("dim_search_test", 3, params)
        .await
        .unwrap();

    // Try to search with wrong dimension
    let result = store.search("dim_search_test", &[1.0, 2.0], 10, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_nonexistent_point() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("empty_test", 3, params).await.unwrap();

    let result = store.get("empty_test", "nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_collection_info_nonexistent() {
    let (store, _temp) = create_test_store();

    let result = store.collection_info("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_nonexistent_point() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("delete_test", 3, params).await.unwrap();

    // Deleting non-existent point should succeed (no-op)
    let result = store.delete("delete_test", "nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_by_uri_prefix_no_match() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("no_match_test", 3, params).await.unwrap();

    let point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
    store.upsert("no_match_test", vec![point]).await.unwrap();

    // Delete with non-matching prefix
    store
        .delete_by_uri_prefix("no_match_test", "/nonexistent")
        .await
        .unwrap();

    // Point should still exist
    assert!(store.get("no_match_test", "p1").await.unwrap().is_some());
}

#[tokio::test]
async fn test_update_uri_no_match() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store
        .create_collection("no_update_test", 3, params)
        .await
        .unwrap();

    let point = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file.txt");
    store.upsert("no_update_test", vec![point]).await.unwrap();

    // Update with a different old_uri (no match)
    store
        .update_uri("no_update_test", "/different/path", "/new/path")
        .await
        .unwrap();

    // URI should remain unchanged
    let unchanged = store.get("no_update_test", "p1").await.unwrap().unwrap();
    let uri = unchanged.payload.get("uri").unwrap().as_str().unwrap();
    assert_eq!(uri, "/old/path/file.txt");
}

#[tokio::test]
async fn test_search_empty_collection() {
    let (store, _temp) = create_test_store();
    let params = IndexParams::default();

    store.create_collection("empty_search", 3, params).await.unwrap();

    // Search in empty collection should return empty results
    let results = store
        .search("empty_search", &[1.0, 0.0, 0.0], 10, None)
        .await
        .unwrap();
    assert!(results.is_empty());
}
