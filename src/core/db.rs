use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use chrono::{DateTime, Utc};
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

/// Parse a stored RFC3339 timestamp into `DateTime<Utc>`. Returns `None`
/// when the string is malformed. Shared free function so call sites that
/// already have the string in hand (e.g., the Dreaming pass's demotion
/// loop, which fetches `last_recalled` as a `String` tuple element
/// without materialising a full `MemoryEntry`) can use the same parse
/// logic as `MemoryEntry::last_recalled_as_datetime`. Same-age-clock
/// invariant (discussion 019).
pub fn parse_last_recalled(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

impl MemoryEntry {
    /// Parse `last_recalled` (RFC3339 string, stored as TEXT in SQLite) into
    /// a `DateTime<Utc>`. Returns `None` when the column is NULL or the
    /// string is malformed. Graceful — does not panic on bad data.
    ///
    /// Shared helper: the Dreaming pass (demotion path) and search
    /// post-fetch re-rank both call this to drive decay off the same
    /// timestamp, satisfying the same-age-clock invariant from
    /// discussion 019.
    pub fn last_recalled_as_datetime(&self) -> Option<DateTime<Utc>> {
        self.last_recalled.as_deref().and_then(parse_last_recalled)
    }
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
    pub is_longterm: bool,
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
                 is_longterm, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(project_id, content_hash) WHERE source_type != 'synthesis' DO UPDATE SET
                source_file = excluded.source_file,
                source_type = excluded.source_type,
                knowledge_type = excluded.knowledge_type,
                title = excluded.title,
                entities = excluded.entities,
                embedding = excluded.embedding,
                embedding_dim = excluded.embedding_dim,
                is_longterm = excluded.is_longterm
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
                mem.is_longterm as i64,
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
                 is_longterm, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(project_id, content_hash) WHERE source_type != 'synthesis' DO UPDATE SET
                source_file = excluded.source_file,
                source_type = excluded.source_type,
                knowledge_type = excluded.knowledge_type,
                title = excluded.title,
                entities = excluded.entities,
                embedding = excluded.embedding,
                embedding_dim = excluded.embedding_dim,
                is_longterm = excluded.is_longterm
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
                mem.is_longterm as i64,
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

    /// Strict helper — write one `memory_search_audit` row plus N
    /// `audit_returned_facts` link rows in a single transaction. Errors
    /// propagate to the caller verbatim; this helper does NOT emit a
    /// `tracing::warn!` line and does NOT touch the failure counter — those
    /// are the wrapper's job (`record_search_audit_best_effort`).
    ///
    /// Empty `returned_fact_ids` is allowed (zero-result searches are still
    /// meaningful to log): the audit row writes, no link rows are inserted.
    /// `rank` on each link row is the 0-indexed position of `fact_id` in
    /// `returned_fact_ids` and is reserved for downstream consumers (no
    /// v0.0.1 query reads it).
    ///
    /// Returns the inserted `audit_id` (rowid alias on `memory_search_audit`).
    ///
    /// Plan F-002 Step 2 / discussion 029 R1.
    pub fn record_search_audit(
        &self,
        query: &str,
        scope: Option<&str>,
        took_ms: i64,
        returned_fact_ids: &[String],
    ) -> anyhow::Result<i64> {
        let searched_at = Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let tx = conn.transaction()?;

        tx.execute(
            "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![query, scope, took_ms, searched_at],
        )?;
        let audit_id = tx.last_insert_rowid();

        for (rank, fact_id) in returned_fact_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO audit_returned_facts (audit_id, fact_id, rank)
                 VALUES (?1, ?2, ?3)",
                params![audit_id, fact_id, rank as i64],
            )?;
        }

        tx.commit()?;
        Ok(audit_id)
    }

    /// Best-effort wrapper around `record_search_audit`. Swallows errors
    /// with a `tracing::warn!` line carrying the query text + a pre-call
    /// timestamp, and bumps `METRIC_AUDIT_WRITE_FAILURES`. Returns `()`.
    ///
    /// Both Step 3 call sites (`mcp_tools::search` + `cli::cmd_search`)
    /// invoke this wrapper, NOT the strict helper directly — audit-write
    /// failures must NOT mutate the search response or propagate as
    /// MCP/CLI errors.
    ///
    /// Plan F-002 Step 2 / discussion 029 Topic 2.
    pub fn record_search_audit_best_effort(
        &self,
        query: &str,
        scope: Option<&str>,
        took_ms: i64,
        returned_fact_ids: &[String],
    ) {
        // Capture timestamp BEFORE the strict-helper call. Under lock
        // contention the Err return path can be arbitrarily later than the
        // wrapper entry; the warn line's `searched_at` must localize the
        // actual search time so post-restart audit-gap recovery (the F1
        // stderr surface) bounds the missing-row window. Plan F-002
        // Doodlestein-adversarial M6.
        let warn_searched_at = Utc::now().to_rfc3339();
        if let Err(e) = self.record_search_audit(query, scope, took_ms, returned_fact_ids) {
            tracing::warn!(
                query = %query,
                searched_at = %warn_searched_at,
                took_ms = took_ms,
                error = %e,
                "audit write failed"
            );
            let _ = self.increment_metric(super::metrics::METRIC_AUDIT_WRITE_FAILURES);
        }
    }

    /// Bulk fetch memory rows by id, one lock acquisition.
    /// Returns rows in input-id order; missing ids are silently skipped
    /// (caller handles the len-mismatch policy).
    pub fn get_memories_by_ids(&self, ids: &[String]) -> anyhow::Result<Vec<MemoryEntry>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

        let placeholders = (1..=ids.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, project_id, source_file, source_type, knowledge_type, \
                    title, content, entities, valid_from, valid_until, \
                    superseded_by, recall_count, avg_relevance, last_recalled, \
                    embedding, embedding_dim, is_longterm, created_at \
             FROM memory_entries WHERE id IN ({placeholders})"
        );

        let params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(id.clone()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let mut by_id = std::collections::HashMap::<String, MemoryEntry>::new();
        let rows = stmt.query_map(param_refs.as_slice(), row_to_entry)?;
        for row in rows {
            let entry = row?;
            by_id.insert(entry.id.clone(), entry);
        }

        // Review feedback: when the fetched count is < requested count, log
        // the missing ids at warn level. Root cause is usually that a memory
        // was invalidated or deleted between cluster_memories and the bulk
        // fetch. Surfacing the ids lets the operator diagnose without
        // re-running with debug logging.
        if by_id.len() < ids.len() {
            let missing: Vec<&String> = ids.iter().filter(|id| !by_id.contains_key(*id)).collect();
            tracing::warn!(
                requested = ids.len(),
                loaded = by_id.len(),
                ?missing,
                "get_memories_by_ids: partial load"
            );
        }

        Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
    }

    /// Insert a synthesis memory AND its source→synthesis link rows in a single
    /// SQLite transaction.
    ///
    /// Dedup key (plan 017): `synthesis_cluster_hash` — derived from the
    /// sorted+deduped `source_ids` via `compute_synthesis_cluster_hash`.
    /// Re-synthesis of the same cluster (e.g. after a `SYSTEM_PROMPT` edit)
    /// produces the SAME key and UPSERTS the existing row, preventing zombie
    /// siblings. Previously this used `content_hash`, which would silently
    /// create a new row whenever the LLM output text changed.
    ///
    /// **`source_type` immutability**: this function writes
    /// `source_type = 'synthesis'`. Downstream code must NOT reclassify
    /// synthesis rows — the partial unique indexes `idx_synthesis_cluster`
    /// (`WHERE source_type = 'synthesis'`) and `idx_memory_content_hash`
    /// (`WHERE source_type != 'synthesis'`) both depend on this invariant.
    /// The source_type trigger (plan 017) limits values to the allowlist
    /// but does not prevent a synthesis row from being UPDATED to another
    /// allowed value — that stays a convention.
    ///
    /// Link rows use INSERT OR IGNORE on the composite PK so repeat sources
    /// collapse silently.
    pub fn insert_synthesis_with_links(
        &self,
        mem: NewMemory,
        source_ids: &[String],
    ) -> anyhow::Result<String> {
        anyhow::ensure!(
            !source_ids.is_empty(),
            "insert_synthesis_with_links: source_ids must be non-empty (caller must provide at least one source memory)"
        );

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let content_hash = super::schema::compute_content_hash(&mem.content);
        let cluster_hash = super::schema::compute_synthesis_cluster_hash(source_ids);
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let tx = conn.transaction()?;

        let returned_id: String = tx.query_row(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, embedding, embedding_dim,
                 is_longterm, created_at, content_hash, synthesis_cluster_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(project_id, synthesis_cluster_hash)
                WHERE source_type = 'synthesis' AND synthesis_cluster_hash IS NOT NULL
                DO UPDATE SET
                    source_file = excluded.source_file,
                    source_type = excluded.source_type,
                    knowledge_type = excluded.knowledge_type,
                    title = excluded.title,
                    content = excluded.content,
                    entities = excluded.entities,
                    embedding = excluded.embedding,
                    embedding_dim = excluded.embedding_dim,
                    is_longterm = excluded.is_longterm,
                    content_hash = excluded.content_hash
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
                mem.is_longterm as i64,
                now,
                content_hash,
                cluster_hash,
            ],
            |row| row.get(0),
        )?;

        for src in source_ids {
            tx.execute(
                "INSERT OR IGNORE INTO memory_synthesis_links \
                     (source_memory_id, synthesis_memory_id, created_at) \
                 VALUES (?1, ?2, ?3)",
                params![src, returned_id, now],
            )?;
        }

        tx.commit()?;
        Ok(returned_id)
    }

    /// Fetch a synthesis memory + its linked source memories for audit (plan 017 Step 3).
    ///
    /// Returns `(synthesis, sources)` where `sources` follows the link-table
    /// order (no specific ordering guarantee beyond that — sources are an
    /// unordered set of inputs). If a linked source row has been hard-deleted
    /// (shouldn't happen under current invariants; FKs are declared but not
    /// enforced), returns a placeholder `MemoryEntry` with title
    /// `"<deleted: {id}>"` rather than aborting — graceful degradation for
    /// the audit use case.
    ///
    /// Errors:
    /// - `id` not found in memory_entries.
    /// - row exists but `source_type != "synthesis"` (wrong row type).
    pub fn get_synthesis_with_sources(
        &self,
        id: &str,
    ) -> anyhow::Result<(MemoryEntry, Vec<MemoryEntry>)> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

        let synthesis: MemoryEntry = conn
            .query_row(
                "SELECT id, project_id, source_file, source_type, knowledge_type,
                        title, content, entities, valid_from, valid_until,
                        superseded_by, recall_count, avg_relevance, last_recalled,
                        embedding, embedding_dim, is_longterm, created_at
                 FROM memory_entries WHERE id = ?1",
                params![id],
                row_to_entry,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    anyhow::anyhow!("synthesis id not found: {id}")
                }
                other => anyhow::anyhow!("query synthesis {id}: {other}"),
            })?;

        if synthesis.source_type != "synthesis" {
            anyhow::bail!(
                "id {id} is not a synthesis row (source_type = '{}')",
                synthesis.source_type
            );
        }

        // Fetch the source IDs from the link table.
        let mut stmt = conn.prepare(
            "SELECT source_memory_id FROM memory_synthesis_links
             WHERE synthesis_memory_id = ?1",
        )?;
        let source_ids: Vec<String> = stmt
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);

        // Fetch each source memory. Missing rows become placeholders.
        let mut sources: Vec<MemoryEntry> = Vec::with_capacity(source_ids.len());
        for src_id in &source_ids {
            let fetched: Option<MemoryEntry> = conn
                .query_row(
                    "SELECT id, project_id, source_file, source_type, knowledge_type,
                            title, content, entities, valid_from, valid_until,
                            superseded_by, recall_count, avg_relevance, last_recalled,
                            embedding, embedding_dim, is_longterm, created_at
                     FROM memory_entries WHERE id = ?1",
                    params![src_id],
                    row_to_entry,
                )
                .optional()?;
            match fetched {
                Some(entry) => sources.push(entry),
                None => sources.push(MemoryEntry {
                    id: src_id.clone(),
                    project_id: String::new(),
                    source_file: String::new(),
                    source_type: "<deleted>".to_string(),
                    knowledge_type: String::new(),
                    title: format!("<deleted: {src_id}>"),
                    content: String::new(),
                    entities: String::new(),
                    valid_from: String::new(),
                    valid_until: None,
                    superseded_by: None,
                    recall_count: 0,
                    avg_relevance: 0.0,
                    last_recalled: None,
                    embedding: None,
                    embedding_dim: None,
                    is_longterm: false,
                    created_at: String::new(),
                }),
            }
        }

        Ok((synthesis, sources))
    }

    /// Count synthesis link rows pointing at a given synthesis memory.
    /// Used by unit + external integration tests; kept `pub` because
    /// `tests/dream_synthesis.rs` is an external crate and can't reach
    /// `pub(crate)` items. NOT a production read path — `run_synthesis_pass`
    /// tracks counts via `SynthesisResult.syntheses_created`. Architecture
    /// review flagged the visibility; the trade-off is: test-harness access
    /// vs surface area. We pick test-harness.
    pub fn count_synthesis_links(&self, synthesis_memory_id: &str) -> anyhow::Result<i64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_synthesis_links WHERE synthesis_memory_id = ?1",
            params![synthesis_memory_id],
            |r| r.get(0),
        )?;
        Ok(count)
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
            is_longterm: false,
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
                is_longterm: false,
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
                is_longterm: false,
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
                is_longterm: false,
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
                is_longterm: false,
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
                is_longterm: false,
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
            is_longterm: false,
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
            is_longterm: false,
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
                is_longterm: false,
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
                is_longterm: false,
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
            is_longterm: false,
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
                is_longterm: false,
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
            is_longterm: false,
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

    #[test]
    fn test_insert_memory_is_longterm_true() {
        let db = test_db();
        let id = db
            .insert_memory(NewMemory {
                project_id: "proj".to_string(),
                source_file: "lt.md".to_string(),
                source_type: "synthesis".to_string(),
                knowledge_type: "factual".to_string(),
                title: "Long-term memory".to_string(),
                content: "seeded longterm content".to_string(),
                entities: "long,term".to_string(),
                embedding: None,
                embedding_dim: None,
                is_longterm: true,
            })
            .unwrap();

        // Round-trip: get_memory should report is_longterm = true.
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(entry.is_longterm);

        // Raw column is stored as 1.
        let conn = db.conn.lock().unwrap();
        let raw: i64 = conn
            .query_row(
                "SELECT is_longterm FROM memory_entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw, 1);
    }

    #[test]
    fn test_insert_memory_is_longterm_default_false() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("proj")).unwrap();
        let entry = db.get_memory(&id).unwrap().unwrap();
        assert!(!entry.is_longterm);

        let conn = db.conn.lock().unwrap();
        let raw: i64 = conn
            .query_row(
                "SELECT is_longterm FROM memory_entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw, 0);
    }

    // ----- last_recalled_as_datetime helper (BL-008 Step 1) -----

    fn entry_with_last_recalled(last: Option<String>) -> MemoryEntry {
        MemoryEntry {
            id: "id".to_string(),
            project_id: "p".to_string(),
            source_file: String::new(),
            source_type: String::new(),
            knowledge_type: String::new(),
            title: String::new(),
            content: String::new(),
            entities: String::new(),
            valid_from: String::new(),
            valid_until: None,
            superseded_by: None,
            recall_count: 0,
            avg_relevance: 0.0,
            last_recalled: last,
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
            created_at: String::new(),
        }
    }

    #[test]
    fn last_recalled_as_datetime_valid_rfc3339() {
        let e = entry_with_last_recalled(Some("2026-04-20T12:00:00Z".to_string()));
        let dt = e.last_recalled_as_datetime().expect("should parse");
        assert_eq!(dt.to_rfc3339(), "2026-04-20T12:00:00+00:00");
    }

    #[test]
    fn last_recalled_as_datetime_none_when_null() {
        let e = entry_with_last_recalled(None);
        assert!(e.last_recalled_as_datetime().is_none());
    }

    #[test]
    fn last_recalled_as_datetime_none_when_malformed() {
        let e = entry_with_last_recalled(Some("not-a-date".to_string()));
        assert!(e.last_recalled_as_datetime().is_none());
        let e = entry_with_last_recalled(Some(String::new()));
        assert!(e.last_recalled_as_datetime().is_none());
    }

    // ----- Plan 017: cluster-hash dedup semantic tests ---------------------

    /// Seed 3 short-term memories in the test DB, return their IDs.
    fn seed_three_sources(db: &Db) -> Vec<String> {
        (0..3)
            .map(|i| {
                db.insert_memory(NewMemory {
                    project_id: "proj".to_string(),
                    source_file: format!("src-{i}.md"),
                    source_type: "conclusion".to_string(),
                    knowledge_type: "decisional".to_string(),
                    title: format!("source {i}"),
                    content: format!("source content {i}"),
                    entities: "tag".to_string(),
                    embedding: None,
                    embedding_dim: None,
                    is_longterm: false,
                })
                .unwrap()
            })
            .collect()
    }

    fn synthesis_mem(content: &str) -> NewMemory {
        NewMemory {
            project_id: "proj".to_string(),
            source_file: "syn.md".to_string(),
            source_type: "synthesis".to_string(),
            knowledge_type: "factual".to_string(),
            title: "synthesis".to_string(),
            content: content.to_string(),
            entities: "tag".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        }
    }

    #[test]
    fn insert_synthesis_with_links_upserts_on_same_cluster() {
        let db = test_db();
        let sources = seed_three_sources(&db);

        let id1 = db
            .insert_synthesis_with_links(synthesis_mem("V1 content"), &sources)
            .unwrap();
        let id2 = db
            .insert_synthesis_with_links(synthesis_mem("V2 content"), &sources)
            .unwrap();

        // Invariant: exactly one synthesis row per cluster per project (plan 017 AC3).
        // This is the COUNT-based invariant, not id-equality (which is only
        // incidentally true for ON CONFLICT DO UPDATE RETURNING id).
        let conn = db.lock_conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis' AND project_id = 'proj'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // Latest content wins.
        let content: String = conn
            .query_row(
                "SELECT content FROM memory_entries WHERE id = ?1",
                params![id2],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(content, "V2 content");

        // ON CONFLICT DO UPDATE RETURNING id returns the existing row's id
        // (implementation detail, but nice to assert for documentation).
        assert_eq!(id1, id2);
    }

    #[test]
    fn insert_synthesis_with_links_different_clusters_coexist() {
        let db = test_db();
        let sources = seed_three_sources(&db);

        let subset_a = vec![sources[0].clone(), sources[1].clone()];
        let subset_b = vec![sources[1].clone(), sources[2].clone()];

        let id_a = db
            .insert_synthesis_with_links(synthesis_mem("Different content A"), &subset_a)
            .unwrap();
        let id_b = db
            .insert_synthesis_with_links(synthesis_mem("Different content B"), &subset_b)
            .unwrap();

        assert_ne!(id_a, id_b, "different clusters must get distinct row ids");

        let conn = db.lock_conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis' AND project_id = 'proj'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "two distinct clusters should yield two rows");
    }

    #[test]
    fn insert_synthesis_with_links_source_id_order_independent() {
        let db = test_db();
        let sources = seed_three_sources(&db);

        let id1 = db
            .insert_synthesis_with_links(
                synthesis_mem("c"),
                &[sources[0].clone(), sources[1].clone(), sources[2].clone()],
            )
            .unwrap();
        let id2 = db
            .insert_synthesis_with_links(
                synthesis_mem("c"),
                &[sources[2].clone(), sources[0].clone(), sources[1].clone()],
            )
            .unwrap();

        assert_eq!(
            id1, id2,
            "same source set in different order must dedup to same row"
        );
    }

    #[test]
    fn insert_synthesis_with_links_rejects_empty_source_ids() {
        let db = test_db();
        let result = db.insert_synthesis_with_links(synthesis_mem("c"), &[]);
        assert!(
            result.is_err(),
            "empty source_ids should return an error (caller bug)"
        );
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("non-empty"),
            "error should explain the invariant, got: {msg}"
        );
    }
}
