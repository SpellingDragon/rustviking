//! Plugin Registry Integration Tests
//!
//! Tests for PluginRegistry with VectorStore and EmbeddingProvider plugins.

use rustviking::embedding::mock::MockEmbeddingProvider;
use rustviking::embedding::{EmbeddingConfig, EmbeddingRequest};
use rustviking::plugins::{PluginInfo, PluginRegistry, PluginType};
use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::IndexParams;

// ============================================================================
// VectorStore Plugin Tests
// ============================================================================

#[test]
fn test_register_and_create_vector_store() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    let store = registry.create_vector_store("memory").unwrap();
    assert_eq!(store.name(), "memory");
    assert_eq!(store.version(), "0.1.0");
}

#[test]
fn test_vector_store_functional() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    let store = registry.create_vector_store("memory").unwrap();

    // Test create collection
    store
        .create_collection("test", 3, IndexParams::default())
        .unwrap();

    let info = store.collection_info("test").unwrap();
    assert_eq!(info.name, "test");
    assert_eq!(info.dimension, 3);
}

// ============================================================================
// Embedding Provider Plugin Tests
// ============================================================================

#[test]
fn test_register_and_create_embedding_provider() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(512)));

    let provider = registry.create_embedding_provider("mock").unwrap();
    assert_eq!(provider.name(), "mock");
    assert_eq!(provider.version(), "0.1.0");
}

#[test]
fn test_embedding_provider_functional() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(128)));

    let provider = registry.create_embedding_provider("mock").unwrap();

    let request = EmbeddingRequest {
        texts: vec!["Hello".to_string()],
        model: None,
        normalize: false,
    };

    let result = provider.embed(request).unwrap();
    assert_eq!(result.dimension, 128);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_create_unknown_vector_store() {
    let registry = PluginRegistry::new();

    let result = registry.create_vector_store("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_create_unknown_embedding_provider() {
    let registry = PluginRegistry::new();

    let result = registry.create_embedding_provider("nonexistent");
    assert!(result.is_err());
}

// ============================================================================
// List Plugins Tests
// ============================================================================

#[test]
fn test_list_plugins_empty() {
    let registry = PluginRegistry::new();
    let plugins = registry.list_plugins();

    assert!(plugins.is_empty());
}

#[test]
fn test_list_plugins_single_vector_store() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 1);

    let plugin = &plugins[0];
    assert_eq!(plugin.name, "memory");
    assert_eq!(plugin.plugin_type, PluginType::VectorStore);
}

#[test]
fn test_list_plugins_single_embedding_provider() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::default()));

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 1);

    let plugin = &plugins[0];
    assert_eq!(plugin.name, "mock");
    assert_eq!(plugin.plugin_type, PluginType::Embedding);
}

#[test]
fn test_list_plugins_multiple() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));
    registry.register_vector_store("memory2", || Box::new(MemoryVectorStore::new()));
    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::default()));
    registry.register_embedding_provider("mock2", || Box::new(MockEmbeddingProvider::new(512)));

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 4);

    // Count by type
    let vector_store_count = plugins
        .iter()
        .filter(|p| p.plugin_type == PluginType::VectorStore)
        .count();
    let embedding_count = plugins
        .iter()
        .filter(|p| p.plugin_type == PluginType::Embedding)
        .count();

    assert_eq!(vector_store_count, 2);
    assert_eq!(embedding_count, 2);
}

#[test]
fn test_list_plugins_info_content() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));
    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::default()));

    let plugins = registry.list_plugins();

    // Find vector store plugin
    let vs_plugin = plugins
        .iter()
        .find(|p| p.name == "memory" && p.plugin_type == PluginType::VectorStore);
    assert!(vs_plugin.is_some());
    let vs_plugin = vs_plugin.unwrap();
    assert!(vs_plugin.description.contains("Vector store"));
    assert!(vs_plugin.description.contains("memory"));

    // Find embedding plugin
    let emb_plugin = plugins
        .iter()
        .find(|p| p.name == "mock" && p.plugin_type == PluginType::Embedding);
    assert!(emb_plugin.is_some());
    let emb_plugin = emb_plugin.unwrap();
    assert!(emb_plugin.description.contains("Embedding"));
    assert!(emb_plugin.description.contains("mock"));
}

