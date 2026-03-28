//! Index command handlers

use crate::index::VectorIndex;
use crate::cli::commands::OutputFormat;
use crate::error::Result;

pub fn exec_index_insert(index: &dyn VectorIndex, id: u64, vector: &[f32], level: u8, _format: &OutputFormat) -> Result<()> {
    index.insert(id, vector, level)?;
    let output = serde_json::json!({
        "status": "ok",
        "operation": "insert",
        "id": id,
        "dimension": vector.len(),
        "level": level,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    Ok(())
}

pub fn exec_index_search(index: &dyn VectorIndex, query: &[f32], k: usize, level: Option<u8>, _format: &OutputFormat) -> Result<()> {
    let results = index.search(query, k, level)?;
    let results_json: Vec<serde_json::Value> = results.iter().map(|r| {
        serde_json::json!({
            "id": r.id,
            "score": r.score,
            "level": r.level,
        })
    }).collect();
    
    let output = serde_json::json!({
        "status": "ok",
        "query_dimension": query.len(),
        "k": k,
        "count": results_json.len(),
        "results": results_json,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    Ok(())
}

pub fn exec_index_delete(index: &dyn VectorIndex, id: u64, _format: &OutputFormat) -> Result<()> {
    index.delete(id)?;
    let output = serde_json::json!({
        "status": "ok",
        "operation": "delete",
        "id": id,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    Ok(())
}

pub fn exec_index_info(index: &dyn VectorIndex, _format: &OutputFormat) -> Result<()> {
    let output = serde_json::json!({
        "status": "ok",
        "count": index.count(),
        "dimension": index.dimension(),
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    Ok(())
}
