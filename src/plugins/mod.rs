//! Storage plugins
//!
//! Plugin system for AGFS filesystem backends.

pub mod localfs;
pub mod memory;

use crate::error::{Result, RustVikingError};
use crate::agfs::FileSystem;

/// Plugin metadata
pub struct PluginInfo {
    pub name: String,
    pub version: String,
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

/// Plugin factory type
type PluginFactory = Box<dyn Fn() -> Box<dyn StoragePlugin> + Send + Sync>;

/// Plugin registry for managing plugin factories
pub struct PluginRegistry {
    factories: HashMap<String, PluginFactory>,
}

use std::collections::HashMap;

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn StoragePlugin> + Send + Sync + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }

    pub fn create(&self, name: &str) -> Result<Box<dyn StoragePlugin>> {
        let factory = self.factories.get(name)
            .ok_or_else(|| RustVikingError::PluginNotFound(name.into()))?;
        
        Ok(factory())
    }

    pub fn list(&self) -> Vec<&str> {
        self.factories.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
