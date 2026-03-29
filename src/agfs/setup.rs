//! AGFS Setup Utilities
//!
//! Provides initialization and mounting utilities for AGFS.

use std::sync::Arc;

use crate::error::Result;
use crate::plugins::localfs::LocalFSPlugin;
use crate::plugins::memory::MemoryPlugin;

use super::MountableFS;

/// Setup AGFS with standard mount points based on configuration
///
/// Creates and mounts the following standard paths:
/// - `/local` - Local filesystem storage
/// - `/memory` - In-memory filesystem
/// - `/resources` - Resources storage
/// - `/user` - User storage
/// - `/agent` - Agent storage
///
/// # Arguments
/// * `storage_path` - Base path for storage
///
/// # Returns
/// * `MountableFS` - Configured mountable filesystem
pub fn setup_agfs(storage_path: &str) -> Result<MountableFS> {
    let agfs = MountableFS::new();

    // Mount local filesystem
    let local_path = format!("{}/local", storage_path);
    let local_plugin = LocalFSPlugin::new(&local_path)?;
    agfs.mount("/local", Arc::new(local_plugin), 100)?;

    // Mount memory filesystem
    let mem_plugin = MemoryPlugin::new();
    agfs.mount("/memory", Arc::new(mem_plugin), 50)?;

    // Mount default resource paths
    let resources_path = format!("{}/resources", storage_path);
    let resources_plugin = LocalFSPlugin::new(&resources_path)?;
    agfs.mount("/resources", Arc::new(resources_plugin), 100)?;

    let user_path = format!("{}/user", storage_path);
    let user_plugin = LocalFSPlugin::new(&user_path)?;
    agfs.mount("/user", Arc::new(user_plugin), 100)?;

    let agent_path = format!("{}/agent", storage_path);
    let agent_plugin = LocalFSPlugin::new(&agent_path)?;
    agfs.mount("/agent", Arc::new(agent_plugin), 100)?;

    Ok(agfs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_setup_agfs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        let agfs = setup_agfs(path).unwrap();

        // Verify mounts exist by checking we can route to them
        assert!(agfs.route("/local/test").is_some());
        assert!(agfs.route("/memory/test").is_some());
        assert!(agfs.route("/resources/test").is_some());
        assert!(agfs.route("/user/test").is_some());
        assert!(agfs.route("/agent/test").is_some());
    }
}
