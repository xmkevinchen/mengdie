use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, ToSql};

use super::clustering::cluster_memories;
use super::db::{parse_last_recalled, Db, NewMemory};
use super::decay;
use super::embeddings::{embedding_to_blob, Embedder, EmbeddingContext};
use super::llm::LlmProvider;
use super::synthesis::{
    build_synthesis_prompt, parse_synthesis_response, SynthesisInput, SynthesisOutcome,
    SYNTHESIS_OUTPUT_SCHEMA,
};

// -- Default thresholds --
// See BL-002-8: overridable via CLI flags.

/// Default minimum recall count for promotion.
pub const DEFAULT_MIN_RECALL: i64 = 3;
/// Default minimum average relevance for promotion.
pub const DEFAULT_MIN_RELEVANCE: f64 = 0.45;
/// Default recency window (days) — last_recalled must be within this window.
pub const DEFAULT_WINDOW_DAYS: i64 = 14;

/// Configurable thresholds for a Dreaming pass.
#[derive(Debug, Clone)]
pub struct DreamingConfig {
    pub min_recall: i64,
    pub min_relevance: f64,
    pub window_days: i64,
}

impl Default for DreamingConfig {
    fn default() -> Self {
        Self {
            min_recall: DEFAULT_MIN_RECALL,
            min_relevance: DEFAULT_MIN_RELEVANCE,
            window_days: DEFAULT_WINDOW_DAYS,
        }
    }
}

/// Result of a Dreaming pass (promotion + decay/demotion).
#[derive(Debug)]
pub struct DreamingResult {
    pub promoted: usize,
    /// Candidates that met thresholds but were not promoted (should be 0 normally).
    pub candidates_not_promoted: usize,
    /// Total non-longterm valid memories in the project.
    pub total_eligible: usize,
    // ----- BL-008 decay/demotion counters -----
    /// Long-term memories demoted by this pass (cleared `is_longterm`).
    /// Always `0` when `write_demotions == false` (dry-run).
    pub demoted: usize,
    /// Mean effective_relevance across all `is_longterm = 1` memories with a
    /// non-null `last_recalled`, measured BEFORE any demotion write.
    pub avg_effective_score_before: f64,
    /// Mean effective_relevance after demotions have been applied (live) or
    /// identical to `avg_effective_score_before` (dry-run — no writes).
    pub avg_effective_score_after: f64,
    /// Count of memories whose effective relevance fell below the floor.
    /// Equals `demoted` in live mode; can be `> demoted` only in dry-run.
    pub decay_floor_breaches: usize,
    /// IDs of memories whose effective relevance fell below the floor.
    /// Populated identically in live and dry-run — consumed by CLI approval
    /// gate in Step 5 and `--decay-dry-run` output in Step 4.
    pub breached_ids: Vec<String>,
}

impl Db {
    /// Run the Dreaming pass (promotion + decay/demotion) with default
    /// thresholds and wall-clock time. Always writes demotions. This is the
    /// production entry point; tests inject a frozen clock or suppress writes
    /// via `run_dreaming_with_config`.
    pub fn run_dreaming(&self, project_id: Option<&str>) -> anyhow::Result<DreamingResult> {
        self.run_dreaming_with_config(project_id, &DreamingConfig::default(), None, true)
    }

    /// Run the Dreaming pass with configurable thresholds, injectable clock,
    /// and controllable demotion writes.
    ///
    /// - `now = None` → `chrono::Utc::now()`; deterministic tests pass `Some(frozen)`.
    /// - `write_demotions = false` → dry-run: compute `decay_floor_breaches`
    ///   and `breached_ids` but do NOT clear any `is_longterm` flags.
    ///   `avg_effective_score_after == avg_effective_score_before` exactly.
    /// - `write_demotions = true` → live: `UPDATE is_longterm = 0` for each
    ///   breached id; `demoted = affected_rows`.
    pub fn run_dreaming_with_config(
        &self,
        project_id: Option<&str>,
        config: &DreamingConfig,
        now: Option<DateTime<Utc>>,
        write_demotions: bool,
    ) -> anyhow::Result<DreamingResult> {
        let conn = self.lock_conn()?;
        let now = now.unwrap_or_else(Utc::now);
        let cutoff = (now - chrono::Duration::days(config.window_days)).to_rfc3339();

        let project_filter_simple = project_id.map(|_| "AND project_id = ?1").unwrap_or("");
        let project_filter = project_id.map(|_| "AND project_id = ?4").unwrap_or("");

        // === Promotion pass (unchanged from pre-BL-008) ===

        // Count total non-longterm valid memories BEFORE promotion
        let count_sql = format!(
            "SELECT COUNT(*) FROM memory_entries
             WHERE is_longterm = 0 AND valid_until IS NULL {project_filter_simple}"
        );
        let total_valid: usize = match project_id {
            Some(pid) => conn.query_row(&count_sql, params![pid], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })?,
            None => conn.query_row(&count_sql, [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })?,
        };

        // Threshold query used for both count and promote
        let threshold_where = format!(
            "WHERE is_longterm = 0
             AND valid_until IS NULL
             AND recall_count >= ?1
             AND avg_relevance >= ?2
             AND last_recalled IS NOT NULL
             AND last_recalled >= ?3 {project_filter}"
        );

        // Count candidates that meet thresholds
        let sql = format!("SELECT COUNT(*) FROM memory_entries {threshold_where}");
        let total_checked: usize = match project_id {
            Some(pid) => conn.query_row(
                &sql,
                params![config.min_recall, config.min_relevance, cutoff, pid],
                |row| row.get::<_, i64>(0).map(|v| v as usize),
            )?,
            None => conn.query_row(
                &sql,
                params![config.min_recall, config.min_relevance, cutoff],
                |row| row.get::<_, i64>(0).map(|v| v as usize),
            )?,
        };

        // Promote qualifying memories
        let sql = format!("UPDATE memory_entries SET is_longterm = 1 {threshold_where}");
        let promoted = match project_id {
            Some(pid) => conn.execute(
                &sql,
                params![config.min_recall, config.min_relevance, cutoff, pid],
            )?,
            None => conn.execute(
                &sql,
                params![config.min_recall, config.min_relevance, cutoff],
            )?,
        };

