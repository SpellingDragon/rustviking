# RustViking CLI API 规范

> 版本: 0.1.0  
> 目标读者: trpcclaw 团队及外部集成开发者  
> 最后更新: 2026-04-04

---

## 1. 概述

### 1.1 设计理念

RustViking CLI 遵循以下核心设计原则：

| 特性 | 说明 |
|------|------|
| **无状态** | 每个命令独立执行，不维护守护进程状态 |
| **JSON 优先** | 默认输出格式为 JSON，便于程序解析 |
| **单二进制** | 单个可执行文件，无需额外依赖 |
| **语义化存储** | 原生支持 L0/L1/L2 三级摘要读取 |

### 1.2 适用场景

- **Python 子进程调用**: 通过 `subprocess.run()` 执行命令
- **Go 集成**: 使用 `os/exec` 包调用
- **Shell 脚本**: 直接命令行调用
- **CI/CD 流水线**: 自动化文档处理流程

---

## 2. 全局选项

全局选项必须放在子命令**之前**：

```bash
rustviking [GLOBAL_OPTIONS] <COMMAND> [ARGS]
```

### 2.1 选项列表

| 选项 | 简写 | 默认值 | 说明 |
|------|------|--------|------|
| `--config <FILE>` | `-c` | `config.toml` | 配置文件路径 |
| `--output <FORMAT>` | `-o` | `json` | 输出格式: `json` / `table` / `plain` |

### 2.2 使用示例

```bash
# 使用自定义配置文件
rustviking --config /etc/rustviking.toml read viking://docs/guide.md

# 使用 table 格式输出（人类可读）
rustviking --output table ls viking://resources

# 组合使用
rustviking -c prod.toml -o plain stat viking://data/file.txt
```

### 2.3 重要提示

**注意**: `--config` 是全局选项，必须放在子命令之前：

```bash
# ✅ 正确
rustviking --config my.toml read viking://file.md

# ❌ 错误 - config 会被当作 read 的参数
rustviking read --config my.toml viking://file.md
```

---

## 3. JSON 响应格式规范

### 3.1 成功响应

```json
{
  "success": true,
  "data": { ... }
}
```

### 3.2 失败响应

```json
{
  "success": false,
  "error": "错误描述信息"
}
```

### 3.3 输出约定

| 输出流 | 内容 |
|--------|------|
| **stdout** | 始终输出合法的 JSON（即使出错） |
| **stderr** | 人类可读日志信息（tracing 输出） |

### 3.4 解析示例

```python
import json
import subprocess

result = subprocess.run(
    ["rustviking", "stat", "viking://docs/readme.md"],
    capture_output=True,
    text=True
)

response = json.loads(result.stdout)
if response["success"]:
    print(f"File size: {response['data']['size']}")
else:
    print(f"Error: {response['error']}")
```

---

## 4. 退出码规范

| 退出码 | 类别 | 说明 |
|--------|------|------|
| `0` | 成功 | 命令执行成功 |
| `1` | 用户错误 | 参数错误、资源不存在、输入格式错误等 |
| `2` | 系统错误 | IO 错误、存储故障、内部错误等 |

### 4.1 错误分类详情

**用户错误 (exit code 1)**:
- `MountNotFound` - 挂载点不存在
- `InvalidDimension` - 向量维度不匹配
- `InvalidUri` - URI 格式错误
- `NotFound` - 资源不存在
- `AlreadyExists` - 资源已存在
- `PermissionDenied` - 权限不足
- `CollectionNotFound` - 集合不存在
- `PluginNotFound` - 插件未找到
- `CliInput` - CLI 输入无效

**系统错误 (exit code 2)**:
- `Agfs` / `Storage` / `RocksDb` - 存储层错误
- `Index` - 索引错误
- `Config` - 配置错误
- `Embedding` - Embedding 服务错误
- `VectorStore` - 向量存储错误
- `Io` - IO 错误
- `Serialization` - 序列化错误
- `Internal` - 内部错误
- `Summary` / `VikingFs` - 文件系统错误

---

## 5. VikingFS 命令（核心命令）

VikingFS 提供语义化文件系统操作，支持三级摘要（L0/L1/L2）。

### 5.1 read - 读取文件内容

读取文件内容，支持指定摘要级别。

**语法**:
```bash
rustviking read <URI> [-l <LEVEL>]
```

