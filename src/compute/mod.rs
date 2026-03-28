//! Compute layer - distance calculations and SIMD optimization

pub mod distance;
pub mod simd;
pub mod normalize;

pub use distance::DistanceComputer;
