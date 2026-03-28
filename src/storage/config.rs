//! Storage configuration

use serde::{Deserialize, Serialize};

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data storage path
    pub path: String,
    /// Create database if missing
    #[serde(default = "default_true")]
    pub create_if_missing: bool,
    /// Maximum open files
    #[serde(default = "default_max_open_files")]
    pub max_open_files: i32,
    /// Use fsync for writes
    #[serde(default)]
    pub use_fsync: bool,
    /// Block cache size in bytes
    pub block_cache_size: Option<usize>,
}

fn default_true() -> bool {
    true
}
fn default_max_open_files() -> i32 {
    10000
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: "./data/rustviking".to_string(),
            create_if_missing: true,
            max_open_files: 10000,
            use_fsync: false,
            block_cache_size: None,
        }
    }
}
