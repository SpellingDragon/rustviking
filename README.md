# RustViking

> **OpenViking Core in Rust** — 高性能、命令行优先的 AI Agent 记忆基础设施

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

## 项目状态

**实验性项目** — 本项目是对 OpenViking 核心概念的 Rust 实现探索，目前处于早期开发阶段。

### 已实现功能

| 功能 | 状态 | 说明 |
|------|------|------|
| **AGFS 虚拟文件系统** | ✅ 基本完成 | 通过 `viking://` URI 访问的统一文件系统抽象 |
| **RocksDB KV 存储** | ✅ 生产可用 | 基于 RocksDB 的持久化键值存储 |
| **HNSW 向量索引** | ✅ 基本完成 | 使用 hnsw_rs 成熟库实现 |
| **IVF 向量索引** | ⚠️ 基础实现 | 简单的 IVF 聚类索引（不含 PQ） |
| **分层索引 L0/L1/L2** | ⚠️ 概念实现 | 支持按层级过滤检索 |
| **OpenAI Embedding** | ✅ 基本完成 | 支持兼容 OpenAI API 的嵌入服务 |
| **CLI 命令** | ✅ 基本完成 | 文件系统、KV、索引操作命令 |

### 与 OpenViking 的差异

OpenViking 是字节跳动开源的生产级 AI Agent 上下文数据库，包含完整的：
- 意图分析
- 分层检索与重排序
- 会话管理与记忆提取
- 文档解析与 LLM 集成

**RustViking 当前仅实现了 OpenViking 的存储层基础组件**，不包含上述高级功能。如需生产级 AI Agent 记忆系统，建议直接使用 [OpenViking](https://github.com/volcengine/OpenViking)。

---

## 快速开始

### 环境要求

- **Rust**: 1.82 或更高版本
- **操作系统**: macOS 10.15+ / Linux (Ubuntu 20.04+) / Windows (WSL2)

### 编译

```bash
# 克隆仓库
git clone https://github.com/SpellingDragon/rustviking.git
cd rustviking

# Debug 模式（开发）
cargo build

# Release 模式（生产，推荐）
cargo build --release
```

### 基础使用

```bash
# 查看帮助
./target/release/rustviking --help

# 文件系统操作
./rustviking fs mkdir viking://resources/project/docs
./rustviking fs write viking://resources/doc.md --data "Hello, RustViking!"
./rustviking fs cat viking://resources/doc.md

# 键值存储操作
./rustviking kv put --key "user:1:name" --value "Alice"
./rustviking kv get --key "user:1:name"

# 向量索引操作
./rustviking index insert --id 1 --vector 0.1,0.2,0.3,0.4 --level 2
./rustviking index search --query 0.1,0.2,0.3,0.4 --k 10
```

---

## 架构概览

```
┌─────────────────────────────────────────────────────┐
│                    CLI Commands                      │
├─────────────────────────────────────────────────────┤
│                  AGFS 虚拟文件系统                   │
│         (Radix Tree 路由 + 多后端挂载)               │
├──────────────┬──────────────┬───────────────────────┤
│   LocalFS    │  MemoryFS    │      VectorStore      │
│   (本地)     │   (内存)     │  (RocksDB/Memory)     │
├──────────────┴──────────────┴───────────────────────┤
│                 存储层 (RocksDB)                     │
│              向量索引 (HNSW/IVF)                     │
└─────────────────────────────────────────────────────┘
```

---

## CLI 命令速查表

### 文件系统命令

| 命令 | 描述 | 示例 |
|------|------|------|
| `fs mkdir` | 创建目录 | `rustviking fs mkdir viking://resources/project/docs` |
| `fs ls` | 列出目录 | `rustviking fs ls viking://resources/project/` |
| `fs cat` | 读取文件 | `rustviking fs cat viking://resources/doc.md` |
| `fs write` | 写入文件 | `rustviking fs write viking://resources/doc.md --data "..."` |
| `fs rm` | 删除文件/目录 | `rustviking fs rm viking://resources/doc.md` |

### 键值存储命令

| 命令 | 描述 | 示例 |
|------|------|------|
| `kv get` | 获取值 | `rustviking kv get --key "user:1:name"` |
| `kv put` | 设置键值 | `rustviking kv put --key "user:1:name" --value "Alice"` |
| `kv del` | 删除键 | `rustviking kv del --key "user:1:name"` |
| `kv scan` | 前缀扫描 | `rustviking kv scan --prefix "user:" --limit 100` |

### 索引命令

| 命令 | 描述 | 示例 |
|------|------|------|
| `index insert` | 插入向量 | `rustviking index insert --id 1 --vector 0.1,0.2 --level 2` |
| `index search` | 向量搜索 | `rustviking index search --query 0.1,0.2 --k 10` |
| `index delete` | 删除向量 | `rustviking index delete --id 1` |
| `index info` | 索引信息 | `rustviking index info` |

---

## 致谢 OpenViking

RustViking 的诞生，源于对 **[OpenViking](https://github.com/volcengine/OpenViking)** 深深的敬意与热爱。

| 维度 | OpenViking | RustViking |
|------|-----------|------------|
| **语言** | Go + Python + C++ | 纯 Rust |
| **交互方式** | HTTP/gRPC 服务 | **命令行优先** |
| **定位** | 完整 Agent 平台 | 存储层基础组件 |
| **成熟度** | 生产级 | 实验性 |

特别感谢 OpenViking 团队的开源贡献！

---

## 贡献指引

欢迎所有形式的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何：

- 提交 Issue 和 Feature Request
- 设置开发环境
- 提交 Pull Request

---

## License

RustViking 采用 [Apache-2.0](LICENSE) 许可证开源。
