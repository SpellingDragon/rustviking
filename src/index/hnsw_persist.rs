//! HNSW Index Persistence
//!
//! Provides persistence capabilities for HNSW indexes.
//! Uses hnsw_rs built-in serialization for the graph structure,
//! and RocksDB for metadata (ID mappings, levels, etc.)

use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Result, RustVikingError};
use crate::index::hnsw::HnswIndex;
use crate::index::vector::{HnswParams, VectorIndex};

/// Key prefixes for metadata storage
mod keys {
    pub const CONFIG: &[u8] = b"hnsw:cfg";
    pub const ID_MAP_PREFIX: &[u8] = b"hnsw:map:";
    pub const LEVEL_PREFIX: &[u8] = b"hnsw:lvl:";
    pub const VECTOR_PREFIX: &[u8] = b"hnsw:vec:";
    pub const NEXT_ID: &[u8] = b"hnsw:next_id";
}

/// HNSW graph file basename
const HNSW_BASENAME: &str = "hnsw_index";

/// Configuration for HNSW index persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswPersistConfig {
    /// Vector dimension
    pub dimension: usize,
    /// HNSW parameters
    pub params: HnswParams,
    /// Next internal ID
    pub next_id: usize,
}

/// Persister for HNSW indexes
pub struct HnswIndexPersister {
    /// RocksDB for metadata storage
    db: Arc<DB>,
    /// Base path for hnsw graph files
    base_path: PathBuf,
}

impl HnswIndexPersister {
    /// Create a new persister at the given path
    ///
    /// # Arguments
    /// * `path` - Directory path for storage
    pub fn new(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, path).map_err(|e| RustVikingError::Storage(e.to_string()))?;

