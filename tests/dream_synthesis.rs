//! End-to-end integration test for `run_synthesis_pass` against a live
//! `claude` CLI.
//!
//! `#[ignore]` by default — requires:
//! - An authenticated `claude` CLI on `$PATH`.
//! - Network access to Anthropic's API.
//! - Real LLM usage (non-zero cost).
//!
//! Run with:
//!     cargo test --test dream_synthesis -- --ignored
//!
//! Unit tests in `src/core/dreaming.rs` cover the synthesis-pass logic via
//! stub LlmProvider implementations and run on every `cargo test`. This file
//! is reserved for the single "does it actually work against a real model?"
//! check required by plan 010 AC4.

use tokio::process::Command;

use std::sync::{Arc, Mutex};

use mengdie::core::config::{LlmConfig, MengdieConfig};
use mengdie::core::db::{Db, NewMemory};
use mengdie::core::dreaming::run_synthesis_pass;
use mengdie::core::embeddings::{embedding_to_blob, Embedder};
use mengdie::core::llm::build_provider;

async fn claude_on_path() -> bool {
    Command::new("which")
        .arg("claude")
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

fn make_384d(base: &[f32], nudge: f32) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    for (i, &b) in base.iter().enumerate() {
        v[i] = b;
    }
    v[3] = nudge;
    v
}

fn seed_memory(db: &Db, project: &str, title: &str, content: &str, emb: &[f32]) -> String {
    db.insert_memory(NewMemory {
        project_id: project.to_string(),
        source_file: format!("{title}-{}.md", uuid::Uuid::new_v4()),
        source_type: "conclusion".to_string(),
        knowledge_type: "decisional".to_string(),
        title: title.to_string(),
        content: content.to_string(),
        entities: "test,bl-007-e2e".to_string(),
        embedding: Some(embedding_to_blob(emb)),
        embedding_dim: Some(emb.len() as i64),
        is_longterm: false,
    })
    .unwrap()
}

#[tokio::test]
#[ignore = "requires authenticated claude CLI on PATH; run with --ignored dream_synthesis"]
async fn end_to_end_dream_synthesis_writes_one_row_with_six_links() {
    if !claude_on_path().await {
        eprintln!("[SKIP] `claude` binary not found on PATH; skipping e2e test.");
        return;
    }

    // 6 near-identical 384-dim embeddings in one project — one tight cluster.
    let db = Db::open_in_memory().unwrap();
    let mut source_ids: Vec<String> = Vec::with_capacity(6);
    for i in 0..6 {
        let emb = make_384d(&[1.0, 0.0, 0.0], 0.001 * i as f32);
        let id = seed_memory(
            &db,
            "e2e-proj",
            &format!("Sync decision {i}"),
            &format!(
                "Decision {i}: use semaphore-bounded workers instead of unbounded spawn. \
                 Prevents runaway task queues under burst load. Applied consistently \
                 across worker pool init in service-a, service-b, and service-c."
            ),
            &emb,
        );
        source_ids.push(id);
    }

    // Use the default model (claude-sonnet-4-6 per config defaults).
    let cfg = MengdieConfig::default();
    let llm_cfg: LlmConfig = cfg.llm;
    let provider = build_provider(&llm_cfg).expect("provider construction should succeed");

    // BL-022 / F-014: synthesis pass now requires an embedder (Arc<Mutex<Embedder>>)
    // so it can populate synthesis rows' embedding columns. The integration test
    // already requires real network / claude CLI, so the ~90MB fastembed model
    // download (first run) is acceptable additional cost.
    let embedder = Arc::new(Mutex::new(
        Embedder::new().expect("Embedder::new failed in dream_synthesis e2e"),
    ));

    let result = run_synthesis_pass(
        &db,
        Some("e2e-proj"),
        provider.as_ref(),
        Arc::clone(&embedder),
        0.9,   // tight threshold — 6 should still cluster with small noise
        3,     // min_size
        20,    // max_cluster_size
        false, // dry_run = false, real LLM call
    )
    .await
    .expect("synthesis pass should succeed");

    assert_eq!(result.clusters_processed, 1);
    assert_eq!(
        result.syntheses_created,
        1,
        "expected exactly 1 synthesis row; got {}, llm_errors={}",
        result.syntheses_created,
        result.llm_errors()
    );
    assert_eq!(result.llm_call_errors, 0);
    assert_eq!(result.parse_errors, 0);
    // Plan 012 pair-cluster counters: the 6-memory cluster is a triple+,
    // not a pair, so both pair-cluster counters MUST be 0. Locks the
    // field semantics into integration coverage (review feedback from
    // challenger D: public API fields need integration-test exercise).
    assert_eq!(
        result.pair_clusters_processed, 0,
        "6-memory cluster is not a pair"
    );
    assert_eq!(result.pair_clusters_skipped, 0);

    // Verify the synthesis row shape.
    let syns: Vec<_> = db
        .list_memories(Some("e2e-proj"))
        .unwrap()
        .into_iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();
    assert_eq!(syns.len(), 1, "expected exactly one synthesis row");
    let syn = &syns[0];

    assert!(
        !syn.title.trim().is_empty(),
        "synthesis title must not be empty"
    );
    assert!(
        !syn.content.trim().is_empty(),
        "synthesis content must not be empty"
    );
    assert!(
        !syn.is_longterm,
        "synthesis should default to is_longterm=false; earned via dreaming"
    );

    // Verify 6 link rows.
    let link_count = db.count_synthesis_links(&syn.id).unwrap();
    assert_eq!(
        link_count, 6,
        "expected 6 memory_synthesis_links rows for the synthesis"
    );

    // Print PASS + model + first 40 chars of title (plan AC4 writeback).
    let model = llm_cfg.model;
    let title_head: String = syn.title.chars().take(40).collect();
    eprintln!("[PASS] model={model} title[:40]={title_head:?}");
}

