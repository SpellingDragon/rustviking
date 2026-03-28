//! Configuration loader

use serde::{Serialize, Deserialize};
use crate::error::{Result, RustVikingError};
use crate::storage::config::StorageConfig;

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

fn default_dimension() -> usize { 768 }
fn default_index_type() -> String { "ivf_pq".to_string() }

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

fn default_partitions() -> usize { 256 }
fn default_sub_vectors() -> usize { 16 }
fn default_pq_bits() -> usize { 8 }
fn default_metric() -> String { "l2".to_string() }

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

fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "json".to_string() }
fn default_log_output() -> String { "stdout".to_string() }

/// AGFS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgfsConfig {
    #[serde(default = "default_scope")]
    pub default_scope: String,
    #[serde(default = "default_account")]
    pub default_account: String,
}

fn default_scope() -> String { "resources".to_string() }
fn default_account() -> String { "default".to_string() }

/// Root configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub vector: Option<VectorConfig>,
    #[serde(default)]
    pub logging: Option<LoggingConfig>,
    #[serde(default)]
    pub agfs: Option<AgfsConfig>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            vector: None,
            logging: None,
            agfs: None,
        }
    }
}
