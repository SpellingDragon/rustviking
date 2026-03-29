//! Vector index trait and core types

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Search result entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: u64,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
    pub level: u8, // L0/L1/L2
}

/// Distance metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    L2,
    Cosine,
    DotProduct,
}

/// Vector index trait
pub trait VectorIndex: Send + Sync {
    /// Insert a single vector
    fn insert(&self, id: u64, vector: &[f32], level: u8) -> Result<()>;

    /// Insert a batch of vectors
    fn insert_batch(&self, vectors: &[(u64, Vec<f32>, u8)]) -> Result<()>;

    /// Search for nearest neighbors
    fn search(
        &self,
        query: &[f32],
        k: usize,
        level_filter: Option<u8>,
    ) -> Result<Vec<SearchResult>>;

    /// Delete a vector by ID
    fn delete(&self, id: u64) -> Result<()>;

    /// Get a vector by ID
    fn get(&self, id: u64) -> Result<Option<Vec<f32>>>;

    /// Get total count of indexed vectors
    fn count(&self) -> u64;

    /// Get the dimension of indexed vectors
    fn dimension(&self) -> usize;
}

/// IVF index parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfParams {
    /// Number of partitions (clusters)
    pub num_partitions: usize,
    /// Distance metric
    pub metric: MetricType,
}

impl Default for IvfParams {
    fn default() -> Self {
        Self {
            num_partitions: 256,
            metric: MetricType::L2,
        }
    }
}

/// HNSW index parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswParams {
    /// Max connections per node (M parameter)
    pub m: usize,
    /// Size of dynamic candidate list during construction
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search
    pub ef_search: usize,
    /// Distance metric
    pub metric: MetricType,
}

impl Default for HnswParams {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            metric: MetricType::L2,
        }
    }
}
