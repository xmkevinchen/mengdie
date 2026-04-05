use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};

use super::db::Db;
use super::embeddings::Embedder;
use std::sync::{Arc, Mutex};

/// MCP server with 3 tools: memory_search, memory_ingest, memory_invalidate.
pub struct SecondBrainServer {
    tool_router: ToolRouter<Self>,
    db: Db,
    embedder: Arc<Mutex<Embedder>>,
    project_id: String,
}

// -- Tool parameter types --

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// The search query text.
    pub query: String,
    /// Search scope: omit for current project, "global" for all projects.
    pub scope: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct IngestParams {
    /// Title of the memory entry.
    pub title: String,
    /// Full content of the memory.
    pub content: String,
    /// Source file path.
    pub source_file: String,
    /// Source type: conclusion, review, plan, retrospect.
    pub source_type: String,
    /// Knowledge type: decisional, experiential, factual.
    pub knowledge_type: String,
    /// Comma-separated entity tags.
    pub entities: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct InvalidateParams {
    /// ID of the memory entry to invalidate.
    pub entry_id: String,
    /// Reason for invalidation.
    pub reason: String,
    /// Optional ID of the memory that supersedes this one.
    pub superseded_by: Option<String>,
}

// -- Tool output types --

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchOutput {
    pub results: Vec<SearchResultItem>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchResultItem {
    pub id: String,
    pub title: String,
    pub source_file: String,
    pub knowledge_type: String,
    pub entities: String,
    pub score: f64,
    pub valid_from: String,
    pub snippet: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct IngestOutput {
    pub entry_id: String,
    pub conflicts: Vec<ConflictItem>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ConflictItem {
    pub id: String,
    pub title: String,
    pub reason: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InvalidateOutput {
    pub success: bool,
    pub entry_id: String,
}

// -- Tool implementations --

#[tool_router]
impl SecondBrainServer {
    #[tool(
        name = "memory_search",
        description = "Search Second Brain memories. Returns relevant memories with provenance."
    )]
    async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Json<SearchOutput> {
        let project_id = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(self.project_id.as_str()),
        };

        // Generate query embedding (blocking → thread pool)
        let query = params.query.clone();
        let embedder = self.embedder.clone();
        let query_embedding = tokio::task::spawn_blocking(move || {
            let mut emb = embedder.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            emb.embed_text(&query)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn: {e}")));

        let results = match query_embedding {
            Ok(embedding) => {
                match self
                    .db
                    .memory_search(&params.query, &embedding, project_id, 10)
                {
                    Ok(results) => results
                        .into_iter()
                        .map(|r| {
                            let snippet =
                                r.entry.content.chars().take(200).collect::<String>();
                            SearchResultItem {
                                id: r.entry.id,
                                title: r.entry.title,
                                source_file: r.entry.source_file,
                                knowledge_type: r.entry.knowledge_type,
                                entities: r.entry.entities,
                                score: r.score,
                                valid_from: r.entry.valid_from,
                                snippet,
                            }
                        })
                        .collect(),
                    Err(e) => {
                        tracing::error!(error = %e, "memory_search failed");
                        vec![]
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "embedding failed, returning empty results");
                vec![]
            }
        };

        Json(SearchOutput { results })
    }

    #[tool(
        name = "memory_ingest",
        description = "Ingest a new memory into Second Brain. Returns entry ID and any detected conflicts."
    )]
    async fn ingest(&self, Parameters(params): Parameters<IngestParams>) -> Json<IngestOutput> {
        let embedder = self.embedder.clone();
        let ctx = super::embeddings::EmbeddingContext {
            knowledge_type: params.knowledge_type.clone(),
            entities: params.entities.clone(),
            project_id: self.project_id.clone(),
            title: params.title.clone(),
        };
        let content = params.content.clone();
        let embedding_result = tokio::task::spawn_blocking(move || {
            let mut emb = embedder.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            emb.embed_with_context(&content, &ctx)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn: {e}")));

        let (embedding_blob, embedding_dim) = match embedding_result {
            Ok(emb) => {
                let dim = emb.len() as i64;
                (Some(super::embeddings::embedding_to_blob(&emb)), Some(dim))
            }
            Err(e) => {
                tracing::error!(error = %e, "embedding failed, storing without embedding");
                (None, None)
            }
        };

        let mem = super::db::NewMemory {
            project_id: self.project_id.clone(),
            source_file: params.source_file,
            source_type: params.source_type,
            knowledge_type: params.knowledge_type,
            title: params.title,
            content: params.content,
            entities: params.entities,
            embedding: embedding_blob,
            embedding_dim,
        };

        match self.db.insert_memory(mem) {
            Ok(entry_id) => {
                // TODO: contradiction detection (Step 5b)
                Json(IngestOutput {
                    entry_id,
                    conflicts: vec![],
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "memory_ingest failed");
                Json(IngestOutput {
                    entry_id: String::new(),
                    conflicts: vec![],
                })
            }
        }
    }

    #[tool(
        name = "memory_invalidate",
        description = "Mark a memory as invalid. Optionally link to the superseding memory."
    )]
    async fn invalidate(
        &self,
        Parameters(params): Parameters<InvalidateParams>,
    ) -> Json<InvalidateOutput> {
        match self
            .db
            .invalidate_memory(&params.entry_id, params.superseded_by.as_deref())
        {
            Ok(updated) => Json(InvalidateOutput {
                success: updated,
                entry_id: params.entry_id,
            }),
            Err(e) => {
                tracing::error!(error = %e, "memory_invalidate failed");
                Json(InvalidateOutput {
                    success: false,
                    entry_id: params.entry_id,
                })
            }
        }
    }
}

impl SecondBrainServer {
    pub fn new(db: Db, embedder: Embedder, project_id: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            embedder: Arc::new(Mutex::new(embedder)),
            project_id,
        }
    }
}

impl ServerHandler for SecondBrainServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("AI-native Second Brain — knowledge management for AI development workflows. Tools: memory_search, memory_ingest, memory_invalidate.")
    }
}
