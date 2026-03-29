//! Embedding Integration Tests
//!
//! Tests for EmbeddingProvider trait and MockEmbeddingProvider implementation.

use rustviking::embedding::mock::MockEmbeddingProvider;
use rustviking::embedding::*;

// ============================================================================
// Basic Embedding Tests
// ============================================================================

#[tokio::test]
async fn test_mock_embed_basic() {
    let provider = MockEmbeddingProvider::new(128);

    let request = EmbeddingRequest {
        texts: vec!["Hello, world!".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();

    assert_eq!(result.embeddings.len(), 1);
    assert_eq!(result.dimension, 128);
    assert_eq!(result.embeddings[0].len(), 128);
}

#[tokio::test]
async fn test_mock_embed_deterministic() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["The quick brown fox jumps over the lazy dog.".to_string()],
        model: None,
        normalize: false,
    };

    let result1 = provider.embed(request.clone()).await.unwrap();
    let result2 = provider.embed(request).await.unwrap();

    // Same text should produce identical embeddings
    assert_eq!(result1.embeddings[0], result2.embeddings[0]);
}

#[tokio::test]
async fn test_mock_embed_different_texts() {
    let provider = MockEmbeddingProvider::new(64);

    let request1 = EmbeddingRequest {
        texts: vec!["Hello".to_string()],
        model: None,
        normalize: false,
    };

    let request2 = EmbeddingRequest {
        texts: vec!["World".to_string()],
        model: None,
        normalize: false,
    };

    let result1 = provider.embed(request1).await.unwrap();
    let result2 = provider.embed(request2).await.unwrap();

    // Different texts should produce different embeddings
    assert_ne!(result1.embeddings[0], result2.embeddings[0]);
}

// ============================================================================
// Batch Embedding Tests
// ============================================================================

#[tokio::test]
async fn test_mock_embed_batch() {
    let provider = MockEmbeddingProvider::new(32);

    let requests = vec![
        EmbeddingRequest {
            texts: vec!["First text".to_string(), "Second text".to_string()],
            model: None,
            normalize: false,
        },
        EmbeddingRequest {
            texts: vec!["Third text".to_string()],
            model: None,
            normalize: false,
        },
    ];

    let results = provider.embed_batch(requests, 2).await.unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].embeddings.len(), 2);
    assert_eq!(results[1].embeddings.len(), 1);

    // All embeddings should have correct dimension
    for result in &results {
        assert_eq!(result.dimension, 32);
        for embedding in &result.embeddings {
            assert_eq!(embedding.len(), 32);
        }
    }
}

#[tokio::test]
async fn test_mock_embed_multiple_texts_in_single_request() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec![
            "Text one".to_string(),
            "Text two".to_string(),
            "Text three".to_string(),
        ],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();

    assert_eq!(result.embeddings.len(), 3);

    // Each embedding should be different
    assert_ne!(result.embeddings[0], result.embeddings[1]);
    assert_ne!(result.embeddings[1], result.embeddings[2]);
}

// ============================================================================
// Normalization Tests
// ============================================================================

#[tokio::test]
async fn test_mock_embed_normalize() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["Test text for normalization".to_string()],
        model: None,
        normalize: true,
    };

    let result = provider.embed(request).await.unwrap();
    let vector = &result.embeddings[0];

    // L2 norm should be approximately 1.0
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-6,
        "Normalized vector should have unit length, got norm = {}",
        norm
    );
}

#[tokio::test]
async fn test_mock_embed_no_normalize() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["Test text without normalization".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    let vector = &result.embeddings[0];

    // L2 norm should NOT be 1.0 (raw random values)
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    // The norm is likely not 1.0 for random values
    // We just verify it's a valid positive number
    assert!(norm > 0.0);
}

// ============================================================================
// Dimension Tests
// ============================================================================

#[test]
fn test_mock_default_dimension() {
    let provider = MockEmbeddingProvider::default();
    assert_eq!(provider.default_dimension(), 1024);
}

