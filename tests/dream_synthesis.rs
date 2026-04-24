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

use mengdie::core::config::{LlmConfig, MengdieConfig};
use mengdie::core::db::{Db, NewMemory};
use mengdie::core::dreaming::run_synthesis_pass;
use mengdie::core::embeddings::embedding_to_blob;
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

    let result = run_synthesis_pass(
        &db,
        Some("e2e-proj"),
        provider.as_ref(),
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
