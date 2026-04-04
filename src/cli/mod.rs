//! CLI command definitions and handlers

pub mod bench_commands;
pub mod commands;
pub mod fs_commands;
pub mod index_commands;
pub mod store_commands;
pub mod viking_commands;

use serde::Serialize;

/// Unified JSON response structure for CLI output
#[derive(Serialize)]
pub struct CliResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> CliResponse<T> {
    /// Create a successful response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"success":false,"error":"Failed to serialize response"}"#.to_string()
        })
    }

    /// Convert to pretty JSON string
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            r#"{"success":false,"error":"Failed to serialize response"}"#.to_string()
        })
    }
}

/// Create a successful response with data
pub fn success<T: Serialize>(data: T) -> CliResponse<T> {
    CliResponse::success(data)
}

/// Create an error response
pub fn error<T: Serialize>(message: impl Into<String>) -> CliResponse<T> {
    CliResponse {
        success: false,
        data: None,
        error: Some(message.into()),
    }
}

/// Output helper for different formats
pub fn output_json<T: Serialize>(response: &CliResponse<T>) {
    println!("{}", response.to_json_pretty());
}

/// Output error to stderr (for human-readable logs)
pub fn output_error_to_stderr(message: &str) {
    eprintln!("Error: {}", message);
}