#[tokio::test]
async fn test_mock_custom_dimension() {
    let provider = MockEmbeddingProvider::new(512);
    assert_eq!(provider.default_dimension(), 512);

    let request = EmbeddingRequest {
        texts: vec!["Test".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    assert_eq!(result.dimension, 512);
    assert_eq!(result.embeddings[0].len(), 512);
}

#[tokio::test]
async fn test_mock_various_dimensions() {
    let dimensions = vec![64, 128, 256, 512, 768, 1024, 1536];

    for dim in dimensions {
        let provider = MockEmbeddingProvider::new(dim);
        assert_eq!(provider.default_dimension(), dim);

        let request = EmbeddingRequest {
            texts: vec!["Test dimension".to_string()],
            model: None,
            normalize: false,
        };

        let result = provider.embed(request).await.unwrap();
        assert_eq!(result.dimension, dim);
    }
}

// ============================================================================
// Model Info Tests
// ============================================================================

#[test]
fn test_mock_supported_models() {
    let provider = MockEmbeddingProvider::default();
    let models = provider.supported_models();

    assert_eq!(models.len(), 1);
    assert_eq!(models[0], "mock-embedding-v1");
}

#[test]
fn test_mock_provider_info() {
    let provider = MockEmbeddingProvider::default();

    assert_eq!(provider.name(), "mock");
    assert_eq!(provider.version(), "0.1.0");
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_mock_initialize() {
    let provider = MockEmbeddingProvider::new(512);

    let config = EmbeddingConfig {
        api_base: "http://localhost".to_string(),
        api_key: Some("test-key".to_string()),
        provider: "mock".to_string(),
        model: "mock-embedding-v1".to_string(),
        dimension: 768,
        max_concurrent: 10,
    };

    provider.initialize(config).await.unwrap();

    // Dimension should be updated from config
    assert_eq!(provider.default_dimension(), 768);
}

#[test]
fn test_embedding_config_default() {
    let config = EmbeddingConfig::default();

    assert_eq!(config.api_base, "");
    assert!(config.api_key.is_none());
    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "mock-embedding");
    assert_eq!(config.dimension, 1024);
    assert_eq!(config.max_concurrent, 10);
}

// ============================================================================
// Result Structure Tests
// ============================================================================

#[tokio::test]
async fn test_embedding_result_model() {
    let provider = MockEmbeddingProvider::new(64);

    // Without specifying model
    let request1 = EmbeddingRequest {
        texts: vec!["Test".to_string()],
        model: None,
        normalize: false,
    };
    let result1 = provider.embed(request1).await.unwrap();
    assert_eq!(result1.model, "mock-embedding-v1");

    // With custom model
    let request2 = EmbeddingRequest {
        texts: vec!["Test".to_string()],
        model: Some("custom-model".to_string()),
        normalize: false,
    };
    let result2 = provider.embed(request2).await.unwrap();
    assert_eq!(result2.model, "custom-model");
}

#[tokio::test]
async fn test_embedding_result_token_count() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["Test token count".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();

    // Mock provider doesn't track token count
    assert!(result.token_count.is_none());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_embed_empty_texts() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec![],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    assert_eq!(result.embeddings.len(), 0);
}

#[tokio::test]
async fn test_embed_empty_string() {
    let provider = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    assert_eq!(result.embeddings.len(), 1);

    // Empty string should still produce a valid embedding
    assert_eq!(result.embeddings[0].len(), 64);
}

#[tokio::test]
async fn test_embed_very_long_text() {
    let provider = MockEmbeddingProvider::new(128);

    let long_text = "a".repeat(10000);
    let request = EmbeddingRequest {
        texts: vec![long_text],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    assert_eq!(result.embeddings.len(), 1);
    assert_eq!(result.embeddings[0].len(), 128);
}

#[tokio::test]
async fn test_embed_special_characters() {
    let provider = MockEmbeddingProvider::new(64);

    let texts = vec![
        "Hello 世界! 🌍".to_string(),
        "Привет мир!".to_string(),
        "🎉🎊🎁".to_string(),
        "Tab\tNewline\nTest".to_string(),
    ];

    let request = EmbeddingRequest {
        texts,
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).await.unwrap();
    assert_eq!(result.embeddings.len(), 4);

    // Each embedding should have correct dimension
    for embedding in &result.embeddings {
        assert_eq!(embedding.len(), 64);
    }
}

// ============================================================================
// Embedding Consistency Tests
// ============================================================================

#[tokio::test]
async fn test_embedding_consistency_across_instances() {
    let provider1 = MockEmbeddingProvider::new(64);
    let provider2 = MockEmbeddingProvider::new(64);

    let request = EmbeddingRequest {
        texts: vec!["Consistency test".to_string()],
        model: None,
        normalize: false,
    };

    let result1 = provider1.embed(request.clone()).await.unwrap();
    let result2 = provider2.embed(request).await.unwrap();

    // Different provider instances should produce same result for same text
    assert_eq!(result1.embeddings[0], result2.embeddings[0]);
}

#[tokio::test]
async fn test_different_dimensions_different_results() {
    let provider_64 = MockEmbeddingProvider::new(64);
    let provider_128 = MockEmbeddingProvider::new(128);

    let request_64 = EmbeddingRequest {
        texts: vec!["Test text".to_string()],
        model: None,
        normalize: false,
    };

    let result_64 = provider_64.embed(request_64).await.unwrap();

    // Same text with different dimensions should produce different length vectors
    let request_128 = EmbeddingRequest {
        texts: vec!["Test text".to_string()],
        model: None,
        normalize: false,
    };
    let result_128 = provider_128.embed(request_128).await.unwrap();

    assert_eq!(result_64.embeddings[0].len(), 64);
    assert_eq!(result_128.embeddings[0].len(), 128);
}

// ============================================================================
// Batch Processing Tests
// ============================================================================

#[tokio::test]
async fn test_batch_empty_requests() {
    let provider = MockEmbeddingProvider::new(64);

    let results = provider.embed_batch(vec![], 10).await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_batch_large_number_of_requests() {
    let provider = MockEmbeddingProvider::new(32);

    let requests: Vec<EmbeddingRequest> = (0..50)
        .map(|i| EmbeddingRequest {
            texts: vec![format!("Text {}", i)],
            model: None,
            normalize: false,
        })
        .collect();

    let results = provider.embed_batch(requests, 10).await.unwrap();
    assert_eq!(results.len(), 50);
}
