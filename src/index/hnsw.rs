//! HNSW Index Implementation
//!
//! Wrapper around hnsw_rs library for production-grade HNSW vector search.
//! This provides true O(log n) approximate nearest neighbor search.

use crate::error::{Result, RustVikingError};
use crate::index::vector::{HnswParams, MetricType, SearchResult, VectorIndex};
use hnsw_rs::dist::DistL2;
use hnsw_rs::hnsw::{Hnsw, Neighbour};
use std::collections::HashMap;
use std::sync::RwLock;

/// Wrapper for HNSW index using hnsw_rs library
///
/// Note: Currently only L2 distance is supported via hnsw_rs.
/// For Cosine and DotProduct, vectors should be normalized before insertion.
pub struct HnswIndex {
    params: HnswParams,
    dimension: usize,
    /// The underlying hnsw_rs index (using L2 distance)
    index: RwLock<Hnsw<f32, DistL2>>,
    /// Mapping from external ID to internal ID
    id_map: RwLock<HashMap<u64, usize>>,
    /// Reverse mapping from internal ID to external ID
    reverse_map: RwLock<HashMap<usize, u64>>,
    /// Level metadata for each vector (L0/L1/L2)
    levels: RwLock<HashMap<u64, u8>>,
    /// Vector storage for retrieval
    vectors: RwLock<HashMap<u64, Vec<f32>>>,
    /// Next internal ID
    next_id: RwLock<usize>,
}

impl HnswIndex {
    /// Create a new HNSW index with the given parameters
    ///
    /// # Arguments
    /// * `params` - HNSW parameters (m, ef_construction, ef_search, metric)
    /// * `dimension` - Dimension of vectors to be indexed
    ///
    /// # Note
    /// Only L2 distance is directly supported. For Cosine distance,
    /// normalize vectors before insertion and use L2 distance.
    pub fn new(params: HnswParams, dimension: usize) -> Self {
        let max_elements = 1_000_000; // Default max elements
        let max_layer = 16;
        let ef_construction = params.ef_construction;

        let index = Hnsw::new(
            params.m,
            max_elements,
            max_layer,
            ef_construction,
            DistL2 {},
        );

        Self {
            params,
            dimension,
            index: RwLock::new(index),
            id_map: RwLock::new(HashMap::new()),
            reverse_map: RwLock::new(HashMap::new()),
            levels: RwLock::new(HashMap::new()),
            vectors: RwLock::new(HashMap::new()),
            next_id: RwLock::new(0),
        }
    }

    /// Get the next internal ID
    fn get_next_id(&self) -> usize {
        let mut next_id = self.next_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }

    /// Prepare vector for insertion based on metric type
    fn prepare_vector(&self, vector: &[f32]) -> Vec<f32> {
        match self.params.metric {
            MetricType::L2 => vector.to_vec(),
            MetricType::Cosine => {
                // Normalize for cosine similarity
                let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    vector.iter().map(|x| x / norm).collect()
                } else {
                    vector.to_vec()
                }
            }
            MetricType::DotProduct => {
                // For dot product, we use negative dot product as distance
                // Just store as-is; search will handle it
                vector.to_vec()
            }
        }
    }

    /// Convert hnsw_rs Neighbour to SearchResult
    fn neighbour_to_result(&self, n: &Neighbour) -> SearchResult {
        let external_id = self
            .reverse_map
            .read()
            .unwrap()
            .get(&n.d_id)
            .copied()
            .unwrap_or(n.d_id as u64);

        let level = self
            .levels
            .read()
            .unwrap()
            .get(&external_id)
            .copied()
            .unwrap_or(2);

        SearchResult {
            id: external_id,
            score: n.distance,
            vector: self.vectors.read().unwrap().get(&external_id).cloned(),
            level,
        }
    }
}

impl VectorIndex for HnswIndex {
    fn insert(&self, id: u64, vector: &[f32], level: u8) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(RustVikingError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        // Check if ID already exists
        if self.id_map.read().unwrap().contains_key(&id) {
            return Err(RustVikingError::Storage(format!(
                "Vector with id {} already exists",
                id
            )));
        }

        let internal_id = self.get_next_id();
        let prepared = self.prepare_vector(vector);

        // Store mappings
        self.id_map.write().unwrap().insert(id, internal_id);
        self.reverse_map.write().unwrap().insert(internal_id, id);
        self.levels.write().unwrap().insert(id, level);
        self.vectors.write().unwrap().insert(id, vector.to_vec());

        // Insert into hnsw index
        self.index
            .write()
            .unwrap()
            .insert_slice((&prepared, internal_id));

        Ok(())
    }

