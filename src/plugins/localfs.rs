//! Local filesystem plugin
//!
//! Implements FileSystem trait using the local filesystem.

use crate::agfs::{FileSystem, FileInfo, WriteFlag};
use crate::error::{Result, RustVikingError};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

/// Local filesystem plugin
pub struct LocalFSPlugin {
    base_path: PathBuf,
}

impl LocalFSPlugin {
    pub fn new(base_path: &str) -> Result<Self> {
        let path = PathBuf::from(base_path);
        // Create base directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(Self { base_path: path })
    }

    /// Resolve a virtual path to an absolute filesystem path
    fn resolve_path(&self, path: &str) -> PathBuf {
        let clean = path.trim_start_matches('/');
        self.base_path.join(clean)
    }

    /// Get timestamp as i64
    fn system_time_to_i64(time: SystemTime) -> i64 {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

impl FileSystem for LocalFSPlugin {
    fn create(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::File::create(&full_path)?;
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }
        if full_path.is_dir() {
            fs::remove_dir(&full_path)?;
        } else {
            fs::remove_file(&full_path)?;
        }
        Ok(())
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old = self.resolve_path(old_path);
        let new = self.resolve_path(new_path);
        if !old.exists() {
            return Err(RustVikingError::NotFound(old_path.into()));
        }
        if let Some(parent) = new.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&old, &new)?;
        Ok(())
    }

    fn mkdir(&self, path: &str, _mode: u32) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir_all(&full_path)?;
        Ok(())
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(&full_path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            entries.push(FileInfo {
                name,
                size: metadata.len(),
                mode: 0o644,
                is_dir: metadata.is_dir(),
                created_at: metadata.created()
                    .map(Self::system_time_to_i64)
                    .unwrap_or(0),
                updated_at: metadata.modified()
                    .map(Self::system_time_to_i64)
                    .unwrap_or(0),
                metadata: Vec::new(),
            });
        }
        Ok(entries)
    }

    fn remove_all(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Ok(());
        }
        if full_path.is_dir() {
            fs::remove_dir_all(&full_path)?;
        } else {
            fs::remove_file(&full_path)?;
        }
        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: u64) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }
        
        let data = fs::read(&full_path)?;
        
        let start = if offset >= 0 { offset as usize } else { 0 };
        let end = if size == 0 {
            data.len()
        } else {
            std::cmp::min(start + size as usize, data.len())
        };
        
        if start >= data.len() {
            return Ok(Vec::new());
        }
        
        Ok(data[start..end].to_vec())
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, flags: WriteFlag) -> Result<u64> {
        let full_path = self.resolve_path(path);
        
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if flags.has(WriteFlag::APPEND) {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full_path)?;
            file.write_all(data)?;
        } else {
            fs::write(&full_path, data)?;
        }
        
        Ok(data.len() as u64)
    }

    fn size(&self, path: &str) -> Result<u64> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }
        let metadata = fs::metadata(&full_path)?;
        Ok(metadata.len())
    }

    fn stat(&self, path: &str) -> Result<FileInfo> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }
        
        let metadata = fs::metadata(&full_path)?;
        let name = full_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        Ok(FileInfo {
            name,
            size: metadata.len(),
            mode: 0o644,
            is_dir: metadata.is_dir(),
            created_at: metadata.created()
                .map(Self::system_time_to_i64)
                .unwrap_or(0),
            updated_at: metadata.modified()
                .map(Self::system_time_to_i64)
                .unwrap_or(0),
            metadata: Vec::new(),
        })
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve_path(path).exists()
    }

    fn open_read(&self, path: &str) -> Result<Box<dyn Read + Send>> {
        let full_path = self.resolve_path(path);
        if !full_path.exists() {
            return Err(RustVikingError::NotFound(path.into()));
        }
        let file = fs::File::open(&full_path)?;
        Ok(Box::new(file))
    }

    fn open_write(&self, path: &str, flags: WriteFlag) -> Result<Box<dyn Write + Send>> {
        let full_path = self.resolve_path(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(flags.has(WriteFlag::APPEND))
            .truncate(!flags.has(WriteFlag::APPEND))
            .open(&full_path)?;
        Ok(Box::new(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_plugin() -> (LocalFSPlugin, TempDir) {
        let dir = TempDir::new().unwrap();
        let plugin = LocalFSPlugin::new(dir.path().to_str().unwrap()).unwrap();
        (plugin, dir)
    }

    #[test]
    fn test_create_and_exists() {
        let (plugin, _dir) = create_test_plugin();
        plugin.create("/test.txt").unwrap();
        assert!(plugin.exists("/test.txt"));
    }

    #[test]
    fn test_write_and_read() {
        let (plugin, _dir) = create_test_plugin();
        plugin.write("/hello.txt", b"Hello!", 0, WriteFlag::CREATE).unwrap();
        let data = plugin.read("/hello.txt", 0, 0).unwrap();
        assert_eq!(data, b"Hello!");
    }

    #[test]
    fn test_mkdir_and_readdir() {
        let (plugin, _dir) = create_test_plugin();
        plugin.mkdir("/mydir", 0o755).unwrap();
        plugin.write("/mydir/file.txt", b"content", 0, WriteFlag::CREATE).unwrap();
        let entries = plugin.read_dir("/mydir").unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_remove() {
        let (plugin, _dir) = create_test_plugin();
        plugin.write("/todelete.txt", b"data", 0, WriteFlag::CREATE).unwrap();
        assert!(plugin.exists("/todelete.txt"));
        plugin.remove("/todelete.txt").unwrap();
        assert!(!plugin.exists("/todelete.txt"));
    }

    #[test]
    fn test_stat() {
        let (plugin, _dir) = create_test_plugin();
        plugin.write("/info.txt", b"12345", 0, WriteFlag::CREATE).unwrap();
        let info = plugin.stat("/info.txt").unwrap();
        assert_eq!(info.size, 5);
        assert!(!info.is_dir);
    }
}
