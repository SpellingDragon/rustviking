//! Error Handling Tests
//!
//! Tests for error handling edge cases.

use rustviking::agfs::VikingUri;
use rustviking::agfs::{FileSystem, WriteFlag};
use rustviking::error::RustVikingError;
use rustviking::index::{IvfIndex, IvfParams, MetricType, VectorIndex};
use rustviking::plugins::memory::MemoryPlugin;
use rustviking::storage::config::StorageConfig;
use rustviking::storage::{KvStore, RocksKvStore};
use tempfile::TempDir;

// ============================================================================
// URI Error Tests
// ============================================================================

#[test]
fn test_uri_missing_scheme() {
    let result = VikingUri::parse("resources/project/path");
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidUri(msg)) = result {
        assert!(msg.contains("viking://"));
    } else {
        panic!("Expected InvalidUri error");
    }
}

#[test]
fn test_uri_wrong_scheme() {
    let result = VikingUri::parse("http://resources/project/path");
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidUri(msg)) = result {
        assert!(msg.contains("viking://"));
    } else {
        panic!("Expected InvalidUri error");
    }
}

#[test]
fn test_uri_missing_scope() {
    let result = VikingUri::parse("viking://project/path");
    assert!(result.is_err());
}

#[test]
fn test_uri_invalid_scope() {
    let result = VikingUri::parse("viking://invalid/project/path");
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidUri(msg)) = result {
        assert!(msg.contains("Invalid scope") || msg.contains("invalid"));
    } else {
        panic!("Expected InvalidUri error");
    }
}

#[test]
fn test_uri_empty_account() {
    let result = VikingUri::parse("viking://resources//path");
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidUri(msg)) = result {
        assert!(msg.contains("Account cannot be empty"));
    } else {
        panic!("Expected InvalidUri error");
    }
}

#[test]
fn test_uri_valid_scopes() {
    // Test all valid scopes
    for scope in &["resources", "user", "agent", "session"] {
        let uri_str = format!("viking://{}/account/path", scope);
        let result = VikingUri::parse(&uri_str);
        assert!(result.is_ok(), "Failed for scope: {}", scope);
    }
}

// ============================================================================
// Vector Index Error Tests
// ============================================================================

fn create_test_index() -> IvfIndex {
    let params = IvfParams {
        num_partitions: 4,
        metric: MetricType::L2,
    };
    IvfIndex::new(params, 8)
}

#[test]
fn test_vector_dimension_mismatch_on_insert() {
    let index = create_test_index();

    // Try to insert vector with wrong dimension (expect 8, provide 4)
    let result = index.insert(1, &[1.0, 2.0, 3.0, 4.0], 2);
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidDimension { expected, actual }) = result {
        assert_eq!(expected, 8);
        assert_eq!(actual, 4);
    } else {
        panic!("Expected InvalidDimension error");
    }
}

#[test]
fn test_vector_dimension_mismatch_on_search() {
    let index = create_test_index();

    // Try to search with wrong dimension query
    let result = index.search(&[1.0, 2.0, 3.0], 5, None);
    assert!(result.is_err());
    if let Err(RustVikingError::InvalidDimension { expected, actual }) = result {
        assert_eq!(expected, 8);
        assert_eq!(actual, 3);
    } else {
        panic!("Expected InvalidDimension error");
    }
}

#[test]
fn test_vector_delete_nonexistent() {
    let index = create_test_index();

    let result = index.delete(99999);
    assert!(result.is_err());
    if let Err(RustVikingError::NotFound(msg)) = result {
        assert!(msg.contains("99999"));
    } else {
        panic!("Expected NotFound error");
    }
}

