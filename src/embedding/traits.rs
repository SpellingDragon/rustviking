//! Embedding Provider Trait
//!
//! Interface for embedding providers.

use crate::error::Result;

use super::types::{EmbeddingConfig, EmbeddingRequest, EmbeddingResult};

/// Embedding Provider Trait
pub trait EmbeddingProvider: Send + Sync {
    /// Provider 名称
    fn name(&self) -> &str;

    /// Provider 版本
    fn version(&self) -> &str;

    /// 初始化
    fn initialize(&self, config: EmbeddingConfig) -> Result<()>;

    /// 生成 Embedding
    fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResult>;

    /// 批量生成 Embedding（支持并发控制）
    fn embed_batch(
        &self,
        requests: Vec<EmbeddingRequest>,
        max_concurrent: usize,
    ) -> Result<Vec<EmbeddingResult>>;

    /// 获取默认维度
    fn default_dimension(&self) -> usize;

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str>;
}
