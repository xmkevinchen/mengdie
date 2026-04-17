use std::collections::HashMap;

use anyhow::Context;

use super::db::{Db, MemoryEntry};
use super::vector::VectorResult;

/// Score multiplier for long-term memories (promoted by Dreaming).
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

/// FTS5 reserved words that must be filtered from query tokens.
const FTS5_RESERVED: &[&str] = &["AND", "OR", "NOT", "NEAR"];

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
    pub fn search_fts(
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
        let mut results = Vec::new();
        for (id, score) in &top_ids {
            if let Some(entry) = self.get_memory(id)? {
                let normalized = (*score / RRF_MAX).clamp(0.0, 1.0);
                let boosted = if entry.is_longterm {
                    (normalized * LONGTERM_BOOST).min(1.0)
                } else {
                    normalized
                };
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
}
