use anyhow::Context;
use rusqlite::params;

use super::db::Db;
use super::embeddings::{blob_to_embedding, cosine_similarity, embedding_to_blob};

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
        let blob = embedding_to_blob(embedding);
        let dim = expected_dim as i64;
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE memory_entries SET embedding = ?1, embedding_dim = ?2 WHERE id = ?3",
            params![blob, dim, id],
        )?;
        Ok(())
    }

    /// Brute-force cosine similarity search over all embeddings in a project.
    /// Returns results sorted by descending similarity, limited to `limit`.
    /// Skips entries with `valid_until < now` (expired).
    pub fn search_vector(
        &self,
        query_embedding: &[f32],
        project_id: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<VectorResult>> {
        let conn = self.lock_conn()?;

        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
            Some(pid) => (
                "SELECT id, embedding FROM memory_entries \
                 WHERE embedding IS NOT NULL \
                 AND valid_until IS NULL \
                 AND project_id = ?1"
                    .to_string(),
                vec![Box::new(pid.to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT id, embedding FROM memory_entries \
                 WHERE embedding IS NOT NULL \
                 AND valid_until IS NULL"
                    .to_string(),
                vec![],
            ),
        };

        let mut stmt = conn.prepare(&sql).context("prepare vector search")?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;

        let mut results: Vec<VectorResult> = Vec::new();
        for row in rows {
            let (id, blob) = row?;
            let stored = blob_to_embedding(&blob);
            let score = cosine_similarity(query_embedding, &stored);
            results.push(VectorResult { id, score });
        }

        // Sort descending by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

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

    fn mem_with_embedding(project_id: &str, title: &str, embedding: &[f32]) -> (NewMemory, Vec<u8>) {
        let mem = NewMemory {
            project_id: project_id.to_string(),
            source_file: "test.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: title.to_string(),
            content: format!("Content about {title}"),
            entities: "test".to_string(),
            embedding: Some(embedding_to_blob(embedding)),
            embedding_dim: Some(embedding.len() as i64),
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
                source_file: "test.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Test".to_string(),
                content: "test content".to_string(),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
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
                source_file: "test.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Test".to_string(),
                content: "test content".to_string(),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
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

        // Insert 3 memories with different embeddings
        let (m1, _) = mem_with_embedding("proj", "auth decision", &[1.0, 0.0, 0.0]);
        let (m2, _) = mem_with_embedding("proj", "database choice", &[0.0, 1.0, 0.0]);
        let (m3, _) = mem_with_embedding("proj", "auth and db", &[0.7, 0.7, 0.0]);

        db.insert_memory(m1).unwrap();
        db.insert_memory(m2).unwrap();
        db.insert_memory(m3).unwrap();

        // Query closest to [1, 0, 0] → should be "auth decision"
        let results = db.search_vector(&[1.0, 0.0, 0.0], Some("proj"), 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, db.search_vector(&[1.0, 0.0, 0.0], Some("proj"), 1).unwrap()[0].id);
        assert!((results[0].score - 1.0).abs() < 0.001); // exact match
    }

    #[test]
    fn test_vector_search_respects_project_filter() {
        let db = test_db();

        let (m1, _) = mem_with_embedding("proj-a", "decision A", &[1.0, 0.0, 0.0]);
        let (m2, _) = mem_with_embedding("proj-b", "decision B", &[1.0, 0.0, 0.0]);

        db.insert_memory(m1).unwrap();
        db.insert_memory(m2).unwrap();

        let results_a = db.search_vector(&[1.0, 0.0, 0.0], Some("proj-a"), 10).unwrap();
        assert_eq!(results_a.len(), 1);

        // Global search (no project filter)
        let results_all = db.search_vector(&[1.0, 0.0, 0.0], None, 10).unwrap();
        assert_eq!(results_all.len(), 2);
    }

    #[test]
    fn test_vector_search_skips_expired() {
        let db = test_db();

        let (m1, _) = mem_with_embedding("proj", "valid memory", &[1.0, 0.0, 0.0]);
        let id = db.insert_memory(m1).unwrap();

        // Should find it
        let results = db.search_vector(&[1.0, 0.0, 0.0], Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 1);

        // Invalidate it
        db.invalidate_memory(&id, None).unwrap();

        // Should no longer find it
        let results = db.search_vector(&[1.0, 0.0, 0.0], Some("proj"), 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
