//! Vector index layer
//!
//! Provides vector indexing with IVF-PQ and HNSW algorithms,
//! layered index management (L0/L1/L2), and bitmap operations.

pub mod bitmap;
pub mod hnsw;
pub mod ivf_pq;
pub mod layered;
pub mod vector;

pub use bitmap::Bitmap;
pub use hnsw::HnswIndex;
pub use ivf_pq::IvfPqIndex;
pub use layered::{LayeredIndex, LEVEL_L0, LEVEL_L1, LEVEL_L2};
pub use vector::{HnswParams, IvfPqParams, MetricType, SearchResult, VectorIndex};
