//! Global error types for RustViking

use thiserror::Error;

/// RustViking 错误类型
#[derive(Error, Debug)]
pub enum RustVikingError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Invalid dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, RustVikingError>;
