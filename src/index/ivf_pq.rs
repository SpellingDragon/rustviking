//! IVF-PQ Index Implementation
//!
//! Inverted File with Product Quantization for large-scale vector search.

use crate::compute::DistanceComputer;
use crate::error::{Result, RustVikingError};
use crate::index::vector::{IvfPqParams, MetricType, SearchResult, VectorIndex};
use std::sync::RwLock;

/// Per-partition data
struct PartitionData {
    ids: Vec<u64>,
    vectors: Vec<Vec<f32>>,
    levels: Vec<u8>,
}

/// IVF-PQ Index
pub struct IvfPqIndex {
    params: IvfPqParams,
    dimension: usize,
    centroids: RwLock<Vec<Vec<f32>>>,
    partitions: RwLock<Vec<PartitionData>>,
    computer: DistanceComputer,
    trained: RwLock<bool>,
}

impl IvfPqIndex {
    pub fn new(params: IvfPqParams, dimension: usize) -> Self {
        let num_partitions = params.num_partitions;
        let partitions = (0..num_partitions)
            .map(|_| PartitionData {
                ids: Vec::new(),
                vectors: Vec::new(),
                levels: Vec::new(),
            })
            .collect();

        Self {
            params,
            dimension,
            centroids: RwLock::new(vec![vec![0.0; dimension]; num_partitions]),
            partitions: RwLock::new(partitions),
            computer: DistanceComputer::new(dimension),
            trained: RwLock::new(false),
        }
    }

    /// Compute distance based on metric type
    fn compute_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.params.metric {
            MetricType::L2 => self.computer.l2_distance(a, b),
            MetricType::Cosine => self.computer.cosine_distance(a, b),
            MetricType::DotProduct => -self.computer.dot_product(a, b), // negate for min-heap
        }
    }

    /// Train centroids using K-Means clustering
    pub fn train(&self, vectors: &[Vec<f32>]) -> Result<()> {
        if vectors.is_empty() {
            return Err(RustVikingError::Internal(
                "Cannot train with empty vectors".into(),
            ));
        }

        let k = self.params.num_partitions;
        let max_iters = 20;

        // Initialize centroids by sampling from input
        let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
        for i in 0..k {
            centroids.push(vectors[i % vectors.len()].clone());
        }

        // K-Means iterations
        for _iter in 0..max_iters {
            let mut counts = vec![0usize; k];
            let mut new_centroids = vec![vec![0.0f32; self.dimension]; k];

            // Assign each vector to nearest centroid
            for v in vectors {
                let mut min_dist = f32::MAX;
                let mut best = 0;
                for (i, c) in centroids.iter().enumerate() {
                    let dist = self.computer.l2_distance(v, c);
                    if dist < min_dist {
                        min_dist = dist;
                        best = i;
                    }
                }
                counts[best] += 1;
                for (j, val) in v.iter().enumerate() {
                    new_centroids[best][j] += val;
                }
            }

            // Update centroids
            for i in 0..k {
                if counts[i] > 0 {
                    for val in new_centroids[i].iter_mut().take(self.dimension) {
                        *val /= counts[i] as f32;
                    }
                } else {
                    // Keep old centroid if no vectors assigned
                    new_centroids[i] = centroids[i].clone();
                }
            }

            centroids = new_centroids;
        }

        // Update centroids
        let mut c = self
            .centroids
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        *c = centroids;

        let mut trained = self
            .trained
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        *trained = true;

        Ok(())
    }

    /// Find the nearest partition for a vector
    fn find_nearest_partition(&self, vector: &[f32], centroids: &[Vec<f32>]) -> usize {
        let mut min_dist = f32::MAX;
        let mut best = 0;
        for (i, c) in centroids.iter().enumerate() {
            let dist = self.computer.l2_distance(vector, c);
            if dist < min_dist {
                min_dist = dist;
                best = i;
            }
        }
        best
    }
}

