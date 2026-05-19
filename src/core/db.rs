use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{ffi::sqlite3_auto_extension, params, Connection, OptionalExtension};
use uuid::Uuid;

use super::schema::{compute_content_hash, run_migrations};

/// Register sqlite-vec extension for all subsequent SQLite connections in
/// this process. `sqlite3_auto_extension` is process-global; calling it
/// twice is harmless but the OnceLock guards against unnecessary duplicate
/// registration.
///
/// **Critical ordering**: `sqlite3_auto_extension` only injects vec0 into
/// connections opened AFTER registration. Conns opened before are missing
/// vec0 — they would error on `CREATE VIRTUAL TABLE … USING vec0(…)`.
/// Always call this function BEFORE `Connection::open*()` for any conn
/// that will run schema migrations or vec0 queries.
///
/// Public to crate so test code in `schema.rs` and elsewhere that opens
/// raw `rusqlite::Connection` (bypassing `Db::open*`) can register the
/// extension before opening. BL-026 Step 2 — adopted post PASS_STATIC
/// spike outcome (`docs/spikes/sqlite-vec-distribution.md`, 2026-05-08).
pub(crate) fn ensure_sqlite_vec_registered() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        // SAFETY: sqlite3_vec_init has the canonical sqlite3_extension entry
        // shape; this transmute matches the pattern in sqlite-vec's own
        // doc-test (`sqlite-vec-0.1.9/src/lib.rs`). The full target signature
        // is the standard sqlite3 extension entry-point per sqlite3ext.h.
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                unsafe extern "C" fn(
                    *mut rusqlite::ffi::sqlite3,
                    *mut *mut i8,
                    *const rusqlite::ffi::sqlite3_api_routines,
                ) -> i32,
            >(
                sqlite_vec::sqlite3_vec_init as *const ()
            )));
        }
    });
}

/// Basic database statistics.
pub struct DbStats {
    pub total: i64,
    pub valid: i64,
    pub longterm: i64,
    pub recalled: i64,
}

/// Status breakdown returned by `Db::status_breakdown()` — F-011 input
/// to the `memory_status` MCP tool. Scoped to a project_id (or global
/// when None at call time). Companion to `DbStats` (which is global +
/// computes total/valid/longterm/recalled but is not project-scoped).
#[derive(Debug, Clone)]
pub struct StatusBreakdown {
    /// Total entries in scope.
    pub total: i64,
    /// Entries with `is_longterm = 1` in scope.
    pub longterm_count: i64,
    /// Entries with `source_type = 'synthesis'` in scope (also present
    /// in `by_source_type`; pulled out for direct access since it's the
    /// primary indicator of "dreaming has run").
    pub synthesis_count: i64,
    /// Per-source_type entry counts in scope.
    pub by_source_type: std::collections::BTreeMap<String, i64>,
    /// `MAX(created_at)` of entries in scope; `None` on empty table.
    pub last_ingest_at: Option<String>,
}

/// Audit-pipeline health snapshot returned by `Db::audit_stats()`.
///
/// The six fields are consumed by the `mengdie audit-stats` CLI subcommand
/// (F-005) so an operator can detect silent breakage of the F-002 audit hook
/// without waiting for the A-MEM `≥5/30d` supersession trigger.
pub struct AuditStats {
    /// `COUNT(*)` of rows in `memory_search_audit`.
    pub audit_count: i64,
    /// `COUNT(*)` of rows in `audit_returned_facts`.
    pub link_count: i64,
    /// `MIN(searched_at)` of `memory_search_audit`. `None` on an empty table.
    pub oldest_row: Option<String>,
    /// `MAX(searched_at)` of `memory_search_audit`. `None` on an empty table.
    pub newest_row: Option<String>,
    /// Count of supersession events in the last 30 days; the bare-`COUNT(*)`
    /// sister of `F002_SUPERSESSION_SQL` (no GROUP BY / HAVING).
    pub supersession_count_30d: i64,
    /// Persistent counter from the `metrics` table
    /// (`METRIC_AUDIT_WRITE_FAILURES`); not session-local.
    pub audit_write_failures: i64,
}

