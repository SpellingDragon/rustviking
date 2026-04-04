//! Global error types for RustViking

use thiserror::Error;

/// Error classification for exit codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// User error (invalid arguments, resource not found, etc.) - exit code 1
    UserError,
    /// System error (IO error, storage failure, etc.) - exit code 2
    SystemError,
}

impl ErrorKind {
    /// Get the exit code for this error kind
    pub fn exit_code(&self) -> i32 {
        match self {
            ErrorKind::UserError => 1,
            ErrorKind::SystemError => 2,
        }
    }
}

/// Trait for classifying errors
pub trait ClassifyError {
    /// Classify this error into user error or system error
    fn classify(&self) -> ErrorKind;
}

/// RustViking 错误类型
#[derive(Error, Debug)]
pub enum RustVikingError {
    // AGFS 相关
    #[error("AGFS error: {0}")]
    Agfs(String),

    #[error("Mount point not found: {0}")]
    MountNotFound(String),

    // 存储相关
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    // 索引相关
    #[error("Index error: {0}")]
    Index(String),

    #[error("Invalid dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    // URI 相关
    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    // 配置相关
    #[error("Config error: {0}")]
    Config(String),

    // Embedding 相关
    #[error("Embedding error: {0}")]
    Embedding(String),

    // 向量存储相关
    #[error("Vector store error: {0}")]
    VectorStore(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    // IO 相关
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // 序列化
    #[error("Serialization error: {0}")]
    Serialization(String),

    // 通用
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Internal error: {0}")]
    Internal(String),

    // 插件相关
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    // 摘要相关（为 Task 3 预留）
    #[error("Summary error: {0}")]
    Summary(String),

    // VikingFS 相关（为 Task 2 预留）
    #[error("VikingFS error: {0}")]
    VikingFs(String),

    // CLI 输入相关
    #[error("Invalid CLI input: {0}")]
    CliInput(String),
}

impl ClassifyError for RustVikingError {
    fn classify(&self) -> ErrorKind {
        match self {
            // User errors - exit code 1
            RustVikingError::MountNotFound(_) => ErrorKind::UserError,
            RustVikingError::InvalidDimension { .. } => ErrorKind::UserError,
            RustVikingError::InvalidUri(_) => ErrorKind::UserError,
            RustVikingError::NotFound(_) => ErrorKind::UserError,
            RustVikingError::AlreadyExists(_) => ErrorKind::UserError,
            RustVikingError::PermissionDenied(_) => ErrorKind::UserError,
            RustVikingError::CollectionNotFound(_) => ErrorKind::UserError,
            RustVikingError::PluginNotFound(_) => ErrorKind::UserError,
            RustVikingError::CliInput(_) => ErrorKind::UserError,

            // System errors - exit code 2
            RustVikingError::Agfs(_) => ErrorKind::SystemError,
            RustVikingError::Storage(_) => ErrorKind::SystemError,
            RustVikingError::RocksDb(_) => ErrorKind::SystemError,
            RustVikingError::Index(_) => ErrorKind::SystemError,
            RustVikingError::Config(_) => ErrorKind::SystemError,
            RustVikingError::Embedding(_) => ErrorKind::SystemError,
            RustVikingError::VectorStore(_) => ErrorKind::SystemError,
            RustVikingError::Io(_) => ErrorKind::SystemError,
            RustVikingError::Serialization(_) => ErrorKind::SystemError,
            RustVikingError::Internal(_) => ErrorKind::SystemError,
            RustVikingError::Summary(_) => ErrorKind::SystemError,
            RustVikingError::VikingFs(_) => ErrorKind::SystemError,
        }
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, RustVikingError>;
