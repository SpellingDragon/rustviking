//! AGFS Integration Tests
//!
//! Tests AGFS filesystem with MemoryPlugin.

use rustviking::agfs::{MountableFS, WriteFlag};
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