**参数**:
| 参数 | 说明 | 必填 |
|------|------|------|
| `URI` | Viking URI (如 `viking://resources/doc.md`) | 是 |
| `-l, --level` | 读取级别: `L0`, `L1`, `L2` | 否 |

**级别说明**:
- `L0` / `0` - 抽象摘要（一句话概括）
- `L1` / `1` - 概述摘要（段落级摘要）
- `L2` / `2` 或不指定 - 完整内容

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/guide.md",
    "level": "L1",
    "content": "本文档介绍了 RustViking 的核心概念..."
  }
}
```

**错误场景**:
- URI 不存在 → exit code 1
- 无效的 level 值 → 使用默认 L2

---

### 5.2 write - 写入文件

写入文件内容，自动触发 embedding 和索引。

**语法**:
```bash
rustviking write <URI> -d <DATA> [--auto-summary]
```

**参数**:
| 参数 | 说明 | 必填 |
|------|------|------|
| `URI` | 目标 Viking URI | 是 |
| `-d, --data` | 要写入的数据 | 是 |
| `--auto-summary` | 自动生成摘要 | 否 (默认 false) |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/new.md",
    "auto_summary": false,
    "bytes_written": 1024
  }
}
```

---

### 5.3 mkdir - 创建目录

创建新目录。

**语法**:
```bash
rustviking mkdir <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/newdir",
    "operation": "mkdir"
  }
}
```

---

### 5.4 rm - 删除文件/目录

删除文件或目录。

**语法**:
```bash
rustviking rm <URI> [-r]
```

**参数**:
| 参数 | 说明 | 必填 |
|------|------|------|
| `URI` | 要删除的资源 URI | 是 |
| `-r, --recursive` | 递归删除目录 | 否 (默认 false) |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/old.md",
    "recursive": false,
    "operation": "rm"
  }
}
```

---

### 5.5 mv - 移动/重命名

移动或重命名文件/目录。

**语法**:
```bash
rustviking mv <FROM_URI> <TO_URI>
```

**参数**:
| 参数 | 说明 | 必填 |
|------|------|------|
| `FROM_URI` | 源 URI | 是 |
| `TO_URI` | 目标 URI | 是 |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "from": "viking://resources/old.md",
    "to": "viking://resources/new.md",
    "operation": "mv"
  }
}
```

---

### 5.6 ls - 列出目录内容

列出目录中的文件和子目录。

**语法**:
```bash
rustviking ls <URI> [-r]
```

