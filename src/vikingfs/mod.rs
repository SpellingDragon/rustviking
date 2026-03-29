//! VikingFS - Virtual Filesystem Core Unified Abstraction Layer
//!
//! This module provides the core VikingFS abstraction that integrates:
//! - AGFS (Abstract Graph File System) for file operations
//! - Vector Store for semantic search
//! - Embedding Provider for text-to-vector conversion
//! - Summary Provider for L0/L1 context generation

use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::agfs::{setup_agfs, FileInfo, MountableFS, VikingUri};
use crate::config::loader::Config;
use crate::embedding::mock::MockEmbeddingProvider;
use crate::embedding::openai::OpenAIEmbeddingProvider;
use crate::embedding::traits::EmbeddingProvider;
use crate::embedding::types::EmbeddingConfig;
use crate::error::{Result, RustVikingError};
use crate::vector_store::memory::MemoryVectorStore;
use crate::vector_store::qdrant::QdrantVectorStore;
use crate::vector_store::rocks::RocksDBVectorStore;
use crate::vector_store::sync::VectorSyncManager;
use crate::vector_store::traits::VectorStore;
use crate::vector_store::types::{Filter, IndexParams};

pub mod aggregator;
pub mod heuristic_summary;

pub use aggregator::{AggregateResult, DirectorySummaryAggregator};
pub use heuristic_summary::HeuristicSummaryProvider;

/// SummaryProvider trait (predefined for Task 3)
///
/// Provides abstraction capabilities for generating summaries at different levels:
/// - L0: Abstract (~100 tokens)
/// - L1: Overview (~2k tokens)
#[async_trait]
pub trait SummaryProvider: Send + Sync {
    /// Generate abstract summary (~100 tokens)
    async fn generate_abstract(&self, text: &str) -> Result<String>;

    /// Generate overview summary (~2k tokens)
    async fn generate_overview(&self, texts: &[String]) -> Result<String>;
}

/// No-operation summary provider (default implementation)
///
/// Task 3 will replace this with a heuristic implementation.
pub struct NoopSummaryProvider;

#[async_trait]
impl SummaryProvider for NoopSummaryProvider {
    async fn generate_abstract(&self, text: &str) -> Result<String> {
        // Simple truncation: take first 200 characters as abstract
        Ok(text.chars().take(200).collect())
    }

    async fn generate_overview(&self, texts: &[String]) -> Result<String> {
        Ok(texts.join("\n\n"))
    }
}

/// Search result from VikingFS
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// Unique identifier
    pub id: String,
    /// URI of the content
    pub uri: String,
    /// Similarity score
    pub score: f32,
    /// Context level (0=L0, 1=L1, 2=L2)
    pub level: u8,
    /// Abstract text (if available)
    pub abstract_text: Option<String>,
}

/// VikingFS - Virtual Filesystem Core Unified Abstraction Layer
///
/// Provides a unified interface for:
/// - File operations (read/write/delete/move) via AGFS
/// - Semantic search via Vector Store
/// - Context hierarchy (L0/L1/L2) management
pub struct VikingFS {
    /// AGFS mountable filesystem
    agfs: Arc<MountableFS>,
    /// Vector store for semantic search
    vector_store: Arc<dyn VectorStore>,
    /// Vector sync manager for automatic embedding
    vector_sync: Arc<VectorSyncManager>,
    /// Summary provider for L0/L1 generation
    summary_provider: Arc<dyn SummaryProvider>,
    /// Embedding provider for text-to-vector
    /// Note: Currently stored for future use (direct embedding access).
    /// The embedding provider is primarily used through VectorSyncManager.
    #[allow(dead_code)]
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl VikingFS {
    /// Create a new VikingFS instance
    ///
    /// # Arguments
    /// * `agfs` - Mountable filesystem
    /// * `vector_store` - Vector store for semantic search
    /// * `vector_sync` - Vector sync manager
    /// * `summary_provider` - Summary provider for L0/L1
    /// * `embedding_provider` - Embedding provider for text-to-vector
    pub fn new(
        agfs: Arc<MountableFS>,
        vector_store: Arc<dyn VectorStore>,
        vector_sync: Arc<VectorSyncManager>,
        summary_provider: Arc<dyn SummaryProvider>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            agfs,
            vector_store,
            vector_sync,
            summary_provider,
            embedding_provider,
        }
    }

