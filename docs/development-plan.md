# RustViking 开发计划

> 本计划旨在将 RustViking 打造成生产级的 AI Agent 记忆基础设施库。

## 目录

- [核心目标](#核心目标)
- [OpenViking 对齐分析](#openviking-对齐分析)
- [Phase 1: 向量索引持久化](#phase-1-向量索引持久化-p0)
- [Phase 2: L0/L1 自动摘要层](#phase-2-l0l1-自动摘要层-p0)
- [Phase 3: VikingFS 虚拟文件系统](#phase-3-vikingfs-虚拟文件系统-p0)
- [Phase 4: CLI 完善](#phase-4-cli-完善-p0)
- [Phase 5: 测试与质量](#phase-5-测试与质量-p1)
- [Phase 6: 文档与发布](#phase-6-文档与发布-p1)
- [任务依赖图](#任务依赖图)
- [目录结构更新](#目录结构更新)

---

## 核心目标

**成为 Agent 友好的 AI 记忆基础设施库**：通过 CLI 提供 Viking URI 抽象、AGFS 虚拟文件系统、L0/L1/L2 分层上下文管理、RocksDB 持久化向量索引。

### 核心价值

| 维度 | 目标 |
|------|------|
| **性能** | CLI 命令延迟 < 5ms，向量检索 < 10ms |
| **稳定性** | 无 GC 停顿，延迟可预测 |
| **简洁性** | 单二进制，零外部依赖（CGO） |
| **可扩展性** | 模块化架构，易于二次开发 |

---

## OpenViking 对齐分析

### 能力对比总览

| 能力维度 | OpenViking | RustViking | 状态 |
|---------|-----------|------------|------|
| **AGFS 虚拟文件系统** | ✅ Go | ✅ 框架完成 | ✅ |
| **Viking URI 路由** | ✅ | ✅ 实现 | ✅ |
| **LocalFS 插件** | ✅ | ✅ 实现 | ✅ |
| **MemoryFS 插件** | ✅ | ✅ 实现 | ✅ |
| **S3FS 插件** | ✅ | ❌ 未实现 | 🔴 |
| **SQLFS 插件** | ✅ | ❌ 未实现 | 🔴 |
| **RocksDB KV** | ✅ | ✅ 实现 | ✅ |
| **IVF-PQ 索引** | C++ Faiss | ✅ 持久化实现 | ✅ |
| **HNSW 索引** | C++ Faiss | ✅ 持久化实现 | ✅ |
| **索引持久化** | ✅ | ✅ RocksDB 持久化 | ✅ |
| **L0 摘要层** | 自动生成 | ✅ 启发式实现 | ✅ |
| **L1 概述层** | 自动生成 | ✅ 启发式实现 | ✅ |
| **分层检索** | 完整 | ✅ 实现 | ✅ |
| **向量同步** | ✅ | ⚠️ 框架 | ⚠️ |
| **OpenAI Embedding** | ✅ | ✅ 实现 | ✅ |
| **HTTP/gRPC 服务** | ✅ | ❌ 未实现 | 🔴 |
| **VikingFS Core** | ✅ | ✅ 实现 | ✅ |

### OpenViking L0/L1/L2 设计

OpenViking 使用三层信息模型平衡检索效率和内容完整性：

| Layer | 名称 | Token 限制 | 用途 |
|-------|------|-----------|------|
| **L0** | Abstract | ~100 tokens | 快速过滤、向量检索 |
| **L1** | Overview | ~2k tokens | 内容导航、重排序 |
| **L2** | Detail | 无限制 | 完整内容，按需加载 |

#### 目录结构

```
viking://resources/docs/auth/
├── .abstract.md          # L0: ~100 tokens
├── .overview.md          # L1: ~2k tokens
├── .relations.json       # 关联资源
├── .meta.json            # 元数据
├── oauth.md              # L2: 完整内容
├── jwt.md                # L2: 完整内容
└── api-keys.md           # L2: 完整内容
```

#### 生成机制

- **自底向上聚合**：子目录 L0 聚合为父目录 L1
- **时机**：资源添加时、会话归档时
- **组件**：SemanticProcessor、SessionCompressor、VLM Model

---

## Phase 1: 向量索引持久化 (P0)

> **目标**：实现 IVF-PQ 和 HNSW 索引的 RocksDB 持久化

### 1.1 IvfIndex RocksDB 持久化

```rust
// 设计目标
impl IvfIndex {
    /// 持久化索引到 RocksDB
    pub fn persist(&self, path: &Path) -> Result<()>;

    /// 从 RocksDB 恢复索引
    pub fn restore(path: &Path) -> Result<Self>;

    /// 增量写入
    pub fn append(&self, id: u64, vector: &[f32], level: u8) -> Result<()>;
}
```

**RocksDB 列族设计**：

| 列族 | 用途 | Key 格式 | Value 格式 |
|------|------|---------|-----------|
| `ivf_centroids` | 聚类中心点 | `centroid:{id}` | `[f32; dimension]` |
| `ivf_vectors` | 分区向量 | `{partition}:{id}` | `[f32; dimension]` |
| `ivf_metadata` | 向量元数据 | `meta:{id}` | JSON {uri, level, created_at} |
| `ivf_config` | 索引配置 | `config` | JSON {dimension, num_partitions, ...} |

**任务清单**：
- [x] 设计 RocksDB 列族结构
- [x] 实现 IvfIndex::persist()
- [x] 实现 IvfIndex::restore()
- [x] 实现训练数据序列化/反序列化
- [x] 实现增量写入 WAL
- [x] 单元测试
- [ ] 性能基准

### 1.2 HnswIndex RocksDB 持久化

```rust
// 设计目标
impl HnswIndex {
    /// 持久化索引到 RocksDB
    pub fn persist(&self, path: &Path) -> Result<()>;

    /// 从 RocksDB 恢复索引
    pub fn restore(path: &Path) -> Result<Self>;

    /// 增量添加节点
    pub fn add_node(&self, id: u64, vector: &[f32]) -> Result<()>;
}
```

**持久化格式设计**：

| 组件 | 存储方式 |
|------|---------|
| 图结构 | 邻接列表序列化 |
| 节点向量 | 按 ID 索引存储 |
| 层信息 | `layer:{id}` -> `u8` |
| 元数据 | `meta:{id}` -> JSON |

**任务清单**：
- [x] 设计 HnswIndex 持久化格式
- [x] 实现图结构序列化
- [x] 实现节点向量存储
- [x] 实现 HnswIndex::persist() / restore()
- [x] 实现图结构的增量更新
- [x] 单元测试
- [ ] 性能基准

### 1.3 向量-AGFS 同步机制

```rust
// 设计目标
pub struct VectorSyncManager {
    vector_store: Arc<dyn VectorStore>,
    agfs: Arc<MountableFS>,
}

impl VectorSyncManager {
    /// 删除同步：AGFS 删除时自动删除向量索引
    pub async fn on_delete(&self, uri_prefix: &str) -> Result<()>;

    /// 移动同步：AGFS 移动时自动更新 URI
    pub async fn on_move(&self, old_uri: &str, new_uri: &str) -> Result<()>;

    /// 写入同步：AGFS 写入时自动索引
    pub async fn on_write(&self, uri: &str, vector: &[f32]) -> Result<()>;
}
```

**任务清单**：
- [ ] 设计 VectorSyncManager 架构
- [ ] 实现 URI 前缀删除同步
- [ ] 实现 URI 重命名同步
- [ ] 实现写入时自动索引更新
- [ ] 集成测试

---

## Phase 2: L0/L1 自动摘要层 (P0)

> **目标**：对齐 OpenViking 的自动摘要生成机制

### 2.1 摘要生成服务抽象

```rust
// 摘要 Provider trait
pub trait SummaryProvider: Send + Sync {
    /// 生成抽象摘要 (~100 tokens)
    fn generate_abstract(&self, text: &str) -> Result<String>;

    /// 生成概述摘要 (~2k tokens)
    fn generate_overview(&self, texts: &[String]) -> Result<String>;

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<&str>;
}

// 实现选项
pub enum SummaryBackend {
    /// 启发式（无需 LLM）
    Heuristic(HeuristicSummaryProvider),
    /// OpenAI
    OpenAI(OpenAISummaryProvider),
    /// Ollama 本地模型
    Ollama(OllamaSummaryProvider),
}
```

**任务清单**：
- [x] 定义 SummaryProvider trait
- [x] 实现 HeuristicSummaryProvider (规则/启发式)
- [ ] 实现 LLM 调用接口 (支持 OpenAI/Ollama)
- [x] 配置化选择 provider

### 2.2 启发式摘要实现

```rust
// 抽象生成策略
impl HeuristicSummaryProvider {
    /// 方法1：首句 + 关键词
    pub fn generate_abstract(&self, text: &str) -> String;

    /// 方法2：TF-IDF 选句
    pub fn extract_key_sentences(&self, text: &str, max_tokens: usize) -> Vec<String>;
}

// 概述生成策略
impl HeuristicSummaryProvider {
    /// 子目录 abstract 聚合
    pub fn generate_overview_from_children(
        &self,
        children: &[ChildSummary],
    ) -> String;

    /// 文件名模式识别
    pub fn infer_structure(&self, files: &[FileInfo]) -> String;
}
```

**任务清单**：
- [x] 实现基于规则的 abstract 生成
  - 文本截断 + 关键词提取
  - Markdown-aware 提取
  - 代码块识别
- [x] 实现基于规则的 overview 生成
  - 子目录 abstract 聚合
  - 文件名模式识别
  - 结构化摘要模板
- [x] 单元测试

### 2.3 AGFS 自动摘要集成

```rust
// 扩展 FileSystem trait
pub trait FileSystem: Send + Sync {
    // ... 现有方法 ...

    /// 带上下文写入（L0/L1/L2）
    fn write_context(
        &self,
        path: &str,
        data: &[u8],
        abstract_: Option<&str>,
        overview: Option<&str>,
    ) -> Result<u64>;

    /// 读取抽象摘要
    fn read_abstract(&self, path: &str) -> Result<String>;

    /// 读取概述摘要
    fn read_overview(&self, path: &str) -> Result<String>;
}
```

**任务清单**：
- [x] 扩展 FileSystem trait
- [x] 实现 LocalFS 插件的 .abstract.md / .overview.md 写入
- [x] 实现 VikingFS.write_context() 方法
- [x] 集成摘要生成到写入流程
- [ ] 异步摘要生成队列 (可选)

### 2.4 自底向上聚合

```rust
// 目录摘要聚合器
pub struct DirectorySummaryAggregator {
    summary_provider: Arc<dyn SummaryProvider>,
    fs: Arc<dyn FileSystem>,
}

impl DirectorySummaryAggregator {
    /// 自底向上聚合
    pub async fn aggregate(&self, root_uri: &str) -> Result<()> {
        // 1. 处理叶子节点 -> 生成 L0
        // 2. 聚合子目录 L0 -> 生成父目录 L1
        // 3. 递归向上直到根目录
    }

    /// 叶子文件 -> L0
    async fn process_leaf(&self, uri: &str) -> Result<String>;

    /// 目录 L0 聚合 -> 父目录 L1
    async fn aggregate_to_parent(&self, dir_uri: &str) -> Result<()>;
}
```

**任务清单**：
- [x] 实现 DirectorySummaryAggregator
- [x] 实现叶子文件 L0 生成
- [x] 实现目录 L0 聚合为 L1
- [x] 实现递归向上聚合
- [x] 实现 VikingFS.commit() 方法
- [ ] 实现 Session 归档时的摘要生成
- [x] 集成测试

---

## Phase 3: VikingFS 虚拟文件系统 (P0)

> **目标**：实现 VikingFS Core，作为 AGFS 和 Vector Index 的统一抽象层

### 3.1 VikingFS Core 实现

```rust
/// VikingFS - 虚拟文件系统核心
pub struct VikingFS {
    /// AGFS 内容存储
    agfs: Arc<MountableFS>,
    /// 向量存储
    vector_store: Arc<dyn VectorStore>,
    /// 向量同步管理器
    vector_sync: Arc<VectorSyncManager>,
    /// 摘要生成器
    summary_provider: Arc<dyn SummaryProvider>,
    /// Embedding 提供者
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl VikingFS {
    // ========== 文件操作 ==========

    /// 读取文件内容
    pub async fn read(&self, uri: &str) -> Result<String>;

    /// 写入文件
    pub async fn write(&self, uri: &str, data: &str) -> Result<()>;

    /// 创建目录
    pub async fn mkdir(&self, uri: &str) -> Result<()>;

    /// 删除文件/目录
    pub async fn rm(&self, uri: &str, recursive: bool) -> Result<()>;

    /// 移动/重命名
    pub async fn mv(&self, from: &str, to: &str) -> Result<()>;

    /// 列出目录
    pub async fn ls(&self, uri: &str) -> Result<Vec<FileInfo>>;

    /// 获取文件信息
    pub async fn stat(&self, uri: &str) -> Result<FileInfo>;

    // ========== L0/L1/L2 操作 ==========

    /// 读取抽象摘要 (L0)
    pub async fn abstract_(&self, uri: &str) -> Result<String>;

    /// 读取概述摘要 (L1)
    pub async fn overview(&self, uri: &str) -> Result<String>;

    /// 读取完整内容 (L2)
    pub async fn detail(&self, uri: &str) -> Result<String>;

    /// 写入上下文（L0/L1/L2 一起）
    pub async fn write_context(
        &self,
        uri: &str,
        data: &str,
        auto_generate_summary: bool,
    ) -> Result<WriteContextResult>;

    /// 提交会话（触发聚合）
    pub async fn commit(&self, session_uri: &str) -> Result<()>;

    // ========== 关联管理 ==========

    /// 创建关联
    pub async fn link(
        &self,
        from_uri: &str,
        to_uri: &str,
        reason: &str,
    ) -> Result<()>;

    /// 获取关联列表
    pub async fn relations(&self, uri: &str) -> Result<Vec<Relation>>;

    // ========== 搜索 ==========

    /// 语义搜索
    pub async fn find(
        &self,
        query: &str,
        target_uri: Option<&str>,
        level: Option<u8>,
        limit: usize,
    ) -> Result<FindResult>;

    /// 向量搜索
    pub async fn search(
        &self,
        collection: &str,
        query_vector: &[f32],
        k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>>;
}
```

**任务清单**：
- [x] 实现 VikingFS 结构体
- [x] 实现文件操作方法 (read/write/mkdir/rm/mv/ls/stat)
- [x] 实现 L0/L1/L2 读写方法
- [x] 实现 write_context 自动摘要生成
- [ ] 实现关联管理 (link/relations)
- [x] 实现搜索方法 (find/search)
- [x] 集成测试

### 3.2 VikingURI 增强

```rust
impl VikingUri {
    /// 支持相对路径
    pub fn resolve(&self, base: &VikingUri, relative: &str) -> Result<VikingUri>;

    /// 路径规范化
    pub fn normalize(&self) -> VikingUri;

    /// 通配符匹配
    pub fn matches_pattern(&self, pattern: &str) -> bool;

    /// 转换为内部路径
    pub fn to_internal_path(&self) -> String;

    /// 获取父目录 URI
    pub fn parent(&self) -> Option<VikingUri>;
}
```

**任务清单**：
- [x] 扩展 VikingUri 解析
- [x] 实现相对路径支持
- [x] 实现路径规范化
- [ ] 实现通配符匹配
- [x] 单元测试

### 3.3 搜索集成

```rust
impl VikingFS {
    /// 语义搜索
    pub async fn find(
        &self,
        query: &str,
        target_uri: Option<&str>,
        level: Option<u8>,
        limit: usize,
    ) -> Result<FindResult> {
        // 1. 生成 query embedding
        let embedding = self.embedding_provider.embed(query)?;

        // 2. 向量检索
        let results = self.vector_store.search(
            "viking",
            &embedding,
            limit,
            filters,
        )?;

        // 3. 获取 L0 abstract
        let results = self.enrich_with_abstract(results).await?;

        Ok(FindResult { results })
    }
}
```

**任务清单**：
- [x] 实现 VikingFS::find() 方法
- [x] 实现向量相似度检索
- [x] 实现 URI 前缀过滤
- [x] 实现 Level 过滤
- [ ] 实现分页支持
- [x] 结果包含 L0 abstract
- [x] 集成测试

---

## Phase 4: CLI 完善 (P0)

> **目标**：提供完整的命令行接口

### 4.1 VikingFS 命令

```bash
# 文件操作
rustviking read <uri> [--level L0|L1|L2]
rustviking write <uri> <data>
rustviking mkdir <uri>
rustviking rm <uri> [--recursive]
rustviking mv <from> <to>
rustviking ls <uri> [--recursive]
rustviking stat <uri>

# L0/L1/L2 操作
rustviking abstract <uri>
rustviking overview <uri>
rustviking detail <uri>

# 关联操作
rustviking link <uri> <related-uri> [--reason <reason>]
rustviking relations <uri>

# 搜索操作
rustviking find <query> [--target <uri>] [--k N] [--level L0|L1|L2]
rustviking search <collection> <vector> [--k N] [--filter <json>]

# 会话操作
rustviking commit <session-uri>
```

**任务清单**：
- [x] 实现 read 命令
- [x] 实现 write 命令
- [x] 实现 mkdir 命令
- [x] 实现 rm 命令
- [x] 实现 mv 命令
- [x] 实现 ls 命令
- [x] 实现 stat 命令
- [x] 实现 abstract 命令
- [x] 实现 overview 命令
- [x] 实现 detail 命令
- [ ] 实现 link 命令
- [ ] 实现 relations 命令
- [x] 实现 find 命令
- [x] 实现 search 命令
- [x] 实现 commit 命令
- [x] 支持 VikingURI 和本地路径两种输入
- [x] 输出 JSON/Table/Plain 三种格式
- [x] 单元测试 + 集成测试

### 4.2 搜索命令增强

```bash
# 向量搜索
rustviking search viking --query 0.1,0.2,0.3 --k 10

# 文本查询自动 embedding
rustviking find "authentication methods" --k 10 --level L1

# 混合搜索 (dense + sparse)
rustviking find "oauth jwt" --hybrid --k 20
```

**任务清单**：
- [ ] 实现向量搜索 CLI
- [ ] 实现文本查询自动 embedding
- [ ] 实现混合搜索
- [ ] 实现过滤条件
- [ ] 集成测试

### 4.3 配置驱动的初始化

```toml
# config.toml

# 向量存储配置
[vector_store]
plugin = "rocksdb"
rocksdb.path = "./data/vector_store"

# Embedding 配置
[embedding]
plugin = "openai"
openai.api_key = "..."
openai.model = "text-embedding-3-small"
openai.dimension = 1536

# 摘要配置
[summary]
provider = "heuristic"  # 或 "openai", "ollama"

# 挂载点配置
[mount]
[[mount.points]]
path = "/local"
plugin = "localfs"
config = { base_path = "./data/local" }

[[mount.points]]
path = "/memory"
plugin = "memory"
```

**任务清单**：
- [x] 实现配置加载到 VikingFS
- [x] 实现插件动态注册
- [ ] 实现 mount/unmount 命令
- [x] 集成测试

---

## Phase 5: 测试与质量 (P1)

### 5.1 单元测试完善

**覆盖率目标：>80%**

| 模块 | 测试文件 | 覆盖内容 |
|------|---------|---------|
| agfs | agfs_test.rs | FileSystem trait、路由、URI 解析 |
| storage | kv_store_test.rs | RocksKvStore CRUD、边界条件 |
| index | index_test.rs | IvfIndex、HnswIndex、LayeredIndex |
| vector_store | vector_store_test.rs | 持久化、同步 |
| embedding | embedding_test.rs | Provider 实现 |
| summary | summary_test.rs | 启发式摘要生成 |
| vikingfs | vikingfs_test.rs | Core 方法 |

**任务清单**：
- [ ] AGFS 层测试 (>80% 覆盖)
- [ ] 存储层测试
- [ ] 索引层测试
- [ ] CLI 层测试
- [ ] 摘要层测试

### 5.2 集成测试

```rust
// 端到端测试
#[tokio::test]
async fn test_vikingfs_full_workflow() {
    // 1. 初始化
    let vikingfs = VikingFS::new().await;

    // 2. 写入文件
    vikingfs.write("viking://resources/docs/auth.md", "# Auth").await;

    // 3. 自动生成摘要
    vikingfs.commit("viking://resources/docs/").await;

    // 4. 验证摘要生成
    let abstract_ = vikingfs.abstract_("viking://resources/docs/auth.md").await;
    assert!(!abstract_.is_empty());

    // 5. 搜索
    let results = vikingfs.find("authentication", None, None, 10).await;
    assert!(!results.is_empty());

    // 6. 删除同步
    vikingfs.rm("viking://resources/docs/auth.md", false).await;
    let vector_exists = vector_store.get("viking://resources/docs/auth.md").await;
    assert!(vector_exists.is_err());
}
```

**任务清单**：
- [ ] VikingFS 端到端测试
- [ ] 配置测试
- [ ] 并发测试
- [ ] 错误处理测试

### 5.3 性能基准

```bash
# 运行基准测试
cargo bench

# KV 读写 QPS
Benchmark kv_bench: 50k+ writes/s, 80k+ reads/s

# 向量检索延迟
Benchmark vector_bench: <10ms P99

# IVF 索引性能
Benchmark ivf_bench: 训练/检索吞吐量

# HNSW 索引性能
Benchmark hnsw_bench: 训练/检索吞吐量

# 分层检索性能
Benchmark layered_bench: L0->L1->L2 渐进检索
```

**任务清单**：
- [ ] 实现 kv_bench
- [ ] 实现 vector_bench
- [ ] 实现 ivf_bench
- [ ] 实现 hnsw_bench
- [ ] 实现 layered_bench
- [ ] 生成性能报告

---

## Phase 6: 文档与发布 (P1)

### 6.1 API 文档

**任务清单**：
- [ ] 完善 rustdoc 注释
- [ ] 生成 API 文档
- [ ] 补充贡献指南
- [ ] 丰富使用示例

### 6.2 发布准备

**版本规划：0.2.0**

| 版本 | 特性 |
|------|------|
| 0.1.0 | 框架搭建、基础功能 |
| 0.2.0 | 索引持久化、L0/L1 摘要、VikingFS Core |
| 0.3.0 | 云存储插件 (S3)、HTTP 服务 |

**任务清单**：
- [ ] 版本号规划
- [ ] CHANGELOG 编写
- [ ] crates.io 发布
- [ ] GitHub Release

---

## 任务依赖图

```
Phase 1 (向量持久化)
     │
     ├─[1.1 IvfIndex持久化]──[1.2 HnswIndex持久化]──[1.3 向量同步]
     │
     └──────────────────────────────────────────────────────┐
                                                          │
Phase 2 (L0/L1摘要)                                       │
     │                                                   │
     ├─[2.1 SummaryProvider抽象]──[2.2 启发式实现]       │
     │                                                    │
     └─[2.3 AGFS集成]──[2.4 聚合]                        │
                                                          │
Phase 3 (VikingFS Core) ◄─────────────────────────────────┘
     │                                    ▲
     │                                    │
     ├─[3.1 VikingFS Core]───────────────┘
     │       │
     ├─[3.2 VikingURI增强]──┘
     │
     └─[3.3 搜索集成]

Phase 4 (CLI完善)
     │
     ├─[4.1 VikingFS命令]
     ├─[4.2 搜索命令]
     └─[4.3 配置驱动]

Phase 5 (测试质量)
     │
     ├─[5.1 单元测试]
     ├─[5.2 集成测试]
     └─[5.3 性能基准]

Phase 6 (文档发布)
     │
     └─[6.1+6.2]──► Release 0.2.0
```

---

## 目录结构更新

```
src/
├── agfs/                      # AGFS 虚拟文件系统
│   ├── filesystem.rs          # FileSystem trait
│   ├── metadata.rs            # 元数据
│   ├── mountable.rs           # Radix Tree 路由
│   ├── viking_uri.rs          # Viking URI 解析
│   └── mod.rs
│
├── vikingfs/                  # VikingFS (新增)
│   ├── mod.rs                 # VikingFS Core
│   ├── summary.rs             # L0/L1 摘要抽象
│   ├── aggregator.rs          # 自底向上聚合
│   ├── relation.rs            # 关系管理
│   └── sync.rs                # 向量同步
│
├── plugins/                   # 存储插件
│   ├── localfs.rs             # 本地文件系统
│   ├── memory.rs              # 内存存储
│   ├── mod.rs                 # 插件注册表
│   └── (s3fs.rs)             # S3 插件 (未来)
│
├── storage/                   # KV 存储
│   ├── kv.rs                  # KV trait
│   ├── rocks_kv.rs            # RocksDB 实现
│   └── config.rs              # 配置
│
├── index/                     # 索引层
│   ├── vector.rs              # VectorIndex trait
│   ├── ivf_pq.rs              # IVF-PQ 实现
│   ├── ivf_persist.rs         # IVF 持久化 (新增)
│   ├── hnsw.rs                # HNSW 实现
│   ├── hnsw_persist.rs        # HNSW 持久化 (新增)
│   ├── layered.rs             # 分层索引
│   ├── bitmap.rs              # 位图
│   └── mod.rs
│
├── vector_store/              # 向量存储
│   ├── traits.rs              # VectorStore trait
│   ├── memory.rs              # 内存实现
│   ├── rocks.rs              # RocksDB 实现
│   └── sync.rs               # 同步管理
│
├── embedding/                 # Embedding
│   ├── traits.rs             # EmbeddingProvider trait
│   ├── mock.rs               # Mock 实现
│   ├── openai.rs             # OpenAI 实现
│   ├── ollama.rs             # Ollama 实现
│   └── types.rs
│
├── summary/                   # 摘要生成 (新增)
│   ├── mod.rs
│   ├── provider.rs           # SummaryProvider trait
│   ├── heuristic.rs          # 启发式实现
│   └── llm.rs               # LLM 调用实现
│
├── cli/                       # CLI
│   ├── commands.rs           # 命令定义
│   ├── viking_commands.rs   # VikingFS 命令
│   ├── fs_commands.rs       # 文件系统命令
│   ├── store_commands.rs    # KV 命令
│   └── index_commands.rs    # 索引命令
│
├── compute/                   # 计算层
│   ├── distance.rs           # 距离计算
│   ├── normalize.rs          # 归一化
│   └── simd.rs              # SIMD
│
├── config/                    # 配置
│   └── loader.rs
│
├── error.rs                   # 错误定义
└── lib.rs                     # 库入口
```

---

## 立即开始

**建议从 Phase 1.1 开始**：

1. **IvfIndex RocksDB 持久化** - 这是所有生产使用的基础

### Phase 1.1 详细任务

```rust
// src/index/ivf_persist.rs (新增文件)

use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

/// IVF 索引持久化器
pub struct IvfIndexPersister {
    db: Arc<DB>,
}

impl IvfIndexPersister {
    /// 创建持久化器
    pub fn new(path: &Path, config: &IvfPersistConfig) -> Result<Self> {
        // 1. 配置列族
        let cfs = vec![
            ColumnFamilyDescriptor::new("centroids", Options::default()),
            ColumnFamilyDescriptor::new("vectors", Options::default()),
            ColumnFamilyDescriptor::new("metadata", Options::default()),
            ColumnFamilyDescriptor::new("config", Options::default()),
        ];

        // 2. 打开 RocksDB
        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self { db: Arc::new(db) })
    }

    /// 持久化中心点
    pub fn persist_centroids(&self, centroids: &[[f32; N]]) -> Result<()>;

    /// 持久化向量
    pub fn persist_vector(&self, partition: usize, id: u64, vector: &[f32]) -> Result<()>;

    /// 持久化元数据
    pub fn persist_metadata(&self, id: u64, meta: &VectorMetadata) -> Result<()>;

    /// 持久化配置
    pub fn persist_config(&self, config: &IvfConfig) -> Result<()>;

    /// 恢复中心点
    pub fn restore_centroids(&self) -> Result<Vec<Vec<f32>>>;

    /// 恢复向量
    pub fn restore_vectors(&self, partition: usize) -> Result<Vec<(u64, Vec<f32>)>>;

    /// 恢复元数据
    pub fn restore_metadata(&self, id: u64) -> Result<Option<VectorMetadata>>;

    /// 恢复配置
    pub fn restore_config(&self) -> Result<Option<IvfConfig>>;
}
```

---

*文档版本: 0.2.0*
*创建日期: 2026-03-29*
