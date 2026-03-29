//! Heuristic Summary Provider Implementation
//!
//! Provides rule-based summary generation without LLM dependency.
//! - L0: Abstract (~100 tokens, ~200-400 chars)
//! - L1: Overview (~2k tokens, ~8000 chars)

use async_trait::async_trait;

use super::SummaryProvider;
use crate::error::Result;

/// Heuristic-based summary provider
///
/// Uses rule-based algorithms to generate summaries:
/// - Markdown heading extraction
/// - First paragraph extraction
/// - Sentence-based extraction
/// - Smart truncation
pub struct HeuristicSummaryProvider {
    /// Maximum characters for L0 abstract
    pub max_abstract_chars: usize,
    /// Maximum characters for L1 overview
    pub max_overview_chars: usize,
}

impl HeuristicSummaryProvider {
    /// Create a new provider with default limits
    pub fn new() -> Self {
        Self {
            max_abstract_chars: 400,
            max_overview_chars: 8000,
        }
    }

    /// Create a new provider with custom limits
    pub fn with_limits(max_abstract_chars: usize, max_overview_chars: usize) -> Self {
        Self {
            max_abstract_chars,
            max_overview_chars,
        }
    }

    /// Extract markdown headings from text
    ///
    /// Returns a formatted string with all headings, or None if no headings found.
    fn extract_markdown_headings(&self, text: &str) -> Option<String> {
        let mut headings = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            // Match markdown headings: # Heading, ## Heading, etc.
            if let Some(stripped) = trimmed.strip_prefix("# ") {
                headings.push(format!("- {}", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                headings.push(format!("  - {}", stripped));
            } else if let Some(stripped) = trimmed.strip_prefix("### ") {
                headings.push(format!("    - {}", stripped));
            }
        }

        if headings.is_empty() {
            None
        } else {
            Some(headings.join("\n"))
        }
    }

    /// Extract the first non-empty paragraph from text
    fn extract_first_paragraph(&self, text: &str) -> String {
        let mut in_code_block = false;

        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();

            // Skip empty paragraphs
            if trimmed.is_empty() {
                continue;
            }

            // Track code blocks
            if trimmed.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }

            // Skip code blocks and frontmatter
            if in_code_block || trimmed.starts_with("---") || trimmed.starts_with("#") {
                continue;
            }

            // Clean up the paragraph: remove newlines within paragraph
            let cleaned = trimmed.replace('\n', " ");
            return cleaned;
        }

