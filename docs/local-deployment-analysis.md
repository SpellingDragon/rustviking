# RustViking 本地化运行与本地 Embedding 方案分析

## 一、项目当前本地化支持评估

### 1.1 存储层：✅ 完全支持本地运行

**现状**：

```toml
# config.toml
[storage]
path = "./data/rustviking"  # 本地路径

[vector_store]
plugin = "rocksdb"  # 本地 KV 存储
```

**支持情况**：

| 存储方案 | 本地支持 | 实现状态 | 说明 |
|---------|---------|---------|------|
| **LocalFS** | 完全支持 | 已实现 | `/local` 挂载点 |
| **Memory** | 完全支持 | 已实现 | 仅内存，重启丢失 |
| **RocksDB** | 完全支持 | 已实现 | 本地持久化存储 |
| **Qdrant** | 需要服务 | 已有接口 | 需要额外部署 Qdrant |

**LocalFS 实现**：

```rust
// src/plugins/localfs.rs
pub struct LocalFSPlugin {
    root: PathBuf,
}

impl FileSystem for LocalFSPlugin {
    fn read(&self, path: &str, offset: i64, size: u64) -> Result<Vec<u8>> {
        let full_path = self.root.join(path.trim_start_matches('/'));
        let data = std::fs::read(&full_path)?;
        Ok(data)
    }
    
    fn write(&self, path: &str, data: &[u8], offset: i64, flags: WriteFlag) -> Result<u64> {
        let full_path = self.root.join(path.trim_start_matches('/'));
        
        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(&full_path, data)?;
        Ok(data.len() as u64)
    }
}
```

**部署模式**：

```bash
# 完全本地运行模式
./rustviking mkdir viking://local/project
./rustviking write viking://local/doc.md "Hello World"
./rustviking read viking://local/doc.md
./rustviking find "search query"
```

**优点**：
- 无需网络连接
- 数据完全私有
- 零额外成本
- 无延迟（本地 IO）

**缺点**：
- 无法跨设备同步
- 单机存储上限
- 无法远程访问

### 1.2 向量索引层：本地索引完全支持

**支持的索引类型**：

| 索引类型 | 本地支持 | 内存需求 | 适用规模 | 说明 |
|---------|---------|---------|---------|------|
| **HNSW** | 是 | 5-10 GB/百万 | 百万级 | 高精度，内存密集 |
| **IVF** | 是 | 2-5 GB/百万 | 千万级 | 平衡精度和内存 |
| **IVF+PQ** | 是 | 200-500 MB/百万 | 亿级 | 压缩率高，精度略低 |

**配置示例**：

```toml
[vector]
dimension = 768
index_type = "ivf_pq"  # 压缩率最高的方案

[vector.ivf_pq]
num_partitions = 256      # 聚类数
num_sub_vectors = 16      # PQ 子空间数
pq_bits = 8              # 每个子空间 8 位
metric = "l2"            # 距离度量
```

**完全本地化的向量检索**：

```rust
// 场景：100 万条对话记录，完全本地存储和检索
let config = Config {
    storage: StorageConfig {
        path: "./data".to_string(),
        create_if_missing: true,
        ..Default::default()
    },
    vector_store: VectorStoreConfig {
        plugin: "rocksdb".to_string(),  // 本地 RocksDB
        ..Default::default()
    },
    embedding: EmbeddingConfig {
        plugin: "ollama".to_string(),   // 本地 Ollama（需要实现）
        ..Default::default()
    },
};

let vikingfs = VikingFS::from_config(&config).await?;
```

### 1.3 Embedding 层：需要补充实现

**当前状态**：

| Provider | 实现状态 | 网络要求 | 本地支持 | 说明 |
|---------|---------|---------|---------|------|
| **Mock** | 已实现 | 不需要 | 完全支持 | 仅用于测试，输出随机向量 |
| **OpenAI** | 已实现 | 需要 | 不支持 | 需要 OpenAI API Key |
| **Ollama** | 配置预留 | 可选 | 推荐 | 本地 embedding，需要实现 |

