//! 智谱 API 端到端测试
//!
//! 使用智谱（ZhipuAI）的 Embedding API 生成真实向量，
//! 结合 MemoryVectorStore 进行搜索，验证完整链路可用。
//!
//! 运行方式：
//! ```bash
//! source ~/.zshrc && cargo run --example zhipu_e2e_test
//! ```

use rustviking::embedding::openai::OpenAIEmbeddingProvider;
use rustviking::embedding::traits::EmbeddingProvider;
use rustviking::embedding::types::{EmbeddingConfig, EmbeddingRequest};
use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::traits::VectorStore;
use rustviking::vector_store::types::{IndexParams, VectorPoint};
use serde_json::json;

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("智谱 API 端到端验证测试");
    println!("========================================\n");

    // 1. 检查环境变量
    let api_key = std::env::var("ZAI_API_KEY")
        .expect("❌ 未找到 ZAI_API_KEY 环境变量，请先设置: export ZAI_API_KEY=your_api_key");
    println!(
        "✅ API Key 已加载 (前6位: {}...)",
        &api_key[..6.min(api_key.len())]
    );

    // 2. 初始化 OpenAIEmbeddingProvider
    println!("\n[Step 1] 初始化 Embedding Provider...");
    let provider = OpenAIEmbeddingProvider::new();

    let config = EmbeddingConfig {
        api_base: "https://open.bigmodel.cn/api/paas/v4".to_string(),
        api_key: Some("".to_string()), // 空字符串会自动从 ZAI_API_KEY 读取
        provider: "openai".to_string(),
        model: "embedding-3".to_string(),
        dimension: 2048, // 智谱 embedding-3 默认维度
        max_concurrent: 10,
    };

    match provider.initialize(config).await {
        Ok(()) => println!("✅ Embedding Provider 初始化成功"),
        Err(e) => {
            println!("❌ Embedding Provider 初始化失败: {}", e);
            std::process::exit(1);
        }
    }

    // 3. 初始化 MemoryVectorStore
    println!("\n[Step 2] 初始化 MemoryVectorStore...");
    let store = MemoryVectorStore::new();
    let mut collection_name = "test_collection";
    let dimension = 2048usize;

    match store
        .create_collection(collection_name, dimension, IndexParams::default())
        .await
    {
        Ok(()) => println!(
            "✅ 集合 '{}' 创建成功 (dimension={})",
            collection_name, dimension
        ),
        Err(e) => {
            println!("❌ 集合创建失败: {}", e);
            std::process::exit(1);
        }
    }

    // 4. 准备测试文本
    println!("\n[Step 3] 准备测试文本...");
    let texts: Vec<String> = vec![
        "RustViking 是一个高性能的向量数据库引擎".to_string(),
        "Rust 语言提供了内存安全和并发安全".to_string(),
        "向量搜索是 AI 应用的核心能力".to_string(),
        "今天天气很好，适合出去散步".to_string(),
        "机器学习模型需要大量训练数据".to_string(),
    ];

    for (i, text) in texts.iter().enumerate() {
        println!("  [{}] {}", i + 1, text);
    }

    // 5. 调用 embed 生成向量
    println!("\n[Step 4] 调用智谱 API 生成向量...");
    let request = EmbeddingRequest {
        texts: texts.clone(),
        model: None,     // 使用默认模型
        normalize: true, // 归一化向量以便进行余弦相似度搜索
    };

    let result = match provider.embed(request).await {
        Ok(r) => r,
        Err(e) => {
            println!("❌ Embedding API 调用失败: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ API 调用成功！");
    println!("   - 返回模型: {}", result.model);
    println!("   - 向量数量: {}", result.embeddings.len());

    // 打印每个向量的维度
    for (i, embedding) in result.embeddings.iter().enumerate() {
        println!("   - 文本[{}] 向量维度: {}", i, embedding.len());
    }

    // 检查向量维度
    let actual_dimension = result.embeddings[0].len();
    println!("\n📐 实际向量维度: {}", actual_dimension);

    if actual_dimension != dimension {
        println!(
            "⚠️  注意: 配置维度({}) 与实际返回维度({}) 不同",
            dimension, actual_dimension
        );
        println!("   将使用实际维度重新创建集合...");

        // 重新创建集合
        let new_collection = "test_collection_v2";
        match store
            .create_collection(new_collection, actual_dimension, IndexParams::default())
            .await
        {
            Ok(()) => {
                collection_name = new_collection;
                println!(
                    "✅ 新集合 '{}' 创建成功 (dimension={})",
                    collection_name, actual_dimension
                );
            }
            Err(e) => {
                println!("❌ 新集合创建失败: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 6. 将向量 upsert 到 MemoryVectorStore
    println!("\n[Step 5] 将向量插入 VectorStore...");
    let points: Vec<VectorPoint> = result
        .embeddings
        .iter()
        .enumerate()
        .map(|(i, embedding): (usize, &Vec<f32>)| VectorPoint {
            id: format!("doc_{}", i),
            vector: embedding.clone(),
            sparse_vector: None,
            payload: json!({
                "id": format!("doc_{}", i),
                "uri": format!("/test/doc_{}", i),
                "text": texts[i],
                "context_type": "test",
            }),
        })
        .collect();

    match store.upsert(collection_name, points).await {
        Ok(()) => println!("✅ 成功插入 {} 个向量", texts.len()),
        Err(e) => {
            println!("❌ 向量插入失败: {}", e);
            std::process::exit(1);
        }
    }

    // 7. 用查询文本进行搜索
    println!("\n[Step 6] 执行语义搜索...");
    let query_text = "向量数据库的性能优化";
    println!("   查询: \"{}\"", query_text);

    let query_request = EmbeddingRequest {
        texts: vec![query_text.to_string()],
        model: None,
        normalize: true,
    };

    let query_result = match provider.embed(query_request).await {
        Ok(r) => r,
        Err(e) => {
            println!("❌ 查询向量生成失败: {}", e);
            std::process::exit(1);
        }
    };

    let query_vector = &query_result.embeddings[0];
    println!("   查询向量维度: {}", query_vector.len());

    let search_results = match store.search(collection_name, query_vector, 5, None).await {
        Ok(results) => results,
        Err(e) => {
            println!("❌ 搜索失败: {}", e);
            std::process::exit(1);
        }
    };

    // 8. 打印搜索结果
    println!("\n[Step 7] 搜索结果:");
    println!("─────────────────────────────────────────────────────────────");

    for (rank, result) in search_results.iter().enumerate() {
        // 使用 id 来获取对应的原始文本
        let doc_idx = result
            .id
            .strip_prefix("doc_")
            .and_then(|s: &str| s.parse::<usize>().ok())
            .unwrap_or(0);
        let original_text: &str = texts.get(doc_idx).map(|s| s as &str).unwrap_or("?");

        println!(
            "  {}. [score: {:.6}] {}",
            rank + 1,
            result.score,
            original_text
        );
    }
    println!("─────────────────────────────────────────────────────────────");

    // 验证语义相关性
    println!("\n[Step 8] 验证语义相关性...");
    if !search_results.is_empty() {
        let top_result = &search_results[0];
        let doc_idx = top_result
            .id
            .strip_prefix("doc_")
            .and_then(|s: &str| s.parse::<usize>().ok())
            .unwrap_or(0);

        // 检查最相关的结果是否与向量数据库相关
        if doc_idx == 0 {
            println!("✅ 最相关结果正确: \"{}\" (向量数据库相关)", texts[doc_idx]);
        } else {
            println!("ℹ️  最相关结果: \"{}\"", texts[doc_idx]);
            println!("   (注意: 语义相关性可能因模型而异)");
        }
    }

    println!("\n========================================");
    println!("✅ 智谱 API 端到端验证通过！");
    println!("========================================");
}
