//! Vector Sync Manager
//!
//! Automatically synchronizes AGFS file operations with the vector store.
//! When files are created, updated, deleted, or moved in AGFS, the corresponding
//! vector embeddings are automatically managed.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use chrono::Utc;
use serde_json::json;

use crate::embedding::traits::EmbeddingProvider;
use crate::embedding::types::EmbeddingRequest;
use crate::error::Result;
use crate::vector_store::traits::VectorStore;
use crate::vector_store::types::{Filter, VectorPoint, VectorSearchResult};

/// Vector Sync Manager
///
/// Coordinates between AGFS file operations and the vector store,
/// automatically generating embeddings and maintaining vector indices.
pub struct VectorSyncManager {
    vector_store: Arc<dyn VectorStore>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    collection: String,
}

impl VectorSyncManager {
    /// Create a new VectorSyncManager
    ///
    /// # Arguments
    /// * `vector_store` - The vector store to sync with
    /// * `embedding_provider` - The embedding provider for generating embeddings
    /// * `collection` - The collection name to use
    pub fn new(
        vector_store: Arc<dyn VectorStore>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        collection: String,
    ) -> Self {
        Self {
            vector_store,
            embedding_provider,
            collection,
        }
    }

    /// Called when a new file is created in AGFS
    ///
    /// Generates embedding and inserts into vector store.
    ///
    /// # Arguments
    /// * `uri` - The URI of the file
    /// * `parent_uri` - Optional parent URI
    /// * `content` - The file content to embed
    /// * `context_type` - Type of context: "resource", "memory", or "skill"
    /// * `name` - Optional name for the file
    /// * `description` - Optional description
    pub async fn on_file_created(
        &self,
        uri: &str,
        parent_uri: Option<&str>,
        content: &str,
        context_type: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        // 1. Generate embedding using EmbeddingProvider
        let request = EmbeddingRequest {
            texts: vec![content.to_string()],
            model: None,
            normalize: true,
        };
        let result = self.embedding_provider.embed(request).await?;

        // 2. Construct VectorPoint
        let id = generate_id(uri);
        let point = VectorPoint {
            id: id.clone(),
            vector: result.embeddings.into_iter().next().unwrap_or_default(),
            sparse_vector: None,
            payload: json!({
                "id": id,
                "uri": uri,
                "parent_uri": parent_uri,
                "context_type": context_type,
                "is_leaf": true,
                "level": 0,  // L0 by default
                "abstract_text": truncate_text(content, 200),
                "name": name,
                "description": description,
                "created_at": Utc::now().to_rfc3339(),
                "active_count": 0,
            }),
        };

        // 3. Upsert into vector store
        self.vector_store
            .upsert(&self.collection, vec![point])
            .await?;
        Ok(())
    }

    /// Called when a file is deleted from AGFS
    ///
    /// Removes all related vectors by URI prefix.
    ///
    /// # Arguments
    /// * `uri` - The URI of the deleted file
    pub async fn on_file_deleted(&self, uri: &str) -> Result<()> {
        self.vector_store
            .delete_by_uri_prefix(&self.collection, uri)
            .await
    }

    /// Called when a file is moved/renamed in AGFS
    ///
    /// Updates URI references in the vector store.
    ///
    /// # Arguments
    /// * `old_uri` - The old URI of the file
    /// * `new_uri` - The new URI of the file
    pub async fn on_file_moved(&self, old_uri: &str, new_uri: &str) -> Result<()> {
        self.vector_store
            .update_uri(&self.collection, old_uri, new_uri)
            .await
    }

    /// Called when a file is updated in AGFS
    ///
    /// Re-generates embedding and updates the vector.
    /// This is equivalent to delete + create.
    ///
    /// # Arguments
    /// * `uri` - The URI of the file
    /// * `parent_uri` - Optional parent URI
    /// * `content` - The updated file content
    /// * `context_type` - Type of context: "resource", "memory", or "skill"
    /// * `name` - Optional name for the file
    /// * `description` - Optional description
    pub async fn on_file_updated(
        &self,
        uri: &str,
        parent_uri: Option<&str>,
        content: &str,
        context_type: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        // 1. Delete old vectors
        self.on_file_deleted(uri).await?;
        // 2. Insert new vectors
        self.on_file_created(uri, parent_uri, content, context_type, name, description)
            .await
    }

    /// Search for similar content
    ///
    /// Generates embedding for the query and searches the vector store.
    ///
    /// # Arguments
    /// * `query` - The search query text
    /// * `k` - Number of results to return
    /// * `filters` - Optional filters to apply
    pub async fn search(
        &self,
        query: &str,
        k: usize,
        filters: Option<Filter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // 1. Generate query vector
        let request = EmbeddingRequest {
            texts: vec![query.to_string()],
            model: None,
            normalize: true,
        };
        let result = self.embedding_provider.embed(request).await?;
        let query_vector = result.embeddings.into_iter().next().unwrap_or_default();

        // 2. Search in vector store
        self.vector_store
            .search(&self.collection, &query_vector, k, filters)
            .await
    }