**缺失的功能**：

```toml
# config.toml.example 中已预留，但代码未实现
[embedding]
plugin = "ollama"  # 这个选项还不工作！

[embedding.ollama]
url = "http://localhost:11434"
model = "nomic-embed-text"
dimension = 768
max_concurrent = 5
```

---

## 二、本地 Embedding 技术方案对比

### 2.1 主流本地 Embedding 模型方案

#### 方案 1：Ollama（推荐 ⭐⭐⭐⭐⭐）

**概述**：
- 最流行的本地大模型运行工具
- 一键安装，自动下载模型
- REST API 设计，与 OpenAI API 兼容
- 支持 Embedding、LLM、视觉模型

**优势**：

```bash
# 安装（macOS/Linux）
curl -fsSL https://ollama.com/install.sh | sh

# 启动服务
ollama serve

# 下载 Embedding 模型
ollama pull nomic-embed-text    # 768 维
ollama pull mxbai-embed-large  # 1024 维，高精度
ollama pull all-minilm         # 384 维，轻量快速

# 测试
curl http://localhost:11434/api/embeddings -d '{
  "model": "nomic-embed-text",
  "prompt": "The quick brown fox"
}'
```

**Rust 集成实现**：

```rust
// src/embedding/ollama.rs（需要新建）

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OllamaEmbeddingProvider {
    url: String,
    model: String,
    dimension: usize,
    client: Client,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResult> {
        // Ollama 一次只支持一个文本，需要循环
        let mut embeddings = Vec::new();
        
        for text in &request.texts {
            let response = self.client
                .post(&format!("{}/api/embeddings", self.url))
                .json(&OllamaRequest {
                    model: self.model.clone(),
                    prompt: text.clone(),
                })
                .send()
                .await?;
            
            let result: OllamaResponse = response.json().await?;
            embeddings.push(result.embedding);
        }
        
        Ok(EmbeddingResult { embeddings })
    }
    
    async fn embed_batch(&self, requests: Vec<EmbeddingRequest>, max_concurrent: usize) -> Result<Vec<EmbeddingResult>> {
        // 实现批量并发（限制并发数）
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut tasks = Vec::new();
        
        for request in requests {
            let sem = semaphore.clone();
            let provider = self.clone();
            
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await?;
                provider.embed(request).await
            });
            tasks.push(task);
        }
        
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|r| r?)
            .collect()
    }
}
```

**性能数据**：

| 模型 | 维度 | 内存占用 | 推理速度 | 精度 |
|------|------|---------|---------|------|
| nomic-embed-text | 768 | 2.7 GB | ~100 tokens/s | 优秀 |
| mxbai-embed-large | 1024 | 4.1 GB | ~50 tokens/s | 极佳 |
| all-minilm | 384 | 1.3 GB | ~300 tokens/s | 良好 |

**适用场景**：
- ✅ 个人/小团队使用
- ✅ 数据隐私敏感场景
- ✅ 离线环境
- ✅ 成本敏感场景

#### 方案 2：llama.rs（高性能 ⭐⭐⭐⭐）

**概述**：
- 纯 Rust 实现的 LLM 推理引擎
- 零外部依赖，高度可移植
- 支持 GGUF 格式模型
- 比 Python 实现快 2-5 倍

**Rust 集成示例**：

