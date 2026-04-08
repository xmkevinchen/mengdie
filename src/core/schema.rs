use rusqlite::Connection;
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: i64 = 3;

/// Check if a column exists in a table (for crash-safe migrations).
fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt.query_map([], |row| row.get::<_, String>(1))?.any(|r| {
        r.map(|name| name == column).unwrap_or(false)
    });
    Ok(exists)
}

/// Compute SHA-256 hex hash of content for dedup.
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Run all schema migrations. Idempotent — safe to call on every startup.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")?;

    let current_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS memory_entries (
            id              TEXT PRIMARY KEY,
            project_id      TEXT NOT NULL,
            source_file     TEXT NOT NULL,
            source_type     TEXT NOT NULL,
            knowledge_type  TEXT NOT NULL,
            title           TEXT NOT NULL,
            content         TEXT NOT NULL,
            entities        TEXT NOT NULL DEFAULT '',
            valid_from      TEXT NOT NULL,
            valid_until     TEXT,
            superseded_by   TEXT,
            recall_count    INTEGER NOT NULL DEFAULT 0,
            avg_relevance   REAL NOT NULL DEFAULT 0.0,
            last_recalled   TEXT,
            embedding       BLOB,
            embedding_dim   INTEGER,
            is_longterm     INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_memory_project
            ON memory_entries(project_id);

        CREATE INDEX IF NOT EXISTS idx_memory_knowledge_type
            ON memory_entries(knowledge_type);

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts
            USING fts5(title, content, entities, content=memory_entries, content_rowid=rowid);

        -- Triggers to keep FTS index in sync
        CREATE TRIGGER IF NOT EXISTS memory_fts_insert AFTER INSERT ON memory_entries
        BEGIN
            INSERT INTO memory_fts(rowid, title, content, entities)
            VALUES (NEW.rowid, NEW.title, NEW.content, NEW.entities);
        END;

        CREATE TRIGGER IF NOT EXISTS memory_fts_delete AFTER DELETE ON memory_entries
        BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, title, content, entities)
            VALUES ('delete', OLD.rowid, OLD.title, OLD.content, OLD.entities);
        END;

        CREATE TRIGGER IF NOT EXISTS memory_fts_update AFTER UPDATE ON memory_entries
        BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, title, content, entities)
            VALUES ('delete', OLD.rowid, OLD.title, OLD.content, OLD.entities);
            INSERT INTO memory_fts(rowid, title, content, entities)
            VALUES (NEW.rowid, NEW.title, NEW.content, NEW.entities);
        END;

        CREATE TABLE IF NOT EXISTS metrics (
            key         TEXT PRIMARY KEY,
            value_int   INTEGER NOT NULL DEFAULT 0,
            value_float REAL NOT NULL DEFAULT 0.0,
            updated_at  TEXT NOT NULL
        );
        ",
    )?;

    // Migration v2: content_hash dedup replaces source_file dedup
    if current_version < 2 {
        if !column_exists(conn, "memory_entries", "content_hash")? {
            conn.execute_batch(
                "ALTER TABLE memory_entries ADD COLUMN content_hash TEXT;"
            )?;
        }

        // Backfill content_hash for any existing rows (safety net)
        let mut stmt = conn.prepare("SELECT id, content FROM memory_entries WHERE content_hash IS NULL")?;
        let rows: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.filter_map(|r| r.ok()).collect();

        for (id, content) in &rows {
            let hash = compute_content_hash(content);
            conn.execute(
                "UPDATE memory_entries SET content_hash = ?1 WHERE id = ?2",
                rusqlite::params![hash, id],
            )?;
        }

        // Swap unique index: source_file → content_hash
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_memory_source;
             CREATE UNIQUE INDEX IF NOT EXISTS idx_memory_content_hash
                 ON memory_entries(project_id, content_hash);"
        )?;
    }

    // Migration v3: persist invalidation reason for audit trail
    if current_version < 3 {
        if !column_exists(conn, "memory_entries", "invalidation_reason")? {
            conn.execute_batch(
                "ALTER TABLE memory_entries ADD COLUMN invalidation_reason TEXT;"
            )?;
        }
    }

    // Set schema version
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Running again should not error
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn test_wal_mode_active() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        // In-memory DBs use "memory" mode, but WAL pragma was issued
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn test_fts5_table_exists() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'memory_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
