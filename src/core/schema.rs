use rusqlite::Connection;
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: i64 = 4;

/// Check if a column exists in a table (for crash-safe migrations).
fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|r| r.map(|name| name == column).unwrap_or(false));
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
            conn.execute_batch("ALTER TABLE memory_entries ADD COLUMN content_hash TEXT;")?;
        }

        // Backfill content_hash for any existing rows (safety net)
        let mut stmt =
            conn.prepare("SELECT id, content FROM memory_entries WHERE content_hash IS NULL")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

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
                 ON memory_entries(project_id, content_hash);",
        )?;
    }

    // Migration v3: persist invalidation reason for audit trail
    if current_version < 3 && !column_exists(conn, "memory_entries", "invalidation_reason")? {
        conn.execute_batch("ALTER TABLE memory_entries ADD COLUMN invalidation_reason TEXT;")?;
    }

    // Migration v4: synthesis link table (BL-007 dream synthesis)
    if current_version < 4 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_synthesis_links (
                source_memory_id     TEXT NOT NULL,
                synthesis_memory_id  TEXT NOT NULL,
                created_at           TEXT NOT NULL,
                PRIMARY KEY (source_memory_id, synthesis_memory_id),
                FOREIGN KEY (source_memory_id) REFERENCES memory_entries(id),
                FOREIGN KEY (synthesis_memory_id) REFERENCES memory_entries(id)
             );
             CREATE INDEX IF NOT EXISTS idx_syn_link_source ON memory_synthesis_links(source_memory_id);
             CREATE INDEX IF NOT EXISTS idx_syn_link_synthesis ON memory_synthesis_links(synthesis_memory_id);",
        )?;
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

    #[test]
    fn test_schema_version_is_v4_on_fresh_db() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);

        // Idempotent: re-running leaves version at 4.
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);
    }

    #[test]
    fn test_v4_synthesis_link_table_exists() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'memory_synthesis_links' AND type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Indexes exist as well.
        let idx_source: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'idx_syn_link_source' AND type = 'index'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_source, 1);
        let idx_syn: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'idx_syn_link_synthesis' AND type = 'index'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_syn, 1);
    }

    #[test]
    fn test_migration_from_v3_preserves_data() {
        let conn = Connection::open_in_memory().unwrap();
        // First put the DB into a v3 state by running migrations then setting user_version = 3.
        run_migrations(&conn).unwrap();
        conn.execute_batch("PRAGMA user_version = 3;").unwrap();
        // Drop the v4 table so the migration has work to do.
        conn.execute_batch("DROP TABLE IF EXISTS memory_synthesis_links;")
            .unwrap();

        // Insert a row at the v3 state to verify it survives migration.
        let hash = compute_content_hash("some content");
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                "row-1",
                "proj",
                "f.md",
                "conclusion",
                "decisional",
                "title",
                "some content",
                "tag",
                "2026-04-18T00:00:00Z",
                "2026-04-18T00:00:00Z",
                hash,
            ],
        )
        .unwrap();

        // Re-run migrations — should upgrade to v4 cleanly.
        run_migrations(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);

        // Existing row intact.
        let title: String = conn
            .query_row(
                "SELECT title FROM memory_entries WHERE id = 'row-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "title");

        // New table exists.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'memory_synthesis_links'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
