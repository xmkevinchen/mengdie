use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::schema::{compute_content_hash, run_migrations};

/// Basic database statistics.
pub struct DbStats {
    pub total: i64,
    pub valid: i64,
    pub longterm: i64,
    pub recalled: i64,
}

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

    /// Default database path: ~/.mengdie/db.sqlite
    pub fn default_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".mengdie").join("db.sqlite")
    }

    /// Insert or update a memory, returning its ID.
    /// On conflict (same project_id + content_hash), updates metadata but preserves
    /// recall stats, timestamps, and ID. Atomic via ON CONFLICT DO UPDATE.
    pub fn insert_memory(&self, mem: NewMemory) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let content_hash = compute_content_hash(&mem.content);
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

        let returned_id: String = conn.query_row(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, embedding, embedding_dim,
                 created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(project_id, content_hash) DO UPDATE SET
                source_file = excluded.source_file,
                source_type = excluded.source_type,
                knowledge_type = excluded.knowledge_type,
                title = excluded.title,
                entities = excluded.entities,
                embedding = excluded.embedding,
                embedding_dim = excluded.embedding_dim
             RETURNING id",
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
                content_hash,
            ],
            |row| row.get(0),
        )?;
        Ok(returned_id)
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
                row_to_entry,
            )
            .optional()?;
        Ok(entry)
    }

    /// Mark a memory as invalid (set valid_until, optionally superseded_by and invalidation_reason).
    pub fn invalidate_memory(
        &self,
        id: &str,
        superseded_by: Option<&str>,
        reason: Option<&str>,
    ) -> anyhow::Result<bool> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let rows = conn.execute(
            "UPDATE memory_entries SET valid_until = ?1, superseded_by = ?2, invalidation_reason = ?3 WHERE id = ?4",
            params![now, superseded_by, reason, id],
        )?;
        Ok(rows > 0)
    }

    /// Insert a new memory and atomically invalidate a set of existing memories that it supersedes.
    /// All writes happen in a single SQLite transaction — safe under process death.
    pub fn insert_memory_resolving(
        &self,
        mem: NewMemory,
        resolves: &[String],
    ) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let content_hash = super::schema::compute_content_hash(&mem.content);
        let project_id = mem.project_id.clone();
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let tx = conn.transaction()?;

        let returned_id: String = tx.query_row(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, embedding, embedding_dim,
                 created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(project_id, content_hash) DO UPDATE SET
                source_file = excluded.source_file,
                source_type = excluded.source_type,
                knowledge_type = excluded.knowledge_type,
                title = excluded.title,
                entities = excluded.entities,
                embedding = excluded.embedding,
                embedding_dim = excluded.embedding_dim
             RETURNING id",
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
                content_hash,
            ],
            |row| row.get(0),
        )?;

        for old_id in resolves {
            tx.execute(
                "UPDATE memory_entries SET valid_until = ?1, superseded_by = ?2, invalidation_reason = 'superseded' WHERE id = ?3 AND project_id = ?4",
                params![now, returned_id, old_id, project_id],
            )?;
        }

        tx.commit()?;
        Ok(returned_id)
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

    /// Access the underlying connection lock. Crate-internal only to prevent
    /// external callers from holding the guard while calling other Db methods (deadlock).
    pub(crate) fn lock_conn(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))
    }

    /// Get basic stats about the database.
    pub fn stats(&self) -> anyhow::Result<DbStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))?;
        let valid: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries WHERE valid_until IS NULL",
            [],
            |r| r.get(0),
        )?;
        let longterm: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries WHERE is_longterm = 1",
            [],
            |r| r.get(0),
        )?;
        let recalled: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries WHERE recall_count > 0",
            [],
            |r| r.get(0),
        )?;
        Ok(DbStats {
            total,
            valid,
            longterm,
            recalled,
        })
    }

    /// List all memories, optionally filtered by project.
    pub fn list_memories(&self, project_id: Option<&str>) -> anyhow::Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
            Some(pid) => (
                "SELECT id, project_id, source_file, source_type, knowledge_type, \
                        title, content, entities, valid_from, valid_until, \
                        superseded_by, recall_count, avg_relevance, last_recalled, \
                        embedding, embedding_dim, is_longterm, created_at \
                 FROM memory_entries WHERE project_id = ?1 \
                 ORDER BY created_at DESC"
                    .to_string(),
                vec![Box::new(pid.to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT id, project_id, source_file, source_type, knowledge_type, \
                        title, content, entities, valid_from, valid_until, \
                        superseded_by, recall_count, avg_relevance, last_recalled, \
                        embedding, embedding_dim, is_longterm, created_at \
                 FROM memory_entries \
                 ORDER BY created_at DESC"
                    .to_string(),
                vec![],
            ),
        };
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), row_to_entry)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Rename a project_id in the database. Merges (deletes) duplicates where
    /// the same content_hash already exists under new_id.
    /// Returns (renamed_count, merged_count).
    pub fn rename_project(&self, old_id: &str, new_id: &str) -> anyhow::Result<(usize, usize)> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let tx = conn.transaction()?;

        // Step 1: Find collision rows (same content_hash exists under both old and new project)
        let collisions: Vec<(String, String)> = {
            let mut stmt = tx.prepare(
                "SELECT old.id, old.title FROM memory_entries old
                 INNER JOIN memory_entries new
                   ON old.content_hash = new.content_hash
                 WHERE old.project_id = ?1 AND new.project_id = ?2",
            )?;
            let rows = stmt.query_map(params![old_id, new_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let merged = collisions.len();

        // Step 2: Delete collision rows from old project (merge — content already exists under new)
        for (id, title) in &collisions {
            tx.execute("DELETE FROM memory_entries WHERE id = ?1", params![id])?;
            tracing::info!(id = %id, title = %title, "merged (deleted duplicate)");
        }

        // Step 3: Rename remaining rows
        let renamed = tx.execute(
            "UPDATE memory_entries SET project_id = ?1 WHERE project_id = ?2",
            params![new_id, old_id],
        )?;

        tx.commit()?;
        Ok((renamed, merged))
    }

    /// Dry-run rename: returns (would_rename, would_merge) without modifying the database.
    pub fn rename_project_dry_run(
        &self,
        old_id: &str,
        new_id: &str,
    ) -> anyhow::Result<(usize, usize)> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

        let collision_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries old
             INNER JOIN memory_entries new
               ON old.content_hash = new.content_hash
             WHERE old.project_id = ?1 AND new.project_id = ?2",
            params![old_id, new_id],
            |row| row.get(0),
        )?;

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries WHERE project_id = ?1",
            params![old_id],
            |row| row.get(0),
        )?;

        let rename_count = (total - collision_count).max(0) as usize;
        Ok((rename_count, collision_count as usize))
    }

    /// List all distinct project_ids with their memory counts.
    pub fn list_projects(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT project_id, COUNT(*) FROM memory_entries GROUP BY project_id ORDER BY COUNT(*) DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
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
        let uid = uuid::Uuid::new_v4();
        NewMemory {
            project_id: project_id.to_string(),
            source_file: format!("docs/discussions/{}/conclusion.md", uid),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "Auth middleware decision".to_string(),
            content: format!("Use JWT tokens with Redis session store ({})", uid),
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
        let updated = db
            .invalidate_memory(&id, Some("new-id"), Some("test reason"))
            .unwrap();
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
        assert!(path.to_string_lossy().contains(".mengdie"));
        assert!(path.to_string_lossy().ends_with("db.sqlite"));
    }

    #[test]
    fn test_content_hash_dedup() {
        let db = test_db();
        let content = "Use JWT tokens with Redis session store";

        // Insert first memory
        let id_a = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "file-a.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Auth decision v1".to_string(),
                content: content.to_string(),
                entities: "auth".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        // Insert same content with different source_file → should upsert (return same id)
        let id_b = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "file-b.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Auth decision v2".to_string(),
                content: content.to_string(),
                entities: "auth,jwt".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        assert_eq!(id_a, id_b, "same content should upsert, returning same ID");

        // Verify the entry was updated (new title, entities)
        let entry = db.get_memory(&id_a).unwrap().unwrap();
        assert_eq!(entry.title, "Auth decision v2");
        assert_eq!(entry.entities, "auth,jwt");

        // Insert different content → should create new entry
        let id_c = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "file-c.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "DB decision".to_string(),
                content: "Use PostgreSQL for persistence".to_string(),
                entities: "database".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        assert_ne!(id_a, id_c, "different content should create new entry");
        assert_eq!(db.count_memories("proj").unwrap(), 2);
    }

    #[test]
    fn test_source_file_optional() {
        let db = test_db();

        // Insert with empty source_file (simulating MCP call without source_file)
        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: String::new(),
                source_type: "conclusion".to_string(),
                knowledge_type: "factual".to_string(),
                title: "Finding A".to_string(),
                content: "Some factual finding".to_string(),
                entities: "topic".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(entry.source_file, "");
        assert_eq!(entry.title, "Finding A");

        // Insert another with empty source_file but different content → new entry
        let id2 = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: String::new(),
                source_type: "conclusion".to_string(),
                knowledge_type: "factual".to_string(),
                title: "Finding B".to_string(),
                content: "Different factual finding".to_string(),
                entities: "other".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        assert_ne!(
            id, id2,
            "different content with empty source_file should create separate entries"
        );
    }

    #[test]
    fn test_content_hash_upsert_fts5_sync() {
        let db = test_db();
        let content = "Use JWT tokens with Redis session store";

        // Insert with title "v1"
        db.insert_memory(NewMemory {
            project_id: "proj".to_string(),
            source_file: "a.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "Auth v1".to_string(),
            content: content.to_string(),
            entities: "auth".to_string(),
            embedding: None,
            embedding_dim: None,
        })
        .unwrap();

        // Upsert same content with title "v2" and different entities
        db.insert_memory(NewMemory {
            project_id: "proj".to_string(),
            source_file: "b.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "Auth v2".to_string(),
            content: content.to_string(),
            entities: "auth,jwt,redis".to_string(),
            embedding: None,
            embedding_dim: None,
        })
        .unwrap();

        // FTS should find "redis" (from updated entities) but NOT find "v1" in title
        let conn = db.conn.lock().unwrap();
        let count_redis: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_fts WHERE memory_fts MATCH 'redis'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count_redis, 1,
            "FTS should find updated entities after upsert"
        );

        // Search for old title should not match (FTS was updated)
        // Note: "v1" is too short for FTS5 default tokenizer, so search for the full title token
        let count_total: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            count_total, 1,
            "FTS should have exactly 1 entry after upsert (not 2)"
        );
    }

    #[test]
    fn test_content_hash_preserves_recall_stats() {
        let db = test_db();
        let content = "Shared decision content";

        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "original.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Decision v1".to_string(),
                content: content.to_string(),
                entities: "tag".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        // Simulate recall
        db.record_recall(&id, 0.8).unwrap();
        db.record_recall(&id, 0.6).unwrap();

        // Upsert same content → should preserve recall stats
        let id2 = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "updated.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "Decision v2".to_string(),
                content: content.to_string(),
                entities: "tag,new".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        assert_eq!(id, id2);
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(
            entry.recall_count, 2,
            "recall stats should be preserved on upsert"
        );
        assert!((entry.avg_relevance - 0.7).abs() < 0.01);
        assert_eq!(entry.title, "Decision v2", "title should be updated");
    }

    #[test]
    fn test_rename_project_basic() {
        let db = test_db();
        db.insert_memory(sample_memory("old")).unwrap();
        db.insert_memory(sample_memory("old")).unwrap();
        db.insert_memory(sample_memory("old")).unwrap();

        let (renamed, merged) = db.rename_project("old", "new").unwrap();
        assert_eq!(renamed, 3);
        assert_eq!(merged, 0);
        assert_eq!(db.count_memories("old").unwrap(), 0);
        assert_eq!(db.count_memories("new").unwrap(), 3);
    }

    #[test]
    fn test_rename_project_collision_merges() {
        let db = test_db();
        let shared_content = "Identical content for collision test";

        // Insert under old project
        db.insert_memory(NewMemory {
            project_id: "old".to_string(),
            source_file: "a.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "Old version".to_string(),
            content: shared_content.to_string(),
            entities: "test".to_string(),
            embedding: None,
            embedding_dim: None,
        })
        .unwrap();

        // Insert same content under new project (will collide)
        let new_id = db
            .insert_memory(NewMemory {
                project_id: "new".to_string(),
                source_file: "b.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: "New version".to_string(),
                content: shared_content.to_string(),
                entities: "test".to_string(),
                embedding: None,
                embedding_dim: None,
            })
            .unwrap();

        // Also insert a non-colliding memory under old
        db.insert_memory(NewMemory {
            project_id: "old".to_string(),
            source_file: "c.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "factual".to_string(),
            title: "Unique to old".to_string(),
            content: "This content has no duplicate".to_string(),
            entities: "unique".to_string(),
            embedding: None,
            embedding_dim: None,
        })
        .unwrap();

        let (renamed, merged) = db.rename_project("old", "new").unwrap();
        assert_eq!(merged, 1, "one collision should be merged");
        assert_eq!(renamed, 1, "one non-colliding row should be renamed");
        assert_eq!(
            db.count_memories("old").unwrap(),
            0,
            "no rows left under old"
        );
        assert_eq!(
            db.count_memories("new").unwrap(),
            2,
            "new has original + renamed"
        );

        // Verify the new-project version was preserved (not the old one)
        let entry = db.get_memory(&new_id).unwrap().unwrap();
        assert_eq!(
            entry.title, "New version",
            "new project's version preserved"
        );
    }

    #[test]
    fn test_rename_project_dry_run() {
        let db = test_db();
        db.insert_memory(sample_memory("old")).unwrap();
        db.insert_memory(sample_memory("old")).unwrap();

        let (would_rename, would_merge) = db.rename_project_dry_run("old", "new").unwrap();
        assert_eq!(would_rename, 2);
        assert_eq!(would_merge, 0);
        // DB unchanged
        assert_eq!(db.count_memories("old").unwrap(), 2);
    }

    #[test]
    fn test_list_projects() {
        let db = test_db();
        db.insert_memory(sample_memory("proj-a")).unwrap();
        db.insert_memory(sample_memory("proj-a")).unwrap();
        db.insert_memory(sample_memory("proj-b")).unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0], ("proj-a".to_string(), 2));
        assert_eq!(projects[1], ("proj-b".to_string(), 1));
    }
}
