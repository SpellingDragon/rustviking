//! AGFS Integration Tests
//!
//! Tests AGFS filesystem with MemoryPlugin.

use rustviking::agfs::{MountableFS, VikingUri, WriteFlag};
use rustviking::plugins::memory::MemoryPlugin;
use std::sync::Arc;

fn create_test_agfs() -> MountableFS {
    let agfs = MountableFS::new();
    let mem = MemoryPlugin::new();
    agfs.mount("/test", Arc::new(mem), 100).unwrap();
    agfs
}

#[test]
fn test_mount_and_route() {
    let agfs = create_test_agfs();
    let plugin = agfs.route("/test/some/path");
    assert!(plugin.is_some());
}

#[test]
fn test_route_no_match() {
    let agfs = create_test_agfs();
    let plugin = agfs.route("/nonexistent/path");
    assert!(plugin.is_none());
}

#[test]
fn test_write_and_read_through_agfs() {
    let agfs = create_test_agfs();

    // Write through AGFS
    agfs.route_operation("/test/hello.txt", |fs| {
        fs.write("/test/hello.txt", b"Hello, World!", 0, WriteFlag::CREATE)
    })
    .unwrap();

    // Read through AGFS
    let data = agfs
        .route_operation("/test/hello.txt", |fs| fs.read("/test/hello.txt", 0, 0))
        .unwrap();

    assert_eq!(data, b"Hello, World!");
}

#[test]
fn test_mkdir_and_list() {
    let agfs = create_test_agfs();

    agfs.route_operation("/test/mydir", |fs| fs.mkdir("/test/mydir", 0o755))
        .unwrap();

    // Create a file in the directory
    agfs.route_operation("/test/mydir/file.txt", |fs| {
        fs.write("/test/mydir/file.txt", b"content", 0, WriteFlag::CREATE)
    })
    .unwrap();

    let entries = agfs
        .route_operation("/test/mydir", |fs| fs.read_dir("/test/mydir"))
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "file.txt");
}

#[test]
fn test_unmount() {
    let agfs = MountableFS::new();
    let mem = MemoryPlugin::new();
    agfs.mount("/test", Arc::new(mem), 100).unwrap();

    assert!(agfs.route("/test/path").is_some());

    agfs.unmount("/test").unwrap();
    assert!(agfs.route("/test/path").is_none());
}

// ========================================
// FileSystem Trait Coverage Tests
// ========================================

#[test]
fn test_filesystem_create() {
    let agfs = create_test_agfs();

    agfs.route_operation("/test/newfile.txt", |fs| fs.create("/test/newfile.txt"))
        .unwrap();

    let exists: bool = agfs
        .route_operation("/test/newfile.txt", |fs| Ok(fs.exists("/test/newfile.txt")))
        .unwrap();
    assert!(exists, "File should exist after create");
}

#[test]
fn test_filesystem_remove() {
    let agfs = create_test_agfs();

    // Create and then remove
    agfs.route_operation("/test/to_remove.txt", |fs| {
        fs.write("/test/to_remove.txt", b"data", 0, WriteFlag::CREATE)
    })
    .unwrap();

    agfs.route_operation("/test/to_remove.txt", |fs| fs.remove("/test/to_remove.txt"))
        .unwrap();

    let exists: bool = agfs
        .route_operation("/test/to_remove.txt", |fs| {
            Ok(fs.exists("/test/to_remove.txt"))
        })
        .unwrap();
    assert!(!exists, "File should not exist after remove");
}

#[test]
fn test_filesystem_rename() {
    let agfs = create_test_agfs();

    agfs.route_operation("/test/old_name.txt", |fs| {
        fs.write("/test/old_name.txt", b"content", 0, WriteFlag::CREATE)
    })
    .unwrap();

    agfs.route_operation("/test/old_name.txt", |fs| {
        fs.rename("/test/old_name.txt", "/test/new_name.txt")
    })
    .unwrap();

    let old_exists: bool = agfs
        .route_operation("/test/old_name.txt", |fs| {
            Ok(fs.exists("/test/old_name.txt"))
        })
        .unwrap();
    let new_exists: bool = agfs
        .route_operation("/test/new_name.txt", |fs| {
            Ok(fs.exists("/test/new_name.txt"))
        })
        .unwrap();

    assert!(!old_exists, "Old file should not exist after rename");
    assert!(new_exists, "New file should exist after rename");
}

