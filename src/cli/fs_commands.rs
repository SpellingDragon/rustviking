//! Filesystem command handlers

use crate::agfs::mountable::MountableFS;
use crate::agfs::VikingUri;
use crate::agfs::WriteFlag;
use crate::cli::commands::OutputFormat;
use crate::cli::{success, CliResponse};
use crate::error::Result;
use serde::Serialize;

/// Output response based on format
fn output_result<T: Serialize>(response: &CliResponse<T>, _format: &OutputFormat) {
    // For now, always output JSON (table/plain support can be added later)
    println!("{}", response.to_json_pretty());
}

/// Execute mkdir command
pub fn exec_mkdir(fs: &MountableFS, path: &str, mode: &str, format: &OutputFormat) -> Result<()> {
    let mode_val = u32::from_str_radix(mode, 8).unwrap_or(0o755);
    let internal_path = resolve_path(path)?;

    fs.route_operation(&internal_path, |plugin| {
        plugin.mkdir(&internal_path, mode_val)
    })?;

    let response = success(serde_json::json!({
        "operation": "mkdir",
        "path": path
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute ls command
pub fn exec_ls(
    fs: &MountableFS,
    path: &str,
    _recursive: bool,
    format: &OutputFormat,
) -> Result<()> {
    let internal_path = resolve_path(path)?;

    let entries = fs.route_operation(&internal_path, |plugin| plugin.read_dir(&internal_path))?;

    let entries_json: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "size": e.size,
                "is_dir": e.is_dir,
                "mode": format!("{:o}", e.mode),
            })
        })
        .collect();

    let response = success(serde_json::json!({
        "path": path,
        "entries": entries_json
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute cat command
pub fn exec_cat(fs: &MountableFS, path: &str, format: &OutputFormat) -> Result<()> {
    let internal_path = resolve_path(path)?;

    let data = fs.route_operation(&internal_path, |plugin| plugin.read(&internal_path, 0, 0))?;

    let text = String::from_utf8_lossy(&data);
    let response = success(serde_json::json!({
        "path": path,
        "data": text,
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute write command
pub fn exec_write(fs: &MountableFS, path: &str, data: &str, format: &OutputFormat) -> Result<()> {
    let internal_path = resolve_path(path)?;

    let bytes_written = fs.route_operation(&internal_path, |plugin| {
        plugin.write(&internal_path, data.as_bytes(), 0, WriteFlag::CREATE)
    })?;

    let response = success(serde_json::json!({
        "operation": "write",
        "path": path,
        "bytes_written": bytes_written,
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute rm command
pub fn exec_rm(fs: &MountableFS, path: &str, recursive: bool, format: &OutputFormat) -> Result<()> {
    let internal_path = resolve_path(path)?;

    if recursive {
        fs.route_operation(&internal_path, |plugin| plugin.remove_all(&internal_path))?;
    } else {
        fs.route_operation(&internal_path, |plugin| plugin.remove(&internal_path))?;
    }

    let response = success(serde_json::json!({
        "operation": "rm",
        "path": path,
    }));
    output_result(&response, format);
    Ok(())
}

/// Execute stat command
pub fn exec_stat(fs: &MountableFS, path: &str, format: &OutputFormat) -> Result<()> {
    let internal_path = resolve_path(path)?;

    let info = fs.route_operation(&internal_path, |plugin| plugin.stat(&internal_path))?;

    let response = success(serde_json::json!({
        "path": path,
        "name": info.name,
        "size": info.size,
        "is_dir": info.is_dir,
        "mode": format!("{:o}", info.mode),
        "created_at": info.created_at,
        "updated_at": info.updated_at,
    }));
    output_result(&response, format);
    Ok(())
}

/// Resolve path - convert Viking URI to internal path, or use as-is
fn resolve_path(path: &str) -> Result<String> {
    if path.starts_with("viking://") {
        let uri = VikingUri::parse(path)?;
        Ok(uri.to_internal_path())
    } else {
        Ok(path.to_string())
    }
}