```rust
// Cargo.toml
[dependencies]
llama-rs = "0.2"

// src/embedding/llama.rs（需要新建）

use llama_rs::{Llama, ModelParameters};

pub struct LlamaEmbeddingProvider {
    llama: Llama,
    tokenizer: Tokenizer,
    dimension: usize,
}

impl LlamaEmbeddingProvider {
    pub fn new(model_path: &str, dimension: usize) -> Result<Self> {
        let params = ModelParameters {
            use_mmap: true,        // 内存映射，节省内存
            use_mlock: false,      // 不锁定内存
            n_threads: Some(8),     // CPU 线程数
            ..Default::default()
        };
        
        let llama = Llama::load(model_path, params)?;
        Ok(Self { llama, dimension })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 1. 分词
        let tokens = self.tokenizer.encode(text);
        
        // 2. 模型推理（取最后一层隐藏状态）
        let hidden_states = self.llama.forward(&tokens);
        
        // 3. Mean Pooling
        let embedding = hidden_states
            .iter()
            .fold(vec![0.0; self.dimension], |mut acc, layer| {
                for (i, val) in layer.iter().enumerate().take(self.dimension) {
                    acc[i] += val;
                }
                acc
            });
        
        // 4. L2 归一化
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let embedding = embedding.iter().map(|x| x / norm).collect();
        
        Ok(embedding)
    }
}
```

**支持的 Embedding 模型**：

| 模型 | 格式 | 维度 | 量化支持 |
|------|------|------|---------|
| **mxbai-embed-large** | GGUF | 1024 | Q4_K_M, Q5_K_M |
| **nomic-embed-text** | GGUF | 768 | Q4_K_M, Q5_K_M |
| **bge** 系列 | GGUF | 768/1024 | Q4_K_M |

**性能对比**：

| 方案 | 推理速度 | 内存占用 | CPU 利用率 | 启动时间 |
|------|---------|---------|-----------|---------|
| Ollama | ~100 tok/s | 3-5 GB | 30-50% | 5-10s |
| llama.rs | ~200 tok/s | 2-4 GB | 50-80% | 1-3s |

**适用场景**：
- ✅ 需要高性能推理
- ✅ 嵌入式部署
- ✅ 无 Docker 环境
- ✅ 自定义推理逻辑

#### 方案 3：Candle（超轻量 ⭐⭐⭐）

**概述**：
- Rust 原生的 ML 框架
- 极其轻量，启动快
- 支持 GPU 加速（CUDA/Metal）
- Meta 开源的 Candle 库

**Rust 集成示例**：

```rust
// Cargo.toml
[dependencies]
candle-core = "0.6"
candle-transformers = "0.6"

use candle_core::{Device, Tensor};
use candle_transformers::models::bert::BertModel;

pub struct CandleEmbeddingProvider {
    model: BertModel,
    tokenizer: BertTokenizer,
    device: Device,
}

impl CandleEmbeddingProvider {
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 1. Tokenize
        let tokens = self.tokenizer.encode(text);
        
        // 2. 运行模型
        let embeddings = self.model.forward(&tokens)?;
        
        // 3. Mean Pooling
        let mean = embeddings.mean(1)?;
        
        Ok(mean.to_vec()?)
    }
}
```

**优势**：
- ✅ 启动极快（< 1s）
- ✅ 内存占用最低（~1-2 GB）
- ✅ Rust 原生，无 CGO
- ✅ 支持 GPU 加速

**劣势**：
- ❌ 模型支持较少
- ❌ 社区相对较小
- ❌ 需要自行实现 tokenization

#### 方案 4：ONNX Runtime（跨平台 ⭐⭐⭐⭐）

**概述**：
- Microsoft 开源的跨平台 ML 推理引擎
- 支持 ONNX 格式的所有模型
- 高度优化，自动选择最佳执行路径
- 支持 CPU/GPU/NPU

**Rust 集成**：

```rust
// Cargo.toml
[dependencies]
onnxruntime = "0.2"

use onnxruntime::{Environment, LoggingLevel};
use std::path::Path;

pub struct OnnxEmbeddingProvider {
    session: Session,
    input_name: String,
    dimension: usize,
}

impl OnnxEmbeddingProvider {
    pub fn new(model_path: &Path) -> Result<Self> {
        let environment = Environment::new(LoggingLevel::Warning)?;
        let session = environment.new_session_builder()?
            .with_model(model_path)?;
        
        Ok(Self {
            session,
            input_name: "input_ids".to_string(),
            dimension: 768,
        })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // 1. Tokenize（需要 Rust tokenizer）
        let input_ids = tokenize_bert(text, self.dimension);
        
        // 2. 运行 ONNX 模型
        let output = self.session.run([
            (&self.input_name, input_ids),
        ])?;
        
        // 3. 提取 embedding
        Ok(output[0].as_slice()?.to_vec())
    }
}
```

