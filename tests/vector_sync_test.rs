//! Vector Sync Manager Integration Tests
//!
//! End-to-end tests for VectorSyncManager using MemoryVectorStore and MockEmbeddingProvider.
//! Tests complete file lifecycle: created -> updated -> moved -> deleted.

use std::sync::Arc;

use rustviking::embedding::mock::MockEmbeddingProvider;
use rustviking::embedding::traits::EmbeddingProvider;
use rustviking::embedding::types::EmbeddingConfig;
use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::sync::VectorSyncManager;
use rustviking::vector_store::traits::VectorStore;
use rustviking::vector_store::types::{Filter, IndexParams};
use serde_json::json;

// Helper to create a configured VectorSyncManager
fn create_test_manager() -> VectorSyncManager {
    let store = Arc::new(MemoryVectorStore::new());
    let provider = Arc::new(MockEmbeddingProvider::new(128));

    // Initialize provider
    provider
        .initialize(EmbeddingConfig {
            api_base: String::new(),
            api_key: None,
            provider: "mock".to_string(),
            model: "mock".to_string(),
            dimension: 128,
            max_concurrent: 1,
        })
        .unwrap();

    // Create collection
    store
        .create_collection("test", 128, IndexParams::default())
        .unwrap();

    VectorSyncManager::new(store, provider, "test".to_string())
}

// ============================================================================
// File Lifecycle Tests
// ============================================================================

#[test]
fn test_file_created_and_search() {
    let manager = create_test_manager();

    // Create a file
    manager
        .on_file_created(
            "/docs/readme",
            Some("/docs"),
            "This is a readme file with documentation",
            "resource",
            Some("README"),
            Some("Documentation file"),
        )
        .unwrap();

    // Search for similar content
    let results = manager.search("documentation", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.uri, "/docs/readme");
    assert_eq!(results[0].metadata.context_type, "resource");
    assert_eq!(results[0].metadata.name, Some("README".to_string()));
    assert_eq!(
        results[0].metadata.description,
        Some("Documentation file".to_string())
    );
}

#[test]
fn test_file_created_with_all_metadata() {
    let manager = create_test_manager();

    manager
        .on_file_created(
            "/skills/my-skill",
            Some("/skills"),
            "This is a skill implementation",
            "skill",
            Some("My Skill"),
            Some("A useful skill"),
        )
        .unwrap();

    let results = manager.search("skill implementation", 10, None).unwrap();
    assert_eq!(results.len(), 1);

    let metadata = &results[0].metadata;
    assert_eq!(metadata.uri, "/skills/my-skill");
    assert_eq!(metadata.parent_uri, Some("/skills".to_string()));
    assert_eq!(metadata.context_type, "skill");
    assert_eq!(metadata.name, Some("My Skill".to_string()));
    assert_eq!(metadata.description, Some("A useful skill".to_string()));
    assert!(metadata.is_leaf);
    assert_eq!(metadata.level, 0);
    assert!(!metadata.created_at.is_empty());
}