    /// Build a complete VikingFS instance from configuration
    ///
    /// # Arguments
    /// * `config` - Configuration object
    pub async fn from_config(config: &Config) -> Result<Self> {
        // 1. Create AGFS with standard mount points
        let agfs = Arc::new(setup_agfs(&config.storage.path)?);

        // 2. Create vector store based on config
        let dimension = config.vector.as_ref().map(|v| v.dimension).unwrap_or(768);

        let vector_store: Arc<dyn VectorStore> = match config.vector_store.plugin.as_str() {
            "memory" => {
                let store = MemoryVectorStore::new();
                store
                    .create_collection("default", dimension, IndexParams::default())
                    .await?;
                Arc::new(store)
            }
            "rocksdb" => {
                let path = config
                    .vector_store
                    .rocksdb
                    .as_ref()
                    .map(|c| c.path.clone())
                    .unwrap_or_else(|| format!("{}/vector_store", config.storage.path));
                let store = RocksDBVectorStore::with_path(&path)?;
                store
                    .create_collection("default", dimension, IndexParams::default())
                    .await?;
                Arc::new(store)
            }
            "qdrant" => {
                let qdrant_config = config
                    .vector_store
                    .qdrant
                    .as_ref()
                    .ok_or_else(|| RustVikingError::Config("Qdrant config missing".into()))?;
                let store = QdrantVectorStore::new(
                    &qdrant_config.url,
                    qdrant_config.api_key.as_deref(),
                    &qdrant_config.collection,
                    qdrant_config.timeout_ms,
                )
                .await?;
                store
                    .create_collection("default", dimension, IndexParams::default())
                    .await?;
                Arc::new(store) as Arc<dyn VectorStore>
            }
            _ => {
                return Err(RustVikingError::Config(format!(
                    "Unknown vector store plugin: {}",
                    config.vector_store.plugin
                )))
            }
        };

        // 3. Create embedding provider based on config
        let embedding_provider: Arc<dyn EmbeddingProvider> = match config.embedding.plugin.as_str()
        {
            "mock" => {
                let dimension = config
                    .embedding
                    .mock
                    .as_ref()
                    .map(|m| m.dimension)
                    .unwrap_or(1024);
                let provider = MockEmbeddingProvider::new(dimension);

                // Initialize embedding provider
                let embedding_config = EmbeddingConfig {
                    api_base: String::new(),
                    api_key: None,
                    provider: "mock".to_string(),
                    model: "mock".to_string(),
                    dimension,
                    max_concurrent: 10,
                };
                provider.initialize(embedding_config).await?;
                Arc::new(provider)
            }
            "openai" => {
                let provider = OpenAIEmbeddingProvider::new();
                if let Some(openai_config) = &config.embedding.openai {
                    let embedding_config = EmbeddingConfig {
                        api_base: openai_config.api_base.clone(),
                        api_key: Some(openai_config.api_key.clone()),
                        provider: "openai".to_string(),
                        model: openai_config.model.clone(),
                        dimension: openai_config.dimension,
                        max_concurrent: openai_config.max_concurrent,
                    };
                    provider.initialize(embedding_config).await?;
                } else {
                    return Err(RustVikingError::Config(
                        "OpenAI embedding provider requires openai configuration".to_string(),
                    ));
                }
                Arc::new(provider)
            }
            _ => {
                return Err(RustVikingError::Config(format!(
                    "Unknown embedding plugin: {}",
                    config.embedding.plugin
                )))
            }
        };

        // 4. Create vector sync manager
        let vector_sync = Arc::new(VectorSyncManager::new(
            Arc::clone(&vector_store),
            Arc::clone(&embedding_provider),
            "default".to_string(),
        ));

        // 5. Create summary provider (Heuristic by default)
        let summary_provider: Arc<dyn SummaryProvider> = Arc::new(HeuristicSummaryProvider::new());

        Ok(Self::new(
            agfs,
            vector_store,
            vector_sync,
            summary_provider,
            embedding_provider,
        ))
    }