**支持模型**：

| 模型 | ONNX 导出 | 精度 | 推理速度 |
|------|----------|------|---------|
| **sentence-transformers** | ✅ 可导出 | 优秀 | 快 |
| **Instructor** | ✅ 可导出 | 极佳 | 中等 |
| **BGE** | ✅ 可导出 | 优秀 | 快 |

**适用场景**：
- ✅ 跨平台部署（Windows/Linux/macOS）
- ✅ 需要 GPU 加速
- ✅ 已有 ONNX 模型

### 2.2 技术方案综合对比

| 维度 | Ollama | llama.rs | Candle | ONNX Runtime |
|------|--------|---------|--------|--------------|
| **易用性** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| **性能** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **内存占用** | 3-5 GB | 2-4 GB | 1-2 GB | 2-4 GB |
| **安装复杂度** | 低（一键安装） | 中 | 高 | 中 |
| **模型生态** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ |
| **离线支持** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **自定义能力** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **社区活跃度** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

---

## 三、推荐技术方案

### 3.1 方案选型决策树

```
需要完全离线运行？
  │
  ├─ 是 → 数据量 < 10GB？
  │         │
  │         ├─ 是 → 推荐 Ollama（最简单）
  │         │
  │         └─ 否 → 需要高性能？
  │                   │
  │                   ├─ 是 → 推荐 llama.rs + 量化模型
  │                   │
  │                   └─ 否 → 推荐 Ollama
  │
  └─ 否 → 有 GPU？
          │
          ├─ 是 → 推荐 ONNX Runtime（GPU 加速）
          │
          └─ 否 → 推荐 Ollama（CPU 优化好）
```

### 3.2 推荐方案详解

#### 🌟 方案 A：Ollama（最适合大多数场景）

**适用人群**：
- 不想折腾，追求开箱即用
- 数据量中等（< 100 万文档）
- 个人开发者或小团队
- 隐私敏感但不追求极致性能

**部署步骤**：

```bash
# 1. 安装 Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 2. 下载模型
ollama pull nomic-embed-text
ollama pull all-minilm-l6-v2  # 轻量快速版

# 3. 启动服务
ollama serve

# 4. 配置 RustViking
# config.toml
[embedding]
plugin = "ollama"  # 需要实现

[embedding.ollama]
url = "http://localhost:11434"
model = "nomic-embed-text"
dimension = 768
max_concurrent = 5

# 5. 运行
cargo run --release
```

**实现优先级**：⭐⭐⭐⭐⭐（最高）

---

#### 🌟🌟 方案 B：llama.rs + 量化模型（高性能场景）

**适用人群**：
- 需要极致推理性能
- 有一定 Rust 经验
- 嵌入式或边缘部署
- 愿意深度定制

**部署步骤**：

```bash
# 1. 下载量化模型（GGUF 格式）
wget https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/main/nomic-embed-text-v1.5-Q4_K_M.gguf

# 2. 配置模型路径
mv nomic-embed-text-v1.5-Q4_K_M.gguf ./models/

# 3. Rust 代码使用
```

```rust
// src/embedding/llama_embedding.rs

pub struct LlamaEmbeddingProvider {
    model: LlamaModel,
    tokenizer: Tokenizer,
}

impl LlamaEmbeddingProvider {
    pub fn new(model_path: &str) -> Result<Self> {
        // 加载模型（支持 MMAP 节省内存）
        let model = LlamaModel::load(model_path, &LlamaParameters {
            n_ctx: 512,           // 上下文长度
            use_mmap: true,       // 内存映射
            n_threads: 8,         // CPU 线程
            ..Default::default()
        })?;
        
        Ok(Self { model })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text);
        
        // 获取 embedding（使用 last hidden state）
        let embedding = self.model.get_embeddings(&tokens)?;
        
        // Mean pooling + L2 normalize
        Ok(normalize(mean_pooling(embedding)))
    }
}
```

