//! Concurrency Tests
//!
//! Tests for thread safety and concurrent operations.

use std::sync::Arc;
use std::thread;
use tempfile::TempDir;
use rustviking::storage::{KvStore, RocksKvStore};
use rustviking::storage::config::StorageConfig;
use rustviking::index::{IvfPqIndex, IvfPqParams, MetricType, VectorIndex};
use rustviking::plugins::memory::MemoryPlugin;
use rustviking::agfs::{FileSystem, WriteFlag};

// ============================================================================
// KV Store Concurrency Tests
// ============================================================================

fn create_temp_kv_store() -> (Arc<RocksKvStore>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = StorageConfig {
        path: temp_dir.path().to_string_lossy().to_string(),
        create_if_missing: true,
        max_open_files: 10000,
        use_fsync: false,
        block_cache_size: None,
    };
    let store = RocksKvStore::new(&config).expect("Failed to create store");
    (Arc::new(store), temp_dir)
}

#[test]
fn test_kv_concurrent_writes() {
    let (store, _temp_dir) = create_temp_kv_store();
    let num_threads = 4;
    let writes_per_thread = 100;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for i in 0..writes_per_thread {
                    let key = format!("thread_{}_key_{}", thread_id, i);
                    let value = format!("value_{}_{}", thread_id, i);
                    store.put(key.as_bytes(), value.as_bytes()).expect("Put failed");
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    // Verify all keys were written
    let mut found_count = 0;
    for thread_id in 0..num_threads {
        for i in 0..writes_per_thread {
            let key = format!("thread_{}_key_{}", thread_id, i);
            if store.get(key.as_bytes()).unwrap().is_some() {
                found_count += 1;
            }
        }
    }
    assert_eq!(found_count, num_threads * writes_per_thread);
}

#[test]
fn test_kv_concurrent_reads() {
    let (store, _temp_dir) = create_temp_kv_store();
    
    // Pre-populate data
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        store.put(key.as_bytes(), value.as_bytes()).expect("Put failed");
    }
    
    let store = Arc::new(store);
    let num_threads = 8;
    let reads_per_thread = 100;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for i in 0..reads_per_thread {
                    let key = format!("key_{}", i % 1000);
                    let result = store.get(key.as_bytes()).expect("Get failed");
                    assert!(result.is_some());
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_kv_concurrent_read_write() {
    let (store, _temp_dir) = create_temp_kv_store();
    
    // Pre-populate some data
    for i in 0..100 {
        let key = format!("key_{}", i);
        store.put(key.as_bytes(), b"initial").expect("Put failed");
    }
    
    let store = Arc::new(store);
    let num_readers = 4;
    let num_writers = 2;
    
    let handles: Vec<_> = (0..num_readers)
        .map(|_| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for i in 0..100 {
                    let key = format!("key_{}", i % 100);
                    let _ = store.get(key.as_bytes());
                }
            })
        })
        .chain((0..num_writers).map(|thread_id| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for i in 0..50 {
                    let key = format!("new_key_{}_{}", thread_id, i);
                    store.put(key.as_bytes(), b"new_value").expect("Put failed");
                }
            })
        }))
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_kv_concurrent_batch_writes() {
    let (store, _temp_dir) = create_temp_kv_store();
    let num_threads = 4;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for batch_num in 0..10 {
                    let mut batch = store.batch().expect("Batch failed");
                    for i in 0..10 {
                        let key = format!("batch_{}_{}", thread_id, batch_num * 10 + i);
                        batch.put(key.into_bytes(), b"value".to_vec());
                    }
                    batch.commit().expect("Commit failed");
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// ============================================================================
// Vector Index Concurrency Tests
// ============================================================================

fn create_test_index() -> Arc<IvfPqIndex> {
    let params = IvfPqParams {
        num_partitions: 8,
        num_sub_vectors: 4,
        pq_bits: 8,
        metric: MetricType::L2,
    };
    Arc::new(IvfPqIndex::new(params, 8))
}

fn generate_vector(seed: u64, dim: usize) -> Vec<f32> {
    (0..dim).map(|i| ((seed + i as u64) % 100) as f32).collect()
}

#[test]
fn test_vector_concurrent_inserts() {
    let index = create_test_index();
    
    // Train first
    let training: Vec<Vec<f32>> = (0..50).map(|i| generate_vector(i, 8)).collect();
    index.train(&training).expect("Train failed");
    
    let num_threads = 4;
    let inserts_per_thread = 50;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for i in 0..inserts_per_thread {
                    let id = (thread_id * inserts_per_thread + i) as u64;
                    let vector = generate_vector(id, 8);
                    index.insert(id, &vector, 2).expect("Insert failed");
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    // Verify count
    let count = index.count();
    assert_eq!(count, (num_threads * inserts_per_thread) as u64);
}

#[test]
fn test_vector_concurrent_searches() {
    let index = create_test_index();
    
    // Train and populate
    let training: Vec<Vec<f32>> = (0..50).map(|i| generate_vector(i, 8)).collect();
    index.train(&training).expect("Train failed");
    
    for i in 0..500 {
        let vector = generate_vector(i as u64, 8);
        index.insert(i as u64, &vector, 2).expect("Insert failed");
    }
    
    let index = Arc::new(index);
    let num_threads = 8;
    let searches_per_thread = 50;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for i in 0..searches_per_thread {
                    let query = generate_vector(i as u64, 8);
                    let results = index.search(&query, 10, None).expect("Search failed");
                    assert!(!results.is_empty());
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_vector_concurrent_insert_and_search() {
    let index = create_test_index();
    
    // Train first
    let training: Vec<Vec<f32>> = (0..50).map(|i| generate_vector(i, 8)).collect();
    index.train(&training).expect("Train failed");
    
    // Pre-populate some data
    for i in 0..100 {
        let vector = generate_vector(i as u64, 8);
        index.insert(i as u64, &vector, 2).expect("Insert failed");
    }
    
    let index = Arc::new(index);
    let num_inserters = 2;
    let num_searchers = 4;
    
    let handles: Vec<_> = (0..num_inserters)
        .map(|thread_id| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for i in 0..50 {
                    let id = (100 + thread_id * 50 + i) as u64;
                    let vector = generate_vector(id, 8);
                    let _ = index.insert(id, &vector, 2);
                }
            })
        })
        .chain((0..num_searchers).map(|_| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for i in 0..50 {
                    let query = generate_vector(i as u64, 8);
                    let _ = index.search(&query, 5, None);
                }
            })
        }))
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// ============================================================================
// Memory Plugin Concurrency Tests
// ============================================================================