        // === Decay / demotion pass (BL-008) ===
        //
        // LONGTERM_BOOST cliff: clearing `is_longterm` here removes the 1.2×
        // boost applied at `search.rs:~142`. The next search drops the
        // memory from `normalized × 1.2 × decay` to `normalized × decay` —
        // intentional discontinuity, NOT a bug. See
        // docs/discussions/019-power-law-decay/conclusion.md Topic 3.

        // Select all live long-term memories with non-null last_recalled for the decay scan.
        // Rows with NULL last_recalled are skipped entirely (no staleness evidence).
        let select_longterm_sql = format!(
            "SELECT id, avg_relevance, last_recalled FROM memory_entries
             WHERE is_longterm = 1
               AND valid_until IS NULL
               AND last_recalled IS NOT NULL {project_filter_simple}"
        );
        let longterm_rows: Vec<(String, f64, String)> = {
            let mut stmt = conn.prepare(&select_longterm_sql)?;
            let mapper = |row: &rusqlite::Row<'_>| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            };
            match project_id {
                Some(pid) => stmt
                    .query_map(params![pid], mapper)?
                    .collect::<rusqlite::Result<_>>()?,
                None => stmt
                    .query_map([], mapper)?
                    .collect::<rusqlite::Result<_>>()?,
            }
        };

        let mut sum_effective_before: f64 = 0.0;
        let mut counted_before: usize = 0;
        let mut breached_ids: Vec<String> = Vec::new();
        for (id, avg, last) in &longterm_rows {
            let last_dt = match parse_last_recalled(last) {
                Some(dt) => dt,
                None => continue, // malformed timestamp — skip defensively
            };
            let eff = decay::effective_relevance(*avg, last_dt, now);
            sum_effective_before += eff;
            counted_before += 1;
            if decay::should_demote(eff) {
                breached_ids.push(id.clone());
            }
        }
        let avg_effective_score_before = if counted_before == 0 {
            0.0
        } else {
            sum_effective_before / counted_before as f64
        };
        let decay_floor_breaches = breached_ids.len();

        // Observability: log NULL-last_recalled long-term memories (they are
        // excluded from decay entirely — documented skip rule).
        let null_count_sql = format!(
            "SELECT COUNT(*) FROM memory_entries
             WHERE is_longterm = 1
               AND valid_until IS NULL
               AND last_recalled IS NULL {project_filter_simple}"
        );
        let null_skip: usize = match project_id {
            Some(pid) => conn.query_row(&null_count_sql, params![pid], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })?,
            None => conn.query_row(&null_count_sql, [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })?,
        };
        if null_skip > 0 {
            tracing::info!(
                skipped = null_skip,
                "skipping decay for long-term memories with NULL last_recalled"
            );
        }

        // Conditional demotion write. Chunked to respect SQLite bind-variable
        // limits (default 999 per statement; bundled SQLite raises this, but
        // keep the conservative 500 for portability).
        //
        // Guard `AND is_longterm = 1` in the WHERE clause is defensive: the
        // scan above already filtered to `is_longterm = 1`, but if a row is
        // concurrently demoted by another path, the guard prevents the chunk
        // from counting a no-op UPDATE toward `demoted`. Under the documented
        // invariant (live mode, no concurrent writers) `demoted ==
        // decay_floor_breaches`; under concurrency it can be less — the
        // `debug_assert!` below would fire only if the divergence happens in
        // a test, not in prod.
        let mut demoted: usize = 0;
        if write_demotions && !breached_ids.is_empty() {
            for chunk in breached_ids.chunks(500) {
                let placeholders = std::iter::repeat_n("?", chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "UPDATE memory_entries SET is_longterm = 0
                     WHERE is_longterm = 1 AND id IN ({placeholders})"
                );
                let params_dyn: Vec<&dyn ToSql> = chunk.iter().map(|s| s as &dyn ToSql).collect();
                demoted += conn.execute(&sql, params_dyn.as_slice())?;
            }
            debug_assert!(
                demoted == decay_floor_breaches,
                "live-mode invariant: demoted ({demoted}) must equal \
                 decay_floor_breaches ({decay_floor_breaches}); divergence \
                 indicates concurrent writes or a guard-clause regression"
            );
        }

        // Post-state mean. In dry-run OR when no demotions fired, this equals
        // the before-mean exactly (no writes happened). Otherwise re-scan the
        // surviving long-term set.
        let avg_effective_score_after = if !write_demotions || breached_ids.is_empty() {
            avg_effective_score_before
        } else {
            let mut stmt = conn.prepare(&select_longterm_sql)?;
            let mapper = |row: &rusqlite::Row<'_>| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            };
            let rows_after: Vec<(String, f64, String)> = match project_id {
                Some(pid) => stmt
                    .query_map(params![pid], mapper)?
                    .collect::<rusqlite::Result<_>>()?,
                None => stmt
                    .query_map([], mapper)?
                    .collect::<rusqlite::Result<_>>()?,
            };
            let mut sum: f64 = 0.0;
            let mut count: usize = 0;
            for (_, avg, last) in &rows_after {
                if let Some(dt) = parse_last_recalled(last) {
                    let eff = decay::effective_relevance(*avg, dt, now);
                    sum += eff;
                    count += 1;
                }
            }
            if count == 0 {
                0.0
            } else {
                sum / count as f64
            }
        };

        Ok(DreamingResult {
            promoted,
            candidates_not_promoted: total_checked.saturating_sub(promoted),
            total_eligible: total_valid,
            demoted,
            avg_effective_score_before,
            avg_effective_score_after,
            decay_floor_breaches,
            breached_ids,
        })
    }
}

// ============================================================================
// BL-007 — Dream Synthesis (first caller of clustering + LlmProvider)
// ============================================================================

