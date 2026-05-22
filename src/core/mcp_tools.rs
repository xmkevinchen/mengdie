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

/// MCP server with 7 tools: memory_search, memory_ingest, memory_get, memory_invalidate, memory_status, memory_entity_facts, memory_lint.
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
pub struct GetParams {
    /// Full UUID (36 chars) OR ≥ 8-char prefix of a memory_entries.id.
    /// Prefix lookup is scoped to the current project unless `project_id`
    /// or `scope: "global"` overrides it.
    pub memory_id: String,
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// Search scope: omit for current project, "global" for all projects.
    pub scope: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct StatusParams {
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// Scope: omit for current project, "global" for cross-project totals.
    pub scope: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct LintParams {
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// Scope: omit for current project, "global" for cross-project scan.
    pub scope: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct EntityFactsParams {
    /// Entity name to look up. Matches the canonical lowercased form
    /// stored in the `entities` table — ingest.rs:77 lowercases at
    /// ingest time, so callers should pass the lowercased term.
    pub entity_name: String,
    /// Override project_id (default: inferred from cwd at server startup).
    pub project_id: Option<String>,
    /// Scope: omit for current project, "global" for cross-project search.
    pub scope: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct InvalidateParams {
    /// ID of the memory entry to invalidate.
    pub entry_id: String,
    /// Reason for invalidation.
    pub reason: String,
    /// Optional ID of the memory that supersedes this one.
    pub superseded_by: Option<String>,
    /// Override project_id (default: inferred from cwd at server startup).
    #[serde(default)]
    pub project_id: Option<String>,
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

/// Full fact shape returned by `memory_get` — mirrors `MemoryEntry` minus
/// the `embedding` BLOB (~1.5KB f32 per fact; not useful over MCP wire).
/// `embedding_dim` retained for diagnostic purposes (e.g., F-008 Memory
/// Lint Check 3 surfacing).
#[derive(Serialize, schemars::JsonSchema)]
pub struct MemoryEntryView {
    pub id: String,
    /// First 8 hex chars of `id` — citable short form (F-009 contract).
    pub short_id: String,
    pub project_id: String,
    pub source_file: String,
    pub source_type: String,
    pub knowledge_type: String,
    pub title: String,
    /// Full content of the memory, NOT a 200-char snippet. The whole
    /// point of memory_get is to expand a cited fact.
    pub content: String,
    pub entities: String,
    pub valid_from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub recall_count: i64,
    pub avg_relevance: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_recalled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dim: Option<i64>,
    pub is_longterm: bool,
    pub created_at: String,
}

impl From<super::db::MemoryEntry> for MemoryEntryView {
    fn from(e: super::db::MemoryEntry) -> Self {
        let short_id = e.id.chars().take(8).collect();
        Self {
            id: e.id,
            short_id,
            project_id: e.project_id,
            source_file: e.source_file,
            source_type: e.source_type,
            knowledge_type: e.knowledge_type,
            title: e.title,
            content: e.content,
            entities: e.entities,
            valid_from: e.valid_from,
            valid_until: e.valid_until,
            superseded_by: e.superseded_by,
            recall_count: e.recall_count,
            avg_relevance: e.avg_relevance,
            last_recalled: e.last_recalled,
            embedding_dim: e.embedding_dim,
            is_longterm: e.is_longterm,
            created_at: e.created_at,
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct GetOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<MemoryEntryView>,
    /// Non-empty when get failed (unknown id, ambiguous prefix, cross-
    /// project mismatch, prefix too short, DB error).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct InvalidateOutput {
    pub success: bool,
    pub entry_id: String,
    /// The ID of the memory that supersedes this one, if provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    /// Non-empty when invalidation could not be performed: unknown ID,
    /// ambiguous prefix (collision), or DB error. Caller should inspect
    /// this before treating `success: false` as a generic failure.
    /// F-009.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Audit-pipeline view nested into `StatusOutput` (F-011). Mirrors
/// `db::AuditStats` field shape with RFC3339 strings for timestamps.
#[derive(Serialize, schemars::JsonSchema, Default)]
pub struct AuditView {
    pub audit_count: i64,
    pub link_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_row: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_row: Option<String>,
    pub supersession_count_30d: i64,
    pub audit_write_failures: i64,
}

/// DB-health response for `memory_status` (F-011). Scoped to the
/// requested project (or "global" via scope param).
#[derive(Serialize, schemars::JsonSchema, Default)]
pub struct StatusOutput {
    /// Project context for this snapshot. `"<global>"` when caller passed
    /// `scope: "global"`; otherwise the resolved project_id.
    pub project_id: String,
    pub total_entries: i64,
    pub longterm_count: i64,
    pub synthesis_count: i64,
    /// Per-source_type entry counts (conclusion / review / plan / analysis
    /// / retrospect / synthesis). Missing keys = 0.
    pub by_source_type: std::collections::BTreeMap<String, i64>,
    /// `MAX(created_at)` across scope; `None` when DB is empty in scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ingest_at: Option<String>,
    /// Persistent counters from the metrics table — search_count /
    /// search_nonempty_count / ingest_count / conflict_count /
    /// audit_write_failures and any future additions. Always global
    /// (not project-scoped) since metrics counters are process-wide.
    pub metrics: std::collections::BTreeMap<String, i64>,
    /// Audit-pipeline nested snapshot (always global; audit table itself
    /// is not project-scoped at the row level today).
    pub audit: AuditView,
    /// Embedding model name used by this mengdie instance. F-011 review
    /// fixup (codex P1 + challenger P1): operators / LLM callers
    /// debugging "search feels off" need to know which model the corpus
    /// was embedded with. Hardcoded to "all-MiniLM-L6-v2" (v0.0.1 ship
    /// model); future model swaps will source from `MengdieConfig`.
    pub embedding_model: String,
    /// Canonical embedding dimension this mengdie expects. Mirrors
    /// schema.rs VEC_DIM (384 for all-MiniLM-L6-v2). Surfaces config
    /// drift to callers without forcing a separate dim-discovery
    /// round trip.
    pub embedding_dim: i64,
    /// Non-empty when status could not be assembled (DB error).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// F-007 entity_facts response: facts tagged with a given entity name.
/// Item shape mirrors SearchResultItem so callers can re-use existing
/// rendering / parsing paths.
#[derive(Serialize, schemars::JsonSchema)]
pub struct EntityFactsOutput {
    pub entity_name: String,
    pub facts: Vec<SearchResultItem>,
    /// Non-empty on DB error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
    pub async fn search(&self, Parameters(params): Parameters<SearchParams>) -> Json<SearchOutput> {
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
    pub async fn ingest(&self, Parameters(params): Parameters<IngestParams>) -> Json<IngestOutput> {
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
        name = "memory_get",
        description = "Fetch the full content of a single memory by ID. Returns the complete fact (not a 200-char snippet) plus provenance, validity, supersession, and recall stats. Side effect: increments recall_count + last_recalled (avg_relevance is NOT touched — direct lookup has no meaningful relevance score). Accepts either a full UUID (36 chars) or an 8+ char prefix; prefix is scoped to the current project unless scope='global'. Use after memory_search to expand a cited fact."
    )]
    pub async fn get(&self, Parameters(params): Parameters<GetParams>) -> Json<GetOutput> {
        // F-010: project scope resolution mirrors memory_search.
        let pid_owned = params
            .project_id
            .clone()
            .unwrap_or_else(|| self.default_project_id.clone());
        let scope_for_lookup: Option<&str> = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid_owned.as_str()),
        };

        // Resolve memory_id: full 36-char UUID → fast path; ≥ 8-char prefix
        // → scoped lookup; < 8-char → reject (F-009 boundary).
        let resolved_id = if params.memory_id.len() == 36 {
            params.memory_id.clone()
        } else if params.memory_id.len() < 8 {
            return Json(GetOutput {
                entry: None,
                error: Some(format!(
                    "Prefix '{}' is too short (need ≥ 8 chars, or pass a full 36-char UUID)",
                    params.memory_id
                )),
            });
        } else {
            match self
                .db
                .find_by_id_prefix(&params.memory_id, scope_for_lookup)
            {
                Ok(matches) if matches.len() == 1 => matches.into_iter().next().unwrap(),
                Ok(matches) if matches.is_empty() => {
                    return Json(GetOutput {
                        entry: None,
                        error: Some(match scope_for_lookup {
                            // Fixup (F-010 review): mention scope=global as a
                            // remediation when the scoped prefix lookup misses —
                            // matches the full-UUID cross-project guard's hint
                            // pattern so LLM callers see consistent escape hatches
                            // regardless of which path produced the miss.
                            Some(p) => format!(
                                "No memory matches prefix '{}' in project '{}'; pass scope='global' to search across projects",
                                params.memory_id, p
                            ),
                            None => format!(
                                "No memory matches prefix '{}' (global scope)",
                                params.memory_id
                            ),
                        }),
                    });
                }
                Ok(matches) => {
                    return Json(GetOutput {
                        entry: None,
                        error: Some(format!(
                            "Prefix '{}' is ambiguous; matches at least: {}; extend prefix to disambiguate",
                            params.memory_id,
                            matches.join(", ")
                        )),
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "find_by_id_prefix failed in memory_get");
                    return Json(GetOutput {
                        entry: None,
                        error: Some(format!("DB error during prefix lookup: {e}")),
                    });
                }
            }
        };

        // Fetch the fact.
        let entry = match self.db.get_memory(&resolved_id) {
            Ok(Some(e)) => e,
            Ok(None) => {
                return Json(GetOutput {
                    entry: None,
                    error: Some(format!("No memory found with id '{resolved_id}'")),
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "get_memory failed");
                return Json(GetOutput {
                    entry: None,
                    error: Some(format!("DB error: {e}")),
                });
            }
        };

        // Cross-project guard (unless caller asked for global scope).
        if let Some(p) = scope_for_lookup {
            if entry.project_id != p {
                return Json(GetOutput {
                    entry: None,
                    error: Some(format!(
                        "Memory '{}' belongs to project '{}', not '{}'; pass scope='global' or set project_id explicitly",
                        resolved_id, entry.project_id, p
                    )),
                });
            }
        }

        // Bump recall (count-only — no EMA contribution; direct lookup
        // has no meaningful relevance score).
        //
        // Fixup (F-010 review): warn on err instead of fully silent swallow.
        // record_recall failures in memory_search log via tracing; parity
        // here makes ops debugging consistent. We still return success —
        // recall_count is a soft signal; corruption of one bump is not
        // worth failing the whole get.
        if let Err(e) = self.db.bump_recall_only(&resolved_id) {
            tracing::warn!(error = %e, id = %resolved_id, "bump_recall_only failed in memory_get; returning entry anyway");
        }

        Json(GetOutput {
            entry: Some(entry.into()),
            error: None,
        })
    }

    #[tool(
        name = "memory_status",
        description = "Snapshot of mengdie DB health: entry counts (total / longterm / synthesis / per-source-type), last ingest timestamp, operational counters (search/ingest/conflict/audit-failure), and audit-pipeline stats. Read-only. Scoped to current project by default; pass scope='global' for cross-project totals. Use this to distinguish 'DB is empty' from 'query missed' when memory_search returns nothing, or to verify F-002 audit pipeline is healthy."
    )]
    pub async fn status(&self, Parameters(params): Parameters<StatusParams>) -> Json<StatusOutput> {
        // F-011: project scope resolution mirrors memory_search / memory_get.
        let pid_owned = params
            .project_id
            .clone()
            .unwrap_or_else(|| self.default_project_id.clone());
        let scope_for_lookup: Option<&str> = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid_owned.as_str()),
        };
        let project_id_label: String = match scope_for_lookup {
            Some(p) => p.to_string(),
            None => "<global>".to_string(),
        };

        // Breakdown — scoped to project (or global)
        let breakdown = match self.db.status_breakdown(scope_for_lookup) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(error = %e, "status_breakdown failed");
                return Json(StatusOutput {
                    project_id: project_id_label,
                    error: Some(format!("DB error during status breakdown: {e}")),
                    ..Default::default()
                });
            }
        };

        // Metrics — always global; counters are process-wide, not project-scoped
        let metrics: std::collections::BTreeMap<String, i64> = self
            .db
            .list_metrics()
            .unwrap_or_default()
            .into_iter()
            .collect();

        // Audit nested — always global at row level (audit table not scoped)
        let audit_view: AuditView = match self.db.audit_stats() {
            Ok(s) => AuditView {
                audit_count: s.audit_count,
                link_count: s.link_count,
                oldest_row: s.oldest_row,
                newest_row: s.newest_row,
                supersession_count_30d: s.supersession_count_30d,
                audit_write_failures: s.audit_write_failures,
            },
            Err(e) => {
                tracing::warn!(error = %e, "audit_stats failed in memory_status; returning empty audit view");
                AuditView::default()
            }
        };

        Json(StatusOutput {
            project_id: project_id_label,
            total_entries: breakdown.total,
            longterm_count: breakdown.longterm_count,
            synthesis_count: breakdown.synthesis_count,
            by_source_type: breakdown.by_source_type,
            last_ingest_at: breakdown.last_ingest_at,
            metrics,
            audit: audit_view,
            // F-011 review fixup: hardcoded for v0.0.1's single
            // embedding model (all-MiniLM-L6-v2 384d via fastembed-rs
            // — see embeddings.rs:55). Future model swaps will source
            // from MengdieConfig + propagate through StatusOutput.
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            embedding_dim: 384,
            error: None,
        })
    }

    #[tool(
        name = "memory_lint",
        description = "Run F-008 Memory Lint deterministic health checks: orphan GC (dangling references in superseded_by / synthesis_links / audit_returned_facts), unresolved contradictions (half-resolved supersession state + size-2 cycles + high-entity-overlap unsuperseded pairs), embedding drift (NULL on non-synthesis + dim mismatch + BL-022 surface). Read-only, idempotent (same DB → same findings except generated_at). Scoped to current project unless scope='global'."
    )]
    pub async fn lint(
        &self,
        Parameters(params): Parameters<LintParams>,
    ) -> Json<super::lint::LintReport> {
        let pid_owned = params
            .project_id
            .clone()
            .unwrap_or_else(|| self.default_project_id.clone());
        let scope_for_lookup: Option<&str> = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid_owned.as_str()),
        };
        match self.db.run_lint(scope_for_lookup) {
            Ok(r) => Json(r),
            Err(e) => {
                tracing::error!(error = %e, "run_lint failed");
                Json(super::lint::LintReport {
                    generated_at: chrono::Utc::now().to_rfc3339(),
                    orphan_gc: Default::default(),
                    unresolved_contradictions: Default::default(),
                    embedding_drift: Default::default(),
                    // F-008 review fixup (challenger P6): populate
                    // error field so callers can distinguish "all
                    // clean" from "lint pass aborted partway".
                    error: Some(format!("run_lint failed: {e}")),
                })
            }
        }
    }

    #[tool(
        name = "memory_entity_facts",
        description = "List all VALID (non-invalidated, non-superseded) facts tagged with a given entity name. Returns SearchResultItem shape, BUT score field is `recall_count` (i64 cast to f64), NOT a 0.0-1.0 similarity score — entity-tag lookup has no relevance signal. Results ordered by recall_count desc (most-consulted facts surface first). Use this for 'show me everything related to <X>' queries when memory_search's semantic match isn't specific enough. Scoped to current project unless scope='global'."
    )]
    pub async fn entity_facts(
        &self,
        Parameters(params): Parameters<EntityFactsParams>,
    ) -> Json<EntityFactsOutput> {
        // F-007: project scope resolution mirrors memory_search.
        let pid_owned = params
            .project_id
            .clone()
            .unwrap_or_else(|| self.default_project_id.clone());
        let scope_for_lookup: Option<&str> = match params.scope.as_deref() {
            Some("global") => None,
            _ => Some(pid_owned.as_str()),
        };

        let fact_ids = match self
            .db
            .facts_with_entity(&params.entity_name, scope_for_lookup)
        {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!(error = %e, entity = %params.entity_name, "facts_with_entity failed");
                return Json(EntityFactsOutput {
                    entity_name: params.entity_name,
                    facts: vec![],
                    error: Some(format!("DB error: {e}")),
                });
            }
        };

        // Hydrate each fact + map to SearchResultItem shape.
        // F-007 review fixup (challenger #3): filter out invalidated /
        // superseded facts (`valid_until IS NOT NULL`). Pre-fixup the
        // tool silently returned stale facts, which is misleading for
        // "show me current knowledge about X" queries. Callers needing
        // the historical view can grep memory_entries directly.
        let mut items: Vec<SearchResultItem> = Vec::with_capacity(fact_ids.len());
        for fid in &fact_ids {
            match self.db.get_memory(fid) {
                Ok(Some(e)) => {
                    if e.valid_until.is_some() {
                        // Skip invalidated/superseded entries — they're
                        // historical, not current state.
                        continue;
                    }
                    let snippet = e.content.chars().take(200).collect::<String>();
                    let short_id = e.id.chars().take(8).collect::<String>();
                    items.push(SearchResultItem {
                        id: e.id,
                        short_id,
                        title: e.title,
                        source_file: e.source_file,
                        source_type: e.source_type,
                        knowledge_type: e.knowledge_type,
                        entities: e.entities,
                        // No relevance score in entity-tag lookup; use
                        // recall_count as a proxy ordering signal —
                        // facts more frequently consulted bubble up.
                        score: e.recall_count as f64,
                        valid_from: e.valid_from,
                        snippet,
                    });
                }
                Ok(None) => {
                    // fact_entity row references a deleted memory_entries.id —
                    // orphan; will surface in F-008 Memory Lint Check 1.
                    tracing::warn!(fact_id = %fid, "fact_entity row references missing memory_entries; orphan");
                }
                Err(e) => {
                    tracing::warn!(error = %e, fact_id = %fid, "get_memory failed for fact_entity hit; skipping");
                }
            }
        }
        // Order by recall_count desc (score field), so most-consulted facts surface first.
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Json(EntityFactsOutput {
            entity_name: params.entity_name,
            facts: items,
            error: None,
        })
    }

    #[tool(
        name = "memory_invalidate",
        description = "Mark a memory as no longer valid. Set superseded_by when a newer memory replaces it — links the records for traceability. The reason field is persisted for audit. Accepts either a full UUID (36 chars) or an 8+ char prefix; collision returns an error listing matches."
    )]
    pub async fn invalidate(
        &self,
        Parameters(params): Parameters<InvalidateParams>,
    ) -> Json<InvalidateOutput> {
        let superseded_by = params.superseded_by.clone();

        // F-009 step 3: resolve UUID prefix when caller passes < 36 chars.
        // Full-UUID input (36 chars) takes the fast path with no prefix
        // lookup; pre-F-009 callers are unaffected.
        //
        // Fixup (review): enforce 8-char minimum so a 1-char prefix doesn't
        // silently match thousands of UUIDs and surface as a misleading
        // "collision" error. Boundary check belongs here (MCP layer); db
        // layer remains permissive for future callers with different rules.
        let resolved_id = if params.entry_id.len() == 36 {
            params.entry_id.clone()
        } else if params.entry_id.len() < 8 {
            return Json(InvalidateOutput {
                success: false,
                entry_id: params.entry_id.clone(),
                superseded_by: None,
                error: Some(format!(
                    "Prefix '{}' is too short (need ≥ 8 chars, or pass a full 36-char UUID)",
                    params.entry_id
                )),
            });
        } else {
            // F-015: caller-authority precedence — params.project_id
            // (non-empty) wins over the server's startup-cached default for
            // prefix-scope. The filter() normalizes Some("") → fallback
            // (stale-template safety). Divergence from the 6 other tools'
            // Some("")-passes-through behavior is an intentional partial
            // fix scoped by d001 conclusion; the other 6 retain pre-existing
            // behavior, filed as follow-up BL. Full-UUID branch above is
            // unscoped by design — UUIDs are globally unique, so
            // project-scoping is semantically unnecessary there.
            let scope = params
                .project_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&self.default_project_id);
            match self.db.find_by_id_prefix(&params.entry_id, Some(scope)) {
                Ok(matches) if matches.len() == 1 => matches.into_iter().next().unwrap(),
                Ok(matches) if matches.is_empty() => {
                    return Json(InvalidateOutput {
                        success: false,
                        entry_id: params.entry_id.clone(),
                        superseded_by: None,
                        error: Some(format!(
                            "No memory matches prefix '{}' in project '{}'",
                            params.entry_id, scope
                        )),
                    });
                }
                Ok(matches) => {
                    // Fixup (review): drop misleading "(N+)" count from
                    // collision wording. find_by_id_prefix caps at LIMIT 2,
                    // so matches.len() is always 2 here — the displayed list
                    // IS the at-least signal; no separate count needed.
                    return Json(InvalidateOutput {
                        success: false,
                        entry_id: params.entry_id.clone(),
                        superseded_by: None,
                        error: Some(format!(
                            "Prefix '{}' is ambiguous; matches at least: {}; extend prefix to disambiguate",
                            params.entry_id,
                            matches.join(", ")
                        )),
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "find_by_id_prefix failed in memory_invalidate");
                    return Json(InvalidateOutput {
                        success: false,
                        entry_id: params.entry_id.clone(),
                        superseded_by: None,
                        error: Some(format!("DB error during prefix lookup: {e}")),
                    });
                }
            }
        };

        match self.db.invalidate_memory(
            &resolved_id,
            params.superseded_by.as_deref(),
            Some(&params.reason),
        ) {
            Ok(true) => Json(InvalidateOutput {
                success: true,
                entry_id: resolved_id,
                superseded_by,
                error: None,
            }),
            Ok(false) => Json(InvalidateOutput {
                success: false,
                entry_id: resolved_id.clone(),
                superseded_by: None,
                error: Some(format!("No memory found with id '{resolved_id}'")),
            }),
            Err(e) => {
                tracing::error!(error = %e, "memory_invalidate failed");
                Json(InvalidateOutput {
                    success: false,
                    entry_id: resolved_id,
                    superseded_by: None,
                    error: Some(format!("DB error: {e}")),
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
            .with_instructions("AI-native Mengdie — knowledge management for AI development workflows. Tools: memory_search, memory_ingest, memory_get, memory_invalidate, memory_status, memory_entity_facts. \
Workflow: (1) search for prior context before making decisions, (2) ingest new knowledge after producing durable output, (3) get full content of a cited fact via memory_get when search snippet is insufficient, (4) memory_status to check DB health, (5) memory_entity_facts to list all facts tagged with a specific entity name (e.g., 'sqlite-vec'). \
Conflict resolution: memory_ingest returns detected conflicts. For 'evolution candidate' conflicts (high similarity, same entity tags), call memory_invalidate with superseded_by=new_entry_id to link old→new. For 'recent conflict', surface to the user before resolving. \
For atomic resolution, pass resolves=[old_id, ...] to memory_ingest to insert and invalidate in one transaction. \
ID shape: memory_search returns short_id (8-char UUID prefix) alongside full id; memory_get and memory_invalidate accept either form (≥8 chars).")
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
    fn test_memory_entry_view_from_preserves_fields() {
        // F-010 contract: From<MemoryEntry> for MemoryEntryView preserves
        // every non-embedding field. Embedding BLOB is dropped (wire cost);
        // short_id is derived from id (F-009 contract).
        use crate::core::db::MemoryEntry;
        let entry = MemoryEntry {
            id: "88a93a9b-3c32-47ba-a1b0-d6789abcdef0".to_string(),
            project_id: "p".to_string(),
            source_file: "f.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "T".to_string(),
            content: "C".repeat(500), // > snippet boundary
            entities: "a,b".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: Some("2026-02-01T00:00:00Z".to_string()),
            superseded_by: Some("other-id".to_string()),
            recall_count: 7,
            avg_relevance: 0.42,
            last_recalled: Some("2026-01-15T00:00:00Z".to_string()),
            embedding: Some(vec![0u8; 1536]), // dropped by view
            embedding_dim: Some(384),
            is_longterm: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let view: MemoryEntryView = entry.into();
        assert_eq!(view.id, "88a93a9b-3c32-47ba-a1b0-d6789abcdef0");
        assert_eq!(view.short_id, "88a93a9b");
        assert_eq!(view.content.len(), 500); // full content, NOT snippet
        assert_eq!(view.recall_count, 7);
        assert_eq!(view.avg_relevance, 0.42);
        assert!(view.is_longterm);
        assert_eq!(view.embedding_dim, Some(384));
        assert_eq!(view.valid_until.as_deref(), Some("2026-02-01T00:00:00Z"));
        assert_eq!(view.superseded_by.as_deref(), Some("other-id"));
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