impl VectorIndex for IvfPqIndex {
    fn insert(&self, id: u64, vector: &[f32], level: u8) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(RustVikingError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let centroids = self
            .centroids
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        let best_partition = self.find_nearest_partition(vector, &centroids);
        drop(centroids);

        let mut partitions = self
            .partitions
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        partitions[best_partition].ids.push(id);
        partitions[best_partition].vectors.push(vector.to_vec());
        partitions[best_partition].levels.push(level);

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

        let centroids = self
            .centroids
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        // Find nearest nprobe partitions
        let nprobe = std::cmp::max(1, (self.params.num_partitions as f32 * 0.1) as usize);
        let mut partition_dists: Vec<(usize, f32)> = centroids
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.computer.l2_distance(query, c)))
            .collect();
        partition_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let selected: Vec<usize> = partition_dists
            .into_iter()
            .take(nprobe)
            .map(|(i, _)| i)
            .collect();
        drop(centroids);

        // Search within selected partitions
        let partitions = self
            .partitions
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        let mut candidates: Vec<(u64, f32, u8)> = Vec::new();
        for &pid in &selected {
            let part = &partitions[pid];
            for (i, v) in part.vectors.iter().enumerate() {
                let level = part.levels[i];
                // Apply level filter
                if let Some(lf) = level_filter {
                    if level != lf {
                        continue;
                    }
                }
                let dist = self.compute_distance(query, v);
                candidates.push((part.ids[i], dist, level));
            }
        }

        // Sort by distance and take top-k
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

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
        let mut partitions = self
            .partitions
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        for part in partitions.iter_mut() {
            if let Some(pos) = part.ids.iter().position(|&x| x == id) {
                part.ids.swap_remove(pos);
                part.vectors.swap_remove(pos);
                part.levels.swap_remove(pos);
                return Ok(());
            }
        }
        Err(RustVikingError::NotFound(format!("vector id={}", id)))
    }

    fn get(&self, id: u64) -> Result<Option<Vec<f32>>> {
        let partitions = self
            .partitions
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        for part in partitions.iter() {
            if let Some(pos) = part.ids.iter().position(|&x| x == id) {
                return Ok(Some(part.vectors[pos].clone()));
            }
        }
        Ok(None)
    }

    fn count(&self) -> u64 {
        self.partitions
            .read()
            .map(|p| p.iter().map(|part| part.ids.len()).sum::<usize>() as u64)
            .unwrap_or(0)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_index() -> IvfPqIndex {
        let params = IvfPqParams {
            num_partitions: 4,
            num_sub_vectors: 2,
            pq_bits: 8,
            metric: MetricType::L2,
        };
        IvfPqIndex::new(params, 4)
    }

    #[test]
    fn test_insert_and_count() {
        let index = create_test_index();
        index.insert(1, &[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        index.insert(2, &[5.0, 6.0, 7.0, 8.0], 1).unwrap();
        assert_eq!(index.count(), 2);
    }

    #[test]
    fn test_insert_wrong_dimension() {
        let index = create_test_index();
        assert!(index.insert(1, &[1.0, 2.0], 2).is_err());
    }

    #[test]
    fn test_search() {
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

        for (i, v) in vectors.iter().enumerate() {
            index.insert(i as u64, v, 2).unwrap();
        }

        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 3, None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, 0); // exact match should be first
    }

    #[test]
    fn test_delete() {
        let index = create_test_index();
        index.insert(1, &[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        assert_eq!(index.count(), 1);
        index.delete(1).unwrap();
        assert_eq!(index.count(), 0);
    }

    #[test]
    fn test_get() {
        let index = create_test_index();
        let v = vec![1.0, 2.0, 3.0, 4.0];
        index.insert(42, &v, 2).unwrap();
        let result = index.get(42).unwrap();
        assert_eq!(result, Some(v));
    }

    #[test]
    fn test_level_filter() {
        let index = create_test_index();
        index.insert(1, &[1.0, 0.0, 0.0, 0.0], 0).unwrap(); // L0
        index.insert(2, &[0.0, 1.0, 0.0, 0.0], 1).unwrap(); // L1
        index.insert(3, &[0.0, 0.0, 1.0, 0.0], 2).unwrap(); // L2

        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 10, Some(0)).unwrap();
        assert!(results.iter().all(|r| r.level == 0));
    }
}
