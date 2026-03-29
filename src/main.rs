//! RustViking CLI Entry Point

use clap::Parser;

use rustviking::agfs::MountableFS;
use rustviking::cli::commands::*;
use rustviking::cli::{fs_commands, index_commands, store_commands, viking_commands};
use rustviking::vikingfs::VikingFS;
use rustviking::config::Config;
use rustviking::embedding::mock::MockEmbeddingProvider;
use rustviking::embedding::openai::OpenAIEmbeddingProvider;
use rustviking::embedding::types::EmbeddingConfig;
use rustviking::embedding::EmbeddingProvider;
use rustviking::index::{IvfIndex, IvfParams, MetricType};
use rustviking::plugins::PluginRegistry;
use rustviking::storage::RocksKvStore;
use rustviking::vector_store::memory::MemoryVectorStore;
use rustviking::vector_store::rocks::RocksDBVectorStore;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustviking=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        let error_output = serde_json::json!({
            "status": "error",
            "message": e.to_string(),
        });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&error_output).unwrap_or_default()
        );
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> rustviking::error::Result<()> {
    // Load config
    let config = Config::load_or_default(&cli.config);

    // Initialize plugin registry
    let mut plugin_registry = PluginRegistry::new();

    // Register VectorStore plugins
    plugin_registry.register_vector_store("memory", || Box::new(MemoryVectorStore::new()));

    // Register RocksDB VectorStore plugin with configured path
    let rocksdb_path = config
        .vector_store
        .rocksdb
        .as_ref()
        .map(|c| c.path.clone())
        .unwrap_or_else(|| format!("{}/vector_store", config.storage.path));
    plugin_registry.register_vector_store("rocksdb", move || {
        match RocksDBVectorStore::with_path(&rocksdb_path) {
            Ok(store) => Box::new(store) as Box<dyn rustviking::vector_store::VectorStore>,
            Err(e) => {
                tracing::error!("Failed to create RocksDBVectorStore: {}", e);
                // Fallback to memory store on error
                Box::new(MemoryVectorStore::new()) as Box<dyn rustviking::vector_store::VectorStore>
            }
        }
    });

    // Register EmbeddingProvider plugins
    plugin_registry
        .register_embedding_provider("mock", || Box::new(MockEmbeddingProvider::default()));

    // Register OpenAI EmbeddingProvider plugin
    plugin_registry
        .register_embedding_provider("openai", || Box::new(OpenAIEmbeddingProvider::new()));

    // Create instances based on config
    let vector_store = plugin_registry.create_vector_store(&config.vector_store.plugin)?;

    // Create and initialize embedding provider based on config
    let embedding_provider = if config.embedding.plugin == "openai" {
        let provider = OpenAIEmbeddingProvider::new();
        if let Some(openai_config) = &config.embedding.openai {
            let embedding_config = EmbeddingConfig {
                api_base: openai_config.api_base.clone(),
                api_key: Some(openai_config.api_key.clone()),
                provider: "openai".to_string(),
                model: openai_config.model.clone(),
                dimension: openai_config.dimension,
                max_concurrent: openai_config.max_concurrent,
            };
            provider.initialize(embedding_config)?;
        }
        Box::new(provider) as Box<dyn rustviking::embedding::EmbeddingProvider>
    } else {
        plugin_registry.create_embedding_provider(&config.embedding.plugin)?
    };

    tracing::info!(
        "Vector store: {}, Embedding provider: {}",
        vector_store.name(),
        embedding_provider.name()
    );

    match cli.command {
        Commands::Fs { operation } => {
            // Initialize AGFS with plugins
            let agfs = rustviking::agfs::setup_agfs(&config.storage.path)?;
            handle_fs_command(&agfs, operation, &cli.output)
        }
        Commands::Kv { operation } => {
            let store = RocksKvStore::new(&config.storage)?;
            handle_kv_command(&store, operation, &cli.output)
        }
        Commands::Index { operation } => {
            let dimension = config.vector.as_ref().map(|v| v.dimension).unwrap_or(768);
            let params = IvfParams {
                num_partitions: 256,
                metric: MetricType::L2,
            };
            let index = IvfIndex::new(params, dimension);
            handle_index_command(&index, operation, &cli.output)
        }
        Commands::Server { operation } => handle_server_command(operation),
        Commands::Bench { test, count } => handle_bench_command(test, count),

        // VikingFS 语义层命令
        Commands::Read { uri, level } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_read(&vikingfs, &uri, level.as_deref(), &cli.output)
        }
        Commands::Write { uri, data, auto_summary } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_write(&vikingfs, &uri, &data, auto_summary, &cli.output)
        }
        Commands::Mkdir { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_mkdir(&vikingfs, &uri, &cli.output)
        }
        Commands::Rm { uri, recursive } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_rm(&vikingfs, &uri, recursive, &cli.output)
        }
        Commands::Mv { from, to } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_mv(&vikingfs, &from, &to, &cli.output)
        }
        Commands::Ls { uri, recursive } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_ls(&vikingfs, &uri, recursive, &cli.output)
        }
        Commands::Stat { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_stat(&vikingfs, &uri, &cli.output)
        }
        Commands::Abstract { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_abstract(&vikingfs, &uri, &cli.output)
        }
        Commands::Overview { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_overview(&vikingfs, &uri, &cli.output)
        }
        Commands::Detail { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_detail(&vikingfs, &uri, &cli.output)
        }
        Commands::Find { query, target, k, level } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_find(&vikingfs, &query, target.as_deref(), k, level.as_deref(), &cli.output)
        }
        Commands::Commit { uri } => {
            let vikingfs = VikingFS::from_config(&config)?;
            viking_commands::handle_commit(&vikingfs, &uri, &cli.output)
        }
    }
}

