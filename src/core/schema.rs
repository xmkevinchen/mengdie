use anyhow::bail;
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

/// Head schema version. Each migration block writes its OWN target version
/// as a literal (per F-002 plan Step 1 Doodlestein-adversarial M7) rather
/// than templating off this constant — so this binding is referenced only
/// from `#[cfg(test)]` head-version assertions (e.g. AC1 idempotency check).
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 6;

/// Allowed `source_type` values. Enforced via BEFORE INSERT / BEFORE UPDATE
/// triggers installed in the v5 migration (plan 017). SQLite does not
/// support `ALTER TABLE ... ADD CONSTRAINT CHECK`, so triggers are the
/// fallback. Update this list and the trigger bodies together.
pub const ALLOWED_SOURCE_TYPES: &[&str] = &[
    "conclusion",
    "review",
    "plan",
    "analysis",
    "retrospect",
    "synthesis",
];

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

/// Compute the cluster-hash dedup key for a synthesis row (plan 017).
///
/// Syntheses dedup on `(project_id, synthesis_cluster_hash)` instead of
/// `(project_id, content_hash)`. The cluster-hash is derived from the
/// sorted + deduped source memory IDs that fed the synthesis — so any
/// re-synthesis of the same cluster (e.g. after a `SYSTEM_PROMPT` edit)
/// produces the SAME key and UPSERTS the existing row instead of creating
/// a zombie sibling.
///
/// Determinism rules, locked here so `Vec::sort()` discipline is not lost
/// in a future refactor:
/// - Lexicographic sort on the UTF-8 `String` form of each ID (default
///   `Vec<String>::sort`). UUIDs are unique so a stable sort is not
///   required; `sort()` is chosen for clarity. Do NOT switch to a
///   byte-wise sort or a numeric sort.
/// - `dedup()` is called after `sort()` to collapse any duplicates the
///   caller passed in (defensive — `insert_synthesis_with_links` is
///   responsible for de-duplicating its input, but the hash MUST be
///   stable when duplicates slip through).
/// - IDs joined with the single ASCII comma `,` character. No spaces.
/// - SHA-256 hex encoding, matching `compute_content_hash`.
///
/// Plan 017 /ae:review F6: empty `source_ids` is a caller bug —
/// `insert_synthesis_with_links` guards against it with `anyhow::ensure!`
/// and the migration's Pre-check 2 catches zero-link synthesis rows. Hashing
/// an empty slice would yield `sha256("")` which silently collapses all
/// zero-source clusters into one, so any other caller that bypasses the
/// insert guard would create hidden dedup collisions. We reject it in debug
/// builds and document the contract; release builds still hash deterministically
/// so a slipped caller fails at partial-index uniqueness check rather than here.
pub fn compute_synthesis_cluster_hash(source_ids: &[String]) -> String {
    debug_assert!(
        !source_ids.is_empty(),
        "compute_synthesis_cluster_hash: source_ids must be non-empty (caller contract)"
    );
    let mut ids: Vec<String> = source_ids.to_vec();
    ids.sort();
    ids.dedup();
    let joined = ids.join(",");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Run all schema migrations. Idempotent — safe to call on every startup.
///
/// Returns `anyhow::Result<()>` rather than `rusqlite::Result<()>` because
/// some migration pre-checks (plan 017 v5) produce custom abort messages
/// that don't map cleanly to rusqlite's feature-gated `ModuleError` variant.
pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout=5000;")?;
    // Plan F-002 / discussion 029 YAGNI 1: FK clauses are documentation-only
    // project-wide. Make this explicit at the connection level so the
    // assumption holds regardless of the SQLite build's FK default and
    // regardless of whether a caller pre-enabled enforcement. Closes BL-015
    // (filed at F-002 Step 1, triggered at Step 4 when the audit-helper unit
    // tests hit FOREIGN KEY constraint errors on synthetic fact_ids that
    // don't reference real memory_entries rows).
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

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

    // Migration v5: synthesis cluster-hash dedup + partial indexes + source_type trigger (plan 017).
    //
    // Wrapped in explicit BEGIN/COMMIT/ROLLBACK via execute_batch because the
    // signature is `&Connection` (immutable) and `Connection::transaction()`
    // requires `&mut self`. Preserving the signature avoids rippling into
    // `Db::open` + `Db::open_in_memory` + all tests.
    //
    // See plan 017 Step 1 for the full rationale of each sub-step.
    if current_version < 5 {
        conn.execute_batch("BEGIN TRANSACTION;")?;
        // Plan 017 /ae:review P2-B: the `PRAGMA user_version` write lives INSIDE
        // the transaction so the version bump commits atomically with the schema
        // changes. A crash between COMMIT and an external PRAGMA write would
        // otherwise leave a v5 schema reading `user_version = 4`, causing the
        // next startup to re-run v5_migration (all pre-checks pass so it would
        // complete idempotently, but it emits confusing restart log noise).
        let migration_result = (|| -> anyhow::Result<()> {
            v5_migration(conn)?;
            // Plan F-002 Doodlestein-adversarial M7: each migration block pins
            // its OWN target version as a literal, NOT `{SCHEMA_VERSION}`. After
            // a future v7 bumps the constant, the v5 block must still write 5,
            // not the head version — otherwise a v6-crash-after-v5-commit
            // produces a v5 schema reading user_version = head.
            conn.execute_batch("PRAGMA user_version = 5;")?;
            Ok(())
        })();
        match migration_result {
            Ok(()) => conn.execute_batch("COMMIT;")?,
            Err(e) => {
                // Best-effort rollback. If ROLLBACK itself fails, surface the
                // original error — the rollback failure is noise by comparison.
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }

    // Migration v6: persisted domain audit (F-002 Wave 1, plan F-002 Step 1).
    //
    // Adds two tables and three indexes:
    //   - `memory_search_audit` — one row per memory_search call (query, scope,
    //     took_ms, searched_at).
    //   - `audit_returned_facts` — link table (audit_id, fact_id, rank) with
    //     unenforced FKs into both audit and memory_entries (PRAGMA
    //     foreign_keys stays OFF project-wide; FK clauses are documentation
    //     per 029 YAGNI 1).
    //   - Three indexes covering the AC4 supersession SQL: composite on
    //     (searched_at, id), reverse-FK covering on (fact_id, audit_id), and
    //     a partial on memory_entries(valid_until, id) WHERE valid_until IS
    //     NOT NULL.
    //
    // Atomicity rests on SQLite's transaction guarantee: the version write
    // commits with the schema as one unit. `IF NOT EXISTS` clauses are
    // belt-and-braces idempotence, not the primary safety mechanism.
    if current_version < 6 {
        conn.execute_batch("BEGIN TRANSACTION;")?;
        let migration_result = (|| -> anyhow::Result<()> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS memory_search_audit (
                    id          INTEGER PRIMARY KEY,
                    query       TEXT NOT NULL,
                    scope       TEXT,
                    took_ms     INTEGER NOT NULL,
                    searched_at TEXT NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS audit_returned_facts (
                    audit_id INTEGER NOT NULL,
                    fact_id  TEXT NOT NULL,
                    rank     INTEGER NOT NULL,
                    PRIMARY KEY (audit_id, fact_id),
                    FOREIGN KEY (audit_id) REFERENCES memory_search_audit(id),
                    FOREIGN KEY (fact_id) REFERENCES memory_entries(id)
                 );

                 CREATE INDEX IF NOT EXISTS idx_memory_search_audit_searched_id
                     ON memory_search_audit(searched_at, id);

                 CREATE INDEX IF NOT EXISTS idx_audit_returned_facts_fact_audit
                     ON audit_returned_facts(fact_id, audit_id);

                 CREATE INDEX IF NOT EXISTS idx_memory_entries_valid_until_id
                     ON memory_entries(valid_until, id)
                     WHERE valid_until IS NOT NULL;

                 PRAGMA user_version = 6;",
            )?;
            Ok(())
        })();
        match migration_result {
            Ok(()) => conn.execute_batch("COMMIT;")?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }

    Ok(())
}

/// v5 migration body — plan 017. Factored out of `run_migrations` so the
/// caller can wrap it in a single BEGIN/COMMIT/ROLLBACK block without the
/// ownership gymnastics of returning from a closure.
fn v5_migration(conn: &Connection) -> anyhow::Result<()> {
    // ---- Pre-check 1: orphan links ------------------------------------
    // Dangling source_memory_id pointers are silent corruption under the
    // current unenforced-FK schema. Abort rather than silently omit.
    let orphan_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memory_synthesis_links l
         LEFT JOIN memory_entries e ON e.id = l.source_memory_id
         WHERE e.id IS NULL",
        [],
        |r| r.get(0),
    )?;
    if orphan_count > 0 {
        bail!(
            "plan 017 v5 migration aborted: {orphan_count} orphan rows in memory_synthesis_links \
             (link.source_memory_id points to missing memory_entries.id). \
             Repair manually before retrying."
        );
    }

    // ---- Pre-check 1b: orphan / wrong-type synthesis_memory_id -----
    // Plan 017 /ae:review P2-A (codex P1.1): Pre-check 1 only validates
    // `source_memory_id`. A link row whose `synthesis_memory_id` does not
    // exist in `memory_entries`, or exists but has `source_type != 'synthesis'`,
    // would cause the coalesce/backfill paths to either silently no-op
    // (wrong-type case — backfill WHERE source_type = 'synthesis' filters
    // it) or — worse — pick the non-synthesis row as a coalesce candidate
    // and potentially tombstone the real synthesis. Abort loudly instead.
    let bad_syn_ref_query = "SELECT DISTINCT l.synthesis_memory_id
         FROM memory_synthesis_links l
         LEFT JOIN memory_entries e ON e.id = l.synthesis_memory_id
         WHERE e.id IS NULL OR e.source_type != 'synthesis'";
    let mut stmt_bad_syn = conn.prepare(bad_syn_ref_query)?;
    let bad_syn_refs: Vec<String> = stmt_bad_syn
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt_bad_syn);
    if !bad_syn_refs.is_empty() {
        bail!(
            "plan 017 v5 migration aborted: {} memory_synthesis_links row(s) reference \
             synthesis_memory_id values that are missing from memory_entries or point \
             to a non-synthesis row: {:?}. Repair manually before retrying.",
            bad_syn_refs.len(),
            bad_syn_refs
        );
    }

    // ---- Pre-check 2: synthesis rows with zero links -------------------
    // Such rows would backfill to `sha256("")` and silently collapse — worse
    // than a visible error.
    let mut stmt_zero_link = conn.prepare(
        "SELECT id FROM memory_entries
         WHERE source_type = 'synthesis'
           AND id NOT IN (SELECT DISTINCT synthesis_memory_id FROM memory_synthesis_links)",
    )?;
    let zero_link_ids: Vec<String> = stmt_zero_link
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt_zero_link);
    if !zero_link_ids.is_empty() {
        bail!(
            "plan 017 v5 migration aborted: {} synthesis row(s) have zero entries in \
             memory_synthesis_links: {:?}. Either restore links or delete rows before retrying.",
            zero_link_ids.len(),
            zero_link_ids
        );
    }

    // ---- Pre-check: invalid source_type values -------------------------
    // The trigger below enforces an allowlist; if existing data falls outside
    // it, the post-trigger state rejects legitimate existing rows on any
    // future UPDATE. Surface violations loudly and halt.
    let allowed_sql_list = ALLOWED_SOURCE_TYPES
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let invalid_source_type_query = format!(
        "SELECT DISTINCT source_type FROM memory_entries WHERE source_type NOT IN ({allowed_sql_list})"
    );
    let mut stmt_invalid = conn.prepare(&invalid_source_type_query)?;
    let invalid_source_types: Vec<String> = stmt_invalid
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt_invalid);
    if !invalid_source_types.is_empty() {
        bail!(
            "plan 017 v5 migration aborted: existing rows use source_type values outside the \
             allowlist {:?}: found {:?}. Reclassify or extend ALLOWED_SOURCE_TYPES before retrying.",
            ALLOWED_SOURCE_TYPES,
            invalid_source_types
        );
    }

    // ---- Pre-check 3: legacy duplicate clusters ------------------------
    // Before the migration, two synthesis rows with the SAME source set
    // could coexist (dedup was on content_hash; a prompt change produced
    // a zombie second row). The new unique index on synthesis_cluster_hash
    // would fail to create if such duplicates remain. Coalesce by keeping
    // the newest row (created_at DESC) and invalidating the older sibling(s).
    //
    // Heuristic note (plan 017 Doodlestein regret): `created_at` is a proxy
    // for "better row", not a truth. At the current 27-row corpus scale
    // the probability of actual legacy duplicates is low; operator has
    // `mengdie invalidate` as a manual escape hatch post-migration.
    let mut stmt_pairs =
        conn.prepare("SELECT synthesis_memory_id, source_memory_id FROM memory_synthesis_links")?;
    let all_pairs: Vec<(String, String)> = stmt_pairs
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt_pairs);
    let mut by_syn: std::collections::HashMap<String, Vec<String>> = Default::default();
    for (syn_id, src_id) in all_pairs {
        by_syn.entry(syn_id).or_default().push(src_id);
    }
    let mut by_cluster: std::collections::HashMap<String, Vec<String>> = Default::default();
    for (syn_id, src_ids) in &by_syn {
        let hash = compute_synthesis_cluster_hash(src_ids);
        by_cluster.entry(hash).or_default().push(syn_id.clone());
    }
    // For each cluster with multiple synthesis rows, coalesce.
    //
    // Plan 017 /ae:review P1-A (codex P1.2): candidates are filtered to
    // LIVE rows only (`valid_until IS NULL`). Before the fix, an
    // already-invalidated sibling with a newer `created_at` could win the
    // sort and cause the only live row in the cluster to be tombstoned —
    // leaving the cluster with no active synthesis.
    //
    // `created_at` tie-break: if two live rows have identical timestamps,
    // the `sort_by` output order depends on the input order which comes
    // from HashMap iteration (non-deterministic). Acceptable risk at
    // current corpus scale (plan 017 Doodlestein regret note); operator
    // escape hatch is `mengdie invalidate` post-migration.
    let now_for_coalesce = chrono::Utc::now().to_rfc3339();
    for (cluster_hash, syn_ids) in &by_cluster {
        if syn_ids.len() > 1 {
            // Fetch (id, created_at) for each LIVE candidate only.
            // Rows that are already invalidated (valid_until IS NOT NULL)
            // must NOT participate in the keeper election — they were
            // invalidated for a reason and are not eligible to displace a
            // live sibling.
            let mut with_ts: Vec<(String, String)> = Vec::with_capacity(syn_ids.len());
            for syn_id in syn_ids {
                let row: Option<(String, Option<String>)> = conn
                    .query_row(
                        "SELECT id, valid_until FROM memory_entries WHERE id = ?1",
                        rusqlite::params![syn_id],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
                    )
                    .optional()?;
                if let Some((_, valid_until)) = row {
                    if valid_until.is_none() {
                        let ts: String = conn.query_row(
                            "SELECT created_at FROM memory_entries WHERE id = ?1",
                            rusqlite::params![syn_id],
                            |r| r.get(0),
                        )?;
                        with_ts.push((syn_id.clone(), ts));
                    }
                }
            }
            // If fewer than 2 live candidates remain, there's no duplicate to
            // coalesce — the cluster is already deduplicated de facto.
            if with_ts.len() < 2 {
                continue;
            }
            // Keep newest live row; invalidate older live siblings.
            with_ts.sort_by(|a, b| b.1.cmp(&a.1));
            let keep_id = with_ts[0].0.clone();
            let invalidate_ids: Vec<String> =
                with_ts[1..].iter().map(|(id, _)| id.clone()).collect();
            tracing::info!(
                cluster = %cluster_hash,
                keep = %keep_id,
                invalidated = ?invalidate_ids,
                "plan 017 v5 migration: coalescing legacy duplicate synthesis cluster"
            );
            for id in &invalidate_ids {
                conn.execute(
                    "UPDATE memory_entries
                     SET valid_until = ?1,
                         invalidation_reason = 'merged by plan 017 cluster-hash migration'
                     WHERE id = ?2",
                    rusqlite::params![now_for_coalesce, id],
                )?;
            }
        }
    }

    // ---- Schema changes -------------------------------------------------
    // Add cluster-hash column. Nullable — primary sources never populate it.
    // Idempotence guard: SQLite does not support `ALTER TABLE ... ADD COLUMN
    // IF NOT EXISTS`, so we check via PRAGMA before adding. Matches v2's
    // pattern at the top of `run_migrations`.
    if !column_exists(conn, "memory_entries", "synthesis_cluster_hash")? {
        conn.execute_batch("ALTER TABLE memory_entries ADD COLUMN synthesis_cluster_hash TEXT;")?;
    }

    // Make idx_memory_content_hash partial so synthesis rows dedup on
    // cluster_hash only. Two syntheses with different source sets but
    // coincidentally identical content text can now coexist.
    conn.execute_batch(
        "DROP INDEX IF EXISTS idx_memory_content_hash;
         CREATE UNIQUE INDEX idx_memory_content_hash
             ON memory_entries(project_id, content_hash)
             WHERE source_type != 'synthesis';",
    )?;

    // ---- Backfill synthesis_cluster_hash for existing rows -------------
    // Row-by-row loop. Batched CTE deferred per plan 017 Out of Scope.
    // Only populate non-invalidated synthesis rows; rows coalesced above
    // keep synthesis_cluster_hash = NULL (they are tombstones).
    for (syn_id, src_ids) in &by_syn {
        let hash = compute_synthesis_cluster_hash(src_ids);
        conn.execute(
            "UPDATE memory_entries
             SET synthesis_cluster_hash = ?1
             WHERE id = ?2
               AND source_type = 'synthesis'
               AND valid_until IS NULL",
            rusqlite::params![hash, syn_id],
        )?;
    }

    // Partial unique index on the new column. `IS NOT NULL` guard: SQLite
    // allows multi-NULL under unique constraints by default; without the
    // guard a synthesis row with accidental NULL cluster_hash could have
    // zombie siblings.
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_synthesis_cluster
             ON memory_entries(project_id, synthesis_cluster_hash)
             WHERE source_type = 'synthesis' AND synthesis_cluster_hash IS NOT NULL;",
    )?;

    // source_type allowlist triggers. ALTER TABLE ADD CONSTRAINT CHECK is
    // unsupported on existing SQLite tables; triggers are the fallback.
    // Note: these limit values to the allowlist but do NOT prevent a
    // synthesis row's source_type from being UPDATED to another allowed
    // value (e.g., "conclusion"). Full write-once enforcement deferred.
    let allowed_sql_list = ALLOWED_SOURCE_TYPES
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS memory_entries_source_type_insert_check
         BEFORE INSERT ON memory_entries
         FOR EACH ROW
         WHEN NEW.source_type NOT IN ({allowed_sql_list})
         BEGIN
             SELECT RAISE(ABORT, 'invalid source_type');
         END;

         CREATE TRIGGER IF NOT EXISTS memory_entries_source_type_update_check
         BEFORE UPDATE OF source_type ON memory_entries
         FOR EACH ROW
         WHEN NEW.source_type NOT IN ({allowed_sql_list})
         BEGIN
             SELECT RAISE(ABORT, 'invalid source_type');
         END;"
    ))?;

    // ---- Final safety: integrity check --------------------------------
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        bail!("plan 017 v5 migration: PRAGMA integrity_check failed: {integrity}");
    }

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
    fn test_v6_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 6);

        // Idempotent: re-running leaves version at 6.
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 6);
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

        // Re-run migrations — should upgrade to current version (v5) cleanly.
        run_migrations(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

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

    // ---- Plan 017 (v5) tests ----------------------------------------------

    /// Seed a v4 DB state from a fresh in-memory connection. The DB ends with
    /// `user_version = 4` and the full v4 schema in place (memory_entries +
    /// memory_synthesis_links). Used by the v5-upgrade tests below.
    fn seed_v4_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        // Production `Db::open` does not set PRAGMA foreign_keys. But
        // `rusqlite` with the `bundled` feature compiles SQLite with FK
        // enforcement ON by default in some builds. Disable explicitly so
        // these tests can seed intentionally-inconsistent states (orphan
        // links) to exercise the migration pre-checks.
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        run_migrations(&conn).unwrap();
        // Drop v5-specific objects so we can replay the migration from v4.
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS memory_entries_source_type_insert_check;
             DROP TRIGGER IF EXISTS memory_entries_source_type_update_check;
             DROP INDEX IF EXISTS idx_synthesis_cluster;",
        )
        .unwrap();
        // Restore non-partial content_hash index.
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_memory_content_hash;
             CREATE UNIQUE INDEX idx_memory_content_hash
                 ON memory_entries(project_id, content_hash);",
        )
        .unwrap();
        // Wipe the v5 column so the backfill re-runs.
        conn.execute_batch("UPDATE memory_entries SET synthesis_cluster_hash = NULL;")
            .unwrap();
        conn.execute_batch("PRAGMA user_version = 4;").unwrap();
        conn
    }

    fn insert_raw_memory(
        conn: &Connection,
        id: &str,
        source_type: &str,
        content: &str,
        created_at: &str,
    ) {
        let hash = compute_content_hash(content);
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id,
                "proj",
                "f.md",
                source_type,
                "decisional",
                format!("title-{id}"),
                content,
                "tag",
                created_at,
                created_at,
                hash,
            ],
        )
        .unwrap();
    }

    fn link_source(conn: &Connection, source_id: &str, synthesis_id: &str) {
        conn.execute(
            "INSERT INTO memory_synthesis_links (source_memory_id, synthesis_memory_id, created_at)
             VALUES (?1, ?2, '2026-04-20T00:00:00Z')",
            rusqlite::params![source_id, synthesis_id],
        )
        .unwrap();
    }

    #[test]
    fn test_compute_synthesis_cluster_hash_order_independent() {
        let h1 =
            compute_synthesis_cluster_hash(&["a".to_string(), "b".to_string(), "c".to_string()]);
        let h2 =
            compute_synthesis_cluster_hash(&["c".to_string(), "a".to_string(), "b".to_string()]);
        assert_eq!(h1, h2, "cluster hash must be sort-order independent");
    }

    #[test]
    fn test_compute_synthesis_cluster_hash_dedups_input() {
        let h1 = compute_synthesis_cluster_hash(&["a".to_string(), "b".to_string()]);
        let h2 =
            compute_synthesis_cluster_hash(&["a".to_string(), "a".to_string(), "b".to_string()]);
        assert_eq!(h1, h2, "dedup should collapse duplicate input IDs");
    }

    #[test]
    fn test_compute_synthesis_cluster_hash_known_value() {
        // Locks the algorithm: sort(["a","b"]) -> "a,b" -> sha256 hex.
        // Any future change that breaks this assertion breaks existing data.
        let h = compute_synthesis_cluster_hash(&["a".to_string(), "b".to_string()]);
        // sha256("a,b") precomputed (shell: printf 'a,b' | shasum -a 256).
        assert_eq!(
            h,
            "1eb7c54d52831bbfe8942af0b1c56b7409523a59ed6ca99c1174fef7eb32c1b5"
        );
    }

    #[test]
    fn test_migration_v4_to_v5_happy_path() {
        let conn = seed_v4_db();
        insert_raw_memory(&conn, "src-1", "conclusion", "s1", "2026-04-20T00:00:00Z");
        insert_raw_memory(&conn, "src-2", "conclusion", "s2", "2026-04-20T00:00:00Z");
        insert_raw_memory(&conn, "syn-1", "synthesis", "v1", "2026-04-20T12:00:00Z");
        link_source(&conn, "src-1", "syn-1");
        link_source(&conn, "src-2", "syn-1");

        run_migrations(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        // Backfilled cluster_hash matches helper.
        let expected = compute_synthesis_cluster_hash(&["src-1".to_string(), "src-2".to_string()]);
        let actual: String = conn
            .query_row(
                "SELECT synthesis_cluster_hash FROM memory_entries WHERE id = 'syn-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actual, expected);

        // Primary source has NULL cluster_hash (synthesis-only column).
        let src_hash: Option<String> = conn
            .query_row(
                "SELECT synthesis_cluster_hash FROM memory_entries WHERE id = 'src-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(src_hash.is_none());
    }

    #[test]
    fn test_migration_v4_to_v5_rejects_orphan_links() {
        let conn = seed_v4_db();
        // Insert a synthesis + one link to a missing source memory.
        insert_raw_memory(&conn, "syn-1", "synthesis", "v1", "2026-04-20T00:00:00Z");
        link_source(&conn, "ghost-source", "syn-1");

        let err = run_migrations(&conn).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("orphan rows"),
            "expected orphan-link error, got: {msg}"
        );
    }

    #[test]
    fn test_migration_v4_to_v5_rejects_zero_link_synthesis() {
        let conn = seed_v4_db();
        insert_raw_memory(&conn, "syn-1", "synthesis", "v1", "2026-04-20T00:00:00Z");
        // No links inserted for syn-1.

        let err = run_migrations(&conn).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("zero entries"),
            "expected zero-link error, got: {msg}"
        );
    }

    #[test]
    fn test_migration_v4_to_v5_rejects_pre_existing_invalid_source_type() {
        let conn = seed_v4_db();
        // v4 has no trigger, so we can insert an invalid source_type directly.
        insert_raw_memory(&conn, "row-1", "notes", "c", "2026-04-20T00:00:00Z");

        let err = run_migrations(&conn).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("outside the allowlist") || msg.contains("allowlist"),
            "expected allowlist-violation error, got: {msg}"
        );
        assert!(
            msg.contains("notes"),
            "expected error to name the offending source_type 'notes', got: {msg}"
        );
    }

    #[test]
    fn test_migration_v4_to_v5_coalesces_legacy_duplicate_clusters() {
        let conn = seed_v4_db();
        insert_raw_memory(&conn, "src-1", "conclusion", "s1", "2026-04-20T00:00:00Z");
        // Two synthesis rows for the SAME source set ({src-1}), different created_at.
        insert_raw_memory(&conn, "syn-old", "synthesis", "v1", "2026-04-20T10:00:00Z");
        insert_raw_memory(&conn, "syn-new", "synthesis", "v2", "2026-04-22T10:00:00Z");
        link_source(&conn, "src-1", "syn-old");
        link_source(&conn, "src-1", "syn-new");

        run_migrations(&conn).unwrap();

        // Newer row kept (valid_until IS NULL), older row invalidated.
        let new_valid_until: Option<String> = conn
            .query_row(
                "SELECT valid_until FROM memory_entries WHERE id = 'syn-new'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            new_valid_until.is_none(),
            "newer row (syn-new) should remain valid"
        );
        let old_valid_until: Option<String> = conn
            .query_row(
                "SELECT valid_until FROM memory_entries WHERE id = 'syn-old'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            old_valid_until.is_some(),
            "older row (syn-old) should be invalidated"
        );
        let reason: Option<String> = conn
            .query_row(
                "SELECT invalidation_reason FROM memory_entries WHERE id = 'syn-old'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            reason,
            Some("merged by plan 017 cluster-hash migration".to_string())
        );

        // Partial unique index allows no duplicates (only syn-new has cluster_hash).
        let syn_new_hash: Option<String> = conn
            .query_row(
                "SELECT synthesis_cluster_hash FROM memory_entries WHERE id = 'syn-new'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(syn_new_hash.is_some());
        let syn_old_hash: Option<String> = conn
            .query_row(
                "SELECT synthesis_cluster_hash FROM memory_entries WHERE id = 'syn-old'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            syn_old_hash.is_none(),
            "invalidated duplicate should retain NULL cluster_hash (tombstone)"
        );
    }

    #[test]
    fn test_trigger_rejects_invalid_source_type_on_insert() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Direct SQL insert with an invalid source_type should be aborted.
        let result = conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at, content_hash)
             VALUES ('x', 'proj', 'f', 'junk', 'decisional', 't', 'c', 'e',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'h')",
            [],
        );
        assert!(result.is_err(), "insert with junk source_type must fail");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("invalid source_type"),
            "expected trigger ABORT message, got: {msg}"
        );
    }

    #[test]
    fn test_trigger_rejects_invalid_source_type_on_update() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at, content_hash)
             VALUES ('x', 'proj', 'f', 'conclusion', 'decisional', 't', 'c', 'e',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'h')",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "UPDATE memory_entries SET source_type = 'junk' WHERE id = 'x'",
            [],
        );
        assert!(
            result.is_err(),
            "update to junk source_type must fail under trigger"
        );
    }

    /// Plan 017 /ae:review P3-B (challenger F3): the UPDATE trigger is
    /// scoped to `BEFORE UPDATE OF source_type` — it must NOT fire on
    /// unrelated column updates. Regression guard for a future refactor
    /// that accidentally broadens trigger scope.
    #[test]
    fn test_trigger_allows_non_source_type_updates_on_synthesis_rows() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Insert a synthesis row directly (migration is complete; trigger active).
        // We bypass insert_synthesis_with_links here because we only need one row.
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at,
                 content_hash, synthesis_cluster_hash)
             VALUES ('syn-x', 'proj', 'synthesis', 'synthesis', 'factual',
                     't', 'c', 'e', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                     'ch', 'clh')",
            [],
        )
        .unwrap();
        // A non-source_type UPDATE must succeed (trigger is BEFORE UPDATE OF source_type).
        let updated = conn
            .execute(
                "UPDATE memory_entries SET recall_count = 5 WHERE id = 'syn-x'",
                [],
            )
            .expect("recall_count UPDATE must not fire the source_type trigger");
        assert_eq!(updated, 1);
    }

    /// Plan 017 /ae:review P1-A regression (codex P1.2): the coalesce
    /// logic must NOT tombstone a live synthesis row just because an
    /// already-invalidated sibling happens to have a newer `created_at`.
    /// Before the fix, `valid_until IS NULL` was not part of the keeper
    /// filter, so an invalidated newer row could displace a live older row
    /// and leave the cluster with no active synthesis.
    #[test]
    fn test_migration_v4_to_v5_coalesce_ignores_already_invalidated_siblings() {
        let conn = seed_v4_db();
        insert_raw_memory(&conn, "src-1", "conclusion", "s1", "2026-04-20T00:00:00Z");
        // Live row (older).
        insert_raw_memory(&conn, "syn-live", "synthesis", "v1", "2026-04-20T10:00:00Z");
        // Already-invalidated row (newer created_at — would win the sort pre-fix).
        insert_raw_memory(
            &conn,
            "syn-tombstone",
            "synthesis",
            "v2",
            "2026-04-25T10:00:00Z",
        );
        conn.execute(
            "UPDATE memory_entries
             SET valid_until = '2026-04-25T11:00:00Z',
                 invalidation_reason = 'pre-existing tombstone'
             WHERE id = 'syn-tombstone'",
            [],
        )
        .unwrap();
        link_source(&conn, "src-1", "syn-live");
        link_source(&conn, "src-1", "syn-tombstone");

        run_migrations(&conn).unwrap();

        // The live row must still be live — the tombstone must not have
        // displaced it via the created_at DESC sort.
        let live_valid_until: Option<String> = conn
            .query_row(
                "SELECT valid_until FROM memory_entries WHERE id = 'syn-live'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            live_valid_until.is_none(),
            "live synthesis row must survive coalesce when invalidated sibling has newer created_at"
        );
        // Live row got backfilled with a cluster_hash.
        let live_hash: Option<String> = conn
            .query_row(
                "SELECT synthesis_cluster_hash FROM memory_entries WHERE id = 'syn-live'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(live_hash.is_some());
        // Pre-existing tombstone reason is preserved (NOT overwritten by coalesce).
        let tombstone_reason: Option<String> = conn
            .query_row(
                "SELECT invalidation_reason FROM memory_entries WHERE id = 'syn-tombstone'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            tombstone_reason,
            Some("pre-existing tombstone".to_string()),
            "pre-existing invalidation_reason must not be overwritten"
        );
    }

    /// Plan 017 /ae:review P2-A (codex P1.1): `memory_synthesis_links.synthesis_memory_id`
    /// pointing to a missing row, or to a row with `source_type != 'synthesis'`,
    /// must abort the migration at the new pre-check 1b. Without this guard
    /// the wrong-type case could cause the coalesce path to tombstone a real
    /// synthesis in favour of a non-synthesis sibling.
    #[test]
    fn test_migration_v4_to_v5_rejects_dangling_or_wrong_type_synthesis_memory_id() {
        let conn = seed_v4_db();
        insert_raw_memory(&conn, "src-1", "conclusion", "s1", "2026-04-20T00:00:00Z");
        insert_raw_memory(
            &conn,
            "not-a-synthesis",
            "conclusion",
            "c1",
            "2026-04-20T00:00:00Z",
        );
        // Link a source to a non-synthesis row (wrong-type synthesis_memory_id).
        link_source(&conn, "src-1", "not-a-synthesis");

        let err = run_migrations(&conn).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("synthesis_memory_id"),
            "expected bad-synthesis-ref error, got: {msg}"
        );
        assert!(
            msg.contains("not-a-synthesis"),
            "expected error to name the offending id, got: {msg}"
        );
    }

    #[test]
    fn test_idx_memory_content_hash_is_partial_after_v5() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Check the index definition via sqlite_master.
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_memory_content_hash'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            sql.contains("WHERE source_type != 'synthesis'"),
            "expected partial index excluding synthesis rows, got: {sql}"
        );
    }

    #[test]
    fn test_idx_synthesis_cluster_is_partial_with_not_null_guard() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_synthesis_cluster'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            sql.contains("source_type = 'synthesis'") && sql.contains("IS NOT NULL"),
            "expected partial unique index with IS NOT NULL guard, got: {sql}"
        );
    }

    // ---- Plan F-002 (v6) tests ------------------------------------------

    /// AC1 — fresh `Db::open_in_memory()` ends with both audit tables and
    /// all three indexes from R7/R4.
    #[test]
    fn test_v6_creates_audit_tables_and_indexes() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        for table in ["memory_search_audit", "audit_returned_facts"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {table} missing after v6 migration");
        }

        for idx in [
            "idx_memory_search_audit_searched_id",
            "idx_audit_returned_facts_fact_audit",
            "idx_memory_entries_valid_until_id",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    rusqlite::params![idx],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "index {idx} missing after v6 migration");
        }
    }

    /// AC1 — seed v5 state with one row, drop v6 objects, replay migration,
    /// assert the row survives the v6 step. Modeled on
    /// `test_migration_from_v3_preserves_data`.
    #[test]
    fn test_migration_v5_to_v6_preserves_data() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let hash = compute_content_hash("v5 row content");
        conn.execute(
            "INSERT INTO memory_entries
                (id, project_id, source_file, source_type, knowledge_type,
                 title, content, entities, valid_from, created_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                "row-v5",
                "proj",
                "f.md",
                "conclusion",
                "decisional",
                "v5 row",
                "v5 row content",
                "tag",
                "2026-04-28T00:00:00Z",
                "2026-04-28T00:00:00Z",
                hash,
            ],
        )
        .unwrap();

        // Force replay of the v6 step from a v5 snapshot.
        conn.execute_batch(
            "DROP TABLE IF EXISTS audit_returned_facts;
             DROP TABLE IF EXISTS memory_search_audit;
             DROP INDEX IF EXISTS idx_memory_entries_valid_until_id;
             PRAGMA user_version = 5;",
        )
        .unwrap();

        run_migrations(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 6);

        let title: String = conn
            .query_row(
                "SELECT title FROM memory_entries WHERE id = 'row-v5'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(title, "v5 row");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'memory_search_audit'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "audit table must be re-created");
    }
}
