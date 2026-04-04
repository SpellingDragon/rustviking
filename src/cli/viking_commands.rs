//! VikingFS command handlers

use crate::cli::commands::OutputFormat;
use crate::cli::{success, CliResponse};
use crate::error::Result;
use crate::vikingfs::VikingFS;
use serde::Serialize;

/// 解析 level 字符串为 u8
fn parse_level(level: &str) -> Option<u8> {
    match level.to_uppercase().as_str() {
        "L0" | "0" => Some(0),
        "L1" | "1" => Some(1),
        "L2" | "2" => Some(2),
        _ => None,
    }
}

/// Output response in JSON format
fn output_json_response<T: Serialize>(response: &CliResponse<T>) {
    println!("{}", response.to_json_pretty());
}

/// 输出纯文本内容
fn output_plain(content: &str) {
    println!("{}", content);
}

/// 输出表格格式（用于列表结果）
fn output_table(headers: &[&str], rows: &[Vec<String>]) {
    // 简单的表格输出
    println!("{}", headers.join(" | "));
    println!("{}", "-".repeat(50));
    for row in rows {
        println!("{}", row.join(" | "));
    }
}

pub async fn handle_read(
    vikingfs: &VikingFS,
    uri: &str,
    level: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let content = match level.and_then(parse_level) {
        Some(0) => vikingfs.read_abstract(uri)?,
        Some(1) => vikingfs.read_overview(uri)?,
        _ => {
            let bytes = vikingfs.read(uri)?;
            String::from_utf8_lossy(&bytes).to_string()
        }
    };

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "level": level,
                "content": content
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            output_plain(&content);
        }
        OutputFormat::Table => {
            output_plain(&content);
        }
    }
    Ok(())
}

pub async fn handle_write(
    vikingfs: &VikingFS,
    uri: &str,
    data: &str,
    auto_summary: bool,
    output_format: &OutputFormat,
) -> Result<()> {
    if auto_summary {
        vikingfs.write_context(uri, data.as_bytes(), true).await?;
    } else {
        vikingfs.write(uri, data.as_bytes()).await?;
    }

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "auto_summary": auto_summary,
                "bytes_written": data.len()
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Written {} bytes to {}", data.len(), uri);
        }
        OutputFormat::Table => {
            println!("Written {} bytes to {}", data.len(), uri);
        }
    }
    Ok(())
}

pub async fn handle_mkdir(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    vikingfs.mkdir(uri)?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "operation": "mkdir"
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Created directory: {}", uri);
        }
        OutputFormat::Table => {
            println!("Created directory: {}", uri);
        }
    }
    Ok(())
}

pub async fn handle_rm(
    vikingfs: &VikingFS,
    uri: &str,
    recursive: bool,
    output_format: &OutputFormat,
) -> Result<()> {
    vikingfs.rm(uri, recursive).await?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "recursive": recursive,
                "operation": "rm"
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Removed: {} (recursive: {})", uri, recursive);
        }
        OutputFormat::Table => {
            println!("Removed: {} (recursive: {})", uri, recursive);
        }
    }
    Ok(())
}

pub async fn handle_mv(
    vikingfs: &VikingFS,
    from: &str,
    to: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    vikingfs.mv(from, to).await?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "from": from,
                "to": to,
                "operation": "mv"
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Moved: {} -> {}", from, to);
        }
        OutputFormat::Table => {
            println!("Moved: {} -> {}", from, to);
        }
    }
    Ok(())
}

pub async fn handle_ls(
    vikingfs: &VikingFS,
    uri: &str,
    _recursive: bool,
    output_format: &OutputFormat,
) -> Result<()> {
    let entries = vikingfs.ls(uri)?;

    match output_format {
        OutputFormat::Json => {
            let entries_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "size": e.size,
                        "is_dir": e.is_dir,
                        "mode": format!("{:o}", e.mode),
                        "created_at": e.created_at,
                        "updated_at": e.updated_at,
                    })
                })
                .collect();
            let response = success(serde_json::json!({
                "uri": uri,
                "entries": entries_json
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            for entry in &entries {
                let type_indicator = if entry.is_dir { "d" } else { "-" };
                println!("{}{:>10} {}", type_indicator, entry.size, entry.name);
            }
        }
        OutputFormat::Table => {
            let mut rows = Vec::new();
            for entry in &entries {
                let type_indicator = if entry.is_dir { "d" } else { "-" };
                rows.push(vec![
                    type_indicator.to_string(),
                    entry.size.to_string(),
                    entry.name.clone(),
                ]);
            }
            output_table(&["Type", "Size", "Name"], &rows);
        }
    }
    Ok(())
}