// ---- Plan 017 Step 3: synthesis-audit subcommand (no LLM required) --------

use std::process::Command as StdCommand;

/// Seed a synthesis + N source memories into the given DB via direct
/// `insert_synthesis_with_links` (no LLM call).
fn seed_synthesis_for_audit(db: &Db) -> String {
    let src1 = db
        .insert_memory(NewMemory {
            project_id: "audit-test".to_string(),
            source_file: "src-a.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "First source title".to_string(),
            content: "First source content — about X.".to_string(),
            entities: "x".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .unwrap();
    let src2 = db
        .insert_memory(NewMemory {
            project_id: "audit-test".to_string(),
            source_file: "src-b.md".to_string(),
            source_type: "review".to_string(),
            knowledge_type: "experiential".to_string(),
            title: "Second source title".to_string(),
            content: "Second source content — about Y.".to_string(),
            entities: "y".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .unwrap();

    db.insert_synthesis_with_links(
        NewMemory {
            project_id: "audit-test".to_string(),
            source_file: "syn.md".to_string(),
            source_type: "synthesis".to_string(),
            knowledge_type: "factual".to_string(),
            title: "Audit fixture synthesis".to_string(),
            content: "Synthesized content mentioning X and Y.".to_string(),
            entities: "x,y".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        },
        &[src1, src2],
    )
    .unwrap()
}

#[test]
fn synthesis_audit_subcommand_prints_synthesis_and_sources() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    let syn_id = {
        let db = Db::open(&db_path).unwrap();
        seed_synthesis_for_audit(&db)
    };

    let output = StdCommand::new(env!("CARGO_BIN_EXE_mengdie"))
        .args([
            "--db-path",
            db_path.to_str().unwrap(),
            "synthesis-audit",
            &syn_id,
        ])
        .output()
        .expect("spawn mengdie");

    assert!(
        output.status.success(),
        "synthesis-audit exit={:?}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(
        stdout.contains("=== Synthesis ==="),
        "missing header: {stdout}"
    );
    assert!(
        stdout.contains("Audit fixture synthesis"),
        "missing synthesis title: {stdout}"
    );
    assert!(
        stdout.contains("Sources (2)"),
        "missing source count: {stdout}"
    );
    assert!(
        stdout.contains("First source title"),
        "missing source 1: {stdout}"
    );
    assert!(
        stdout.contains("Second source title"),
        "missing source 2: {stdout}"
    );
    assert!(
        stdout.contains("Type:   conclusion"),
        "missing source 1 type: {stdout}"
    );
    assert!(
        stdout.contains("Type:   review"),
        "missing source 2 type: {stdout}"
    );

    drop(tmp);
}

#[test]
fn synthesis_audit_subcommand_errors_on_non_synthesis_id() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    let src_id = {
        let db = Db::open(&db_path).unwrap();
        db.insert_memory(NewMemory {
            project_id: "audit-test".to_string(),
            source_file: "only.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "A primary source, not a synthesis".to_string(),
            content: "content".to_string(),
            entities: "".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .unwrap()
    };

    let output = StdCommand::new(env!("CARGO_BIN_EXE_mengdie"))
        .args([
            "--db-path",
            db_path.to_str().unwrap(),
            "synthesis-audit",
            &src_id,
        ])
        .output()
        .expect("spawn mengdie");

    assert!(
        !output.status.success(),
        "expected non-zero exit for non-synthesis id"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a synthesis row"),
        "expected clear error message, got stderr: {stderr}"
    );

    drop(tmp);
}

// ---- Plan 017 Step 5: cluster-hash dedup regression tests at integration
// level (complements the src/core/db.rs unit tests with external-crate-level
// coverage; uses the COUNT=1-per-cluster-per-project invariant, not
// id-equality — robust against future ON CONFLICT refactors per plan 017
// challenger P1). --------------------------------------------------------

fn seed_three_source_memories(db: &Db, project: &str) -> Vec<String> {
    (0..3)
        .map(|i| {
            db.insert_memory(NewMemory {
                project_id: project.to_string(),
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

fn synthesis_new_memory(project: &str, content: &str) -> NewMemory {
    NewMemory {
        project_id: project.to_string(),
        source_file: "syn.md".to_string(),
        source_type: "synthesis".to_string(),
        knowledge_type: "factual".to_string(),
        title: "Synthesis fixture".to_string(),
        content: content.to_string(),
        entities: "tag".to_string(),
        embedding: None,
        embedding_dim: None,
        is_longterm: false,
    }
}

#[test]
fn cluster_hash_dedup_survives_prompt_change_integration() {
    let db = Db::open_in_memory().unwrap();
    let sources = seed_three_source_memories(&db, "p017-int-1");

    db.insert_synthesis_with_links(synthesis_new_memory("p017-int-1", "V1 content"), &sources)
        .unwrap();
    // Simulate a SYSTEM_PROMPT edit by re-synthesizing with the same source
    // set but different LLM output text.
    db.insert_synthesis_with_links(synthesis_new_memory("p017-int-1", "V2 content"), &sources)
        .unwrap();

    // COUNT=1-per-cluster-per-project invariant — NOT id-equality (which is
    // incidentally true for ON CONFLICT DO UPDATE but would break under a
    // future INSERT OR REPLACE refactor; the COUNT invariant is stable).
    let syns: Vec<_> = db
        .list_memories(Some("p017-int-1"))
        .unwrap()
        .into_iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();
    assert_eq!(
        syns.len(),
        1,
        "prompt change must UPDATE existing synthesis row, not create zombie"
    );
    assert_eq!(syns[0].content, "V2 content", "latest content wins");
}

#[test]
fn different_source_sets_with_identical_content_coexist_integration() {
    // Cross-cluster coexistence — exercises the Step 1 partial-index fix
    // (`idx_memory_content_hash WHERE source_type != 'synthesis'`). Without
    // the WHERE predicate, two syntheses with identical content text would
    // conflict on content_hash even though their clusters differ.
    let db = Db::open_in_memory().unwrap();
    let sources = seed_three_source_memories(&db, "p017-int-2");

    let subset_a = vec![sources[0].clone(), sources[1].clone()];
    let subset_b = vec![sources[1].clone(), sources[2].clone()];

    // Deliberately identical content text to exercise the content_hash
    // partial-index branch.
    db.insert_synthesis_with_links(
        synthesis_new_memory("p017-int-2", "Identical text"),
        &subset_a,
    )
    .unwrap();
    db.insert_synthesis_with_links(
        synthesis_new_memory("p017-int-2", "Identical text"),
        &subset_b,
    )
    .unwrap();

    let syns: Vec<_> = db
        .list_memories(Some("p017-int-2"))
        .unwrap()
        .into_iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();
    assert_eq!(
        syns.len(),
        2,
        "two different clusters with identical content must coexist (Step 1 partial-index fix)"
    );
}

#[test]
fn cluster_hash_stable_across_source_id_order_integration() {
    let db = Db::open_in_memory().unwrap();
    let sources = seed_three_source_memories(&db, "p017-int-3");

    db.insert_synthesis_with_links(
        synthesis_new_memory("p017-int-3", "same content"),
        &[sources[0].clone(), sources[1].clone(), sources[2].clone()],
    )
    .unwrap();
    db.insert_synthesis_with_links(
        synthesis_new_memory("p017-int-3", "same content"),
        &[sources[2].clone(), sources[0].clone(), sources[1].clone()],
    )
    .unwrap();

    let syns: Vec<_> = db
        .list_memories(Some("p017-int-3"))
        .unwrap()
        .into_iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();
    assert_eq!(
        syns.len(),
        1,
        "source_id order must not affect cluster identity"
    );
}

#[test]
fn synthesis_audit_subcommand_errors_on_unknown_id() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    let _db = Db::open(&db_path).unwrap();

    let output = StdCommand::new(env!("CARGO_BIN_EXE_mengdie"))
        .args([
            "--db-path",
            db_path.to_str().unwrap(),
            "synthesis-audit",
            "definitely-not-a-real-id",
        ])
        .output()
        .expect("spawn mengdie");

    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown id"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "expected 'not found' error, got: {stderr}"
    );

    drop(tmp);
}

/// Plan 017 /ae:review P2-D (challenger F8): `get_synthesis_with_sources`
/// must return a placeholder `MemoryEntry` for any source that has been
/// hard-deleted, rather than aborting the audit. Plan Step 3 explicitly
/// required this test coverage but the original commit shipped without it.
///
/// Tested at the library level (`Db::get_synthesis_with_sources`) rather
/// than via subprocess — the placeholder fields are what matters, and the
/// subprocess-wired `cmd_synthesis_audit` prints `<deleted:` titles
/// verbatim (verified by reading cli.rs cmd_synthesis_audit).
#[test]
fn get_synthesis_with_sources_returns_placeholder_for_deleted_source() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    let db = Db::open(&db_path).unwrap();

    // Seed a synthesis + 2 sources.
    let syn_id = seed_synthesis_for_audit(&db);

    // Find a source id (any) and hard-delete it via raw SQL through
    // the existing library surface. `Db::list_memories` returns primary
    // sources (it filters by source_type under the hood via project scope).
    let memories = db.list_memories(Some("audit-test")).unwrap();
    let a_source = memories
        .iter()
        .find(|m| m.source_type != "synthesis")
        .expect("at least one primary source must exist");
    let deleted_id = a_source.id.clone();

    // Raw hard-delete — not an API Db exposes, because we intentionally
    // DO NOT want to support hard deletion as a first-class op. Use the
    // available library surface: invalidate + audit. For this test we
    // really need a hard-delete to exercise the placeholder path, so
    // fall back to re-opening a raw Connection.
    {
        use rusqlite::Connection;
        let conn = Connection::open(&db_path).unwrap();
        // rusqlite `bundled` defaults FK ON for a fresh Connection::open,
        // whereas `Db::open` doesn't set the PRAGMA (BL-enable-pragma-foreign-keys).
        // Disable explicitly so we can seed the hard-deleted state this test
        // needs to exercise — mirroring the production risk window where an
        // orphan-link condition could exist on a live DB.
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "DELETE FROM memory_entries WHERE id = ?1",
            rusqlite::params![deleted_id],
        )
        .unwrap();
    }

    // Now fetch the synthesis + sources — the deleted source must surface
    // as a placeholder.
    let (syn, sources) = db.get_synthesis_with_sources(&syn_id).unwrap();
    assert_eq!(syn.id, syn_id);
    assert_eq!(sources.len(), 2, "both linked sources must be returned");

    let placeholder = sources
        .iter()
        .find(|s| s.id == deleted_id)
        .expect("deleted source must appear as placeholder");
    assert_eq!(
        placeholder.source_type, "<deleted>",
        "placeholder must carry source_type = '<deleted>'"
    );
    assert!(
        placeholder.title.starts_with("<deleted: "),
        "placeholder title must signal deletion, got: {}",
        placeholder.title
    );
    assert!(
        placeholder.content.is_empty(),
        "placeholder content must be empty (no leaked stale row)"
    );
    assert_eq!(placeholder.recall_count, 0);
    assert_eq!(placeholder.avg_relevance, 0.0);

    // The non-deleted source must still render as a real row.
    let real = sources
        .iter()
        .find(|s| s.id != deleted_id)
        .expect("the other source must still be present");
    assert_ne!(real.source_type, "<deleted>");
    assert!(!real.content.is_empty());

    drop(tmp);
}

// ============================================================================
// F-014 / BL-022 — AC1: synthesis rows have non-NULL embedding post-pass
// ============================================================================
//
// Unit-test-style integration test for the eager-embed fix. Uses a local
// FixedProvider stub (no live `claude` CLI required) so this runs in CI
// without `--ignored`. The embedder is real `Embedder::new()` per the
// codebase convention (~90MB cached fastembed model; ~100-200ms ONNX
// load post-cache).

use std::sync::atomic::{AtomicUsize, Ordering};

use mengdie::core::llm::{LlmFuture, LlmProvider};

/// Minimal LlmProvider stub that returns a fixed synthesis JSON for every
/// call. Mirrors the pattern from `src/core/dreaming.rs::tests::FixedProvider`
/// (cannot import from a `#[cfg(test)] mod tests` block into an integration
/// test; intentional duplication).
struct FixedSynthesisProvider {
    payload: String,
    call_count: AtomicUsize,
}

impl FixedSynthesisProvider {
    fn new(payload: impl Into<String>) -> Self {
        Self {
            payload: payload.into(),
            call_count: AtomicUsize::new(0),
        }
    }
}

impl LlmProvider for FixedSynthesisProvider {
    fn complete<'a>(&'a self, _system: &'a str, _prompt: &'a str) -> LlmFuture<'a> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let payload = self.payload.clone();
        Box::pin(async move { Ok(payload) })
    }
    fn complete_structured<'a>(
        &'a self,
        system: &'a str,
        prompt: &'a str,
        _schema: &'a str,
    ) -> LlmFuture<'a> {
        self.complete(system, prompt)
    }
    fn model(&self) -> &str {
        "stub-fixed-synthesis"
    }
}

/// AC1: after `run_synthesis_pass` produces a synthesis row, that row has
/// `embedding IS NOT NULL` and `embedding_dim = 384` (BL-022 regression
/// guard — pre-fix the row was stored with `embedding=None`).
#[tokio::test]
async fn synthesis_rows_have_non_null_embedding_after_pass() {
    let db = Db::open_in_memory().unwrap();

    // Seed 3 source memories with near-identical embeddings → one tight cluster.
    // Real embeddings (small nudge per row) so `cluster_memories` finds them
    // at threshold 0.9.
    let project = "ac1-proj";
    for i in 0..3 {
        let emb = make_384d(&[1.0, 0.0, 0.0], 0.001 * i as f32);
        seed_memory(
            &db,
            project,
            &format!("Source {i}"),
            &format!(
                "Source memory {i}: synthesis-eligible content for the F-014 AC1 test. \
                 Repeated entity tags so clustering groups them at threshold."
            ),
            &emb,
        );
    }

    // Stub provider returns a fixed synthesis JSON — no live LLM needed.
    const SYNTHESIS_JSON: &str = r#"{
        "title": "AC1 Synthesis",
        "content": "Synthesized fact derived from 3 source memories.",
        "entities": ["bl-022-ac1", "synthesis"],
        "source_memory_ids": []
    }"#;
    let provider = FixedSynthesisProvider::new(SYNTHESIS_JSON);

    let embedder = Arc::new(Mutex::new(
        Embedder::new().expect("Embedder::new failed in AC1 test"),
    ));

    let result = run_synthesis_pass(
        &db,
        Some(project),
        &provider,
        Arc::clone(&embedder),
        0.9,
        3,
        20,
        false, // dry_run = false — must write
    )
    .await
    .expect("synthesis pass should succeed");

    // Defensive guard: zero syntheses_created would silently pass the SQL
    // assertion below (no rows to check). Fail loudly if the cluster didn't
    // produce anything — the test setup is broken in that case.
    assert!(
        result.syntheses_created > 0,
        "expected at least 1 synthesis row; got 0 (clusters_processed={}, llm_errors={}, parse_errors={})",
        result.clusters_processed,
        result.llm_call_errors,
        result.parse_errors
    );

    // BL-022 / AC1 core assertion: every synthesis row in this project has
    // a non-NULL embedding of the expected dimension. Use `db.list_memories`
    // (pub API) rather than reaching into pub(crate) `lock_conn`.
    let all_rows = db
        .list_memories(Some(project))
        .expect("list_memories should succeed");
    let synthesis_rows: Vec<&mengdie::core::db::MemoryEntry> = all_rows
        .iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();

    assert!(
        !synthesis_rows.is_empty(),
        "synthesis row(s) should exist in the project after run_synthesis_pass"
    );

    for row in &synthesis_rows {
        assert!(
            row.embedding.is_some(),
            "synthesis row {} has NULL embedding (BL-022 regression — F-014 fix not applied)",
            row.id
        );
        assert_eq!(
            row.embedding_dim,
            Some(384),
            "synthesis row {} has embedding_dim={:?}, expected Some(384) (all-MiniLM-L6-v2 dim)",
            row.id,
            row.embedding_dim
        );
    }
}
