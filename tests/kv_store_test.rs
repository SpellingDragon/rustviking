//! KV Store Integration Tests
//!
//! Tests RocksDB KV storage operations.

use rustviking::storage::{KvStore, RocksKvStore, StorageConfig};
use tempfile::TempDir;

fn create_test_store() -> (RocksKvStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let config = StorageConfig {
        path: dir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 100,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config).unwrap();
    (store, dir)
}

#[test]
fn test_put_and_get() {
    let (store, _dir) = create_test_store();
    store.put(b"key1", b"value1").unwrap();
    let result = store.get(b"key1").unwrap();
    assert_eq!(result, Some(b"value1".to_vec()));
}

#[test]
fn test_get_nonexistent() {
    let (store, _dir) = create_test_store();
    let result = store.get(b"nonexistent").unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_delete() {
    let (store, _dir) = create_test_store();
    store.put(b"key1", b"value1").unwrap();
    store.delete(b"key1").unwrap();
    let result = store.get(b"key1").unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_overwrite() {
    let (store, _dir) = create_test_store();
    store.put(b"key1", b"v1").unwrap();
    store.put(b"key1", b"v2").unwrap();
    let result = store.get(b"key1").unwrap();
    assert_eq!(result, Some(b"v2".to_vec()));
}

#[test]
fn test_scan_prefix() {
    let (store, _dir) = create_test_store();
    store.put(b"user:1:name", b"Alice").unwrap();
    store.put(b"user:1:email", b"alice@test.com").unwrap();
    store.put(b"user:2:name", b"Bob").unwrap();
    store.put(b"other:key", b"value").unwrap();
    
    let results = store.scan_prefix(b"user:1:").unwrap();
    // Should find user:1:name and user:1:email
    assert!(results.len() >= 2);
    // Verify all results start with the prefix
    for (key, _) in &results {
        assert!(key.starts_with(b"user:1:"));
    }
}

#[test]
fn test_range_query() {
    let (store, _dir) = create_test_store();
    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();
    store.put(b"c", b"3").unwrap();
    store.put(b"d", b"4").unwrap();
    
    let results = store.range(b"b", b"d").unwrap();
    assert_eq!(results.len(), 2); // b, c
}

#[test]
fn test_batch_write() {
    let (store, _dir) = create_test_store();
    let mut batch = store.batch().unwrap();
    batch.put(b"batch:1".to_vec(), b"v1".to_vec());
    batch.put(b"batch:2".to_vec(), b"v2".to_vec());
    batch.put(b"batch:3".to_vec(), b"v3".to_vec());
    batch.commit().unwrap();
    
    assert_eq!(store.get(b"batch:1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(store.get(b"batch:2").unwrap(), Some(b"v2".to_vec()));
    assert_eq!(store.get(b"batch:3").unwrap(), Some(b"v3".to_vec()));
}