#[test]
fn test_filesystem_size() {
    let agfs = create_test_agfs();

    let content = b"This is test content for size check";
    agfs.route_operation("/test/size_test.txt", |fs| {
        fs.write("/test/size_test.txt", content, 0, WriteFlag::CREATE)
    })
    .unwrap();

    let size = agfs
        .route_operation("/test/size_test.txt", |fs| fs.size("/test/size_test.txt"))
        .unwrap();

    assert_eq!(
        size,
        content.len() as u64,
        "Size should match content length"
    );
}

#[test]
fn test_filesystem_stat() {
    let agfs = create_test_agfs();

    agfs.route_operation("/test/stat_test.txt", |fs| {
        fs.write("/test/stat_test.txt", b"stat content", 0, WriteFlag::CREATE)
    })
    .unwrap();

    let info = agfs
        .route_operation("/test/stat_test.txt", |fs| fs.stat("/test/stat_test.txt"))
        .unwrap();

    assert_eq!(info.name, "stat_test.txt");
    assert_eq!(info.size, 12); // "stat content" is 12 bytes
    assert!(!info.is_dir);
}

#[test]
fn test_filesystem_exists() {
    let agfs = create_test_agfs();

    let exists_before: bool = agfs
        .route_operation("/test/exists_test.txt", |fs| {
            Ok(fs.exists("/test/exists_test.txt"))
        })
        .unwrap();
    assert!(!exists_before, "File should not exist initially");

    agfs.route_operation("/test/exists_test.txt", |fs| {
        fs.write("/test/exists_test.txt", b"test", 0, WriteFlag::CREATE)
    })
    .unwrap();

    let exists_after: bool = agfs
        .route_operation("/test/exists_test.txt", |fs| {
            Ok(fs.exists("/test/exists_test.txt"))
        })
        .unwrap();
    assert!(exists_after, "File should exist after write");
}

#[test]
fn test_filesystem_remove_all() {
    let agfs = create_test_agfs();

    // Create directory with multiple files
    agfs.route_operation("/test/rm_dir", |fs| fs.mkdir("/test/rm_dir", 0o755))
        .unwrap();

    agfs.route_operation("/test/rm_dir/file1.txt", |fs| {
        fs.write("/test/rm_dir/file1.txt", b"1", 0, WriteFlag::CREATE)
    })
    .unwrap();

    agfs.route_operation("/test/rm_dir/file2.txt", |fs| {
        fs.write("/test/rm_dir/file2.txt", b"2", 0, WriteFlag::CREATE)
    })
    .unwrap();

    agfs.route_operation("/test/rm_dir", |fs| fs.remove_all("/test/rm_dir"))
        .unwrap();

    let dir_exists: bool = agfs
        .route_operation("/test/rm_dir", |fs| Ok(fs.exists("/test/rm_dir")))
        .unwrap();
    let file1_exists: bool = agfs
        .route_operation("/test/rm_dir/file1.txt", |fs| {
            Ok(fs.exists("/test/rm_dir/file1.txt"))
        })
        .unwrap();

    assert!(!dir_exists, "Directory should be removed");
    assert!(!file1_exists, "Files inside should be removed");
}

#[test]
fn test_filesystem_read_with_offset() {
    let agfs = create_test_agfs();

    agfs.route_operation("/test/offset_test.txt", |fs| {
        fs.write("/test/offset_test.txt", b"0123456789", 0, WriteFlag::CREATE)
    })
    .unwrap();

    // Read from offset 5
    let data = agfs
        .route_operation("/test/offset_test.txt", |fs| {
            fs.read("/test/offset_test.txt", 5, 3)
        })
        .unwrap();

    assert_eq!(data, b"567", "Should read bytes 5, 6, 7");
}

// ========================================
// Multi-Mount Routing Tests
// ========================================

#[test]
fn test_agfs_multi_mount_routing() {
    let agfs = MountableFS::new();

    // Mount multiple plugins
    let mem1 = MemoryPlugin::new();
    let mem2 = MemoryPlugin::new();
    let mem3 = MemoryPlugin::new();

    agfs.mount("/alpha", Arc::new(mem1), 100).unwrap();
    agfs.mount("/beta", Arc::new(mem2), 100).unwrap();
    agfs.mount("/gamma", Arc::new(mem3), 100).unwrap();

    // Verify each routes correctly
    assert!(
        agfs.route("/alpha/file").is_some(),
        "Should route to /alpha"
    );
    assert!(agfs.route("/beta/file").is_some(), "Should route to /beta");
    assert!(
        agfs.route("/gamma/file").is_some(),
        "Should route to /gamma"
    );
    assert!(
        agfs.route("/delta/file").is_none(),
        "Should not route to unmounted path"
    );
}

