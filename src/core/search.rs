use std::collections::HashMap;

use anyhow::Context;

use super::db::{Db, MemoryEntry};
use super::decay;
use super::vector::VectorResult;

/// Score multiplier for long-term memories (promoted by Dreaming).
///
/// Note — LONGTERM_BOOST cliff: when the Dreaming pass clears `is_longterm`
/// on a stale memory (effective_relevance < DEMOTION_FLOOR, see
/// `core::dreaming::run_dreaming_with_config`), the next search of that
/// memory drops its boost from this 1.2× multiplier to 1.0×. Combined with
/// the decay multiplier applied below, a demoted memory's score collapses
/// from `normalized × 1.2 × decay` to `normalized × decay` on the next
/// query — a one-time discontinuity. This is the mechanism, not a bug.
/// See docs/discussions/019-power-law-decay/conclusion.md Topic 3.
const LONGTERM_BOOST: f64 = 1.2;

/// A search result with merged score and full memory data.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: MemoryEntry,
    pub score: f64, // RRF-merged, normalized 0-1
}

/// FTS5 search result (id + BM25 score).
#[derive(Debug)]
pub struct FtsResult {
    pub id: String,
    pub bm25_score: f64,
}

// ---- F-003 Wave 2 orchestrator types (plan F-003 Step 1; discussion 001 Topic 1) ----

/// Controls embedding-failure behavior at the `memory_search_audited`
/// orchestrator. Per-surface defaults per discussion 001 Topic 1:
/// - **MCP** (`mcp_tools::search`) → `HybridOrFtsOnly` (graceful fallback to
///   FTS-only on embed-fail; matches today's MCP behavior).
/// - **CLI** (`cli::cmd_search`) → `HybridOrError` (hard-error on embed-fail;
///   matches today's CLI behavior — operator surface).
/// - **Internal/test callers** → `HybridOrError` (deterministic error path).
///
/// Plan F-003 Step 1 / discussion 001 Topic 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Embedding failure → propagate as `Err` (no FTS fallback).
    HybridOrError,
    /// Embedding failure → fall back to FTS-only path with normalized scores.
    HybridOrFtsOnly,
}

/// Indicates which retrieval path produced the results in
/// `MemorySearchOutcome`. Consumers map this to their own degraded-mode
/// representation (e.g., MCP's `degraded` string).
///
/// Plan F-003 Step 1 / discussion 001 Topic 1 + Topic 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRoute {
    /// Hybrid FTS5 + vector + RRF merge (the canonical path).
    Hybrid,
    /// FTS-only fallback (embed-fail under `FallbackPolicy::HybridOrFtsOnly`).
    /// Scores are normalized to [0, 1] per Topic 6.
    FtsOnly,
}

/// Populated when `SearchRoute::FtsOnly` was reached via fallback rather than
/// explicit caller request. v0.0.1 has only one fallback reason; the enum
/// shape leaves room for future reasons (e.g., `IndexCorrupted`,
/// `VectorDimensionMismatch`) without breaking callers that match it
/// exhaustively.
///
/// Plan F-003 Step 1 / discussion 001 Topic 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// Embedding generation failed (model unavailable, ONNX runtime error,
    /// etc.). The orchestrator fell back to FTS-only under
    /// `FallbackPolicy::HybridOrFtsOnly`.
    EmbeddingUnavailable,
}

/// Return value of `memory_search_audited` (plan F-003 Step 2). Carries the
/// post-filter result list PLUS the route metadata so consumers can
/// distinguish "0 results because hybrid found nothing" from "0 results
/// because FTS fallback returned empty under embed-fail".
///
/// Plan F-003 Step 1 / discussion 001 Topic 1.
#[derive(Debug)]
pub struct MemorySearchOutcome {
    pub results: Vec<SearchResult>,
    pub route: SearchRoute,
    pub fallback_reason: Option<FallbackReason>,
}

/// FTS5 reserved words that must be filtered from query tokens.
const FTS5_RESERVED: &[&str] = &["AND", "OR", "NOT", "NEAR"];

/// Compute the post-fetch search score: apply the long-term boost (if
/// applicable) and the time-decay multiplier, clamping at 1.0.
///
/// Extracted to a pure helper so the boost-and-decay ordering is unit-
/// testable without spinning up the embedding infra. Never-recalled
/// memories (`last_recalled IS NULL`) receive no decay penalty —
/// symmetric with the Dreaming pass's NULL-recall skip. Both compute
/// sites derive the age clock from `MemoryEntry::last_recalled_as_datetime()`
/// (same-age-clock invariant from discussion 019).
fn apply_boost_and_decay(
    normalized_rrf: f64,
    entry: &MemoryEntry,
    now: chrono::DateTime<chrono::Utc>,
) -> f64 {
    let decay_mult = entry
        .last_recalled_as_datetime()
        .map(|last| {
            let days = (now - last).num_seconds() as f64 / 86_400.0;
            decay::decay_factor(days)
        })
        .unwrap_or(1.0);
    if entry.is_longterm {
        (normalized_rrf * LONGTERM_BOOST * decay_mult).min(1.0)
    } else {
        normalized_rrf * decay_mult
    }
}