        // Fallback: return first non-empty line
        text.lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .unwrap_or("")
            .to_string()
    }

    /// Extract sentences until character limit is reached
    fn extract_sentences(&self, text: &str, max_chars: usize) -> String {
        // Simple sentence splitting by punctuation followed by space or end
        let sentence_endings = ['.', '!', '?', '。', '！', '？'];
        let mut result = String::new();
        let mut current_pos = 0;

        while current_pos < text.len() && result.len() < max_chars {
            // Find next sentence ending
            let mut next_end = None;
            for (i, ch) in text[current_pos..].char_indices() {
                if sentence_endings.contains(&ch) {
                    // Check if it's followed by space, newline, or end of string
                    let after_idx = current_pos + i + ch.len_utf8();
                    if after_idx >= text.len() {
                        next_end = Some(after_idx);
                        break;
                    }
                    if let Some(next_ch) = text[after_idx..].chars().next() {
                        if next_ch.is_whitespace() {
                            next_end = Some(after_idx);
                            break;
                        }
                    }
                }
            }

            let sentence_end = next_end.unwrap_or(text.len().min(current_pos + max_chars));
            let sentence = &text[current_pos..sentence_end.min(text.len())];
            let trimmed = sentence.trim();

            if !trimmed.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(trimmed);
            }

            current_pos = sentence_end;

            // Skip whitespace
            while current_pos < text.len() {
                if let Some(ch) = text[current_pos..].chars().next() {
                    if ch.is_whitespace() {
                        current_pos += ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Stop if we've exceeded the limit
            if result.len() >= max_chars {
                break;
            }
        }

        // Smart truncation: try to end at a sentence boundary
        if result.len() > max_chars {
            // Find the last sentence ending before max_chars
            let truncate_at = result[..max_chars]
                .rfind(|c: char| sentence_endings.contains(&c))
                .map(|i| i + 1)
                .unwrap_or(max_chars);
            result.truncate(truncate_at);
        }

        result.trim().to_string()
    }

    /// Smart truncate text at word/sentence boundary
    fn smart_truncate(&self, text: &str, max_chars: usize) -> String {
        if text.len() <= max_chars {
            return text.to_string();
        }

        // Try to find sentence boundary
        let truncate_point = text[..max_chars]
            .rfind(['.', '!', '?', '\n'])
            .map(|i| i + 1)
            .or_else(|| {
                // Try word boundary
                text[..max_chars].rfind(|c: char| c.is_whitespace())
            })
            .unwrap_or(max_chars);

        let mut result = text[..truncate_point].to_string();

        // Add ellipsis if truncated
        if truncate_point < text.len() {
            result.push_str("...");
        }

        result
    }

    /// Infer content type from filename
    /// Note: Currently unused but reserved for future enhancement to include
    /// content type metadata in generated summaries.
    #[allow(dead_code)]
    fn infer_content_type(&self, filename: &str) -> Option<String> {
        let lower = filename.to_lowercase();

        // Common patterns
        if lower.contains("auth") || lower.contains("login") || lower.contains("oauth") {
            Some("认证相关".to_string())
        } else if lower.contains("user") || lower.contains("account") {
            Some("用户管理".to_string())
        } else if lower.contains("api") || lower.contains("endpoint") {
            Some("API接口".to_string())
        } else if lower.contains("config") || lower.contains("setting") {
            Some("配置说明".to_string())
        } else if lower.contains("doc") || lower.contains("readme") {
            Some("文档说明".to_string())
        } else if lower.contains("test") || lower.contains("spec") {
            Some("测试相关".to_string())
        } else if lower.contains("deploy") || lower.contains("docker") || lower.contains("k8s") {
            Some("部署相关".to_string())
        } else if lower.contains("db") || lower.contains("database") || lower.contains("sql") {
            Some("数据库相关".to_string())
        } else if lower.contains("ui") || lower.contains("frontend") || lower.contains("css") {
            Some("前端相关".to_string())
        } else if lower.contains("backend") || lower.contains("server") {
            Some("后端相关".to_string())
        } else {
            None
        }
    }
}

impl Default for HeuristicSummaryProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SummaryProvider for HeuristicSummaryProvider {
    async fn generate_abstract(&self, text: &str) -> Result<String> {
        let text = text.trim();

        if text.is_empty() {
            return Ok("(empty document)".to_string());
        }

        // Strategy 1: Try markdown headings
        if let Some(headings) = self.extract_markdown_headings(text) {
            if headings.len() >= 50 {
                // At least some meaningful content
                return Ok(self.smart_truncate(&headings, self.max_abstract_chars));
            }
        }

        // Strategy 2: Extract first paragraph
        let first_para = self.extract_first_paragraph(text);
        if !first_para.is_empty() && first_para.len() >= 50 {
            if first_para.len() <= self.max_abstract_chars {
                return Ok(first_para);
            }
            return Ok(self.smart_truncate(&first_para, self.max_abstract_chars));
        }

        // Strategy 3: Extract sentences
        let sentences = self.extract_sentences(text, self.max_abstract_chars);
        if !sentences.is_empty() {
            return Ok(sentences);
        }

        // Strategy 4: Smart truncate
        Ok(self.smart_truncate(text, self.max_abstract_chars))
    }

    async fn generate_overview(&self, texts: &[String]) -> Result<String> {
        if texts.is_empty() {
            return Ok("(no content)".to_string());
        }

        let mut overview_parts = Vec::new();
        let mut total_chars = 0;

        for (i, text) in texts.iter().enumerate() {
            let abstract_text = if text.len() > self.max_abstract_chars {
                self.smart_truncate(text, self.max_abstract_chars)
            } else {
                text.clone()
            };

            let part = format!("{}. {}", i + 1, abstract_text);

            // Check if adding this would exceed limit
            if total_chars + part.len() + 2 > self.max_overview_chars && !overview_parts.is_empty()
            {
                break;
            }

            overview_parts.push(part);
            total_chars += abstract_text.len() + 4; // Account for numbering and newlines
        }

        let overview = overview_parts.join("\n\n");

        // Final truncation if needed
        if overview.len() > self.max_overview_chars {
            Ok(self.smart_truncate(&overview, self.max_overview_chars))
        } else {
            Ok(overview)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markdown_headings() {
        let provider = HeuristicSummaryProvider::new();

        let text = r#"# Main Title
Some content here.

## Section 1
More content.

### Subsection 1.1
Details.

## Section 2
Final content.
"#;

        let headings = provider.extract_markdown_headings(text);
        assert!(headings.is_some());
        let headings = headings.unwrap();
        assert!(headings.contains("- Main Title"));
        assert!(headings.contains("  - Section 1"));
        assert!(headings.contains("    - Subsection 1.1"));
        assert!(headings.contains("  - Section 2"));
    }

    #[test]
    fn test_extract_markdown_headings_none() {
        let provider = HeuristicSummaryProvider::new();

        let text = "Just some plain text without any headings.";
        assert!(provider.extract_markdown_headings(text).is_none());
    }

    #[test]
    fn test_extract_first_paragraph() {
        let provider = HeuristicSummaryProvider::new();

        let text = r#"This is the first paragraph.
It continues here.

This is the second paragraph.
"#;

        let para = provider.extract_first_paragraph(text);
        assert_eq!(para, "This is the first paragraph. It continues here.");
    }

    #[test]
    fn test_extract_first_paragraph_skips_frontmatter() {
        let provider = HeuristicSummaryProvider::new();

        let text = r#"---
title: My Doc
---

This is the real first paragraph.
"#;

        let para = provider.extract_first_paragraph(text);
        assert_eq!(para, "This is the real first paragraph.");
    }

    #[test]
    fn test_extract_sentences() {
        let provider = HeuristicSummaryProvider::new();

        let text = "First sentence. Second sentence! Third sentence? Fourth sentence.";
        let extracted = provider.extract_sentences(text, 100);

        assert!(extracted.contains("First sentence"));
        assert!(extracted.contains("Second sentence"));
    }

    #[test]
    fn test_smart_truncate() {
        let provider = HeuristicSummaryProvider::new();

        let text = "This is a very long text that needs to be truncated. It has multiple sentences. And more content here.";
        let truncated = provider.smart_truncate(text, 50);

        assert!(truncated.len() <= 53); // 50 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_smart_truncate_short() {
        let provider = HeuristicSummaryProvider::new();

        let text = "Short text.";
        let truncated = provider.smart_truncate(text, 50);

        assert_eq!(truncated, "Short text.");
    }

    #[test]
    fn test_infer_content_type() {
        let provider = HeuristicSummaryProvider::new();

        assert_eq!(
            provider.infer_content_type("auth.md"),
            Some("认证相关".to_string())
        );
        assert_eq!(
            provider.infer_content_type("user-guide.md"),
            Some("用户管理".to_string())
        );
        assert_eq!(
            provider.infer_content_type("api-reference.md"),
            Some("API接口".to_string())
        );
        assert_eq!(
            provider.infer_content_type("config.yaml"),
            Some("配置说明".to_string())
        );
        assert_eq!(provider.infer_content_type("random.txt"), None);
    }

    #[tokio::test]
    async fn test_generate_abstract_markdown() {
        let provider = HeuristicSummaryProvider::new();

        let text = r#"# Project Documentation

This is the introduction paragraph that explains what this project does.

## Getting Started
Follow these steps.

## API Reference
Check the docs.
"#;

        let abstract_text = provider.generate_abstract(text).await.unwrap();
        assert!(abstract_text.contains("Project Documentation"));
        assert!(abstract_text.contains("Getting Started"));
        assert!(abstract_text.contains("API Reference"));
    }

    #[tokio::test]
    async fn test_generate_abstract_plain_text() {
        let provider = HeuristicSummaryProvider::new();

        let text = "This is the first paragraph of a plain text document. It explains the main concepts. Second paragraph here.";

        let abstract_text = provider.generate_abstract(text).await.unwrap();
        assert!(abstract_text.contains("This is the first paragraph"));
    }

    #[tokio::test]
    async fn test_generate_abstract_empty() {
        let provider = HeuristicSummaryProvider::new();

        assert_eq!(provider.generate_abstract("").await.unwrap(), "(empty document)");
        assert_eq!(
            provider.generate_abstract("   ").await.unwrap(),
            "(empty document)"
        );
    }

    #[tokio::test]
    async fn test_generate_abstract_long_text() {
        let provider = HeuristicSummaryProvider::new();

        let text = "A. ".repeat(500); // Very long text

        let abstract_text = provider.generate_abstract(&text).await.unwrap();
        assert!(abstract_text.len() <= 450); // max_abstract_chars + some buffer for "..."
    }

    #[tokio::test]
    async fn test_generate_overview() {
        let provider = HeuristicSummaryProvider::new();

        let texts = vec![
            "First document abstract here.".to_string(),
            "Second document abstract here.".to_string(),
            "Third document abstract here.".to_string(),
        ];

        let overview = provider.generate_overview(&texts).await.unwrap();
        assert!(overview.contains("1. First document"));
        assert!(overview.contains("2. Second document"));
        assert!(overview.contains("3. Third document"));
    }

    #[tokio::test]
    async fn test_generate_overview_empty() {
        let provider = HeuristicSummaryProvider::new();

        let texts: Vec<String> = vec![];
        assert_eq!(provider.generate_overview(&texts).await.unwrap(), "(no content)");
    }

    #[tokio::test]
    async fn test_generate_overview_respects_limit() {
        let provider = HeuristicSummaryProvider::with_limits(400, 100);

        let texts = vec![
            "This is a very long abstract that should be truncated.".repeat(10),
            "Second abstract here.".to_string(),
        ];

        let overview = provider.generate_overview(&texts).await.unwrap();
        assert!(overview.len() <= 150); // 100 + buffer for "..."
    }

    #[test]
    fn test_with_limits() {
        let provider = HeuristicSummaryProvider::with_limits(100, 500);

        assert_eq!(provider.max_abstract_chars, 100);
        assert_eq!(provider.max_overview_chars, 500);
    }
}