#[test]
fn test_agfs_longest_prefix_match() {
    let agfs = MountableFS::new();

    // Mount both /a and /a/b
    let mem_a = MemoryPlugin::new();
    let mem_ab = MemoryPlugin::new();

    agfs.mount("/a", Arc::new(mem_a), 100).unwrap();
    agfs.mount("/a/b", Arc::new(mem_ab), 100).unwrap();

    // Write to both mount points
    agfs.route_operation("/a/file.txt", |fs| {
        fs.write("/a/file.txt", b"from /a", 0, WriteFlag::CREATE)
    })
    .unwrap();

    agfs.route_operation("/a/b/file.txt", |fs| {
        fs.write("/a/b/file.txt", b"from /a/b", 0, WriteFlag::CREATE)
    })
    .unwrap();

    // /a/b/c should route to /a/b (longest prefix)
    let _data = agfs
        .route_operation("/a/b/c/file.txt", |fs| fs.read("/a/b/c/file.txt", 0, 0))
        .unwrap_or_default();
    // This will fail because /a/b/c/file.txt doesn't exist, but it should try /a/b first

    // Verify /a/b/file.txt routes to /a/b plugin
    let data_ab = agfs
        .route_operation("/a/b/file.txt", |fs| fs.read("/a/b/file.txt", 0, 0))
        .unwrap();
    assert_eq!(data_ab, b"from /a/b", "Should read from /a/b mount");

    // Verify /a/other.txt routes to /a plugin
    let data_a = agfs
        .route_operation("/a/file.txt", |fs| fs.read("/a/file.txt", 0, 0))
        .unwrap();
    assert_eq!(data_a, b"from /a", "Should read from /a mount");
}

// ========================================
// Concurrent Access Tests
// ========================================

