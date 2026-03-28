//! CLI End-to-End Tests
//!
//! Tests for CLI commands using assert_cmd.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("RustViking"))
        .stdout(predicate::str::contains("Usage"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rustviking"))
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_cli_no_command() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // No subcommand should show help and fail
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage")
            .or(predicate::str::contains("Commands")));
}

#[test]
fn test_cli_fs_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["fs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Filesystem"))
        .stdout(predicate::str::contains("mkdir"))
        .stdout(predicate::str::contains("ls"))
        .stdout(predicate::str::contains("cat"));
}

#[test]
fn test_cli_kv_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["kv", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Key-value"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("put"))
        .stdout(predicate::str::contains("del"));
}

#[test]
fn test_cli_index_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["index", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vector"))
        .stdout(predicate::str::contains("insert"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_cli_server_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["server", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Server"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("stop"))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_cli_bench_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["bench", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmark"))
        .stdout(predicate::str::contains("kv-write"))
        .stdout(predicate::str::contains("vector-search"));
}

#[test]
fn test_cli_invalid_command() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.arg("invalid_command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error")
            .or(predicate::str::contains("unrecognized")));
}

#[test]
fn test_cli_fs_mkdir_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["fs", "mkdir", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create directory"))
        .stdout(predicate::str::contains("path"))
        .stdout(predicate::str::contains("mode"));
}

#[test]
fn test_cli_kv_put_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["kv", "put", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set key-value"))
        .stdout(predicate::str::contains("-k"))
        .stdout(predicate::str::contains("-v"));
}

#[test]
fn test_cli_index_insert_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["index", "insert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Insert a vector"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("--vector"));
}

#[test]
fn test_cli_index_search_help() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["index", "search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Search for similar"))
        .stdout(predicate::str::contains("--query"))
        .stdout(predicate::str::contains("-k"));
}

#[test]
fn test_cli_output_format_option() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-o"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn test_cli_config_option() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config file"))
        .stdout(predicate::str::contains("-c"));
}

#[test]
fn test_cli_server_start() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // Server start should succeed (even though it's not fully implemented)
    cmd.args(["server", "start"])
        .assert()
        .success()
        .stdout(predicate::str::contains("info")
            .or(predicate::str::contains("not yet implemented")));
}

#[test]
fn test_cli_server_status() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["server", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("info")
            .or(predicate::str::contains("not yet implemented")));
}

#[test]
fn test_cli_bench_command() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["bench", "kv-write", "-c", "10"])
        .assert()
        .success()
        .stdout(predicate::str::contains("info")
            .or(predicate::str::contains("Benchmark")));
}

#[test]
fn test_cli_invalid_bench_test() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["bench", "invalid-test"])
        .assert()
        .failure();
}

#[test]
fn test_cli_fs_ls_missing_path() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // Missing required path argument
    cmd.args(["fs", "ls"])
        .assert()
        .failure();
}

#[test]
fn test_cli_kv_get_missing_key() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // Missing required key option
    cmd.args(["kv", "get"])
        .assert()
        .failure();
}

#[test]
fn test_cli_index_insert_missing_vector() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // Missing required vector option
    cmd.args(["index", "insert", "--id", "1"])
        .assert()
        .failure();
}

#[test]
fn test_cli_index_search_missing_query() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    // Missing required query option
    cmd.args(["index", "search", "-k", "10"])
        .assert()
        .failure();
}

#[test]
fn test_cli_output_format_json() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["-o", "json", "server", "status"])
        .assert()
        .success();
}

#[test]
fn test_cli_output_format_table() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["-o", "table", "server", "status"])
        .assert()
        .success();
}

#[test]
fn test_cli_output_format_plain() {
    let mut cmd = Command::cargo_bin("rustviking").unwrap();
    
    cmd.args(["-o", "plain", "server", "status"])
        .assert()
        .success();
}