/// Sanitize a query string for safe use in FTS5 MATCH.
/// Splits on non-alphanumeric boundaries (aligning with FTS5's unicode61 tokenizer),
/// filters empty tokens and FTS5 reserved words, joins with AND.
/// Returns an empty string if no valid tokens remain.
pub fn sanitize_fts_query(query: &str) -> String {
    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_string())
        .filter(|token| !token.is_empty())
        .filter(|token| !FTS5_RESERVED.iter().any(|r| r.eq_ignore_ascii_case(token)))
        .collect();
    tokens.join(" AND ")
}

impl Db {
    /// FTS5 full-text search. Returns results ranked by BM25.
    /// Filters out expired entries.
    ///
    /// `pub(crate)` post-F-003 (plan F-003 Step 3 / discussion 001 Topic 3
    /// hybrid): callers go through `search::memory_search_audited` orchestrator
    /// which owns audit-hook integration + min_score filtering + FTS-only
    /// fallback normalization. Direct callers of `search_fts` would bypass
    /// the orchestrator and silently break the F-002 audit invariant.
    pub(crate) fn search_fts(
        &self,
        query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<FtsResult>> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        // Sanitize query: allowlist alphanumeric chars, filter reserved words, join with AND.
        let safe_query = sanitize_fts_query(query);
        if safe_query.is_empty() {
            return Ok(vec![]);
        }

        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
            Some(pid) => (
                "SELECT me.id, bm25(memory_fts) as score \
                 FROM memory_fts \
                 JOIN memory_entries me ON me.rowid = memory_fts.rowid \
                 WHERE memory_fts MATCH ?1 \
                 AND (me.valid_until IS NULL OR me.valid_until > ?2) \
                 AND me.project_id = ?3 \
                 ORDER BY score \
                 LIMIT ?4"
                    .to_string(),
                vec![
                    Box::new(safe_query.clone()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(now),
                    Box::new(pid.to_string()),
                    Box::new(limit as i64),
                ],
            ),
            None => (
                "SELECT me.id, bm25(memory_fts) as score \
                 FROM memory_fts \
                 JOIN memory_entries me ON me.rowid = memory_fts.rowid \
                 WHERE memory_fts MATCH ?1 \
                 AND (me.valid_until IS NULL OR me.valid_until > ?2) \
                 ORDER BY score \
                 LIMIT ?3"
                    .to_string(),
                vec![
                    Box::new(safe_query.clone()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(now),
                    Box::new(limit as i64),
                ],
            ),
        };

        let mut stmt = conn.prepare(&sql).context("prepare FTS5 search")?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(FtsResult {
                id: row.get(0)?,
                bm25_score: row.get(1)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Hybrid search: FTS5 + vector similarity merged via RRF.
    /// Updates recall stats on each hit.
    pub fn memory_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        project_id: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let fts_limit = limit * 3; // Over-fetch for RRF merge
        let vec_limit = limit * 3;

        // Run both searches
        let fts_results = self.search_fts(query, project_id, fts_limit)?;
        let vec_results = self.search_vector(query_embedding, project_id, vec_limit)?;

        // RRF merge
        let merged = rrf_merge(&fts_results, &vec_results, 60.0);

        // Take top `limit` results
        let top_ids: Vec<(String, f64)> = merged.into_iter().take(limit).collect();

        // Fetch full entries and update recall stats
        // RRF scores are raw (~0.01-0.03). Normalize to 0-1 for Dreaming's avg_relevance.
        // Max theoretical RRF: 2 rankers at rank 1 = 2/(k+1) = 2/61 ≈ 0.0328
        const RRF_MAX: f64 = 2.0 / 61.0;
        // `now` is captured once per search call and reused across every
        // result in this response. Same-age-clock invariant with the
        // Dreaming pass (discussion 019, challenger Q4): both call sites
        // drive decay off `entry.last_recalled` via the shared helper
        // `MemoryEntry::last_recalled_as_datetime()`.
        let now = chrono::Utc::now();
        let mut results = Vec::new();
        for (id, score) in &top_ids {
            if let Some(entry) = self.get_memory(id)? {
                let normalized = (*score / RRF_MAX).clamp(0.0, 1.0);
                let boosted = apply_boost_and_decay(normalized, &entry, now);
                // Record recall with original score, not boosted — avoid circular amplification
                if let Err(e) = self.record_recall(id, normalized) {
                    tracing::warn!(id = %id, error = %e, "failed to record recall");
                }
                results.push(SearchResult {
                    entry,
                    score: boosted,
                });
            }
        }

        // Re-sort after boost may have changed ordering
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }
}

/// Reciprocal Rank Fusion: merges two ranked lists by rank position.
/// score(d) = Σ 1/(k + rank_i(d)) for each ranker.
/// Returns (id, rrf_score) sorted descending.
fn rrf_merge(
    fts_results: &[FtsResult],
    vec_results: &[VectorResult],
    k: f64,
) -> Vec<(String, f64)> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (rank, result) in fts_results.iter().enumerate() {
        *scores.entry(result.id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }

    for (rank, result) in vec_results.iter().enumerate() {
        *scores.entry(result.id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }

    let mut merged: Vec<(String, f64)> = scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Raw RRF scores (not normalized). These are small (~0.01-0.03) but comparable
    // across queries — normalizing by max would make top result always 1.0, corrupting
    // avg_relevance tracking in Dreaming.
    merged
}

// ---- F-003 Wave 2 orchestrator (plan F-003 Step 2) ----

/// Linear-rescale-by-per-call-max normalization for FTS-only score
/// fallback. Per discussion 001 Topic 6 + plan F-003 Step 2 HARD GATE
/// benchmark: this function preserves upper-range discriminability
/// (output(50)-output(5) ≈ 0.902 ≥ 0.10) and monotonicity, while sigmoid
/// (compresses upper to ≈0.007) and tanh-half-positive (≈0.0001) both
/// fail — the chosen function ensures `min_score` filters remain
/// effective in degraded mode.
///
/// Input: `&[FtsResult]` with raw `bm25_score` (SQLite FTS5 returns BM25
/// with sign convention where lower = better; we work on `.abs()`).
/// Output: parallel `Vec<f64>` where each element is in [0, 1].
///
/// Edge cases: empty input returns empty Vec; single element returns
/// `[0.0]` (single-result corner case — a min_score check still passes
/// at 0.0 since the result was the only match); range collapse
/// (max-min < 1.0) uses `range = 1.0` to avoid division-blowup, which
/// produces small differential outputs in [0, 1] without panic.
fn linear_rescale_normalize(fts_results: &[FtsResult]) -> Vec<f64> {
    if fts_results.is_empty() {
        return Vec::new();
    }
    let abs_scores: Vec<f64> = fts_results.iter().map(|r| r.bm25_score.abs()).collect();
    let min = abs_scores.iter().copied().fold(f64::INFINITY, f64::min);
    let max = abs_scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1.0);
    abs_scores
        .iter()
        .map(|&s| ((s - min) / range).clamp(0.0, 1.0))
        .collect()
}

/// FTS-only fallback path for `memory_search_audited`. Calls
/// `db.search_fts`, hydrates each FtsResult into a SearchResult by
/// fetching the full MemoryEntry, and normalizes scores to [0, 1] via
/// `linear_rescale_normalize` (plan F-003 Topic 6 / discussion 001 HARD
/// GATE).
fn fts_only_with_normalization(
    db: &Db,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<SearchResult>> {
    let fts_results = db.search_fts(query, project_id, limit)?;
    let normalized_scores = linear_rescale_normalize(&fts_results);
    let mut results = Vec::with_capacity(fts_results.len());
    for (idx, fts) in fts_results.iter().enumerate() {
        if let Some(entry) = db.get_memory(&fts.id)? {
            results.push(SearchResult {
                entry,
                score: normalized_scores[idx],
            });
        }
    }
    Ok(results)
}

/// Free-function orchestrator over `&Db` that:
///
/// 1. Routes search via embedding when `query_embedding_result` is
///    `Ok(...)`, falling back to FTS-only under
///    `FallbackPolicy::HybridOrFtsOnly`. `Err(...)` under
///    `FallbackPolicy::HybridOrError` propagates unchanged.
/// 2. Applies `min_score` filter to the raw results BEFORE the audit
///    hook fires (F-002 Doodlestein-strategic invariant: "record what
///    the caller saw").
/// 3. Fires the F-002 audit hook via
///    `Db::record_search_audit_best_effort` exactly ONCE per call
///    (replaces the two duplicated call-site hooks from F-002 Wave 1).
/// 4. Returns route + fallback metadata so callers can map to
///    surface-specific degraded representations.
///
/// `audit_start: Instant` is passed by the CALLER (preserves F-002
/// Topic 1 Option B "took_ms includes embed latency" invariant — if
/// the orchestrator owned the clock, embed time would be excluded).
///
/// `query_embedding_result` accepts the caller's `Result<Vec<f32>>`
/// directly so the orchestrator can decide fallback based on the
/// embedding outcome WITHOUT re-running the embedder.
///
/// Plan F-003 Step 2 / discussion 001 Topic 1.
//
// 8 parameters — each is a distinct caller concern (query, embedding result,
// scope, limit, min_score, audit timer, fallback policy + the &Db handle).
// Wrapping into a builder struct adds call-site indirection without reducing
// cognitive surface; clippy's 7-arg ceiling is calibrated for typical
// methods, not orchestration boundaries with many independent inputs.
#[allow(clippy::too_many_arguments)]
pub fn memory_search_audited(
    db: &Db,
    query: &str,
    query_embedding_result: anyhow::Result<Vec<f32>>,
    project_id: Option<&str>,
    limit: usize,
    min_score: f64,
    audit_start: std::time::Instant,
    fallback_policy: FallbackPolicy,
) -> anyhow::Result<MemorySearchOutcome> {
    let (raw_results, route, fallback_reason) = match query_embedding_result {
        Ok(embedding) => (
            db.memory_search(query, &embedding, project_id, limit)?,
            SearchRoute::Hybrid,
            None,
        ),
        Err(e) => match fallback_policy {
            FallbackPolicy::HybridOrError => {
                tracing::warn!(error = %e, "embedding failed; HybridOrError policy returns Err");
                return Err(e);
            }
            FallbackPolicy::HybridOrFtsOnly => {
                tracing::warn!(error = %e, "embedding failed; falling back to FTS-only");
                (
                    fts_only_with_normalization(db, query, project_id, limit)?,
                    SearchRoute::FtsOnly,
                    Some(FallbackReason::EmbeddingUnavailable),
                )
            }
        },
    };

    // Apply min_score filter post-search, pre-audit. F-002 Doodlestein-strategic
    // invariant: audit records what the caller saw, not the pre-filter raw set.
    let filtered: Vec<SearchResult> = raw_results
        .into_iter()
        .filter(|r| r.score >= min_score)
        .collect();

    // F-002 audit hook: caller passes audit_start so took_ms includes embed
    // latency (Topic 1 Option B). returned_fact_ids extracted from post-filter
    // results (Doodlestein-strategic finding).
    let took_ms = audit_start.elapsed().as_millis() as i64;
    let returned_fact_ids: Vec<String> = filtered.iter().map(|r| r.entry.id.clone()).collect();
    db.record_search_audit_best_effort(query, project_id, took_ms, &returned_fact_ids);

    Ok(MemorySearchOutcome {
        results: filtered,
        route,
        fallback_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;
    use crate::core::embeddings::embedding_to_blob;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn insert_test_memory(
        db: &Db,
        project: &str,
        title: &str,
        content: &str,
        entities: &str,
        embedding: &[f32],
    ) -> String {
        let id = db
            .insert_memory(NewMemory {
                project_id: project.to_string(),
                source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: title.to_string(),
                content: content.to_string(),
                entities: entities.to_string(),
                embedding: Some(embedding_to_blob(embedding)),
                embedding_dim: Some(embedding.len() as i64),
                is_longterm: false,
            })
            .unwrap();
        id
    }

    #[test]
    fn test_sanitize_fts_query_multi_word() {
        assert_eq!(
            sanitize_fts_query("JWT authentication"),
            "JWT AND authentication"
        );
    }

    #[test]
    fn test_sanitize_fts_query_single_word() {
        assert_eq!(sanitize_fts_query("JWT"), "JWT");
    }

    #[test]
    fn test_sanitize_fts_query_with_operators() {
        assert_eq!(
            sanitize_fts_query("JWT AND authentication"),
            "JWT AND authentication"
        );
        assert_eq!(sanitize_fts_query("JWT OR auth"), "JWT AND auth");
        assert_eq!(sanitize_fts_query("NOT bad"), "bad");
    }

    #[test]
    fn test_sanitize_fts_query_special_chars() {
        assert_eq!(sanitize_fts_query("rust *** memory"), "rust AND memory");
        // Splits on non-alnum boundaries (aligns with FTS5 unicode61 tokenizer)
        assert_eq!(
            sanitize_fts_query("rust-lang (systems)"),
            "rust AND lang AND systems"
        );
        assert_eq!(
            sanitize_fts_query("title:rust ^fast"),
            "title AND rust AND fast"
        );
        assert_eq!(sanitize_fts_query("NEAR/5 test"), "5 AND test");
    }

    #[test]
    fn test_sanitize_fts_query_strips_to_empty() {
        assert_eq!(sanitize_fts_query("***"), "");
        assert_eq!(sanitize_fts_query("AND OR NOT"), "");
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("   "), "");
    }

    #[test]
    fn test_sanitize_fts_query_consecutive_spaces() {
        assert_eq!(
            sanitize_fts_query("JWT    authentication"),
            "JWT AND authentication"
        );
    }

    #[test]
    fn test_sanitize_fts_query_mixed_case_reserved() {
        assert_eq!(sanitize_fts_query("rust And memory"), "rust AND memory");
        assert_eq!(sanitize_fts_query("near Or far"), "far");
    }

    #[test]
    fn test_fts5_search_keyword() {
        let db = test_db();
        insert_test_memory(
            &db,
            "proj",
            "JWT Auth Decision",
            "Use JWT tokens for authentication",
            "auth,jwt",
            &[1.0, 0.0, 0.0],
        );
        insert_test_memory(
            &db,
            "proj",
            "Database Choice",
            "Use PostgreSQL for persistence",
            "database,postgresql",
            &[0.0, 1.0, 0.0],
        );

        let results = db.search_fts("JWT", Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].id.is_empty());
    }

    #[test]
    fn test_fts5_search_empty_query() {
        let db = test_db();
        insert_test_memory(&db, "proj", "Test", "content", "tag", &[1.0, 0.0, 0.0]);
        let results = db.search_fts("", Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_fts5_search_respects_project() {
        let db = test_db();
        insert_test_memory(
            &db,
            "proj-a",
            "Auth Decision",
            "Use JWT tokens",
            "auth",
            &[1.0, 0.0, 0.0],
        );
        insert_test_memory(
            &db,
            "proj-b",
            "Auth Decision",
            "Use OAuth tokens",
            "auth",
            &[1.0, 0.0, 0.0],
        );

        let results = db.search_fts("tokens", Some("proj-a"), 10).unwrap();
        assert_eq!(results.len(), 1);

        let results_global = db.search_fts("tokens", None, 10).unwrap();
        assert_eq!(results_global.len(), 2);
    }

    #[test]
    fn test_fts5_search_skips_expired() {
        let db = test_db();
        let id = insert_test_memory(
            &db,
            "proj",
            "Old Decision",
            "Use Redis for caching",
            "redis",
            &[1.0, 0.0, 0.0],
        );
        db.invalidate_memory(&id, None, None).unwrap();

        let results = db.search_fts("Redis", Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_rrf_merge_combines_rankers() {
        let fts = vec![
            FtsResult {
                id: "a".to_string(),
                bm25_score: -5.0,
            },
            FtsResult {
                id: "b".to_string(),
                bm25_score: -3.0,
            },
        ];
        let vec = vec![
            VectorResult {
                id: "b".to_string(),
                score: 0.9,
            },
            VectorResult {
                id: "c".to_string(),
                score: 0.8,
            },
        ];

        let merged = rrf_merge(&fts, &vec, 60.0);

        // "b" appears in both → highest RRF score
        assert_eq!(merged[0].0, "b");
        // All three IDs present
        let ids: Vec<&str> = merged.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_rrf_merge_raw_scores() {
        let fts = vec![FtsResult {
            id: "a".to_string(),
            bm25_score: -5.0,
        }];
        let vec = vec![VectorResult {
            id: "a".to_string(),
            score: 0.9,
        }];

        let merged = rrf_merge(&fts, &vec, 60.0);
        // "a" in both rankers at rank 1 → score = 2 * 1/(60+1) ≈ 0.0328
        assert_eq!(merged.len(), 1);
        let expected = 2.0 / 61.0;
        assert!((merged[0].1 - expected).abs() < 0.001);
    }

    #[test]
    fn test_rrf_better_than_single_ranker() {
        // "a" matches keyword only (high BM25, low vector)
        // "b" matches meaning only (low BM25, high vector)
        let fts = vec![
            FtsResult {
                id: "a".to_string(),
                bm25_score: -10.0,
            }, // rank 1
               // "b" not in FTS results
        ];
        let vec = vec![
            VectorResult {
                id: "b".to_string(),
                score: 0.95,
            }, // rank 1
               // "a" not in vector results (or very low)
        ];

        let merged = rrf_merge(&fts, &vec, 60.0);
        let ids: Vec<&str> = merged.iter().map(|(id, _)| id.as_str()).collect();
        // RRF includes both — neither FTS-only nor vector-only would
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn test_memory_search_updates_recall() {
        let db = test_db();
        let id = insert_test_memory(
            &db,
            "proj",
            "JWT Auth",
            "Use JWT tokens for auth",
            "auth,jwt",
            &[1.0, 0.0, 0.0],
        );

        let results = db
            .memory_search("JWT", &[0.9, 0.1, 0.0], Some("proj"), 10)
            .unwrap();
        assert!(!results.is_empty());

        // Check recall was updated
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(entry.recall_count > 0);
        assert!(entry.avg_relevance > 0.0);
        assert!(entry.last_recalled.is_some());
    }

    #[test]
    fn test_memory_search_global_scope() {
        let db = test_db();
        insert_test_memory(
            &db,
            "proj-a",
            "Auth A",
            "JWT tokens for A",
            "auth",
            &[1.0, 0.0, 0.0],
        );
        insert_test_memory(
            &db,
            "proj-b",
            "Auth B",
            "JWT tokens for B",
            "auth",
            &[0.9, 0.1, 0.0],
        );

        let results = db.memory_search("JWT", &[1.0, 0.0, 0.0], None, 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_memory_search_scores_normalized() {
        let db = test_db();
        // Insert entry that matches both FTS and vector — should get highest possible RRF score
        insert_test_memory(
            &db,
            "proj",
            "JWT Auth",
            "Use JWT tokens for auth",
            "auth,jwt",
            &[1.0, 0.0, 0.0],
        );
        insert_test_memory(
            &db,
            "proj",
            "DB Choice",
            "Use PostgreSQL for persistence",
            "db",
            &[0.0, 1.0, 0.0],
        );

        // "JWT auth tokens" now uses AND-term matching: "JWT AND auth AND tokens"
        // FTS5 should match the JWT Auth entry (contains "JWT", "auth", "tokens")
        // Both FTS and vector match → dual-ranker → score should be > 0.5
        let results = db
            .memory_search("JWT auth tokens", &[0.9, 0.1, 0.0], Some("proj"), 10)
            .unwrap();
        assert!(!results.is_empty());
        // Dual-ranker hits produce scores > 0.5 (confirming FTS5 is now contributing)
        assert!(
            results[0].score > 0.5,
            "dual-ranker normalized score should be > 0.5, got {}",
            results[0].score
        );
        for r in &results {
            assert!(r.score >= 0.0, "score should be >= 0.0, got {}", r.score);
            assert!(r.score <= 1.0, "score should be <= 1.0, got {}", r.score);
        }
    }

    #[test]
    fn test_fts5_multi_word_non_adjacent_match() {
        let db = test_db();
        // "JWT" in title, "authentication" in content — NOT adjacent
        insert_test_memory(
            &db,
            "proj",
            "JWT tokens",
            "for authentication and authorization",
            "auth,jwt",
            &[1.0, 0.0, 0.0],
        );

        // AND-term matching: "JWT AND authentication" should match even though terms are non-adjacent
        let results = db
            .search_fts("JWT authentication", Some("proj"), 10)
            .unwrap();
        assert!(
            !results.is_empty(),
            "FTS5 AND-term should match non-adjacent terms across title and content"
        );
    }

    // =========================================================================
    // BL-008 Step 3: search-path decay re-rank (apply_boost_and_decay helper)
    // =========================================================================

    fn entry_with(last_recalled: Option<&str>, is_longterm: bool) -> MemoryEntry {
        MemoryEntry {
            id: "id".to_string(),
            project_id: "p".to_string(),
            source_file: String::new(),
            source_type: String::new(),
            knowledge_type: String::new(),
            title: String::new(),
            content: String::new(),
            entities: String::new(),
            valid_from: String::new(),
            valid_until: None,
            superseded_by: None,
            recall_count: 0,
            avg_relevance: 0.5,
            last_recalled: last_recalled.map(|s| s.to_string()),
            embedding: None,
            embedding_dim: None,
            is_longterm,
            created_at: String::new(),
        }
    }

    fn frozen_now() -> chrono::DateTime<chrono::Utc> {
        chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 7, 20, 12, 0, 0).unwrap()
    }

    fn rfc3339_n_days_ago(d: i64) -> String {
        (frozen_now() - chrono::Duration::days(d)).to_rfc3339()
    }

    #[test]
    fn apply_boost_no_last_recalled_no_decay_penalty() {
        let entry = entry_with(None, false);
        let out = apply_boost_and_decay(0.5, &entry, frozen_now());
        assert_eq!(out, 0.5, "never-recalled non-longterm memory: no change");
    }

    #[test]
    fn apply_boost_longterm_fresh_applies_boost_full() {
        let entry = entry_with(Some(&rfc3339_n_days_ago(0)), true);
        let out = apply_boost_and_decay(0.5, &entry, frozen_now());
        // d=0 → decay_factor=1.0 → 0.5 × 1.2 × 1.0 = 0.6
        assert!((out - 0.6).abs() < 1e-9, "expected 0.6, got {out}");
    }

    #[test]
    fn apply_boost_longterm_stale_reduced_by_decay() {
        // 60 days old, longterm: 0.5 × 1.2 × 0.5 = 0.3
        let entry = entry_with(Some(&rfc3339_n_days_ago(60)), true);
        let out = apply_boost_and_decay(0.5, &entry, frozen_now());
        assert!((out - 0.3).abs() < 1e-6, "expected 0.3, got {out}");
    }

    #[test]
    fn apply_boost_nonlongterm_stale_still_decays() {
        // 60 days old, not longterm: 0.5 × 0.5 = 0.25 (no boost, yes decay)
        let entry = entry_with(Some(&rfc3339_n_days_ago(60)), false);
        let out = apply_boost_and_decay(0.5, &entry, frozen_now());
        assert!((out - 0.25).abs() < 1e-6, "expected 0.25, got {out}");
    }

    #[test]
    fn apply_boost_clamps_at_one() {
        let entry = entry_with(Some(&rfc3339_n_days_ago(0)), true);
        // 0.9 × 1.2 × 1.0 = 1.08 → clamped to 1.0
        let out = apply_boost_and_decay(0.9, &entry, frozen_now());
        assert_eq!(out, 1.0);
    }

    #[test]
    fn apply_boost_ranks_fresh_above_stale_with_equal_avg_relevance() {
        // Two longterm memories, same avg_relevance (pre-boost score), different ages.
        let fresh = entry_with(Some(&rfc3339_n_days_ago(1)), true);
        let stale = entry_with(Some(&rfc3339_n_days_ago(60)), true);
        let fresh_score = apply_boost_and_decay(0.5, &fresh, frozen_now());
        let stale_score = apply_boost_and_decay(0.5, &stale, frozen_now());
        assert!(
            fresh_score > stale_score,
            "fresh ({fresh_score}) should rank above stale ({stale_score})"
        );
        // Ratio should match decay_factor(1) / decay_factor(60) ≈ 0.9885 / 0.5 ≈ 1.977
        let ratio = fresh_score / stale_score;
        let expected_ratio = decay::decay_factor(1.0) / decay::decay_factor(60.0);
        assert!(
            (ratio - expected_ratio).abs() < 1e-6,
            "ratio {ratio} should match decay_factor ratio {expected_ratio}"
        );
    }

    #[test]
    fn apply_boost_same_age_clock_invariant_with_dreaming_pass() {
        // Same-age-clock invariant: given identical (avg_relevance, last_recalled, now),
        // search's decay factor and Dreaming's effective_relevance must agree on the
        // multiplier. We verify by computing both and checking the ratio.
        let now = frozen_now();
        let last_str = rfc3339_n_days_ago(30);
        let last_dt = chrono::DateTime::parse_from_rfc3339(&last_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        // Dreaming's effective_relevance for avg=1.0 → IS the decay factor.
        let dreaming_side = crate::core::decay::effective_relevance(1.0, last_dt, now);

        // Search's non-longterm path for normalized=1.0 → IS the decay factor.
        let entry = entry_with(Some(&last_str), false);
        let search_side = apply_boost_and_decay(1.0, &entry, now);

        assert!(
            (dreaming_side - search_side).abs() < 1e-9,
            "same-age-clock invariant violated: dreaming={dreaming_side} vs search={search_side}"
        );
    }

    #[test]
    fn apply_boost_malformed_last_recalled_falls_back_to_no_decay() {
        // Graceful — a malformed timestamp must not panic and must not apply decay.
        let entry = entry_with(Some("not-a-date"), false);
        let out = apply_boost_and_decay(0.5, &entry, frozen_now());
        assert_eq!(out, 0.5, "malformed timestamp: treat as no decay");
    }

    // ---- F-003 Step 2 HARD GATE benchmark (plan F-003 / discussion 001 Topic 6) ----

    /// HARD CORRECTNESS GATE on FTS-only score normalization function selection.
    ///
    /// The 5 fixture inputs span 500x (`{0.1, 1.0, 5.0, 10.0, 50.0}`) — a
    /// pathological range that surfaces upper-end compression in non-linear
    /// normalization functions. Three candidates were evaluated:
    ///
    /// - **sigmoid**: `1.0 / (1.0 + (-x).exp())` — compresses upper to ≈[0.99, 1.0].
    /// - **tanh-half-positive**: `(x.tanh() + 1.0) / 2.0` — compresses even faster.
    /// - **linear-rescale-by-per-call-max**: `(x - min) / max(max-min, 1.0)` — preserves
    ///   discriminability via per-call rescaling.
    ///
    /// Acceptance: chosen function MUST pass (a) all outputs in [0, 1],
    /// (b) `output(50) - output(5) >= 0.10` (upper-range discriminability),
    /// (c) monotonicity. Sigmoid and tanh both FAIL (b) — kept here as
    /// negative-assertion sanity checks to prevent future maintainers from
    /// accidentally re-introducing them.
    #[test]
    fn test_fts_score_normalization_discriminability() {
        let bm25_inputs = [0.1f64, 1.0, 5.0, 10.0, 50.0];

        // Candidate functions:
        let sigmoid = |x: f64| 1.0 / (1.0 + (-x).exp());
        let tanh_pos = |x: f64| (x.tanh() + 1.0) / 2.0;

        let sigmoid_outputs: Vec<f64> = bm25_inputs.iter().map(|&x| sigmoid(x)).collect();
        let tanh_outputs: Vec<f64> = bm25_inputs.iter().map(|&x| tanh_pos(x)).collect();

        // Linear-rescale via the production `linear_rescale_normalize` helper —
        // ensures the test pins the exact function shape the orchestrator uses.
        let fts_input: Vec<FtsResult> = bm25_inputs
            .iter()
            .map(|&x| FtsResult {
                id: format!("fact-{x}"),
                bm25_score: x,
            })
            .collect();
        let linear_outputs = linear_rescale_normalize(&fts_input);

        // ---- Negative-assertion sanity checks: sigmoid + tanh DO compress ----
        // (proves rejection rationale; prevents future re-adoption)
        assert!(
            sigmoid_outputs[4] - sigmoid_outputs[2] < 0.10,
            "sigmoid MUST compress upper range (output(50) - output(5) < 0.10) — got {} - {} = {}",
            sigmoid_outputs[4],
            sigmoid_outputs[2],
            sigmoid_outputs[4] - sigmoid_outputs[2]
        );
        assert!(
            tanh_outputs[4] - tanh_outputs[2] < 0.10,
            "tanh-half-positive MUST compress upper range — got {} - {} = {}",
            tanh_outputs[4],
            tanh_outputs[2],
            tanh_outputs[4] - tanh_outputs[2]
        );

        // ---- Chosen function (linear-rescale-by-per-call-max) MUST pass ----

        // (a) all outputs in [0, 1]
        for (i, &out) in linear_outputs.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&out),
                "linear-rescale output[{i}] = {out} out of [0, 1]"
            );
        }

        // (b) upper-range discriminability: output(50) - output(5) >= 0.10
        let upper_diff = linear_outputs[4] - linear_outputs[2];
        assert!(
            upper_diff >= 0.10,
            "linear-rescale upper-range discriminability FAILED: output(50)-output(5) = {} - {} = {} < 0.10",
            linear_outputs[4],
            linear_outputs[2],
            upper_diff
        );

        // (c) monotonicity
        for i in 1..linear_outputs.len() {
            assert!(
                linear_outputs[i] > linear_outputs[i - 1],
                "linear-rescale non-monotonic at index {}: {} → {}",
                i,
                linear_outputs[i - 1],
                linear_outputs[i]
            );
        }
    }

    /// Edge case: empty input → empty output (no panic).
    #[test]
    fn test_linear_rescale_normalize_empty() {
        let out = linear_rescale_normalize(&[]);
        assert!(out.is_empty());
    }

    /// Edge case: single element → [0.0] (max-min=0; range clamped to 1.0).
    #[test]
    fn test_linear_rescale_normalize_single() {
        let inp = vec![FtsResult {
            id: "fact-1".to_string(),
            bm25_score: 7.5,
        }];
        let out = linear_rescale_normalize(&inp);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], 0.0);
    }

    /// Edge case: all-equal inputs → [0, 0, ...] (range clamp prevents division blowup).
    #[test]
    fn test_linear_rescale_normalize_equal_inputs() {
        let inp: Vec<FtsResult> = (0..5)
            .map(|i| FtsResult {
                id: format!("fact-{i}"),
                bm25_score: 5.0,
            })
            .collect();
        let out = linear_rescale_normalize(&inp);
        for &x in &out {
            assert_eq!(x, 0.0);
        }
    }
}
