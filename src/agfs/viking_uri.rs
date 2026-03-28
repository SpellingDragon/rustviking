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
            return Err(RustVikingError::InvalidUri("Account cannot be empty".into()));
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
}
