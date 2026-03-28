//! RustViking CLI Entry Point

use clap::Parser;
use std::sync::Arc;

use rustviking::cli::commands::*;
use rustviking::cli::{fs_commands, store_commands, index_commands};
use rustviking::config::Config;
use rustviking::agfs::MountableFS;
use rustviking::plugins::localfs::LocalFSPlugin;
use rustviking::plugins::memory::MemoryPlugin;
use rustviking::storage::RocksKvStore;
use rustviking::index::{IvfPqIndex, IvfPqParams, MetricType};


fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustviking=info".parse().unwrap())
        )
        .init();

    let cli = Cli::parse();
    
    if let Err(e) = run(cli) {
        let error_output = serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        });
        eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_default());
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> rustviking::error::Result<()> {
    // Load config
    let config = Config::load_or_default(&cli.config);
    
    match cli.command {
        Commands::Fs { operation } => {
            // Initialize AGFS with plugins
            let agfs = setup_agfs(&config)?;
            handle_fs_command(&agfs, operation, &cli.output)
        }
        Commands::Kv { operation } => {
            let store = RocksKvStore::new(&config.storage)?;
            handle_kv_command(&store, operation, &cli.output)
        }
        Commands::Index { operation } => {
            let dimension = config.vector.as_ref().map(|v| v.dimension).unwrap_or(768);
            let params = IvfPqParams {
                num_partitions: 256,
                num_sub_vectors: 16,
                pq_bits: 8,
                metric: MetricType::L2,
            };
            let index = IvfPqIndex::new(params, dimension);
            handle_index_command(&index, operation, &cli.output)
        }
        Commands::Server { operation } => {
            handle_server_command(operation)
        }
        Commands::Bench { test, count } => {
            handle_bench_command(test, count)
        }
    }
}

fn setup_agfs(config: &Config) -> rustviking::error::Result<MountableFS> {
    let agfs = MountableFS::new();
    
    // Mount local filesystem
    let local_path = format!("{}/local", config.storage.path);
    let local_plugin = LocalFSPlugin::new(&local_path)?;
    agfs.mount("/local", Arc::new(local_plugin), 100)?;
    
    // Mount memory filesystem
    let mem_plugin = MemoryPlugin::new();
    agfs.mount("/memory", Arc::new(mem_plugin), 50)?;
    
    // Mount default resource paths
    let resources_path = format!("{}/resources", config.storage.path);
    let resources_plugin = LocalFSPlugin::new(&resources_path)?;
    agfs.mount("/resources", Arc::new(resources_plugin), 100)?;
    
    let user_path = format!("{}/user", config.storage.path);
    let user_plugin = LocalFSPlugin::new(&user_path)?;
    agfs.mount("/user", Arc::new(user_plugin), 100)?;
    
    let agent_path = format!("{}/agent", config.storage.path);
    let agent_plugin = LocalFSPlugin::new(&agent_path)?;
    agfs.mount("/agent", Arc::new(agent_plugin), 100)?;
    
    Ok(agfs)
}

fn handle_fs_command(agfs: &MountableFS, op: FsOperation, format: &OutputFormat) -> rustviking::error::Result<()> {
    match op {
        FsOperation::Mkdir { path, mode } => fs_commands::exec_mkdir(agfs, &path, &mode, format),
        FsOperation::Ls { path, recursive } => fs_commands::exec_ls(agfs, &path, recursive, format),
        FsOperation::Cat { path } => fs_commands::exec_cat(agfs, &path, format),
        FsOperation::Write { path, data } => fs_commands::exec_write(agfs, &path, &data, format),
        FsOperation::Rm { path, recursive } => fs_commands::exec_rm(agfs, &path, recursive, format),
        FsOperation::Stat { path } => fs_commands::exec_stat(agfs, &path, format),
    }
}

fn handle_kv_command(store: &RocksKvStore, op: KvOperation, format: &OutputFormat) -> rustviking::error::Result<()> {
    match op {
        KvOperation::Get { key } => store_commands::exec_kv_get(store, &key, format),
        KvOperation::Put { key, value } => store_commands::exec_kv_put(store, &key, &value, format),
        KvOperation::Del { key } => store_commands::exec_kv_del(store, &key, format),
        KvOperation::Scan { prefix, limit } => store_commands::exec_kv_scan(store, &prefix, limit, format),
    }
}

fn handle_index_command(index: &IvfPqIndex, op: IndexOperation, format: &OutputFormat) -> rustviking::error::Result<()> {
    match op {
        IndexOperation::Insert { id, vector, level } => {
            index_commands::exec_index_insert(index, id, &vector, level, format)
        }
        IndexOperation::Search { query, k, level } => {
            index_commands::exec_index_search(index, &query, k, level, format)
        }
        IndexOperation::Delete { id } => {
            index_commands::exec_index_delete(index, id, format)
        }
        IndexOperation::Info {} => {
            index_commands::exec_index_info(index, format)
        }
    }
}

fn handle_server_command(op: ServerOperation) -> rustviking::error::Result<()> {
    match op {
        ServerOperation::Start { port } => {
            println!("{}", serde_json::json!({"status": "info", "message": format!("Server mode not yet implemented. Port: {}", port)}));
            Ok(())
        }
        ServerOperation::Stop {} => {
            println!("{}", serde_json::json!({"status": "info", "message": "Server stop not yet implemented"}));
            Ok(())
        }
        ServerOperation::Status {} => {
            println!("{}", serde_json::json!({"status": "info", "message": "Server status not yet implemented"}));
            Ok(())
        }
    }
}

fn handle_bench_command(test: BenchTest, count: usize) -> rustviking::error::Result<()> {
    println!("{}", serde_json::json!({
        "status": "info",
        "message": format!("Benchmark not yet implemented. Test: {:?}, Count: {}", 
            match test {
                BenchTest::KvWrite => "kv-write",
                BenchTest::KvRead => "kv-read",
                BenchTest::VectorSearch => "vector-search",
                BenchTest::BitmapOps => "bitmap-ops",
            },
            count
        )
    }));
    Ok(())
}