**量化模型选择**：

| 量化等级 | 压缩率 | 内存占用 | 精度损失 | 推荐场景 |
|---------|--------|---------|---------|---------|
| **Q2_K** | 75% | ~1 GB | ~5% | 极致内存 |
| **Q4_K_M** | 62% | ~1.5 GB | ~2% | ⭐推荐 |
| **Q5_K_M** | 56% | ~1.8 GB | ~1% | 高精度 |
| **Q8_0** | 50% | ~2.5 GB | <0.5% | 几乎无损 |

**实现优先级**：⭐⭐⭐⭐（高）

---

#### 🌟🌟🌟 方案 C：Candle（超轻量/嵌入式）

**适用人群**：
- 资源极其受限（< 2GB RAM）
- 嵌入式系统部署
- 需要最快启动时间
- 愿意深入定制

**性能指标**：

| 模型 | 量化 | 内存 | 启动 | 推理速度 |
|------|------|------|------|---------|
| MiniLM | Q4 | 1.1 GB | 0.5s | 300 tok/s |
| nomic-embed | Q4 | 1.8 GB | 1.2s | 120 tok/s |

**实现优先级**：⭐⭐⭐（中）

---

### 3.3 Ollama 实现详细方案

由于 Ollama 是推荐方案，让我提供完整的实现代码：

#### 步骤 1：创建 Ollama Provider 模块

```rust
// src/embedding/ollama.rs

//! Ollama 本地 Embedding Provider
//!
//! 支持本地运行的 Ollama 服务，无需网络调用 OpenAI API

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use super::traits::EmbeddingProvider;
use super::types::{EmbeddingConfig, EmbeddingRequest, EmbeddingResult};
use crate::error::{Result, RustVikingError};

/// Ollama Embedding Provider 配置
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Ollama 服务地址
    pub url: String,
    /// 模型名称
    pub model: String,
    /// Embedding 向量维度
    pub dimension: usize,
    /// 最大并发请求数
    pub max_concurrent: usize,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
            dimension: 768,
            max_concurrent: 5,
        }
    }
}

/// Ollama API 请求体
#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

/// Ollama API 响应
#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

/// Ollama API 列表模型响应
#[derive(Debug, Deserialize)]
struct OllamaListResponse {
    models: Vec<OllamaModelInfo>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    name: String,
}

/// Ollama Embedding Provider
#[derive(Debug, Clone)]
pub struct OllamaEmbeddingProvider {
    config: OllamaConfig,
    client: Client,
}

impl OllamaEmbeddingProvider {
    /// 创建新的 Ollama Provider
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        Self { config, client }
    }
    
    /// 从配置创建
    pub fn from_config(embedding_config: EmbeddingConfig) -> Result<Self> {
        let ollama_config = OllamaConfig {
            url: embedding_config.api_base,
            model: embedding_config.model,
            dimension: embedding_config.dimension,
            max_concurrent: embedding_config.max_concurrent,
        };
        
        Ok(Self::new(ollama_config))
    }
    
    /// 检查 Ollama 服务是否可用
    pub async fn health_check(&self) -> Result<bool> {
        match self.client
            .get(format!("{}/api/tags", self.config.url))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    
    /// 列出可用的模型
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let response = self.client
            .get(format!("{}/api/tags", self.config.url))
            .send()
            .await
            .map_err(|e| RustVikingError::Internal(format!("Failed to list models: {}", e)))?;
        
        let result: OllamaListResponse = response
            .json()
            .await
            .map_err(|e| RustVikingError::Internal(format!("Failed to parse models: {}", e)))?;
        
        Ok(result.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn name(&self) -> &str {
        "ollama"
    }
    
    fn version(&self) -> &str {
        "0.1.0"
    }
    
    async fn initialize(&self, _config: EmbeddingConfig) -> Result<()> {
        // Ollama 不需要额外的初始化
        // 连接会在首次请求时建立
        Ok(())
    }
    
    /// 生成单个 Embedding
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResult> {
        let mut embeddings = Vec::with_capacity(request.texts.len());
        
        for text in request.texts {
            let response = self.client
                .post(format!("{}/api/embeddings", self.config.url))
                .json(&OllamaEmbeddingRequest {
                    model: self.config.model.clone(),
                    prompt: text,
                })
                .send()
                .await
                .map_err(|e| {
                    RustVikingError::Internal(format!("Ollama API request failed: {}", e))
                })?;
            
            if !response.status().is_success() {
                return Err(RustVikingError::Internal(format!(
                    "Ollama API returned error: {}",
                    response.status()
                )));
            }
            
            let result: OllamaEmbeddingResponse = response
                .json()
                .await
                .map_err(|e| {
                    RustVikingError::Internal(format!("Failed to parse Ollama response: {}", e))
                })?;
            
            embeddings.push(result.embedding);
        }
        
        Ok(EmbeddingResult { embeddings })
    }
    
    /// 批量生成 Embedding（带并发控制）
    async fn embed_batch(
        &self,
        requests: Vec<EmbeddingRequest>,
        max_concurrent: usize,
    ) -> Result<Vec<EmbeddingResult>> {
        use tokio::sync::Semaphore;
        
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let provider = self.clone();
        
        let tasks: Vec<_> = requests
            .into_iter()
            .map(|request| {
                let sem = semaphore.clone();
                let provider = provider.clone();
                
                tokio::spawn(async move {
                    let _permit = sem.acquire().await
                        .map_err(|e| RustVikingError::Internal(e.to_string()))?;
                    provider.embed(request).await
                })
            })
            .collect();
        
        let results = futures::future::join_all(tasks).await;
        
        results
            .into_iter()
            .map(|r| r.map_err(|e| RustVikingError::Internal(e.to_string()))?)
            .collect()
    }
    
    fn default_dimension(&self) -> usize {
        self.config.dimension
    }
    
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "nomic-embed-text",
            "mxbai-embed-large",
            "all-minilm-l6-v2",
            "bge-m3",
        ]
    }
}
```

