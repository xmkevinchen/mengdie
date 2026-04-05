use rusqlite::params;

use super::db::Db;

/// Result of a Dreaming promotion pass.
#[derive(Debug)]
pub struct DreamingResult {
    pub promoted: usize,
    /// Candidates that met thresholds but were not promoted (should be 0 normally).
    pub candidates_not_promoted: usize,
    /// Total non-longterm valid memories in the project.
    pub total_eligible: usize,
}

impl Db {
    /// Run the Dreaming promotion pass.
    /// Promotes memories where: recall_count >= 3 AND avg_relevance >= 0.65
    /// AND last_recalled within 14 days. Sets is_longterm = true.
    pub fn run_dreaming(&self) -> anyhow::Result<DreamingResult> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now();
        let cutoff = (now - chrono::Duration::days(14)).to_rfc3339();

        // Count total non-longterm valid memories BEFORE promotion
        let total_valid: usize = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries
             WHERE is_longterm = 0 AND valid_until IS NULL",
            [],
            |row| row.get::<_, i64>(0).map(|v| v as usize),
        )?;

        // Count candidates that meet thresholds
        let total_checked: usize = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries
             WHERE is_longterm = 0
             AND valid_until IS NULL
             AND recall_count >= 3
             AND avg_relevance >= 0.65
             AND last_recalled IS NOT NULL
             AND last_recalled >= ?1",
            params![cutoff],
            |row| row.get::<_, i64>(0).map(|v| v as usize),
        )?;

        // Promote qualifying memories
        let promoted = conn.execute(
            "UPDATE memory_entries
             SET is_longterm = 1
             WHERE is_longterm = 0
             AND valid_until IS NULL
             AND recall_count >= 3
             AND avg_relevance >= 0.65
             AND last_recalled IS NOT NULL
             AND last_recalled >= ?1",
            params![cutoff],
        )?;

        Ok(DreamingResult {
            promoted,
            candidates_not_promoted: total_checked.saturating_sub(promoted),
            total_eligible: total_valid,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;

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

        let result = db.run_dreaming().unwrap();
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

        let result = db.run_dreaming().unwrap();
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

        let result = db.run_dreaming().unwrap();
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
        let result = db.run_dreaming().unwrap();
        assert_eq!(result.promoted, 1);

        // Second pass — already long-term, should not re-promote
        let result = db.run_dreaming().unwrap();
        assert_eq!(result.promoted, 0);
    }

    #[test]
    fn test_dreaming_skips_invalidated() {
        let db = test_db();
        let id = insert_mem(&db, "Invalidated Memory");

        for _ in 0..5 {
            db.record_recall(&id, 0.8).unwrap();
        }
        db.invalidate_memory(&id, None).unwrap();

        let result = db.run_dreaming().unwrap();
        assert_eq!(result.promoted, 0);
    }
}
