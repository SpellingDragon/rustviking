//! FileSystem Trait Definition
//!
//! POSIX-style filesystem interface.

use crate::error::Result;
use std::io::{Read, Write};

/// File information
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub mode: u32,
    pub is_dir: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Vec<(String, String)>,
}

/// File system operation flags
#[derive(Debug, Clone, Copy)]
pub struct WriteFlag(u32);

impl WriteFlag {
    pub const NONE: WriteFlag = WriteFlag(0);
    pub const APPEND: WriteFlag = WriteFlag(1 << 0);
    pub const CREATE: WriteFlag = WriteFlag(1 << 1);
    pub const EXCLUSIVE: WriteFlag = WriteFlag(1 << 2);
    pub const TRUNCATE: WriteFlag = WriteFlag(1 << 3);
    pub const SYNC: WriteFlag = WriteFlag(1 << 4);

    /// Check if a flag bit is set
    pub fn has(&self, flag: WriteFlag) -> bool {
        self.0 & flag.0 != 0
    }
}

impl std::ops::BitOr for WriteFlag {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        WriteFlag(self.0 | rhs.0)
    }
}

/// FileSystem Trait - POSIX-style interface
pub trait FileSystem: Send + Sync {
    // File operations
    fn create(&self, path: &str) -> Result<()>;
    fn remove(&self, path: &str) -> Result<()>;
    fn rename(&self, old_path: &str, new_path: &str) -> Result<()>;

    // Directory operations
    fn mkdir(&self, path: &str, mode: u32) -> Result<()>;
    fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>>;
    fn remove_all(&self, path: &str) -> Result<()>;

    // Content operations
    fn read(&self, path: &str, offset: i64, size: u64) -> Result<Vec<u8>>;
    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<u64>;
    fn size(&self, path: &str) -> Result<u64>;

    // Metadata
    fn stat(&self, path: &str) -> Result<FileInfo>;
    fn exists(&self, path: &str) -> bool;

    // Streaming operations
    fn open_read(&self, path: &str) -> Result<Box<dyn Read + Send>>;
    fn open_write(&self, path: &str, flags: WriteFlag) -> Result<Box<dyn Write + Send>>;
}
