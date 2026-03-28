//! RocksDB KV Store Implementation

use rocksdb::{DB, Options, WriteBatch, WriteOptions};
use std::sync::Arc;

use crate::error::{Result, RustVikingError};
use super::{KvStore, BatchWriter};
use crate::storage::config::StorageConfig;

/// RocksDB-based KV store
pub struct RocksKvStore {
    db: Arc<DB>,
}

impl RocksKvStore {
    pub fn new(config: &StorageConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        opts.set_max_open_files(config.max_open_files);
        opts.set_use_fsync(config.use_fsync);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, &config.path)
            .map_err(|e| RustVikingError::Storage(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }
}

impl KvStore for RocksKvStore {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.db.get(key)
            .map_err(|e| RustVikingError::Storage(e.to_string()))
            .map(|opt| opt.map(|v| v.to_vec()))
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value)
            .map_err(|e| RustVikingError::Storage(e.to_string()))
    }

    fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.delete(key)
            .map_err(|e| RustVikingError::Storage(e.to_string()))
    }

    fn scan_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self.db.prefix_iterator(prefix);

        for item in iter {
            let (k, v) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            // Verify the key actually starts with the prefix
            if k.starts_with(prefix) {
                results.push((k.to_vec(), v.to_vec()));
            }
        }

        Ok(results)
    }

    fn range(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self.db.iterator(
            rocksdb::IteratorMode::From(start, rocksdb::Direction::Forward)
        );

        for item in iter {
            let (k, v) = item.map_err(|e| RustVikingError::Storage(e.to_string()))?;
            // Compare as slices
            let k_slice: &[u8] = &k;
            if k_slice >= end {
                break;
            }
            results.push((k.to_vec(), v.to_vec()));
        }

        Ok(results)
    }

    fn batch(&self) -> Result<Box<dyn BatchWriter>> {
        Ok(Box::new(RocksBatchWriter {
            db: self.db.clone(),
            batch: WriteBatch::default(),
        }))
    }
}

/// RocksDB batch writer
pub struct RocksBatchWriter {
    db: Arc<DB>,
    batch: WriteBatch,
}

impl BatchWriter for RocksBatchWriter {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.batch.put(&key, &value);
    }

    fn delete(&mut self, key: Vec<u8>) {
        self.batch.delete(&key);
    }

    fn commit(self: Box<Self>) -> Result<()> {
        let mut opts = WriteOptions::default();
        opts.set_sync(true);
        
        self.db.write_opt(self.batch, &opts)
            .map_err(|e| RustVikingError::Storage(e.to_string()))
    }
}
