//! HNSW Index Implementation
//!
//! Hierarchical Navigable Small World graph for approximate nearest neighbor search.

use crate::compute::DistanceComputer;
use crate::error::{Result, RustVikingError};
use crate::index::vector::{HnswParams, MetricType, SearchResult, VectorIndex};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::RwLock;

/// Node in the HNSW graph
struct HnswNode {
    id: u64,
    vector: Vec<f32>,
    level: u8,                // context level (L0/L1/L2)
    _max_layer: usize,        // max layer in HNSW graph
    neighbors: Vec<Vec<u64>>, // neighbors per layer
}

/// HNSW Index
pub struct HnswIndex {
    params: HnswParams,
    dimension: usize,
    nodes: RwLock<HashMap<u64, HnswNode>>,
    entry_point: RwLock<Option<u64>>,
    max_layer: RwLock<usize>,
    computer: DistanceComputer,
}

impl HnswIndex {
    pub fn new(params: HnswParams, dimension: usize) -> Self {
        Self {
            params,
            dimension,
            nodes: RwLock::new(HashMap::new()),
            entry_point: RwLock::new(None),
            max_layer: RwLock::new(0),
            computer: DistanceComputer::new(dimension),
        }
    }

    /// Compute distance based on metric
    fn compute_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.params.metric {
            MetricType::L2 => self.computer.l2_distance(a, b),
            MetricType::Cosine => self.computer.cosine_distance(a, b),
            MetricType::DotProduct => -self.computer.dot_product(a, b),
        }
    }

    /// Generate random layer for new node
    fn random_layer(&self) -> usize {
        let ml = 1.0 / (self.params.m as f64).ln();
        let r: f64 = rand_f64();
        (-r.ln() * ml).floor() as usize
    }
}

/// Simple pseudo-random f64 in [0, 1) using thread-local state
fn rand_f64() -> f64 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = Cell::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        );
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x as f64) / (u64::MAX as f64)
    })
}

impl VectorIndex for HnswIndex {
    fn insert(&self, id: u64, vector: &[f32], level: u8) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(RustVikingError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let new_layer = self.random_layer();
        let node = HnswNode {
            id,
            vector: vector.to_vec(),
            level,
            _max_layer: new_layer,
            neighbors: (0..=new_layer).map(|_| Vec::new()).collect(),
        };

        let mut nodes = self
            .nodes
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        let entry = self
            .entry_point
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?
            .clone();

        if entry.is_none() {
            // First node
            let id = node.id;
            nodes.insert(id, node);
            drop(nodes);

            let mut ep = self
                .entry_point
                .write()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            *ep = Some(id);
            let mut ml = self
                .max_layer
                .write()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            *ml = new_layer;
            return Ok(());
        }

        // Simple greedy insert: connect to nearest neighbors at each layer
        // Find nearest nodes using brute force within the graph
        let max_connections = self.params.m;
        let mut nearest: Vec<(u64, f32)> = nodes
            .iter()
            .filter(|(&nid, _)| nid != id)
            .map(|(&nid, n)| {
                let dist = self.compute_distance(vector, &n.vector);
                (nid, dist)
            })
            .collect();
        nearest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        nearest.truncate(max_connections);

        // Add bidirectional connections at layer 0
        let neighbor_ids: Vec<u64> = nearest.iter().map(|(nid, _)| *nid).collect();

        // Create and insert the new node first so we can reference it
        let mut new_node = node;
        if !new_node.neighbors.is_empty() {
            new_node.neighbors[0] = neighbor_ids.clone();
        }
        let new_vector = new_node.vector.clone();
        nodes.insert(id, new_node);

        // Pre-compute distances for trimming neighbors
        let mut neighbor_trim_data: Vec<(u64, Vec<(u64, f32)>)> = Vec::new();
        for &nid in &neighbor_ids {
            if let Some(neighbor_node) = nodes.get(&nid) {
                if !neighbor_node.neighbors.is_empty() {
                    if neighbor_node.neighbors[0].len() + 1 > max_connections * 2 {
                        // Pre-compute distances for trimming
                        let nv = &neighbor_node.vector;
                        let scored: Vec<(u64, f32)> = neighbor_node.neighbors[0]
                            .iter()
                            .filter_map(|&cid| {
                                nodes
                                    .get(&cid)
                                    .map(|cn| (cid, self.compute_distance(nv, &cn.vector)))
                            })
                            .collect();
                        neighbor_trim_data.push((nid, scored));
                    }
                }
            }
        }

        // Update neighbors' lists
        for &nid in &neighbor_ids {
            if let Some(neighbor_node) = nodes.get_mut(&nid) {
                if !neighbor_node.neighbors.is_empty() {
                    if !neighbor_node.neighbors[0].contains(&id) {
                        neighbor_node.neighbors[0].push(id);
                        // Trim to max connections
                        if neighbor_node.neighbors[0].len() > max_connections * 2 {
                            // Find pre-computed distances
                            if let Some((_, mut scored)) = neighbor_trim_data
                                .iter()
                                .find(|(n, _)| *n == nid)
                                .cloned()
                            {
                                // Add the new node
                                let nv = &neighbor_node.vector;
                                scored.push((id, self.compute_distance(nv, &new_vector)));
                                scored.sort_by(|a, b| {
                                    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)
                                });
                                scored.truncate(max_connections * 2);
                                neighbor_node.neighbors[0] =
                                    scored.into_iter().map(|(nid, _)| nid).collect();
                            }
                        }
                    }
                }
            }
        }

