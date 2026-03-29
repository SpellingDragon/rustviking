//! OpenAI-compatible Embedding Provider
//!
//! Supports any OpenAI-compatible API (OpenAI, Azure, ZhiPu/智谱, etc.)
//! API Key can be provided via config or environment variable ZAI_API_KEY.

use crate::compute::normalize::l2_normalize;
use crate::embedding::traits::EmbeddingProvider;
use crate::embedding::types::*;
use crate::error::{Result, RustVikingError};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// OpenAI-compatible Embedding Provider
pub struct OpenAIEmbeddingProvider {
    config: Mutex<Option<OpenAIProviderConfig>>,
}

/// Internal config after initialization
struct OpenAIProviderConfig {
    api_base: String,
    api_key: String,
    model: String,
    dimension: usize,
    max_concurrent: usize,
    client: reqwest::blocking::Client,
}

/// OpenAI API request body
#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

/// OpenAI API response
#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    #[allow(dead_code)]
    index: usize,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    total_tokens: Option<usize>,
}

impl OpenAIEmbeddingProvider {
    /// Create a new OpenAI embedding provider
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
        }
    }

    /// Resolve API key from config or environment variables
    ///
    /// Supports the following formats:
    /// - "env:VAR_NAME" - reads from environment variable VAR_NAME
    /// - plain string - uses the string directly as the API key
    /// - empty string - tries ZAI_API_KEY, then OPENAI_API_KEY environment variables
    fn resolve_api_key(api_key_config: &str) -> Result<String> {
        if let Some(env_var) = api_key_config.strip_prefix("env:") {
            // Extract environment variable name
            std::env::var(env_var).map_err(|_| {
                RustVikingError::Internal(format!(
                    "Environment variable '{}' not found for API key",
                    env_var
                ))
            })
        } else if api_key_config.is_empty() {
            // Try environment variables in order: ZAI_API_KEY, OPENAI_API_KEY
            std::env::var("ZAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .map_err(|_| {
                    RustVikingError::Internal(
                        "API key not found. Set ZAI_API_KEY or OPENAI_API_KEY environment variable, \
                         or provide api_key in config".to_string(),
                    )
                })
        } else {
            Ok(api_key_config.to_string())
        }
    }
}

impl Default for OpenAIEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider for OpenAIEmbeddingProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn initialize(&self, config: EmbeddingConfig) -> Result<()> {
        // Resolve API key
        let api_key = Self::resolve_api_key(&config.api_key.unwrap_or_default())?;

        // Create HTTP client
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| {
                RustVikingError::Internal(format!("Failed to create HTTP client: {}", e))
            })?;

        let provider_config = OpenAIProviderConfig {
            api_base: config.api_base,
            api_key,
            model: config.model,
            dimension: config.dimension,
            max_concurrent: config.max_concurrent,
            client,
        };

        let mut guard = self.config.lock().unwrap();
        *guard = Some(provider_config);

        Ok(())
    }

    fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResult> {
        let config = self
            .config
            .lock()
            .unwrap()
            .as_ref()
            .ok_or_else(|| {
                RustVikingError::Internal("OpenAI provider not initialized".to_string())
            })?
            .clone();

        // Build request URL
        let url = format!("{}/embeddings", config.api_base.trim_end_matches('/'));

        // Build request body
        let model = request.model.unwrap_or_else(|| config.model.clone());
        let body = OpenAIEmbeddingRequest {
            input: request.texts.clone(),
            model,
        };

        // Send request
        let response = config
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| {
                RustVikingError::Internal(format!("Embedding API request failed: {}", e))
            })?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RustVikingError::Internal(format!(
                "Embedding API request failed with status {}: {}",
                status, error_text
            )));
        }

        // Parse response
        let embedding_response: OpenAIEmbeddingResponse = response.json().map_err(|e| {
            RustVikingError::Internal(format!("Failed to parse embedding response: {}", e))
        })?;

        // Extract embeddings
        let mut embeddings: Vec<Vec<f32>> = embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        // Apply L2 normalization if requested
        if request.normalize {
            for embedding in &mut embeddings {
                *embedding = l2_normalize(embedding);
            }
        }

        let token_count = embedding_response.usage.and_then(|u| u.total_tokens);

        Ok(EmbeddingResult {
            embeddings,
            model: embedding_response.model,
            dimension: config.dimension,
            token_count,
        })
    }

    fn embed_batch(
        &self,
        requests: Vec<EmbeddingRequest>,
        _max_concurrent: usize,
    ) -> Result<Vec<EmbeddingResult>> {
        // Sequential processing for now (sync implementation)
        requests.into_iter().map(|req| self.embed(req)).collect()
    }

    fn default_dimension(&self) -> usize {
        self.config
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.dimension)
            .unwrap_or(1536)
    }

    fn supported_models(&self) -> Vec<&str> {
        vec![
            "text-embedding-3-small",
            "text-embedding-3-large",
            "text-embedding-ada-002",
        ]
    }
}

