# RustViking

> OpenViking Core in Rust — A high-performance, CLI-first AI Agent memory infrastructure.

<p align="center">
  <a href="https://github.com/SpellingDragon/rustviking/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/SpellingDragon/rustviking/ci.yml?branch=main&label=CI&style=flat-square" alt="CI Status">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License">
  </a>
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/rust-1.82+-orange.svg?style=flat-square" alt="Rust Version">
  </a>
</p>

---

## Project Status

**Experimental Project** — This is a Rust implementation exploration of OpenViking core concepts, currently in active development.

### Feature Matrix

| Feature | Status | Description |
|---------|--------|-------------|
| **AGFS Virtual File System** | ✅ Ready | Unified filesystem abstraction via `viking://` URI |
| **RocksDB KV Storage** | ✅ Production | Persistent key-value storage with RocksDB |
| **HNSW Vector Index** | ✅ Persistent | HNSW implementation with RocksDB persistence |
| **IVF Vector Index** | ✅ Persistent | IVF clustering index with RocksDB persistence |
| **L0/L1 Summary Layer** | ✅ Heuristic | Automatic abstract/overview generation |
| **VikingFS Core** | ✅ Ready | Unified abstraction layer for AGFS and Vector Store |
| **VikingFS CLI** | ✅ 12 Commands | read/write/mkdir/rm/mv/ls/stat/abstract/overview/detail/find/commit |
| **统一 JSON 输出** | ✅ Ready | 所有 CLI 命令返回统一 CliResponse JSON 格式 |
| **CLI Bench 命令** | ✅ Ready | 内置性能测试：kv-write/kv-read/vector-search/bitmap-ops |
| **KV Batch 操作** | ✅ Ready | 支持从文件或 stdin 批量执行 put/delete 操作 |
| **CLI 错误分类** | ✅ Ready | CliInput 错误类型用于用户输入错误处理 |
| **OpenAI Embedding** | ✅ Ready | Compatible with OpenAI API embedding services |
| **SIMD Optimization** | ✅ Ready | ARM NEON / x86 AVX2/FMA acceleration (4-8x speedup) |
| **Qdrant Vector Store** | ✅ Ready | Async VectorStore adapter for Qdrant cloud/local |
| **Async VectorStore** | ✅ Ready | OpenViking CollectionAdapter pattern with `async_trait` |
| **S3FS Plugin** | ❌ Missing | S3-compatible storage backend |
| **SQLFS Plugin** | ❌ Missing | SQL database storage backend |
| **HTTP/gRPC Service** | ❌ Missing | REST API and gRPC interface |

### v0.2.0 Highlights

本次版本更新重点：
- **CLI 体验优化**：统一 JSON 响应格式 (CliResponse)，标准化退出码 (0=成功, 1=用户错误, 2=系统错误)
- **批量操作支持**：新增 `kv batch` 命令，支持从文件或 stdin 批量执行 put/delete 操作
- **性能测试内置化**：新增 `bench` 命令，无需外部工具即可测试 kv-write/kv-read/vector-search/bitmap-ops
- **测试覆盖率提升**：新增约 100 个测试用例，覆盖 AGFS、索引、KV 存储等核心功能

### Differences from OpenViking

