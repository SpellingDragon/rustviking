//! OpenAI Embedding Provider Integration Tests
//!
//! Tests for OpenAI-compatible Embedding Provider initialization and configuration.
//! Note: These tests use mock approaches and do NOT call actual external APIs.

use rustviking::embedding::openai::OpenAIEmbeddingProvider;
use rustviking::embedding::traits::EmbeddingProvider;
use rustviking::embedding::types::{EmbeddingConfig, EmbeddingRequest};

// ============================================================================
// Initialization and Configuration Tests
// ============================================================================

#[test]
fn test_openai_provider_default() {
    let provider = OpenAIEmbeddingProvider::default();
    assert_eq!(provider.name(), "openai");
    assert_eq!(provider.version(), "0.1.0");
}

#[test]
fn test_openai_provider_new() {
    let provider = OpenAIEmbeddingProvider::new();
    assert_eq!(provider.name(), "openai");
    // Default dimension before initialization
    assert_eq!(provider.default_dimension(), 1536);
}

#[test]
fn test_openai_provider_initialize_with_direct_api_key() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-direct-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
    assert_eq!(provider.default_dimension(), 1536);
}

#[test]
fn test_openai_provider_initialize_with_different_dimensions() {
    let dimensions = vec![256, 512, 768, 1024, 1536, 2048];

    for dim in dimensions {
        let provider = OpenAIEmbeddingProvider::new();
        let config = EmbeddingConfig {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: Some(format!("sk-test-key-{}", dim)),
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            dimension: dim,
            max_concurrent: 10,
        };

        assert!(provider.initialize(config).is_ok());
        assert_eq!(provider.default_dimension(), dim);
    }
}

#[test]
fn test_openai_provider_initialize_with_different_models() {
    let models = vec![
        "text-embedding-3-small",
        "text-embedding-3-large",
        "text-embedding-ada-002",
    ];

    for model in models {
        let provider = OpenAIEmbeddingProvider::new();
        let config = EmbeddingConfig {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: Some("sk-test-key".to_string()),
            provider: "openai".to_string(),
            model: model.to_string(),
            dimension: 1536,
            max_concurrent: 10,
        };

        assert!(provider.initialize(config).is_ok());
    }
}

#[test]
fn test_openai_provider_initialize_with_custom_api_base() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://custom-api.example.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
}

// ============================================================================
// API Key Environment Variable Tests
// ============================================================================

#[test]
fn test_resolve_api_key_direct() {
    // Test direct API key (not env: format)
    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-direct-key-123".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
}

#[test]
fn test_resolve_api_key_from_env_format() {
    // Set a test environment variable
    std::env::set_var("TEST_OPENAI_API_KEY", "sk-from-env-456");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("env:TEST_OPENAI_API_KEY".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());

    // Clean up
    std::env::remove_var("TEST_OPENAI_API_KEY");
}

#[test]
fn test_resolve_api_key_from_env_format_missing() {
    // Ensure the env var is not set
    std::env::remove_var("NONEXISTENT_API_KEY_VAR");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("env:NONEXISTENT_API_KEY_VAR".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    let result = provider.initialize(config);
    assert!(result.is_err());
}

#[test]
fn test_resolve_api_key_empty_with_zai_env() {
    // Save current env var if exists
    let original = std::env::var("ZAI_API_KEY").ok();

    // Set ZAI_API_KEY
    std::env::set_var("ZAI_API_KEY", "sk-zai-env-key");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("".to_string()), // Empty string should trigger env fallback
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());

    // Restore original env var
    match original {
        Some(val) => std::env::set_var("ZAI_API_KEY", val),
        None => std::env::remove_var("ZAI_API_KEY"),
    }
}

#[test]
fn test_resolve_api_key_empty_with_openai_env() {
    // Save current env vars
    let original_zai = std::env::var("ZAI_API_KEY").ok();
    let original_openai = std::env::var("OPENAI_API_KEY").ok();

    // Ensure ZAI_API_KEY is not set
    std::env::remove_var("ZAI_API_KEY");
    // Set OPENAI_API_KEY
    std::env::set_var("OPENAI_API_KEY", "sk-openai-env-key");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("".to_string()), // Empty string should trigger env fallback
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());

    if let Some(val) = original_zai {
        std::env::set_var("ZAI_API_KEY", val);
    }
    match original_openai {
        Some(val) => std::env::set_var("OPENAI_API_KEY", val),
        None => std::env::remove_var("OPENAI_API_KEY"),
    }
}

