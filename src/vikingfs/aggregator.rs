//! Directory Summary Aggregator
//!
//! Performs bottom-up aggregation of summaries:
//! - Generates L0 abstracts for individual files
//! - Aggregates L0s into L1 overviews for directories

use std::sync::Arc;
use crate::agfs::MountableFS;
use crate::error::Result;
use super::SummaryProvider;

/// Result of a directory aggregation operation
#[derive(Debug)]
pub struct AggregateResult {
    /// Number of abstracts generated
    pub abstracts_generated: usize,
    /// Whether an overview was generated
    pub overview_generated: bool,
    /// Non-fatal errors encountered during aggregation
    pub errors: Vec<String>,
}

impl AggregateResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            abstracts_generated: 0,
            overview_generated: false,
            errors: Vec::new(),
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Increment abstracts count
    pub fn increment_abstracts(&mut self) {
        self.abstracts_generated += 1;
    }

    /// Mark overview as generated
    pub fn set_overview_generated(&mut self) {
        self.overview_generated = true;
    }
}

impl Default for AggregateResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Directory summary aggregator
///
/// Performs bottom-up aggregation:
/// 1. Scans directory for files (non-recursive)
/// 2. Generates L0 abstract for each file
/// 3. Aggregates all L0s into L1 overview
pub struct DirectorySummaryAggregator {
    summary_provider: Arc<dyn SummaryProvider>,
    agfs: Arc<MountableFS>,
}

impl DirectorySummaryAggregator {
    /// Create a new aggregator
    pub fn new(summary_provider: Arc<dyn SummaryProvider>, agfs: Arc<MountableFS>) -> Self {
        Self {
            summary_provider,
            agfs,
        }
    }

    /// Perform aggregation on a directory
    ///
    /// # Arguments
    /// * `dir_path` - Directory path (internal format, e.g., "/resources/project/docs")
    ///
    /// # Returns
    /// * `AggregateResult` - Statistics and any non-fatal errors
    pub fn aggregate(&self, dir_path: &str) -> Result<AggregateResult> {
        let mut result = AggregateResult::new();
        let mut abstracts = Vec::new();

        // List directory contents
        let entries = match self.agfs.route_operation(dir_path, |fs| fs.read_dir(dir_path)) {
            Ok(entries) => entries,
            Err(e) => {
                result.add_error(format!("Failed to read directory '{}': {}", dir_path, e));
                return Ok(result);
            }
        };

        // Process each file
        for entry in entries {
            // Skip directories and special files
            if entry.is_dir {
                continue;
            }

            // Skip already-generated summary files (suffix pattern)
            if entry.name.ends_with(".abstract.md") || entry.name.ends_with(".overview.md") {
                continue;
            }

            // Build file path
            let file_path = if dir_path.ends_with('/') {
                format!("{}{}", dir_path, entry.name)
            } else {
                format!("{}/{}", dir_path, entry.name)
            };

            // Try to read and summarize the file
            match self.process_file(&file_path, &entry.name) {
                Ok(Some(abstract_text)) => {
                    abstracts.push(abstract_text);
                    result.increment_abstracts();
                }
                Ok(None) => {
                    // File skipped (e.g., binary, empty)
                }
                Err(e) => {
                    result.add_error(format!("Failed to process '{}': {}", entry.name, e));
                    // Continue with other files
                }
            }
        }

        // Generate overview if we have abstracts
        if !abstracts.is_empty() {
            match self.generate_overview(dir_path, &abstracts) {
                Ok(_) => {
                    result.set_overview_generated();
                }
                Err(e) => {
                    result.add_error(format!("Failed to generate overview: {}", e));
                }
            }
        }

        Ok(result)
    }

    /// Process a single file: read content and generate abstract
    fn process_file(&self, file_path: &str, file_name: &str) -> Result<Option<String>> {
        // Read file content
        let data = self.agfs.route_operation(file_path, |fs| {
            let size = fs.size(file_path)?;
            // Skip very large files (> 10MB)
            if size > 10 * 1024 * 1024 {
                return Err(crate::error::RustVikingError::Storage(
                    "File too large for summarization".to_string()
                ));
            }
            fs.read(file_path, 0, size)
        })?;

        // Try to convert to text
        let content = match String::from_utf8(data) {
            Ok(s) => s,
            Err(_) => {
                // Binary file, skip
                return Ok(None);
            }
        };

        // Skip empty files
        if content.trim().is_empty() {
            return Ok(None);
        }

        // Generate abstract
        let abstract_text = self.summary_provider.generate_abstract(&content)?;

        // Write .abstract.md file
        let abstract_path = format!("{}.abstract.md", file_path);
        let abstract_content = format!(
            "# Abstract: {}\n\n{}\n\n---\n*Auto-generated abstract for {}*\n",
            file_name,
            abstract_text,
            file_name
        );

        self.agfs.route_operation(&abstract_path, |fs| {
            use crate::agfs::WriteFlag;
            fs.write(&abstract_path, abstract_content.as_bytes(), 0, 
                WriteFlag::CREATE | WriteFlag::TRUNCATE)?;
            Ok(())
        })?;

        // Return the abstract for aggregation
        Ok(Some(abstract_text))
    }