**参数**:
| 参数 | 说明 | 必填 |
|------|------|------|
| `URI` | 目录 URI | 是 |
| `-r, --recursive` | 递归列出 | 否 (默认 false) |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources",
    "entries": [
      {
        "name": "doc1.md",
        "size": 1024,
        "is_dir": false,
        "mode": "644",
        "created_at": 1712234567,
        "updated_at": 1712234567
      },
      {
        "name": "subdir",
        "size": 0,
        "is_dir": true,
        "mode": "755",
        "created_at": 1712234567,
        "updated_at": 1712234567
      }
    ]
  }
}
```

---

### 5.7 stat - 获取文件信息

获取文件或目录的详细信息。

**语法**:
```bash
rustviking stat <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/doc.md",
    "name": "doc.md",
    "size": 1024,
    "is_dir": false,
    "mode": "644",
    "created_at": 1712234567,
    "updated_at": 1712234567
  }
}
```

---

### 5.8 abstract - 读取 L0 摘要

读取文件的 L0 级抽象摘要（一句话）。

**语法**:
```bash
rustviking abstract <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/doc.md",
    "level": "L0",
    "abstract": "本文档是关于 RustViking CLI 的使用指南。"
  }
}
```

---

### 5.9 overview - 读取 L1 摘要

读取文件的 L1 级概述摘要（段落级）。

**语法**:
```bash
rustviking overview <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/doc.md",
    "level": "L1",
    "overview": "本文档详细介绍了 RustViking CLI 的所有命令..."
  }
}
```

---

### 5.10 detail - 读取 L2 完整内容

读取文件的完整内容（L2 级）。

**语法**:
```bash
rustviking detail <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources/doc.md",
    "level": "L2",
    "content": "# RustViking CLI 文档\n\n## 概述..."
  }
}
```

---

### 5.11 find - 语义搜索

基于文本查询进行语义搜索，自动进行 embedding。

**语法**:
```bash
rustviking find <QUERY> [-t <TARGET>] [-k <COUNT>] [-l <LEVEL>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `QUERY` | 搜索查询文本 | 必填 |
| `-t, --target` | 目标 URI 范围 | 无（全局搜索） |
| `-k` | 返回结果数量 | 10 |
| `-l, --level` | 搜索级别: L0, L1, L2 | 无 |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "query": "如何配置存储路径",
    "target": "viking://docs",
    "level": null,
    "results": [
      {
        "id": "doc_001",
        "uri": "viking://docs/config.md",
        "score": 0.9234,
        "level": 2,
        "abstract": "配置文件使用 TOML 格式..."
      },
      {
        "id": "doc_002",
        "uri": "viking://docs/quickstart.md",
        "score": 0.8543,
        "level": 2,
        "abstract": "快速开始指南..."
      }
    ]
  }
}
```

---

### 5.12 commit - 提交目录

触发目录的摘要聚合。

**语法**:
```bash
rustviking commit <URI>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "uri": "viking://resources",
    "operation": "commit"
  }
}
```

---

## 6. KV 存储命令

KV 存储提供键值对操作，基于 RocksDB 实现。

### 6.1 kv get - 获取值

**语法**:
```bash
rustviking kv get -k <KEY>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "key": "mykey",
    "value": "myvalue"
  }
}
```

Key 不存在时:
```json
{
  "success": true,
  "data": {
    "key": "mykey",
    "value": null
  }
}
```

---

### 6.2 kv put - 设置键值

**语法**:
```bash
rustviking kv put -k <KEY> -v <VALUE>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "put",
    "key": "mykey"
  }
}
```

---

### 6.3 kv del - 删除键

**语法**:
```bash
rustviking kv del -k <KEY>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "delete",
    "key": "mykey"
  }
}
```

---

### 6.4 kv scan - 前缀扫描

**语法**:
```bash
rustviking kv scan -p <PREFIX> [-l <LIMIT>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-p, --prefix` | 键前缀 | 必填 |
| `-l, --limit` | 最大返回数量 | 100 |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "prefix": "user:",
    "count": 2,
    "entries": [
      {"key": "user:001", "value": "Alice"},
      {"key": "user:002", "value": "Bob"}
    ]
  }
}
```

---

### 6.5 kv batch - 批量操作

执行批量 put/delete 操作，支持从文件或 stdin 读取。

**语法**:
```bash
rustviking kv batch [-f <FILE>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-f, --file` | JSON 文件路径，`-` 表示从 stdin 读取 | `-` |

**输入文件格式**:
```json
[
  {"op": "put", "key": "k1", "value": "v1"},
  {"op": "put", "key": "k2", "value": "v2"},
  {"op": "delete", "key": "k3"}
]
```

**操作类型**:
| 操作 | 说明 |
|------|------|
| `put` | 设置键值，需包含 `key` 和 `value` |
| `delete` | 删除键，需包含 `key` |

**从 stdin 输入示例**:
```bash
echo '[
  {"op": "put", "key": "name", "value": "test"},
  {"op": "delete", "key": "old_key"}
]' | rustviking kv batch -f -
```

**JSON 输出示例（成功）**:
```json
{
  "success": true,
  "data": {
    "operation": "batch",
    "puts": 2,
    "deletes": 1,
    "total": 3
  }
}
```

**JSON 输出示例（部分失败）**:
```json
{
  "success": true,
  "data": {
    "operation": "batch",
    "puts": 2,
    "deletes": 0,
    "total": 2,
    "errors": [
      "Operation 2 (delete k3): Key not found"
    ]
  }
}
```

---

## 7. 索引命令

向量索引操作，用于直接操作底层向量索引。

### 7.1 index insert - 插入向量

**语法**:
```bash
rustviking index insert -i <ID> -v <VECTOR> [-l <LEVEL>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-i, --id` | 向量 ID (u64) | 必填 |
| `-v, --vector` | 向量值，逗号分隔 (如 `0.1,0.2,0.3`) | 必填 |
| `-l, --level` | 索引级别 | 1 |

**示例**:
```bash
rustviking index insert -i 1001 -v "0.1,0.2,0.3,0.4" -l 2
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "insert",
    "id": 1001,
    "dimension": 4,
    "level": 2
  }
}
```

---

### 7.2 index search - 向量搜索

**语法**:
```bash
rustviking index search -q <QUERY> [-k <COUNT>] [-l <LEVEL>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-q, --query` | 查询向量，逗号分隔 | 必填 |
| `-k` | 返回结果数量 | 10 |
| `-l, --level` | 搜索级别 | 无（搜索所有级别） |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "query_dimension": 4,
    "k": 10,
    "count": 3,
    "results": [
      {"id": 1001, "score": 0.95, "level": 2},
      {"id": 1002, "score": 0.87, "level": 2},
      {"id": 1003, "score": 0.82, "level": 1}
    ]
  }
}
```

---

### 7.3 index delete - 删除向量

**语法**:
```bash
rustviking index delete -i <ID>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "delete",
    "id": 1001
  }
}
```

---

### 7.4 index info - 索引信息

**语法**:
```bash
rustviking index info
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "count": 10000,
    "dimension": 768
  }
}
```

---

## 8. 低层文件系统命令（Legacy）

`fs` 子命令提供低层文件系统操作，主要用于兼容和调试。

### 8.1 fs mkdir

```bash
rustviking fs mkdir <PATH> [-m <MODE>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `PATH` | 路径或 Viking URI | 必填 |
| `-m, --mode` | 目录权限（八进制） | `0755` |

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "mkdir",
    "path": "/local/test"
  }
}
```

---

### 8.2 fs ls

```bash
rustviking fs ls <PATH> [-r]
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "path": "/local/test",
    "entries": [
      {"name": "file1.txt", "size": 100, "is_dir": false, "mode": "644"}
    ]
  }
}
```

---

### 8.3 fs cat

```bash
rustviking fs cat <PATH>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "path": "/local/test.txt",
    "data": "文件内容..."
  }
}
```

---

### 8.4 fs write

```bash
rustviking fs write <PATH> -d <DATA>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "write",
    "path": "/local/test.txt",
    "bytes_written": 12
  }
}
```

---

### 8.5 fs rm

```bash
rustviking fs rm <PATH> [-r]
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "operation": "rm",
    "path": "/local/test.txt"
  }
}
```

---

### 8.6 fs stat

```bash
rustviking fs stat <PATH>
```

**JSON 输出示例**:
```json
{
  "success": true,
  "data": {
    "path": "/local/test.txt",
    "name": "test.txt",
    "size": 1024,
    "is_dir": false,
    "mode": "644",
    "created_at": 1712234567,
    "updated_at": 1712234567
  }
}
```

---

## 9. Bench 命令

性能基准测试命令。

### 9.1 bench kv-write

```bash
rustviking bench kv-write [-c <COUNT>]
```

**参数**:
| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-c, --count` | 操作次数 | 1000 |