#[test]
fn test_resolve_api_key_empty_no_env() {
    // Save current env vars
    let original_zai = std::env::var("ZAI_API_KEY").ok();
    let original_openai = std::env::var("OPENAI_API_KEY").ok();

    // Ensure neither env var is set
    std::env::remove_var("ZAI_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("".to_string()), // Empty string with no env vars
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    let result = provider.initialize(config);
    assert!(result.is_err());

    // Restore original env vars
    if let Some(val) = original_zai {
        std::env::set_var("ZAI_API_KEY", val);
    }
    if let Some(val) = original_openai {
        std::env::set_var("OPENAI_API_KEY", val);
    }
}

#[test]
fn test_resolve_api_key_none_config() {
    // Save current env vars
    let original_zai = std::env::var("ZAI_API_KEY").ok();
    let original_openai = std::env::var("OPENAI_API_KEY").ok();

    // Ensure neither env var is set
    std::env::remove_var("ZAI_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");

    let provider = OpenAIEmbeddingProvider::new();
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: None, // None should trigger env fallback
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    let result = provider.initialize(config);
    assert!(result.is_err());

    // Restore original env vars
    if let Some(val) = original_zai {
        std::env::set_var("ZAI_API_KEY", val);
    }
    if let Some(val) = original_openai {
        std::env::set_var("OPENAI_API_KEY", val);
    }
}

// ============================================================================
// Supported Models Tests
// ============================================================================

#[test]
fn test_supported_models() {
    let provider = OpenAIEmbeddingProvider::new();
    let models = provider.supported_models();

    assert_eq!(models.len(), 3);
    assert!(models.contains(&"text-embedding-3-small"));
    assert!(models.contains(&"text-embedding-3-large"));
    assert!(models.contains(&"text-embedding-ada-002"));
}

#[test]
fn test_supported_models_after_init() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-large".to_string(),
        dimension: 3072,
        max_concurrent: 10,
    };

    provider.initialize(config).unwrap();

    // Supported models should remain the same after initialization
    let models = provider.supported_models();
    assert_eq!(models.len(), 3);
}

// ============================================================================
// Default Dimension Tests
// ============================================================================

#[test]
fn test_default_dimension_before_init() {
    let provider = OpenAIEmbeddingProvider::new();
    // Default dimension is 1536 before initialization
    assert_eq!(provider.default_dimension(), 1536);
}

#[test]
fn test_default_dimension_after_init() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    provider.initialize(config).unwrap();
    assert_eq!(provider.default_dimension(), 1536);
}

#[test]
fn test_default_dimension_different_values() {
    let test_cases = vec![
        (256, "text-embedding-3-small"),
        (512, "text-embedding-3-small"),
        (768, "text-embedding-3-small"),
        (1536, "text-embedding-3-small"),
        (3072, "text-embedding-3-large"),
    ];

    for (dim, model) in test_cases {
        let provider = OpenAIEmbeddingProvider::new();

        let config = EmbeddingConfig {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: Some(format!("sk-test-key-{}", dim)),
            provider: "openai".to_string(),
            model: model.to_string(),
            dimension: dim,
            max_concurrent: 10,
        };

        provider.initialize(config).unwrap();
        assert_eq!(provider.default_dimension(), dim);
    }
}

// ============================================================================
// Not Initialized Error Tests
// ============================================================================

#[test]
fn test_embed_without_initialize() {
    let provider = OpenAIEmbeddingProvider::new();

    let request = EmbeddingRequest {
        texts: vec!["Hello, world!".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("not initialized") || error_msg.contains("OpenAI"));
}

#[test]
fn test_embed_batch_without_initialize() {
    let provider = OpenAIEmbeddingProvider::new();

    let requests = vec![EmbeddingRequest {
        texts: vec!["Hello".to_string()],
        model: None,
        normalize: false,
    }];

    let results = provider.embed_batch(requests, 10);
    assert!(results.is_err());
}

// ============================================================================
// Azure OpenAI Configuration Tests
// ============================================================================

#[test]
fn test_azure_openai_configuration() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://my-resource.openai.azure.com/openai/deployments/my-deployment"
            .to_string(),
        api_key: Some("azure-api-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-ada-002".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
}

#[test]
fn test_custom_openai_compatible_api() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.mistral.ai/v1".to_string(),
        api_key: Some("mistral-api-key".to_string()),
        provider: "openai".to_string(),
        model: "mistral-embed".to_string(),
        dimension: 1024,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
}

// ============================================================================
// Concurrent Configuration Tests
// ============================================================================

#[test]
fn test_max_concurrent_configuration() {
    let concurrency_levels = vec![1, 5, 10, 20, 50, 100];

    for max_concurrent in concurrency_levels {
        let provider = OpenAIEmbeddingProvider::new();

        let config = EmbeddingConfig {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: Some(format!("sk-test-key-{}", max_concurrent)),
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            max_concurrent,
        };

        assert!(provider.initialize(config).is_ok());
    }
}

// ============================================================================
// Provider Info Tests
// ============================================================================

#[test]
fn test_provider_name() {
    let provider = OpenAIEmbeddingProvider::new();
    assert_eq!(provider.name(), "openai");
}

#[test]
fn test_provider_version() {
    let provider = OpenAIEmbeddingProvider::new();
    assert_eq!(provider.version(), "0.1.0");
}

#[test]
fn test_provider_info_after_init() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    provider.initialize(config).unwrap();

    assert_eq!(provider.name(), "openai");
    assert_eq!(provider.version(), "0.1.0");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_initialize_with_empty_api_base() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    // Empty api_base should still initialize (though would fail on actual request)
    assert!(provider.initialize(config).is_ok());
}

#[test]
fn test_initialize_with_special_chars_in_api_key() {
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-key-with-special-chars-!@#$%^&*()".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
}

#[test]
fn test_reinitialize_with_different_config() {
    let provider = OpenAIEmbeddingProvider::new();

    // First initialization
    let config1 = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-first-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    provider.initialize(config1).unwrap();
    assert_eq!(provider.default_dimension(), 1536);

    // Re-initialization with different dimension
    let config2 = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-second-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-large".to_string(),
        dimension: 3072,
        max_concurrent: 20,
    };

    provider.initialize(config2).unwrap();
    assert_eq!(provider.default_dimension(), 3072);
}
