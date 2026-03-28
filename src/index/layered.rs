//! Layered Index Manager
//!
//! Manages L0 (abstract), L1 (overview), and L2 (detail) index layers.

use crate::error::Result;
use crate::index::vector::{SearchResult, VectorIndex};
use std::sync::Arc;

/// Context level constants
pub const LEVEL_L0: u8 = 0; // Abstract (~100 tokens)
pub const LEVEL_L1: u8 = 1; // Overview (~2k tokens)
pub const LEVEL_L2: u8 = 2; // Full detail

/// Layered index that routes to appropriate level
pub struct LayeredIndex {
    /// Underlying vector index (shared across levels)
    inner: Arc<dyn VectorIndex>,
}

impl LayeredIndex {
    pub fn new(index: Arc<dyn VectorIndex>) -> Self {
        Self { inner: index }
    }

    /// Search at a specific level
    pub fn search_level(&self, query: &[f32], k: usize, level: u8) -> Result<Vec<SearchResult>> {
        self.inner.search(query, k, Some(level))
    }

    /// Search L0 (abstract/summary) layer
    pub fn search_abstract(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        self.search_level(query, k, LEVEL_L0)
    }

    /// Search L1 (overview) layer
    pub fn search_overview(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        self.search_level(query, k, LEVEL_L1)
    }

    /// Search L2 (detail) layer
    pub fn search_detail(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        self.search_level(query, k, LEVEL_L2)
    }

    /// Hierarchical search: L0 -> L1 -> L2 progressive refinement
    pub fn hierarchical_search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        // 1. Fast screening at L0
        let l0_results = self.search_abstract(query, k * 3)?;

        if l0_results.is_empty() {
            // Fallback to L2 direct search
            return self.search_detail(query, k);
        }

        // 2. Get L1 overview for top candidates
        let l1_results = self.search_overview(query, k * 2)?;

        // 3. Final detail search at L2
        let l2_results = self.search_detail(query, k)?;

        // Merge results, prioritizing L2 for detail
        let mut all_results = l2_results;

        // Add L1 results not already in L2
        for r in l1_results {
            if !all_results.iter().any(|existing| existing.id == r.id) {
                all_results.push(r);
            }
        }

        // Sort by score
        all_results.sort_by(|a, b| {
            a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(k);

        Ok(all_results)
    }

    /// Insert with automatic level assignment
    pub fn insert(&self, id: u64, vector: &[f32], level: u8) -> Result<()> {
        self.inner.insert(id, vector, level)
    }

    /// Delete from all levels
    pub fn delete(&self, id: u64) -> Result<()> {
        self.inner.delete(id)
    }

    /// Get total count
    pub fn count(&self) -> u64 {
        self.inner.count()
    }
}
