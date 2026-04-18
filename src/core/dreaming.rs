use rusqlite::params;

use super::db::Db;

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
    /// Run the Dreaming promotion pass, optionally scoped to a project.
    /// Uses default thresholds. See `run_dreaming_with_config` for custom thresholds.
    pub fn run_dreaming(&self, project_id: Option<&str>) -> anyhow::Result<DreamingResult> {
        self.run_dreaming_with_config(project_id, &DreamingConfig::default())
    }

    /// Run the Dreaming promotion pass with configurable thresholds.
    pub fn run_dreaming_with_config(
        &self,
        project_id: Option<&str>,
        config: &DreamingConfig,
    ) -> anyhow::Result<DreamingResult> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now();
        let cutoff = (now - chrono::Duration::days(config.window_days)).to_rfc3339();

        let project_filter_simple = project_id.map(|_| "AND project_id = ?1").unwrap_or("");
        let project_filter = project_id.map(|_| "AND project_id = ?4").unwrap_or("");

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
}