---

### 9.2 bench kv-read

```bash
rustviking bench kv-read [-c <COUNT>]
```

---

### 9.3 bench vector-search

```bash
rustviking bench vector-search [-c <COUNT>]
```

---

### 9.4 bench bitmap-ops

```bash
rustviking bench bitmap-ops [-c <COUNT>]
```

---

### 9.5 输出格式说明

所有 bench 命令输出统一的性能统计：

```json
{
  "success": true,
  "data": {
    "test": "kv-write",
    "count": 1000,
    "total_ms": 125.5,
    "qps": 7968.0,
    "avg_us": 125.5,
    "p50_us": 120.0,
    "p99_us": 250.0,
    "min_us": 80.0,
    "max_us": 500.0
  }
}
```

**字段说明**:
| 字段 | 说明 |
|------|------|
| `test` | 测试类型 |
| `count` | 操作次数 |
| `total_ms` | 总耗时（毫秒） |
| `qps` | 每秒查询数 |
| `avg_us` | 平均延迟（微秒） |
| `p50_us` | P50 延迟（微秒） |
| `p99_us` | P99 延迟（微秒） |
| `min_us` | 最小延迟（微秒） |
| `max_us` | 最大延迟（微秒） |

---

## 10. 集成示例

### 10.1 Python 子进程调用

