//! Mock EmbeddingProvider implementation
//!
//! Mock implementation for testing purposes - see Task 5

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use crate::compute::normalize::l2_normalize;
use crate::error::Result;

use super::traits::EmbeddingProvider;
use super::types::{EmbeddingConfig, EmbeddingRequest, EmbeddingResult};

/// Mock Embedding Provider - 用于测试和 Benchmark
///
/// 生成确定性的假向量（基于文本 hash），相同文本始终生成相同向量。
/// 支持配置向量维度和归一化选项。
pub struct MockEmbeddingProvider {
    dimension: Mutex<usize>,
}

/// 简单的伪随机数生成器（LCG）
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// 创建新的 RNG 实例
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// 生成下一个 u64 随机数
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    /// 生成下一个 f32 随机数（范围 0.0 到 1.0）
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

impl MockEmbeddingProvider {
    /// 创建新的 MockEmbeddingProvider 实例
    ///
    /// # Arguments
    /// * `dimension` - 生成向量的维度
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension: Mutex::new(dimension),
        }
    }

    /// 为单个文本生成确定性向量
    fn generate_embedding(&self, text: &str, dimension: usize) -> Vec<f32> {
        // 使用 DefaultHasher 计算文本哈希作为种子
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        // 使用 LCG 生成伪随机向量
        let mut rng = SimpleRng::new(seed);
        (0..dimension).map(|_| rng.next_f32()).collect()
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl EmbeddingProvider for MockEmbeddingProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn initialize(&self, config: EmbeddingConfig) -> Result<()> {
        // 存储配置，更新 dimension
        let mut dimension = self.dimension.lock().unwrap();
        *dimension = config.dimension;
        Ok(())
    }

    fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResult> {
        let dimension = *self.dimension.lock().unwrap();

        // 为每个文本生成 embedding
        let mut embeddings: Vec<Vec<f32>> = Vec::new();

        for text in &request.texts {
            let mut vector = self.generate_embedding(text, dimension);

            // 如果需要归一化，使用 l2_normalize
            if request.normalize {
                vector = l2_normalize(&vector);
            }

            embeddings.push(vector);
        }

        Ok(EmbeddingResult {
            embeddings,
            model: request
                .model
                .unwrap_or_else(|| "mock-embedding-v1".to_string()),
            dimension,
            token_count: None,
        })
    }

    fn embed_batch(
        &self,
        requests: Vec<EmbeddingRequest>,
        _max_concurrent: usize,
    ) -> Result<Vec<EmbeddingResult>> {
        // 循环调用 embed 处理每个请求
        requests.into_iter().map(|req| self.embed(req)).collect()
    }

    fn default_dimension(&self) -> usize {
        *self.dimension.lock().unwrap()
    }

    fn supported_models(&self) -> Vec<&str> {
        vec!["mock-embedding-v1"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider_default() {
        let provider = MockEmbeddingProvider::default();
        assert_eq!(provider.name(), "mock");
        assert_eq!(provider.version(), "0.1.0");
        assert_eq!(provider.default_dimension(), 1024);
        assert_eq!(provider.supported_models(), vec!["mock-embedding-v1"]);
    }

    #[test]
    fn test_mock_provider_new() {
        let provider = MockEmbeddingProvider::new(512);
        assert_eq!(provider.default_dimension(), 512);
    }

    #[test]
    fn test_mock_provider_initialize() {
        let provider = MockEmbeddingProvider::new(512);
        let config = EmbeddingConfig {
            dimension: 768,
            ..Default::default()
        };
        assert!(provider.initialize(config).is_ok());
        assert_eq!(provider.default_dimension(), 768);
    }

    #[test]
    fn test_mock_provider_embed_deterministic() {
        let provider = MockEmbeddingProvider::new(128);
        let request = EmbeddingRequest {
            texts: vec!["hello world".to_string()],
            model: None,
            normalize: false,
        };

        let result1 = provider.embed(request.clone()).unwrap();
        let result2 = provider.embed(request).unwrap();

        // 相同文本应生成相同向量
        assert_eq!(result1.embeddings[0], result2.embeddings[0]);
        assert_eq!(result1.dimension, 128);
    }

    #[test]
    fn test_mock_provider_embed_normalized() {
        let provider = MockEmbeddingProvider::new(64);
        let request = EmbeddingRequest {
            texts: vec!["test text".to_string()],
            model: None,
            normalize: true,
        };

        let result = provider.embed(request).unwrap();
        let vector = &result.embeddings[0];

        // 检查归一化后向量的 L2 范数是否为 1
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-6,
            "Normalized vector should have unit length"
        );
    }

    #[test]
    fn test_mock_provider_embed_batch() {
        let provider = MockEmbeddingProvider::new(32);
        let requests = vec![
            EmbeddingRequest {
                texts: vec!["text1".to_string(), "text2".to_string()],
                model: None,
                normalize: false,
            },
            EmbeddingRequest {
                texts: vec!["text3".to_string()],
                model: None,
                normalize: false,
            },
        ];

        let results = provider.embed_batch(requests, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].embeddings.len(), 2);
        assert_eq!(results[1].embeddings.len(), 1);
    }

    #[test]
    fn test_simple_rng() {
        let mut rng1 = SimpleRng::new(12345);
        let mut rng2 = SimpleRng::new(12345);

        // 相同种子应生成相同序列
        assert_eq!(rng1.next_u64(), rng2.next_u64());
        assert_eq!(rng1.next_f32(), rng2.next_f32());

        // f32 值应在 0.0 到 1.0 之间
        for _ in 0..100 {
            let val = rng1.next_f32();
            assert!((0.0..=1.0).contains(&val));
        }
    }
}
