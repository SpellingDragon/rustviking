# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Qdrant vector store adapter (`src/vector_store/qdrant.rs`)
- Async VectorStore trait with `async_trait` support
- Tokio async runtime integration
- VikingFS unified abstraction layer (`src/vikingfs/`)
- L0/L1 heuristic summary generation system
- 12 VikingFS top-level CLI commands (read/write/mkdir/rm/mv/ls/stat/abstract/overview/detail/find/commit)
- IVF index RocksDB persistence (`src/index/ivf_persist.rs`)
- HNSW index RocksDB persistence (`src/index/hnsw_persist.rs`)
- AGFS setup module (`src/agfs/setup.rs`)
- DirectorySummaryAggregator for bottom-up summary aggregation
- HeuristicSummaryProvider with Markdown-aware extraction
- VikingFS end-to-end tests (`tests/vikingfs_test.rs`)

### Changed
- VectorStore trait refactored to async (OpenViking CollectionAdapter pattern)
- MemoryVectorStore adapted to async with tokio::sync::RwLock
- RocksDBVectorStore adapted to async with spawn_blocking bridge
- VectorSyncManager, VikingFS, and CLI commands fully async
- EmbeddingProvider trait converted to async
- All tests migrated to #[tokio::test]
- Replaced `croaring` (C binding) with `roaring` (pure Rust) for zero-CGO compliance
- Expanded `RustVikingError` from 6 to 18 error variants
- Enhanced `VikingUri` with normalize/parent/join/starts_with methods
- Extended configuration with `[summary]` section
- VikingFS::from_config now properly mounts AGFS and supports RocksDB vector store

### Fixed
- Qdrant VectorsOptions pattern matching for latest qdrant-client API
- Dead code warnings in qdrant.rs and rocks.rs
- AGFS mount initialization in VikingFS (was creating empty MountableFS)
- L0 summary file path convention unified to suffix pattern (file.md.abstract.md)
- Aggregator now correctly skips all summary files using suffix matching

## [0.1.0] - 2026-03-29

### Added

#### AGFS 虚拟文件系统
- 实现 AGFS (Agent File System) 虚拟文件系统核心
- 支持 Viking URI 格式 (`viking://scope/account/path`)
- 基于 Radix Tree 的挂载点路由系统
- POSIX 风格的文件系统接口 (create, read, write, mkdir, stat 等)
- 多存储后端挂载支持

#### 存储层
- RocksDB 键值存储集成
- KV 存储抽象接口，支持 get/put/delete/scan/range/batch 操作
- 批量写入器支持原子操作
- 可配置的存储参数 (max_open_files, fsync, block_cache 等)

#### 向量索引层
- 分层上下文索引架构 (L0/L1/L2)
  - L0: 摘要层 (~100 tokens)，用于快速检索
  - L1: 概述层 (~2k tokens)，用于规划阶段
  - L2: 详细内容层，完整原始数据
- IVF-PQ 向量索引实现
- HNSW 向量索引支持
- 多种距离度量支持 (L2, Cosine, DotProduct)
- 基于 faer 的 SIMD 优化向量计算

#### CLI 命令行工具
- 文件系统命令: `fs mkdir`, `fs ls`, `fs cat`, `fs write`, `fs rm`, `fs stat`
- 键值存储命令: `kv get`, `kv put`, `kv del`, `kv scan`, `kv batch`
- 索引命令: `index insert`, `index search`, `index delete`, `index info`
- 服务器管理命令: `server start`, `server stop`, `server status`
- 基准测试命令: `bench kv-write`, `bench kv-read`, `bench vector-search`, `bench bitmap-ops`
- 多种输出格式支持 (JSON, Table, Plain)

#### 插件系统
- 存储插件抽象接口
- LocalFS 插件实现（本地文件系统）
- 插件注册表支持动态插件加载

#### 基础设施
- 项目基础架构和目录结构
- 配置管理系统（TOML 格式）
- 错误处理框架
- 日志系统（tracing）
- 单元测试和集成测试框架
- 基准测试框架（criterion）

#### 文档
- 核心概念文档 (concept.md)
- 配置文件示例 (config.toml.example)
- 开源基础文档 (README, LICENSE, CHANGELOG, SECURITY)

### Technical Details

#### Dependencies
- `radix_trie`: 基数树路由
- `rocksdb`: KV 存储后端
- `rusqlite`: SQL 数据库支持
- `faer`: SIMD 矩阵运算
- `half`: f16 支持
- `clap`: CLI 解析
- `serde`: 序列化
- `tracing`: 日志框架

#### Performance Targets
- CLI 命令延迟: < 5ms
- 向量检索延迟: < 10ms
- 单二进制部署，零 CGO 依赖

[Unreleased]: https://github.com/SpellingDragon/rustviking/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/SpellingDragon/rustviking/releases/tag/v0.1.0
