//! KV store command handlers

use crate::cli::commands::OutputFormat;
use crate::error::Result;
use crate::storage::KvStore;

pub fn exec_kv_get(store: &dyn KvStore, key: &str, _format: &OutputFormat) -> Result<()> {
    let value = store.get(key.as_bytes())?;
    let output = match value {
        Some(v) => {
            let text = String::from_utf8_lossy(&v);
            serde_json::json!({
                "status": "ok",
                "key": key,
                "value": text,
            })
        }
        None => {
            serde_json::json!({
                "status": "ok",
                "key": key,
                "value": null,
            })
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    Ok(())
}

pub fn exec_kv_put(
    store: &dyn KvStore,
    key: &str,
    value: &str,
    _format: &OutputFormat,
) -> Result<()> {
    store.put(key.as_bytes(), value.as_bytes())?;
    let output = serde_json::json!({
        "status": "ok",
        "operation": "put",
        "key": key,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    Ok(())
}

pub fn exec_kv_del(store: &dyn KvStore, key: &str, _format: &OutputFormat) -> Result<()> {
    store.delete(key.as_bytes())?;
    let output = serde_json::json!({
        "status": "ok",
        "operation": "delete",
        "key": key,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    Ok(())
}

pub fn exec_kv_scan(
    store: &dyn KvStore,
    prefix: &str,
    limit: usize,
    _format: &OutputFormat,
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

    let output = serde_json::json!({
        "status": "ok",
        "prefix": prefix,
        "count": entries.len(),
        "entries": entries,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    Ok(())
}
