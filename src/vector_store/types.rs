use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 向量记录元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub id: String,
    pub uri: String,
    pub parent_uri: Option<String>,
    pub context_type: String, // resource/memory/skill
    pub is_leaf: bool,
    pub level: u8, // L0/L1/L2
    pub abstract_text: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub active_count: i64,
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: VectorMetadata,
}

/// 向量点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub sparse_vector: Option<HashMap<usize, f32>>,
    pub payload: Value,
}

/// 索引参数
#[derive(Debug, Clone)]
pub struct IndexParams {
    pub index_type: IndexType,
    pub distance: DistanceType,
    pub quantization: Option<QuantizationType>,
    // HNSW 参数
    pub m: Option<usize>,
    pub ef_construction: Option<usize>,
    pub ef_search: Option<usize>,
    // IVF 参数
    pub num_partitions: Option<usize>,
    pub nprobe: Option<usize>,
}

impl Default for IndexParams {
    fn default() -> Self {
        Self {
            index_type: IndexType::Hnsw,
            distance: DistanceType::Cosine,
            quantization: None,
            m: Some(16),
            ef_construction: Some(200),
            ef_search: Some(50),
            num_partitions: Some(256),
            nprobe: Some(16),
        }
    }
}

/// 索引类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    Flat,
    Hnsw,
    Ivf,
    FlatHybrid,
}

/// 距离类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceType {
    Cosine,
    L2,
    DotProduct,
}

/// 量化类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizationType {
    Int8,
    Int16,
    Binary,
}

/// 过滤条件
#[derive(Debug, Clone)]
pub enum Filter {
    Eq(String, Value),
    In(String, Vec<Value>),
    Range(String, Option<Value>, Option<Value>),
    And(Vec<Filter>),
    Or(Vec<Filter>),
}

/// 集合信息
#[derive(Debug, Clone)]
pub struct CollectionInfo {
    pub name: String,
    pub dimension: usize,
    pub count: u64,
    pub index_type: IndexType,
    pub distance: DistanceType,
}