#[test]
fn test_memory_fs_concurrent_writes() {
    let mem = Arc::new(MemoryPlugin::new());
    let num_threads = 4;
    let writes_per_thread = 50;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let mem = Arc::clone(&mem);
            thread::spawn(move || {
                for i in 0..writes_per_thread {
                    let path = format!("/thread_{}/file_{}", thread_id, i);
                    mem.mkdir(&format!("/thread_{}", thread_id), 0o755).expect("Mkdir failed");
                    mem.write(&path, b"test data", 0, WriteFlag::CREATE).expect("Write failed");
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    // Verify files exist
    for thread_id in 0..num_threads {
        for i in 0..writes_per_thread {
            let path = format!("/thread_{}/file_{}", thread_id, i);
            assert!(mem.exists(&path), "File {} should exist", path);
        }
    }
}

#[test]
fn test_memory_fs_concurrent_reads() {
    let mem = Arc::new(MemoryPlugin::new());
    
    // Pre-populate
    for i in 0..100 {
        let path = format!("/file_{}", i);
        mem.write(&path, b"data", 0, WriteFlag::CREATE).expect("Write failed");
    }
    
    let num_threads = 8;
    let reads_per_thread = 100;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let mem = Arc::clone(&mem);
            thread::spawn(move || {
                for i in 0..reads_per_thread {
                    let path = format!("/file_{}", i % 100);
                    let _ = mem.read(&path, 0, 0);
                    let _ = mem.exists(&path);
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_memory_fs_concurrent_mixed_operations() {
    let mem = Arc::new(MemoryPlugin::new());
    
    // Create initial structure
    mem.mkdir("/shared", 0o755).expect("Mkdir failed");
    for i in 0..50 {
        let path = format!("/shared/file_{}", i);
        mem.write(&path, b"initial", 0, WriteFlag::CREATE).expect("Write failed");
    }
    
    let num_readers = 4;
    let num_writers = 2;
    
    let handles: Vec<_> = (0..num_readers)
        .map(|_| {
            let mem = Arc::clone(&mem);
            thread::spawn(move || {
                for i in 0..50 {
                    let path = format!("/shared/file_{}", i % 50);
                    let _ = mem.read(&path, 0, 0);
                    let _ = mem.stat(&path);
                    let _ = mem.size(&path);
                }
            })
        })
        .chain((0..num_writers).map(|thread_id| {
            let mem = Arc::clone(&mem);
            thread::spawn(move || {
                for i in 0..25 {
                    let path = format!("/shared/new_{}_{}", thread_id, i);
                    mem.write(&path, b"new data", 0, WriteFlag::CREATE).expect("Write failed");
                }
            })
        }))
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_kv_stress_high_concurrency() {
    let (store, _temp_dir) = create_temp_kv_store();
    let num_threads = 16;
    let ops_per_thread = 100;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for i in 0..ops_per_thread {
                    let key = format!("stress_key_{}_{}", thread_id, i);
                    // Write
                    store.put(key.as_bytes(), b"stress_value").expect("Put failed");
                    // Read back
                    let _ = store.get(key.as_bytes());
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_vector_stress_many_threads() {
    let index = create_test_index();
    
    // Train
    let training: Vec<Vec<f32>> = (0..100).map(|i| generate_vector(i, 8)).collect();
    index.train(&training).expect("Train failed");
    
    let num_threads = 16;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                // Mix of inserts and searches
                for i in 0..20 {
                    let id = (thread_id * 100 + i) as u64;
                    let vector = generate_vector(id, 8);
                    index.insert(id, &vector, 2).expect("Insert failed");
                    
                    let query = generate_vector(i as u64, 8);
                    let _ = index.search(&query, 5, None);
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