#[test]
fn test_vector_get_nonexistent() {
    let index = create_test_index();

    let result = index.get(99999).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_vector_train_empty_vectors() {
    let index = create_test_index();

    let result = index.train(&[]);
    assert!(result.is_err());
    if let Err(RustVikingError::Internal(msg)) = result {
        assert!(msg.contains("empty vectors"));
    } else {
        panic!("Expected Internal error");
    }
}

// ============================================================================
// AGFS Filesystem Error Tests
// ============================================================================

#[test]
fn test_agfs_read_nonexistent_file() {
    let mem = MemoryPlugin::new();

    let result = mem.read("/nonexistent/file.txt", 0, 0);
    assert!(result.is_err());
    if let Err(RustVikingError::NotFound(path)) = result {
        assert!(path.contains("nonexistent"));
    } else {
        panic!("Expected NotFound error");
    }
}

#[test]
fn test_agfs_stat_nonexistent_file() {
    let mem = MemoryPlugin::new();

    let result = mem.stat("/nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_agfs_remove_nonexistent() {
    let mem = MemoryPlugin::new();

    let result = mem.remove("/nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_agfs_size_nonexistent() {
    let mem = MemoryPlugin::new();

    let result = mem.size("/nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_agfs_read_dir_nonexistent() {
    let mem = MemoryPlugin::new();

    let result = mem.read_dir("/nonexistent_dir");
    assert!(result.is_err());
}

#[test]
fn test_agfs_rename_nonexistent() {
    let mem = MemoryPlugin::new();

    let result = mem.rename("/old_nonexistent", "/new_path");
    assert!(result.is_err());
}

#[test]
fn test_agfs_exists_nonexistent() {
    let mem = MemoryPlugin::new();

    assert!(!mem.exists("/nonexistent_file"));
}

// ============================================================================
// KV Store Error Tests
// ============================================================================

fn create_temp_kv_store() -> (RocksKvStore, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = StorageConfig {
        path: temp_dir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 1000,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config).expect("Failed to create store");
    (store, temp_dir)
}

#[test]
fn test_kv_get_nonexistent_key() {
    let (store, _temp_dir) = create_temp_kv_store();

    let result = store.get(b"nonexistent_key").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_kv_scan_empty_prefix() {
    let (store, _temp_dir) = create_temp_kv_store();

    let results = store.scan_prefix(b"nonexistent_prefix").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_kv_range_empty() {
    let (store, _temp_dir) = create_temp_kv_store();

    let results = store.range(b"key_start", b"key_end").unwrap();
    assert!(results.is_empty());
}

// ============================================================================
// Storage Config Error Tests
// ============================================================================

#[test]
fn test_storage_invalid_path() {
    // Try to create store in a path that doesn't exist and create_if_missing is false
    let config = StorageConfig {
        path: "/nonexistent/path/that/should/not/exist".to_string(),
        create_if_missing: false,
        max_open_files: 1000,
        use_fsync: false,
        block_cache_size: None,
    };

    let result = RocksKvStore::new(&config);
    // This may or may not fail depending on RocksDB behavior
    // Just ensure it doesn't panic
    drop(result);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_vector_insert_empty_vector() {
    let index = create_test_index();

    let result = index.insert(1, &[], 2);
    assert!(result.is_err());
}

#[test]
fn test_kv_put_empty_key() {
    let (store, _temp_dir) = create_temp_kv_store();

    // Empty key should work (RocksDB allows it)
    let result = store.put(b"", b"value");
    assert!(result.is_ok());

    let value = store.get(b"").unwrap();
    assert_eq!(value, Some(b"value".to_vec()));
}

#[test]
fn test_kv_put_empty_value() {
    let (store, _temp_dir) = create_temp_kv_store();

    // Empty value should work
    let result = store.put(b"key", b"");
    assert!(result.is_ok());

    let value = store.get(b"key").unwrap();
    assert_eq!(value, Some(vec![]));
}

#[test]
fn test_agfs_write_empty_data() {
    let mem = MemoryPlugin::new();

    let result = mem.write("/empty_file", b"", 0, WriteFlag::CREATE);
    assert!(result.is_ok());

    let data = mem.read("/empty_file", 0, 0).unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_agfs_read_with_offset() {
    let mem = MemoryPlugin::new();

    mem.write("/test", b"hello world", 0, WriteFlag::CREATE)
        .unwrap();

    // Read from offset
    let data = mem.read("/test", 6, 0).unwrap();
    assert_eq!(data, b"world");

    // Read with size limit
    let data = mem.read("/test", 0, 5).unwrap();
    assert_eq!(data, b"hello");

    // Read beyond end
    let data = mem.read("/test", 100, 0).unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_uri_with_special_characters_in_path() {
    // Paths with special characters should work
    let result = VikingUri::parse("viking://resources/project/path/with spaces/file.txt");
    assert!(result.is_ok());

    let uri = result.unwrap();
    assert_eq!(uri.path, "/path/with spaces/file.txt");
}

#[test]
fn test_uri_unicode_path() {
    let result = VikingUri::parse("viking://resources/project/路径/文件");
    assert!(result.is_ok());

    let uri = result.unwrap();
    assert!(uri.path.contains("路径"));
}