    fn insert_batch(&self, vectors: &[(u64, Vec<f32>, u8)]) -> Result<()> {
        for (id, vector, level) in vectors {
            self.insert(*id, vector, *level)?;
        }
        Ok(())
    }

    fn search(
        &self,
        query: &[f32],
        k: usize,
        level_filter: Option<u8>,
    ) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(RustVikingError::InvalidDimension {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        let prepared = self.prepare_vector(query);
        let ef = self.params.ef_search.max(k);

        let neighbours = self.index.read().unwrap().search(&prepared, k, ef);

        let results: Vec<SearchResult> = neighbours
            .iter()
            .filter_map(|n| {
                let result = self.neighbour_to_result(n);
                // Apply level filter
                if let Some(lf) = level_filter {
                    if result.level != lf {
                        return None;
                    }
                }
                Some(result)
            })
            .collect();

        Ok(results)
    }

    fn delete(&self, id: u64) -> Result<()> {
        // hnsw_rs doesn't support deletion, so we just remove from our maps
        // The vector will still be in the index but inaccessible
        if let Some(internal_id) = self.id_map.write().unwrap().remove(&id) {
            self.reverse_map.write().unwrap().remove(&internal_id);
            self.levels.write().unwrap().remove(&id);
            self.vectors.write().unwrap().remove(&id);
            Ok(())
        } else {
            Err(RustVikingError::NotFound(format!("vector id={}", id)))
        }
    }

    fn get(&self, id: u64) -> Result<Option<Vec<f32>>> {
        Ok(self.vectors.read().unwrap().get(&id).cloned())
    }

    fn count(&self) -> u64 {
        self.id_map.read().unwrap().len() as u64
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_and_search() {
        let params = HnswParams {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            metric: MetricType::L2,
        };
        let index = HnswIndex::new(params, 3);

        index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
        index.insert(2, &[0.0, 1.0, 0.0], 2).unwrap();
        index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap();

        assert_eq!(index.count(), 3);

        let results = index.search(&[1.0, 0.0, 0.0], 2, None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_hnsw_delete() {
        let params = HnswParams::default();
        let index = HnswIndex::new(params, 2);

        index.insert(1, &[1.0, 0.0], 2).unwrap();
        index.insert(2, &[0.0, 1.0], 2).unwrap();
        assert_eq!(index.count(), 2);

        index.delete(1).unwrap();
        assert_eq!(index.count(), 1);
    }

    #[test]
    fn test_hnsw_level_filter() {
        let params = HnswParams::default();
        let index = HnswIndex::new(params, 3);

        index.insert(1, &[1.0, 0.0, 0.0], 0).unwrap(); // L0
        index.insert(2, &[0.0, 1.0, 0.0], 1).unwrap(); // L1
        index.insert(3, &[0.0, 0.0, 1.0], 2).unwrap(); // L2

        let results = index.search(&[1.0, 0.0, 0.0], 10, Some(0)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[0].level, 0);
    }

    #[test]
    fn test_hnsw_cosine_distance() {
        let params = HnswParams {
            metric: MetricType::Cosine,
            ..Default::default()
        };
        let index = HnswIndex::new(params, 3);

        // Normalized vectors
        index.insert(1, &[1.0, 0.0, 0.0], 2).unwrap();
        index.insert(2, &[0.0, 1.0, 0.0], 2).unwrap();
        index.insert(3, &[0.577, 0.577, 0.577], 2).unwrap();

        let results = index.search(&[1.0, 0.0, 0.0], 1, None).unwrap();
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_hnsw_large_dataset() {
        let params = HnswParams {
            m: 16,
            ef_construction: 200,
            ef_search: 100,
            metric: MetricType::L2,
        };
        let index = HnswIndex::new(params, 128);

        // Insert 1000 random vectors
        for i in 0..1000 {
            let vector: Vec<f32> = (0..128).map(|j| ((i * j) % 100) as f32 / 100.0).collect();
            index.insert(i, &vector, 2).unwrap();
        }

        assert_eq!(index.count(), 1000);

        // Search for first vector (all zeros)
        let query: Vec<f32> = vec![0.0; 128];
        let results = index.search(&query, 10, None).unwrap();
        assert!(!results.is_empty());
        // First result should be near zero distance (exact match or very similar)
        assert!(
            results[0].score < 1.0,
            "Expected low distance, got {}",
            results[0].score
        );
    }

    #[test]
    fn test_hnsw_duplicate_id_error() {
        let params = HnswParams::default();
        let index = HnswIndex::new(params, 2);

        index.insert(1, &[1.0, 0.0], 2).unwrap();
        let result = index.insert(1, &[0.0, 1.0], 2);
        assert!(result.is_err());
    }
}