```python
import json
import subprocess
from typing import Optional, Dict, Any


def rustviking_call(*args: str) -> Dict[str, Any]:
    """调用 rustviking CLI 并解析 JSON 响应"""
    result = subprocess.run(
        ["rustviking", *args],
        capture_output=True,
        text=True
    )
    
    # 解析 stdout 中的 JSON
    response = json.loads(result.stdout)
    
    # 检查退出码
    if result.returncode != 0:
        raise RuntimeError(
            f"rustviking failed with code {result.returncode}: {response.get('error')}"
        )
    
    return response


def viking_read(uri: str, level: Optional[str] = None) -> str:
    """读取 Viking 文件内容"""
    args = ["read", uri]
    if level:
        args.extend(["-l", level])
    
    response = rustviking_call(*args)
    return response["data"]["content"]


def viking_find(query: str, target: Optional[str] = None, k: int = 10):
    """语义搜索"""
    args = ["find", query, "-k", str(k)]
    if target:
        args.extend(["-t", target])
    
    response = rustviking_call(*args)
    return response["data"]["results"]


# 使用示例
if __name__ == "__main__":
    # 读取文件
    content = viking_read("viking://docs/guide.md", level="L1")
    print(f"摘要: {content}")
    
    # 搜索
    results = viking_find("如何配置存储", target="viking://docs", k=5)
    for r in results:
        print(f"[{r['score']:.2f}] {r['uri']}")
```

---

### 10.2 Go exec.Command 调用

```go
package main

import (
	"encoding/json"
	"fmt"
	"os/exec"
)

// CliResponse 定义 CLI 响应结构
type CliResponse struct {
	Success bool            `json:"success"`
	Data    json.RawMessage `json:"data,omitempty"`
	Error   string          `json:"error,omitempty"`
}

// ReadResponse read 命令的数据结构
type ReadResponse struct {
	URI     string `json:"uri"`
	Level   string `json:"level"`
	Content string `json:"content"`
}

// FindResult find 命令的搜索结果
type FindResult struct {
	ID       string  `json:"id"`
	URI      string  `json:"uri"`
	Score    float64 `json:"score"`
	Level    int     `json:"level"`
	Abstract string  `json:"abstract"`
}

// FindResponse find 命令的数据结构
type FindResponse struct {
	Query   string       `json:"query"`
	Target  string       `json:"target"`
	Results []FindResult `json:"results"`
}

// RustVikingClient CLI 客户端
type RustVikingClient struct {
	ConfigPath string
}

// NewClient 创建新客户端
func NewClient(configPath string) *RustVikingClient {
	return &RustVikingClient{ConfigPath: configPath}
}

// exec 执行命令
func (c *RustVikingClient) exec(args ...string) (*CliResponse, error) {
	cmdArgs := []string{"--config", c.ConfigPath}
	cmdArgs = append(cmdArgs, args...)
	
	cmd := exec.Command("rustviking", cmdArgs...)
	output, err := cmd.Output()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			// 解析错误输出
			var resp CliResponse
			if json.Unmarshal(exitErr.Stdout, &resp) == nil {
				return &resp, fmt.Errorf("rustviking error: %s", resp.Error)
			}
		}
		return nil, err
	}
	
	var resp CliResponse
	if err := json.Unmarshal(output, &resp); err != nil {
		return nil, err
	}
	
	return &resp, nil
}

// Read 读取文件
func (c *RustVikingClient) Read(uri string, level string) (*ReadResponse, error) {
	args := []string{"read", uri}
	if level != "" {
		args = append(args, "-l", level)
	}
	
	resp, err := c.exec(args...)
	if err != nil {
		return nil, err
	}
	
	var data ReadResponse
	if err := json.Unmarshal(resp.Data, &data); err != nil {
		return nil, err
	}
	
	return &data, nil
}

// Find 语义搜索
func (c *RustVikingClient) Find(query, target string, k int) (*FindResponse, error) {
	args := []string{"find", query, "-k", fmt.Sprintf("%d", k)}
	if target != "" {
		args = append(args, "-t", target)
	}
	
	resp, err := c.exec(args...)
	if err != nil {
		return nil, err
	}
	
	var data FindResponse
	if err := json.Unmarshal(resp.Data, &data); err != nil {
		return nil, err
	}
	
	return &data, nil
}

func main() {
	client := NewClient("/etc/rustviking.toml")
	
	// 读取文件
	content, err := client.Read("viking://docs/guide.md", "L1")
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		return
	}
	fmt.Printf("Content: %s\n", content.Content)
	
	// 搜索
	results, err := client.Find("配置说明", "viking://docs", 5)
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		return
	}
	for _, r := range results.Results {
		fmt.Printf("[%0.2f] %s\n", r.Score, r.URI)
	}
}
```

