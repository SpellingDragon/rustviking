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

    // === VikingFS 语义层命令 ===

    /// 读取文件内容（支持 L0/L1/L2 级别）
    Read {
        #[arg(help = "Viking URI")]
        uri: String,
        #[arg(short, long, help = "读取级别: L0, L1, L2")]
        level: Option<String>,
    },

    /// 写入文件（自动 embedding + 索引）
    Write {
        #[arg(help = "Viking URI")]
        uri: String,
        #[arg(short, long)]
        data: String,
        #[arg(long, default_value = "false", help = "自动生成摘要")]
        auto_summary: bool,
    },

    /// 创建目录
    Mkdir {
        #[arg(help = "Viking URI")]
        uri: String,
    },

    /// 删除文件/目录
    Rm {
        #[arg(help = "Viking URI")]
        uri: String,
        #[arg(short, long)]
        recursive: bool,
    },

    /// 移动/重命名
    Mv {
        #[arg(help = "源 Viking URI")]
        from: String,
        #[arg(help = "目标 Viking URI")]
        to: String,
    },

    /// 列出目录内容
    Ls {
        #[arg(help = "Viking URI")]
        uri: String,
        #[arg(short, long)]
        recursive: bool,
    },

    /// 获取文件信息
    Stat {
        #[arg(help = "Viking URI")]
        uri: String,
    },

    /// 读取抽象摘要 (L0)
    Abstract {
        #[arg(help = "Viking URI")]
        uri: String,
    },

    /// 读取概述摘要 (L1)
    Overview {
        #[arg(help = "Viking URI")]
        uri: String,
    },

    /// 读取完整内容 (L2)
    Detail {
        #[arg(help = "Viking URI")]
        uri: String,
    },

    /// 语义搜索（文本输入，自动 embedding）
    Find {
        #[arg(help = "搜索查询文本")]
        query: String,
        #[arg(short, long, help = "目标 URI 范围")]
        target: Option<String>,
        #[arg(short, long, default_value = "10")]
        k: usize,
        #[arg(short, long, help = "搜索级别: L0, L1, L2")]
        level: Option<String>,
    },

    /// 提交目录（触发摘要聚合）
    Commit {
        #[arg(help = "Viking URI（目录）")]
        uri: String,
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

    // === VikingFS 命令解析测试 ===

    #[test]
    fn test_parse_vikingfs_read() {
        let cli = Cli::parse_from(["rustviking", "read", "viking://resources/test.md"]);
        match cli.command {
            Commands::Read { uri, level } => {
                assert_eq!(uri, "viking://resources/test.md");
                assert_eq!(level, None);
            }
            _ => panic!("Expected Read command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_read_with_level() {
        let cli = Cli::parse_from(["rustviking", "read", "viking://resources/test.md", "-l", "L0"]);
        match cli.command {
            Commands::Read { uri, level } => {
                assert_eq!(uri, "viking://resources/test.md");
                assert_eq!(level, Some("L0".to_string()));
            }
            _ => panic!("Expected Read command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_write() {
        let cli = Cli::parse_from([
            "rustviking",
            "write",
            "viking://resources/test.md",
            "-d",
            "Hello World",
        ]);
        match cli.command {
            Commands::Write { uri, data, auto_summary } => {
                assert_eq!(uri, "viking://resources/test.md");
                assert_eq!(data, "Hello World");
                assert!(!auto_summary);
            }
            _ => panic!("Expected Write command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_write_with_auto_summary() {
        let cli = Cli::parse_from([
            "rustviking",
            "write",
            "viking://resources/test.md",
            "-d",
            "Hello",
            "--auto-summary",
        ]);
        match cli.command {
            Commands::Write { auto_summary, .. } => {
                assert!(auto_summary);
            }
            _ => panic!("Expected Write command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_mkdir() {
        let cli = Cli::parse_from(["rustviking", "mkdir", "viking://resources/newdir"]);
        match cli.command {
            Commands::Mkdir { uri } => {
                assert_eq!(uri, "viking://resources/newdir");
            }
            _ => panic!("Expected Mkdir command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_rm() {
        let cli = Cli::parse_from(["rustviking", "rm", "viking://resources/test.md"]);
        match cli.command {
            Commands::Rm { uri, recursive } => {
                assert_eq!(uri, "viking://resources/test.md");
                assert!(!recursive);
            }
            _ => panic!("Expected Rm command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_rm_recursive() {
        let cli = Cli::parse_from(["rustviking", "rm", "viking://resources/dir", "-r"]);
        match cli.command {
            Commands::Rm { uri, recursive } => {
                assert_eq!(uri, "viking://resources/dir");
                assert!(recursive);
            }
            _ => panic!("Expected Rm command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_mv() {
        let cli = Cli::parse_from([
            "rustviking",
            "mv",
            "viking://resources/old.md",
            "viking://resources/new.md",
        ]);
        match cli.command {
            Commands::Mv { from, to } => {
                assert_eq!(from, "viking://resources/old.md");
                assert_eq!(to, "viking://resources/new.md");
            }
            _ => panic!("Expected Mv command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_ls() {
        let cli = Cli::parse_from(["rustviking", "ls", "viking://resources"]);
        match cli.command {
            Commands::Ls { uri, recursive } => {
                assert_eq!(uri, "viking://resources");
                assert!(!recursive);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_stat() {
        let cli = Cli::parse_from(["rustviking", "stat", "viking://resources/test.md"]);
        match cli.command {
            Commands::Stat { uri } => {
                assert_eq!(uri, "viking://resources/test.md");
            }
            _ => panic!("Expected Stat command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_abstract() {
        let cli = Cli::parse_from(["rustviking", "abstract", "viking://resources/test.md"]);
        match cli.command {
            Commands::Abstract { uri } => {
                assert_eq!(uri, "viking://resources/test.md");
            }
            _ => panic!("Expected Abstract command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_overview() {
        let cli = Cli::parse_from(["rustviking", "overview", "viking://resources"]);
        match cli.command {
            Commands::Overview { uri } => {
                assert_eq!(uri, "viking://resources");
            }
            _ => panic!("Expected Overview command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_detail() {
        let cli = Cli::parse_from(["rustviking", "detail", "viking://resources/test.md"]);
        match cli.command {
            Commands::Detail { uri } => {
                assert_eq!(uri, "viking://resources/test.md");
            }
            _ => panic!("Expected Detail command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_find() {
        let cli = Cli::parse_from(["rustviking", "find", "search query"]);
        match cli.command {
            Commands::Find { query, target, k, level } => {
                assert_eq!(query, "search query");
                assert_eq!(target, None);
                assert_eq!(k, 10); // default value
                assert_eq!(level, None);
            }
            _ => panic!("Expected Find command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_find_with_options() {
        let cli = Cli::parse_from([
            "rustviking",
            "find",
            "search query",
            "-t",
            "viking://resources",
            "-k",
            "5",
            "-l",
            "L1",
        ]);
        match cli.command {
            Commands::Find { query, target, k, level } => {
                assert_eq!(query, "search query");
                assert_eq!(target, Some("viking://resources".to_string()));
                assert_eq!(k, 5);
                assert_eq!(level, Some("L1".to_string()));
            }
            _ => panic!("Expected Find command"),
        }
    }

    #[test]
    fn test_parse_vikingfs_commit() {
        let cli = Cli::parse_from(["rustviking", "commit", "viking://resources"]);
        match cli.command {
            Commands::Commit { uri } => {
                assert_eq!(uri, "viking://resources");
            }
            _ => panic!("Expected Commit command"),
        }
    }
}
