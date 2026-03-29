//! Embedding Module
//!
//! Embedding provider abstraction and implementations.

pub mod mock;
pub mod openai;
pub mod traits;
pub mod types;

pub use traits::EmbeddingProvider;
pub use types::*;
