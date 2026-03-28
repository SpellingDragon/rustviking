//! In-memory filesystem plugin
//!
//! HashMap-based filesystem for testing and temporary storage.

use crate::agfs::{FileInfo, FileSystem, WriteFlag};
use crate::error::{Result, RustVikingError};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::sync::RwLock;

/// Entry in the in-memory filesystem
#[derive(Debug, Clone)]
enum MemEntry {
    File { data: Vec<u8>, mode: u32 },
    Dir { mode: u32 },
}

/// In-memory filesystem plugin
pub struct MemoryPlugin {
    entries: RwLock<HashMap<String, MemEntry>>,
}

impl MemoryPlugin {
    pub fn new() -> Self {
        let mut entries = HashMap::new();
        // Create root directory
        entries.insert("/".to_string(), MemEntry::Dir { mode: 0o755 });
        Self {
            entries: RwLock::new(entries),
        }
    }

    /// Normalize path
    fn normalize_path(path: &str) -> String {
        let mut normalized = path.to_string();
        if !normalized.starts_with('/') {
            normalized = format!("/{}", normalized);
        }
        // Remove trailing slash unless root
        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }
        normalized
    }

    fn now_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

impl Default for MemoryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for MemoryPlugin {
    fn create(&self, path: &str) -> Result<()> {
        let path = Self::normalize_path(path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        entries.insert(
            path,
            MemEntry::File {
                data: Vec::new(),
                mode: 0o644,
            },
        );
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<()> {
        let path = Self::normalize_path(path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        if entries.remove(&path).is_none() {
            return Err(RustVikingError::NotFound(path));
        }
        Ok(())
    }

    fn rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old = Self::normalize_path(old_path);
        let new = Self::normalize_path(new_path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        let entry = entries.remove(&old).ok_or(RustVikingError::NotFound(old))?;
        entries.insert(new, entry);
        Ok(())
    }

    fn mkdir(&self, path: &str, mode: u32) -> Result<()> {
        let path = Self::normalize_path(path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;
        entries.insert(path, MemEntry::Dir { mode });
        Ok(())
    }

    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>> {
        let path = Self::normalize_path(path);
        let entries = self
            .entries
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        // Check directory exists
        match entries.get(&path) {
            Some(MemEntry::Dir { .. }) => {}
            _ => return Err(RustVikingError::NotFound(path)),
        }

        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{}/", path)
        };
        let mut results = Vec::new();

        for (entry_path, entry) in entries.iter() {
            if entry_path == &path {
                continue;
            }
            if !entry_path.starts_with(&prefix) {
                continue;
            }

            // Only direct children (no deeper nesting)
            let relative = &entry_path[prefix.len()..];
            if relative.contains('/') {
                continue;
            }

            let now = Self::now_timestamp();
            match entry {
                MemEntry::File { data, mode } => {
                    results.push(FileInfo {
                        name: relative.to_string(),
                        size: data.len() as u64,
                        mode: *mode,
                        is_dir: false,
                        created_at: now,
                        updated_at: now,
                        metadata: Vec::new(),
                    });
                }
                MemEntry::Dir { mode } => {
                    results.push(FileInfo {
                        name: relative.to_string(),
                        size: 0,
                        mode: *mode,
                        is_dir: true,
                        created_at: now,
                        updated_at: now,
                        metadata: Vec::new(),
                    });
                }
            }
        }
        Ok(results)
    }

    fn remove_all(&self, path: &str) -> Result<()> {
        let path = Self::normalize_path(path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        let prefix = format!("{}/", path);
        let keys_to_remove: Vec<String> = entries
            .keys()
            .filter(|k| *k == &path || k.starts_with(&prefix))
            .cloned()
            .collect();

        for key in keys_to_remove {
            entries.remove(&key);
        }
        Ok(())
    }

    fn read(&self, path: &str, offset: i64, size: u64) -> Result<Vec<u8>> {
        let path = Self::normalize_path(path);
        let entries = self
            .entries
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        match entries.get(&path) {
            Some(MemEntry::File { data, .. }) => {
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
            _ => Err(RustVikingError::NotFound(path)),
        }
    }

    fn write(&self, path: &str, data: &[u8], _offset: i64, flags: WriteFlag) -> Result<u64> {
        let path = Self::normalize_path(path);
        let mut entries = self
            .entries
            .write()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        if flags.has(WriteFlag::APPEND) {
            if let Some(MemEntry::File { data: existing, .. }) = entries.get_mut(&path) {
                existing.extend_from_slice(data);
                return Ok(data.len() as u64);
            }
        }

        entries.insert(
            path,
            MemEntry::File {
                data: data.to_vec(),
                mode: 0o644,
            },
        );
        Ok(data.len() as u64)
    }

    fn size(&self, path: &str) -> Result<u64> {
        let path = Self::normalize_path(path);
        let entries = self
            .entries
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        match entries.get(&path) {
            Some(MemEntry::File { data, .. }) => Ok(data.len() as u64),
            Some(MemEntry::Dir { .. }) => Ok(0),
            None => Err(RustVikingError::NotFound(path)),
        }
    }

    fn stat(&self, path: &str) -> Result<FileInfo> {
        let path = Self::normalize_path(path);
        let entries = self
            .entries
            .read()
            .map_err(|_| RustVikingError::Internal("lock poisoned".into()))?;

        let now = Self::now_timestamp();
        match entries.get(&path) {
            Some(MemEntry::File { data, mode }) => {
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                Ok(FileInfo {
                    name,
                    size: data.len() as u64,
                    mode: *mode,
                    is_dir: false,
                    created_at: now,
                    updated_at: now,
                    metadata: Vec::new(),
                })
            }
            Some(MemEntry::Dir { mode }) => {
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                Ok(FileInfo {
                    name,
                    size: 0,
                    mode: *mode,
                    is_dir: true,
                    created_at: now,
                    updated_at: now,
                    metadata: Vec::new(),
                })
            }
            None => Err(RustVikingError::NotFound(path)),
        }
    }

    fn exists(&self, path: &str) -> bool {
        let path = Self::normalize_path(path);
        self.entries
            .read()
            .map(|e| e.contains_key(&path))
            .unwrap_or(false)
    }

    fn open_read(&self, path: &str) -> Result<Box<dyn Read + Send>> {
        let data = self.read(path, 0, 0)?;
        Ok(Box::new(Cursor::new(data)))
    }

    fn open_write(&self, _path: &str, _flags: WriteFlag) -> Result<Box<dyn Write + Send>> {
        // For memory plugin, write returns a cursor that doesn't persist
        // Real usage should use the write() method directly
        Ok(Box::new(Cursor::new(Vec::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_read() {
        let mem = MemoryPlugin::new();
        mem.write("/test.txt", b"hello", 0, WriteFlag::CREATE)
            .unwrap();
        let data = mem.read("/test.txt", 0, 0).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_mkdir_and_readdir() {
        let mem = MemoryPlugin::new();
        mem.mkdir("/docs", 0o755).unwrap();
        mem.write("/docs/a.txt", b"a", 0, WriteFlag::CREATE)
            .unwrap();
        mem.write("/docs/b.txt", b"b", 0, WriteFlag::CREATE)
            .unwrap();

        let entries = mem.read_dir("/docs").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_remove_all() {
        let mem = MemoryPlugin::new();
        mem.mkdir("/dir", 0o755).unwrap();
        mem.write("/dir/file1.txt", b"1", 0, WriteFlag::CREATE)
            .unwrap();
        mem.write("/dir/file2.txt", b"2", 0, WriteFlag::CREATE)
            .unwrap();

        mem.remove_all("/dir").unwrap();
        assert!(!mem.exists("/dir"));
        assert!(!mem.exists("/dir/file1.txt"));
    }

    #[test]
    fn test_stat() {
        let mem = MemoryPlugin::new();
        mem.write("/test.txt", b"hello world", 0, WriteFlag::CREATE)
            .unwrap();
        let info = mem.stat("/test.txt").unwrap();
        assert_eq!(info.size, 11);
        assert!(!info.is_dir);
    }

    #[test]
    fn test_rename() {
        let mem = MemoryPlugin::new();
        mem.write("/old.txt", b"data", 0, WriteFlag::CREATE)
            .unwrap();
        mem.rename("/old.txt", "/new.txt").unwrap();
        assert!(!mem.exists("/old.txt"));
        assert!(mem.exists("/new.txt"));
    }

    #[test]
    fn test_append() {
        let mem = MemoryPlugin::new();
        mem.write("/log.txt", b"line1\n", 0, WriteFlag::CREATE)
            .unwrap();
        mem.write("/log.txt", b"line2\n", 0, WriteFlag::APPEND)
            .unwrap();
        let data = mem.read("/log.txt", 0, 0).unwrap();
        assert_eq!(data, b"line1\nline2\n");
    }

    #[test]
    fn test_not_found() {
        let mem = MemoryPlugin::new();
        assert!(mem.read("/nonexistent", 0, 0).is_err());
        assert!(mem.stat("/nonexistent").is_err());
    }
}
