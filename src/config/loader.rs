//! Configuration loader

use crate::error::{Result, RustVikingError};
use crate::storage::config::StorageConfig;
use serde::{Deserialize, Serialize};

/// Vector index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    #[serde(default = "default_dimension")]
    pub dimension: usize,
    #[serde(default = "default_index_type")]
    pub index_type: String,
    #[serde(default)]
    pub ivf_pq: Option<IvfPqConfig>,
}

fn default_dimension() -> usize {
    768
}
fn default_index_type() -> String {
    "ivf_pq".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfPqConfig {
    #[serde(default = "default_partitions")]
    pub num_partitions: usize,
    #[serde(default = "default_sub_vectors")]
    pub num_sub_vectors: usize,
    #[serde(default = "default_pq_bits")]
    pub pq_bits: usize,
    #[serde(default = "default_metric")]
    pub metric: String,
}

fn default_partitions() -> usize {
    256
}
fn default_sub_vectors() -> usize {
    16
}
fn default_pq_bits() -> usize {
    8
}
fn default_metric() -> String {
    "l2".to_string()
}

// ============================================================================
// VectorStore Plugin Configuration
// ============================================================================

fn default_vector_store_plugin() -> String {
    "memory".to_string()
}
fn default_embedding_plugin() -> String {
    "mock".to_string()
}
fn default_qdrant_timeout() -> u64 {
    5000
}
fn default_mock_dimension() -> usize {
    1024
}
fn default_openai_model() -> String {
    "text-embedding-3-small".to_string()
}
fn default_openai_dimension() -> usize {
    1536
}
fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_ollama_model() -> String {
    "nomic-embed-text".to_string()
}
fn default_ollama_dimension() -> usize {
    768
}
fn default_max_concurrent() -> usize {
    10
}

fn default_summary_provider() -> String {
    "noop".to_string()
}

/// Summary provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryConfig {
    /// Provider type: "noop", "heuristic", "openai"
    #[serde(default = "default_summary_provider")]
    pub provider: String,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            provider: default_summary_provider(),
        }
    }
}

/// 向量存储插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// 当前使用的插件名称: memory, rocksdb, qdrant, http
    #[serde(default = "default_vector_store_plugin")]
    pub plugin: String,
    /// Memory 插件配置（无需额外参数）
    #[serde(default)]
    pub memory: Option<MemoryVectorStoreConfig>,
    /// RocksDB 插件配置
    #[serde(default)]
    pub rocksdb: Option<RocksDBVectorStoreConfig>,
    /// Qdrant 插件配置
    #[serde(default)]
    pub qdrant: Option<QdrantConfig>,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            plugin: default_vector_store_plugin(),
            memory: None,
            rocksdb: None,
            qdrant: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVectorStoreConfig {
    // 预留，当前无需配置
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDBVectorStoreConfig {
    /// RocksDB 数据存储路径
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
    #[serde(default = "default_qdrant_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Embedding 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingPluginConfig {
    /// 当前使用的插件名称: mock, openai, ollama
    #[serde(default = "default_embedding_plugin")]
    pub plugin: String,
    /// Mock 插件配置
    #[serde(default)]
    pub mock: Option<MockEmbeddingConfig>,
    /// OpenAI 插件配置
    #[serde(default)]
    pub openai: Option<OpenAIEmbeddingConfig>,
    /// Ollama 插件配置
    #[serde(default)]
    pub ollama: Option<OllamaEmbeddingConfig>,
}

impl Default for EmbeddingPluginConfig {
    fn default() -> Self {
        Self {
            plugin: default_embedding_plugin(),
            mock: None,
            openai: None,
            ollama: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEmbeddingConfig {
    #[serde(default = "default_mock_dimension")]
    pub dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIEmbeddingConfig {
    pub api_base: String,
    pub api_key: String,
    #[serde(default = "default_openai_model")]
    pub model: String,
    #[serde(default = "default_openai_dimension")]
    pub dimension: usize,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingConfig {
    #[serde(default = "default_ollama_url")]
    pub url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
    #[serde(default = "default_ollama_dimension")]
    pub dimension: usize,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_output")]
    pub output: String,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_log_output() -> String {
    "stdout".to_string()
}

/// AGFS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgfsConfig {
    #[serde(default = "default_scope")]
    pub default_scope: String,
    #[serde(default = "default_account")]
    pub default_account: String,
}

fn default_scope() -> String {
    "resources".to_string()
}
fn default_account() -> String {
    "default".to_string()
}

/// Root configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub vector: Option<VectorConfig>,
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
    #[serde(default)]
    pub agfs: Option<AgfsConfig>,
    #[serde(default)]
    pub vector_store: VectorStoreConfig,
    #[serde(default)]
    pub embedding: EmbeddingPluginConfig,
    #[serde(default)]
    pub summary: Option<SummaryConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RustVikingError::Config(format!("Failed to read config: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| RustVikingError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    /// Load with fallback to defaults
    pub fn load_or_default(path: &str) -> Self {
        Self::load(path).unwrap_or_default()
    }
}
