//! Qdrant vector store adapter

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{Result, RustVikingError};
use crate::vector_store::traits::VectorStore;
use crate::vector_store::types::*;

use qdrant_client::qdrant::{
    CollectionExistsRequest, Condition, CreateCollectionBuilder, DeletePointsBuilder,
    Distance, GetCollectionInfoRequest, GetPointsBuilder, PointId, PointStruct, Range,
    ScrollPointsBuilder, SearchPointsBuilder, SetPayloadPointsBuilder, UpsertPointsBuilder,
    VectorParamsBuilder,
};
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::Payload;
use qdrant_client::Qdrant;

/// Qdrant vector store implementation
pub struct QdrantVectorStore {
    client: Qdrant,
    /// Reserved for future use (e.g., default collection name when not specified in operations)
    #[allow(dead_code)]
    default_collection: String,
    /// Reserved for future use (e.g., custom timeout per operation)
    #[allow(dead_code)]
    timeout_ms: u64,
}

impl QdrantVectorStore {
    /// Create a new QdrantVectorStore instance
    pub async fn new(
        url: &str,
        api_key: Option<&str>,
        collection: &str,
        timeout_ms: u64,
    ) -> Result<Self> {
        let mut builder = Qdrant::from_url(url).timeout(timeout_ms);

        if let Some(key) = api_key {
            builder = builder.api_key(key.to_string());
        }

        let client = builder
            .build()
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to create Qdrant client: {}", e)))?;

        Ok(Self {
            client,
            default_collection: collection.to_string(),
            timeout_ms,
        })
    }

    /// Convert RustViking DistanceType to Qdrant Distance
    fn to_qdrant_distance(distance: DistanceType) -> Distance {
        match distance {
            DistanceType::Cosine => Distance::Cosine,
            DistanceType::L2 => Distance::Euclid,
            DistanceType::DotProduct => Distance::Dot,
        }
    }

    /// Convert Qdrant Distance to RustViking DistanceType
    fn from_qdrant_distance(distance: Distance) -> DistanceType {
        match distance {
            Distance::Cosine => DistanceType::Cosine,
            Distance::Euclid => DistanceType::L2,
            Distance::Dot => DistanceType::DotProduct,
            _ => DistanceType::Cosine,
        }
    }

    /// Convert RustViking Filter to Qdrant Filter
    fn compile_filter(filter: &Filter) -> qdrant_client::qdrant::Filter {
        match filter {
            Filter::Eq(field, value) => {
                let match_value = Self::json_value_to_match_value(value);
                qdrant_client::qdrant::Filter::must([Condition::matches(field, match_value)])
            }
            Filter::In(field, values) => {
                let keywords: Vec<String> = values
                    .iter()
                    .map(|v| {
                        if let Some(s) = v.as_str() {
                            s.to_string()
                        } else {
                            v.to_string()
                        }
                    })
                    .collect();
                qdrant_client::qdrant::Filter::must([Condition::matches(field, keywords)])
            }
            Filter::Range(field, min, max) => {
                let range = Range {
                    gte: min.as_ref().and_then(|v| v.as_f64()),
                    lte: max.as_ref().and_then(|v| v.as_f64()),
                    gt: None,
                    lt: None,
                };
                qdrant_client::qdrant::Filter::must([Condition::range(field, range)])
            }
            Filter::And(filters) => {
                let conditions: Vec<Condition> = filters
                    .iter()
                    .map(|f| {
                        let qf = Self::compile_filter(f);
                        // Convert Filter to Condition by wrapping it
                        Condition {
                            condition_one_of: Some(
                                qdrant_client::qdrant::condition::ConditionOneOf::Filter(qf),
                            ),
                        }
                    })
                    .collect();
                qdrant_client::qdrant::Filter::must(conditions)
            }
            Filter::Or(filters) => {
                let conditions: Vec<Condition> = filters
                    .iter()
                    .map(|f| {
                        let qf = Self::compile_filter(f);
                        Condition {
                            condition_one_of: Some(
                                qdrant_client::qdrant::condition::ConditionOneOf::Filter(qf),
                            ),
                        }
                    })
                    .collect();
                qdrant_client::qdrant::Filter::should(conditions)
            }
        }
    }

