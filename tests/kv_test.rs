//! KV Store Integration Tests
//!
//! Tests RocksDB KV store operations.

use rustviking::storage::{KvStore, RocksKvStore, StorageConfig};
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_kvstore() -> (RocksKvStore, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };
    let store = RocksKvStore::new(&config).unwrap();
    (store, temp_dir)
}

// ========================================
// Basic Operations Tests
// ========================================

#[test]
fn test_kv_put_and_get() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    let value = store.get(b"key1").unwrap();

    assert_eq!(value, Some(b"value1".to_vec()));
}

#[test]
fn test_kv_get_nonexistent() {
    let (store, _temp_dir) = create_test_kvstore();

    let value = store.get(b"nonexistent").unwrap();
    assert!(value.is_none());
}

#[test]
fn test_kv_delete() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    assert!(store.get(b"key1").unwrap().is_some());

    store.delete(b"key1").unwrap();
    assert!(store.get(b"key1").unwrap().is_none());
}

#[test]
fn test_kv_delete_nonexistent() {
    let (store, _temp_dir) = create_test_kvstore();

    // Deleting non-existent key should succeed
    store.delete(b"nonexistent").unwrap();
}

#[test]
fn test_kv_overwrite() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    store.put(b"key1", b"value2").unwrap();

    let value = store.get(b"key1").unwrap();
    assert_eq!(value, Some(b"value2".to_vec()));
}

#[test]
fn test_kv_multiple_keys() {
    let (store, _temp_dir) = create_test_kvstore();

    for i in 0..100 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        store.put(key.as_bytes(), value.as_bytes()).unwrap();
    }

    for i in 0..100 {
        let key = format!("key{}", i);
        let expected = format!("value{}", i);
        let value = store.get(key.as_bytes()).unwrap();
        assert_eq!(value, Some(expected.into_bytes()));
    }
}

// ========================================
// Range Operation Tests
// ========================================

#[test]
fn test_kv_range_basic() {
    let (store, _temp_dir) = create_test_kvstore();

    // Insert keys in order
    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();
    store.put(b"c", b"3").unwrap();
    store.put(b"d", b"4").unwrap();
    store.put(b"e", b"5").unwrap();

    // Range [b, d) should return b, c
    let results = store.range(b"b", b"d").unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, b"b");
    assert_eq!(results[1].0, b"c");
}

#[test]
fn test_kv_range_start_equals_end() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();

    // start == end should return empty
    let results = store.range(b"b", b"b").unwrap();
    assert!(results.is_empty(), "start == end should return empty results");
}

#[test]
fn test_kv_range_start_greater_than_end() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();
    store.put(b"c", b"3").unwrap();

    // start > end should return empty (iterator won't find anything)
    let results = store.range(b"c", b"a").unwrap();
    assert!(results.is_empty(), "start > end should return empty results");
}

#[test]
fn test_kv_range_empty_database() {
    let (store, _temp_dir) = create_test_kvstore();

    let results = store.range(b"a", b"z").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_kv_range_no_match() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();

    // Range that doesn't include any keys
    let results = store.range(b"x", b"z").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_kv_range_single_key() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    store.put(b"key2", b"value2").unwrap();

    // Range that captures only one key
    let results = store.range(b"key1", b"key2").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, b"key1");
}

#[test]
fn test_kv_range_full_scan() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"a", b"1").unwrap();
    store.put(b"m", b"13").unwrap();
    store.put(b"z", b"26").unwrap();

    // Full scan from beginning
    let results = store.range(b"a", b"zzz").unwrap();
    assert_eq!(results.len(), 3);
}

// ========================================
// Scan Prefix Tests
// ========================================

#[test]
fn test_kv_scan_prefix_basic() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"user:1", b"alice").unwrap();
    store.put(b"user:2", b"bob").unwrap();
    store.put(b"item:1", b"apple").unwrap();
    store.put(b"item:2", b"banana").unwrap();

    let results = store.scan_prefix(b"user:").unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|(k, _)| k == b"user:1"));
    assert!(results.iter().any(|(k, _)| k == b"user:2"));
}

#[test]
fn test_kv_scan_prefix_empty_results() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    store.put(b"key2", b"value2").unwrap();

    // Non-existent prefix should return empty
    let results = store.scan_prefix(b"nonexistent:").unwrap();
    assert!(results.is_empty(), "Non-existent prefix should return empty array");
}

