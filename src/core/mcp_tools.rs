use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_router, ServerHandler};
use serde::{Deserialize, Serialize};

use super::db::Db;
use super::embeddings::Embedder;
use super::metrics;
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

/// Valid source types for memory entries.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Conclusion,
    Review,
    Plan,
    Retrospect,
    Synthesis,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conclusion => write!(f, "conclusion"),
            Self::Review => write!(f, "review"),
            Self::Plan => write!(f, "plan"),
            Self::Retrospect => write!(f, "retrospect"),
            Self::Synthesis => write!(f, "synthesis"),
        }
    }
}

/// Valid knowledge types for memory entries.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeType {
    Decisional,
    Experiential,
    Factual,
}

impl std::fmt::Display for KnowledgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decisional => write!(f, "decisional"),
            Self::Experiential => write!(f, "experiential"),
            Self::Factual => write!(f, "factual"),
        }
    }
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
    /// Source type for this memory.
    pub source_type: SourceType,
    /// Knowledge type for this memory.
    pub knowledge_type: KnowledgeType,
    /// Comma-separated entity tags.
    pub entities: String,
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// IDs of existing memories this new memory supersedes. When provided, the
    /// insert and all invalidations are wrapped in a single atomic transaction.
    pub resolves: Option<Vec<String>>,
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
    /// One of: conclusion | review | plan | retrospect | synthesis.
    /// Review feedback: callers (ae:analyze Round 0 injection, operators
    /// reading `mengdie search`) need to distinguish primary-source memories
    /// from LLM-synthesized summaries so they can apply appropriate
    /// epistemic weight.
    pub source_type: String,
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
    /// The ID of the memory that supersedes this one, if provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
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
        description = "Search Mengdie memories by query. Returns ranked results with title, snippet (first 200 characters of content, not full text), score, and provenance. Results are ranked by hybrid FTS5 + vector similarity merged via Reciprocal Rank Fusion. Use min_score to filter low-relevance results. Use scope='global' to search across all projects (default: current project only)."
    )]
    async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Json<SearchOutput> {
        if params.query.len() > MAX_QUERY_LEN {
            return Json(SearchOutput {
                results: vec![],
                degraded: Some(format!("query too long (max {MAX_QUERY_LEN} chars)")),
            });
        }
        // F-002 Wave 1 audit timer (plan F-002 Step 3 / Topic 1 Option B):
        // start AFTER input-length validation but BEFORE embedding generation
        // so `took_ms` includes embedding latency. The audit hook below covers
        // both the embedding-success path AND the FTS-only fallback path.
        let audit_start = std::time::Instant::now();
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
                        (
                            results,
                            Some("degraded: embedding unavailable, FTS-only".into()),
                        )
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
                    source_type: r.entry.source_type,
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

        // F-002 Wave 1 audit hook: record what the CALLER saw (post-`min_score`
        // filter), not what the search internally ranked. Recording pre-filter
        // IDs would inflate the supersession signal — facts the caller never
        // received would still count as "returned then superseded" toward the
        // A-MEM trigger, producing false-positive triggers. Plan F-002 Step 3 /
        // strategic-post Doodlestein finding.
        let took_ms = audit_start.elapsed().as_millis() as i64;
        let returned_fact_ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
        self.db.record_search_audit_best_effort(
            &params.query,
            project_id,
            took_ms,
            &returned_fact_ids,
        );

        Json(SearchOutput {
            results: items,
            degraded,
        })
    }

    #[tool(
        name = "memory_ingest",
        description = "Ingest a new memory into Mengdie. Returns entry_id and any detected conflicts (evolution candidates or recent conflicts with existing memories sharing entity tags). Pass resolves=[id, ...] to atomically insert this memory and invalidate the listed memories in one transaction. See server instructions for the full conflict resolution workflow."
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
        // Enum types validated at deserialization — unknown values rejected by serde
        let source_type = params.source_type.to_string();
        let knowledge_type = params.knowledge_type.to_string();

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
        let entities_for_check: Vec<String> = params
            .entities
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
            is_longterm: false,
        };

        let resolves = params.resolves.unwrap_or_default();
        let insert_result = if resolves.is_empty() {
            self.db.insert_memory(mem)
        } else {
            self.db.insert_memory_resolving(mem, &resolves)
        };

        match insert_result {
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
        description = "Mark a memory as no longer valid. Set superseded_by when a newer memory replaces it — links the records for traceability. The reason field is persisted for audit."
    )]
    async fn invalidate(
        &self,
        Parameters(params): Parameters<InvalidateParams>,
    ) -> Json<InvalidateOutput> {
        let superseded_by = params.superseded_by.clone();
        match self.db.invalidate_memory(
            &params.entry_id,
            params.superseded_by.as_deref(),
            Some(&params.reason),
        ) {
            Ok(updated) => Json(InvalidateOutput {
                success: updated,
                entry_id: params.entry_id,
                superseded_by,
            }),
            Err(e) => {
                tracing::error!(error = %e, "memory_invalidate failed");
                Json(InvalidateOutput {
                    success: false,
                    entry_id: params.entry_id,
                    superseded_by: None,
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
            .with_instructions("AI-native Mengdie — knowledge management for AI development workflows. Tools: memory_search, memory_ingest, memory_invalidate. \
Workflow: (1) search for prior context before making decisions, (2) ingest new knowledge after producing durable output. \
Conflict resolution: memory_ingest returns detected conflicts. For 'evolution candidate' conflicts (high similarity, same entity tags), call memory_invalidate with superseded_by=new_entry_id to link old→new. For 'recent conflict', surface to the user before resolving. \
For atomic resolution, pass resolves=[old_id, ...] to memory_ingest to insert and invalidate in one transaction.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_params_rejects_invalid_source_type() {
        let json = serde_json::json!({
            "title": "Test",
            "content": "Test content",
            "source_type": "decision",
            "knowledge_type": "factual",
            "entities": "test"
        });
        let result = serde_json::from_value::<IngestParams>(json);
        assert!(
            result.is_err(),
            "should reject 'decision' (valid is 'decisional' or source types)"
        );
    }

    #[test]
    fn test_ingest_params_accepts_valid_source_type() {
        let json = serde_json::json!({
            "title": "Test",
            "content": "Test content",
            "source_type": "conclusion",
            "knowledge_type": "decisional",
            "entities": "test"
        });
        let result = serde_json::from_value::<IngestParams>(json);
        assert!(
            result.is_ok(),
            "should accept valid source_type 'conclusion'"
        );
        let params = result.unwrap();
        assert_eq!(params.source_type.to_string(), "conclusion");
        assert_eq!(params.knowledge_type.to_string(), "decisional");
    }

    #[test]
    fn test_ingest_params_rejects_invalid_knowledge_type() {
        let json = serde_json::json!({
            "title": "Test",
            "content": "Test content",
            "source_type": "conclusion",
            "knowledge_type": "decision",
            "entities": "test"
        });
        let result = serde_json::from_value::<IngestParams>(json);
        assert!(
            result.is_err(),
            "should reject 'decision' as knowledge_type"
        );
    }

    #[test]
    fn test_source_type_synthesis_display() {
        assert_eq!(SourceType::Synthesis.to_string(), "synthesis");
    }

    #[test]
    fn test_ingest_params_accepts_synthesis_source_type() {
        let json = serde_json::json!({
            "title": "Synthesis memory",
            "content": "Distilled from several sources.",
            "source_type": "synthesis",
            "knowledge_type": "factual",
            "entities": "syn,test"
        });
        let result = serde_json::from_value::<IngestParams>(json);
        assert!(result.is_ok(), "should accept source_type 'synthesis'");
        let params = result.unwrap();
        assert_eq!(params.source_type.to_string(), "synthesis");
    }
}
