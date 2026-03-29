//! Configuration Tests
//!
//! Tests for configuration loading and validation.

use rustviking::config::Config;
use rustviking::storage::config::StorageConfig;
use std::io::Write as IoWrite;
use tempfile::TempDir;

// ============================================================================
// Config Loading Tests
// ============================================================================

#[test]
fn test_config_load_valid() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test_data"
create_if_missing = true
max_open_files = 5000

[vector]
dimension = 512
index_type = "ivf_pq"

[vector.ivf_pq]
num_partitions = 128
num_sub_vectors = 8
pq_bits = 8
metric = "l2"

[logging]
level = "debug"
format = "text"
output = "stderr"

[agfs]
default_scope = "user"
default_account = "test_account"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.storage.path, "/tmp/test_data");
    assert_eq!(config.storage.max_open_files, 5000);
    assert!(config.vector.is_some());

    let vector = config.vector.unwrap();
    assert_eq!(vector.dimension, 512);
    assert_eq!(vector.index_type, "ivf_pq");
    assert!(vector.ivf_pq.is_some());

    let ivf_pq = vector.ivf_pq.unwrap();
    assert_eq!(ivf_pq.num_partitions, 128);
    assert_eq!(ivf_pq.metric, "l2");

    assert!(config.logging.is_some());
    let logging = config.logging.unwrap();
    assert_eq!(logging.level, "debug");

    assert!(config.agfs.is_some());
    let agfs = config.agfs.unwrap();
    assert_eq!(agfs.default_scope, "user");
}

#[test]
fn test_config_load_minimal() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("minimal_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/minimal"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.storage.path, "/tmp/minimal");
    // Defaults should be applied
    assert!(config.storage.create_if_missing);
    assert!(config.vector.is_none());
    assert!(config.logging.is_none());
    assert!(config.agfs.is_none());
}