/// Result of a single synthesis pass.
///
/// `llm_call_errors` and `parse_errors` are tracked separately so an operator
/// can distinguish transient infra failures (timeouts, rate limits — worth
/// retrying) from structural regressions (LLM wrapping JSON in fences,
/// dropping required fields — worth a prompt fix). Review feedback:
/// collapsing both into a single counter loses the discriminator the operator
/// needs to decide remediation.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SynthesisResult {
    /// Clusters seen (including ones that failed the LLM call or parse).
    pub clusters_processed: usize,
    /// Synthesis rows written (≤ clusters_processed; dry_run → always 0).
    pub syntheses_created: usize,
    /// Clusters where `provider.complete(...)` returned `Err(_)`.
    pub llm_call_errors: usize,
    /// Clusters where the LLM call succeeded but `parse_synthesis_response`
    /// failed (NoJsonObject, InvalidJson, MissingField, EmptyTitle, …).
    pub parse_errors: usize,
    /// Memories that didn't reach `min_size` and were logged + skipped.
    pub residuals_skipped: usize,
    /// Memories whose content was truncated at `CONTENT_CHAR_LIMIT` before
    /// inclusion in the prompt. Populated by the prompt-build loop via the
    /// tracing layer — if this is > 0, synthesis quality may be degraded
    /// because source signal landed past the cap.
    pub memories_truncated: usize,
    /// Clusters where the LLM returned `{"skip": true, ...}` (null-escape-
    /// hatch, plan 011). The LLM judged the cluster members as topically
    /// unrelated and declined to synthesize. No DB row written. Track
    /// separately from `llm_call_errors` and `parse_errors` — this is a
    /// deliberate opt-out, not a failure. Revisit threshold/min_size if
    /// skip rate exceeds 25% of pair-clusters across 3–5 runs.
    pub syntheses_llm_skipped: usize,
    /// Clusters of size exactly 2 processed this pass (counted PRE-DB-load
    /// at `trimmed_ids.len() == 2`, matching `clusters_processed`'s
    /// attribution stage). Denominator for the pair-cluster skip percentage
    /// reported by the CLI.
    pub pair_clusters_processed: usize,
    /// Subset of pair-clusters (size == 2) that took the
    /// `SynthesisOutcome::Skipped` branch. Numerator for the pair-cluster
    /// skip percentage reported by the CLI. MUST NOT be incremented for
    /// non-pair-cluster skips — those count toward `syntheses_llm_skipped`
    /// only. Plan 012 fixes the prior CLI bug where the numerator was
    /// `syntheses_llm_skipped` (total) against a pair-only denominator.
    pub pair_clusters_skipped: usize,
}

impl SynthesisResult {
    /// Total LLM-adjacent failures (sum of call + parse). Preserved for
    /// operators who want the old flat metric; prefer the split fields.
    pub fn llm_errors(&self) -> usize {
        self.llm_call_errors + self.parse_errors
    }
}