/// The verbatim F-002 supersession SQL — promoted from a `#[cfg(test)] const`
/// to a crate-internal item, currently dead in production but reachable by
/// AC4's `test_audit_stats_where_clause_shared` invariant. Returns one row
/// per `(window_start, supersession_count)` pair where ≥5 superseded facts
/// were returned by audited searches in the last 30 days.
///
/// **Honest rationale** (F-005 challenger #2): the `pub(crate)` promotion
/// originally intended a future production caller (an A-MEM bucketed-trigger
/// path) that did not materialize in F-005. Production audit-stats consumes
/// only the bare-COUNT sister `AUDIT_STATS_SUPERSESSION_COUNT_SQL`. The
/// promotion + `#[allow(dead_code)]` is therefore tracking-debt: a real
/// caller is owed by the next feature that needs the bucketed form (BL-031
/// captures the cross-layer test gap that would benefit from it). Until
/// then, `#[allow(dead_code)]` silences a legitimate compiler signal so
/// the const can stay reachable by the AC4 grep-invariant test.
#[allow(dead_code)]
pub(crate) const F002_SUPERSESSION_SQL: &str = "
    SELECT
        DATE(a.searched_at, 'start of day', '-30 days') AS window_start,
        COUNT(*) AS supersession_count
    FROM memory_search_audit a
    JOIN audit_returned_facts arf ON arf.audit_id = a.id
    JOIN memory_entries me ON me.id = arf.fact_id
    WHERE me.valid_until IS NOT NULL
      AND JULIANDAY(me.valid_until) - JULIANDAY(a.searched_at) <= 7
      AND a.searched_at >= DATE('now', '-30 days')
    GROUP BY window_start
    HAVING supersession_count >= 5;
";

/// Sister query of `F002_SUPERSESSION_SQL` — same WHERE clause, but drops the
/// GROUP BY / HAVING aggregation and returns a single `COUNT(*)` of all
/// supersession events in the trailing 30-day window. Used by
/// `Db::audit_stats()` to expose the raw event count (the F-002 query
/// surfaces only buckets that already crossed the ≥5 trigger threshold,
/// which is the wrong shape for a health-check display).
pub(crate) const AUDIT_STATS_SUPERSESSION_COUNT_SQL: &str = "
    SELECT COUNT(*)
    FROM memory_search_audit a
    JOIN audit_returned_facts arf ON arf.audit_id = a.id
    JOIN memory_entries me ON me.id = arf.fact_id
    WHERE me.valid_until IS NOT NULL
      AND JULIANDAY(me.valid_until) - JULIANDAY(a.searched_at) <= 7
      AND a.searched_at >= DATE('now', '-30 days');
