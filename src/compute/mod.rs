//! Compute layer - distance calculations and SIMD optimization

pub mod distance;
pub mod normalize;
pub mod simd;

pub use distance::DistanceComputer;