---

### 10.3 Shell 脚本调用

```bash
#!/bin/bash

# RustViking CLI 封装脚本

RUSTVIKING=${RUSTVIKING:-"rustviking"}
CONFIG=${RUSTVIKING_CONFIG:-"config.toml"}

# 执行命令并解析 JSON 结果
rv_call() {
    local output
    output=$($RUSTVIKING --config "$CONFIG" "$@" 2>/dev/null)
    echo "$output"
}

# 检查命令是否成功
rv_success() {
    local json="$1"
    echo "$json" | jq -r '.success'
}

# 获取错误信息
rv_error() {
    local json="$1"
    echo "$json" | jq -r '.error // empty'
}

# 读取文件内容
rv_read() {
    local uri="$1"
    local level="${2:-}"
    
    local args=("read" "$uri")
    [[ -n "$level" ]] && args+=("-l" "$level")
    
    local result
    result=$(rv_call "${args[@]}")
    
    if [[ "$(rv_success "$result")" == "true" ]]; then
        echo "$result" | jq -r '.data.content'
    else
        echo "Error: $(rv_error "$result")" >&2
        return 1
    fi
}

# 语义搜索
rv_find() {
    local query="$1"
    local target="${2:-}"
    local k="${3:-10}"
    
    local args=("find" "$query" "-k" "$k")
    [[ -n "$target" ]] && args+=("-t" "$target")
    
    rv_call "${args[@]}"
}

# 使用示例
main() {
    # 读取文件摘要
    echo "=== 读取文件摘要 ==="
    rv_read "viking://docs/readme.md" "L0"
    
    # 搜索文档
    echo -e "\n=== 搜索结果 ==="
    local results
    results=$(rv_find "如何配置存储路径" "viking://docs" 5)
    
    echo "$results" | jq -r '.data.results[] | "[\(.score)] \(.uri)"'
}

main "$@"
```

---

### 10.4 错误处理最佳实践

```python
import json
import subprocess
from enum import IntEnum


class ExitCode(IntEnum):
    SUCCESS = 0
    USER_ERROR = 1
    SYSTEM_ERROR = 2


class RustVikingError(Exception):
    """RustViking CLI 错误"""
    def __init__(self, message: str, exit_code: int, kind: str):
        super().__init__(message)
        self.exit_code = exit_code
        self.kind = kind


def call_rustviking(*args, check=True):
    """
    调用 rustviking CLI，进行完整的错误处理
    
    Args:
        *args: 命令参数
        check: 是否检查返回码，抛出异常
    
    Returns:
        解析后的 JSON 响应
    
    Raises:
        RustVikingError: 当命令失败时
        json.JSONDecodeError: 当输出不是合法 JSON 时
    """
    result = subprocess.run(
        ["rustviking", *args],
        capture_output=True,
        text=True
    )
    
    # 尝试解析 stdout 中的 JSON
    try:
        response = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise json.JSONDecodeError(
            f"Invalid JSON output: {result.stdout[:200]}",
            e.doc,
            e.pos
        )
    
    # 检查退出码
    if result.returncode != ExitCode.SUCCESS:
        error_msg = response.get("error", "Unknown error")
        
        if result.returncode == ExitCode.USER_ERROR:
            kind = "UserError"
        elif result.returncode == ExitCode.SYSTEM_ERROR:
            kind = "SystemError"
        else:
            kind = "Unknown"
        
        error = RustVikingError(error_msg, result.returncode, kind)
        
        if check:
            raise error
    
    return response


# 使用示例
def safe_read(uri: str, level: str = None):
    """安全地读取文件，带重试逻辑"""
    args = ["read", uri]
    if level:
        args.extend(["-l", level])
    
    try:
        response = call_rustviking(*args)
        return response["data"]["content"]
    except RustVikingError as e:
        if e.kind == "UserError":
            # 用户错误通常不需要重试
            print(f"User error: {e}")
            return None
        elif e.kind == "SystemError":
            # 系统错误可能需要重试或报警
            print(f"System error: {e}")
            raise
    except json.JSONDecodeError as e:
        print(f"Failed to parse response: {e}")
        raise


# 批量操作，处理部分失败
def batch_kv_operations(operations: list):
    """执行批量 KV 操作"""
    import tempfile
    import os
    
    # 写入临时文件
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(operations, f)
        temp_path = f.name
    
    try:
        response = call_rustviking("kv", "batch", "-f", temp_path)
        data = response["data"]
        
        print(f"Batch completed: {data['puts']} puts, {data['deletes']} deletes")
        
        if "errors" in data:
            print(f"Errors: {data['errors']}")
            # 处理部分失败...
        
        return data
    finally:
        os.unlink(temp_path)
```

