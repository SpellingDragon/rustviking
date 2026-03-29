//! RocksDB-backed persistent VectorStore implementation
//!
//! Uses RocksDB for persistent storage of vectors with the following key scheme:
//! - `vs:meta:{collection}` - Collection metadata
//! - `vs:data:{collection}:{id}` - Vector data
//! - `vs:uri:{collection}:{uri}` - URI to ID index

use crate::compute::distance::DistanceComputer;
use crate::compute::simd::{top_k_smallest, PARALLEL_THRESHOLD};
use crate::error::{Result, RustVikingError};
use crate::storage::config::StorageConfig;
use crate::storage::kv::KvStore;
use crate::storage::rocks_kv::RocksKvStore;
use crate::vector_store::traits::VectorStore;
use crate::vector_store::types::*;
use async_trait::async_trait;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Collection metadata stored in RocksDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionMeta {
    name: String,
    dimension: usize,
    index_type: IndexType,
    distance: DistanceType,
    count: u64,
}

/// RocksDB-backed persistent vector store
pub struct RocksDBVectorStore {
    kv: RocksKvStore,
    /// Cache for collection metadata to avoid repeated deserialization
    /// Reserved for future optimization (e.g., caching collection metadata in memory)
    #[allow(dead_code)]
    meta_cache: RwLock<HashMap<String, CollectionMeta>>,
}

// Key encoding functions

/// Collection metadata key: `vs:meta:{collection}`
fn meta_key(collection: &str) -> Vec<u8> {
    format!("vs:meta:{}", collection).into_bytes()
}

/// Vector data key: `vs:data:{collection}:{id}`
fn data_key(collection: &str, id: &str) -> Vec<u8> {
    format!("vs:data:{}:{}", collection, id).into_bytes()
}

/// Data prefix for scanning all vectors in a collection: `vs:data:{collection}:`
fn data_prefix(collection: &str) -> Vec<u8> {
    format!("vs:data:{}:", collection).into_bytes()
}

/// URI index key: `vs:uri:{collection}:{uri}`
fn uri_key(collection: &str, uri: &str) -> Vec<u8> {
    format!("vs:uri:{}:{}", collection, uri).into_bytes()
}

/// URI prefix for scanning: `vs:uri:{collection}:{uri_prefix}`
fn uri_prefix_key(collection: &str, uri_prefix: &str) -> Vec<u8> {
    format!("vs:uri:{}:{}", collection, uri_prefix).into_bytes()
}

