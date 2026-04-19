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
