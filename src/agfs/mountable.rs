//! Mountable FileSystem
//!
//! Plugin-based filesystem with Radix Tree routing.

use crate::agfs::FileSystem;
use crate::error::{Result, RustVikingError};
use radix_trie::{Trie, TrieCommon};
use std::sync::{Arc, RwLock};

/// Mount point
pub struct MountPoint {
    pub path: String,
    pub plugin: Arc<dyn FileSystem>,
    pub priority: u32,
}

/// Mountable filesystem with plugin routing
pub struct MountableFS {
    mount_tree: RwLock<Trie<String, MountPoint>>,
}

impl MountableFS {
    pub fn new() -> Self {
        Self {
            mount_tree: RwLock::new(Trie::new()),
        }
    }

    /// Mount a plugin to a path
    pub fn mount(&self, path: &str, plugin: Arc<dyn FileSystem>, priority: u32) -> Result<()> {
        let mut tree = self
            .mount_tree
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        tree.insert(
            path.to_string(),
            MountPoint {
                path: path.to_string(),
                plugin,
                priority,
            },
        );

        Ok(())
    }

    /// Unmount a path
    pub fn unmount(&self, path: &str) -> Result<()> {
        let mut tree = self
            .mount_tree
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        tree.remove(path);
        Ok(())
    }

    /// Route lookup - longest prefix match
    pub fn route(&self, path: &str) -> Option<Arc<dyn FileSystem>> {
        let tree = self.mount_tree.read().ok()?;

        // Find the longest matching prefix
        let mut best_key_len: usize = 0;
        let mut best_plugin: Option<Arc<dyn FileSystem>> = None;

        for (key, value) in tree.iter() {
            if path.starts_with(key.as_str()) && key.len() > best_key_len {
                best_key_len = key.len();
                best_plugin = Some(Arc::clone(&value.plugin));
            }
        }

        best_plugin
    }

    /// Forward filesystem operation
    pub fn route_operation<F, R>(&self, path: &str, op: F) -> Result<R>
    where
        F: FnOnce(&dyn FileSystem) -> Result<R>,
    {
        let fs = self
            .route(path)
            .ok_or_else(|| RustVikingError::NotFound(path.into()))?;

        op(fs.as_ref())
    }
}

impl Default for MountableFS {
    fn default() -> Self {
        Self::new()
    }
}