        // Update entry point if new node has higher layer
        drop(nodes);
        let current_max = *self
            .max_layer
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        if new_layer > current_max {
            let mut ep = self
                .entry_point
                .write()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            *ep = Some(id);
            let mut ml = self
                .max_layer
                .write()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            *ml = new_layer;
        }

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

        let nodes = self
            .nodes
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        if nodes.is_empty() {
            return Ok(Vec::new());
        }

        // Compute distances to all nodes (with level filter)
        let mut candidates: Vec<(u64, f32, u8)> = nodes
            .values()
            .filter(|n| level_filter.map_or(true, |lf| n.level == lf))
            .map(|n| {
                let dist = self.compute_distance(query, &n.vector);
                (n.id, dist, n.level)
            })
            .collect();

        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        Ok(candidates
            .into_iter()
            .take(k)
            .map(|(id, score, level)| SearchResult {
                id,
                score,
                vector: None,
                level,
            })
            .collect())
    }

    fn delete(&self, id: u64) -> Result<()> {
        let mut nodes = self
            .nodes
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        if nodes.remove(&id).is_none() {
            return Err(RustVikingError::NotFound(format!("vector id={}", id)));
        }

        // Remove from all neighbors' lists
        for node in nodes.values_mut() {
            for layer_neighbors in node.neighbors.iter_mut() {
                layer_neighbors.retain(|&nid| nid != id);
            }
        }

        // Update entry point if needed
        drop(nodes);
        let ep = self
            .entry_point
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?
            .clone();
        if ep == Some(id) {
            let nodes = self
                .nodes
                .read()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            let new_ep = nodes.keys().next().copied();
            drop(nodes);
            let mut ep_w = self
                .entry_point
                .write()
                .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
            *ep_w = new_ep;
        }

        Ok(())
    }

    fn get(&self, id: u64) -> Result<Option<Vec<f32>>> {
        let nodes = self
            .nodes
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        Ok(nodes.get(&id).map(|n| n.vector.clone()))
    }

    fn count(&self) -> u64 {
        self.nodes
            .read()
            .map(|n| n.len() as u64)
            .unwrap_or(0)
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
            m: 4,
            ef_construction: 16,
            ef_search: 10,
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
}
