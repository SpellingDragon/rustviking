//! Memory-based VectorStore implementation
//!
//! HashMap-based vector storage for development and testing.
//! Uses brute-force search with DistanceComputer for similarity search.

use crate::compute::distance::DistanceComputer;
use crate::compute::simd::{top_k_smallest, PARALLEL_THRESHOLD};
use crate::error::{Result, RustVikingError};
use crate::vector_store::traits::VectorStore;
use crate::vector_store::types::*;
use async_trait::async_trait;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// In-memory collection data
struct MemoryCollection {
    name: String,
    dimension: usize,
    index_type: IndexType,
    distance: DistanceType,
    points: HashMap<String, VectorPoint>,
}

/// Memory-based vector store - for development and testing
pub struct MemoryVectorStore {
    collections: RwLock<HashMap<String, MemoryCollection>>,
}

impl Default for MemoryVectorStore {
    fn default() -> Self {
        Self {
            collections: RwLock::new(HashMap::new()),
        }
    }
}

impl MemoryVectorStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute distance between two vectors using the specified distance type
    fn compute_distance(
        computer: &DistanceComputer,
        a: &[f32],
        b: &[f32],
        distance_type: DistanceType,
    ) -> f32 {
        match distance_type {
            DistanceType::Cosine => computer.cosine_distance(a, b),
            DistanceType::L2 => computer.l2_distance(a, b),
            DistanceType::DotProduct => {
                // For dot product, we return negative as "distance" (higher dot = lower distance)
                -computer.dot_product(a, b)
            }
        }
    }

    /// Check if a point matches the filter
    /// Note: Only Eq and In filters are supported, other filters are ignored
    fn matches_filter(point: &VectorPoint, filter: &Filter) -> bool {
        match filter {
            Filter::Eq(field, value) => {
                if let Some(payload_value) = point.payload.get(field) {
                    payload_value == value
                } else {
                    false
                }
            }
            Filter::In(field, values) => {
                if let Some(payload_value) = point.payload.get(field) {
                    values.contains(payload_value)
                } else {
                    false
                }
            }
            // Range, And, Or filters are not supported in this simplified implementation
            _ => true,
        }
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    fn name(&self) -> &str {
        "memory"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn initialize(&self, _config: &Value) -> Result<()> {
        // No initialization needed for memory store
        Ok(())
    }

    async fn create_collection(&self, name: &str, dimension: usize, params: IndexParams) -> Result<()> {
        let mut collections = self.collections.write().await;

        if collections.contains_key(name) {
            return Err(RustVikingError::Storage(format!(
                "Collection '{}' already exists",
                name
            )));
        }

        let collection = MemoryCollection {
            name: name.to_string(),
            dimension,
            index_type: params.index_type,
            distance: params.distance,
            points: HashMap::new(),
        };

        collections.insert(name.to_string(), collection);
        Ok(())
    }

    async fn upsert(&self, collection: &str, points: Vec<VectorPoint>) -> Result<()> {
        let mut collections = self.collections.write().await;

        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        for point in points {
            // Validate dimension
            if point.vector.len() != coll.dimension {
                return Err(RustVikingError::InvalidDimension {
                    expected: coll.dimension,
                    actual: point.vector.len(),
                });
            }
            coll.points.insert(point.id.clone(), point);
        }

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
    ) -> Result<Vec<VectorSearchResult>> {
        let collections = self.collections.read().await;

        let coll = collections
            .get(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        // Validate query dimension
        if query.len() != coll.dimension {
            return Err(RustVikingError::InvalidDimension {
                expected: coll.dimension,
                actual: query.len(),
            });
        }

        // Collect all points into a vector for processing
        let points: Vec<&VectorPoint> = coll.points.values().collect();

        // Use parallel computation for large collections
        let results = if points.len() >= PARALLEL_THRESHOLD {
            Self::search_parallel(query, k, filters, &points, coll.distance, coll.dimension)
        } else {
            Self::search_sequential(query, k, filters, &points, coll.distance, coll.dimension)
        };

        Ok(results)
    }

    async fn get(&self, collection: &str, id: &str) -> Result<Option<VectorPoint>> {
        let collections = self.collections.read().await;

        let coll = collections
            .get(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        Ok(coll.points.get(id).cloned())
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        coll.points.remove(id);
        Ok(())
    }

    async fn delete_by_uri_prefix(&self, collection: &str, uri_prefix: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        let ids_to_remove: Vec<String> = coll
            .points
            .values()
            .filter(|point| {
                if let Some(Value::String(uri)) = point.payload.get("uri") {
                    uri.starts_with(uri_prefix)
                } else {
                    false
                }
            })
            .map(|point| point.id.clone())
            .collect();

        for id in ids_to_remove {
            coll.points.remove(&id);
        }

        Ok(())
    }

    async fn update_uri(&self, collection: &str, old_uri: &str, new_uri: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        let coll = collections
            .get_mut(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        for point in coll.points.values_mut() {
            // Update uri field if it starts with old_uri (prefix match)
            if let Some(Value::String(uri)) = point.payload.get("uri") {
                if let Some(stripped) = uri.strip_prefix(old_uri) {
                    let new_uri_value = format!("{}{}", new_uri, stripped);
                    if let Some(obj) = point.payload.as_object_mut() {
                        obj.insert("uri".to_string(), Value::String(new_uri_value));
                    }
                }
            }

            // Update parent_uri if it starts with old_uri
            if let Some(Value::String(parent_uri)) = point.payload.get("parent_uri") {
                if let Some(stripped) = parent_uri.strip_prefix(old_uri) {
                    let new_parent_uri = format!("{}{}", new_uri, stripped);
                    if let Some(obj) = point.payload.as_object_mut() {
                        obj.insert("parent_uri".to_string(), Value::String(new_parent_uri));
                    }
                }
            }
        }

        Ok(())
    }

    async fn collection_info(&self, collection: &str) -> Result<CollectionInfo> {
        let collections = self.collections.read().await;

        let coll = collections
            .get(collection)
            .ok_or_else(|| RustVikingError::NotFound(format!("Collection '{}'", collection)))?;

        Ok(CollectionInfo {
            name: coll.name.clone(),
            dimension: coll.dimension,
            count: coll.points.len() as u64,
            index_type: coll.index_type,
            distance: coll.distance,
        })
    }
}

impl MemoryVectorStore {
    /// Sequential search for small collections
    fn search_sequential(
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
        points: &[&VectorPoint],
        distance_type: DistanceType,
        dimension: usize,
    ) -> Vec<VectorSearchResult> {
        let computer = DistanceComputer::new(dimension);
        let mut results: Vec<VectorSearchResult> = Vec::with_capacity(points.len().min(k));

        for point in points {
            // Apply filter if provided
            if let Some(ref filter) = &filters {
                if !Self::matches_filter(point, filter) {
                    continue;
                }
            }

            let distance = Self::compute_distance(&computer, query, &point.vector, distance_type);
            let metadata = Self::extract_metadata(&point.payload);

            results.push(VectorSearchResult {
                id: point.id.clone(),
                score: distance,
                metadata,
            });
        }

        // Sort by score (lower is better for Cosine and L2)
        results.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top k
        results.truncate(k);
        results
    }

    /// Parallel search for large collections using rayon
    fn search_parallel(
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
        points: &[&VectorPoint],
        distance_type: DistanceType,
        dimension: usize,
    ) -> Vec<VectorSearchResult> {
        // First, filter points in parallel
        let filtered_points: Vec<&VectorPoint> = if let Some(ref filter) = filters {
            points
                .par_iter()
                .filter(|&&point| Self::matches_filter(point, filter))
                .copied()
                .collect()
        } else {
            points.par_iter().copied().collect()
        };

        // Compute distances in parallel
        let distances: Vec<f32> = filtered_points
            .par_iter()
            .map(|point| {
                // Create a new DistanceComputer for each thread
                let computer = DistanceComputer::new(dimension);
                Self::compute_distance(&computer, query, &point.vector, distance_type)
            })
            .collect();

        // Use top_k_smallest to get the best k results efficiently
        let top_k_indices = top_k_smallest(&distances, k);

        // Build results from top-k indices
        top_k_indices
            .into_iter()
            .map(|(idx, score)| {
                let point = filtered_points[idx];
                let metadata = Self::extract_metadata(&point.payload);
                VectorSearchResult {
                    id: point.id.clone(),
                    score,
                    metadata,
                }
            })
            .collect()
    }

    /// Extract VectorMetadata from payload
    fn extract_metadata(payload: &Value) -> VectorMetadata {
        let get_string = |key: &str| -> Option<String> {
            payload
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        VectorMetadata {
            id: get_string("id").unwrap_or_default(),
            uri: get_string("uri").unwrap_or_default(),
            parent_uri: get_string("parent_uri"),
            context_type: get_string("context_type").unwrap_or_default(),
            is_leaf: payload
                .get("is_leaf")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            level: payload
                .get("level")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8)
                .unwrap_or(0),
            abstract_text: get_string("abstract_text"),
            name: get_string("name"),
            description: get_string("description"),
            created_at: get_string("created_at").unwrap_or_default(),
            active_count: payload
                .get("active_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_point(id: &str, vector: Vec<f32>, uri: &str) -> VectorPoint {
        let mut payload = serde_json::Map::new();
        payload.insert("id".to_string(), Value::String(id.to_string()));
        payload.insert("uri".to_string(), Value::String(uri.to_string()));
        payload.insert(
            "context_type".to_string(),
            Value::String("resource".to_string()),
        );
        payload.insert("is_leaf".to_string(), Value::Bool(true));
        payload.insert("level".to_string(), Value::Number(0.into()));
        payload.insert(
            "created_at".to_string(),
            Value::String("2024-01-01".to_string()),
        );

        VectorPoint {
            id: id.to_string(),
            vector,
            sparse_vector: None,
            payload: Value::Object(payload),
        }
    }

    #[tokio::test]
    async fn test_create_collection() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();
        let info = store.collection_info("test").await.unwrap();

        assert_eq!(info.name, "test");
        assert_eq!(info.dimension, 3);
        assert_eq!(info.count, 0);
    }

    #[tokio::test]
    async fn test_create_collection_duplicate() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params.clone()).await.unwrap();
        assert!(store.create_collection("test", 3, params).await.is_err());
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let point = create_test_point("p1", vec![1.0, 2.0, 3.0], "/test/file1");
        store.upsert("test", vec![point.clone()]).await.unwrap();

        let retrieved = store.get("test", "p1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "p1");
    }

    #[tokio::test]
    async fn test_upsert_wrong_dimension() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let point = create_test_point("p1", vec![1.0, 2.0], "/test/file1");
        assert!(store.upsert("test", vec![point]).await.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let point = create_test_point("p1", vec![1.0, 2.0, 3.0], "/test/file1");
        store.upsert("test", vec![point]).await.unwrap();

        store.delete("test", "p1").await.unwrap();
        assert!(store.get("test", "p1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_search() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
        let p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/file2");
        let p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/file3");

        store.upsert("test", vec![p1, p2, p3]).await.unwrap();

        // Search for vector closest to [1.0, 0.0, 0.0]
        let results = store.search("test", &[1.0, 0.0, 0.0], 2, None).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "p1"); // Closest
        assert!(results[0].score < results[1].score); // Lower score is better
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let mut p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
        let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/file2");

        // Add context_type to payload for filtering
        if let Value::Object(ref mut obj) = p1.payload {
            obj.insert(
                "context_type".to_string(),
                Value::String("resource".to_string()),
            );
        }
        if let Value::Object(ref mut obj) = p2.payload {
            obj.insert(
                "context_type".to_string(),
                Value::String("memory".to_string()),
            );
        }

        store.upsert("test", vec![p1, p2]).await.unwrap();

        let filter = Filter::Eq(
            "context_type".to_string(),
            Value::String("resource".to_string()),
        );
        let results = store
            .search("test", &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "p1");
    }

    #[tokio::test]
    async fn test_delete_by_uri_prefix() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/docs/file1");
        let p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/docs/subdir/file2");
        let p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/other/file3");

        store.upsert("test", vec![p1, p2, p3]).await.unwrap();

        store.delete_by_uri_prefix("test", "/docs").await.unwrap();

        assert!(store.get("test", "p1").await.unwrap().is_none());
        assert!(store.get("test", "p2").await.unwrap().is_none());
        assert!(store.get("test", "p3").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_update_uri() {
        let store = MemoryVectorStore::new();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/old/path/file1");
        let mut p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/old/path/subdir/file2");

        // Add parent_uri to p2
        if let Value::Object(ref mut obj) = p2.payload {
            obj.insert(
                "parent_uri".to_string(),
                Value::String("/old/path".to_string()),
            );
        }

        store.upsert("test", vec![p1, p2]).await.unwrap();

        store.update_uri("test", "/old/path", "/new/path").await.unwrap();

        let updated_p1 = store.get("test", "p1").await.unwrap().unwrap();
        let updated_p2 = store.get("test", "p2").await.unwrap().unwrap();

        assert_eq!(
            updated_p1.payload.get("uri").unwrap().as_str().unwrap(),
            "/new/path/file1"
        );
        assert_eq!(
            updated_p2.payload.get("uri").unwrap().as_str().unwrap(),
            "/new/path/subdir/file2"
        );
        assert_eq!(
            updated_p2
                .payload
                .get("parent_uri")
                .unwrap()
                .as_str()
                .unwrap(),
            "/new/path"
        );
    }

    #[tokio::test]
    async fn test_collection_info() {
        let store = MemoryVectorStore::new();
        let params = IndexParams {
            index_type: IndexType::Flat,
            distance: DistanceType::L2,
            ..Default::default()
        };

        store.create_collection("test", 128, params).await.unwrap();

        let info = store.collection_info("test").await.unwrap();
        assert_eq!(info.name, "test");
        assert_eq!(info.dimension, 128);
        assert_eq!(info.count, 0);
        assert_eq!(info.index_type, IndexType::Flat);
        assert_eq!(info.distance, DistanceType::L2);
    }

    #[test]
    fn test_name_and_version() {
        let store = MemoryVectorStore::new();
        assert_eq!(store.name(), "memory");
        assert_eq!(store.version(), "0.1.0");
    }

    #[tokio::test]
    async fn test_initialize() {
        let store = MemoryVectorStore::new();
        assert!(store.initialize(&Value::Null).await.is_ok());
    }
}
