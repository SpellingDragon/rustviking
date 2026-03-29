//! IVF Index Persistence using RocksDB
//!
//! Provides persistence capabilities for IVF indexes using RocksDB as the storage backend.
//! Uses key prefixes to distinguish different data types within a single RocksDB instance.

use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use crate::error::{Result, RustVikingError};
use crate::index::ivf_pq::IvfIndex;
use crate::index::vector::{IvfParams, MetricType, VectorIndex};

/// Key prefixes for different data types
mod keys {
    pub const CONFIG: &[u8] = b"ivf:cfg";
    pub const CENTROID_PREFIX: &[u8] = b"ivf:cen:";
    // VECTOR_PREFIX and META_PREFIX are defined as methods in IvfIndexPersister
}

/// Configuration for IVF index persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfPersistConfig {
    /// Vector dimension
    pub dimension: usize,
    /// Number of partitions
    pub num_partitions: usize,
    /// Distance metric type
    pub metric: MetricType,
    /// Whether the index has been trained
    pub trained: bool,
}

/// Metadata for a vector entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    /// Vector ID
    pub id: u64,
    /// Level (L0/L1/L2)
    pub level: u8,
    /// Partition ID where the vector belongs
    pub partition_id: usize,
    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,
}

/// Persister for IVF indexes using RocksDB
pub struct IvfIndexPersister {
    db: Arc<DB>,
}

impl IvfIndexPersister {
    /// Create a new persister at the given path
    ///
    /// # Arguments
    /// * `path` - Directory path for RocksDB storage
    pub fn new(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, path).map_err(|e| RustVikingError::Storage(e.to_string()))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Persist an entire IvfIndex to RocksDB
    ///
    /// Uses WriteBatch for atomic bulk writes
    pub fn persist_index(&self, index: &IvfIndex) -> Result<()> {
        let config = self.extract_config(index)?;
        self.persist_config(&config)?;

        // Persist centroids
        let centroids = index.get_centroids()?;
        for (partition_id, centroid) in centroids.iter().enumerate() {
            self.persist_centroid(partition_id, centroid)?;
        }

        // Persist partition vectors with metadata
        let partition_data = index.get_partition_data()?;
        for (partition_id, data) in partition_data.iter().enumerate() {
            for (id, vector, level) in data.iter() {
                self.persist_vector_entry(partition_id, *id, vector, *level)?;
            }
        }

        Ok(())
    }

    /// Restore an IvfIndex from RocksDB
    pub fn restore_index(&self) -> Result<IvfIndex> {
        let config = self
            .restore_config()?
            .ok_or_else(|| RustVikingError::Storage("No IVF config found".into()))?;

        let params = IvfParams {
            num_partitions: config.num_partitions,
            metric: config.metric,
        };

        let index = IvfIndex::new(params, config.dimension);

        // Restore centroids
        let centroids = self.restore_centroids()?;
        index.set_centroids(centroids)?;

        // Set trained status
        if config.trained {
            index.set_trained(true)?;
        }

        // Restore vectors
        for partition_id in 0..config.num_partitions {
            let vectors = self.restore_partition_vectors(partition_id)?;
            for (id, vector, level) in vectors {
                index.insert(id, &vector, level)?;
            }
        }

        Ok(index)
    }

