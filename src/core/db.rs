use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::schema::run_migrations;

/// Shared database handle, safe to clone across async tasks.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

/// A memory entry as stored in the database.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub project_id: String,
    pub source_file: String,
    pub source_type: String,
    pub knowledge_type: String,
    pub title: String,
    pub content: String,
    pub entities: String, // comma-separated
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub superseded_by: Option<String>,
    pub recall_count: i64,
    pub avg_relevance: f64,
    pub last_recalled: Option<String>,
    pub embedding: Option<Vec<u8>>,
    pub embedding_dim: Option<i64>,
    pub is_longterm: bool,
    pub created_at: String,
}

/// Input for creating a new memory entry.
pub struct NewMemory {
    pub project_id: String,
    pub source_file: String,
    pub source_type: String,
    pub knowledge_type: String,
    pub title: String,
    pub content: String,
    pub entities: String,
    pub embedding: Option<Vec<u8>>,
    pub embedding_dim: Option<i64>,
}

impl Db {
    /// Open or create the database at the given path, running migrations.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open database at {}", path.display()))?;
        run_migrations(&conn)
            .with_context(|| format!("failed to run migrations on {}", path.display()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Default database path: ~/.second-brain/db.sqlite
    pub fn default_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".second-brain").join("db.sqlite")
    }

    /// Insert a new memory, returning its generated ID.
    pub fn insert_memory(&self, mem: NewMemory) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, embedding, embedding_dim, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id,
                mem.project_id,
                mem.source_file,
                mem.source_type,
                mem.knowledge_type,
                mem.title,
                mem.content,
                mem.entities,
                now,
                mem.embedding,
                mem.embedding_dim,
                now,
            ],
        )?;
        Ok(id)
    }

    /// Get a memory by ID.
    pub fn get_memory(&self, id: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let entry = conn
            .query_row(
                "SELECT id, project_id, source_file, source_type, knowledge_type,
                        title, content, entities, valid_from, valid_until,
                        superseded_by, recall_count, avg_relevance, last_recalled,
                        embedding, embedding_dim, is_longterm, created_at
                 FROM memory_entries WHERE id = ?1",
                params![id],
                |row| row_to_entry(row),
            )
            .optional()?;
        Ok(entry)
    }

    /// Mark a memory as invalid (set valid_until and optionally superseded_by).
    pub fn invalidate_memory(
        &self,
        id: &str,
        superseded_by: Option<&str>,
    ) -> anyhow::Result<bool> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let rows = conn.execute(
            "UPDATE memory_entries SET valid_until = ?1, superseded_by = ?2 WHERE id = ?3",
            params![now, superseded_by, id],
        )?;
        Ok(rows > 0)
    }

    /// Update recall statistics after a search hit. Returns false if ID not found.
    pub fn record_recall(&self, id: &str, relevance_score: f64) -> anyhow::Result<bool> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        // Running average: new_avg = (old_avg * count + new_score) / (count + 1)
        let rows = conn.execute(
            "UPDATE memory_entries SET
                avg_relevance = (avg_relevance * recall_count + ?1) / (recall_count + 1),
                recall_count = recall_count + 1,
                last_recalled = ?2
             WHERE id = ?3",
            params![relevance_score, now, id],
        )?;
        Ok(rows > 0)
    }

    /// Set a memory as long-term (promoted by Dreaming). Returns false if ID not found.
    pub fn promote_to_longterm(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let rows = conn.execute(
            "UPDATE memory_entries SET is_longterm = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(rows > 0)
    }

    /// Access the underlying connection lock.
    pub fn lock_conn(
        &self,
    ) -> anyhow::Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))
    }

    /// Count all memories for a given project.
    pub fn count_memories(&self, project_id: &str) -> anyhow::Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        source_file: row.get(2)?,
        source_type: row.get(3)?,
        knowledge_type: row.get(4)?,
        title: row.get(5)?,
        content: row.get(6)?,
        entities: row.get(7)?,
        valid_from: row.get(8)?,
        valid_until: row.get(9)?,
        superseded_by: row.get(10)?,
        recall_count: row.get(11)?,
        avg_relevance: row.get(12)?,
        last_recalled: row.get(13)?,
        embedding: row.get(14)?,
        embedding_dim: row.get(15)?,
        is_longterm: row.get::<_, i64>(16)? != 0,
        created_at: row.get(17)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn sample_memory(project_id: &str) -> NewMemory {
        NewMemory {
            project_id: project_id.to_string(),
            source_file: format!("docs/discussions/{}/conclusion.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "Auth middleware decision".to_string(),
            content: "Use JWT tokens with Redis session store".to_string(),
            entities: "auth,middleware,jwt".to_string(),
            embedding: None,
            embedding_dim: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(entry.title, "Auth middleware decision");
        assert_eq!(entry.project_id, "test-project");
        assert_eq!(entry.knowledge_type, "decisional");
        assert_eq!(entry.recall_count, 0);
        assert!(!entry.is_longterm);
    }

    #[test]
    fn test_get_nonexistent() {
        let db = test_db();
        let entry = db.get_memory("nonexistent").unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn test_invalidate() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        let updated = db.invalidate_memory(&id, Some("new-id")).unwrap();
        assert!(updated);
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(entry.valid_until.is_some());
        assert_eq!(entry.superseded_by.as_deref(), Some("new-id"));
    }

    #[test]
    fn test_record_recall() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();

        assert!(db.record_recall(&id, 0.8).unwrap());
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(entry.recall_count, 1);
        assert!((entry.avg_relevance - 0.8).abs() < 0.001);

        assert!(db.record_recall(&id, 0.6).unwrap());
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(entry.recall_count, 2);
        assert!((entry.avg_relevance - 0.7).abs() < 0.001);

        // Non-existent ID returns false
        assert!(!db.record_recall("nonexistent", 0.5).unwrap());
    }

    #[test]
    fn test_promote_to_longterm() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        assert!(db.promote_to_longterm(&id).unwrap());
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(entry.is_longterm);

        // Non-existent ID returns false
        assert!(!db.promote_to_longterm("nonexistent").unwrap());
    }

    #[test]
    fn test_count_memories() {
        let db = test_db();
        db.insert_memory(sample_memory("proj-a")).unwrap();
        db.insert_memory(sample_memory("proj-a")).unwrap();
        db.insert_memory(sample_memory("proj-b")).unwrap();
        assert_eq!(db.count_memories("proj-a").unwrap(), 2);
        assert_eq!(db.count_memories("proj-b").unwrap(), 1);
        assert_eq!(db.count_memories("proj-c").unwrap(), 0);
    }

    #[test]
    fn test_fts5_search() {
        let db = test_db();
        db.insert_memory(sample_memory("test-project")).unwrap();
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH 'jwt'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_default_path() {
        let path = Db::default_path();
        assert!(path.to_string_lossy().contains(".second-brain"));
        assert!(path.to_string_lossy().ends_with("db.sqlite"));
    }
}