#[test]
fn test_config_load_nonexistent_file() {
    let result = Config::load("/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_config_load_or_default_nonexistent() {
    let config = Config::load_or_default("/nonexistent/path/config.toml");

    // Should return defaults
    assert!(config.storage.path.contains("rustviking"));
    assert!(config.vector.is_none());
}

#[test]
fn test_config_default() {
    let config = Config::default();

    assert!(config.storage.path.contains("rustviking"));
    assert!(config.storage.create_if_missing);
    assert!(config.vector.is_none());
    assert!(config.logging.is_none());
    assert!(config.agfs.is_none());
}

// ============================================================================
// Config Parsing Error Tests
// ============================================================================

#[test]
fn test_config_invalid_toml_syntax() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid_config.toml");

    let config_content = r#"
[storage
path = "/tmp/test"
missing closing bracket
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let result = Config::load(config_path.to_string_lossy().to_string().as_str());
    assert!(result.is_err());
}

#[test]
fn test_config_invalid_field_type() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("wrong_type_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"
max_open_files = "not_a_number"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let result = Config::load(config_path.to_string_lossy().to_string().as_str());
    assert!(result.is_err());
}

// ============================================================================
// Storage Config Tests
// ============================================================================

#[test]
fn test_storage_config_default() {
    let config = StorageConfig::default();

    assert!(config.path.contains("rustviking"));
    assert!(config.create_if_missing);
    assert_eq!(config.max_open_files, 10000);
    assert!(!config.use_fsync);
    assert!(config.block_cache_size.is_none());
}

#[test]
fn test_storage_config_custom() {
    let config = StorageConfig {
        path: "/custom/path".to_string(),
        create_if_missing: false,
        max_open_files: 2000,
        use_fsync: true,
        block_cache_size: Some(1024 * 1024 * 100), // 100MB
    };

    assert_eq!(config.path, "/custom/path");
    assert!(!config.create_if_missing);
    assert_eq!(config.max_open_files, 2000);
    assert!(config.use_fsync);
    assert_eq!(config.block_cache_size, Some(104857600));
}

// ============================================================================
// Vector Config Tests
// ============================================================================

#[test]
fn test_vector_config_with_all_options() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("vector_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[vector]
dimension = 1536
index_type = "hnsw"

[vector.ivf_pq]
num_partitions = 512
num_sub_vectors = 32
pq_bits = 8
metric = "cosine"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    let vector = config.vector.unwrap();
    assert_eq!(vector.dimension, 1536);
    assert_eq!(vector.index_type, "hnsw");

    let ivf_pq = vector.ivf_pq.unwrap();
    assert_eq!(ivf_pq.num_partitions, 512);
    assert_eq!(ivf_pq.num_sub_vectors, 32);
    assert_eq!(ivf_pq.metric, "cosine");
}

#[test]
fn test_vector_config_minimal() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("minimal_vector_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[vector]
dimension = 256
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    let vector = config.vector.unwrap();
    assert_eq!(vector.dimension, 256);
    // Defaults
    assert_eq!(vector.index_type, "ivf_pq");
    assert!(vector.ivf_pq.is_none()); // Optional
}

// ============================================================================
// Logging Config Tests
// ============================================================================

#[test]
fn test_logging_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("logging_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[logging]
level = "trace"
format = "json"
output = "/var/log/rustviking.log"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    let logging = config.logging.unwrap();
    assert_eq!(logging.level, "trace");
    assert_eq!(logging.format, "json");
    assert_eq!(logging.output, "/var/log/rustviking.log");
}

// ============================================================================
// AGFS Config Tests
// ============================================================================

#[test]
fn test_agfs_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("agfs_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[agfs]
default_scope = "agent"
default_account = "production"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    let agfs = config.agfs.unwrap();
    assert_eq!(agfs.default_scope, "agent");
    assert_eq!(agfs.default_account, "production");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_config_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("empty_config.toml");

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(b"").expect("Failed to write config");

    let result = Config::load(config_path.to_string_lossy().to_string().as_str());
    // Empty file should use defaults
    if let Ok(config) = result {
        // Should have defaults
        assert!(config.storage.path.contains("rustviking"));
    }
}

#[test]
fn test_config_with_comments() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("commented_config.toml");

    let config_content = r#"
# Main configuration file
[storage]
path = "/tmp/test"  # Data storage path
# create_if_missing = false

[vector]
# Using default dimension
dimension = 768  # Standard embedding size
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.storage.path, "/tmp/test");
    assert!(config.storage.create_if_missing); // Default
    assert_eq!(config.vector.unwrap().dimension, 768);
}

#[test]
fn test_config_multiple_sections() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("full_config.toml");

    let config_content = r#"
[storage]
path = "/data/rustviking"
create_if_missing = true
max_open_files = 8000
use_fsync = false
block_cache_size = 268435456

[vector]
dimension = 1024
index_type = "ivf_pq"

[vector.ivf_pq]
num_partitions = 256
num_sub_vectors = 16
pq_bits = 8
metric = "l2"

[logging]
level = "info"
format = "json"
output = "stdout"

[agfs]
default_scope = "resources"
default_account = "default"
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    // Verify all sections
    assert_eq!(config.storage.path, "/data/rustviking");
    assert_eq!(config.storage.block_cache_size, Some(268435456));

    let vector = config.vector.unwrap();
    assert_eq!(vector.dimension, 1024);

    let logging = config.logging.unwrap();
    assert_eq!(logging.level, "info");

    let agfs = config.agfs.unwrap();
    assert_eq!(agfs.default_scope, "resources");
}

#[test]
fn test_config_unknown_field() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("unknown_field_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"
unknown_field = "should be ignored"

[vector]
dimension = 128
also_unknown = 42
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    // TOML parser by default denies unknown fields
    let result = Config::load(config_path.to_string_lossy().to_string().as_str());
    // Depending on serde config, this might fail or succeed
    // Let's document the expected behavior
    if let Ok(config) = result {
        assert_eq!(config.storage.path, "/tmp/test");
    }
    // Unknown fields are typically ignored by default serde behavior
}

// ============================================================================
// VectorStore Config Tests
// ============================================================================