#### 步骤 2：在 mod.rs 中导出

```rust
// src/embedding/mod.rs

pub mod mock;
pub mod ollama;  // 新增
pub mod openai;
pub mod traits;
pub mod types;

pub use mock::MockEmbeddingProvider;
pub use ollama::OllamaEmbeddingProvider;  // 新增
pub use openai::OpenAIEmbeddingProvider;
pub use traits::EmbeddingProvider;
pub use types::*;
```

#### 步骤 3：在配置加载中添加 Ollama 支持

```rust
// src/config/loader.rs

// 在 create_embedding_provider 函数中添加：

match embedding_config.plugin.as_str() {
    "mock" => {
        let dimension = embedding_config
            .mock
            .as_ref()
            .map(|m| m.dimension)
            .unwrap_or(1024);
        let provider = MockEmbeddingProvider::new(dimension);
        provider.initialize(embedding_config.clone()).await?;
        Ok(Arc::new(provider))
    }
    "openai" => {
        let provider = OpenAIEmbeddingProvider::new();
        if let Some(openai_config) = &embedding_config.openai {
            let config = EmbeddingConfig {
                api_base: openai_config.api_base.clone(),
                api_key: Some(openai_config.api_key.clone()),
                provider: "openai".to_string(),
                model: openai_config.model.clone(),
                dimension: openai_config.dimension,
                max_concurrent: openai_config.max_concurrent,
            };
            provider.initialize(config).await?;
        } else {
            return Err(ConfigError::MissingField(
                "OpenAI embedding requires 'embedding.openai' config".to_string(),
            ));
        }
        Ok(Arc::new(provider))
    }
    "ollama" => {  // 新增
        let provider = OllamaEmbeddingProvider::from_config(embedding_config.clone())?;
        Ok(Arc::new(provider))
    }
    _ => Err(ConfigError::UnknownPlugin(format!(
        "Unknown embedding plugin: {}",
        embedding_config.plugin
    ))),
}
```