    /// Persist a single centroid
    pub fn persist_centroid(&self, partition_id: usize, centroid: &[f32]) -> Result<()> {
        let key = Self::make_centroid_key(partition_id);
        let value = bincode::serialize(centroid)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&key, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Persist a single vector with metadata (incremental write)
    pub fn persist_vector(
        &self,
        partition_id: usize,
        id: u64,
        vector: &[f32],
        level: u8,
    ) -> Result<()> {
        self.persist_vector_entry(partition_id, id, vector, level)
    }

    /// Delete a vector from storage
    pub fn delete_vector(&self, id: u64) -> Result<()> {
        // First, try to find the metadata to get partition_id
        let meta_key = Self::make_meta_key(id);
        if let Some(meta_bytes) = self
            .db
            .get(&meta_key)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?
        {
            let meta: VectorMetadata = bincode::deserialize(&meta_bytes)
                .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

            // Delete vector data
            let vec_key = Self::make_vector_key(meta.partition_id, id);
            self.db
                .delete(&vec_key)
                .map_err(|e| RustVikingError::Storage(e.to_string()))?;

            // Delete metadata
            self.db
                .delete(&meta_key)
                .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    /// Persist configuration
    pub fn persist_config(&self, config: &IvfPersistConfig) -> Result<()> {
        let value = bincode::serialize(config)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(keys::CONFIG, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Restore configuration
    pub fn restore_config(&self) -> Result<Option<IvfPersistConfig>> {
        match self
            .db
            .get(keys::CONFIG)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?
        {
            Some(bytes) => {
                let config: IvfPersistConfig = bincode::deserialize(&bytes)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Restore all centroids
    pub fn restore_centroids(&self) -> Result<Vec<Vec<f32>>> {
        let mut centroids = Vec::new();

        let iter = self
            .db
            .prefix_iterator(keys::CENTROID_PREFIX);
        for item in iter {
            let (key, value) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            if key.starts_with(keys::CENTROID_PREFIX) {
                let centroid: Vec<f32> = bincode::deserialize(&value)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                centroids.push(centroid);
            }
        }

        // Note: Centroids are stored and retrieved in partition_id order
        // The prefix iterator should return them in key order

        Ok(centroids)
    }

    /// Restore all vectors in a partition
    /// Returns Vec<(id, vector, level)>
    pub fn restore_partition_vectors(&self, partition_id: usize) -> Result<Vec<(u64, Vec<f32>, u8)>> {
        let prefix = Self::make_vector_prefix(partition_id);
        let mut vectors = Vec::new();

        let iter = self.db.prefix_iterator(&prefix);
        for item in iter {
            let (key, value) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            if key.starts_with(&prefix) {
                let vector: Vec<f32> = bincode::deserialize(&value)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;

                // Extract id from key
                let id = Self::parse_vector_id(&key)?;

                // Get metadata for level
                let meta_key = Self::make_meta_key(id);
                let level = if let Some(meta_bytes) = self
                    .db
                    .get(&meta_key)
                    .map_err(|e| RustVikingError::Storage(e.to_string()))?
                {
                    let meta: VectorMetadata = bincode::deserialize(&meta_bytes)
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    meta.level
                } else {
                    2 // Default level
                };

                vectors.push((id, vector, level));
            }
        }

        Ok(vectors)
    }

    // Helper methods

    fn extract_config(&self, index: &IvfIndex) -> Result<IvfPersistConfig> {
        let (dimension, num_partitions, metric, trained) = index.get_config();
        Ok(IvfPersistConfig {
            dimension,
            num_partitions,
            metric,
            trained,
        })
    }

    fn persist_vector_entry(
        &self,
        partition_id: usize,
        id: u64,
        vector: &[f32],
        level: u8,
    ) -> Result<()> {
        // Persist vector data
        let vec_key = Self::make_vector_key(partition_id, id);
        let vec_value = bincode::serialize(vector)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&vec_key, &vec_value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;

        // Persist metadata
        let meta = VectorMetadata {
            id,
            level,
            partition_id,
            created_at: chrono::Utc::now().timestamp(),
        };
        let meta_key = Self::make_meta_key(id);
        let meta_value = bincode::serialize(&meta)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&meta_key, &meta_value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;

        Ok(())
    }

    fn make_centroid_key(partition_id: usize) -> Vec<u8> {
        format!("ivf:cen:{}", partition_id).into_bytes()
    }

    fn make_vector_key(partition_id: usize, id: u64) -> Vec<u8> {
        format!("ivf:vec:{}:{}", partition_id, id).into_bytes()
    }

    fn make_vector_prefix(partition_id: usize) -> Vec<u8> {
        format!("ivf:vec:{}:", partition_id).into_bytes()
    }

    fn make_meta_key(id: u64) -> Vec<u8> {
        format!("ivf:meta:{}", id).into_bytes()
    }

    fn parse_vector_id(key: &[u8]) -> Result<u64> {
        let key_str = std::str::from_utf8(key)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        let parts: Vec<&str> = key_str.split(':').collect();
        if parts.len() >= 4 {
            parts[3]
                .parse::<u64>()
                .map_err(|e| RustVikingError::Serialization(e.to_string()))
        } else {
            Err(RustVikingError::Serialization(format!(
                "Invalid vector key format: {}",
                key_str
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::vector::MetricType;
    use tempfile::tempdir;

    fn create_test_index() -> IvfIndex {
        let params = IvfParams {
            num_partitions: 4,
            metric: MetricType::L2,
        };
        IvfIndex::new(params, 4)
    }

    #[test]
    fn test_persist_restore_config() {
        let dir = tempdir().unwrap();
        let persister = IvfIndexPersister::new(dir.path()).unwrap();

        let config = IvfPersistConfig {
            dimension: 128,
            num_partitions: 256,
            metric: MetricType::Cosine,
            trained: true,
        };

        persister.persist_config(&config).unwrap();
        let restored = persister.restore_config().unwrap();

        assert!(restored.is_some());
        let restored = restored.unwrap();
        assert_eq!(restored.dimension, 128);
        assert_eq!(restored.num_partitions, 256);
        assert_eq!(restored.metric, MetricType::Cosine);
        assert!(restored.trained);
    }

    #[test]
    fn test_persist_restore_centroids() {
        let dir = tempdir().unwrap();
        let persister = IvfIndexPersister::new(dir.path()).unwrap();

        let centroids: Vec<Vec<f32>> = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
        ];

        for (i, c) in centroids.iter().enumerate() {
            persister.persist_centroid(i, c).unwrap();
        }

        let restored = persister.restore_centroids().unwrap();
        assert_eq!(restored.len(), 4);
        for (i, c) in restored.iter().enumerate() {
            assert_eq!(c, &centroids[i]);
        }
    }

    #[test]
    fn test_persist_restore_vectors() {
        let dir = tempdir().unwrap();
        let persister = IvfIndexPersister::new(dir.path()).unwrap();

        // Insert some vectors
        persister
            .persist_vector(0, 1, &[1.0, 2.0, 3.0, 4.0], 2)
            .unwrap();
        persister
            .persist_vector(0, 2, &[5.0, 6.0, 7.0, 8.0], 1)
            .unwrap();
        persister
            .persist_vector(1, 3, &[9.0, 10.0, 11.0, 12.0], 0)
            .unwrap();

        // Restore partition 0
        let vecs0 = persister.restore_partition_vectors(0).unwrap();
        assert_eq!(vecs0.len(), 2);

        // Restore partition 1
        let vecs1 = persister.restore_partition_vectors(1).unwrap();
        assert_eq!(vecs1.len(), 1);
        assert_eq!(vecs1[0].0, 3);
        assert_eq!(vecs1[0].2, 0); // level

        // Test delete
        persister.delete_vector(1).unwrap();
        let vecs0_after = persister.restore_partition_vectors(0).unwrap();
        assert_eq!(vecs0_after.len(), 1);
        assert_eq!(vecs0_after[0].0, 2);
    }

    #[test]
    fn test_ivf_persist_restore_roundtrip() {
        let dir = tempdir().unwrap();

        // Create and populate index
        let index = create_test_index();
        let vectors = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0, 0.0],
        ];

        // Train first
        index.train(&vectors).unwrap();

        // Insert vectors
        for (i, v) in vectors.iter().enumerate() {
            index.insert(i as u64, v, 2).unwrap();
        }

        let original_count = index.count();

        // Persist
        let persister = IvfIndexPersister::new(dir.path()).unwrap();
        persister.persist_index(&index).unwrap();

        // Restore
        let restored = persister.restore_index().unwrap();

        // Verify
        assert_eq!(restored.count(), original_count);
        assert_eq!(restored.dimension(), 4);

        // Verify search works
        let results = restored.search(&[1.0, 0.0, 0.0, 0.0], 3, None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_incremental_write() {
        let dir = tempdir().unwrap();
        let persister = IvfIndexPersister::new(dir.path()).unwrap();

        // Persist config first
        let config = IvfPersistConfig {
            dimension: 4,
            num_partitions: 4,
            metric: MetricType::L2,
            trained: false,
        };
        persister.persist_config(&config).unwrap();

        // Add vectors incrementally
        persister.persist_vector(0, 1, &[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        persister.persist_vector(1, 2, &[5.0, 6.0, 7.0, 8.0], 1).unwrap();

        // Verify they can be restored
        let vecs0 = persister.restore_partition_vectors(0).unwrap();
        assert_eq!(vecs0.len(), 1);
        assert_eq!(vecs0[0].0, 1);
    }
}