#[test]
fn test_file_updated() {
    let manager = create_test_manager();

    // Create a file
    manager
        .on_file_created(
            "/file",
            None,
            "old content unique phrase",
            "resource",
            None,
            None,
        )
        .unwrap();

    // Verify file exists
    let results = manager.search("unique phrase", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/file");

    // Update the file
    manager
        .on_file_updated(
            "/file",
            None,
            "new content here xyz123",
            "resource",
            None,
            None,
        )
        .unwrap();

    // Verify file still exists after update
    let results = manager.search("xyz123", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/file");
}

#[test]
fn test_file_updated_preserves_metadata() {
    let manager = create_test_manager();

    // Create a file with metadata
    manager
        .on_file_created(
            "/docs/guide",
            Some("/docs"),
            "Initial guide content",
            "resource",
            Some("User Guide"),
            Some("Guide description"),
        )
        .unwrap();

    // Update with new content but same metadata
    manager
        .on_file_updated(
            "/docs/guide",
            Some("/docs"),
            "Updated guide content with more details",
            "resource",
            Some("User Guide"),
            Some("Guide description"),
        )
        .unwrap();

    // Verify metadata is preserved
    let results = manager.search("guide", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, Some("User Guide".to_string()));
    assert_eq!(
        results[0].metadata.description,
        Some("Guide description".to_string())
    );
}

#[test]
fn test_file_moved() {
    let manager = create_test_manager();

    // Create a file
    manager
        .on_file_created(
            "/old/path/file",
            Some("/old/path"),
            "test content",
            "memory",
            Some("test file"),
            None,
        )
        .unwrap();

    // Move the file
    manager.on_file_moved("/old/path", "/new/path").unwrap();

    // Verify the URI was updated
    let results = manager.search("test content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.uri, "/new/path/file");
    assert_eq!(
        results[0].metadata.parent_uri,
        Some("/new/path".to_string())
    );
}

#[test]
fn test_file_moved_nested() {
    let manager = create_test_manager();

    // Create multiple files in nested structure
    manager
        .on_file_created(
            "/a/b/c/file1",
            Some("/a/b/c"),
            "content 1",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created(
            "/a/b/file2",
            Some("/a/b"),
            "content 2",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created("/a/file3", Some("/a"), "content 3", "resource", None, None)
        .unwrap();
    manager
        .on_file_created("/x/file4", Some("/x"), "content 4", "resource", None, None)
        .unwrap();

    // Move /a to /new-a
    manager.on_file_moved("/a", "/new-a").unwrap();

    // Verify all files under /a were moved
    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 4);

    let uris: Vec<&str> = results.iter().map(|r| r.metadata.uri.as_str()).collect();
    assert!(uris.contains(&"/new-a/b/c/file1"));
    assert!(uris.contains(&"/new-a/b/file2"));
    assert!(uris.contains(&"/new-a/file3"));
    assert!(uris.contains(&"/x/file4")); // unchanged
}

#[test]
fn test_file_deleted() {
    let manager = create_test_manager();

    // Create files
    manager
        .on_file_created("/docs/file1", None, "content 1", "resource", None, None)
        .unwrap();
    manager
        .on_file_created("/docs/file2", None, "content 2", "resource", None, None)
        .unwrap();
    manager
        .on_file_created("/other/file3", None, "content 3", "resource", None, None)
        .unwrap();

    // Delete by URI prefix
    manager.on_file_deleted("/docs").unwrap();

    // Verify deletion
    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.uri, "/other/file3");
}

#[test]
fn test_complete_file_lifecycle() {
    let manager = create_test_manager();

    // 1. Create
    manager
        .on_file_created(
            "/project/readme.md",
            Some("/project"),
            "Initial readme content abc789",
            "resource",
            Some("README"),
            Some("Project readme"),
        )
        .unwrap();

    let results = manager.search("abc789", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/project/readme.md");

    // 2. Update
    manager
        .on_file_updated(
            "/project/readme.md",
            Some("/project"),
            "Updated readme with more information def456",
            "resource",
            Some("README"),
            Some("Project readme"),
        )
        .unwrap();

    // File should still exist after update
    let results = manager.search("def456", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/project/readme.md");

    // 3. Move
    manager
        .on_file_moved("/project", "/archive/project")
        .unwrap();

    let results = manager.search("def456", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/archive/project/readme.md");

    // 4. Delete
    manager.on_file_deleted("/archive").unwrap();

    let results = manager.search("def456", 10, None).unwrap();
    assert!(results.is_empty());
}

// ============================================================================
// Search Tests
// ============================================================================

#[test]
fn test_search_basic() {
    let manager = create_test_manager();

    manager
        .on_file_created(
            "/doc1",
            None,
            "Rust programming language abc123",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created(
            "/doc2",
            None,
            "Python programming language def456",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created(
            "/doc3",
            None,
            "Cooking recipes xyz789",
            "resource",
            None,
            None,
        )
        .unwrap();

    // All 3 files should be searchable
    let results = manager.search("abc123", 10, None).unwrap();
    assert!(!results.is_empty());

    let results = manager.search("def456", 10, None).unwrap();
    assert!(!results.is_empty());

    let results = manager.search("xyz789", 10, None).unwrap();
    assert!(!results.is_empty());

    // Total count should be 3
    let results = manager.search("programming", 10, None).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_search_with_limit() {
    let manager = create_test_manager();

    // Create 5 files
    for i in 0..5 {
        manager
            .on_file_created(
                &format!("/doc{}", i),
                None,
                &format!("Document number {}", i),
                "resource",
                None,
                None,
            )
            .unwrap();
    }

    let results = manager.search("Document", 3, None).unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_search_with_filter() {
    let manager = create_test_manager();

    // Create files with different context types
    manager
        .on_file_created(
            "/resource/1",
            None,
            "resource content",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created("/memory/1", None, "memory content", "memory", None, None)
        .unwrap();
    manager
        .on_file_created("/skill/1", None, "skill content", "skill", None, None)
        .unwrap();

    // Search with filter for memory type
    let filter = Filter::Eq("context_type".to_string(), json!("memory"));
    let results = manager.search("content", 10, Some(filter)).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.context_type, "memory");
}

#[test]
fn test_search_with_in_filter() {
    let manager = create_test_manager();

    // Create files with different context types
    manager
        .on_file_created(
            "/resource/1",
            None,
            "resource content",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created("/memory/1", None, "memory content", "memory", None, None)
        .unwrap();
    manager
        .on_file_created("/skill/1", None, "skill content", "skill", None, None)
        .unwrap();
    manager
        .on_file_created(
            "/resource/2",
            None,
            "another resource",
            "resource",
            None,
            None,
        )
        .unwrap();

    // Search with filter for multiple types
    let filter = Filter::In(
        "context_type".to_string(),
        vec![json!("resource"), json!("skill")],
    );
    let results = manager.search("content", 10, Some(filter)).unwrap();

    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(
            result.metadata.context_type == "resource" || result.metadata.context_type == "skill"
        );
    }
}

#[test]
fn test_search_empty_query() {
    let manager = create_test_manager();

    manager
        .on_file_created("/doc1", None, "some content", "resource", None, None)
        .unwrap();

    // Empty query should still return results (though relevance may vary)
    let results = manager.search("", 10, None).unwrap();
    // Mock embedding provider generates deterministic vectors, so we should get results
    assert!(!results.is_empty());
}

#[test]
fn test_search_no_results() {
    let manager = create_test_manager();

    manager
        .on_file_created("/doc1", None, "apples and oranges", "resource", None, None)
        .unwrap();

    // Search for something completely different
    let results = manager.search("quantum physics", 10, None).unwrap();
    // We may still get results due to mock embedding nature, but they won't be relevant
    // Just verify the search doesn't error
    assert!(results.len() <= 1);
}

// ============================================================================
// Multiple Files Tests
// ============================================================================

#[test]
fn test_multiple_files_same_collection() {
    let manager = create_test_manager();

    // Create multiple files
    for i in 0..10 {
        manager
            .on_file_created(
                &format!("/docs/file{}", i),
                Some("/docs"),
                &format!("Content of document number {}", i),
                "resource",
                Some(&format!("File {}", i)),
                None,
            )
            .unwrap();
    }

    let results = manager.search("document", 10, None).unwrap();
    assert_eq!(results.len(), 10);
}

#[test]
fn test_delete_partial_prefix() {
    let manager = create_test_manager();

    // Create files in different directories
    manager
        .on_file_created(
            "/docs/guide/intro",
            None,
            "intro content",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created(
            "/docs/guide/advanced",
            None,
            "advanced content",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created(
            "/docs/api/reference",
            None,
            "api content",
            "resource",
            None,
            None,
        )
        .unwrap();
    manager
        .on_file_created("/other/file", None, "other content", "resource", None, None)
        .unwrap();

    // Delete only /docs/guide prefix
    manager.on_file_deleted("/docs/guide").unwrap();

    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 2);

    let uris: Vec<&str> = results.iter().map(|r| r.metadata.uri.as_str()).collect();
    assert!(uris.contains(&"/docs/api/reference"));
    assert!(uris.contains(&"/other/file"));
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[test]
fn test_metadata_fields_populated() {
    let manager = create_test_manager();

    manager
        .on_file_created(
            "/test/path",
            Some("/test"),
            "content for metadata test",
            "skill",
            Some("Test Skill"),
            Some("A test skill description"),
        )
        .unwrap();

    let results = manager.search("metadata", 10, None).unwrap();
    assert_eq!(results.len(), 1);

    let metadata = &results[0].metadata;
    assert_eq!(metadata.uri, "/test/path");
    assert_eq!(metadata.parent_uri, Some("/test".to_string()));
    assert_eq!(metadata.context_type, "skill");
    assert!(metadata.is_leaf);
    assert_eq!(metadata.level, 0);
    assert_eq!(metadata.name, Some("Test Skill".to_string()));
    assert_eq!(
        metadata.description,
        Some("A test skill description".to_string())
    );
    assert!(!metadata.created_at.is_empty());
}

#[test]
fn test_abstract_text_truncation() {
    let manager = create_test_manager();

    // Create file with very long content
    let long_content = "a".repeat(1000);
    manager
        .on_file_created("/long/file", None, &long_content, "resource", None, None)
        .unwrap();

    let results = manager.search("a", 10, None).unwrap();
    assert_eq!(results.len(), 1);

    // Abstract should be truncated
    let abstract_text = results[0].metadata.abstract_text.as_ref().unwrap();
    assert!(abstract_text.len() < 300); // Should be truncated with "..."
    assert!(abstract_text.ends_with("..."));
}

#[test]
fn test_optional_metadata_fields() {
    let manager = create_test_manager();

    // Create file without optional fields
    manager
        .on_file_created(
            "/minimal/file",
            None,
            "minimal content",
            "resource",
            None,
            None,
        )
        .unwrap();

    let results = manager.search("minimal", 10, None).unwrap();
    assert_eq!(results.len(), 1);

    let metadata = &results[0].metadata;
    assert_eq!(metadata.name, None);
    assert_eq!(metadata.description, None);
}

// ============================================================================
// Collection Name Tests
// ============================================================================

#[test]
fn test_collection_name() {
    let manager = create_test_manager();
    assert_eq!(manager.collection(), "test");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_create_file_with_empty_content() {
    let manager = create_test_manager();

    manager
        .on_file_created("/empty/file", None, "", "resource", None, None)
        .unwrap();

    // Should still create a vector (with mock embedding)
    let results = manager.search("anything", 10, None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_update_nonexistent_file() {
    let manager = create_test_manager();

    // Updating a file that doesn't exist should still work (delete is no-op, then create)
    manager
        .on_file_updated(
            "/nonexistent/file",
            None,
            "new content",
            "resource",
            None,
            None,
        )
        .unwrap();

    let results = manager.search("new content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_delete_nonexistent_prefix() {
    let manager = create_test_manager();

    // Create one file
    manager
        .on_file_created("/real/file", None, "content", "resource", None, None)
        .unwrap();

    // Delete non-existent prefix should succeed
    manager.on_file_deleted("/nonexistent").unwrap();

    // Original file should still exist
    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_move_with_no_matching_uris() {
    let manager = create_test_manager();

    // Create file
    manager
        .on_file_created("/path/file", None, "content", "resource", None, None)
        .unwrap();

    // Move with non-matching prefix should succeed but do nothing
    manager.on_file_moved("/nonexistent", "/new").unwrap();

    // File should remain unchanged
    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.uri, "/path/file");
}

#[test]
fn test_unicode_content() {
    let manager = create_test_manager();

    manager
        .on_file_created(
            "/unicode/file",
            None,
            "Hello 世界! Привет мир! 🌍",
            "resource",
            Some("Unicode Test"),
            None,
        )
        .unwrap();

    let results = manager.search("world", 10, None).unwrap();
    // Should handle unicode content without errors
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.name, Some("Unicode Test".to_string()));
}

#[test]
fn test_special_characters_in_uri() {
    let manager = create_test_manager();

    manager
        .on_file_created(
            "/path/with spaces/file-name_v1.2.txt",
            Some("/path/with spaces"),
            "content",
            "resource",
            None,
            None,
        )
        .unwrap();

    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].metadata.uri,
        "/path/with spaces/file-name_v1.2.txt"
    );
}

#[test]
fn test_consecutive_updates() {
    let manager = create_test_manager();

    // Create
    manager
        .on_file_created("/file", None, "version 1 xyz111", "resource", None, None)
        .unwrap();

    // Update multiple times
    manager
        .on_file_updated("/file", None, "version 2 xyz222", "resource", None, None)
        .unwrap();
    manager
        .on_file_updated("/file", None, "version 3 xyz333", "resource", None, None)
        .unwrap();
    manager
        .on_file_updated("/file", None, "version 4 xyz444", "resource", None, None)
        .unwrap();

    // File should exist after all updates
    let results = manager.search("xyz444", 10, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.uri, "/file");

    // Only one file should exist (updates should replace, not add)
    let results = manager.search("version", 10, None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_move_after_update() {
    let manager = create_test_manager();

    // Create
    manager
        .on_file_created("/original/file", None, "content", "resource", None, None)
        .unwrap();

    // Update
    manager
        .on_file_updated(
            "/original/file",
            None,
            "updated content",
            "resource",
            None,
            None,
        )
        .unwrap();

    // Move
    manager.on_file_moved("/original", "/moved").unwrap();

    // Verify
    let results = manager.search("updated content", 10, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.uri, "/moved/file");
}

#[test]
fn test_delete_after_move() {
    let manager = create_test_manager();

    // Create
    manager
        .on_file_created("/a/file", None, "content", "resource", None, None)
        .unwrap();

    // Move
    manager.on_file_moved("/a", "/b").unwrap();

    // Delete using new prefix
    manager.on_file_deleted("/b").unwrap();

    // Verify deleted
    let results = manager.search("content", 10, None).unwrap();
    assert_eq!(results.len(), 0);
}