    // =========================================================================
    // File Operations (delegated to AGFS layer)
    // =========================================================================

    /// Read file content
    ///
    /// # Arguments
    /// * `uri` - Viking URI string (e.g., "viking://resources/project/docs/file.md")
    pub fn read(&self, uri: &str) -> Result<Vec<u8>> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs
            .route_operation(&path, |fs| fs.read(&path, 0, fs.size(&path)?))
    }

    /// Write file content
    ///
    /// Triggers vector sync after write.
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    /// * `data` - File content
    pub async fn write(&self, uri: &str, data: &[u8]) -> Result<()> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs.route_operation(&path, |fs| {
            use crate::agfs::WriteFlag;
            fs.write(&path, data, 0, WriteFlag::CREATE | WriteFlag::TRUNCATE)?;
            Ok(())
        })?;

        // Trigger vector sync
        let content = String::from_utf8_lossy(data);
        let parent_uri = viking_uri.parent().map(|p| p.to_uri_string());
        self.vector_sync
            .on_file_created(uri, parent_uri.as_deref(), &content, "resource", None, None)
            .await?;

        Ok(())
    }

    /// Create a directory
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    pub fn mkdir(&self, uri: &str) -> Result<()> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs
            .route_operation(&path, |fs| fs.mkdir(&path, 0o755))
    }

    /// Remove a file or directory
    ///
    /// Triggers vector sync deletion.
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    /// * `recursive` - If true, remove directory and all contents
    pub async fn rm(&self, uri: &str, recursive: bool) -> Result<()> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs.route_operation(&path, |fs| {
            if recursive {
                fs.remove_all(&path)
            } else {
                fs.remove(&path)
            }
        })?;

        // Trigger vector sync deletion
        self.vector_sync.on_file_deleted(uri).await?;

        Ok(())
    }

    /// Move/rename a file or directory
    ///
    /// Triggers vector sync update.
    ///
    /// # Arguments
    /// * `from` - Source Viking URI
    /// * `to` - Destination Viking URI
    pub async fn mv(&self, from: &str, to: &str) -> Result<()> {
        let from_uri = VikingUri::parse(from)?;
        let to_uri = VikingUri::parse(to)?;
        let from_path = from_uri.to_internal_path();
        let to_path = to_uri.to_internal_path();

        // Find the mount point and get the filesystem
        let fs = self
            .agfs
            .route(&from_path)
            .ok_or_else(|| RustVikingError::NotFound(from_path.clone()))?;

        fs.rename(&from_path, &to_path)?;

        // Trigger vector sync update
        self.vector_sync.on_file_moved(from, to).await?;

        Ok(())
    }

    /// List directory contents
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    pub fn ls(&self, uri: &str) -> Result<Vec<FileInfo>> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs.route_operation(&path, |fs| fs.read_dir(&path))
    }

    /// Get file/directory information
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    pub fn stat(&self, uri: &str) -> Result<FileInfo> {
        let viking_uri = VikingUri::parse(uri)?;
        let path = viking_uri.to_internal_path();

        self.agfs.route_operation(&path, |fs| fs.stat(&path))
    }

    // =========================================================================
    // L0/L1/L2 Operations
    // =========================================================================

    /// Read L0 abstract (`.abstract.md`)
    ///
    /// # Arguments
    /// * `uri` - Base URI (without .abstract.md suffix)
    pub fn read_abstract(&self, uri: &str) -> Result<String> {
        let viking_uri = VikingUri::parse(uri)?;
        // Use suffix pattern: file.md -> file.md.abstract.md
        let abstract_path = format!("{}.abstract.md", viking_uri.path);
        let abstract_uri = VikingUri {
            scheme: viking_uri.scheme.clone(),
            scope: viking_uri.scope.clone(),
            account: viking_uri.account.clone(),
            path: abstract_path,
        };
        let path = abstract_uri.to_internal_path();

        let data = self
            .agfs
            .route_operation(&path, |fs| fs.read(&path, 0, fs.size(&path)?))?;

        String::from_utf8(data).map_err(|e| {
            RustVikingError::Serialization(format!("Invalid UTF-8 in abstract: {}", e))
        })
    }

    /// Read L1 overview (`.overview.md`)
    ///
    /// # Arguments
    /// * `uri` - Base URI (directory)
    pub fn read_overview(&self, uri: &str) -> Result<String> {
        let viking_uri = VikingUri::parse(uri)?;
        let overview_uri = viking_uri.join(".overview.md");
        let path = overview_uri.to_internal_path();

        let data = self
            .agfs
            .route_operation(&path, |fs| fs.read(&path, 0, fs.size(&path)?))?;

        String::from_utf8(data).map_err(|e| {
            RustVikingError::Serialization(format!("Invalid UTF-8 in overview: {}", e))
        })
    }

    /// Read L2 detail (equivalent to read)
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    pub fn read_detail(&self, uri: &str) -> Result<Vec<u8>> {
        self.read(uri)
    }

    /// Write context with optional auto-summary generation
    ///
    /// Writes L2 content and optionally generates L0 abstract.
    /// Summary generation errors are logged but don't block the main write.
    ///
    /// # Arguments
    /// * `uri` - Viking URI string
    /// * `data` - File content
    /// * `auto_summary` - If true, generate L0 abstract
    pub async fn write_context(&self, uri: &str, data: &[u8], auto_summary: bool) -> Result<()> {
        // Write L2 content
        self.write(uri, data).await?;

        if auto_summary {
            // Generate L0 abstract
            let content = String::from_utf8_lossy(data);

            match self.summary_provider.generate_abstract(&content).await {
                Ok(abstract_text) => {
                    // Write .abstract.md using suffix pattern: file.md -> file.md.abstract.md
                    let viking_uri = match VikingUri::parse(uri) {
                        Ok(uri) => uri,
                        Err(e) => {
                            eprintln!("[VikingFS] Failed to parse URI for abstract: {}", e);
                            return Ok(());
                        }
                    };
                    let abstract_path = format!("{}.abstract.md", viking_uri.path);
                    let abstract_uri = VikingUri {
                        scheme: viking_uri.scheme.clone(),
                        scope: viking_uri.scope.clone(),
                        account: viking_uri.account.clone(),
                        path: abstract_path,
                    };

                    if let Err(e) = self
                        .write(&abstract_uri.to_uri_string(), abstract_text.as_bytes())
                        .await
                    {
                        eprintln!("[VikingFS] Failed to write abstract: {}", e);
                    }

                    // Sync to vector store if embedding provider is available
                    if let Err(e) = self
                        .vector_sync
                        .on_file_created(
                            &abstract_uri.to_uri_string(),
                            Some(uri),
                            &abstract_text,
                            "abstract",
                            None,
                            Some(&abstract_text),
                        )
                        .await
                    {
                        eprintln!("[VikingFS] Failed to sync abstract to vector store: {}", e);
                    }
                }
                Err(e) => {
                    // Log error but don't block main write
                    eprintln!("[VikingFS] Failed to generate abstract: {}", e);
                }
            }
        }

        Ok(())
    }

    // =========================================================================
    // Search Operations
    // =========================================================================

    /// Search by text query
    ///
    /// Converts text to embedding and performs vector search.
    ///
    /// # Arguments
    /// * `query` - Search query text
    /// * `target_uri` - Optional URI prefix to filter results
    /// * `k` - Number of results to return
    /// * `level` - Optional context level filter (0, 1, or 2)
    pub async fn find(
        &self,
        query: &str,
        target_uri: Option<&str>,
        k: usize,
        level: Option<u8>,
    ) -> Result<Vec<SearchResult>> {
        // Build filter
        let filters = match (target_uri, level) {
            (Some(uri), Some(l)) => Some(Filter::And(vec![
                Filter::In(
                    "uri".to_string(),
                    vec![serde_json::Value::String(uri.to_string())],
                ),
                Filter::Eq("level".to_string(), serde_json::Value::Number(l.into())),
            ])),
            (Some(uri), None) => Some(Filter::In(
                "uri".to_string(),
                vec![serde_json::Value::String(uri.to_string())],
            )),
            (None, Some(l)) => Some(Filter::Eq(
                "level".to_string(),
                serde_json::Value::Number(l.into()),
            )),
            (None, None) => None,
        };

        // Search via vector sync manager
        let results = self.vector_sync.search(query, k, filters).await?;

        // Convert to SearchResult
        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                uri: r.metadata.uri,
                score: r.score,
                level: r.metadata.level,
                abstract_text: r.metadata.abstract_text,
            })
            .collect())
    }

    /// Search by vector directly
    ///
    /// # Arguments
    /// * `collection` - Collection name
    /// * `query_vector` - Query vector
    /// * `k` - Number of results to return
    pub async fn search(
        &self,
        collection: &str,
        query_vector: &[f32],
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        let results = self
            .vector_store
            .search(collection, query_vector, k, None)
            .await?;

        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                uri: r.metadata.uri,
                score: r.score,
                level: r.metadata.level,
                abstract_text: r.metadata.abstract_text,
            })
            .collect())
    }

    // =========================================================================
    // Lifecycle Operations
    // =========================================================================

    /// Commit directory-level summary aggregation
    ///
    /// Traverses directory files and generates/updates L0/L1 summaries.
    /// Uses DirectorySummaryAggregator for bottom-up aggregation.
    ///
    /// # Arguments
    /// * `uri` - Directory URI
    pub async fn commit(&self, uri: &str) -> Result<()> {
        let viking_uri = VikingUri::parse(uri)?;
        let dir_path = viking_uri.to_internal_path();

        // Use DirectorySummaryAggregator for aggregation
        let aggregator = DirectorySummaryAggregator::new(
            Arc::clone(&self.summary_provider),
            Arc::clone(&self.agfs),
        );

        let result = aggregator.aggregate(&dir_path).await?;

        // Log any non-fatal errors
        for error in &result.errors {
            eprintln!("[VikingFS] Aggregation warning: {}", error);
        }

        // Log summary
        println!(
            "[VikingFS] Committed '{}': {} abstracts generated, overview: {}",
            uri,
            result.abstracts_generated,
            if result.overview_generated {
                "yes"
            } else {
                "no"
            }
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_summary_provider() {
        let provider = NoopSummaryProvider;

        let text = "This is a long text that should be truncated to 200 characters for the abstract summary.";
        let abstract_text = provider.generate_abstract(text).await.unwrap();
        assert!(abstract_text.len() <= 200);

        let texts = vec!["Text 1".to_string(), "Text 2".to_string()];
        let overview = provider.generate_overview(&texts).await.unwrap();
        assert_eq!(overview, "Text 1\n\nText 2");
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            id: "test-id".to_string(),
            uri: "viking://resources/project/file.md".to_string(),
            score: 0.95,
            level: 2,
            abstract_text: Some("Test abstract".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("viking://resources/project/file.md"));
    }
}