fn handle_fs_command(
    agfs: &MountableFS,
    op: FsOperation,
    format: &OutputFormat,
) -> rustviking::error::Result<()> {
    match op {
        FsOperation::Mkdir { path, mode } => fs_commands::exec_mkdir(agfs, &path, &mode, format),
        FsOperation::Ls { path, recursive } => fs_commands::exec_ls(agfs, &path, recursive, format),
        FsOperation::Cat { path } => fs_commands::exec_cat(agfs, &path, format),
        FsOperation::Write { path, data } => fs_commands::exec_write(agfs, &path, &data, format),
        FsOperation::Rm { path, recursive } => fs_commands::exec_rm(agfs, &path, recursive, format),
        FsOperation::Stat { path } => fs_commands::exec_stat(agfs, &path, format),
    }
}

fn handle_kv_command(
    store: &RocksKvStore,
    op: KvOperation,
    format: &OutputFormat,
) -> rustviking::error::Result<()> {
    match op {
        KvOperation::Get { key } => store_commands::exec_kv_get(store, &key, format),
        KvOperation::Put { key, value } => store_commands::exec_kv_put(store, &key, &value, format),
        KvOperation::Del { key } => store_commands::exec_kv_del(store, &key, format),
        KvOperation::Scan { prefix, limit } => {
            store_commands::exec_kv_scan(store, &prefix, limit, format)
        }
    }
}

fn handle_index_command(
    index: &IvfIndex,
    op: IndexOperation,
    format: &OutputFormat,
) -> rustviking::error::Result<()> {
    match op {
        IndexOperation::Insert { id, vector, level } => {
            index_commands::exec_index_insert(index, id, &vector, level, format)
        }
        IndexOperation::Search { query, k, level } => {
            index_commands::exec_index_search(index, &query, k, level, format)
        }
        IndexOperation::Delete { id } => index_commands::exec_index_delete(index, id, format),
        IndexOperation::Info {} => index_commands::exec_index_info(index, format),
    }
}

fn handle_server_command(op: ServerOperation) -> rustviking::error::Result<()> {
    match op {
        ServerOperation::Start { port } => {
            println!(
                "{}",
                serde_json::json!({"status": "info", "message": format!("Server mode not yet implemented. Port: {}", port)})
            );
            Ok(())
        }
        ServerOperation::Stop {} => {
            println!(
                "{}",
                serde_json::json!({"status": "info", "message": "Server stop not yet implemented"})
            );
            Ok(())
        }
        ServerOperation::Status {} => {
            println!(
                "{}",
                serde_json::json!({"status": "info", "message": "Server status not yet implemented"})
            );
            Ok(())
        }
    }
}

fn handle_bench_command(test: BenchTest, count: usize) -> rustviking::error::Result<()> {
    println!(
        "{}",
        serde_json::json!({
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
        })
    );
    Ok(())
}