    /// Generate L1 overview from abstracts
    fn generate_overview(&self, dir_path: &str, abstracts: &[String]) -> Result<()> {
        // Generate overview text
        let overview_text = self.summary_provider.generate_overview(abstracts)?;

        // Format as markdown
        let overview_content = format!(
            "# Directory Overview\n\n## Summary\n\n{}\n\n## Files\n\n{}",
            overview_text,
            self.format_file_list(abstracts)
        );

        // Write .overview.md
        let overview_path = if dir_path.ends_with('/') {
            format!("{}.overview.md", dir_path)
        } else {
            format!("{}/.overview.md", dir_path)
        };

        self.agfs.route_operation(&overview_path, |fs| {
            use crate::agfs::WriteFlag;
            fs.write(&overview_path, overview_content.as_bytes(), 0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Format file list for overview
    fn format_file_list(&self, abstracts: &[String]) -> String {
        abstracts
            .iter()
            .enumerate()
            .map(|(i, _)| format!("- File {}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agfs::{FileInfo, FileSystem, WriteFlag};
    use crate::vikingfs::{HeuristicSummaryProvider, NoopSummaryProvider};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::sync::Mutex;

    /// Simple in-memory filesystem for testing
    struct MemoryFS {
        files: Mutex<HashMap<String, Vec<u8>>>,
        dirs: Mutex<HashMap<String, Vec<FileInfo>>>,
    }

    impl MemoryFS {
        fn new() -> Self {
            let mut dirs = HashMap::new();
            dirs.insert("/".to_string(), Vec::new());
            
            Self {
                files: Mutex::new(HashMap::new()),
                dirs: Mutex::new(dirs),
            }
        }

        #[allow(dead_code)]
        fn add_file(&self, path: &str, content: &[u8]) {
            let mut files = self.files.lock().unwrap();
            let mut dirs = self.dirs.lock().unwrap();

            files.insert(path.to_string(), content.to_vec());

            // Add to parent directory listing
            let parent = path.rfind('/').map(|i| &path[..i]).unwrap_or("/");
            let parent = if parent.is_empty() { "/" } else { parent };
            let name = path.rfind('/').map(|i| &path[i + 1..]).unwrap_or(path);

            let entry = FileInfo {
                name: name.to_string(),
                size: content.len() as u64,
                mode: 0o644,
                is_dir: false,
                created_at: 0,
                updated_at: 0,
                metadata: Vec::new(),
            };

            dirs.entry(parent.to_string())
                .or_insert_with(Vec::new)
                .push(entry);
        }
    }

    impl FileSystem for MemoryFS {
        fn create(&self, _path: &str) -> Result<()> {
            Ok(())
        }

        fn remove(&self, path: &str) -> Result<()> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        fn rename(&self, _old_path: &str, _new_path: &str) -> Result<()> {
            Ok(())
        }

        fn mkdir(&self, path: &str, _mode: u32) -> Result<()> {
            self.dirs.lock().unwrap().insert(path.to_string(), Vec::new());
            Ok(())
        }

        fn read_dir(&self, path: &str) -> Result<Vec<FileInfo>> {
            Ok(self.dirs.lock().unwrap().get(path).cloned().unwrap_or_default())
        }

        fn remove_all(&self, path: &str) -> Result<()> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        fn read(&self, path: &str, offset: i64, size: u64) -> Result<Vec<u8>> {
            let files = self.files.lock().unwrap();
            let data = files.get(path).ok_or_else(|| {
                crate::error::RustVikingError::NotFound(path.to_string())
            })?;

            let start = offset as usize;
            let end = (start + size as usize).min(data.len());
            Ok(data[start..end].to_vec())
        }

        fn write(&self, path: &str, data: &[u8], offset: i64, _flags: WriteFlag) -> Result<u64> {
            let mut files = self.files.lock().unwrap();
            let entry = files.entry(path.to_string()).or_insert_with(Vec::new);

            let offset = offset as usize;
            if offset + data.len() > entry.len() {
                entry.resize(offset + data.len(), 0);
            }
            entry[offset..offset + data.len()].copy_from_slice(data);

            // Update directory listing
            let mut dirs = self.dirs.lock().unwrap();
            let parent = path.rfind('/').map(|i| &path[..i]).unwrap_or("/");
            let parent = if parent.is_empty() { "/" } else { parent };
            let name = path.rfind('/').map(|i| &path[i + 1..]).unwrap_or(path);

            let dir_entries = dirs.entry(parent.to_string()).or_insert_with(Vec::new);
            if let Some(existing) = dir_entries.iter_mut().find(|e| e.name == name) {
                existing.size = entry.len() as u64;
            } else {
                dir_entries.push(FileInfo {
                    name: name.to_string(),
                    size: entry.len() as u64,
                    mode: 0o644,
                    is_dir: false,
                    created_at: 0,
                    updated_at: 0,
                    metadata: Vec::new(),
                });
            }

            Ok(data.len() as u64)
        }

        fn size(&self, path: &str) -> Result<u64> {
            let files = self.files.lock().unwrap();
            files.get(path).map(|d| d.len() as u64).ok_or_else(|| {
                crate::error::RustVikingError::NotFound(path.to_string())
            })
        }

        fn stat(&self, path: &str) -> Result<FileInfo> {
            let files = self.files.lock().unwrap();
            let size = files.get(path).map(|d| d.len() as u64).unwrap_or(0);
            
            let is_dir = self.dirs.lock().unwrap().contains_key(path);
            let name = path.rfind('/').map(|i| &path[i + 1..]).unwrap_or(path);

            Ok(FileInfo {
                name: name.to_string(),
                size,
                mode: if is_dir { 0o755 } else { 0o644 },
                is_dir,
                created_at: 0,
                updated_at: 0,
                metadata: Vec::new(),
            })
        }

        fn exists(&self, path: &str) -> bool {
            self.files.lock().unwrap().contains_key(path) ||
            self.dirs.lock().unwrap().contains_key(path)
        }

        fn open_read(&self, _path: &str) -> Result<Box<dyn Read + Send>> {
            unimplemented!()
        }

        fn open_write(&self, _path: &str, _flags: WriteFlag) -> Result<Box<dyn Write + Send>> {
            unimplemented!()
        }
    }

    fn create_test_mountable_fs() -> Arc<MountableFS> {
        let agfs = Arc::new(MountableFS::new());
        let fs = Arc::new(MemoryFS::new());
        agfs.mount("/resources/test", fs, 0).unwrap();
        agfs
    }

    #[test]
    fn test_aggregate_empty_directory() {
        let agfs = create_test_mountable_fs();
        let summary_provider: Arc<dyn SummaryProvider> = Arc::new(NoopSummaryProvider);
        let aggregator = DirectorySummaryAggregator::new(summary_provider, agfs);

        let result = aggregator.aggregate("/resources/test").unwrap();
        
        assert_eq!(result.abstracts_generated, 0);
        assert!(!result.overview_generated);
    }

    #[test]
    fn test_aggregate_with_files() {
        let agfs = create_test_mountable_fs();
        
        // Add test files
        if let Some(fs) = agfs.route("/resources/test") {
            let fs = Arc::clone(&fs);
            fs.write(
                "/resources/test/doc1.md",
                b"# Document 1\n\nThis is the content of document 1.",
                0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE,
            ).unwrap();
            fs.write(
                "/resources/test/doc2.md",
                b"# Document 2\n\nThis is the content of document 2.",
                0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE,
            ).unwrap();
        }

        let summary_provider: Arc<dyn SummaryProvider> = Arc::new(HeuristicSummaryProvider::new());
        let aggregator = DirectorySummaryAggregator::new(summary_provider, agfs.clone());

        let result = aggregator.aggregate("/resources/test").unwrap();
        
        assert_eq!(result.abstracts_generated, 2);
        assert!(result.overview_generated);
        assert!(result.errors.is_empty());

        // Verify .abstract.md files were created
        if let Some(fs) = agfs.route("/resources/test") {
            assert!(fs.exists("/resources/test/doc1.md.abstract.md"));
            assert!(fs.exists("/resources/test/doc2.md.abstract.md"));
        }

        // Verify .overview.md was created
        if let Some(fs) = agfs.route("/resources/test") {
            assert!(fs.exists("/resources/test/.overview.md"));
        }
    }

    #[test]
    fn test_aggregate_skips_special_files() {
        let agfs = create_test_mountable_fs();
        
        // Add test files including special ones
        if let Some(fs) = agfs.route("/resources/test") {
            let fs = Arc::clone(&fs);
            fs.write(
                "/resources/test/doc.md",
                b"# Document\n\nContent here.",
                0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE,
            ).unwrap();
            fs.write(
                "/resources/test/.abstract.md",
                b"Existing abstract",
                0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE,
            ).unwrap();
            fs.write(
                "/resources/test/.overview.md",
                b"Existing overview",
                0,
                WriteFlag::CREATE | WriteFlag::TRUNCATE,
            ).unwrap();
        }

        let summary_provider: Arc<dyn SummaryProvider> = Arc::new(HeuristicSummaryProvider::new());
        let aggregator = DirectorySummaryAggregator::new(summary_provider, agfs);

        let result = aggregator.aggregate("/resources/test").unwrap();
        
        // Should only process doc.md, not the special files
        assert_eq!(result.abstracts_generated, 1);
    }

    #[test]
    fn test_aggregate_result() {
        let mut result = AggregateResult::new();
        
        assert_eq!(result.abstracts_generated, 0);
        assert!(!result.overview_generated);
        assert!(result.errors.is_empty());

        result.increment_abstracts();
        assert_eq!(result.abstracts_generated, 1);

        result.set_overview_generated();
        assert!(result.overview_generated);

        result.add_error("Test error".to_string());
        assert_eq!(result.errors.len(), 1);
    }
}