// Clone implementation for OpenAIProviderConfig
impl Clone for OpenAIProviderConfig {
    fn clone(&self) -> Self {
        Self {
            api_base: self.api_base.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            dimension: self.dimension,
            max_concurrent: self.max_concurrent,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to clone HTTP client"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_default() {
        let provider = OpenAIEmbeddingProvider::default();
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.version(), "0.1.0");
        assert_eq!(
            provider.supported_models(),
            vec![
                "text-embedding-3-small",
                "text-embedding-3-large",
                "text-embedding-ada-002",
            ]
        );
    }

    #[test]
    fn test_openai_provider_new() {
        let provider = OpenAIEmbeddingProvider::new();
        assert_eq!(provider.name(), "openai");
        // Default dimension before initialization
        assert_eq!(provider.default_dimension(), 1536);
    }

    #[test]
    fn test_resolve_api_key_direct() {
        let key = "sk-test123";
        let result = OpenAIEmbeddingProvider::resolve_api_key(key).unwrap();
        assert_eq!(result, "sk-test123");
    }

    #[test]
    fn test_resolve_api_key_from_env_format() {
        // Set a test environment variable
        std::env::set_var("TEST_API_KEY_VAR", "sk-from-env");
        let result = OpenAIEmbeddingProvider::resolve_api_key("env:TEST_API_KEY_VAR").unwrap();
        assert_eq!(result, "sk-from-env");
        std::env::remove_var("TEST_API_KEY_VAR");
    }

    #[test]
    fn test_resolve_api_key_missing_env_var() {
        let result = OpenAIEmbeddingProvider::resolve_api_key("env:NON_EXISTENT_VAR_XYZ");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_api_key_empty_with_env_fallback() {
        // Set ZAI_API_KEY
        std::env::set_var("ZAI_API_KEY", "sk-zai-key");
        let result = OpenAIEmbeddingProvider::resolve_api_key("").unwrap();
        assert_eq!(result, "sk-zai-key");
        std::env::remove_var("ZAI_API_KEY");
    }

    #[test]
    fn test_resolve_api_key_empty_with_openai_fallback() {
        // Save current env vars
        let zai_key = std::env::var("ZAI_API_KEY").ok();
        let openai_key = std::env::var("OPENAI_API_KEY").ok();

        // Set OPENAI_API_KEY when ZAI_API_KEY is not set
        std::env::set_var("OPENAI_API_KEY", "sk-openai-key");
        // Ensure ZAI_API_KEY is not set
        std::env::remove_var("ZAI_API_KEY");

        let result = OpenAIEmbeddingProvider::resolve_api_key("").unwrap();
        assert_eq!(result, "sk-openai-key");

        // Restore env vars
        if let Some(val) = zai_key {
            std::env::set_var("ZAI_API_KEY", val);
        } else {
            std::env::remove_var("ZAI_API_KEY");
        }
        if let Some(val) = openai_key {
            std::env::set_var("OPENAI_API_KEY", val);
        } else {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_resolve_api_key_empty_no_env() {
        // Save current env vars
        let zai_key = std::env::var("ZAI_API_KEY").ok();
        let openai_key = std::env::var("OPENAI_API_KEY").ok();

        // Ensure neither env var is set
        std::env::remove_var("ZAI_API_KEY");
        std::env::remove_var("OPENAI_API_KEY");

        let result = OpenAIEmbeddingProvider::resolve_api_key("");
        assert!(result.is_err());

        // Restore env vars
        if let Some(val) = zai_key {
            std::env::set_var("ZAI_API_KEY", val);
        }
        if let Some(val) = openai_key {
            std::env::set_var("OPENAI_API_KEY", val);
        }
    }

    #[test]
    fn test_openai_provider_initialize() {
        let provider = OpenAIEmbeddingProvider::new();
        let config = EmbeddingConfig {
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: Some("sk-test".to_string()),
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            dimension: 1536,
            max_concurrent: 10,
        };
        assert!(provider.initialize(config).is_ok());
        assert_eq!(provider.default_dimension(), 1536);
    }

    #[test]
    fn test_openai_provider_not_initialized() {
        let provider = OpenAIEmbeddingProvider::new();
        let request = EmbeddingRequest {
            texts: vec!["hello".to_string()],
            model: None,
            normalize: false,
        };
        let result = provider.embed(request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
    }
}