        Ok(Self {
            db: Arc::new(db),
            base_path: path.to_path_buf(),
        })
    }

    /// Persist an entire HnswIndex
    pub fn persist_index(&self, index: &HnswIndex) -> Result<()> {
        // Get and persist config
        let config = self.extract_config(index)?;
        self.persist_config(&config)?;

        // Persist ID mappings
        let id_map = index.get_id_map()?;
        for (external_id, internal_id) in id_map.iter() {
            self.persist_id_mapping(*external_id, *internal_id)?;
        }

        // Persist reverse mappings
        let reverse_map = index.get_reverse_map()?;
        for (internal_id, external_id) in reverse_map.iter() {
            self.persist_reverse_mapping(*internal_id, *external_id)?;
        }

        // Persist levels
        let levels = index.get_levels()?;
        for (id, level) in levels.iter() {
            self.persist_level(*id, *level)?;
        }

        // Persist vectors
        let vectors = index.get_vectors()?;
        for (id, vector) in vectors.iter() {
            self.persist_vector(*id, vector)?;
        }

        // Persist next_id
        self.persist_next_id(index.get_next_id_value())?;

        // Dump HNSW graph using hnsw_rs built-in serialization
        index.dump_graph(&self.base_path, HNSW_BASENAME)?;

        Ok(())
    }

    /// Restore an HnswIndex
    pub fn restore_index(&self) -> Result<HnswIndex> {
        let config = self
            .restore_config()?
            .ok_or_else(|| RustVikingError::Storage("No HNSW config found".into()))?;

        // Create a new index
        let index = HnswIndex::new(config.params.clone(), config.dimension);

        // Restore vectors and metadata
        let id_map = self.restore_id_map()?;
        let levels = self.restore_levels()?;
        let vectors = self.restore_vectors()?;
        let next_id = self.restore_next_id()?;

        // Re-insert vectors into the HNSW graph
        // We need to insert in the same order as the original (sorted by internal_id)
        let mut entries: Vec<(usize, u64, Vec<f32>, u8)> = id_map
            .iter()
            .filter_map(|(external_id, internal_id)| {
                vectors.get(external_id).map(|v| {
                    let level = levels.get(external_id).copied().unwrap_or(2);
                    (*internal_id, *external_id, v.clone(), level)
                })
            })
            .collect();
        entries.sort_by_key(|e| e.0);

        // Insert vectors back into the graph
        for (_internal_id, external_id, vector, level) in entries {
            index.insert(external_id, &vector, level)?;
        }

        // Set next_id to the correct value
        index.set_next_id(next_id)?;

        Ok(index)
    }

    /// Persist configuration
    pub fn persist_config(&self, config: &HnswPersistConfig) -> Result<()> {
        let value = bincode::serialize(config)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(keys::CONFIG, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Restore configuration
    pub fn restore_config(&self) -> Result<Option<HnswPersistConfig>> {
        match self
            .db
            .get(keys::CONFIG)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?
        {
            Some(bytes) => {
                let config: HnswPersistConfig = bincode::deserialize(&bytes)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    // ========================================
    // Private helper methods
    // ========================================

    fn extract_config(&self, index: &HnswIndex) -> Result<HnswPersistConfig> {
        let (dimension, params, next_id) = index.get_persist_config();
        Ok(HnswPersistConfig {
            dimension,
            params,
            next_id,
        })
    }

    fn persist_id_mapping(&self, external_id: u64, internal_id: usize) -> Result<()> {
        let key = format!("hnsw:map:{}", external_id).into_bytes();
        let value = bincode::serialize(&internal_id)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&key, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    fn persist_reverse_mapping(&self, internal_id: usize, external_id: u64) -> Result<()> {
        let key = format!("hnsw:rmap:{}", internal_id).into_bytes();
        let value = bincode::serialize(&external_id)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&key, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    fn persist_level(&self, id: u64, level: u8) -> Result<()> {
        let key = format!("hnsw:lvl:{}", id).into_bytes();
        let value = bincode::serialize(&level)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&key, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    fn persist_vector(&self, id: u64, vector: &[f32]) -> Result<()> {
        let key = format!("hnsw:vec:{}", id).into_bytes();
        let value = bincode::serialize(vector)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(&key, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    fn persist_next_id(&self, next_id: usize) -> Result<()> {
        let value = bincode::serialize(&next_id)
            .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
        self.db
            .put(keys::NEXT_ID, &value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;
        Ok(())
    }

    fn restore_id_map(&self) -> Result<HashMap<u64, usize>> {
        let mut map = HashMap::new();
        let iter = self.db.prefix_iterator(keys::ID_MAP_PREFIX);
        for item in iter {
            let (key, value) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            if key.starts_with(keys::ID_MAP_PREFIX) {
                let key_str = std::str::from_utf8(&key)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                let parts: Vec<&str> = key_str.split(':').collect();
                if parts.len() >= 3 {
                    let external_id: u64 = parts[2]
                        .parse::<u64>()
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    let internal_id: usize = bincode::deserialize(&value)
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    map.insert(external_id, internal_id);
                }
            }
        }
        Ok(map)
    }

    fn restore_levels(&self) -> Result<HashMap<u64, u8>> {
        let mut map = HashMap::new();
        let iter = self.db.prefix_iterator(keys::LEVEL_PREFIX);
        for item in iter {
            let (key, value) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            if key.starts_with(keys::LEVEL_PREFIX) {
                let key_str = std::str::from_utf8(&key)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                let parts: Vec<&str> = key_str.split(':').collect();
                if parts.len() >= 3 {
                    let id: u64 = parts[2]
                        .parse::<u64>()
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    let level: u8 = bincode::deserialize(&value).map_err(|e: bincode::Error| {
                        RustVikingError::Serialization(e.to_string())
                    })?;
                    map.insert(id, level);
                }
            }
        }
        Ok(map)
    }

    fn restore_vectors(&self) -> Result<HashMap<u64, Vec<f32>>> {
        let mut map = HashMap::new();
        let iter = self.db.prefix_iterator(keys::VECTOR_PREFIX);
        for item in iter {
            let (key, value) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            if key.starts_with(keys::VECTOR_PREFIX) {
                let key_str = std::str::from_utf8(&key)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                let parts: Vec<&str> = key_str.split(':').collect();
                if parts.len() >= 3 {
                    let id: u64 = parts[2]
                        .parse::<u64>()
                        .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                    let vector: Vec<f32> =
                        bincode::deserialize(&value).map_err(|e: bincode::Error| {
                            RustVikingError::Serialization(e.to_string())
                        })?;
                    map.insert(id, vector);
                }
            }
        }
        Ok(map)
    }

    fn restore_next_id(&self) -> Result<usize> {
        match self
            .db
            .get(keys::NEXT_ID)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?
        {
            Some(bytes) => {
                let next_id: usize = bincode::deserialize(&bytes)
                    .map_err(|e| RustVikingError::Serialization(e.to_string()))?;
                Ok(next_id)
            }
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::vector::{MetricType, VectorIndex};
    use tempfile::tempdir;

    #[test]
    fn test_hnsw_persist_restore_config() {
        let dir = tempdir().unwrap();
        let persister = HnswIndexPersister::new(dir.path()).unwrap();

        let config = HnswPersistConfig {
            dimension: 128,
            params: HnswParams {
                m: 16,
                ef_construction: 200,
                ef_search: 50,
                metric: MetricType::L2,
            },
            next_id: 42,
        };

        persister.persist_config(&config).unwrap();
        let restored = persister.restore_config().unwrap();

        assert!(restored.is_some());
        let restored = restored.unwrap();
        assert_eq!(restored.dimension, 128);
        assert_eq!(restored.params.m, 16);
        assert_eq!(restored.next_id, 42);
    }

    #[test]
    fn test_hnsw_persist_restore_roundtrip() {
        let dir = tempdir().unwrap();

        // Create and populate index
        let params = HnswParams {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            metric: MetricType::L2,
        };
        let index = HnswIndex::new(params, 3);

        index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
        index.insert(2, &[0.0, 1.0, 0.0], 1).unwrap();
        index.insert(3, &[0.0, 0.0, 1.0], 0).unwrap();

        let original_count = index.count();

        // Persist
        let persister = HnswIndexPersister::new(dir.path()).unwrap();
        persister.persist_index(&index).unwrap();

        // Restore
        let restored = persister.restore_index().unwrap();

        // Verify
        assert_eq!(restored.count(), original_count);
        assert_eq!(restored.dimension(), 3);

        // Verify search works
        let results = restored.search(&[1.0, 0.0, 0.0], 2, None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hnsw_persist_with_level_filter() {
        let dir = tempdir().unwrap();

        let params = HnswParams::default();
        let index = HnswIndex::new(params, 3);

        index.insert(1, &[1.0, 0.0, 0.0], 0).unwrap();
        index.insert(2, &[0.0, 1.0, 0.0], 1).unwrap();
        index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap();

        // Persist and restore
        let persister = HnswIndexPersister::new(dir.path()).unwrap();
        persister.persist_index(&index).unwrap();
        let restored = persister.restore_index().unwrap();

        // Verify level filter works
        let results = restored.search(&[1.0, 0.0, 0.0], 10, Some(0)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[0].level, 0);
    }
}
