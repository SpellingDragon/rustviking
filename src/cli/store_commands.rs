//! KV store command handlers

use std::io::{self, Read};

use crate::cli::commands::OutputFormat;
use crate::cli::{success, CliResponse};
use crate::error::{Result, RustVikingError};
use crate::storage::KvStore;
use serde::{Deserialize, Serialize};

/// Output response based on format
fn output_result<T: Serialize>(response: &CliResponse<T>, _format: &OutputFormat) {
    // For now, always output JSON (table/plain support can be added later)
    println!("{}", response.to_json_pretty());
}

/// A single batch operation
#[derive(Debug, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "lowercase")]
enum BatchOp {
    Put { key: String, value: String },
    Delete { key: String },
}

pub fn exec_kv_get(store: &dyn KvStore, key: &str, format: &OutputFormat) -> Result<()> {
    let value = store.get(key.as_bytes())?;
    let response = match value {
        Some(v) => {
            let text = String::from_utf8_lossy(&v);
            success(serde_json::json!({
                "key": key,
                "value": text,
            }))
        }
        None => success(serde_json::json!({
            "key": key,
            "value": serde_json::Value::Null,
        })),
    };
    output_result(&response, format);
    Ok(())
}

pub fn exec_kv_put(
    store: &dyn KvStore,
    key: &str,
    value: &str,
    format: &OutputFormat,
) -> Result<()> {
    store.put(key.as_bytes(), value.as_bytes())?;
    let response = success(serde_json::json!({
        "operation": "put",
        "key": key,
    }));
    output_result(&response, format);
    Ok(())
}

pub fn exec_kv_del(store: &dyn KvStore, key: &str, format: &OutputFormat) -> Result<()> {
    store.delete(key.as_bytes())?;
    let response = success(serde_json::json!({
        "operation": "delete",
        "key": key,
    }));
    output_result(&response, format);
    Ok(())
}

pub fn exec_kv_scan(
    store: &dyn KvStore,
    prefix: &str,
    limit: usize,
    format: &OutputFormat,
) -> Result<()> {
    let results = store.scan_prefix(prefix.as_bytes())?;
    let entries: Vec<serde_json::Value> = results
        .into_iter()
        .take(limit)
        .map(|(k, v)| {
            serde_json::json!({
                "key": String::from_utf8_lossy(&k),
                "value": String::from_utf8_lossy(&v),
            })
        })
        .collect();

    let response = success(serde_json::json!({
        "prefix": prefix,
        "count": entries.len(),
        "entries": entries,
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute batch operations from file or stdin
pub fn exec_kv_batch(store: &dyn KvStore, file: &str, format: &OutputFormat) -> Result<()> {
    // Read input
    let input = if file == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| RustVikingError::Serialization(format!("Failed to read stdin: {}", e)))?;
        buffer
    } else {
        // Read from file
        std::fs::read_to_string(file).map_err(RustVikingError::Io)?
    };

    // Parse operations
    let ops: Vec<BatchOp> = serde_json::from_str(&input)
        .map_err(|e| RustVikingError::CliInput(format!("Invalid batch JSON: {}", e)))?;

    // Execute operations
    let mut put_count = 0;
    let mut delete_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, op) in ops.into_iter().enumerate() {
        match op {
            BatchOp::Put { key, value } => match store.put(key.as_bytes(), value.as_bytes()) {
                Ok(()) => put_count += 1,
                Err(e) => errors.push(format!("Operation {} (put {}): {}", i, key, e)),
            },
            BatchOp::Delete { key } => match store.delete(key.as_bytes()) {
                Ok(()) => delete_count += 1,
                Err(e) => errors.push(format!("Operation {} (delete {}): {}", i, key, e)),
            },
        }
    }

    let response = if errors.is_empty() {
        success(serde_json::json!({
            "operation": "batch",
            "puts": put_count,
            "deletes": delete_count,
            "total": put_count + delete_count,
        }))
    } else {
        // Partial success - still return success but include errors
        success(serde_json::json!({
            "operation": "batch",
            "puts": put_count,
            "deletes": delete_count,
            "total": put_count + delete_count,
            "errors": errors,
        }))
    };

    output_result(&response, format);
    Ok(())
}
