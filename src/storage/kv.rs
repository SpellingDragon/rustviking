//! KV Store Trait
//!
//! Key-value storage interface.

use crate::error::Result;

/// KV Store Trait
pub trait KvStore: Send + Sync {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&self, key: &[u8]) -> Result<()>;
    fn scan_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn range(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
    fn batch(&self) -> Result<Box<dyn BatchWriter>>;
}

/// Batch writer for bulk operations
pub trait BatchWriter: Send {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn delete(&mut self, key: Vec<u8>);
    fn commit(self: Box<Self>) -> Result<()>;
}