#[test]
fn test_kv_scan_prefix_empty_database() {
    let (store, _temp_dir) = create_test_kvstore();

    let results = store.scan_prefix(b"any:").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_kv_scan_prefix_partial_match() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"prefix_a", b"1").unwrap();
    store.put(b"prefix_b", b"2").unwrap();
    store.put(b"prefix", b"3").unwrap();  // Just the prefix, no suffix
    store.put(b"other", b"4").unwrap();

    let results = store.scan_prefix(b"prefix").unwrap();
    assert_eq!(results.len(), 3);  // prefix, prefix_a, prefix_b
}

// ========================================
// Batch Operation Tests
// ========================================

#[test]
fn test_kv_batch_write() {
    let (store, _temp_dir) = create_test_kvstore();

    let mut batch = store.batch().unwrap();
    batch.put(b"key1".to_vec(), b"value1".to_vec());
    batch.put(b"key2".to_vec(), b"value2".to_vec());
    batch.put(b"key3".to_vec(), b"value3".to_vec());
    batch.commit().unwrap();

    assert_eq!(store.get(b"key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(store.get(b"key2").unwrap(), Some(b"value2".to_vec()));
    assert_eq!(store.get(b"key3").unwrap(), Some(b"value3".to_vec()));
}

#[test]
fn test_kv_batch_delete() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"key1", b"value1").unwrap();
    store.put(b"key2", b"value2").unwrap();
    store.put(b"key3", b"value3").unwrap();

    let mut batch = store.batch().unwrap();
    batch.delete(b"key1".to_vec());
    batch.delete(b"key2".to_vec());
    batch.commit().unwrap();

    assert!(store.get(b"key1").unwrap().is_none());
    assert!(store.get(b"key2").unwrap().is_none());
    assert_eq!(store.get(b"key3").unwrap(), Some(b"value3".to_vec()));
}

#[test]
fn test_kv_batch_mixed_operations() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"existing", b"old_value").unwrap();

    let mut batch = store.batch().unwrap();
    batch.put(b"new_key".to_vec(), b"new_value".to_vec());
    batch.put(b"existing".to_vec(), b"new_value".to_vec());
    batch.delete(b"to_delete".to_vec());
    batch.commit().unwrap();

    assert_eq!(store.get(b"new_key").unwrap(), Some(b"new_value".to_vec()));
    assert_eq!(store.get(b"existing").unwrap(), Some(b"new_value".to_vec()));
}

#[test]
fn test_kv_batch_large_batch() {
    let (store, _temp_dir) = create_test_kvstore();

    let mut batch = store.batch().unwrap();
    for i in 0..1000 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        batch.put(key.into_bytes(), value.into_bytes());
    }
    batch.commit().unwrap();

    for i in 0..1000 {
        let key = format!("key{}", i);
        let expected = format!("value{}", i);
        let value = store.get(key.as_bytes()).unwrap();
        assert_eq!(value, Some(expected.into_bytes()));
    }
}

// ========================================
// Concurrent Write Tests
// ========================================

