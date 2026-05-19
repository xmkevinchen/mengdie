//! MCP tool integration tests — exercise MengdieServer dispatch paths
//! from outside the `db::*` layer (F-013).
//!
//! Each test constructs a fresh `Harness` (in-memory Db + lazily-loaded
//! Embedder). The Embedder loads its ONNX session per Harness (~100-200ms
//! post-cache); the fastembed model is downloaded once per test process
//! via `common::ensure_embedder_warm()` so cold-cache runs cost ~10s once
//! and the rest are subsecond.

mod common;

use common::Harness;
use mengdie::core::mcp_tools::{IngestParams, KnowledgeType, SourceType};

/// Smoke test: ingest a fact through the MCP `memory_ingest` tool, then
/// search for it through `memory_search`, and confirm the round trip
/// passes through the full dispatch path (params parsing → tool body →
/// output shape). This is the F-013 step 2 acceptance — proves the
/// harness wiring works end-to-end before per-tool prefix-path coverage
/// (steps 3-4) is layered on top.
#[tokio::test]
async fn smoke_ingest_then_search() {
    let h = Harness::new();

    // Ingest one fact.
    let ingest = h
        .ingest(IngestParams {
            title: "Auth Decision".to_string(),
            content: "Use JWT tokens with Redis session store for authentication.".to_string(),
            source_file: "".to_string(),
            source_type: SourceType::Conclusion,
            knowledge_type: KnowledgeType::Decisional,
            entities: "auth,jwt,redis".to_string(),
            project_id: None,
            resolves: None,
        })
        .await;
    assert!(
        ingest.error.is_none(),
        "ingest unexpectedly errored: {:?}",
        ingest.error
    );
    assert!(
        !ingest.entry_id.is_empty(),
        "ingest returned empty entry_id"
    );
    let entry_id = ingest.entry_id.clone();

    // Search for it.
    let search = h
        .search(mengdie::core::mcp_tools::SearchParams {
            query: "JWT authentication".to_string(),
            scope: None,
            project_id: None,
            limit: Some(10),
            min_score: None,
        })
        .await;
    assert!(
        search.degraded.is_none(),
        "search degraded unexpectedly: {:?}",
        search.degraded
    );
    assert!(
        !search.results.is_empty(),
        "search returned no results for ingested fact"
    );

    // Verify F-009 contract: short_id present + matches first 8 of id.
    let hit = &search.results[0];
    assert_eq!(hit.id, entry_id);
    assert_eq!(hit.short_id.len(), 8);
    assert_eq!(hit.short_id, &entry_id[..8]);
}
