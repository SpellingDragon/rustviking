//! Viking URI Parser
//!
//! Format: viking://scope/account/path

use crate::error::{Result, RustVikingError};
use serde::{Deserialize, Serialize};

/// Viking URI structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VikingUri {
    pub scheme: String,  // "viking"
    pub scope: String,   // "resources" | "user" | "agent" | "session"
    pub account: String, // account/project identifier
    pub path: String,    // relative path
}

impl VikingUri {
    /// Parse a URI string
    /// Format: viking://scope/account/path
    pub fn parse(uri: &str) -> Result<Self> {
        let uri = uri.trim();

        if !uri.starts_with("viking://") {
            return Err(RustVikingError::InvalidUri(
                "URI must start with viking://".into(),
            ));
        }

        let rest = &uri[9..]; // strip "viking://"
        let parts: Vec<&str> = rest.splitn(3, '/').collect();

        if parts.len() < 2 {
            return Err(RustVikingError::InvalidUri(
                "URI format: viking://scope/account/path".into(),
            ));
        }

        let scope = parts[0].to_string();
        let account = parts[1].to_string();
        let path = if parts.len() > 2 {
            format!("/{}", parts[2])
        } else {
            String::from("/")
        };

        // Validate scope
        if !["resources", "user", "agent", "session"].contains(&scope.as_str()) {
            return Err(RustVikingError::InvalidUri(format!(
                "Invalid scope '{}', must be one of: resources, user, agent, session",
                scope
            )));
        }

        if account.is_empty() {
            return Err(RustVikingError::InvalidUri(
                "Account cannot be empty".into(),
            ));
        }

        Ok(Self {
            scheme: "viking".into(),
            scope,
            account,
            path,
        })
    }

    /// Convert to internal filesystem path
    pub fn to_internal_path(&self) -> String {
        format!("/{}/{}{}", self.scope, self.account, self.path)
    }

    /// Convert to mount point path
    pub fn to_mount_path(&self) -> String {
        format!("/{}/{}", self.scope, self.account)
    }

    /// Convert back to URI string
    pub fn to_uri_string(&self) -> String {
        let path_part = if self.path == "/" {
            String::new()
        } else {
            self.path[1..].to_string() // strip leading /
        };
        if path_part.is_empty() {
            format!("viking://{}/{}", self.scope, self.account)
        } else {
            format!("viking://{}/{}/{}", self.scope, self.account, path_part)
        }
    }
}

impl VikingUri {
    /// Normalize path (handle `.` and `..` components)
    ///
    /// Returns a new VikingUri with a normalized path.
    pub fn normalize(&self) -> Self {
        let path = self.path.clone();
        let mut normalized = Vec::new();

        for component in path.split('/') {
            match component {
                "" | "." => {
                    // Skip empty and current directory
                }
                ".." => {
                    // Go up one directory
                    normalized.pop();
                }
                _ => {
                    normalized.push(component);
                }
            }
        }

        let normalized_path = if normalized.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", normalized.join("/"))
        };

        Self {
            scheme: self.scheme.clone(),
            scope: self.scope.clone(),
            account: self.account.clone(),
            path: normalized_path,
        }
    }

    /// Get parent directory URI
    ///
    /// Returns `None` if already at root.
    pub fn parent(&self) -> Option<Self> {
        if self.path == "/" || self.path.is_empty() {
            return None;
        }

        let path = self.path.trim_end_matches('/');
        let last_slash = path.rfind('/')?;

        let parent_path = if last_slash == 0 {
            "/".to_string()
        } else {
            path[..last_slash].to_string()
        };

        Some(Self {
            scheme: self.scheme.clone(),
            scope: self.scope.clone(),
            account: self.account.clone(),
            path: parent_path,
        })
    }

    /// Join a child path component
    ///
    /// Creates a new URI with the child appended to the path.
    pub fn join(&self, child: &str) -> Self {
        let child = child.trim_start_matches('/');

        let new_path = if self.path == "/" || self.path.is_empty() {
            format!("/{}", child)
        } else {
            format!("{}/{}", self.path.trim_end_matches('/'), child)
        };

        Self {
            scheme: self.scheme.clone(),
            scope: self.scope.clone(),
            account: self.account.clone(),
            path: new_path,
        }
    }

    /// Check if this URI starts with the given prefix
    ///
    /// Returns true if this URI is a descendant of the prefix.
    pub fn starts_with(&self, prefix: &VikingUri) -> bool {
        if self.scheme != prefix.scheme
            || self.scope != prefix.scope
            || self.account != prefix.account
        {
            return false;
        }

        // Normalize both paths for comparison
        let self_path = self.path.trim_end_matches('/');
        let prefix_path = prefix.path.trim_end_matches('/');

        if prefix_path == "/" || prefix_path.is_empty() {
            return true; // Root prefix matches everything
        }

        self_path == prefix_path || self_path.starts_with(&format!("{}/", prefix_path))
    }
}