[OpenViking](https://github.com/volcengine/OpenViking) is ByteDance's production-grade AI Agent context database, featuring:
- Intent analysis
- Hierarchical retrieval and reranking
- Session management and memory extraction
- Document parsing and LLM integration

**RustViking currently implements only the storage layer foundation** of OpenViking, without the advanced features above. For production-grade AI Agent memory systems, we recommend using [OpenViking](https://github.com/volcengine/OpenViking) directly.

Key advantages of RustViking:
- **Pure Rust implementation**: No Python dependencies, single binary deployment
- **SIMD acceleration**: Platform-specific vector instructions (ARM NEON / x86 AVX2/FMA)
- **CLI-first design**: Optimized for command-line workflows and scripting
- **Zero CGO**: Easy cross-compilation and deployment

---

## Quick Start

### Requirements

- **Rust**: 1.82 or higher
- **OS**: macOS 10.15+ / Linux (Ubuntu 20.04+) / Windows (WSL2)

### Build

```bash
# Clone repository
git clone https://github.com/SpellingDragon/rustviking.git
cd rustviking

# Debug build (development)
cargo build

# Release build (production, recommended)
cargo build --release
```

### Basic Usage

```bash
# Show help
./target/release/rustviking --help

# VikingFS commands (top-level)
./rustviking mkdir viking://resources/project/docs
./rustviking write viking://resources/doc.md -d "Hello, RustViking!"
./rustviking read viking://resources/doc.md
./rustviking ls viking://resources/
./rustviking stat viking://resources/doc.md

# L0/L1 Summary commands
./rustviking abstract viking://resources/doc.md    # Generate L0 abstract
./rustviking overview viking://resources/          # Generate L1 overview
./rustviking detail viking://resources/doc.md      # Read L2 full content
./rustviking commit viking://resources/            # Trigger summary aggregation

# Search commands
./rustviking find "authentication" -k 10           # Semantic search
./rustviking find "query" -t viking://resources/ -k 5 -l L1

# Legacy commands
./rustviking kv put -k "user:1:name" -v "Alice"
./rustviking kv get -k "user:1:name"
./rustviking kv batch -f - < batch_ops.json       # Batch operations from stdin
./rustviking index insert -i 1 --vector 0.1,0.2,0.3,0.4 -l 2
./rustviking index search -q 0.1,0.2,0.3,0.4 -k 10

# Benchmark commands
./rustviking bench kv-write -c 10000              # KV write benchmark
./rustviking bench kv-read -c 10000               # KV read benchmark
./rustviking bench vector-search -c 1000          # Vector search benchmark
./rustviking bench bitmap-ops -c 10000            # Bitmap operations benchmark
```

---

## Architecture

```mermaid
flowchart TB
    subgraph CLI["CLI Commands"]
    end
    subgraph VikingFS["VikingFS Core<br/>(Unified Abstraction)"]
    end
    subgraph AGFS["AGFS Virtual File System<br/>(Radix Tree Routing + Multi-backend Mount)"]
    end
    subgraph Backends["Backend Implementations"]
        LocalFS["LocalFS<br/>(Local Filesystem)"]
        MemoryFS["MemoryFS<br/>(In-Memory)"]
        VectorStore["VectorStore<br/>(Adapter Pattern)"]
    end
    subgraph VectorAdapters["VectorStore Adapters<br/>(async_trait)"]
        MemoryVector["MemoryVectorStore<br/>(tokio::sync::RwLock)"]
        RocksVector["RocksDBVectorStore<br/>(spawn_blocking)"]
        QdrantVector["QdrantVectorStore<br/>(New)"]
    end
    subgraph Storage["Underlying Storage"]
        RocksDB["Storage Layer (RocksDB)"]
        VectorIndex["Vector Index (HNSW/IVF)"]
        QdrantCloud["Qdrant Cloud/Local"]
    end
    CLI --> VikingFS
    VikingFS --> AGFS
    AGFS --> LocalFS
    AGFS --> MemoryFS
    AGFS --> VectorStore
    LocalFS --> RocksDB
    MemoryFS --> RocksDB
    VectorStore --> VectorAdapters
    MemoryVector --> RocksDB
    RocksVector --> RocksDB
    RocksVector --> VectorIndex
    QdrantVector --> QdrantCloud
```

### Module Structure

```
src/
├── agfs/           # AGFS Virtual File System
├── vikingfs/       # VikingFS Core (unified abstraction)
├── index/          # Vector Index (HNSW/IVF with persistence)
├── storage/        # KV Storage (RocksDB)
├── vector_store/   # Vector Store abstraction (Adapter Pattern)
│   ├── traits.rs   # Async VectorStore trait
│   ├── memory.rs   # In-memory backend
│   ├── rocks.rs    # RocksDB backend
│   └── qdrant.rs   # Qdrant cloud backend
├── embedding/      # Embedding Providers
├── compute/        # SIMD-optimized Distance Computations
│   ├── simd.rs     # ARM NEON / x86 AVX2/FMA acceleration
│   └── distance.rs # Distance computation kernels
├── cli/            # CLI Commands
├── config/         # Configuration
└── error.rs        # Error Types (18 variants)
```

### SIMD Optimization

RustViking uses platform-specific SIMD instructions for high-performance vector computations:

- **ARM64 (Apple Silicon, etc.)**: NEON intrinsics for batch dot product and L2 distance
- **x86_64**: AVX2/FMA intrinsics when available, with automatic fallback to scalar operations
- **Parallel Processing**: Rayon-based parallel iteration for large-scale batch operations

Performance improvements:
- Up to **4-8x faster** for dot product computations (depending on vector dimension)
- Significant speedup in vector search and indexing operations
- Automatic runtime detection of CPU features

See `benches/compute_bench.rs` for benchmark comparisons between SIMD and scalar implementations.

---

## CLI Commands

### VikingFS Commands (Top-Level)

| Command | Description | Example |
|---------|-------------|---------|
| `read` | Read file content | `rustviking read viking://resources/doc.md` |
| `write` | Write file | `rustviking write viking://resources/doc.md "content"` |
| `mkdir` | Create directory | `rustviking mkdir viking://resources/project/docs` |
| `rm` | Remove file/directory | `rustviking rm viking://resources/doc.md` |
| `mv` | Move/rename | `rustviking mv viking://old.md viking://new.md` |
| `ls` | List directory | `rustviking ls viking://resources/` |
| `stat` | Get file info | `rustviking stat viking://resources/doc.md` |
| `abstract` | Read/generate L0 abstract | `rustviking abstract viking://resources/doc.md` |
| `overview` | Read/generate L1 overview | `rustviking overview viking://resources/` |
| `detail` | Read L2 full content | `rustviking detail viking://resources/doc.md` |
| `find` | Search content | `rustviking find "query" --k 10` |
| `commit` | Trigger aggregation | `rustviking commit viking://resources/` |

### Legacy Commands

| Command | Description | Example |
|---------|-------------|---------|
| `kv get` | Get value | `rustviking kv get -k "user:1:name"` |
| `kv put` | Set key-value | `rustviking kv put -k "user:1:name" -v "Alice"` |
| `kv del` | Delete key | `rustviking kv del -k "user:1:name"` |
| `kv scan` | Prefix scan | `rustviking kv scan --prefix "user:" --limit 100` |
| `kv batch` | Batch operations | `rustviking kv batch -f ops.json` or `cat ops.json \| rustviking kv batch -f -` |

### KV Batch Operations

批量操作 JSON 格式：

```json
[
  {"op": "put", "key": "user:1:name", "value": "Alice"},
  {"op": "put", "key": "user:1:email", "value": "alice@example.com"},
  {"op": "del", "key": "user:2:name"},
  {"op": "put", "key": "session:active", "value": "true"}
]
```

支持的批量操作：
- `put`: 写入键值对
- `del`: 删除键
| `index insert` | Insert vector | `rustviking index insert -i 1 --vector 0.1,0.2 -l 2` |
| `index search` | Vector search | `rustviking index search -q 0.1,0.2 -k 10` |
| `bench` | Benchmark tests | `rustviking bench kv-write -c 10000` |

### Benchmark Commands

| Subcommand | Description | Example |
|------------|-------------|---------|
| `kv-write` | KV write throughput | `rustviking bench kv-write -c 10000` |
| `kv-read` | KV read throughput | `rustviking bench kv-read -c 10000` |
| `vector-search` | Vector search latency | `rustviking bench vector-search -c 1000` |
| `bitmap-ops` | Bitmap operations | `rustviking bench bitmap-ops -c 10000` |

Benchmark options:
- `-c, --count`: Number of operations (default: 1000)
- `--latency`: Show latency statistics (P50/P95/P99)

---

## Configuration

Create a `config.toml` file:

```toml
[storage]
path = "./data/rustviking"
create_if_missing = true

[vector]
dimension = 768
index_type = "ivf_pq"

# Vector Store Backend: "memory", "rocksdb", or "qdrant"
[vector_store]
plugin = "rocksdb"

[vector_store.rocksdb]
path = "./data/rustviking/vector_store"

# Qdrant Configuration (when plugin = "qdrant")
[vector_store.qdrant]
url = "http://localhost:6334"
collection_name = "rustviking"
# api_key = "your-api-key"  # For Qdrant Cloud

[embedding]
plugin = "mock"

[summary]
provider = "heuristic"  # Options: "noop", "heuristic"
```

See [config.toml.example](config.toml.example) for full configuration options.

### CLI Reference

For detailed CLI API documentation, see `docs/cli-api-specification.md` (即将创建).

### Vector Store Backends

| Backend | Plugin Name | Use Case |
|---------|-------------|----------|
| **Memory** | `memory` | Development, testing, ephemeral data |
| **RocksDB** | `rocksdb` | Local production, embedded deployments |
| **Qdrant** | `qdrant` | Cloud-native, distributed, high-scale |

---

## As a Rust Library

```rust
use rustviking::vikingfs::VikingFS;
use rustviking::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_file("config.toml")?;
    
    // Initialize VikingFS
    let vikingfs = VikingFS::from_config(&config).await?;
    
    // Write file
    vikingfs.write("viking://resources/doc.md", "Hello, World!").await?;
    
    // Read file
    let content = vikingfs.read("viking://resources/doc.md").await?;
    println!("{}", content);
    
    // Generate abstract
    let abstract_text = vikingfs.abstract_("viking://resources/doc.md").await?;
    
    // Search
    let results = vikingfs.find("query", None, None, 10).await?;
    
    Ok(())
}
```

---

## Benchmarks

```bash
# Run all benchmarks
cargo bench

# KV benchmarks
cargo bench --bench kv_bench

# Vector benchmarks
cargo bench --bench vector_bench

# AGFS benchmarks
cargo bench --bench agfs_bench

# SIMD vs Scalar computation benchmarks
cargo bench --bench compute_bench
```

Benchmark highlights:
- **compute_bench**: Compares SIMD-accelerated vs scalar implementations for dot product and L2 distance
- **vector_bench**: Measures vector search performance with SIMD optimizations enabled
- See `benches/` directory for detailed benchmark suites

Performance targets:
- CLI command latency: < 5ms
- Vector search latency: < 10ms (P99)
- Single binary deployment, zero CGO dependencies

---

## Tribute to OpenViking

RustViking was inspired by **[OpenViking](https://github.com/volcengine/OpenViking)**.

| Dimension | OpenViking | RustViking |
|-----------|-----------|------------|
| **Language** | Go + Python + C++ | Pure Rust |
| **Interaction** | HTTP/gRPC Service | **CLI-first** |
| **Scope** | Full Agent Platform | Storage Layer Foundation |
| **Maturity** | Production-grade | Experimental |

Special thanks to the OpenViking team for their open-source contribution!

---

## Contributing

All forms of contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Submitting Issues and Feature Requests
- Setting up development environment
- Submitting Pull Requests

---

## License

RustViking is licensed under [Apache-2.0](LICENSE).

---

*For Chinese documentation, see [concept.md](concept.md)*
