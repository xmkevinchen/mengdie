use rusqlite::params;

use super::db::Db;

/// Metric keys used by the observability system.
pub const METRIC_SEARCH_COUNT: &str = "search_count";
pub const METRIC_SEARCH_NONEMPTY: &str = "search_nonempty_count";
pub const METRIC_INGEST_COUNT: &str = "ingest_count";
pub const METRIC_CONFLICT_COUNT: &str = "conflict_count";
/// Counter for failed `Db::record_search_audit_best_effort` writes
/// (F-002 Wave 1, plan F-002 Step 2). Bumped from the wrapper's Err path;
/// surfaced via `mengdie stats` and the `metrics` table for post-restart
/// audit-gap recovery analysis.
pub const METRIC_AUDIT_WRITE_FAILURES: &str = "audit_write_failures";

impl Db {
    /// Increment an integer metric by 1.
    pub fn increment_metric(&self, key: &str) -> anyhow::Result<()> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO metrics (key, value_int, updated_at)
             VALUES (?1, 1, ?2)
             ON CONFLICT(key) DO UPDATE SET
                value_int = value_int + 1,
                updated_at = ?2",
            params![key, now],
        )?;
        Ok(())
    }

    /// Get all metrics as (key, value) pairs.
    pub fn list_metrics(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT key, value_int FROM metrics ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Get an integer metric value. Returns 0 if not found.
    pub fn get_metric(&self, key: &str) -> anyhow::Result<i64> {
        let conn = self.lock_conn()?;
        let value = conn
            .query_row(
                "SELECT value_int FROM metrics WHERE key = ?1",
                params![key],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_metric() {
        let db = Db::open_in_memory().unwrap();
        db.increment_metric("test_counter").unwrap();
        db.increment_metric("test_counter").unwrap();
        db.increment_metric("test_counter").unwrap();
        assert_eq!(db.get_metric("test_counter").unwrap(), 3);
    }

    #[test]
    fn test_get_metric_missing() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_metric("nonexistent").unwrap(), 0);
    }

    #[test]
    fn test_separate_metrics() {
        let db = Db::open_in_memory().unwrap();
        db.increment_metric("a").unwrap();
        db.increment_metric("b").unwrap();
        db.increment_metric("b").unwrap();
        assert_eq!(db.get_metric("a").unwrap(), 1);
        assert_eq!(db.get_metric("b").unwrap(), 2);
    }
}