---

## 四、完整本地化部署方案

### 4.1 完全离线部署架构

```
┌─────────────────────────────────────────────────────────┐
│                     RustViking                          │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  VikingFS    │  │  VectorStore │  │  Embedding   │   │
│  │  (AGFS)      │  │  (RocksDB)   │  │  (Ollama)    │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                  │           │
│         │                 │                  │           │
│         ▼                 ▼                  ▼           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  LocalFS     │  │  HNSW/IVF    │  │  Ollama     │   │
│  │  (文件系统)   │  │  (向量索引)   │  │  (本地模型)  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                          │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
              ┌─────────────────────────┐
              │    本地磁盘存储           │
              │  ./data/rustviking/      │
              │  - storage/              │
              │  - vector_store/         │
              │  - local/                │
              └─────────────────────────┘
```

### 4.2 部署配置文件

```toml
# config.toml - 完全本地化配置

# ============ 存储配置 ============
[storage]
path = "./data/rustviking"        # 本地数据目录
create_if_missing = true           # 自动创建
max_open_files = 10000            # RocksDB 性能调优
use_fsync = false                 # 关闭 fsync 提升性能（可丢失最后几秒数据）

# ============ 向量配置 ============
[vector]
dimension = 768                    # Embedding 维度
index_type = "hnsw"               # 使用 HNSW 高精度索引

[vector.hnsw]
m = 16                             # 邻居数
ef_construction = 200             # 建图精度
ef_search = 100                   # 搜索精度

# ============ 向量存储 ============
[vector_store]
plugin = "rocksdb"                # 本地 RocksDB

[vector_store.rocksdb]
path = "./data/rustviking/vector_store"

# ============ Embedding 配置 ============
[embedding]
plugin = "ollama"                 # 使用本地 Ollama

[embedding.ollama]
url = "http://localhost:11434"    # Ollama 服务地址
model = "nomic-embed-text"        # Embedding 模型
dimension = 768
max_concurrent = 5                 # 并发限制

# ============ 日志配置 ============
[logging]
level = "info"
format = "json"
output = "stdout"

# ============ AGFS 配置 ============
[agfs]
default_scope = "resources"
default_account = "default"
```

### 4.3 启动脚本

```bash
#!/bin/bash
# deploy-local.sh

set -e

echo "🚀 RustViking 本地部署脚本"
echo "=============================="

# 1. 检查 Ollama 是否安装
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama 未安装，正在安装..."
    curl -fsSL https://ollama.com/install.sh | sh
else
    echo "✅ Ollama 已安装: $(ollama --version)"
fi

# 2. 启动 Ollama 服务（后台）
echo "🔧 启动 Ollama 服务..."
ollama serve &
OLLAMA_PID=$!
sleep 3

# 3. 下载 Embedding 模型
echo "📥 下载 Embedding 模型..."
ollama pull nomic-embed-text
ollama pull all-minilm-l6-v2  # 可选：轻量版

# 4. 创建数据目录
echo "📁 创建数据目录..."
mkdir -p ./data/rustviking/{storage,vector_store,local}

# 5. 编译项目
echo "🔨 编译 RustViking..."
cargo build --release

# 6. 运行
echo "✅ 启动 RustViking..."
./target/release/rustviking --config config.toml

# 清理
trap "kill $OLLAMA_PID" EXIT
```

### 4.4 Docker Compose 完整方案

```yaml
# docker-compose.yml
version: '3.8'

services:
  rustviking:
    build: .
    ports:
      - "8080:8080"  # HTTP API（如果有）
    volumes:
      - ./data:/app/data
      - ./config.toml:/app/config.toml
    environment:
      - RUST_LOG=info
    depends_on:
      - ollama
    networks:
      - rustviking-net

  ollama:
    image: ollama/ollama:latest
    volumes:
      - ollama-models:/root/.ollama
    ports:
      - "11434:11434"
    networks:
      - rustviking-net
    deploy:
      resources:
        limits:
          memory: 8G
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]

volumes:
  ollama-models:

networks:
  rustviking-net:
    driver: bridge
```

