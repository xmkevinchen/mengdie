use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};

use super::db::Db;
use super::metrics;
use super::embeddings::Embedder;
use std::sync::{Arc, Mutex};

/// MCP server with 3 tools: memory_search, memory_ingest, memory_invalidate.
pub struct MengdieServer {
    tool_router: ToolRouter<Self>,
    db: Db,
    embedder: Arc<Mutex<Embedder>>,
    default_project_id: String,
}

// -- Tool parameter types --

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// The search query text.
    pub query: String,
    /// Search scope: omit for current project, "global" for all projects.
    pub scope: Option<String>,
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// Maximum number of results to return (default: 10).
    pub limit: Option<usize>,
    /// Minimum score threshold — results below this are filtered out (0.0-1.0).
    pub min_score: Option<f64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct IngestParams {
    /// Title of the memory entry.
    pub title: String,
    /// Full content of the memory.
    pub content: String,
    /// Source file path (optional — omit or pass empty string if not applicable).
    #[serde(default)]
    pub source_file: String,
    /// Source type: conclusion, review, plan, retrospect.
    pub source_type: String,
    /// Knowledge type: decisional, experiential, factual.
    pub knowledge_type: String,
    /// Comma-separated entity tags.
    pub entities: String,
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
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
    /// Non-empty if search ran in degraded mode (e.g., embedding failed, FTS-only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<String>,
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
    /// Non-empty if ingestion failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

// -- Input validation --

const MAX_QUERY_LEN: usize = 10_000;
const MAX_CONTENT_LEN: usize = 100_000;
const MAX_FIELD_LEN: usize = 1_000;

// -- Tool implementations --

#[tool_router]
impl MengdieServer {
    #[tool(
        name = "memory_search",
        description = "Search Mengdie memories. Returns relevant memories with provenance."
    )]
    async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Json<SearchOutput> {
        if params.query.len() > MAX_QUERY_LEN {
            return Json(SearchOutput {
                results: vec![],
                degraded: Some(format!("query too long (max {MAX_QUERY_LEN} chars)")),
            });
        }
        let pid = params
            .project_id
            .as_deref()
            .unwrap_or(&self.default_project_id);
        let project_id = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid),
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

        let limit = params.limit.unwrap_or(10);
        let min_score = params.min_score.unwrap_or(0.0);

        let (results, degraded) = match query_embedding {
            Ok(embedding) => {
                match self
                    .db
                    .memory_search(&params.query, &embedding, project_id, limit)
                {
                    Ok(results) => (results, None),
                    Err(e) => {
                        tracing::error!(error = %e, "memory_search failed");
                        (vec![], Some("search temporarily unavailable".into()))
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "embedding failed, falling back to FTS-only");
                // FTS-only fallback
                match self.db.search_fts(&params.query, project_id, limit) {
                    Ok(fts_results) => {
                        let mut results = Vec::new();
                        for fts in &fts_results {
                            if let Ok(Some(entry)) = self.db.get_memory(&fts.id) {
                                results.push(super::search::SearchResult {
                                    entry,
                                    score: fts.bm25_score.abs(),
                                });
                            }
                        }
                        (results, Some("degraded: embedding unavailable, FTS-only".into()))
                    }
                    Err(e2) => {
                        tracing::error!(error = %e2, "FTS fallback also failed");
                        (vec![], Some("search temporarily unavailable".into()))
                    }
                }
            }
        };

        let items: Vec<SearchResultItem> = results
            .into_iter()
            .filter(|r| r.score >= min_score)
            .map(|r| {
                let snippet = r.entry.content.chars().take(200).collect::<String>();
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
            .collect();

        // Track metrics
        let _ = self.db.increment_metric(metrics::METRIC_SEARCH_COUNT);
        if !items.is_empty() {
            let _ = self.db.increment_metric(metrics::METRIC_SEARCH_NONEMPTY);
        }

        Json(SearchOutput { results: items, degraded })
    }

    #[tool(
        name = "memory_ingest",
        description = "Ingest a new memory into Mengdie. Returns entry ID and any detected conflicts."
    )]
    async fn ingest(&self, Parameters(params): Parameters<IngestParams>) -> Json<IngestOutput> {
        if params.content.len() > MAX_CONTENT_LEN {
            return Json(IngestOutput {
                entry_id: String::new(),
                conflicts: vec![],
                error: Some(format!("content too long (max {MAX_CONTENT_LEN} chars)")),
            });
        }
        if params.title.len() > MAX_FIELD_LEN || params.entities.len() > MAX_FIELD_LEN {
            return Json(IngestOutput {
                entry_id: String::new(),
                conflicts: vec![],
                error: Some(format!("field too long (max {MAX_FIELD_LEN} chars)")),
            });
        }
        // Validate and normalize source_type / knowledge_type
        let source_type = super::parser::validate_source_type(&params.source_type).to_string();
        let knowledge_type = super::parser::validate_knowledge_type(&params.knowledge_type).to_string();
        if source_type != params.source_type {
            tracing::warn!(given = %params.source_type, normalized = %source_type, "unknown source_type, defaulting");
        }
        if knowledge_type != params.knowledge_type {
            tracing::warn!(given = %params.knowledge_type, normalized = %knowledge_type, "unknown knowledge_type, defaulting");
        }

        let pid = params
            .project_id
            .clone()
            .unwrap_or_else(|| self.default_project_id.clone());
        let embedder = self.embedder.clone();
        let ctx = super::embeddings::EmbeddingContext {
            knowledge_type: knowledge_type.clone(),
            entities: params.entities.clone(),
            project_id: pid.clone(),
            title: params.title.clone(),
        };
        let content = params.content.clone();
        let embedding_result = tokio::task::spawn_blocking(move || {
            let mut emb = embedder.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            emb.embed_with_context(&content, &ctx)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn: {e}")));

        let raw_embedding: Option<Vec<f32>> = match embedding_result {
            Ok(emb) => Some(emb),
            Err(e) => {
                tracing::error!(error = %e, "embedding failed, storing without embedding");
                None
            }
        };

        let (embedding_blob, embedding_dim) = match &raw_embedding {
            Some(emb) => {
                let dim = emb.len() as i64;
                (Some(super::embeddings::embedding_to_blob(emb)), Some(dim))
            }
            None => (None, None),
        };

        // Capture for contradiction check before move into NewMemory
        let entities_for_check: Vec<String> = params.entities
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let knowledge_type_for_check = knowledge_type.clone();

        // Contradiction check BEFORE insert (so we don't match the new entry against itself)
        let embedding_for_check = raw_embedding.as_deref();
        let conflicts = match self.db.check_contradictions(
            &entities_for_check,
            embedding_for_check,
            &knowledge_type_for_check,
            &pid,
        ) {
            Ok(cs) => cs
                .into_iter()
                .map(|c| ConflictItem {
                    id: c.existing_id,
                    title: c.existing_title,
                    reason: c.reason.to_string(),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "contradiction check failed");
                vec![]
            }
        };

        let mem = super::db::NewMemory {
            project_id: pid.clone(),
            source_file: params.source_file,
            source_type,
            knowledge_type,
            title: params.title,
            content: params.content,
            entities: params.entities,
            embedding: embedding_blob,
            embedding_dim,
        };

        match self.db.insert_memory(mem) {
            Ok(entry_id) => {
                // Track metrics
                let _ = self.db.increment_metric(metrics::METRIC_INGEST_COUNT);
                if !conflicts.is_empty() {
                    let _ = self.db.increment_metric(metrics::METRIC_CONFLICT_COUNT);
                }
                Json(IngestOutput {
                    entry_id,
                    conflicts,
                    error: None,
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "memory_ingest failed");
                Json(IngestOutput {
                    entry_id: String::new(),
                    conflicts: vec![],
                    error: Some("ingestion failed".into()),
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

impl MengdieServer {
    pub fn new(db: Db, embedder: Embedder, default_project_id: String) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            embedder: Arc::new(Mutex::new(embedder)),
            default_project_id,
        }
    }
}

#[rmcp::tool_handler]
impl ServerHandler for MengdieServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("AI-native Mengdie — knowledge management for AI development workflows. Tools: memory_search, memory_ingest, memory_invalidate.")
    }
}