/// Cluster the given project's memories then either log the prompts (dry_run)
/// or feed each cluster to the LLM provider and store the resulting synthesis.
/// One LLM error per cluster increments `llm_errors` and does NOT abort the
/// pass — recovery is to re-run (content_hash dedup makes that idempotent).
///
/// Returns a `SynthesisResult` whose fields include both the outcome counters
/// (`syntheses_created`, `syntheses_llm_skipped`, error counters, …) and the
/// cluster-geometry observations (`pair_clusters_processed`,
/// `pair_clusters_skipped`) used by the CLI to compute the pair-cluster skip
/// percentage. The co-location is intentional — plan 012 consolidated these
/// onto a single struct after the plan review (unanimous challenger C win)
/// found that a metric's numerator and denominator belong together. See
/// `docs/plans/012-synthesis-cli-skip-metric.md`.
///
/// **Attribution invariant**: both `pair_clusters_processed` and its subset
/// `pair_clusters_skipped` are counted by inspecting `trimmed_ids.len() == 2`
/// PRE-DB-load (before `db.get_memories_by_ids`), matching the attribution
/// stage used by `clusters_processed`. Moving either counter to post-DB-load
/// would produce a pair-cluster skip percentage that silently undercounts on
/// DB-load misses — see plan 011 AC3 for the original denominator fix, plan
/// 012 for the numerator extension, and `BL-synthesis-preload-db-miss-edge`
/// for the remaining asymmetry (denominator counted, numerator never
/// incremented if DB load fails — low-probability edge; not yet observed).
// Crossed clippy's default arg limit (7) with the embedder addition. The
// alternative — bundling threshold/min_size/max_cluster_size into a config
// struct — is purely cosmetic and would churn 13 call-sites for no signal.
#[allow(clippy::too_many_arguments)]
pub async fn run_synthesis_pass(
    db: &Db,
    project_id: Option<&str>,
    provider: &dyn LlmProvider,
    embedder: Arc<Mutex<Embedder>>,
    threshold: f32,
    min_size: usize,
    max_cluster_size: usize,
    dry_run: bool,
) -> anyhow::Result<SynthesisResult> {
    let clustering = cluster_memories(db, project_id, threshold, min_size)?;

    let mut result = SynthesisResult {
        clusters_processed: clustering.clusters.len(),
        residuals_skipped: clustering.residuals.len(),
        ..Default::default()
    };

    if !clustering.residuals.is_empty() {
        tracing::info!(
            residuals = clustering.residuals.len(),
            "synthesis: skipping residuals (MVP policy)"
        );
    }

    for cluster in &clustering.clusters {
        let trimmed_ids: Vec<String> = cluster
            .memory_ids
            .iter()
            .take(max_cluster_size)
            .cloned()
            .collect();
        if trimmed_ids.len() < min_size {
            // Truncation pushed this cluster below min_size — skip (counted
            // under `residuals_skipped` would be wrong since these memories
            // WERE cluster-eligible; count the cluster as processed but
            // produce no synthesis).
            continue;
        }

        // Count pair-clusters PRE-DB-load for consistent denominator with
        // the pair-cluster skip numerator (plan 011 AC3; plan 012 moved the
        // counter from a local binding onto SynthesisResult). `is_pair_cluster`
        // is reused below in the Skipped branch to prevent drift between the
        // two increment sites (review feedback: duplicate `len() == 2` checks
        // are a refactor hazard).
        let is_pair_cluster = trimmed_ids.len() == 2;
        if is_pair_cluster {
            result.pair_clusters_processed += 1;
        }

        let memories = db.get_memories_by_ids(&trimmed_ids)?;
        if memories.len() < min_size {
            tracing::warn!(
                cluster_ids = ?trimmed_ids,
                loaded = memories.len(),
                "synthesis: loaded fewer rows than expected, skipping cluster"
            );
            continue;
        }

        let proj = memories[0].project_id.clone();

        // Count how many memories in this cluster will hit the 4000-char
        // truncation cap before the pure prompt builder applies it. Review
        // feedback: silent truncation loses signal; surface it in the
        // SynthesisResult so the operator can tell if synthesis quality is
        // being degraded by content truncation.
        for mem in &memories {
            if mem.content.chars().count() > super::synthesis::CONTENT_CHAR_LIMIT {
                result.memories_truncated += 1;
            }
        }

        let input = SynthesisInput {
            cluster_memories: &memories,
            cluster_centroid: &cluster.centroid,
            project_id: &proj,
        };
        let (system, user) = build_synthesis_prompt(&input);

        if dry_run {
            tracing::info!(
                cluster_size = memories.len(),
                system_len = system.len(),
                user_len = user.len(),
                "synthesis: dry-run, prompt built, skipping LLM + write"
            );
            println!(
                "DRY-RUN cluster ({} memories):\nSYSTEM:\n{system}\n\nUSER:\n{user}\n---",
                memories.len()
            );
            continue;
        }

        let raw = match provider
            .complete_structured(&system, &user, SYNTHESIS_OUTPUT_SCHEMA)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    cluster_ids = ?trimmed_ids,
                    error = %e,
                    "synthesis: LLM call failed, skipping cluster"
                );
                result.llm_call_errors += 1;
                continue;
            }
        };

        let draft = match parse_synthesis_response(&raw, &trimmed_ids) {
            Ok(SynthesisOutcome::Synthesized(draft)) => draft,
            Ok(SynthesisOutcome::Skipped { reason }) => {
                // Null-escape-hatch (plan 011): LLM judged the cluster as
                // lacking a common thread and declined to synthesize. Count
                // it as a skip (not an error), no DB write. Log level varies
                // by reason quality: info for normal skips with a reason,
                // warn when reason is empty (prompt-adherence signal — the
                // LLM should include a reason per prompt instruction; a flood
                // of empty reasons means the prompt is failing the `reason`
                // field expectation). Review feedback: empty `reason=""` log
                // lines have low audit signal on their own; escalating to
                // warn makes prompt-drift visible.
                let log_reason = if reason.is_empty() {
                    "(unspecified)".to_string()
                } else {
                    reason.clone()
                };
                if reason.is_empty() {
                    tracing::warn!(
                        cluster_ids = ?trimmed_ids,
                        cluster_size = trimmed_ids.len(),
                        reason = %log_reason,
                        "synthesis: LLM skipped cluster (null-escape-hatch) — EMPTY reason, check prompt adherence"
                    );
                } else {
                    tracing::info!(
                        cluster_ids = ?trimmed_ids,
                        cluster_size = trimmed_ids.len(),
                        reason = %log_reason,
                        "synthesis: LLM skipped cluster (null-escape-hatch)"
                    );
                }
                result.syntheses_llm_skipped += 1;
                if is_pair_cluster {
                    // Pair-cluster skip subset — numerator of the pair-cluster
                    // skip percentage (plan 012 AC2). Same `is_pair_cluster`
                    // binding as the denominator increment above — single
                    // source of truth prevents the two sites from drifting
                    // if the size rule ever changes.
                    result.pair_clusters_skipped += 1;
                }
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    cluster_ids = ?trimmed_ids,
                    error = %e,
                    "synthesis: parse failed, skipping cluster"
                );
                result.parse_errors += 1;
                continue;
            }
        };

        // BL-022 / F-014: embed the synthesis content BEFORE insert so the row
        // is queryable by vector search and participates in future clustering.
        // The naive in-place embed_with_context call would (a) block the async
        // executor on 2-10ms fastembed inference and (b) make this future !Send
        // on multi-threaded tokio runtimes — both bad. Use the established
        // Arc<Mutex<Embedder>> + spawn_blocking pattern from mcp_tools.rs:387,515.
        let embedding = {
            let embedder_handle = Arc::clone(&embedder);
            let content = draft.content.clone();
            let ctx = EmbeddingContext {
                knowledge_type: "factual".to_string(),
                entities: draft.entities.clone(),
                project_id: proj.clone(),
                title: draft.title.clone(),
            };
            tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<f32>> {
                let mut emb = embedder_handle
                    .lock()
                    .map_err(|e| anyhow::anyhow!("embedder lock poisoned: {e}"))?;
                emb.embed_with_context(&content, &ctx)
            })
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("spawn_blocking failed: {e}")))?
        };
        let embedding_dim = embedding.len() as i64;

        let new_mem = NewMemory {
            project_id: proj.clone(),
            source_file: format!("synthesis/{}.md", uuid::Uuid::new_v4()),
            source_type: "synthesis".to_string(),
            knowledge_type: "factual".to_string(),
            title: draft.title,
            content: draft.content,
            entities: draft.entities,
            embedding: Some(embedding_to_blob(&embedding)),
            embedding_dim: Some(embedding_dim),
            is_longterm: false, // syntheses earn long-term via dreaming, not by construction
        };

        db.insert_synthesis_with_links(new_mem, &draft.source_memory_ids)?;
        result.syntheses_created += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;

    /// Embedder for the dreaming test module — synthesis pass requires an
    /// `Arc<Mutex<Embedder>>` per F-014's BL-022 fix. We use real
    /// `Embedder::new()` here (matches `tests/common/mod.rs::ensure_embedder_warm`
    /// convention): the ~90MB fastembed model downloads once per cold
    /// `~/.cache/fastembed/`, then in-memory ONNX session-load cost is
    /// ~100-200ms per test invocation. Acceptable for synthesis-path tests
    /// that exercise the real embed call. BL-051 will revisit when the
    /// per-test load cost becomes painful.
    fn test_embedder() -> Arc<Mutex<Embedder>> {
        Arc::new(Mutex::new(
            Embedder::new().expect("Embedder::new failed in dreaming test"),
        ))
    }

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn insert_mem(db: &Db, title: &str) -> String {
        db.insert_memory(NewMemory {
            project_id: "proj".to_string(),
            source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: title.to_string(),
            content: "test content".to_string(),
            entities: "test".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .unwrap()
    }

    #[test]
    fn test_dreaming_promotes_qualifying() {
        let db = test_db();
        let id = insert_mem(&db, "Popular Memory");

        // Simulate 5 recalls with high relevance
        for _ in 0..5 {
            db.record_recall(&id, 0.8).unwrap();
        }

        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 1);

        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(entry.is_longterm);
    }

    #[test]
    fn test_dreaming_skips_low_recall() {
        let db = test_db();
        let id = insert_mem(&db, "Rarely Used");

        // Only 1 recall — below threshold of 3
        db.record_recall(&id, 0.9).unwrap();

        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 0);

        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(!entry.is_longterm);
    }

    #[test]
    fn test_dreaming_skips_low_relevance() {
        let db = test_db();
        let id = insert_mem(&db, "Low Quality");

        // 5 recalls but low relevance
        for _ in 0..5 {
            db.record_recall(&id, 0.3).unwrap();
        }

        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 0);
    }

    #[test]
    fn test_dreaming_skips_already_longterm() {
        let db = test_db();
        let id = insert_mem(&db, "Already Promoted");

        for _ in 0..5 {
            db.record_recall(&id, 0.8).unwrap();
        }

        // First pass promotes
        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 1);

        // Second pass — already long-term, should not re-promote
        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 0);
    }

    #[test]
    fn test_dreaming_skips_invalidated() {
        let db = test_db();
        let id = insert_mem(&db, "Invalidated Memory");

        for _ in 0..5 {
            db.record_recall(&id, 0.8).unwrap();
        }
        db.invalidate_memory(&id, None, None).unwrap();

        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.promoted, 0);
    }

    // ========================================================================
    // BL-007 — synthesis pass tests (stub LlmProvider)
    // ========================================================================

    use crate::core::embeddings::embedding_to_blob;
    use crate::core::llm::{LlmError, LlmFuture, LlmProvider};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FixedProvider {
        payload: String,
        call_count: AtomicUsize,
    }

    impl FixedProvider {
        fn new(payload: impl Into<String>) -> Self {
            Self {
                payload: payload.into(),
                call_count: AtomicUsize::new(0),
            }
        }
    }

    impl LlmProvider for FixedProvider {
        fn complete<'a>(&'a self, _system: &'a str, _prompt: &'a str) -> LlmFuture<'a> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let payload = self.payload.clone();
            Box::pin(async move { Ok(payload) })
        }
        // Plan 019 Step 3: dreaming.rs now invokes complete_structured.
        // Mock delegates to the same canned payload — schema is ignored
        // (the mock's `payload` is already a JSON string mimicking what
        // claude-CLI's `.structured_output` field would carry).
        fn complete_structured<'a>(
            &'a self,
            system: &'a str,
            prompt: &'a str,
            _schema: &'a str,
        ) -> LlmFuture<'a> {
            self.complete(system, prompt)
        }
        fn model(&self) -> &str {
            "stub-fixed"
        }
    }

    struct PanicProvider;
    impl LlmProvider for PanicProvider {
        fn complete<'a>(&'a self, _system: &'a str, _prompt: &'a str) -> LlmFuture<'a> {
            Box::pin(async { panic!("PanicProvider::complete must not be called in dry_run") })
        }
        fn complete_structured<'a>(
            &'a self,
            _system: &'a str,
            _prompt: &'a str,
            _schema: &'a str,
        ) -> LlmFuture<'a> {
            Box::pin(async {
                panic!("PanicProvider::complete_structured must not be called in dry_run")
            })
        }
        fn model(&self) -> &str {
            "stub-panic"
        }
    }

    struct TimeoutOnFirst {
        counter: AtomicUsize,
        ok_payload: String,
    }
    impl LlmProvider for TimeoutOnFirst {
        fn complete<'a>(&'a self, _system: &'a str, _prompt: &'a str) -> LlmFuture<'a> {
            let n = self.counter.fetch_add(1, Ordering::SeqCst);
            let payload = self.ok_payload.clone();
            Box::pin(async move {
                if n == 0 {
                    Err(LlmError::Timeout(std::time::Duration::from_millis(1)))
                } else {
                    Ok(payload)
                }
            })
        }
        fn complete_structured<'a>(
            &'a self,
            system: &'a str,
            prompt: &'a str,
            _schema: &'a str,
        ) -> LlmFuture<'a> {
            self.complete(system, prompt)
        }
        fn model(&self) -> &str {
            "stub-timeout-first"
        }
    }

    fn make_384d(base: &[f32]) -> Vec<f32> {
        let mut v = vec![0.0_f32; 384];
        for (i, &b) in base.iter().enumerate() {
            v[i] = b;
        }
        v
    }

    fn insert_with_emb(db: &Db, project_id: &str, title: &str, base: &[f32], nudge: f32) -> String {
        let mut e = make_384d(base);
        e[3] = nudge;
        db.insert_memory(NewMemory {
            project_id: project_id.to_string(),
            source_file: format!("{title}-{}.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: title.to_string(),
            content: format!("content for {title} {}", uuid::Uuid::new_v4()),
            entities: "test".to_string(),
            embedding: Some(embedding_to_blob(&e)),
            embedding_dim: Some(e.len() as i64),
            is_longterm: false,
        })
        .unwrap()
    }

    fn seed_tight_cluster(db: &Db, project: &str, n: usize, base: &[f32]) -> Vec<String> {
        (0..n)
            .map(|i| {
                insert_with_emb(
                    db,
                    project,
                    &format!("{project}-{i}"),
                    base,
                    0.001 * i as f32,
                )
            })
            .collect()
    }

    const OK_JSON: &str = r#"{"title":"Consolidated","content":"This synthesis consolidates the cluster.","entities":["x","y"]}"#;

    #[tokio::test]
    async fn test_synthesis_dry_run_makes_no_llm_calls_no_writes() {
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);

        let provider = PanicProvider;
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            true,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 1);
        assert_eq!(r.syntheses_created, 0);
        assert_eq!(r.llm_errors(), 0);
        // No synthesis rows should exist
        let total: i64 = {
            let conn = db.lock_conn().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis'",
                [],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(total, 0);
    }

    #[tokio::test]
    async fn test_synthesis_stub_creates_expected_rows() {
        let db = Db::open_in_memory().unwrap();
        let src_ids = seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);

        let provider = FixedProvider::new(OK_JSON);
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 1);
        assert_eq!(r.syntheses_created, 1);
        assert_eq!(r.llm_errors(), 0);
        assert_eq!(provider.call_count.load(Ordering::SeqCst), 1);

        // 1 synthesis row in DB with source_type = "synthesis" and is_longterm = 0
        let (count, syn_id): (i64, String) = {
            let conn = db.lock_conn().unwrap();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM memory_entries WHERE source_type = 'synthesis' AND is_longterm = 0",
                )
                .unwrap();
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            (ids.len() as i64, ids.into_iter().next().unwrap())
        };
        assert_eq!(count, 1);

        // 3 link rows pointing at the synthesis
        let links = db.count_synthesis_links(&syn_id).unwrap();
        assert_eq!(links, 3);
        // and each source is represented
        for sid in &src_ids {
            let present: i64 = {
                let conn = db.lock_conn().unwrap();
                conn.query_row(
                    "SELECT COUNT(*) FROM memory_synthesis_links \
                     WHERE source_memory_id = ?1 AND synthesis_memory_id = ?2",
                    params![sid, syn_id],
                    |r| r.get(0),
                )
                .unwrap()
            };
            assert_eq!(present, 1, "missing link for source {sid}");
        }
    }

    #[tokio::test]
    async fn test_synthesis_llm_error_isolated_from_other_clusters() {
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);
        seed_tight_cluster(&db, "proj", 3, &[0.0, 1.0, 0.0]);

        let provider = TimeoutOnFirst {
            counter: AtomicUsize::new(0),
            ok_payload: OK_JSON.to_string(),
        };
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 2);
        assert_eq!(r.syntheses_created, 1);
        assert_eq!(r.llm_errors(), 1);
    }

    #[tokio::test]
    async fn test_synthesis_rerun_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);

        let provider = FixedProvider::new(OK_JSON);
        let r1 = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r1.syntheses_created, 1);

        let r2 = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            false,
        )
        .await
        .unwrap();
        // Re-run still counts the cluster as processed + "created" in-stat, but
        // the DB write is a no-op via content_hash ON CONFLICT DO UPDATE.
        assert_eq!(r2.syntheses_created, 1);

        // Net row count: exactly one synthesis, exactly three link rows.
        let (syn_count, link_count): (i64, i64) = {
            let conn = db.lock_conn().unwrap();
            let s: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let l: i64 = conn
                .query_row("SELECT COUNT(*) FROM memory_synthesis_links", [], |r| {
                    r.get(0)
                })
                .unwrap();
            (s, l)
        };
        assert_eq!(syn_count, 1);
        assert_eq!(link_count, 3);
    }

    #[tokio::test]
    async fn test_synthesis_max_cluster_size_below_min_yields_no_synthesis() {
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 5, &[1.0, 0.0, 0.0]);

        let provider = PanicProvider; // must not be called
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            2,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 1);
        assert_eq!(r.syntheses_created, 0);
        assert_eq!(r.llm_errors(), 0);
    }

    #[tokio::test]
    async fn test_synthesis_threshold_changes_cluster_count() {
        // 3 near-identical + 3 near-identical on a different axis.
        // threshold 0.5 → both groups cluster (2 clusters).
        // threshold 0.99 + small noise → same groups still cluster (2).
        // threshold 1.5 → no clusters.
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);
        seed_tight_cluster(&db, "proj", 3, &[0.0, 1.0, 0.0]);

        let provider = FixedProvider::new(OK_JSON);

        let r_high = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            1.5,
            3,
            20,
            true,
        )
        .await
        .unwrap();
        assert_eq!(r_high.clusters_processed, 0);

        let r_low = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            true,
        )
        .await
        .unwrap();
        assert_eq!(r_low.clusters_processed, 2);
    }

    // ========================================================================
    // BL-residuals-reduction (plan 011) — null-escape-hatch tests
    // ========================================================================

    const SKIP_JSON: &str = r#"{"skip": true, "reason": "topically adjacent"}"#;

    #[tokio::test]
    async fn test_synthesis_skip_increments_counter_no_db_write() {
        // Stub provider returns skip-JSON for the single pair-cluster fixture.
        // Expected: syntheses_llm_skipped == 1, syntheses_created == 0,
        // zero rows in memory_entries, zero link rows.
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 2, &[1.0, 0.0, 0.0]); // pair cluster

        let provider = FixedProvider::new(SKIP_JSON);
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            2,
            20,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 1);
        assert_eq!(r.syntheses_created, 0);
        assert_eq!(r.syntheses_llm_skipped, 1);
        assert_eq!(r.pair_clusters_processed, 1);
        assert_eq!(r.pair_clusters_skipped, 1);

        // No synthesis row, no link rows.
        let syn_count: i64 = {
            let conn = db.lock_conn().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis'",
                [],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(syn_count, 0);
        let link_count: i64 = {
            let conn = db.lock_conn().unwrap();
            conn.query_row("SELECT COUNT(*) FROM memory_synthesis_links", [], |r| {
                r.get(0)
            })
            .unwrap()
        };
        assert_eq!(link_count, 0);
    }

    /// Cluster-size-aware stub: inspects the user prompt to count
    /// "--- MEMORY N ---" separators. If exactly 2 memories → return SKIP_JSON;
    /// otherwise return OK_JSON. Lets us MIX skip (pair-clusters) with
    /// synthesis (triple-clusters) in a single test — which is necessary to
    /// verify the pair_skip_pct denominator discrimination (review feedback:
    /// an all-skip fixture can't distinguish pair-count from total-count).
    struct ClusterSizeAwareProvider;
    impl LlmProvider for ClusterSizeAwareProvider {
        fn complete<'a>(&'a self, _system: &'a str, prompt: &'a str) -> LlmFuture<'a> {
            let memory_count = prompt.matches("--- MEMORY ").count();
            let payload = if memory_count == 2 {
                SKIP_JSON.to_string()
            } else {
                OK_JSON.to_string()
            };
            Box::pin(async move { Ok(payload) })
        }
        fn complete_structured<'a>(
            &'a self,
            system: &'a str,
            prompt: &'a str,
            _schema: &'a str,
        ) -> LlmFuture<'a> {
            self.complete(system, prompt)
        }
        fn model(&self) -> &str {
            "stub-cluster-size-aware"
        }
    }

    #[tokio::test]
    async fn test_synthesis_pair_skip_percentage_computed_against_pairs() {
        // Fixture: 2 pair-clusters (2 memories each) + 2 triple-clusters
        // (3 memories each). Stub skips ONLY the pair-clusters; triples
        // synthesize normally. Expected:
        //   - syntheses_created == 2 (from 2 triple-clusters)
        //   - syntheses_llm_skipped == 2 (from 2 pair-clusters)
        //   - pair_count == 2 (denominator = pairs, NOT total)
        //   - CLI arithmetic: 2 * 100 / 2 = 100% (all pair-clusters skipped)
        // A buggy implementation that used total-cluster denominator would
        // compute 2 * 100 / 4 = 50%. This test discriminates: mixed outcomes
        // are necessary to expose the bug. Plan 011 AC3.
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 2, &[1.0, 0.0, 0.0]);
        seed_tight_cluster(&db, "proj", 2, &[0.0, 1.0, 0.0]);
        seed_tight_cluster(&db, "proj", 3, &[0.0, 0.0, 1.0]);
        seed_tight_cluster(&db, "proj", 3, &[1.0, 1.0, 0.0]);

        let provider = ClusterSizeAwareProvider;
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            2,
            20,
            false,
        )
        .await
        .unwrap();

        assert_eq!(r.clusters_processed, 4);
        assert_eq!(
            r.pair_clusters_processed, 2,
            "expected 2 pair-clusters pre-DB-load"
        );
        assert_eq!(
            r.syntheses_llm_skipped, 2,
            "expected 2 skips (pair-clusters only)"
        );
        assert_eq!(
            r.pair_clusters_skipped, 2,
            "all skips came from pair-clusters — pair-skipped equals total in this fixture"
        );
        assert_eq!(
            r.syntheses_created, 2,
            "expected 2 syntheses (triple-clusters only)"
        );

        // Exercise the CLI pair_skip_pct arithmetic directly: (S_pair * 100) / P.
        // With S_pair=2 pair-cluster skips and P=2 pair-clusters → 100%. If
        // the implementation mistakenly divided by total clusters (4), it
        // would yield 50%. Plan 012: numerator is pair_clusters_skipped
        // (NOT syntheses_llm_skipped — they coincide here only because the
        // ClusterSizeAwareProvider skips only pairs by design). The
        // `test_pair_clusters_skipped_excludes_non_pair_skips` test below
        // exercises the case where they differ.
        let pair_skip_pct = (r.pair_clusters_skipped * 100) / r.pair_clusters_processed;
        assert_eq!(
            pair_skip_pct, 100,
            "pair_skip_pct must use pair_clusters_processed as denominator"
        );
    }

    #[tokio::test]
    async fn test_pair_clusters_skipped_excludes_non_pair_skips() {
        // Discrimination fixture for plan 012 AC2: 2 pair-clusters (2 memories
        // each) + 2 triple-clusters (3 memories each). `FixedProvider` returns
        // SKIP_JSON for every call, so all 4 clusters take the Skipped branch.
        //
        // Expected:
        //   - pair_clusters_processed == 2 (denominator)
        //   - pair_clusters_skipped == 2 (numerator — ONLY the pair-cluster skips)
        //   - syntheses_llm_skipped == 4 (all 4 clusters skipped)
        //   - syntheses_created == 0
        //
        // A buggy implementation that incremented `pair_clusters_skipped` on
        // every skip would yield pair_clusters_skipped == 4 — this test
        // discriminates that bug. Companion to
        // `test_synthesis_pair_skip_percentage_computed_against_pairs` which
        // discriminates the denominator; this one discriminates the numerator.
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 2, &[1.0, 0.0, 0.0]);
        seed_tight_cluster(&db, "proj", 2, &[0.0, 1.0, 0.0]);
        seed_tight_cluster(&db, "proj", 3, &[0.0, 0.0, 1.0]);
        seed_tight_cluster(&db, "proj", 3, &[1.0, 1.0, 0.0]);

        let provider = FixedProvider::new(SKIP_JSON);
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            2,
            20,
            false,
        )
        .await
        .unwrap();

        assert_eq!(r.clusters_processed, 4);
        assert_eq!(r.pair_clusters_processed, 2);
        assert_eq!(
            r.pair_clusters_skipped, 2,
            "pair-cluster skip counter must exclude the 2 triple-cluster skips"
        );
        assert_eq!(
            r.syntheses_llm_skipped, 4,
            "total LLM-skip counter includes all 4 (2 pairs + 2 triples)"
        );
        assert_eq!(r.syntheses_created, 0);

        // CLI arithmetic check: (S_pair * 100) / P = (2 * 100) / 2 = 100%.
        // A buggy impl using syntheses_llm_skipped as numerator would yield
        // (4 * 100) / 2 = 200% — impossible for a percentage, exposing the bug.
        let pair_skip_pct = (r.pair_clusters_skipped * 100) / r.pair_clusters_processed;
        assert_eq!(pair_skip_pct, 100);
    }

    #[tokio::test]
    async fn test_synthesis_skip_precedence_over_title_content() {
        // Plan 011 review (ai-engineer P3): explicit precedence test. When
        // the LLM emits both `skip: true` AND synthesis fields, the skip
        // must win. Prevents a future parser refactor from silently
        // flipping the precedence rule.
        let db = Db::open_in_memory().unwrap();
        seed_tight_cluster(&db, "proj", 3, &[1.0, 0.0, 0.0]);

        const SKIP_PLUS_FIELDS: &str = r#"{"skip":true,"reason":"adjacent","title":"Ignored","content":"Ignored body.","entities":["x"]}"#;
        let provider = FixedProvider::new(SKIP_PLUS_FIELDS);
        let r = run_synthesis_pass(
            &db,
            Some("proj"),
            &provider,
            test_embedder(),
            0.9,
            3,
            20,
            false,
        )
        .await
        .unwrap();
        assert_eq!(r.clusters_processed, 1);
        assert_eq!(r.syntheses_created, 0, "skip must win over title+content");
        assert_eq!(r.syntheses_llm_skipped, 1);

        // No synthesis row should have been written with the "Ignored" title.
        let found_ignored: i64 = {
            let conn = db.lock_conn().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE title = 'Ignored'",
                [],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(
            found_ignored, 0,
            "synthesis body from skip=true response must NOT land in DB"
        );
    }

    // =========================================================================
    // BL-008 decay / demotion tests (plan 013 Step 2)
    // =========================================================================

    /// Insert a memory with unique content (dodges content_hash dedup) and
    /// fabricate a long-term row with a specific `last_recalled` + `avg_relevance`,
    /// bypassing promotion thresholds.
    fn seed_longterm(db: &Db, title: &str, avg: f64, last_recalled: Option<&str>) -> String {
        let uid = uuid::Uuid::new_v4();
        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: format!("test-{uid}.md"),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: title.to_string(),
                content: format!("seed content for {title} ({uid})"),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
                is_longterm: false,
            })
            .unwrap();
        let conn = db.lock_conn().unwrap();
        conn.execute(
            "UPDATE memory_entries SET is_longterm = 1, avg_relevance = ?1, last_recalled = ?2
             WHERE id = ?3",
            params![avg, last_recalled, id],
        )
        .unwrap();
        drop(conn);
        id
    }

    fn frozen_now() -> chrono::DateTime<chrono::Utc> {
        chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 7, 20, 12, 0, 0).unwrap()
    }

    fn days_before(now: chrono::DateTime<chrono::Utc>, d: i64) -> String {
        (now - chrono::Duration::days(d)).to_rfc3339()
    }

    #[test]
    fn decay_demotes_below_floor_and_preserves_above() {
        let db = test_db();
        let now = frozen_now();
        // Fresh recall (d=15) at avg=0.50 → effective ≈ 0.420 > 0.20, survives.
        let fresh = seed_longterm(&db, "Fresh", 0.50, Some(&days_before(now, 15)));
        // Stale recall (d=78) at avg=0.487 → effective ≈ 0.198 < 0.20, demotes.
        let stale = seed_longterm(&db, "Stale", 0.487, Some(&days_before(now, 78)));
        // Deep-stale (d=137) at avg=0.487 → effective ≈ 0.100, demotes hard.
        let deep = seed_longterm(&db, "Deep", 0.487, Some(&days_before(now, 137)));

        let result = db
            .run_dreaming_with_config(None, &DreamingConfig::default(), Some(now), true)
            .unwrap();

        assert_eq!(result.demoted, 2);
        assert_eq!(result.decay_floor_breaches, 2);
        assert_eq!(result.breached_ids.len(), 2);
        assert!(result.breached_ids.contains(&stale));
        assert!(result.breached_ids.contains(&deep));
        assert!(!result.breached_ids.contains(&fresh));

        let fresh_entry = db.get_memory(&fresh).unwrap().unwrap();
        let stale_entry = db.get_memory(&stale).unwrap().unwrap();
        let deep_entry = db.get_memory(&deep).unwrap().unwrap();
        assert!(fresh_entry.is_longterm, "fresh should stay promoted");
        assert!(!stale_entry.is_longterm, "stale should demote");
        assert!(!deep_entry.is_longterm, "deep-stale should demote");

        // After writes: only `fresh` remains long-term. fresh: avg=0.50, d=15
        // → effective = 0.50 × 2^(-15/60) ≈ 0.4204.
        assert!(
            (result.avg_effective_score_after - 0.4204).abs() < 0.01,
            "after mean should reflect the surviving fresh memory (~0.4204), got {}",
            result.avg_effective_score_after
        );
        assert!(
            result.avg_effective_score_before < result.avg_effective_score_after,
            "before ({}) should be below after ({}) — demotions remove low-effective rows",
            result.avg_effective_score_before,
            result.avg_effective_score_after
        );
    }

    #[test]
    fn decay_skips_null_last_recalled() {
        let db = test_db();
        let now = frozen_now();
        // NULL last_recalled — must be skipped entirely, no decay, no demotion.
        let untouchable = seed_longterm(&db, "NeverRecalled", 0.487, None);
        // Companion stale memory so the pass has work to do.
        let stale = seed_longterm(&db, "Stale", 0.487, Some(&days_before(now, 90)));

        let result = db
            .run_dreaming_with_config(None, &DreamingConfig::default(), Some(now), true)
            .unwrap();

        assert_eq!(result.demoted, 1);
        assert_eq!(result.breached_ids.len(), 1);
        assert_eq!(result.breached_ids[0], stale);

        let e = db.get_memory(&untouchable).unwrap().unwrap();
        assert!(
            e.is_longterm,
            "NULL-last_recalled memory must stay is_longterm=1"
        );
    }

    #[test]
    fn decay_dry_run_counts_breaches_but_never_writes() {
        let db = test_db();
        let now = frozen_now();
        let stale = seed_longterm(&db, "Stale", 0.487, Some(&days_before(now, 90)));
        let fresh = seed_longterm(&db, "Fresh", 0.50, Some(&days_before(now, 15)));

        let result = db
            .run_dreaming_with_config(None, &DreamingConfig::default(), Some(now), false)
            .unwrap();

        assert_eq!(result.demoted, 0, "dry-run must not demote");
        assert_eq!(
            result.decay_floor_breaches, 1,
            "breach count unchanged by write flag"
        );
        assert_eq!(result.breached_ids, vec![stale.clone()]);
        // In dry-run, _after is exactly _before (no writes happened).
        assert_eq!(
            result.avg_effective_score_before, result.avg_effective_score_after,
            "dry-run _after must equal _before exactly"
        );

        // DB state unchanged — both memories still is_longterm=1.
        assert!(db.get_memory(&stale).unwrap().unwrap().is_longterm);
        assert!(db.get_memory(&fresh).unwrap().unwrap().is_longterm);
    }

    #[test]
    fn decay_no_longterm_yields_empty_counters() {
        let db = test_db();
        let now = frozen_now();
        // No long-term memories at all.
        let result = db
            .run_dreaming_with_config(None, &DreamingConfig::default(), Some(now), true)
            .unwrap();
        assert_eq!(result.demoted, 0);
        assert_eq!(result.decay_floor_breaches, 0);
        assert!(result.breached_ids.is_empty());
        assert_eq!(result.avg_effective_score_before, 0.0);
        assert_eq!(result.avg_effective_score_after, 0.0);
    }

    #[test]
    fn decay_wrapper_backwards_compat_uses_wall_clock() {
        // `run_dreaming` (no-arg wrapper) must still work. With a freshly-recalled
        // memory (< 1 day old), decay is ~1.0, nothing demotes.
        let db = test_db();
        let fresh = seed_longterm(&db, "Fresh", 0.50, Some(&chrono::Utc::now().to_rfc3339()));
        let result = db.run_dreaming(None).unwrap();
        assert_eq!(result.demoted, 0);
        assert!(db.get_memory(&fresh).unwrap().unwrap().is_longterm);
    }
}