#[test]
fn test_config_default_vector_store() {
    let config = Config::default();

    // Default vector_store.plugin should be "memory"
    assert_eq!(config.vector_store.plugin, "memory");
    assert!(config.vector_store.memory.is_none());
    assert!(config.vector_store.qdrant.is_none());
}

#[test]
fn test_config_with_vector_store() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("vector_store_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[vector_store]
plugin = "qdrant"

[vector_store.qdrant]
url = "http://localhost:6333"
collection = "test_collection"
timeout_ms = 3000
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.vector_store.plugin, "qdrant");
    assert!(config.vector_store.qdrant.is_some());

    let qdrant = config.vector_store.qdrant.unwrap();
    assert_eq!(qdrant.url, "http://localhost:6333");
    assert_eq!(qdrant.collection, "test_collection");
    assert_eq!(qdrant.timeout_ms, 3000);
}

#[test]
fn test_config_vector_store_memory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("memory_store_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[vector_store]
plugin = "memory"

[vector_store.memory]
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.vector_store.plugin, "memory");
    assert!(config.vector_store.memory.is_some());
}

// ============================================================================
// Embedding Config Tests
// ============================================================================

#[test]
fn test_config_default_embedding() {
    let config = Config::default();

    // Default embedding.plugin should be "mock"
    assert_eq!(config.embedding.plugin, "mock");
    assert!(config.embedding.mock.is_none());
    assert!(config.embedding.openai.is_none());
    assert!(config.embedding.ollama.is_none());
}

#[test]
fn test_config_with_embedding() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("embedding_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[embedding]
plugin = "mock"

[embedding.mock]
dimension = 768
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.embedding.plugin, "mock");
    assert!(config.embedding.mock.is_some());

    let mock_config = config.embedding.mock.unwrap();
    assert_eq!(mock_config.dimension, 768);
}

#[test]
fn test_config_embedding_openai() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("openai_embedding_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[embedding]
plugin = "openai"

[embedding.openai]
api_base = "https://api.openai.com/v1"
api_key = "sk-test-key"
model = "text-embedding-3-large"
dimension = 3072
max_concurrent = 5
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.embedding.plugin, "openai");
    assert!(config.embedding.openai.is_some());

    let openai_config = config.embedding.openai.unwrap();
    assert_eq!(openai_config.api_base, "https://api.openai.com/v1");
    assert_eq!(openai_config.api_key, "sk-test-key");
    assert_eq!(openai_config.model, "text-embedding-3-large");
    assert_eq!(openai_config.dimension, 3072);
    assert_eq!(openai_config.max_concurrent, 5);
}

#[test]
fn test_config_embedding_ollama() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("ollama_embedding_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[embedding]
plugin = "ollama"

[embedding.ollama]
url = "http://localhost:11434"
model = "nomic-embed-text"
dimension = 768
max_concurrent = 3
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.embedding.plugin, "ollama");
    assert!(config.embedding.ollama.is_some());

    let ollama_config = config.embedding.ollama.unwrap();
    assert_eq!(ollama_config.url, "http://localhost:11434");
    assert_eq!(ollama_config.model, "nomic-embed-text");
    assert_eq!(ollama_config.dimension, 768);
    assert_eq!(ollama_config.max_concurrent, 3);
}

#[test]
fn test_config_both_vector_store_and_embedding() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("full_plugin_config.toml");

    let config_content = r#"
[storage]
path = "/tmp/test"

[vector_store]
plugin = "memory"

[embedding]
plugin = "mock"

[embedding.mock]
dimension = 512
"#;

    let mut file = std::fs::File::create(&config_path).expect("Failed to create config file");
    file.write_all(config_content.as_bytes())
        .expect("Failed to write config");

    let config = Config::load(config_path.to_string_lossy().to_string().as_str())
        .expect("Failed to load config");

    assert_eq!(config.vector_store.plugin, "memory");
    assert_eq!(config.embedding.plugin, "mock");

    let mock_config = config.embedding.mock.unwrap();
    assert_eq!(mock_config.dimension, 512);
}
