//! CLI command definitions

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "rustviking")]
#[command(version = "0.1.0")]
#[command(about = "RustViking - OpenViking Core in Rust")]
pub struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    /// Output format
    #[arg(short, long, default_value = "json", value_enum)]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Filesystem operations
    Fs {
        #[command(subcommand)]
        operation: FsOperation,
    },

    /// Key-value store operations
    Kv {
        #[command(subcommand)]
        operation: KvOperation,
    },

    /// Vector index operations
    Index {
        #[command(subcommand)]
        operation: IndexOperation,
    },

    /// Server management
    Server {
        #[command(subcommand)]
        operation: ServerOperation,
    },

    /// Benchmark tests
    Bench {
        /// Test type
        #[arg(value_enum)]
        test: BenchTest,
        /// Number of operations
        #[arg(short, long, default_value = "1000")]
        count: usize,
    },
}

#[derive(Subcommand)]
pub enum FsOperation {
    /// Create directory
    Mkdir {
        /// Viking URI or path
        path: String,
        /// Directory mode
        #[arg(short, long, default_value = "0755")]
        mode: String,
    },
    /// List directory contents
    Ls {
        /// Viking URI or path
        path: String,
        /// Recursive listing
        #[arg(short, long)]
        recursive: bool,
    },
    /// Read file content
    Cat {
        /// Viking URI or path
        path: String,
    },
    /// Write data to file
    Write {
        /// Viking URI or path
        path: String,
        /// Data to write
        #[arg(short, long)]
        data: String,
    },
    /// Remove file or directory
    Rm {
        /// Viking URI or path
        path: String,
        /// Recursive removal
        #[arg(short, long)]
        recursive: bool,
    },
    /// Get file information
    Stat {
        /// Viking URI or path
        path: String,
    },
}

#[derive(Subcommand)]
pub enum KvOperation {
    /// Get value by key
    Get {
        #[arg(short, long)]
        key: String,
    },
    /// Set key-value pair
    Put {
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        value: String,
    },
    /// Delete a key
    Del {
        #[arg(short, long)]
        key: String,
    },
    /// Scan keys by prefix
    Scan {
        #[arg(short, long)]
        prefix: String,
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum IndexOperation {
    /// Insert a vector
    Insert {
        #[arg(short, long)]
        id: u64,
        #[arg(short, long, value_delimiter = ',')]
        vector: Vec<f32>,
        #[arg(short, long, default_value = "1")]
        level: u8,
    },
    /// Search for similar vectors
    Search {
        #[arg(short, long, value_delimiter = ',')]
        query: Vec<f32>,
        #[arg(short, long, default_value = "10")]
        k: usize,
        #[arg(short, long)]
        level: Option<u8>,
    },
    /// Delete a vector
    Delete {
        #[arg(short, long)]
        id: u64,
    },
    /// Show index information
    Info {},
}

#[derive(Subcommand)]
pub enum ServerOperation {
    /// Start server
    Start {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Stop server
    Stop {},
    /// Show server status
    Status {},
}

#[derive(ValueEnum, Clone)]
pub enum BenchTest {
    KvWrite,
    KvRead,
    VectorSearch,
    BitmapOps,
}

#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    Json,
    Table,
    Plain,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_fs_mkdir() {
        let cli = Cli::parse_from(["rustviking", "fs", "mkdir", "/local/test"]);
        match cli.command {
            Commands::Fs {
                operation: FsOperation::Mkdir { path, .. },
            } => {
                assert_eq!(path, "/local/test");
            }
            _ => panic!("Expected Fs Mkdir"),
        }
    }

    #[test]
    fn test_parse_kv_put() {
        let cli = Cli::parse_from(["rustviking", "kv", "put", "-k", "mykey", "-v", "myval"]);
        match cli.command {
            Commands::Kv {
                operation: KvOperation::Put { key, value },
            } => {
                assert_eq!(key, "mykey");
                assert_eq!(value, "myval");
            }
            _ => panic!("Expected Kv Put"),
        }
    }

    #[test]
    fn test_parse_index_search() {
        let cli = Cli::parse_from([
            "rustviking",
            "index",
            "search",
            "-q",
            "0.1,0.2,0.3",
            "-k",
            "5",
        ]);
        match cli.command {
            Commands::Index {
                operation: IndexOperation::Search { query, k, .. },
            } => {
                assert_eq!(query, vec![0.1, 0.2, 0.3]);
                assert_eq!(k, 5);
            }
            _ => panic!("Expected Index Search"),
        }
    }

    #[test]
    fn test_default_output_format() {
        let cli = Cli::parse_from(["rustviking", "fs", "ls", "/test"]);
        assert!(matches!(cli.output, OutputFormat::Json));
    }
}
