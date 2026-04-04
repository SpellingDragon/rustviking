//! Index command handlers

use crate::cli::commands::OutputFormat;
use crate::cli::{success, CliResponse};
use crate::error::Result;
use crate::index::VectorIndex;
use serde::Serialize;

/// Output response based on format
fn output_result<T: Serialize>(response: &CliResponse<T>, _format: &OutputFormat) {
    // For now, always output JSON (table/plain support can be added later)
    println!("{}", response.to_json_pretty());
}

pub fn exec_index_insert(
    index: &dyn VectorIndex,
    id: u64,
    vector: &[f32],
    level: u8,
    format: &OutputFormat,
) -> Result<()> {
    index.insert(id, vector, level)?;
    let response = success(serde_json::json!({
        "operation": "insert",
        "id": id,
        "dimension": vector.len(),
        "level": level,
    }));
    output_result(&response, format);
    Ok(())
}

pub fn exec_index_search(
    index: &dyn VectorIndex,
    query: &[f32],
    k: usize,
    level: Option<u8>,
    format: &OutputFormat,
) -> Result<()> {
    let results = index.search(query, k, level)?;
    let results_json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "score": r.score,
                "level": r.level,
            })
        })
        .collect();

    let response = success(serde_json::json!({
        "query_dimension": query.len(),
        "k": k,
        "count": results_json.len(),
        "results": results_json,
    }));
    output_result(&response, format);
    Ok(())
}

pub fn exec_index_delete(index: &dyn VectorIndex, id: u64, format: &OutputFormat) -> Result<()> {
    index.delete(id)?;
    let response = success(serde_json::json!({
        "operation": "delete",
        "id": id,
    }));
    output_result(&response, format);
    Ok(())
}

pub fn exec_index_info(index: &dyn VectorIndex, format: &OutputFormat) -> Result<()> {
    let response = success(serde_json::json!({
        "count": index.count(),
        "dimension": index.dimension(),
    }));
    output_result(&response, format);
    Ok(())
}
