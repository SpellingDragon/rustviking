//! Storage plugins
//!
//! Plugin system for AGFS filesystem backends.

pub mod localfs;
pub mod memory;

use crate::agfs::FileSystem;
use crate::error::{Result, RustVikingError};

use crate::embedding::EmbeddingProvider;
use crate::vector_store::VectorStore;

use std::collections::HashMap;

/// Plugin type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Storage,
    VectorStore,
    Embedding,
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub description: String,
}

/// Storage plugin trait - extends FileSystem with lifecycle
pub trait StoragePlugin: FileSystem {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin version
    fn version(&self) -> &str;

    /// Validate configuration
    fn validate_config(&self, config: &toml::Value) -> Result<()>;

    /// Initialize plugin
    fn initialize(&self, config: &toml::Value) -> Result<()>;

    /// Shutdown plugin
    fn shutdown(&self) -> Result<()>;
}

/// Storage plugin factory type
type StoragePluginFactory = Box<dyn Fn() -> Box<dyn StoragePlugin> + Send + Sync>;

/// Vector store plugin factory type
type VectorStoreFactory = Box<dyn Fn() -> Box<dyn VectorStore> + Send + Sync>;

/// Embedding provider plugin factory type
type EmbeddingProviderFactory = Box<dyn Fn() -> Box<dyn EmbeddingProvider> + Send + Sync>;

/// Plugin registry for managing plugin factories
pub struct PluginRegistry {
    storage_factories: HashMap<String, StoragePluginFactory>,
    vector_stores: HashMap<String, VectorStoreFactory>,
    embedding_providers: HashMap<String, EmbeddingProviderFactory>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            storage_factories: HashMap::new(),
            vector_stores: HashMap::new(),
            embedding_providers: HashMap::new(),
        }
    }

    /// Register a storage plugin factory
    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn StoragePlugin> + Send + Sync + 'static,
    {
        self.storage_factories
            .insert(name.to_string(), Box::new(factory));
    }

    /// Create a storage plugin instance by name
    pub fn create(&self, name: &str) -> Result<Box<dyn StoragePlugin>> {
        let factory = self
            .storage_factories
            .get(name)
            .ok_or_else(|| RustVikingError::PluginNotFound(name.into()))?;

        Ok(factory())
    }

    /// List all registered storage plugin names
    pub fn list(&self) -> Vec<&str> {
        self.storage_factories.keys().map(|k| k.as_str()).collect()
    }

    /// Register a vector store plugin factory
    pub fn register_vector_store<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn VectorStore> + Send + Sync + 'static,
    {
        self.vector_stores
            .insert(name.to_string(), Box::new(factory));
    }

    /// Register an embedding provider plugin factory
    pub fn register_embedding_provider<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn EmbeddingProvider> + Send + Sync + 'static,
    {
        self.embedding_providers
            .insert(name.to_string(), Box::new(factory));
    }

    /// Create a vector store instance by name
    pub fn create_vector_store(&self, name: &str) -> Result<Box<dyn VectorStore>> {
        let factory = self
            .vector_stores
            .get(name)
            .ok_or_else(|| RustVikingError::PluginNotFound(name.into()))?;

        Ok(factory())
    }

    /// Create an embedding provider instance by name
    pub fn create_embedding_provider(&self, name: &str) -> Result<Box<dyn EmbeddingProvider>> {
        let factory = self
            .embedding_providers
            .get(name)
            .ok_or_else(|| RustVikingError::PluginNotFound(name.into()))?;

        Ok(factory())
    }

    /// List all registered plugins with their metadata
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut plugins = Vec::new();

        // Add storage plugins
        for name in self.storage_factories.keys() {
            plugins.push(PluginInfo {
                name: name.clone(),
                version: "0.1.0".to_string(),
                plugin_type: PluginType::Storage,
                description: format!("Storage plugin: {}", name),
            });
        }

        // Add vector store plugins
        for name in self.vector_stores.keys() {
            plugins.push(PluginInfo {
                name: name.clone(),
                version: "0.1.0".to_string(),
                plugin_type: PluginType::VectorStore,
                description: format!("Vector store plugin: {}", name),
            });
        }

        // Add embedding provider plugins
        for name in self.embedding_providers.keys() {
            plugins.push(PluginInfo {
                name: name.clone(),
                version: "0.1.0".to_string(),
                plugin_type: PluginType::Embedding,
                description: format!("Embedding provider plugin: {}", name),
            });
        }

        plugins
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