#[test]
fn test_kv_concurrent_writes() {
    use std::sync::Barrier;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };
    let store = RocksKvStore::new(&config).unwrap();
    let store = Arc::new(store);
    let barrier = Arc::new(Barrier::new(4));

    let mut handles = vec![];

    for thread_id in 0..4 {
        let store_clone = Arc::clone(&store);
        let barrier_clone = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            barrier_clone.wait();

            for i in 0..100 {
                let key = format!("thread_{}_key_{}", thread_id, i);
                let value = format!("thread_{}_value_{}", thread_id, i);
                store_clone.put(key.as_bytes(), value.as_bytes()).unwrap();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all writes succeeded
    for thread_id in 0..4 {
        for i in 0..100 {
            let key = format!("thread_{}_key_{}", thread_id, i);
            let expected = format!("thread_{}_value_{}", thread_id, i);
            let value = store.get(key.as_bytes()).unwrap();
            assert_eq!(
                value,
                Some(expected.into_bytes()),
                "Key {} should have correct value",
                key
            );
        }
    }
}

#[test]
fn test_kv_concurrent_read_write() {
    use std::sync::Barrier;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        path: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };
    let store = RocksKvStore::new(&config).unwrap();
    let store = Arc::new(store);

    // Pre-populate
    for i in 0..100 {
        let key = format!("key{}", i);
        store.put(key.as_bytes(), b"initial").unwrap();
    }

    let barrier = Arc::new(Barrier::new(2));
    let mut handles = vec![];

    // Writer thread
    {
        let store_clone = Arc::clone(&store);
        let barrier_clone = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            barrier_clone.wait();
            for i in 0..100 {
                let key = format!("key{}", i);
                let value = format!("updated_{}", i);
                store_clone.put(key.as_bytes(), value.as_bytes()).unwrap();
            }
        });
        handles.push(handle);
    }

    // Reader thread
    {
        let store_clone = Arc::clone(&store);
        let barrier_clone = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            barrier_clone.wait();
            for _ in 0..100 {
                // Just read some keys
                for i in 0..10 {
                    let key = format!("key{}", i);
                    let _ = store_clone.get(key.as_bytes());
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// ========================================
// Large Value Tests
// ========================================

#[test]
fn test_kv_large_value_1mb() {
    let (store, _temp_dir) = create_test_kvstore();

    // Create 1MB+ value
    let large_value: Vec<u8> = (0..(1024 * 1024 + 100)).map(|i| (i % 256) as u8).collect();

    store.put(b"large_key", &large_value).unwrap();

    let retrieved = store.get(b"large_key").unwrap().unwrap();
    assert_eq!(retrieved.len(), large_value.len());
    assert_eq!(retrieved, large_value);
}

#[test]
fn test_kv_large_value_multiple() {
    let (store, _temp_dir) = create_test_kvstore();

    // Create multiple large values
    for i in 0..5 {
        let key = format!("large_key_{}", i);
        let value: Vec<u8> = (0..(100 * 1024)).map(|j| ((i + j) % 256) as u8).collect();
        store.put(key.as_bytes(), &value).unwrap();
    }

    // Verify all
    for i in 0..5 {
        let key = format!("large_key_{}", i);
        let expected: Vec<u8> = (0..(100 * 1024)).map(|j| ((i + j) % 256) as u8).collect();
        let retrieved = store.get(key.as_bytes()).unwrap().unwrap();
        assert_eq!(retrieved, expected);
    }
}

#[test]
fn test_kv_overwrite_large_value() {
    let (store, _temp_dir) = create_test_kvstore();

    // Start with small value
    store.put(b"key", b"small").unwrap();

    // Overwrite with large value
    let large_value: Vec<u8> = (0..(512 * 1024)).map(|i| (i % 256) as u8).collect();
    store.put(b"key", &large_value).unwrap();

    let retrieved = store.get(b"key").unwrap().unwrap();
    assert_eq!(retrieved.len(), large_value.len());
}

// ========================================
// Key Encoding Tests
// ========================================

#[test]
fn test_kv_binary_keys() {
    let (store, _temp_dir) = create_test_kvstore();

    // Keys with binary data (non-UTF8)
    let key1 = vec![0x00, 0x01, 0x02, 0x03];
    let key2 = vec![0xFF, 0xFE, 0xFD];
    let key3 = vec![0x00, 0x00, 0x00];  // Null bytes

    store.put(&key1, b"value1").unwrap();
    store.put(&key2, b"value2").unwrap();
    store.put(&key3, b"value3").unwrap();

    assert_eq!(store.get(&key1).unwrap(), Some(b"value1".to_vec()));
    assert_eq!(store.get(&key2).unwrap(), Some(b"value2".to_vec()));
    assert_eq!(store.get(&key3).unwrap(), Some(b"value3".to_vec()));
}

#[test]
fn test_kv_unicode_keys() {
    let (store, _temp_dir) = create_test_kvstore();

    // Unicode keys
    let keys = vec![
        "中文键",
        "日本語キー",
        "한국어 키",
        "emoji 🔑",
        "emoji 🎉",
        "ελληνικά",
        "العربية",
        "русский",
    ];

    for (i, key) in keys.iter().enumerate() {
        let value = format!("value_{}", i);
        store.put(key.as_bytes(), value.as_bytes()).unwrap();
    }

    for (i, key) in keys.iter().enumerate() {
        let expected = format!("value_{}", i);
        let value = store.get(key.as_bytes()).unwrap();
        assert_eq!(value, Some(expected.into_bytes()));
    }
}

#[test]
fn test_kv_special_character_keys() {
    let (store, _temp_dir) = create_test_kvstore();

    // Keys with special characters
    let keys = vec![
        "key:with:colons",
        "key/with/slashes",
        "key with spaces",
        "key\twith\ttabs",
        "key\nwith\nnewlines",
        "key=with=equals",
        "key\"with\"quotes",
        "key'with'quotes",
    ];

    for (i, key) in keys.iter().enumerate() {
        let value = format!("value_{}", i);
        store.put(key.as_bytes(), value.as_bytes()).unwrap();
    }

    for (i, key) in keys.iter().enumerate() {
        let expected = format!("value_{}", i);
        let value = store.get(key.as_bytes()).unwrap();
        assert_eq!(value, Some(expected.into_bytes()));
    }
}

#[test]
fn test_kv_unicode_values() {
    let (store, _temp_dir) = create_test_kvstore();

    let values = vec![
        "中文值",
        "日本語バリュー",
        "한국어 값",
        "emoji 🎊 🎉 🚀",
    ];

    for (i, value) in values.iter().enumerate() {
        let key = format!("key_{}", i);
        store.put(key.as_bytes(), value.as_bytes()).unwrap();
    }

    for (i, expected) in values.iter().enumerate() {
        let key = format!("key_{}", i);
        let value = store.get(key.as_bytes()).unwrap();
        assert_eq!(value, Some(expected.as_bytes().to_vec()));
    }
}

#[test]
fn test_kv_empty_key() {
    let (store, _temp_dir) = create_test_kvstore();

    // Empty key
    store.put(b"", b"empty_key_value").unwrap();
    let value = store.get(b"").unwrap();
    assert_eq!(value, Some(b"empty_key_value".to_vec()));
}

#[test]
fn test_kv_empty_value() {
    let (store, _temp_dir) = create_test_kvstore();

    // Empty value
    store.put(b"empty_value", b"").unwrap();
    let value = store.get(b"empty_value").unwrap();
    assert_eq!(value, Some(vec![]));
}

// ========================================
// Edge Cases Tests
// ========================================

#[test]
fn test_kv_key_ordering() {
    let (store, _temp_dir) = create_test_kvstore();

    // Insert in non-lexicographic order
    store.put(b"z", b"1").unwrap();
    store.put(b"a", b"2").unwrap();
    store.put(b"m", b"3").unwrap();

    // Range should return in sorted order
    let results = store.range(b"a", b"zzz").unwrap();
    assert_eq!(results[0].0, b"a");
    assert_eq!(results[1].0, b"m");
    assert_eq!(results[2].0, b"z");
}

#[test]
fn test_kv_prefix_boundary() {
    let (store, _temp_dir) = create_test_kvstore();

    store.put(b"prefix", b"1").unwrap();
    store.put(b"prefixa", b"2").unwrap();
    store.put(b"prefix:", b"3").unwrap();
    store.put(b"prefix0", b"4").unwrap();

    // All should match prefix
    let results = store.scan_prefix(b"prefix").unwrap();
    assert_eq!(results.len(), 4);
}

#[test]
fn test_kv_lexicographic_ordering() {
    let (store, _temp_dir) = create_test_kvstore();

    // Test lexicographic ordering (important for range scans)
    store.put(b"key1", b"1").unwrap();
    store.put(b"key10", b"10").unwrap();
    store.put(b"key2", b"2").unwrap();
    store.put(b"key20", b"20").unwrap();

    // Lexicographic order: key1, key10, key2, key20
    // Range [key1, key3) includes key1, key10, key2, key20 (key20 < key3 in lexicographic order)
    let results = store.range(b"key1", b"key3").unwrap();
    // key20 comes before key3 in lexicographic order: "key20" < "key3" because 'k'='k', 'e'='e', 'y'='y', '2' < '3'
    assert_eq!(results.len(), 4);  // key1, key10, key2, key20
    assert_eq!(results[0].0, b"key1");
    assert_eq!(results[1].0, b"key10");
    assert_eq!(results[2].0, b"key2");
    assert_eq!(results[3].0, b"key20");
}
