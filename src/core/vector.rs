use anyhow::Context;
use rusqlite::params;

use super::db::Db;
use super::embeddings::{embedding_to_blob, validate_embedding};

/// Embedding dimension required by the sqlite-vec `vec_memories` virtual
/// table. MUST match `schema.rs::VEC_DIM` (single source of truth lives
/// in schema.rs since it's a column-declaration constant). Duplicated
/// here as a compile-time-checked sanity net for the strict dimension
/// assert in `search_vector`.
const VEC_DIM: usize = 384;

/// A scored search result from vector similarity search.
#[derive(Debug, Clone)]
pub struct VectorResult {
    pub id: String,
    pub score: f32, // cosine similarity, 0.0-1.0
}

impl Db {
    /// Store an embedding for a memory entry.
    /// Validates dimension matches the expected model dimension.
    pub fn store_embedding(
        &self,
        id: &str,
        embedding: &[f32],
        expected_dim: usize,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            embedding.len() == expected_dim,
            "embedding dimension mismatch: got {}, expected {}",
            embedding.len(),
            expected_dim
        );
        validate_embedding(embedding)?;
        let blob = embedding_to_blob(embedding);
        let dim = expected_dim as i64;
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE memory_entries SET embedding = ?1, embedding_dim = ?2 WHERE id = ?3",
            params![blob, dim, id],
        )?;
        Ok(())
    }

    /// ANN cosine-similarity search via sqlite-vec `vec_memories` virtual
    /// table (BL-026 Step 2 adoption, replaces 2026-04 brute-force scan).
    /// Returns results sorted by descending similarity, limited to `limit`.
    /// Skips expired entries (valid_until set and in the past).
    ///
    /// **Strict dimension**: query_embedding MUST be `VEC_DIM` (= 384).
    /// vec0 vtables are dim-fixed at creation; mismatched-dim queries
    /// would error inside SQLite. The Rust-side check produces a clearer
    /// error message earlier.
    ///
    /// **Score**: cosine `distance ∈ [0, 2]` → `similarity = 1 - distance/2 ∈ [0, 1]`.
    /// For unit-normalized vectors (fastembed default) this matches the
    /// previous brute-force `cosine_similarity` semantics for non-negative
    /// scores. RRF merge in `search.rs` is rank-based per F-001 finding,
    /// so absolute-score change is non-breaking; ranking order preserved.
    ///
    /// `pub(crate)` post-F-003 (plan F-003 Step 3 / discussion 001 Topic 3
    /// hybrid): only `search::memory_search` (the existing hybrid orchestrator)
    /// calls this primitive. Direct external callers would bypass the RRF
    /// merge + boost-and-decay logic in `memory_search`.
    pub(crate) fn search_vector(
        &self,
        query_embedding: &[f32],
        project_id: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<VectorResult>> {
        anyhow::ensure!(
            query_embedding.len() == VEC_DIM,
            "vec0 search query dim mismatch: got {}, expected {}",
            query_embedding.len(),
            VEC_DIM
        );
        validate_embedding(query_embedding)?;

        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let q_blob = embedding_to_blob(query_embedding);
        let limit_i = limit as i64;

        // vec0 KNN with project + valid_until filter via IN subquery. The
        // candidate-set restriction is evaluated BEFORE the KNN scan
        // (sqlite-vec auxiliary-filter pattern), so KNN only ranks the
        // already-filtered subset.
        let (sql, params_vec): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
            Some(pid) => (
                "SELECT v.memory_id, v.distance \
                 FROM vec_memories v \
                 WHERE v.embedding MATCH ?1 AND v.k = ?2 \
                   AND v.memory_id IN ( \
                       SELECT id FROM memory_entries \
                       WHERE project_id = ?3 \
                         AND (valid_until IS NULL OR valid_until > ?4) \
                   ) \
                 ORDER BY v.distance",
                vec![
                    Box::new(q_blob) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit_i),
                    Box::new(pid.to_string()),
                    Box::new(now),
                ],
            ),
            None => (
                "SELECT v.memory_id, v.distance \
                 FROM vec_memories v \
                 WHERE v.embedding MATCH ?1 AND v.k = ?2 \
                   AND v.memory_id IN ( \
                       SELECT id FROM memory_entries \
                       WHERE valid_until IS NULL OR valid_until > ?3 \
                   ) \
                 ORDER BY v.distance",
                vec![
                    Box::new(q_blob) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit_i),
                    Box::new(now),
                ],
            ),
        };

        let mut stmt = conn.prepare(sql).context("prepare vec0 search")?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;

        let mut results: Vec<VectorResult> = Vec::new();
        for row in rows {
            let (id, distance) = row?;
            // distance ∈ [0, 2] for unit-normalized fastembed output;
            // similarity = 1 - distance/2 ∈ [0, 1].
            let score = 1.0_f32 - (distance as f32) / 2.0_f32;
            results.push(VectorResult { id, score });
        }
        // SQL already orders by distance ASC == similarity DESC; truncation
        // already enforced by `v.k = ?2` matching `limit`.
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;
    use crate::core::embeddings::embedding_to_blob;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    /// Zero-pad a low-dim "key" embedding to the 384-d vec0 contract.
    /// Lets test cases stay readable (`make_384d(&[1.0, 0.0, 0.0])`) while
    /// satisfying the post-BL-026 dim-strict `search_vector` invariant.
    fn make_384d(base: &[f32]) -> Vec<f32> {
        let mut v = vec![0.0_f32; 384];
        for (i, &x) in base.iter().enumerate() {
            v[i] = x;
        }
        v
    }

    fn mem_with_embedding(
        project_id: &str,
        title: &str,
        embedding: &[f32],
    ) -> (NewMemory, Vec<u8>) {
        let mem = NewMemory {
            project_id: project_id.to_string(),
            source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: title.to_string(),
            content: format!("Content about {title}"),
            entities: "test".to_string(),
            embedding: Some(embedding_to_blob(embedding)),
            embedding_dim: Some(embedding.len() as i64),
            is_longterm: false,
        };
        (mem, embedding_to_blob(embedding))
    }

    #[test]
    fn test_store_and_search_embedding() {
        let db = test_db();
        // Create a memory, then store embedding separately
        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Test".to_string(),
                content: "test content".to_string(),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
                is_longterm: false,
            })
            .unwrap();

        let emb = vec![1.0_f32, 0.0, 0.0];
        db.store_embedding(&id, &emb, 3).unwrap();

        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(entry.embedding_dim, Some(3));
        assert!(entry.embedding.is_some());
    }

    #[test]
    fn test_store_embedding_dimension_mismatch() {
        let db = test_db();
        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Test".to_string(),
                content: "test content".to_string(),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
                is_longterm: false,
            })
            .unwrap();

        let emb = vec![1.0_f32, 0.0, 0.0];
        let err = db.store_embedding(&id, &emb, 5);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("dimension mismatch"));
    }

    #[test]
    fn test_vector_search_returns_closest() {
        let db = test_db();

        // Insert 3 memories with different embeddings (384-d for vec0).
        let (m1, _) = mem_with_embedding("proj", "auth decision", &make_384d(&[1.0, 0.0, 0.0]));
        let (m2, _) = mem_with_embedding("proj", "database choice", &make_384d(&[0.0, 1.0, 0.0]));
        let (m3, _) = mem_with_embedding("proj", "auth and db", &make_384d(&[0.7, 0.7, 0.0]));

        db.insert_memory(m1).unwrap();
        db.insert_memory(m2).unwrap();
        db.insert_memory(m3).unwrap();

        // Query closest to [1, 0, 0, 0, …] → should be "auth decision"
        let q = make_384d(&[1.0, 0.0, 0.0]);
        let results = db.search_vector(&q, Some("proj"), 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(
            results[0].id,
            db.search_vector(&q, Some("proj"), 1).unwrap()[0].id
        );
        assert!((results[0].score - 1.0).abs() < 0.001); // exact match
    }

    #[test]
    fn test_vector_search_respects_project_filter() {
        let db = test_db();

        let (m1, _) = mem_with_embedding("proj-a", "decision A", &make_384d(&[1.0, 0.0, 0.0]));
        let (m2, _) = mem_with_embedding("proj-b", "decision B", &make_384d(&[1.0, 0.0, 0.0]));

        db.insert_memory(m1).unwrap();
        db.insert_memory(m2).unwrap();

        let q = make_384d(&[1.0, 0.0, 0.0]);
        let results_a = db.search_vector(&q, Some("proj-a"), 10).unwrap();
        assert_eq!(results_a.len(), 1);

        // Global search (no project filter)
        let results_all = db.search_vector(&q, None, 10).unwrap();
        assert_eq!(results_all.len(), 2);
    }

    #[test]
    fn test_vector_search_skips_expired() {
        let db = test_db();

        let (m1, _) = mem_with_embedding("proj", "valid memory", &make_384d(&[1.0, 0.0, 0.0]));
        let id = db.insert_memory(m1).unwrap();

        // Should find it
        let q = make_384d(&[1.0, 0.0, 0.0]);
        let results = db.search_vector(&q, Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 1);

        // Invalidate it
        db.invalidate_memory(&id, None, None).unwrap();

        // Should no longer find it
        let results = db.search_vector(&q, Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