impl RocksDBVectorStore {
    /// Create a new RocksDBVectorStore with the given storage config
    pub fn new(config: &StorageConfig) -> Result<Self> {
        let kv = RocksKvStore::new(config)?;
        Ok(Self {
            kv,
            meta_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Create a new RocksDBVectorStore with a path string
    pub fn with_path(path: &str) -> Result<Self> {
        let config = StorageConfig {
            path: path.to_string(),
            create_if_missing: true,
            ..Default::default()
        };
        Self::new(&config)
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

    /// Get URI from payload if present
    fn get_uri_from_payload(payload: &Value) -> Option<String> {
        payload
            .get("uri")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Sequential search for small collections
    fn search_sequential(
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
        points: &[VectorPoint],
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
        points: &[VectorPoint],
        distance_type: DistanceType,
        dimension: usize,
    ) -> Vec<VectorSearchResult> {
        // First, filter points in parallel
        let filtered_points: Vec<&VectorPoint> = if let Some(ref filter) = filters {
            points
                .par_iter()
                .filter(|point| Self::matches_filter(point, filter))
                .collect()
        } else {
            points.par_iter().collect()
        };

        // Compute distances in parallel
        let distances: Vec<f32> = filtered_points
            .par_iter()
            .map(|point| {
                // Create a new DistanceComputer for each thread (DistanceComputer is not Send)
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
}

#[async_trait]
impl VectorStore for RocksDBVectorStore {
    fn name(&self) -> &str {
        "rocksdb"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn initialize(&self, _config: &Value) -> Result<()> {
        // RocksDB is already initialized in constructor
        Ok(())
    }

    async fn create_collection(
        &self,
        name: &str,
        dimension: usize,
        params: IndexParams,
    ) -> Result<()> {
        let name_owned = name.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Check if collection already exists
            let key = meta_key(&name_owned);
            if kv.get(&key)?.is_some() {
                return Err(RustVikingError::Storage(format!(
                    "Collection '{}' already exists",
                    name_owned
                )));
            }

            let meta = CollectionMeta {
                name: name_owned.clone(),
                dimension,
                index_type: params.index_type,
                distance: params.distance,
                count: 0,
            };

            let bytes = bincode::serialize(&meta)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
            kv.put(&key, &bytes)?;

            // Update cache (blocking context, but we need to handle the async RwLock)
            // Since we're in spawn_blocking, we use try_lock or skip cache update
            // For simplicity, we'll skip the cache update here and let subsequent reads populate it
            Ok(())
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn upsert(&self, collection: &str, points: Vec<VectorPoint>) -> Result<()> {
        let collection_owned = collection.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Load collection meta
            let key = meta_key(&collection_owned);
            let meta_bytes = kv.get(&key)?.ok_or_else(|| {
                RustVikingError::NotFound(format!("Collection '{}'", collection_owned))
            })?;
            let meta: CollectionMeta = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            let mut batch = kv.batch()?;
            let mut new_count = meta.count;

            for point in points {
                // Validate dimension
                if point.vector.len() != meta.dimension {
                    return Err(RustVikingError::InvalidDimension {
                        expected: meta.dimension,
                        actual: point.vector.len(),
                    });
                }

                // Check if this is a new point or an update
                let data_key_bytes = data_key(&collection_owned, &point.id);
                let is_new = kv.get(&data_key_bytes)?.is_none();

                // Serialize and store vector data using serde_json
                let data_bytes = serde_json::to_vec(&point)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                batch.put(data_key_bytes, data_bytes);

                // Update URI index if present
                if let Some(uri) = Self::get_uri_from_payload(&point.payload) {
                    let uri_key_bytes = uri_key(&collection_owned, &uri);
                    batch.put(uri_key_bytes, point.id.clone().into_bytes());
                }

                if is_new {
                    new_count += 1;
                }
            }

            // Commit batch
            batch.commit()?;

            // Update count if changed
            if new_count != meta.count {
                let updated_meta = CollectionMeta {
                    count: new_count,
                    ..meta
                };
                let updated_bytes = bincode::serialize(&updated_meta)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                kv.put(&key, &updated_bytes)?;
            }

            Ok(())
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn search(
        &self,
        collection: &str,
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
    ) -> Result<Vec<VectorSearchResult>> {
        let collection_owned = collection.to_string();
        let query_owned = query.to_vec();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Load collection meta
            let key = meta_key(&collection_owned);
            let meta_bytes = kv.get(&key)?.ok_or_else(|| {
                RustVikingError::NotFound(format!("Collection '{}'", collection_owned))
            })?;
            let meta: CollectionMeta = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            // Validate query dimension
            if query_owned.len() != meta.dimension {
                return Err(RustVikingError::InvalidDimension {
                    expected: meta.dimension,
                    actual: query_owned.len(),
                });
            }

            // Scan all vectors in the collection
            let prefix = data_prefix(&collection_owned);
            let entries = kv.scan_prefix(&prefix)?;

            // Deserialize all points first
            let points: Vec<VectorPoint> = entries
                .into_iter()
                .filter_map(|(_, value_bytes)| {
                    serde_json::from_slice::<VectorPoint>(&value_bytes).ok()
                })
                .collect();

            // Use parallel computation for large collections
            let results = if points.len() >= PARALLEL_THRESHOLD {
                Self::search_parallel(
                    &query_owned,
                    k,
                    filters,
                    &points,
                    meta.distance,
                    meta.dimension,
                )
            } else {
                Self::search_sequential(
                    &query_owned,
                    k,
                    filters,
                    &points,
                    meta.distance,
                    meta.dimension,
                )
            };

            Ok(results)
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn get(&self, collection: &str, id: &str) -> Result<Option<VectorPoint>> {
        let collection_owned = collection.to_string();
        let id_owned = id.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Verify collection exists
            let meta_key_bytes = meta_key(&collection_owned);
            if kv.get(&meta_key_bytes)?.is_none() {
                return Err(RustVikingError::NotFound(format!(
                    "Collection '{}'",
                    collection_owned
                )));
            }

            let key = data_key(&collection_owned, &id_owned);
            match kv.get(&key)? {
                Some(bytes) => {
                    let point: VectorPoint = serde_json::from_slice(&bytes)
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    Ok(Some(point))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let collection_owned = collection.to_string();
        let id_owned = id.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Load collection meta
            let meta_key_bytes = meta_key(&collection_owned);
            let meta_bytes = kv.get(&meta_key_bytes)?.ok_or_else(|| {
                RustVikingError::NotFound(format!("Collection '{}'", collection_owned))
            })?;
            let meta: CollectionMeta = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            // Get the point to check for URI index
            let data_key_bytes = data_key(&collection_owned, &id_owned);
            let point_exists = if let Some(bytes) = kv.get(&data_key_bytes)? {
                // Delete URI index if present
                let point: VectorPoint = serde_json::from_slice(&bytes)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                if let Some(uri) = Self::get_uri_from_payload(&point.payload) {
                    kv.delete(&uri_key(&collection_owned, &uri))?;
                }
                true
            } else {
                false
            };

            // Delete vector data
            kv.delete(&data_key_bytes)?;

            // Update count if point existed
            if point_exists && meta.count > 0 {
                let updated_meta = CollectionMeta {
                    count: meta.count - 1,
                    ..meta
                };
                let updated_bytes = bincode::serialize(&updated_meta)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                kv.put(&meta_key_bytes, &updated_bytes)?;
            }

            Ok(())
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn delete_by_uri_prefix(&self, collection: &str, uri_prefix: &str) -> Result<()> {
        let collection_owned = collection.to_string();
        let uri_prefix_owned = uri_prefix.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Load collection meta
            let meta_key_bytes = meta_key(&collection_owned);
            let meta_bytes = kv.get(&meta_key_bytes)?.ok_or_else(|| {
                RustVikingError::NotFound(format!("Collection '{}'", collection_owned))
            })?;
            let meta: CollectionMeta = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            // Find all matching URI index entries
            let prefix = uri_prefix_key(&collection_owned, &uri_prefix_owned);
            let uri_entries = kv.scan_prefix(&prefix)?;

            let mut batch = kv.batch()?;
            let mut deleted_count = 0u64;

            for (uri_key_bytes, id_bytes) in uri_entries {
                let id = String::from_utf8_lossy(&id_bytes);

                // Delete URI index
                batch.delete(uri_key_bytes);

                // Delete vector data
                let data_key_bytes = data_key(&collection_owned, &id);
                batch.delete(data_key_bytes);

                deleted_count += 1;
            }

            // Commit batch
            batch.commit()?;

            // Update count
            if deleted_count > 0 && meta.count >= deleted_count {
                let updated_meta = CollectionMeta {
                    count: meta.count - deleted_count,
                    ..meta
                };
                let updated_bytes = bincode::serialize(&updated_meta)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                kv.put(&meta_key_bytes, &updated_bytes)?;
            }

            Ok(())
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn update_uri(&self, collection: &str, old_uri: &str, new_uri: &str) -> Result<()> {
        let collection_owned = collection.to_string();
        let old_uri_owned = old_uri.to_string();
        let new_uri_owned = new_uri.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            // Verify collection exists
            let meta_key_bytes = meta_key(&collection_owned);
            if kv.get(&meta_key_bytes)?.is_none() {
                return Err(RustVikingError::NotFound(format!(
                    "Collection '{}'",
                    collection_owned
                )));
            }

            // Scan all vectors in the collection
            let prefix = data_prefix(&collection_owned);
            let entries = kv.scan_prefix(&prefix)?;

            let mut batch = kv.batch()?;

            for (key_bytes, value_bytes) in entries {
                // Deserialize vector point
                let mut point: VectorPoint = serde_json::from_slice(&value_bytes)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

                let mut updated = false;
                let mut old_uri_value = None;

                // Update uri field if it starts with old_uri (prefix match)
                if let Some(Value::String(uri)) = point.payload.get("uri") {
                    if let Some(stripped) = uri.strip_prefix(&old_uri_owned) {
                        let new_uri_value = format!("{}{}", new_uri_owned, stripped);
                        old_uri_value = Some(uri.clone());
                        if let Some(obj) = point.payload.as_object_mut() {
                            obj.insert("uri".to_string(), Value::String(new_uri_value));
                        }
                        updated = true;
                    }
                }

                // Update parent_uri if it starts with old_uri
                if let Some(Value::String(parent_uri)) = point.payload.get("parent_uri") {
                    if let Some(stripped) = parent_uri.strip_prefix(&old_uri_owned) {
                        let new_parent_uri = format!("{}{}", new_uri_owned, stripped);
                        if let Some(obj) = point.payload.as_object_mut() {
                            obj.insert("parent_uri".to_string(), Value::String(new_parent_uri));
                        }
                        updated = true;
                    }
                }

                if updated {
                    // Delete old URI index if present
                    if let Some(old_uri_str) = old_uri_value {
                        batch.delete(uri_key(&collection_owned, &old_uri_str));
                    }

                    // Add new URI index if present
                    if let Some(new_uri_value) = Self::get_uri_from_payload(&point.payload) {
                        batch.put(
                            uri_key(&collection_owned, &new_uri_value),
                            point.id.clone().into_bytes(),
                        );
                    }

                    // Serialize and update vector data
                    let new_data_bytes = serde_json::to_vec(&point)
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    batch.put(key_bytes, new_data_bytes);
                }
            }

            // Commit batch
            batch.commit()
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }

    async fn collection_info(&self, collection: &str) -> Result<CollectionInfo> {
        let collection_owned = collection.to_string();
        let kv = self.kv.clone();

        tokio::task::spawn_blocking(move || {
            let key = meta_key(&collection_owned);
            let meta_bytes = kv.get(&key)?.ok_or_else(|| {
                RustVikingError::NotFound(format!("Collection '{}'", collection_owned))
            })?;
            let meta: CollectionMeta = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            Ok(CollectionInfo {
                name: meta.name,
                dimension: meta.dimension,
                count: meta.count,
                index_type: meta.index_type,
                distance: meta.distance,
            })
        })
        .await
        .map_err(|e| RustVikingError::Internal(format!("spawn_blocking error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

    fn create_test_store() -> (RocksDBVectorStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = RocksDBVectorStore::with_path(temp_dir.path().to_str().unwrap()).unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_create_collection() {
        let (store, _temp) = create_test_store();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();
        let info = store.collection_info("test").await.unwrap();

        assert_eq!(info.name, "test");
        assert_eq!(info.dimension, 3);
        assert_eq!(info.count, 0);
    }

    #[tokio::test]
    async fn test_create_collection_duplicate() {
        let (store, _temp) = create_test_store();
        let params = IndexParams::default();

        store
            .create_collection("test", 3, params.clone())
            .await
            .unwrap();
        assert!(store.create_collection("test", 3, params).await.is_err());
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
        assert_eq!(retrieved.unwrap().id, "p1");

        // Verify count updated
        let info = store.collection_info("test").await.unwrap();
        assert_eq!(info.count, 1);
    }

    #[tokio::test]
    async fn test_upsert_wrong_dimension() {
        let (store, _temp) = create_test_store();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let point = create_test_point("p1", vec![1.0, 2.0], "/test/file1");
        assert!(store.upsert("test", vec![point]).await.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let (store, _temp) = create_test_store();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let point = create_test_point("p1", vec![1.0, 2.0, 3.0], "/test/file1");
        store.upsert("test", vec![point]).await.unwrap();

        store.delete("test", "p1").await.unwrap();
        assert!(store.get("test", "p1").await.unwrap().is_none());

        // Verify count updated
        let info = store.collection_info("test").await.unwrap();
        assert_eq!(info.count, 0);
    }

    #[tokio::test]
    async fn test_search() {
        let (store, _temp) = create_test_store();
        let params = IndexParams::default();

        store.create_collection("test", 3, params).await.unwrap();

        let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
        let p2 = create_test_point("p2", vec![0.0, 1.0, 0.0], "/test/file2");
        let p3 = create_test_point("p3", vec![0.0, 0.0, 1.0], "/test/file3");

        store.upsert("test", vec![p1, p2, p3]).await.unwrap();

        // Search for vector closest to [1.0, 0.0, 0.0]
        let results = store
            .search("test", &[1.0, 0.0, 0.0], 2, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "p1"); // Closest
        assert!(results[0].score < results[1].score); // Lower score is better
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let (store, _temp) = create_test_store();
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
        let (store, _temp) = create_test_store();
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

        // Verify count updated
        let info = store.collection_info("test").await.unwrap();
        assert_eq!(info.count, 1);
    }

    #[tokio::test]
    async fn test_update_uri() {
        let (store, _temp) = create_test_store();
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

        store
            .update_uri("test", "/old/path", "/new/path")
            .await
            .unwrap();

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
        let (store, _temp) = create_test_store();
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
        let (store, _temp) = create_test_store();
        assert_eq!(store.name(), "rocksdb");
        assert_eq!(store.version(), "0.1.0");
    }

    #[tokio::test]
    async fn test_initialize() {
        let (store, _temp) = create_test_store();
        assert!(store.initialize(&Value::Null).await.is_ok());
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create store and add data
        {
            let store = RocksDBVectorStore::with_path(path).unwrap();
            let params = IndexParams::default();
            store.create_collection("test", 3, params).await.unwrap();

            let p1 = create_test_point("p1", vec![1.0, 0.0, 0.0], "/test/file1");
            store.upsert("test", vec![p1]).await.unwrap();
        }

        // Reopen store and verify data persists
        {
            let store = RocksDBVectorStore::with_path(path).unwrap();
            let info = store.collection_info("test").await.unwrap();
            assert_eq!(info.count, 1);

            let retrieved = store.get("test", "p1").await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().id, "p1");
        }
    }
}