// ============================================================================
// Override Tests
// ============================================================================

#[test]
fn test_register_override_vector_store() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    // Register again with same name should override
    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    let plugins = registry.list_plugins();
    let count = plugins.iter().filter(|p| p.name == "memory").count();
    assert_eq!(count, 1);
}

#[test]
fn test_register_override_embedding_provider() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(512)));

    // Register again with same name should override
    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(1024)));

    let plugins = registry.list_plugins();
    let count = plugins.iter().filter(|p| p.name == "mock").count();
    assert_eq!(count, 1);
}

// ============================================================================
// Multiple Instance Tests
// ============================================================================

#[test]
fn test_create_multiple_vector_store_instances() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    let store1 = registry.create_vector_store("memory").unwrap();
    let store2 = registry.create_vector_store("memory").unwrap();

    // Each call should create a new instance
    store1
        .create_collection("test1", 64, IndexParams::default())
        .unwrap();
    store2
        .create_collection("test2", 128, IndexParams::default())
        .unwrap();

    // Verify they are independent
    let info1 = store1.collection_info("test1").unwrap();
    let info2 = store2.collection_info("test2").unwrap();

    assert_eq!(info1.dimension, 64);
    assert_eq!(info2.dimension, 128);
}

#[test]
fn test_create_multiple_embedding_instances() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(256)));

    let provider1 = registry.create_embedding_provider("mock").unwrap();
    let provider2 = registry.create_embedding_provider("mock").unwrap();

    // Each call creates a new instance with same default config
    assert_eq!(provider1.default_dimension(), 256);
    assert_eq!(provider2.default_dimension(), 256);

    // Modifying one should not affect the other
    provider1
        .initialize(EmbeddingConfig {
            dimension: 512,
            ..Default::default()
        })
        .unwrap();

    // provider2 should still have original dimension
    assert_eq!(provider2.default_dimension(), 256);
}

// ============================================================================
// Plugin Type Tests
// ============================================================================

#[test]
fn test_plugin_type_enum() {
    assert_eq!(PluginType::Storage, PluginType::Storage);
    assert_eq!(PluginType::VectorStore, PluginType::VectorStore);
    assert_eq!(PluginType::Embedding, PluginType::Embedding);

    assert_ne!(PluginType::Storage, PluginType::VectorStore);
    assert_ne!(PluginType::VectorStore, PluginType::Embedding);
}

#[test]
fn test_plugin_info_debug() {
    let info = PluginInfo {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: PluginType::VectorStore,
        description: "Test plugin".to_string(),
    };

    // Should implement Debug
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("test"));
    assert!(debug_str.contains("VectorStore"));
}

#[test]
fn test_plugin_info_clone() {
    let info = PluginInfo {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: PluginType::Embedding,
        description: "Test plugin".to_string(),
    };

    let cloned = info.clone();
    assert_eq!(cloned.name, info.name);
    assert_eq!(cloned.version, info.version);
    assert_eq!(cloned.plugin_type, info.plugin_type);
    assert_eq!(cloned.description, info.description);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_workflow_vector_store() {
    let mut registry = PluginRegistry::new();

    // Register plugin
    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    // Create instance
    let store = registry.create_vector_store("memory").unwrap();

    // Initialize
    store.initialize(&serde_json::json!({})).unwrap();

    // Create collection
    store
        .create_collection("documents", 256, IndexParams::default())
        .unwrap();

    // Insert vectors
    let points = vec![
        rustviking::vector_store::VectorPoint {
            id: "doc1".to_string(),
            vector: vec![0.1; 256],
            sparse_vector: None,
            payload: serde_json::json!({"uri": "/docs/doc1"}),
        },
        rustviking::vector_store::VectorPoint {
            id: "doc2".to_string(),
            vector: vec![0.2; 256],
            sparse_vector: None,
            payload: serde_json::json!({"uri": "/docs/doc2"}),
        },
    ];
    store.upsert("documents", points).unwrap();

    // Search
    let results = store
        .search("documents", &vec![0.15; 256], 10, None)
        .unwrap();
    assert_eq!(results.len(), 2);

    // Clean up
    store.delete_by_uri_prefix("documents", "/docs").unwrap();

    let info = store.collection_info("documents").unwrap();
    assert_eq!(info.count, 0);
}

#[test]
fn test_full_workflow_embedding() {
    let mut registry = PluginRegistry::new();

    // Register plugin
    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(128)));

    // Create instance
    let provider = registry.create_embedding_provider("mock").unwrap();

    // Initialize with config
    let config = EmbeddingConfig {
        dimension: 512,
        ..Default::default()
    };
    provider.initialize(config).unwrap();

    // Generate embeddings
    let request = EmbeddingRequest {
        texts: vec!["First document".to_string(), "Second document".to_string()],
        model: None,
        normalize: true,
    };

    let result = provider.embed(request).unwrap();
    assert_eq!(result.embeddings.len(), 2);
    assert_eq!(result.dimension, 512);

    // Verify normalization
    for embedding in &result.embeddings {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}