";

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
        ensure_sqlite_vec_registered();
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
        ensure_sqlite_vec_registered();
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

    /// Test helper: insert a memory with a caller-provided ID instead of
    /// a freshly-generated UUID v4. Useful for tests that need to construct
    /// scenarios where two facts share a known ID prefix (e.g., F-013
    /// collision-path coverage for memory_invalidate / memory_get).
    ///
    /// Exposed as `pub` (not `#[cfg(test)]`) because integration tests
    /// under `tests/` cannot reach `#[cfg(test)]` items in `src/`. Marked
    /// `#[doc(hidden)]` to discourage production use — `insert_memory`
    /// is the canonical ingest path.
    #[doc(hidden)]
    pub fn insert_memory_with_id(&self, id: &str, mem: NewMemory) -> anyhow::Result<String> {
        self.insert_memory_inner(id.to_string(), mem)
    }

    /// Insert or update a memory, returning its ID.
    /// On conflict (same project_id + content_hash), updates metadata but preserves
    /// recall stats, timestamps, and ID. Atomic via ON CONFLICT DO UPDATE.
    pub fn insert_memory(&self, mem: NewMemory) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        self.insert_memory_inner(id, mem)
    }

    /// Shared body for `insert_memory` and `insert_memory_with_id`.
    /// Holds the SQL + parameter binding; the only diff between the two
    /// public entry points is whether the ID is caller-provided or freshly
    /// UUID v4-generated.
    fn insert_memory_inner(&self, id: String, mem: NewMemory) -> anyhow::Result<String> {
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

    /// Resolve a UUID prefix to matching memory_entries IDs.
    ///
    /// Returns at most 2 matches — callers only need to distinguish
    /// 0 / 1 / ≥2 (collision detection); full enumeration of N collisions
    /// is not useful. Uses `WHERE id LIKE 'prefix%'`, which is sargable
    /// against the `memory_entries(id)` primary-key index.
    ///
    /// Caller must pass a prefix of at least 8 hex chars (UUID v4 has
    /// ~1 in 4 billion collision probability at 8 chars for a personal
    /// corpus). This is enforced at MCP/CLI boundary, not in db.rs.
    pub fn find_by_id_prefix(
        &self,
        prefix: &str,
        project_id: Option<&str>,
    ) -> anyhow::Result<Vec<String>> {
        let pattern = format!("{prefix}%");
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let rows: Vec<String> = match project_id {
            Some(pid) => {
                let mut stmt = conn.prepare(
                    "SELECT id FROM memory_entries
                     WHERE id LIKE ?1 AND project_id = ?2
                     LIMIT 2",
                )?;
                let mapped =
                    stmt.query_map(params![pattern, pid], |row| row.get::<_, String>(0))?;
                mapped.collect::<rusqlite::Result<Vec<_>>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id FROM memory_entries
                     WHERE id LIKE ?1 LIMIT 2",
                )?;
                let mapped = stmt.query_map(params![pattern], |row| row.get::<_, String>(0))?;
                mapped.collect::<rusqlite::Result<Vec<_>>>()?
            }
        };
        Ok(rows)
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

    /// Bump recall_count + last_recalled without touching avg_relevance.
    ///
    /// Used by `memory_get` (F-010): a direct fact lookup is a "consult"
    /// signal but has no meaningful relevance score (caller explicitly
    /// chose this fact, not the ranker). Mixing 0.0 or 1.0 into the EMA
    /// would corrupt the running average — recall_count alone is the
    /// right channel for "this fact was consulted".
    ///
    /// Returns false if ID not found.
    pub fn bump_recall_only(&self, id: &str) -> anyhow::Result<bool> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let rows = conn.execute(
            "UPDATE memory_entries SET
                recall_count = recall_count + 1,
                last_recalled = ?1
             WHERE id = ?2",
            params![now, id],
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

    /// Status breakdown for the `memory_status` MCP tool (F-011): aggregate
    /// counts + last-ingest timestamp scoped to a given project_id (or
    /// global when `project_id` is `None`). Single connection lock so the
    /// snapshot is consistent. Pairs with `audit_stats()` + `list_metrics()`
    /// to compose the full status response in `mcp_tools::status`.
    pub fn status_breakdown(&self, project_id: Option<&str>) -> anyhow::Result<StatusBreakdown> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

        // total entries (scoped or global)
        let total: i64 = match project_id {
            Some(pid) => conn.query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE project_id = ?1",
                params![pid],
                |r| r.get(0),
            )?,
            None => conn.query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))?,
        };

        let longterm_count: i64 = match project_id {
            Some(pid) => conn.query_row(
                "SELECT COUNT(*) FROM memory_entries
                 WHERE is_longterm = 1 AND project_id = ?1",
                params![pid],
                |r| r.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM memory_entries WHERE is_longterm = 1",
                [],
                |r| r.get(0),
            )?,
        };

        // by_source_type breakdown — fixed enum set, so an empty count is
        // genuine information (e.g., synthesis_count = 0 means no dreaming
        // synthesis run yet for this project).
        let mut by_source_type: std::collections::BTreeMap<String, i64> =
            std::collections::BTreeMap::new();
        {
            let sql = match project_id {
                Some(_) => {
                    "SELECT source_type, COUNT(*) FROM memory_entries
                            WHERE project_id = ?1 GROUP BY source_type"
                }
                None => {
                    "SELECT source_type, COUNT(*) FROM memory_entries
                         GROUP BY source_type"
                }
            };
            let mut stmt = conn.prepare(sql)?;
            let rows = match project_id {
                Some(pid) => stmt
                    .query_map(params![pid], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
                None => stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            };
            for (st, cnt) in rows {
                by_source_type.insert(st, cnt);
            }
        }

        let synthesis_count: i64 = by_source_type.get("synthesis").copied().unwrap_or(0);

        let last_ingest_at: Option<String> = match project_id {
            Some(pid) => conn.query_row(
                "SELECT MAX(created_at) FROM memory_entries WHERE project_id = ?1",
                params![pid],
                |r| r.get(0),
            )?,
            None => conn.query_row("SELECT MAX(created_at) FROM memory_entries", [], |r| {
                r.get(0)
            })?,
        };

        Ok(StatusBreakdown {
            total,
            longterm_count,
            synthesis_count,
            by_source_type,
            last_ingest_at,
        })
    }

    /// Audit-pipeline health snapshot — six numbers an operator can read at
    /// a glance to detect silent breakage of the F-002 audit hook (F-005).
    ///
    /// All six queries run under a single `self.conn.lock()` guard so the
    /// snapshot is a consistent view of the audit substrate; matches the
    /// discipline of `Db::stats()`.
    pub fn audit_stats(&self) -> anyhow::Result<AuditStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let audit_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM memory_search_audit", [], |r| r.get(0))?;
        let link_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM audit_returned_facts", [], |r| {
                r.get(0)
            })?;
        let oldest_row: Option<String> = conn.query_row(
            "SELECT MIN(searched_at) FROM memory_search_audit",
            [],
            |r| r.get(0),
        )?;
        let newest_row: Option<String> = conn.query_row(
            "SELECT MAX(searched_at) FROM memory_search_audit",
            [],
            |r| r.get(0),
        )?;
        let supersession_count_30d: i64 =
            conn.query_row(AUDIT_STATS_SUPERSESSION_COUNT_SQL, [], |r| r.get(0))?;
        let audit_write_failures: i64 = conn
            .query_row(
                "SELECT value_int FROM metrics WHERE key = ?1",
                params![super::metrics::METRIC_AUDIT_WRITE_FAILURES],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(AuditStats {
            audit_count,
            link_count,
            oldest_row,
            newest_row,
            supersession_count_30d,
            audit_write_failures,
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
    fn test_find_by_id_prefix_full_uuid() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        let matches = db.find_by_id_prefix(&id, None).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], id);
    }

    #[test]
    fn test_find_by_id_prefix_unique_short() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        let prefix = &id[..8];
        let matches = db.find_by_id_prefix(prefix, None).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], id);
    }

    #[test]
    fn test_find_by_id_prefix_no_match() {
        let db = test_db();
        db.insert_memory(sample_memory("test-project")).unwrap();
        // "zz" is not a valid hex prefix; no UUID can start with it
        let matches = db.find_by_id_prefix("zz", None).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_by_id_prefix_caps_at_two() {
        let db = test_db();
        db.insert_memory(sample_memory("test-project")).unwrap();
        db.insert_memory(sample_memory("test-project")).unwrap();
        db.insert_memory(sample_memory("test-project")).unwrap();
        // empty prefix matches all 3 rows via LIKE '%'; LIMIT 2 caps the result
        // so callers see "≥2" as the collision signal without enumerating all
        let matches = db.find_by_id_prefix("", None).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_find_by_id_prefix_respects_project_id() {
        let db = test_db();
        let id_a = db.insert_memory(sample_memory("project-a")).unwrap();
        let _id_b = db.insert_memory(sample_memory("project-b")).unwrap();
        let matches = db.find_by_id_prefix("", Some("project-a")).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], id_a);
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
    fn test_bump_recall_only_increments_count_and_last_recalled() {
        let db = test_db();
        let id = db.insert_memory(sample_memory("test-project")).unwrap();
        let before = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(before.recall_count, 0);
        assert!(before.last_recalled.is_none());
        let pre_avg = before.avg_relevance;

        let bumped = db.bump_recall_only(&id).unwrap();
        assert!(bumped);

        let after = db.get_memory(&id).unwrap().unwrap();
        assert_eq!(after.recall_count, 1);
        assert!(after.last_recalled.is_some());
        // avg_relevance MUST NOT change — that's the whole point of having a
        // separate helper. record_recall mixes the score into the EMA;
        // bump_recall_only does not.
        assert_eq!(after.avg_relevance, pre_avg);
    }

    #[test]
    fn test_bump_recall_only_returns_false_for_unknown_id() {
        let db = test_db();
        let bumped = db.bump_recall_only("nonexistent-id").unwrap();
        assert!(!bumped);
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

    // ---- F-002 audit helper / wrapper / supersession-SQL tests ----------

    use super::super::metrics::METRIC_AUDIT_WRITE_FAILURES;

    /// AC2 — strict helper writes one audit row plus N link rows with rank
    /// = 0-indexed position in input order.
    #[test]
    fn test_record_search_audit_writes_audit_and_links() {
        let db = test_db();
        let audit_id = db
            .record_search_audit(
                "q",
                Some("proj-x"),
                42,
                &["f1".to_string(), "f2".to_string(), "f3".to_string()],
            )
            .unwrap();

        let conn = db.lock_conn().unwrap();
        let (q, scope, took_ms): (String, Option<String>, i64) = conn
            .query_row(
                "SELECT query, scope, took_ms FROM memory_search_audit WHERE id = ?1",
                rusqlite::params![audit_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(q, "q");
        assert_eq!(scope, Some("proj-x".to_string()));
        assert_eq!(took_ms, 42);

        let mut stmt = conn
            .prepare(
                "SELECT fact_id, rank FROM audit_returned_facts \
                 WHERE audit_id = ?1 ORDER BY rank ASC",
            )
            .unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map(rusqlite::params![audit_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], ("f1".to_string(), 0));
        assert_eq!(rows[1], ("f2".to_string(), 1));
        assert_eq!(rows[2], ("f3".to_string(), 2));
    }

    /// AC2 — empty `returned_fact_ids` writes the audit row only (zero
    /// link rows).
    #[test]
    fn test_record_search_audit_empty_facts_writes_audit_only() {
        let db = test_db();
        let audit_id = db.record_search_audit("q", None, 7, &[]).unwrap();

        let conn = db.lock_conn().unwrap();
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_search_audit WHERE id = ?1",
                rusqlite::params![audit_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);

        let link_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_returned_facts WHERE audit_id = ?1",
                rusqlite::params![audit_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(link_count, 0);
    }

    /// AC2 — strict helper returns `Err` when the link table is dropped
    /// mid-test. Uses `lock_conn` (pub(crate)) for failure injection.
    /// Drop-table is the right injection: invalid fact_id INSERTs would
    /// NOT fail under PRAGMA foreign_keys = OFF.
    #[test]
    fn test_record_search_audit_returns_err_on_dropped_table() {
        let db = test_db();
        {
            let conn = db.lock_conn().unwrap();
            conn.execute("DROP TABLE audit_returned_facts;", [])
                .unwrap();
        }
        let result = db.record_search_audit("q", None, 1, &["f1".to_string()]);
        assert!(
            result.is_err(),
            "strict helper must return Err when audit_returned_facts is dropped"
        );
    }

    /// AC2 — best-effort wrapper does NOT panic and DOES bump the failure
    /// counter when the strict helper errors. This is the unit test that
    /// satisfies AC2's counter-increment claim — the wrapper is the unit
    /// under test, not the strict helper.
    #[test]
    fn test_record_search_audit_best_effort_increments_counter_on_failure() {
        let db = test_db();
        {
            let conn = db.lock_conn().unwrap();
            conn.execute("DROP TABLE audit_returned_facts;", [])
                .unwrap();
        }
        // Wrapper returns () — no panic.
        db.record_search_audit_best_effort("q", None, 1, &["f1".to_string()]);

        let counter = db.get_metric(METRIC_AUDIT_WRITE_FAILURES).unwrap();
        assert_eq!(counter, 1);
    }

    /// Helper for AC4 supersession-SQL tests: insert one memory_entries row
    /// pre-tombstoned with `valid_until` and one audit row at `searched_at`,
    /// linked together. Returns the inserted audit_id.
    ///
    /// `searched_at` and `valid_until` are caller-supplied so tests can seed
    /// in-window or out-of-window scenarios without depending on `Utc::now`
    /// at fixture construction time.
    fn seed_supersession_pair(
        conn: &Connection,
        fact_id: &str,
        searched_at: &str,
        valid_until: &str,
    ) -> i64 {
        let hash = format!("hash-{fact_id}-{}", uuid::Uuid::new_v4());
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, valid_until,
                 created_at, content_hash)
             VALUES (?1, 'proj', 'f.md', 'conclusion', 'decisional',
                     'title', 'content', '', ?2, ?3, ?2, ?4)",
            rusqlite::params![fact_id, searched_at, valid_until, hash],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at)
             VALUES ('q', 'proj', 1, ?1)",
            rusqlite::params![searched_at],
        )
        .unwrap();
        let aid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO audit_returned_facts (audit_id, fact_id, rank)
             VALUES (?1, ?2, 0)",
            rusqlite::params![aid, fact_id],
        )
        .unwrap();
        aid
    }

    // The supersession SQL itself is now a crate-internal const at module
    // scope (`super::F002_SUPERSESSION_SQL`) so production audit-stats code
    // and these tests share one source of truth. Reachable here via the
    // `use super::*;` at the top of this `mod tests` block.

    /// AC4 — 5 supersession events on a single day produce ≥1 row with
    /// `supersession_count >= 5`. Strategic-post Finding 1 schema-correctness
    /// gate.
    #[test]
    fn test_supersession_sql_returns_expected_rows() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();
        let valid_until = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();

        {
            let conn = db.lock_conn().unwrap();
            for i in 0..5 {
                let fact_id = format!("fact-{i}");
                seed_supersession_pair(&conn, &fact_id, &now, &valid_until);
            }
        }

        let conn = db.lock_conn().unwrap();
        let mut stmt = conn.prepare(F002_SUPERSESSION_SQL).unwrap();
        let counts: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            !counts.is_empty(),
            "expected at least one window with supersession_count >= 5"
        );
        let max_count = counts.iter().copied().max().unwrap_or(0);
        assert!(
            max_count >= 5,
            "expected at least one window with count >= 5, got max {max_count}"
        );
    }

    /// AC4 — 1 audit row + 5 link rows + 5 superseded entries also produces
    /// a row with `supersession_count >= 5`. Proves COUNT(*) counts joined
    /// audit/link/memory triples, NOT distinct audit rows. Documents the
    /// metric semantic for the downstream A-MEM trigger plan.
    #[test]
    fn test_supersession_sql_groups_by_joined_pairs() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();
        let valid_until = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();

        {
            let conn = db.lock_conn().unwrap();
            // One audit row.
            conn.execute(
                "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at)
                 VALUES ('q', 'proj', 1, ?1)",
                rusqlite::params![now],
            )
            .unwrap();
            let aid = conn.last_insert_rowid();
            // Five facts, all linked to that single audit row.
            for i in 0..5 {
                let fact_id = format!("pair-fact-{i}");
                let hash = format!("hash-{fact_id}-{}", uuid::Uuid::new_v4());
                conn.execute(
                    "INSERT INTO memory_entries
                        (id, project_id, source_file, source_type, knowledge_type,
                         title, content, entities, valid_from, valid_until,
                         created_at, content_hash)
                     VALUES (?1, 'proj', 'f.md', 'conclusion', 'decisional',
                             'title', 'content', '', ?2, ?3, ?2, ?4)",
                    rusqlite::params![fact_id, now, valid_until, hash],
                )
                .unwrap();
                conn.execute(
                    "INSERT INTO audit_returned_facts (audit_id, fact_id, rank)
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![aid, fact_id, i as i64],
                )
                .unwrap();
            }
        }

        let conn = db.lock_conn().unwrap();
        let mut stmt = conn.prepare(F002_SUPERSESSION_SQL).unwrap();
        let counts: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            !counts.is_empty(),
            "expected COUNT to count joined pairs, not distinct audit rows"
        );
        let max_count = counts.iter().copied().max().unwrap_or(0);
        assert!(
            max_count >= 5,
            "single audit row + 5 link rows must produce count >= 5, got {max_count}"
        );
    }

    /// AC4 — supersession events outside the 30-day window OR with
    /// valid_until > 7 days after searched_at must NOT appear in the output.
    #[test]
    fn test_supersession_sql_excludes_outside_window() {
        let db = test_db();
        // Case (a): searched_at 31 days ago — outside the 30-day window.
        let old_searched = (chrono::Utc::now() - chrono::Duration::days(31)).to_rfc3339();
        let old_valid_until = (chrono::Utc::now() - chrono::Duration::days(31)
            + chrono::Duration::days(1))
        .to_rfc3339();
        // Case (b): valid_until 10 days after searched_at — outside 7-day diff.
        let now = chrono::Utc::now().to_rfc3339();
        let far_valid_until = (chrono::Utc::now() + chrono::Duration::days(10)).to_rfc3339();

        {
            let conn = db.lock_conn().unwrap();
            for i in 0..5 {
                let id = format!("old-{i}");
                seed_supersession_pair(&conn, &id, &old_searched, &old_valid_until);
            }
            for i in 0..5 {
                let id = format!("far-{i}");
                seed_supersession_pair(&conn, &id, &now, &far_valid_until);
            }
        }

        let conn = db.lock_conn().unwrap();
        let mut stmt = conn.prepare(F002_SUPERSESSION_SQL).unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            rows.is_empty(),
            "out-of-window seeds must not satisfy HAVING supersession_count >= 5, got {rows:?}"
        );
    }

    // ---- F-005 audit-stats accessor tests ---------------------------------

    /// AC1 — fresh, freshly-migrated DB returns all-zeros snapshot with
    /// `oldest_row` / `newest_row` `None` and `audit_write_failures` 0.
    #[test]
    fn test_audit_stats_empty_db() {
        let db = test_db();
        let s = db.audit_stats().unwrap();
        assert_eq!(s.audit_count, 0);
        assert_eq!(s.link_count, 0);
        assert!(s.oldest_row.is_none());
        assert!(s.newest_row.is_none());
        assert_eq!(s.supersession_count_30d, 0);
        assert_eq!(s.audit_write_failures, 0);
    }

    /// AC1 — seeded DB with 3 audit rows + 5 link rows + audit_write_failures=2
    /// returns the expected six-field snapshot. Timestamps use a fixed RFC3339
    /// trio so MIN/MAX assertions are deterministic (independent of `Utc::now`
    /// at fixture construction time).
    #[test]
    fn test_audit_stats_populated() {
        let db = test_db();
        let t0 = "2026-05-01T00:00:00+00:00".to_string();
        let t1 = "2026-05-04T12:00:00+00:00".to_string();
        let t2 = "2026-05-08T08:00:00+00:00".to_string();
        // Seed 3 audit rows + 5 link rows. We attach all 5 link rows to the
        // first audit row (the audit/link cardinality is independent in this
        // test, so a 3+5 split is the minimum that exercises the two
        // independent counts).
        let aid_first = {
            let conn = db.lock_conn().unwrap();
            conn.execute(
                "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
                 VALUES ('q', 'proj', 1, ?1)",
                rusqlite::params![t0],
            )
            .unwrap();
            let aid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
                 VALUES ('q', 'proj', 1, ?1)",
                rusqlite::params![t1],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
                 VALUES ('q', 'proj', 1, ?1)",
                rusqlite::params![t2],
            )
            .unwrap();
            // 5 fact rows (just memory_entries — no valid_until needed for
            // the count-only assertions in this test) + 5 link rows.
            for i in 0..5 {
                let fact_id = format!("populated-fact-{i}");
                let hash = format!("hash-{fact_id}-{}", uuid::Uuid::new_v4());
                conn.execute(
                    "INSERT INTO memory_entries \
                        (id, project_id, source_file, source_type, knowledge_type, \
                         title, content, entities, valid_from, created_at, content_hash) \
                     VALUES (?1, 'proj', 'f.md', 'conclusion', 'decisional', \
                             'title', 'content', '', ?2, ?2, ?3)",
                    rusqlite::params![fact_id, t0, hash],
                )
                .unwrap();
                conn.execute(
                    "INSERT INTO audit_returned_facts (audit_id, fact_id, rank) \
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![aid, fact_id, i as i64],
                )
                .unwrap();
            }
            aid
        };
        let _ = aid_first; // suppress unused warning if test rewritten

        // Bump audit_write_failures to 2.
        db.increment_metric(METRIC_AUDIT_WRITE_FAILURES).unwrap();
        db.increment_metric(METRIC_AUDIT_WRITE_FAILURES).unwrap();

        let s = db.audit_stats().unwrap();
        assert_eq!(s.audit_count, 3);
        assert_eq!(s.link_count, 5);
        assert_eq!(s.oldest_row.as_deref(), Some(t0.as_str()));
        assert_eq!(s.newest_row.as_deref(), Some(t2.as_str()));
        assert_eq!(s.audit_write_failures, 2);
        // Locks in the AC1 fixture's 6th-field expectation: fact rows seeded
        // without `valid_until` produce zero supersession events. AC4's
        // `test_audit_stats_supersession_count` exercises the non-zero path.
        assert_eq!(s.supersession_count_30d, 0);
    }

    /// AC1 + AC4 — seeding 5 supersession events on a single day produces
    /// `supersession_count_30d == 5` from the new sister query (no
    /// GROUP BY / HAVING — the bare COUNT*. Mirrors the seed shape of
    /// `test_supersession_sql_returns_expected_rows` so the two queries can
    /// be cross-checked against the same fixture if the test is ever
    /// re-run side-by-side.
    #[test]
    fn test_audit_stats_supersession_count() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();
        let valid_until = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
        {
            let conn = db.lock_conn().unwrap();
            for i in 0..5 {
                let fact_id = format!("ssn-fact-{i}");
                seed_supersession_pair(&conn, &fact_id, &now, &valid_until);
            }
        }
        let s = db.audit_stats().unwrap();
        assert_eq!(
            s.supersession_count_30d, 5,
            "5 in-window superseded facts should yield supersession_count_30d == 5"
        );
    }

    /// AC4 — both supersession SQL consts must contain the exact shared
    /// WHERE-clause fragment, so any future drift between them is caught
    /// at compile-time test-pass-time. The literal text is the one piece
    /// the F-005 plan AC4 calls out by name.
    #[test]
    fn test_audit_stats_where_clause_shared() {
        const SHARED_WHERE_FRAGMENT: &str =
            "JULIANDAY(me.valid_until) - JULIANDAY(a.searched_at) <= 7";
        assert!(
            super::F002_SUPERSESSION_SQL.contains(SHARED_WHERE_FRAGMENT),
            "F002_SUPERSESSION_SQL must contain the shared WHERE fragment"
        );
        assert!(
            super::AUDIT_STATS_SUPERSESSION_COUNT_SQL.contains(SHARED_WHERE_FRAGMENT),
            "AUDIT_STATS_SUPERSESSION_COUNT_SQL must contain the shared WHERE fragment"
        );
    }
}
