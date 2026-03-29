use super::types::*;
use crate::error::Result;
use serde_json::Value;

/// 向量存储插件 Trait
pub trait VectorStore: Send + Sync {
    /// 插件名称
    fn name(&self) -> &str;

    /// 插件版本
    fn version(&self) -> &str;

    /// 初始化
    fn initialize(&self, config: &Value) -> Result<()>;

    /// 创建集合
    fn create_collection(&self, name: &str, dimension: usize, params: IndexParams) -> Result<()>;

    /// 插入/更新向量
    fn upsert(&self, collection: &str, points: Vec<VectorPoint>) -> Result<()>;

    /// 搜索向量
    fn search(
        &self,
        collection: &str,
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
    ) -> Result<Vec<VectorSearchResult>>;

    /// 获取向量
    fn get(&self, collection: &str, id: &str) -> Result<Option<VectorPoint>>;

    /// 删除向量
    fn delete(&self, collection: &str, id: &str) -> Result<()>;

    /// 按 URI 前缀删除（用于向量同步）
    fn delete_by_uri_prefix(&self, collection: &str, uri_prefix: &str) -> Result<()>;

    /// 更新 URI（用于向量同步）
    fn update_uri(&self, collection: &str, old_uri: &str, new_uri: &str) -> Result<()>;

    /// 获取集合信息
    fn collection_info(&self, collection: &str) -> Result<CollectionInfo>;
}
