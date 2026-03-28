//! File metadata extensions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extended metadata for AGFS files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Content type (e.g., "text/markdown", "application/octet-stream")
    pub content_type: Option<String>,
    /// Content encoding
    pub encoding: Option<String>,
    /// Context level: 0=L0(abstract), 1=L1(overview), 2=L2(detail)
    pub level: Option<u8>,
    /// Custom key-value pairs
    pub custom: HashMap<String, String>,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            content_type: None,
            encoding: None,
            level: None,
            custom: HashMap::new(),
        }
    }

    pub fn with_level(mut self, level: u8) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_content_type(mut self, ct: &str) -> Self {
        self.content_type = Some(ct.to_string());
        self
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.custom.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.custom.get(key).map(|s| s.as_str())
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}