impl std::fmt::Display for VikingUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uri_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let uri = VikingUri::parse("viking://resources/project/docs/api.md").unwrap();
        assert_eq!(uri.scope, "resources");
        assert_eq!(uri.account, "project");
        assert_eq!(uri.path, "/docs/api.md");
    }

    #[test]
    fn test_parse_no_path() {
        let uri = VikingUri::parse("viking://user/alice").unwrap();
        assert_eq!(uri.scope, "user");
        assert_eq!(uri.account, "alice");
        assert_eq!(uri.path, "/");
    }

    #[test]
    fn test_invalid_scheme() {
        assert!(VikingUri::parse("http://resources/project").is_err());
    }

    #[test]
    fn test_invalid_scope() {
        assert!(VikingUri::parse("viking://invalid/project").is_err());
    }

    #[test]
    fn test_to_internal_path() {
        let uri = VikingUri::parse("viking://resources/project/docs/api.md").unwrap();
        assert_eq!(uri.to_internal_path(), "/resources/project/docs/api.md");
    }

    #[test]
    fn test_roundtrip() {
        let original = "viking://resources/project/docs/api.md";
        let uri = VikingUri::parse(original).unwrap();
        assert_eq!(uri.to_uri_string(), original);
    }

    #[test]
    fn test_normalize() {
        // Test with . components
        let uri = VikingUri::parse("viking://resources/project/docs/./file.md").unwrap();
        let normalized = uri.normalize();
        assert_eq!(normalized.path, "/docs/file.md");

        // Test with .. components
        let uri = VikingUri::parse("viking://resources/project/docs/../file.md").unwrap();
        let normalized = uri.normalize();
        assert_eq!(normalized.path, "/file.md");

        // Test with mixed components
        let uri = VikingUri::parse("viking://resources/project/a/b/../../c/file.md").unwrap();
        let normalized = uri.normalize();
        assert_eq!(normalized.path, "/c/file.md");
    }

    #[test]
    fn test_parent() {
        // File parent
        let uri = VikingUri::parse("viking://resources/project/docs/api.md").unwrap();
        let parent = uri.parent().unwrap();
        assert_eq!(parent.path, "/docs");

        // Directory parent
        let uri = VikingUri::parse("viking://resources/project/docs").unwrap();
        let parent = uri.parent().unwrap();
        assert_eq!(parent.path, "/");

        // Root has no parent
        let uri = VikingUri::parse("viking://resources/project").unwrap();
        assert!(uri.parent().is_none());
    }

    #[test]
    fn test_join() {
        // Join to root
        let uri = VikingUri::parse("viking://resources/project").unwrap();
        let joined = uri.join("docs");
        assert_eq!(joined.path, "/docs");

        // Join to path
        let uri = VikingUri::parse("viking://resources/project/docs").unwrap();
        let joined = uri.join("api.md");
        assert_eq!(joined.path, "/docs/api.md");

        // Join with leading slash
        let uri = VikingUri::parse("viking://resources/project").unwrap();
        let joined = uri.join("/file.md");
        assert_eq!(joined.path, "/file.md");
    }

    #[test]
    fn test_starts_with() {
        let prefix = VikingUri::parse("viking://resources/project").unwrap();
        let child = VikingUri::parse("viking://resources/project/docs/api.md").unwrap();

        assert!(child.starts_with(&prefix));
        assert!(prefix.starts_with(&prefix)); // Self-match

        let sibling = VikingUri::parse("viking://resources/other").unwrap();
        assert!(!child.starts_with(&sibling));

        let different_scope = VikingUri::parse("viking://user/project/docs").unwrap();
        assert!(!different_scope.starts_with(&prefix));
    }
}