---

## 五、性能与资源评估

### 5.1 本地部署资源需求

| 组件 | 内存占用 | CPU | 磁盘 | 说明 |
|------|---------|-----|------|------|
| **RustViking** | 1-2 GB | 2-4 核 | 100MB | 应用本身 |
| **Ollama (CPU)** | 3-5 GB | 4-8 核 | 3-5 GB | Embedding 模型 |
| **Ollama (GPU)** | 2-3 GB | 2 核 | 3-5 GB | Embedding 模型 |
| **RocksDB** | 1-2 GB | 1-2 核 | 视数据量 | 向量存储 |
| **HNSW 索引** | 5-10 GB | 2-4 核 | - | 百万向量 |

**总计（无 GPU）**：
- 内存：10-15 GB
- CPU：8-16 核
- 磁盘：10-50 GB（取决于数据量）

**总计（有 GPU）**：
- 内存：6-10 GB
- CPU：4-8 核
- GPU：4-8 GB VRAM
- 磁盘：10-50 GB

### 5.2 Embedding 生成速度对比

| 方案 | 单次延迟 | 批量吞吐 | 适用场景 |
|------|---------|---------|---------|
| **OpenAI API** | 200-500ms | 200-500/s | 云端，高精度 |
| **Ollama (CPU)** | 50-100ms | 50-100/s | 本地，快速迭代 |
| **llama.rs** | 20-50ms | 100-300/s | 本地，高性能 |
| **ONNX + GPU** | 5-15ms | 500-1000/s | 本地，生产环境 |

### 5.3 成本对比

| 方案 | 一次性成本 | 月度成本 | 说明 |
|------|-----------|---------|------|
| **OpenAI API** | $0 | $50-500 | 按 token 计费 |
| **Ollama (自托管)** | $500-2000 | $50-100 | 服务器成本 |
| **llama.rs (自托管)** | $500-2000 | $50-100 | 服务器成本 |

---

## 六、总结与建议

### 6.1 当前状态

| 模块 | 本地支持 | 实现状态 | 说明 |
|------|---------|---------|------|
| **存储层** | ✅ 完整 | ✅ 已完成 | LocalFS, RocksDB |
| **索引层** | ✅ 完整 | ✅ 已完成 | HNSW, IVF |
| **Embedding** | ⚠️ 部分 | ❌ 待实现 | 缺少 Ollama Provider |

### 6.2 推荐实现路径

**Phase 1：快速可用（1-2 天）**
1. ✅ 实现 Ollama Provider（最简单）
2. ✅ 适配配置加载
3. ✅ 测试基本流程

**Phase 2：性能优化（1 周）**
1. 添加模型缓存
2. 实现批量推理
3. 添加请求队列
4. 性能测试和调优

**Phase 3：生产就绪（2 周）**
1. 支持多种本地模型
2. 添加 GPU 加速支持
3. 实现模型热加载
4. 添加监控指标

### 6.3 最终推荐

**对于大多数用户**：
> 推荐使用 **Ollama**，因为：
> - 一键安装，自动下载模型
> - 与现有 OpenAI 接口高度兼容
> - 社区活跃，模型丰富
> - 零配置即可运行

**对于性能敏感用户**：
> 推荐使用 **llama.rs + 量化模型**，因为：
> - 纯 Rust，性能最优
> - 支持内存映射，资源占用低
> - 可深度定制
> - 无外部依赖

**下一步行动**：
1. 实现 `src/embedding/ollama.rs`
2. 更新配置加载逻辑
3. 添加集成测试
4. 更新文档

---

*文档生成时间：2026-03-29*
*RustViking 版本：main 分支最新*
