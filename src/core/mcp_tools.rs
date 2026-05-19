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
    /// First 8 hex chars of `id` — a citable short form for LLM output.
    /// Use with `memory_invalidate` (and future `memory_get`) prefix
    /// lookup. F-009.
    pub short_id: String,
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
        // F-003 Wave 2 audit timer: pass to orchestrator so took_ms includes
        // embed latency (preserves F-002 Topic 1 Option B invariant).
        let audit_start = std::time::Instant::now();
        let pid = params
            .project_id
            .as_deref()
            .unwrap_or(&self.default_project_id);
        let project_id = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid),
        };

        // Generate query embedding (blocking → thread pool). The orchestrator
        // accepts the Result directly so it can decide fallback without
        // re-running the embedder.
        let query = params.query.clone();
        let embedder = self.embedder.clone();
        let query_embedding_result = tokio::task::spawn_blocking(move || {
            let mut emb = embedder.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            emb.embed_text(&query)
        })
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn: {e}")));

        let limit = params.limit.unwrap_or(10);
        let min_score = params.min_score.unwrap_or(0.0);

        // F-003 Wave 2 orchestrator: routes hybrid → FTS-only fallback per
        // FallbackPolicy::HybridOrFtsOnly (MCP per-surface default per Topic 1);
        // applies min_score filter pre-audit; fires audit hook exactly ONCE
        // post-filter (replaces F-002 Wave 1's two duplicated call-site hooks).
        let outcome = match super::search::memory_search_audited(
            &self.db,
            &params.query,
            query_embedding_result,
            project_id,
            limit,
            min_score,
            audit_start,
            super::search::FallbackPolicy::HybridOrFtsOnly,
        ) {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = %e, "memory_search_audited failed");
                return Json(SearchOutput {
                    results: vec![],
                    degraded: Some("search temporarily unavailable".into()),
                });
            }
        };

        // Map MemorySearchOutcome.route → user-facing degraded string.
        // Preserves F-002 Wave 1 string verbatim for backward compatibility.
        let degraded = match outcome.route {
            super::search::SearchRoute::FtsOnly => {
                Some("degraded: embedding unavailable, FTS-only".into())
            }
            super::search::SearchRoute::Hybrid => None,
        };

        let items: Vec<SearchResultItem> = outcome
            .results
            .into_iter()
            .map(|r| {
                let snippet = r.entry.content.chars().take(200).collect::<String>();
                let short_id = r.entry.id.chars().take(8).collect::<String>();
                SearchResultItem {
                    id: r.entry.id,
                    short_id,
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

        // Track metrics (audit hook already fired inside memory_search_audited;
        // F-002 invariant preserved — record_search_audit_best_effort fires once
        // per call with post-filter IDs).
        let _ = self.db.increment_metric(metrics::METRIC_SEARCH_COUNT);
        if !items.is_empty() {
            let _ = self.db.increment_metric(metrics::METRIC_SEARCH_NONEMPTY);
        }

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

        let pid = params
            .project_id
            .unwrap_or_else(|| self.default_project_id.clone());
        let metadata = super::ingest::IngestMetadata {
            title: params.title,
            entities: params.entities,
            source_file: params.source_file,
            source_type: params.source_type.to_string(),
            knowledge_type: params.knowledge_type.to_string(),
            is_longterm: false,
        };
        let content = params.content;
        let resolves = params.resolves.unwrap_or_default();

        // F-003 Wave 2: route through ingest::ingest_text or
        // ingest_text_with_resolves via spawn_blocking (preserves the
        // Arc<Mutex<Embedder>> lifecycle pattern from F-002 — plan AC10
        // requires ZERO changes to this lock shape; the existing
        // `embedder.clone() + lock + emb.embed_with_context` flow is
        // structurally preserved, only the post-embed code path moves
        // into the shared ingest::* helpers).
        //
        // F-003 Topic 4 implicit semantic: embed-fail is now a hard error
        // (was soft "store without embedding" in pre-F-003 MCP path).
        // Behavior change is bounded — the caller sees `error: "ingestion
        // failed"` instead of a partial-stored memory; converges with
        // file-ingest path's hard-error behavior.
        let db = self.db.clone();
        let embedder = self.embedder.clone();
        let result =
            tokio::task::spawn_blocking(move || -> anyhow::Result<super::ingest::IngestResult> {
                let mut emb = embedder.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
                if resolves.is_empty() {
                    super::ingest::ingest_text(&db, &mut *emb, &content, metadata, &pid)
                } else {
                    super::ingest::ingest_text_with_resolves(
                        &db, &mut *emb, &content, metadata, &pid, &resolves,
                    )
                }
            })
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn: {e}")));

        match result {
            Ok(ingest_result) => {
                let conflicts: Vec<ConflictItem> = ingest_result
                    .conflicts
                    .into_iter()
                    .map(|c| ConflictItem {
                        id: c.existing_id,
                        title: c.existing_title,
                        reason: c.reason.to_string(),
                    })
                    .collect();
                let _ = self.db.increment_metric(metrics::METRIC_INGEST_COUNT);
                if !conflicts.is_empty() {
                    let _ = self.db.increment_metric(metrics::METRIC_CONFLICT_COUNT);
                }
                Json(IngestOutput {
                    entry_id: ingest_result.entry_id,
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
    fn test_search_result_short_id_derivation() {
        // F-009 contract: SearchResultItem::short_id is the first 8
        // chars of `id` — see construction site in McpServer::search.
        // This test pins the derivation rule (chars().take(8)) so any
        // future refactor that changes the prefix length or source
        // field has to update both the struct doc + this assertion.
        let item = SearchResultItem {
            id: "88a93a9b-3c32-47ba-a1b0-d6789abcdef0".to_string(),
            short_id: "88a93a9b".to_string(),
            title: "t".to_string(),
            source_file: "f".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            entities: "e".to_string(),
            score: 0.0,
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            snippet: "s".to_string(),
        };
        assert_eq!(item.short_id, item.id.chars().take(8).collect::<String>());
        assert_eq!(item.short_id.len(), 8);
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