// ============================================================================
// RocksDB VectorStore Plugin Tests
// ============================================================================

#[test]
fn test_register_and_create_rocksdb_vector_store() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("rocksdb", || {
        Box::new(
            rustviking::vector_store::rocks::RocksDBVectorStore::with_path(
                tempfile::tempdir().unwrap().path().to_str().unwrap(),
            )
            .unwrap(),
        )
    });

    let store = registry.create_vector_store("rocksdb").unwrap();
    assert_eq!(store.name(), "rocksdb");
    assert_eq!(store.version(), "0.1.0");
}

#[test]
fn test_rocksdb_vector_store_functional() {
    use rustviking::vector_store::IndexParams;

    let mut registry = PluginRegistry::new();

    registry.register_vector_store("rocksdb", || {
        Box::new(
            rustviking::vector_store::rocks::RocksDBVectorStore::with_path(
                tempfile::tempdir().unwrap().path().to_str().unwrap(),
            )
            .unwrap(),
        )
    });

    let store = registry.create_vector_store("rocksdb").unwrap();

    // Test create collection
    store
        .create_collection("test", 3, IndexParams::default())
        .unwrap();

    let info = store.collection_info("test").unwrap();
    assert_eq!(info.name, "test");
    assert_eq!(info.dimension, 3);
}

#[test]
fn test_rocksdb_vector_store_upsert_and_search() {
    use rustviking::vector_store::types::VectorPoint;
    use rustviking::vector_store::IndexParams;
    use serde_json::json;

    let mut registry = PluginRegistry::new();

    registry.register_vector_store("rocksdb", || {
        Box::new(
            rustviking::vector_store::rocks::RocksDBVectorStore::with_path(
                tempfile::tempdir().unwrap().path().to_str().unwrap(),
            )
            .unwrap(),
        )
    });

    let store = registry.create_vector_store("rocksdb").unwrap();

    // Create collection
    store
        .create_collection("search_test", 3, IndexParams::default())
        .unwrap();

    // Insert vectors
    let points = vec![
        VectorPoint {
            id: "p1".to_string(),
            vector: vec![1.0, 0.0, 0.0],
            sparse_vector: None,
            payload: json!({"uri": "/docs/p1", "context_type": "resource"}),
        },
        VectorPoint {
            id: "p2".to_string(),
            vector: vec![0.0, 1.0, 0.0],
            sparse_vector: None,
            payload: json!({"uri": "/docs/p2", "context_type": "resource"}),
        },
    ];
    store.upsert("search_test", points).unwrap();

    // Search
    let results = store
        .search("search_test", &[1.0, 0.0, 0.0], 2, None)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "p1");
}

// ============================================================================
// OpenAI Embedding Provider Plugin Tests
// ============================================================================

#[test]
fn test_register_and_create_openai_embedding_provider() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("openai", || {
        Box::new(rustviking::embedding::openai::OpenAIEmbeddingProvider::new())
    });

    let provider = registry.create_embedding_provider("openai").unwrap();
    assert_eq!(provider.name(), "openai");
    assert_eq!(provider.version(), "0.1.0");
}