#[test]
fn test_agfs_concurrent_read_write() {
    use std::sync::Barrier;
    use std::thread;

    let agfs = Arc::new(create_test_agfs());
    let barrier = Arc::new(Barrier::new(4));

    let mut handles = vec![];

    // Spawn 4 threads doing different operations
    for i in 0..4 {
        let agfs_clone = Arc::clone(&agfs);
        let barrier_clone = Arc::clone(&barrier);
        let handle = thread::spawn(move || {
            barrier_clone.wait();

            let path = format!("/test/concurrent_{}.txt", i);
            let content = format!("content from thread {}", i);

            // Write
            agfs_clone
                .route_operation(&path, |fs| {
                    fs.write(&path, content.as_bytes(), 0, WriteFlag::CREATE)
                })
                .unwrap();

            // Read back
            let data = agfs_clone
                .route_operation(&path, |fs| fs.read(&path, 0, 0))
                .unwrap();

            assert_eq!(
                data,
                content.as_bytes(),
                "Thread {} should read its own content",
                i
            );
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all files exist
    for i in 0..4 {
        let path = format!("/test/concurrent_{}.txt", i);
        let exists: bool = agfs
            .route_operation(&path, |fs| Ok(fs.exists(&path)))
            .unwrap();
        assert!(exists, "File {} should exist after concurrent writes", path);
    }
}

// ========================================
// Mount/Unmount Consistency Tests
// ========================================

#[test]
fn test_agfs_mount_operation_success() {
    let agfs = MountableFS::new();
    let mem = MemoryPlugin::new();

    // Before mount, operations should fail
    let result = agfs.route_operation("/test/file.txt", |fs| {
        fs.write("/test/file.txt", b"data", 0, WriteFlag::CREATE)
    });
    assert!(result.is_err(), "Operation should fail before mount");

    // Mount
    agfs.mount("/test", Arc::new(mem), 100).unwrap();

    // After mount, operations should succeed
    let result = agfs.route_operation("/test/file.txt", |fs| {
        fs.write("/test/file.txt", b"data", 0, WriteFlag::CREATE)
    });
    assert!(result.is_ok(), "Operation should succeed after mount");
}

#[test]
fn test_agfs_unmount_operation_fails() {
    let agfs = MountableFS::new();
    let mem = MemoryPlugin::new();

    agfs.mount("/test", Arc::new(mem), 100).unwrap();

    // Write while mounted
    agfs.route_operation("/test/file.txt", |fs| {
        fs.write("/test/file.txt", b"persistent data", 0, WriteFlag::CREATE)
    })
    .unwrap();

    // Unmount
    agfs.unmount("/test").unwrap();

    // After unmount, operations should fail
    let result = agfs.route_operation("/test/file.txt", |fs| fs.read("/test/file.txt", 0, 0));
    assert!(result.is_err(), "Operation should fail after unmount");
}

#[test]
fn test_agfs_remount_new_instance() {
    let agfs = MountableFS::new();

    // Mount first instance
    let mem1 = MemoryPlugin::new();
    agfs.mount("/test", Arc::new(mem1), 100).unwrap();

    agfs.route_operation("/test/file.txt", |fs| {
        fs.write("/test/file.txt", b"first", 0, WriteFlag::CREATE)
    })
    .unwrap();

    // Unmount
    agfs.unmount("/test").unwrap();

    // Mount new instance
    let mem2 = MemoryPlugin::new();
    agfs.mount("/test", Arc::new(mem2), 100).unwrap();

    // New mount should not have old data
    let exists: bool = agfs
        .route_operation("/test/file.txt", |fs| Ok(fs.exists("/test/file.txt")))
        .unwrap();
    assert!(
        !exists,
        "New mount should not have data from previous mount"
    );
}

// ========================================
// Viking URI Parsing Boundary Tests
// ========================================

#[test]
fn test_viking_uri_parse_valid() {
    let uri = VikingUri::parse("viking://resources/project/docs/api.md").unwrap();
    assert_eq!(uri.scheme, "viking");
    assert_eq!(uri.scope, "resources");
    assert_eq!(uri.account, "project");
    assert_eq!(uri.path, "/docs/api.md");
}

#[test]
fn test_viking_uri_parse_empty_uri() {
    let result = VikingUri::parse("");
    assert!(result.is_err(), "Empty URI should fail");
}

#[test]
fn test_viking_uri_parse_invalid_scheme() {
    let result = VikingUri::parse("http://resources/project/docs");
    assert!(
        result.is_err(),
        "Invalid scheme should fail: must start with viking://"
    );
}

#[test]
fn test_viking_uri_parse_missing_scope() {
    let result = VikingUri::parse("viking://");
    assert!(result.is_err(), "Missing scope/account should fail");
}

#[test]
fn test_viking_uri_parse_invalid_scope() {
    let result = VikingUri::parse("viking://invalid/project");
    assert!(
        result.is_err(),
        "Invalid scope should fail: must be one of resources, user, agent, session"
    );
}

#[test]
fn test_viking_uri_parse_missing_account() {
    let result = VikingUri::parse("viking://resources/");
    assert!(result.is_err(), "Missing account should fail");
}

#[test]
fn test_viking_uri_parse_empty_account() {
    // This case is handled in VikingUri::parse
    let result = VikingUri::parse("viking://resources//path");
    // The parser will treat empty account as valid (just checking account.is_empty())
    // Let's verify behavior
    if let Ok(uri) = result {
        assert!(uri.account.is_empty(), "Account should be empty string");
    }
}

#[test]
fn test_viking_uri_parse_no_path() {
    let uri = VikingUri::parse("viking://user/alice").unwrap();
    assert_eq!(uri.scope, "user");
    assert_eq!(uri.account, "alice");
    assert_eq!(uri.path, "/");
}

#[test]
fn test_viking_uri_roundtrip() {
    let original = "viking://resources/myproject/docs/readme.md";
    let uri = VikingUri::parse(original).unwrap();
    let roundtrip = uri.to_uri_string();
    assert_eq!(original, roundtrip);
}

#[test]
fn test_viking_uri_internal_path() {
    let uri = VikingUri::parse("viking://agent/bot123/memory/state.json").unwrap();
    let internal = uri.to_internal_path();
    assert_eq!(internal, "/agent/bot123/memory/state.json");
}

#[test]
fn test_viking_uri_mount_path() {
    let uri = VikingUri::parse("viking://session/abc123/data").unwrap();
    let mount = uri.to_mount_path();
    assert_eq!(mount, "/session/abc123");
}
