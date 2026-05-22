//! F-014 / BL-022 — backfill pass for synthesis rows stored with `embedding=NULL`.
//!
//! Pre-fix (before commit `99e26c2`), `run_synthesis_pass` constructed
//! synthesis `NewMemory` with `embedding: None`, leaving rows unreachable
//! via vector search and excluded from clustering. F-008's `memory_lint`
//! `synthesis_null_embedding` check surfaces existing broken rows.
//!
//! This module's one-shot library function repairs them. Idempotent: the
//! WHERE clause `source_type='synthesis' AND embedding IS NULL` naturally
//! gates re-runs to 0 matches once backfill completes.
//!
//! Architecture note: the library function is `sync` (not `async`) because
//! the CLI invocation context is a one-shot batch — the operator expects
//! it to block until done. No `tokio::task::spawn_blocking` wrapper
//! needed here (compare to `run_synthesis_pass` where the embed call sits
//! inside an `async fn` and must offload via spawn_blocking per the
//! `mcp_tools.rs:387,515` convention).

use std::sync::{Arc, Mutex};

use anyhow::Context;
use rusqlite::params;

use crate::core::db::Db;
use crate::core::embeddings::{Embedder, EmbeddingContext};

/// Outcome of a `reembed_synthesis_rows` invocation.
#[derive(Debug, Clone)]
pub struct ReembedResult {
    /// IDs of synthesis rows that were (or, in dry-run, would have been)
    /// re-embedded. Operator can grep this for audit.
    pub affected: Vec<String>,
    /// Whether the call ran in dry-run mode (no writes).
    pub dry_run: bool,
}

/// Re-embed every `source_type='synthesis'` row with `embedding IS NULL`,
/// optionally scoped to a single project.
///
/// - `dry_run = true`: collect affected IDs and return; no `UPDATE`s.
///   `embedder` may be `None` (no fastembed init needed for preview).
/// - `dry_run = false`: requires `Some(embedder)`. Locks per row, calls
///   `embed_with_context`, then `db.store_embedding(id, embedding, dim)`.
///   Returns `Err` if `embedder` is `None` (caller bug).
///
/// Embedding `EmbeddingContext` is constructed per row using its stored
/// `title`, `entities`, `project_id`, and the fixed `knowledge_type =
/// "factual"` (matching `run_synthesis_pass`'s eager-embed path post-F-014
/// so backfilled rows rank identically to freshly-synthesized ones).
///
/// The optional embedder parameter (F-014 review fixup, Codex P2): lets
/// callers skip the ~100-200ms ONNX session-load + potential 90MB
/// fastembed cold download when only the preview is needed.
pub fn reembed_synthesis_rows(
    db: &Db,
    embedder: Option<Arc<Mutex<Embedder>>>,
    project: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<ReembedResult> {
    // Pull affected rows in a single short-lived borrow of the conn, then
    // drop it before the embed loop so we don't hold the conn lock across
    // 2-10ms fastembed inferences (would block any concurrent reader/writer
    // of Db).
    let rows: Vec<(String, String, String, String, String)> = {
        let conn = db
            .lock_conn()
            .context("lock conn for synthesis_null_embedding scan")?;
        // mapper closure inlined per arm — sharing across arms triggers
        // rustc lifetime issues with the per-arm stmt borrow.
        match project {
            Some(pid) => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, entities, content
                       FROM memory_entries
                      WHERE source_type='synthesis'
                        AND embedding IS NULL
                        AND project_id = ?1",
                )?;
                let rows: Result<Vec<_>, _> = stmt
                    .query_map(params![pid], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                            r.get::<_, String>(4)?,
                        ))
                    })?
                    .collect();
                rows?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, title, entities, content
                       FROM memory_entries
                      WHERE source_type='synthesis'
                        AND embedding IS NULL",
                )?;
                let rows: Result<Vec<_>, _> = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                            r.get::<_, String>(4)?,
                        ))
                    })?
                    .collect();
                rows?
            }
        }
    };

    let mut affected = Vec::with_capacity(rows.len());
    if dry_run {
        for (id, _, _, _, _) in rows {
            affected.push(id);
        }
        return Ok(ReembedResult {
            affected,
            dry_run: true,
        });
    }

    // Live mode requires an embedder. Caller bug if None.
    let embedder = embedder.ok_or_else(|| {
        anyhow::anyhow!("reembed_synthesis_rows requires Some(embedder) when dry_run=false")
    })?;

    for (id, project_id, title, entities, content) in rows {
        let ctx = EmbeddingContext {
            knowledge_type: "factual".to_string(),
            entities: entities.clone(),
            project_id: project_id.clone(),
            title: title.clone(),
        };
        let embedding = {
            let mut emb = embedder
                .lock()
                .map_err(|e| anyhow::anyhow!("embedder lock poisoned: {e}"))?;
            emb.embed_with_context(&content, &ctx)
                .with_context(|| format!("embed_with_context failed for synthesis row {id}"))?
        };
        let dim = embedding.len();
        db.store_embedding(&id, &embedding, dim)
            .with_context(|| format!("store_embedding failed for synthesis row {id}"))?;
        affected.push(id);
    }

    Ok(ReembedResult {
        affected,
        dry_run: false,
    })
}