---

## 11. 配置参考

### 11.1 最小化配置示例

```toml
# config.toml - 最小化配置

[storage]
path = "./data/rustviking"

[vector]
dimension = 768

[embedding]
plugin = "mock"
```

### 11.2 完整配置示例

```toml
# config.toml - 生产环境配置

[storage]
path = "/var/lib/rustviking"
create_if_missing = true
max_open_files = 10000
use_fsync = true

[vector]
dimension = 1536
index_type = "ivf_pq"

[vector.ivf_pq]
num_partitions = 256
num_sub_vectors = 16
pq_bits = 8
metric = "l2"

[logging]
level = "info"
format = "json"

[agfs]
default_scope = "resources"
default_account = "default"

[vector_store]
plugin = "rocksdb"

[vector_store.rocksdb]
path = "/var/lib/rustviking/vector_store"

[embedding]
plugin = "openai"

[embedding.openai]
api_base = "https://api.openai.com/v1"
api_key = "${OPENAI_API_KEY}"
model = "text-embedding-3-small"
dimension = 1536
max_concurrent = 10

[summary]
provider = "heuristic"
```

### 11.3 配置项说明

| 配置节 | 键 | 说明 | 默认值 |
|--------|-----|------|--------|
| `storage` | `path` | 数据存储路径 | `./data/rustviking` |
| `storage` | `create_if_missing` | 自动创建目录 | `true` |
| `storage` | `max_open_files` | RocksDB 最大打开文件数 | `10000` |
| `storage` | `use_fsync` | 是否使用 fsync | `false` |
| `vector` | `dimension` | 向量维度 | `768` |
| `vector` | `index_type` | 索引类型 | `ivf_pq` |
| `vector_store` | `plugin` | 向量存储插件 | `memory` |
| `embedding` | `plugin` | Embedding 插件 | `mock` |
| `summary` | `provider` | 摘要生成器 | `heuristic` |

### 11.4 向量维度配置

根据使用的 embedding 模型配置正确的维度：

| 模型 | 维度 |
|------|------|
| text-embedding-3-small | 1536 |
| text-embedding-3-large | 3072 |
| text-embedding-ada-002 | 1536 |
| nomic-embed-text | 768 |
| mock (随机) | 可配置 |

---

## 附录 A: 快速参考卡

### A.1 命令速查表

| 操作 | 命令 |
|------|------|
| 读取文件 | `rustviking read <URI> [-l L0/L1/L2]` |
| 写入文件 | `rustviking write <URI> -d <DATA>` |
| 创建目录 | `rustviking mkdir <URI>` |
| 删除 | `rustviking rm <URI> [-r]` |
| 移动 | `rustviking mv <FROM> <TO>` |
| 列出 | `rustviking ls <URI> [-r]` |
| 搜索 | `rustviking find <QUERY> [-t <TARGET>] [-k <N>]` |
| KV 获取 | `rustviking kv get -k <KEY>` |
| KV 设置 | `rustviking kv put -k <KEY> -v <VALUE>` |
| KV 批量 | `rustviking kv batch -f <FILE>` |
| 向量插入 | `rustviking index insert -i <ID> -v <VEC>` |
| 向量搜索 | `rustviking index search -q <VEC> [-k <N>]` |

### A.2 URI 格式

```
viking://<scope>/<path>

示例:
  viking://resources/docs/guide.md
  viking://data/configs/app.toml
  viking://shared/templates/email.html
```

### A.3 退出码速查

```bash
# 检查退出码
rustviking read viking://test.md
code=$?

case $code in
  0) echo "成功" ;;
  1) echo "用户错误（检查参数）" ;;
  2) echo "系统错误（检查日志）" ;;
esac
```

---

## 附录 B: 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-04-04 | 初始版本 |