    /// Convert serde_json::Value to a type that implements Into<MatchValue>
    fn json_value_to_match_value(value: &Value) -> MatchValue {
        match value {
            Value::String(s) => MatchValue::Keyword(s.clone()),
            Value::Bool(b) => MatchValue::Boolean(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    MatchValue::Integer(i)
                } else {
                    MatchValue::Integer(n.as_f64().unwrap_or(0.0) as i64)
                }
            }
            _ => MatchValue::Keyword(value.to_string()),
        }
    }

    /// Convert VectorPoint to Qdrant PointStruct
    fn to_point_struct(point: &VectorPoint) -> Result<PointStruct> {
        // Try to parse ID as u64, otherwise use string-based ID
        let point_id = if let Ok(num_id) = point.id.parse::<u64>() {
            PointId::from(num_id)
        } else {
            PointId::from(point.id.clone())
        };

        // Convert payload to Qdrant Payload
        // Extract the object from the payload if it's an object
        let payload: Payload = if let Some(obj) = point.payload.as_object() {
            obj.clone().into()
        } else {
            // If payload is not an object, wrap it in an object
            let mut map = serde_json::Map::new();
            map.insert("data".to_string(), point.payload.clone());
            map.into()
        };

        Ok(PointStruct::new(
            point_id,
            point.vector.clone(),
            payload,
        ))
    }

    /// Convert Qdrant ScoredPoint to VectorSearchResult
    fn to_search_result(point: &qdrant_client::qdrant::ScoredPoint) -> Result<VectorSearchResult> {
        let id = match &point.id {
            Some(pid) => match &pid.point_id_options {
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(num)) => num.to_string(),
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) => uuid.clone(),
                None => return Err(RustVikingError::VectorStore("Missing point ID".to_string())),
            },
            None => return Err(RustVikingError::VectorStore("Missing point ID".to_string())),
        };

        // Extract metadata from payload
        let metadata = Self::extract_metadata(&point.payload)?;

        Ok(VectorSearchResult {
            id,
            score: point.score,
            metadata,
        })
    }

    /// Convert Qdrant RetrievedPoint to VectorPoint
    fn to_vector_point(point: &qdrant_client::qdrant::RetrievedPoint) -> Result<VectorPoint> {
        let id = match &point.id {
            Some(pid) => match &pid.point_id_options {
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(num)) => num.to_string(),
                Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) => uuid.clone(),
                None => return Err(RustVikingError::VectorStore("Missing point ID".to_string())),
            },
            None => return Err(RustVikingError::VectorStore("Missing point ID".to_string())),
        };

        let vector = point
            .vectors
            .as_ref()
            .and_then(|v| v.vectors_options.as_ref())
            .and_then(|vo| match vo {
                qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v) => {
                    v.vector.as_ref().and_then(|vec| match vec {
                        qdrant_client::qdrant::vector_output::Vector::Dense(dense) => {
                            Some(dense.data.clone())
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .unwrap_or_default();

        let payload = Self::payload_to_json(&point.payload)?;

        Ok(VectorPoint {
            id,
            vector,
            sparse_vector: None,
            payload,
        })
    }

    /// Extract VectorMetadata from payload
    fn extract_metadata(payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>) -> Result<VectorMetadata> {
        let get_string = |key: &str| -> Option<String> {
            payload.get(key).and_then(|v| {
                v.kind.as_ref().and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
                    _ => None,
                })
            })
        };

        let get_int = |key: &str| -> Option<i64> {
            payload.get(key).and_then(|v| {
                v.kind.as_ref().and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::IntegerValue(i) => Some(*i),
                    _ => None,
                })
            })
        };

        let get_bool = |key: &str| -> Option<bool> {
            payload.get(key).and_then(|v| {
                v.kind.as_ref().and_then(|k| match k {
                    qdrant_client::qdrant::value::Kind::BoolValue(b) => Some(*b),
                    _ => None,
                })
            })
        };

        Ok(VectorMetadata {
            id: get_string("id").unwrap_or_default(),
            uri: get_string("uri").unwrap_or_default(),
            parent_uri: get_string("parent_uri"),
            context_type: get_string("context_type").unwrap_or_default(),
            is_leaf: get_bool("is_leaf").unwrap_or(true),
            level: get_int("level").map(|l| l as u8).unwrap_or(0),
            abstract_text: get_string("abstract_text"),
            name: get_string("name"),
            description: get_string("description"),
            created_at: get_string("created_at").unwrap_or_default(),
            active_count: get_int("active_count").unwrap_or(0),
        })
    }

    /// Convert Qdrant payload to serde_json::Value
    fn payload_to_json(
        payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    ) -> Result<Value> {
        let mut map = serde_json::Map::new();
        for (key, value) in payload {
            let json_value = Self::qdrant_value_to_json(value)?;
            map.insert(key.clone(), json_value);
        }
        Ok(Value::Object(map))
    }

    /// Convert Qdrant Value to serde_json::Value
    fn qdrant_value_to_json(value: &qdrant_client::qdrant::Value) -> Result<Value> {
        match &value.kind {
            Some(kind) => match kind {
                qdrant_client::qdrant::value::Kind::NullValue(_) => Ok(Value::Null),
                qdrant_client::qdrant::value::Kind::DoubleValue(v) => {
                    Ok(Value::Number(serde_json::Number::from_f64(*v).unwrap_or(0.into())))
                }
                qdrant_client::qdrant::value::Kind::IntegerValue(v) => Ok(Value::Number((*v).into())),
                qdrant_client::qdrant::value::Kind::StringValue(v) => Ok(Value::String(v.clone())),
                qdrant_client::qdrant::value::Kind::BoolValue(v) => Ok(Value::Bool(*v)),
                qdrant_client::qdrant::value::Kind::StructValue(s) => {
                    let mut map = serde_json::Map::new();
                    for (k, v) in &s.fields {
                        map.insert(k.clone(), Self::qdrant_value_to_json(v)?);
                    }
                    Ok(Value::Object(map))
                }
                qdrant_client::qdrant::value::Kind::ListValue(l) => {
                    let arr: Result<Vec<Value>> =
                        l.values.iter().map(Self::qdrant_value_to_json).collect();
                    Ok(Value::Array(arr?))
                }
            },
            None => Ok(Value::Null),
        }
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    fn name(&self) -> &str {
        "qdrant"
    }

    fn version(&self) -> &str {
        "1.17.0"
    }

    async fn initialize(&self, _config: &Value) -> Result<()> {
        // Qdrant client is already initialized in new()
        // Optionally check connection here
        self.client
            .health_check()
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Qdrant health check failed: {}", e)))?;
        Ok(())
    }

    async fn create_collection(
        &self,
        name: &str,
        dimension: usize,
        params: IndexParams,
    ) -> Result<()> {
        let distance = Self::to_qdrant_distance(params.distance);

        let builder = CreateCollectionBuilder::new(name)
            .vectors_config(VectorParamsBuilder::new(dimension as u64, distance));

        self.client
            .create_collection(builder)
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to create collection: {}", e)))?;

        Ok(())
    }

    async fn upsert(&self, collection: &str, points: Vec<VectorPoint>) -> Result<()> {
        let qdrant_points: Result<Vec<PointStruct>> = points.iter().map(Self::to_point_struct).collect();
        let qdrant_points = qdrant_points?;

        self.client
            .upsert_points(UpsertPointsBuilder::new(collection, qdrant_points))
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to upsert points: {}", e)))?;

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query: &[f32],
        k: usize,
        filters: Option<Filter>,
    ) -> Result<Vec<VectorSearchResult>> {
        let mut builder = SearchPointsBuilder::new(collection, query.to_vec(), k as u64)
            .with_payload(true)
            .with_vectors(false);

        if let Some(filter) = filters {
            let qdrant_filter = Self::compile_filter(&filter);
            builder = builder.filter(qdrant_filter);
        }

        let response = self
            .client
            .search_points(builder)
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Search failed: {}", e)))?;

        response
            .result
            .iter()
            .map(Self::to_search_result)
            .collect()
    }

    async fn get(&self, collection: &str, id: &str) -> Result<Option<VectorPoint>> {
        let point_id = if let Ok(num_id) = id.parse::<u64>() {
            PointId::from(num_id)
        } else {
            PointId::from(id.to_string())
        };

        let response = self
            .client
            .get_points(
                GetPointsBuilder::new(collection, vec![point_id])
                    .with_vectors(true)
                    .with_payload(true),
            )
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to get point: {}", e)))?;

        if let Some(point) = response.result.first() {
            Ok(Some(Self::to_vector_point(point)?))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        let point_id = if let Ok(num_id) = id.parse::<u64>() {
            PointId::from(num_id)
        } else {
            PointId::from(id.to_string())
        };

        self.client
            .delete_points(
                DeletePointsBuilder::new(collection)
                    .points(vec![point_id])
                    .wait(true),
            )
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to delete point: {}", e)))?;

        Ok(())
    }

    async fn delete_by_uri_prefix(&self, collection: &str, uri_prefix: &str) -> Result<()> {
        // Use scroll to find all points with URI matching the prefix, then delete them
        let filter = qdrant_client::qdrant::Filter::must([Condition::matches(
            "uri",
            uri_prefix.to_string(),
        )]);

        self.client
            .delete_points(
                DeletePointsBuilder::new(collection)
                    .points(filter)
                    .wait(true),
            )
            .await
            .map_err(|e| {
                RustVikingError::VectorStore(format!("Failed to delete by URI prefix: {}", e))
            })?;

        Ok(())
    }

    async fn update_uri(&self, collection: &str, old_uri: &str, new_uri: &str) -> Result<()> {
        // First, scroll to find all points with the old URI
        let filter = qdrant_client::qdrant::Filter::must([Condition::matches(
            "uri",
            old_uri.to_string(),
        )]);

        let scroll_response = self
            .client
            .scroll(
                ScrollPointsBuilder::new(collection)
                    .filter(filter)
                    .with_payload(true)
                    .limit(1000),
            )
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to scroll points: {}", e)))?;

        // Update URI for each found point
        for point in &scroll_response.result {
            let point_id = match &point.id {
                Some(pid) => pid.clone(),
                None => continue,
            };

            let new_payload: Payload = {
                let mut map = serde_json::Map::new();
                map.insert("uri".to_string(), serde_json::Value::String(new_uri.to_string()));
                map.into()
            };

            self.client
                .set_payload(
                    SetPayloadPointsBuilder::new(collection, new_payload)
                        .points_selector(vec![point_id])
                        .wait(true),
                )
                .await
                .map_err(|e| RustVikingError::VectorStore(format!("Failed to update URI: {}", e)))?;
        }

        Ok(())
    }

    async fn collection_info(&self, collection: &str) -> Result<CollectionInfo> {
        let exists = self
            .client
            .collection_exists(CollectionExistsRequest {
                collection_name: collection.to_string(),
            })
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to check collection: {}", e)))?;

        if !exists {
            return Err(RustVikingError::CollectionNotFound(collection.to_string()));
        }

        let info = self
            .client
            .collection_info(GetCollectionInfoRequest {
                collection_name: collection.to_string(),
            })
            .await
            .map_err(|e| RustVikingError::VectorStore(format!("Failed to get collection info: {}", e)))?;

        let result = info.result.ok_or_else(|| {
            RustVikingError::VectorStore("Missing collection info result".to_string())
        })?;

        let config = result.config.ok_or_else(|| {
            RustVikingError::VectorStore("Missing collection config".to_string())
        })?;

        let params = config.params.ok_or_else(|| {
            RustVikingError::VectorStore("Missing collection params".to_string())
        })?;

        let vectors_config = params.vectors_config.ok_or_else(|| {
            RustVikingError::VectorStore("Missing vectors config".to_string())
        })?;

        // Extract vector params (handle both single and multiple vectors config)
        let vector_params = match vectors_config.config {
            Some(qdrant_client::qdrant::vectors_config::Config::Params(params)) => params,
            Some(qdrant_client::qdrant::vectors_config::Config::ParamsMap(_)) => {
                // For multi-vector collections, use default
                return Ok(CollectionInfo {
                    name: collection.to_string(),
                    dimension: 0,
                    count: result.points_count.unwrap_or(0),
                    index_type: IndexType::Hnsw,
                    distance: DistanceType::Cosine,
                });
            }
            None => {
                return Err(RustVikingError::VectorStore(
                    "Missing vector config".to_string(),
                ))
            }
        };

        Ok(CollectionInfo {
            name: collection.to_string(),
            dimension: vector_params.size as usize,
            count: result.points_count.unwrap_or(0),
            index_type: IndexType::Hnsw, // Qdrant uses HNSW by default
            distance: Self::from_qdrant_distance(vector_params.distance()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_filter_eq() {
        let filter = Filter::Eq("field".to_string(), serde_json::json!("value"));
        let qdrant_filter = QdrantVectorStore::compile_filter(&filter);
        assert_eq!(qdrant_filter.must.len(), 1);
    }

    #[test]
    fn test_compile_filter_and() {
        let filter = Filter::And(vec![
            Filter::Eq("field1".to_string(), serde_json::json!("value1")),
            Filter::Eq("field2".to_string(), serde_json::json!("value2")),
        ]);
        let qdrant_filter = QdrantVectorStore::compile_filter(&filter);
        assert_eq!(qdrant_filter.must.len(), 2);
    }

    #[test]
    fn test_compile_filter_or() {
        let filter = Filter::Or(vec![
            Filter::Eq("field1".to_string(), serde_json::json!("value1")),
            Filter::Eq("field2".to_string(), serde_json::json!("value2")),
        ]);
        let qdrant_filter = QdrantVectorStore::compile_filter(&filter);
        assert_eq!(qdrant_filter.should.len(), 2);
    }
}