#[test]
fn test_openai_embedding_provider_functional() {
    use rustviking::embedding::types::EmbeddingConfig;

    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("openai", || {
        Box::new(rustviking::embedding::openai::OpenAIEmbeddingProvider::new())
    });

    let provider = registry.create_embedding_provider("openai").unwrap();

    // Initialize with config
    let config = EmbeddingConfig {
        api_base: "https://api.openai.com/v1".to_string(),
        api_key: Some("sk-test-key".to_string()),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        max_concurrent: 10,
    };

    assert!(provider.initialize(config).is_ok());
    assert_eq!(provider.default_dimension(), 1536);
}

#[test]
fn test_openai_embedding_provider_supported_models() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("openai", || {
        Box::new(rustviking::embedding::openai::OpenAIEmbeddingProvider::new())
    });

    let provider = registry.create_embedding_provider("openai").unwrap();

    let models = provider.supported_models();
    assert_eq!(models.len(), 3);
    assert!(models.contains(&"text-embedding-3-small"));
    assert!(models.contains(&"text-embedding-3-large"));
    assert!(models.contains(&"text-embedding-ada-002"));
}

// ============================================================================
// Mixed Plugin Registration Tests
// ============================================================================

#[test]
fn test_register_all_plugin_types() {
    let mut registry = PluginRegistry::new();

    // Register Memory VectorStore
    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    // Register RocksDB VectorStore
    registry.register_vector_store("rocksdb", || {
        Box::new(
            rustviking::vector_store::rocks::RocksDBVectorStore::with_path(
                tempfile::tempdir().unwrap().path().to_str().unwrap(),
            )
            .unwrap(),
        )
    });

    // Register Mock EmbeddingProvider
    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(512)));

    // Register OpenAI EmbeddingProvider
    registry.register_embedding_provider("openai", || {
        Box::new(rustviking::embedding::openai::OpenAIEmbeddingProvider::new())
    });

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 4);

    // Count by type
    let vector_store_count = plugins
        .iter()
        .filter(|p| p.plugin_type == PluginType::VectorStore)
        .count();
    let embedding_count = plugins
        .iter()
        .filter(|p| p.plugin_type == PluginType::Embedding)
        .count();

    assert_eq!(vector_store_count, 2);
    assert_eq!(embedding_count, 2);
}

#[test]
fn test_create_different_vector_store_types() {
    let mut registry = PluginRegistry::new();

    registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    registry.register_vector_store("rocksdb", || {
        Box::new(
            rustviking::vector_store::rocks::RocksDBVectorStore::with_path(
                tempfile::tempdir().unwrap().path().to_str().unwrap(),
            )
            .unwrap(),
        )
    });

    // Create memory store
    let memory_store = registry.create_vector_store("memory").unwrap();
    assert_eq!(memory_store.name(), "memory");

    // Create rocksdb store
    let rocksdb_store = registry.create_vector_store("rocksdb").unwrap();
    assert_eq!(rocksdb_store.name(), "rocksdb");
}

#[test]
fn test_create_different_embedding_providers() {
    let mut registry = PluginRegistry::new();

    registry.register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::new(512)));

    registry.register_embedding_provider("openai", || {
        Box::new(rustviking::embedding::openai::OpenAIEmbeddingProvider::new())
    });

    // Create mock provider
    let mock_provider = registry.create_embedding_provider("mock").unwrap();
    assert_eq!(mock_provider.name(), "mock");
    assert_eq!(mock_provider.default_dimension(), 512);

    // Create openai provider
    let openai_provider = registry.create_embedding_provider("openai").unwrap();
    assert_eq!(openai_provider.name(), "openai");
    assert_eq!(openai_provider.default_dimension(), 1536);
}

// ============================================================================
// Registry Default Tests
// ============================================================================

#[test]
fn test_registry_default() {
    let registry = PluginRegistry::default();
    let plugins = registry.list_plugins();
    assert!(plugins.is_empty());
}

#[test]
fn test_registry_new() {
    let registry = PluginRegistry::new();
    let plugins = registry.list_plugins();
    assert!(plugins.is_empty());
}