    /// Get the collection name
    pub fn collection(&self) -> &str {
        &self.collection
    }
}

/// Generate a deterministic ID from URI
///
/// Uses DefaultHasher to create a consistent hash from the URI.
/// Same URI always produces the same ID.
fn generate_id(uri: &str) -> String {
    let mut hasher = DefaultHasher::new();
    uri.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Truncate text to approximate character count
///
/// Safely handles UTF-8 character boundaries, never truncating
/// in the middle of a multi-byte character.
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::Value;

    use crate::embedding::mock::MockEmbeddingProvider;
    use crate::embedding::types::EmbeddingConfig;
    use crate::vector_store::memory::MemoryVectorStore;
    use crate::vector_store::types::IndexParams;

    async fn create_test_manager() -> VectorSyncManager {
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
            .await
            .unwrap();

        // Create collection
        store
            .create_collection("test", 128, IndexParams::default())
            .await
            .unwrap();

        VectorSyncManager::new(store, provider, "test".to_string())
    }

    #[tokio::test]
    async fn test_file_created_and_search() {
        let manager = create_test_manager().await;

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
            .await
            .unwrap();

        // Search for similar content
        let results = manager.search("documentation", 10, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.uri, "/docs/readme");
        assert_eq!(results[0].metadata.context_type, "resource");
        assert_eq!(results[0].metadata.name, Some("README".to_string()));
    }

    #[tokio::test]
    async fn test_file_deleted() {
        let manager = create_test_manager().await;

        // Create a file
        manager
            .on_file_created("/docs/file1", None, "content 1", "resource", None, None)
            .await
            .unwrap();
        manager
            .on_file_created("/docs/file2", None, "content 2", "resource", None, None)
            .await
            .unwrap();
        manager
            .on_file_created("/other/file3", None, "content 3", "resource", None, None)
            .await
            .unwrap();

        // Delete by URI prefix
        manager.on_file_deleted("/docs").await.unwrap();

        // Verify deletion
        let results = manager.search("content", 10, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.uri, "/other/file3");
    }

    #[tokio::test]
    async fn test_file_moved() {
        let manager = create_test_manager().await;

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
            .await
            .unwrap();

        // Move the file
        manager
            .on_file_moved("/old/path", "/new/path")
            .await
            .unwrap();

        // Verify the URI was updated
        let results = manager.search("test content", 10, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.uri, "/new/path/file");
        assert_eq!(
            results[0].metadata.parent_uri,
            Some("/new/path".to_string())
        );
    }

    #[tokio::test]
    async fn test_file_updated() {
        let manager = create_test_manager().await;

        // Create a file
        manager
            .on_file_created("/file", None, "old content", "resource", None, None)
            .await
            .unwrap();

        // Update the file
        manager
            .on_file_updated("/file", None, "new content here", "resource", None, None)
            .await
            .unwrap();

        // Verify update
        let results = manager.search("new content", 10, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.uri, "/file");
    }

    #[tokio::test]
    async fn test_search_with_filter() {
        let manager = create_test_manager().await;

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
            .await
            .unwrap();
        manager
            .on_file_created("/memory/1", None, "memory content", "memory", None, None)
            .await
            .unwrap();

        // Search with filter
        let filter = Filter::Eq(
            "context_type".to_string(),
            Value::String("memory".to_string()),
        );
        let results = manager.search("content", 10, Some(filter)).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.context_type, "memory");
    }

    #[test]
    fn test_generate_id_deterministic() {
        let id1 = generate_id("/test/uri");
        let id2 = generate_id("/test/uri");
        assert_eq!(id1, id2, "Same URI should produce same ID");

        let id3 = generate_id("/different/uri");
        assert_ne!(id1, id3, "Different URIs should produce different IDs");
    }

    #[test]
    fn test_truncate_text_short() {
        let text = "short text";
        let truncated = truncate_text(text, 100);
        assert_eq!(truncated, text);
    }

    #[test]
    fn test_truncate_text_long() {
        let text = "This is a longer text that needs to be truncated";
        let truncated = truncate_text(text, 10);
        assert_eq!(truncated, "This is a ...");
    }

    #[test]
    fn test_truncate_text_unicode() {
        // Test with multi-byte UTF-8 characters
        let text = "这是一些中文文本，用于测试截断功能";
        let truncated = truncate_text(text, 5);
        assert_eq!(truncated, "这是一些中...");
        assert_eq!(truncated.chars().count(), 8); // 5 chars + "..."
    }

    #[tokio::test]
    async fn test_metadata_fields() {
        let manager = create_test_manager().await;

        manager
            .on_file_created(
                "/test/path",
                Some("/test"),
                "content for metadata test",
                "skill",
                Some("Test Skill"),
                Some("A test skill description"),
            )
            .await
            .unwrap();

        let results = manager.search("metadata", 10, None).await.unwrap();
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

    #[tokio::test]
    async fn test_collection_name() {
        let manager = create_test_manager().await;
        assert_eq!(manager.collection(), "test");
    }
}
