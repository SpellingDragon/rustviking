//! Embedding Types
//!
//! Data types for embedding operations.

use serde::{Deserialize, Serialize};

/// Embedding 请求
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub texts: Vec<String>,
    pub model: Option<String>,
    pub normalize: bool,
}

/// Embedding 结果
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub dimension: usize,
    pub token_count: Option<usize>,
}

/// Embedding 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub api_base: String,
    pub api_key: Option<String>,
    pub provider: String,
    pub model: String,
    pub dimension: usize,
    pub max_concurrent: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_base: String::new(),
            api_key: None,
            provider: "mock".to_string(),
            model: "mock-embedding".to_string(),
            dimension: 1024,
            max_concurrent: 10,
        }
    }
}
