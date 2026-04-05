use std::path::PathBuf;

use anyhow::Context;
use tracing_subscriber::EnvFilter;

use second_brain::core::db::Db;
use second_brain::core::embeddings::Embedder;
use second_brain::core::mcp_tools::SecondBrainServer;
use second_brain::core::project::infer_project_id;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // All logging to stderr — stdout is MCP transport
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("second-brain-mcp starting");

    // Open database
    let db_path = Db::default_path();
    let db = Db::open(&db_path)
        .with_context(|| format!("failed to open database at {}", db_path.display()))?;

    // Initialize embedder (downloads model on first run)
    tracing::info!("loading embedding model...");
    let embedder = Embedder::new().context("failed to initialize embedding model")?;
    tracing::info!("embedding model loaded");

    // Infer project ID from current directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    tracing::info!(project_id = %project_id, "project identified");

    // Create and start MCP server
    let server = SecondBrainServer::new(db, embedder, project_id);
    let transport = rmcp::transport::io::stdio();
    let service = rmcp::serve_server(server, transport).await?;
    service.waiting().await?;

    Ok(())
}