pub async fn handle_stat(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    let info = vikingfs.stat(uri)?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "name": info.name,
                "size": info.size,
                "is_dir": info.is_dir,
                "mode": format!("{:o}", info.mode),
                "created_at": info.created_at,
                "updated_at": info.updated_at,
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Name: {}", info.name);
            println!("Size: {}", info.size);
            println!("Is Dir: {}", info.is_dir);
            println!("Mode: {:o}", info.mode);
            println!("Created: {}", info.created_at);
            println!("Updated: {}", info.updated_at);
        }
        OutputFormat::Table => {
            let rows = vec![
                vec!["Name".to_string(), info.name.clone()],
                vec!["Size".to_string(), info.size.to_string()],
                vec!["Is Dir".to_string(), info.is_dir.to_string()],
                vec!["Mode".to_string(), format!("{:o}", info.mode)],
                vec!["Created".to_string(), info.created_at.to_string()],
                vec!["Updated".to_string(), info.updated_at.to_string()],
            ];
            output_table(&["Property", "Value"], &rows);
        }
    }
    Ok(())
}

pub async fn handle_abstract(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    let content = vikingfs.read_abstract(uri)?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "level": "L0",
                "abstract": content
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            output_plain(&content);
        }
        OutputFormat::Table => {
            output_plain(&content);
        }
    }
    Ok(())
}

pub async fn handle_overview(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    let content = vikingfs.read_overview(uri)?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "level": "L1",
                "overview": content
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            output_plain(&content);
        }
        OutputFormat::Table => {
            output_plain(&content);
        }
    }
    Ok(())
}

pub async fn handle_detail(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    let bytes = vikingfs.read_detail(uri)?;
    let content = String::from_utf8_lossy(&bytes);

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "level": "L2",
                "content": content.to_string()
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            output_plain(&content);
        }
        OutputFormat::Table => {
            output_plain(&content);
        }
    }
    Ok(())
}

pub async fn handle_find(
    vikingfs: &VikingFS,
    query: &str,
    target: Option<&str>,
    k: usize,
    level: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let level_num = level.and_then(parse_level);
    let results = vikingfs.find(query, target, k, level_num).await?;

    match output_format {
        OutputFormat::Json => {
            let results_json: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "uri": r.uri,
                        "score": r.score,
                        "level": r.level,
                        "abstract": r.abstract_text,
                    })
                })
                .collect();
            let response = success(serde_json::json!({
                "query": query,
                "target": target,
                "level": level,
                "results": results_json
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Search results for: {}", query);
            for (i, result) in results.iter().enumerate() {
                println!(
                    "{}. [{}] {} (score: {:.4})",
                    i + 1,
                    result.level,
                    result.uri,
                    result.score
                );
                if let Some(abstract_text) = &result.abstract_text {
                    println!(
                        "   Abstract: {}",
                        abstract_text.chars().take(100).collect::<String>()
                    );
                }
            }
        }
        OutputFormat::Table => {
            let mut rows = Vec::new();
            for result in &results {
                let abstract_short = result
                    .abstract_text
                    .as_ref()
                    .map(|a| a.chars().take(50).collect::<String>() + "...")
                    .unwrap_or_default();
                rows.push(vec![
                    result.id.clone(),
                    result.uri.clone(),
                    format!("{:.4}", result.score),
                    result.level.to_string(),
                    abstract_short,
                ]);
            }
            output_table(&["ID", "URI", "Score", "Level", "Abstract"], &rows);
        }
    }
    Ok(())
}

pub async fn handle_commit(
    vikingfs: &VikingFS,
    uri: &str,
    output_format: &OutputFormat,
) -> Result<()> {
    vikingfs.commit(uri).await?;

    match output_format {
        OutputFormat::Json => {
            let response = success(serde_json::json!({
                "uri": uri,
                "operation": "commit"
            }));
            output_json_response(&response);
        }
        OutputFormat::Plain => {
            println!("Committed directory: {}", uri);
        }
        OutputFormat::Table => {
            println!("Committed directory: {}", uri);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level() {
        assert_eq!(parse_level("L0"), Some(0));
        assert_eq!(parse_level("l0"), Some(0));
        assert_eq!(parse_level("0"), Some(0));
        assert_eq!(parse_level("L1"), Some(1));
        assert_eq!(parse_level("l1"), Some(1));
        assert_eq!(parse_level("1"), Some(1));
        assert_eq!(parse_level("L2"), Some(2));
        assert_eq!(parse_level("l2"), Some(2));
        assert_eq!(parse_level("2"), Some(2));
        assert_eq!(parse_level("invalid"), None);
        assert_eq!(parse_level("L3"), None);
    }
}
