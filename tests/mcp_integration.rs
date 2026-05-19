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
use mengdie::core::db::NewMemory;
use mengdie::core::mcp_tools::{
    GetParams, IngestParams, InvalidateParams, KnowledgeType, SearchParams, SourceType,
};

/// Build a minimal NewMemory (for direct `Db::insert_memory_with_id` calls).
/// Avoids re-typing the fields in every test.
fn sample_new_memory(project_id: &str, title: &str, content: &str) -> NewMemory {
    NewMemory {
        project_id: project_id.to_string(),
        source_file: format!("docs/{}.md", title.replace(' ', "-")),
        source_type: "conclusion".to_string(),
        knowledge_type: "decisional".to_string(),
        title: title.to_string(),
        content: content.to_string(),
        entities: "test".to_string(),
        embedding: None,
        embedding_dim: None,
        is_longterm: false,
    }
}

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
        .search(SearchParams {
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

// =====================================================================
// F-009 retroactive coverage: memory_invalidate prefix dispatch paths
// =====================================================================

#[tokio::test]
async fn invalidate_full_uuid_success() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
            .unwrap();
    assert_eq!(id.len(), 36, "UUID v4 string should be 36 chars");

    let out = h
        .invalidate(InvalidateParams {
            entry_id: id.clone(),
            reason: "test".to_string(),
            superseded_by: None,
        })
        .await;
    assert!(
        out.error.is_none(),
        "expected success, got: {:?}",
        out.error
    );
    assert!(out.success);
    assert_eq!(out.entry_id, id);
}

#[tokio::test]
async fn invalidate_unique_prefix_success() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
            .unwrap();
    let prefix = id[..8].to_string();

    let out = h
        .invalidate(InvalidateParams {
            entry_id: prefix.clone(),
            reason: "test".to_string(),
            superseded_by: None,
        })
        .await;
    assert!(
        out.error.is_none(),
        "expected unique-prefix success, got: {:?}",
        out.error
    );
    assert!(out.success);
    assert_eq!(
        out.entry_id, id,
        "resolved_id should be the full UUID, not the prefix"
    );
}

#[tokio::test]
async fn invalidate_prefix_too_short() {
    let h = Harness::new();
    h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
        .unwrap();

    let out = h
        .invalidate(InvalidateParams {
            entry_id: "abc".to_string(), // 3 chars — below 8-char minimum
            reason: "test".to_string(),
            superseded_by: None,
        })
        .await;
    assert!(!out.success);
    let err = out.error.expect("expected too-short error");
    assert!(
        err.contains("too short"),
        "error should mention 'too short', got: {err}"
    );
    assert!(
        err.contains("8 chars"),
        "error should mention '8 chars', got: {err}"
    );
}

#[tokio::test]
async fn invalidate_prefix_no_match() {
    let h = Harness::new();
    h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
        .unwrap();

    let out = h
        .invalidate(InvalidateParams {
            // 8+ chars but not a valid hex prefix of any inserted UUID
            entry_id: "zzzzzzzz".to_string(),
            reason: "test".to_string(),
            superseded_by: None,
        })
        .await;
    assert!(!out.success);
    let err = out.error.expect("expected no-match error");
    assert!(
        err.contains("No memory matches prefix"),
        "error should announce no match, got: {err}"
    );
    assert!(
        err.contains("'zzzzzzzz'"),
        "error should quote the prefix, got: {err}"
    );
    assert!(
        err.contains("in project 'test-project'"),
        "error should mention the scoped project, got: {err}"
    );
}

#[tokio::test]
async fn invalidate_prefix_collision() {
    let h = Harness::new();
    // Manually construct 2 UUIDs sharing first 8 chars to force collision.
    let id_a = "deadbeef-aaaa-4bbb-cccc-dddddddddddd".to_string();
    let id_b = "deadbeef-bbbb-4ccc-dddd-eeeeeeeeeeee".to_string();
    h.db.insert_memory_with_id(
        &id_a,
        sample_new_memory("test-project", "A", "Content for A"),
    )
    .unwrap();
    h.db.insert_memory_with_id(
        &id_b,
        sample_new_memory("test-project", "B", "Content for B"),
    )
    .unwrap();

    let out = h
        .invalidate(InvalidateParams {
            entry_id: "deadbeef".to_string(), // 8 chars, shared prefix
            reason: "test".to_string(),
            superseded_by: None,
        })
        .await;
    assert!(!out.success);
    let err = out.error.expect("expected collision error");
    assert!(
        err.contains("ambiguous"),
        "error should call it ambiguous, got: {err}"
    );
    assert!(
        err.contains("matches at least:"),
        "error should list matches, got: {err}"
    );
    // Both ids should appear (cap-at-2 contract from F-009 find_by_id_prefix).
    assert!(err.contains(&id_a) || err.contains(&id_b));
    assert!(
        err.contains("extend prefix"),
        "error should suggest extending, got: {err}"
    );
}

// =====================================================================
// F-010 retroactive coverage: memory_get prefix dispatch paths
// =====================================================================

#[tokio::test]
async fn get_full_uuid_success() {
    let h = Harness::new();
    let id = h
        .db
        .insert_memory(sample_new_memory(
            "test-project",
            "Long Content Fact",
            "This is a much longer content body that exceeds the 200-char search snippet boundary so we can confirm memory_get returns the full text not the snippet. Padding padding padding padding padding padding padding padding padding.",
        ))
        .unwrap();

    let out = h
        .get(GetParams {
            memory_id: id.clone(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(
        out.error.is_none(),
        "expected success, got: {:?}",
        out.error
    );
    let entry = out.entry.expect("expected populated entry");
    assert_eq!(entry.id, id);
    assert_eq!(entry.short_id, &id[..8]);
    assert!(
        entry.content.len() > 200,
        "expected full content (>200 chars), got {} chars",
        entry.content.len()
    );
}

#[tokio::test]
async fn get_unique_prefix_success() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
            .unwrap();
    let prefix = id[..8].to_string();

    let out = h
        .get(GetParams {
            memory_id: prefix,
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    let entry = out.entry.expect("expected populated entry");
    assert_eq!(entry.id, id, "resolved to full UUID via prefix");
}

#[tokio::test]
async fn get_prefix_too_short() {
    let h = Harness::new();
    h.db.insert_memory(sample_new_memory("test-project", "T1", "C1"))
        .unwrap();

    let out = h
        .get(GetParams {
            memory_id: "abc".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.entry.is_none());
    let err = out.error.expect("expected too-short error");
    assert!(err.contains("too short") && err.contains("8 chars"));
}

#[tokio::test]
async fn get_cross_project_blocked_by_default() {
    let h = Harness::new();
    // Ingest into a DIFFERENT project (not the harness default).
    let id =
        h.db.insert_memory(sample_new_memory("other-project", "X", "Y"))
            .unwrap();

    // Get without scope=global should fail: prefix is scoped to default
    // project, so the fact in 'other-project' is not visible.
    let out = h
        .get(GetParams {
            memory_id: id[..8].to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.entry.is_none());
    let err = out.error.expect("expected scope-mismatch error");
    assert!(
        err.contains("No memory matches prefix"),
        "scoped prefix lookup should report no-match within current project, got: {err}"
    );
    // F-010 review fixup: prefix-path no-match should hint at scope=global
    // for parity with full-UUID cross-project guard's remediation hint.
    assert!(
        err.contains("scope='global'"),
        "no-match error should suggest scope=global remediation, got: {err}"
    );
}

#[tokio::test]
async fn get_full_uuid_cross_project_blocked_by_default() {
    // F-010 review fixup: AC5 had a test gap — only the prefix path was
    // exercised for cross-project blocking; the full-UUID path's
    // "belongs to project X, not Y" guard had zero coverage. A future
    // refactor that moved get_memory inside the resolved-id branch could
    // silently break it.
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("other-project", "X", "Y"))
            .unwrap();

    // Full 36-char UUID takes the fast path, skipping prefix lookup;
    // the cross-project guard fires AFTER db::get_memory returns the row.
    let out = h
        .get(GetParams {
            memory_id: id.clone(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.entry.is_none());
    let err = out.error.expect("expected belongs-to-other-project error");
    assert!(
        err.contains("belongs to project 'other-project'"),
        "full-UUID guard should name the offending project, got: {err}"
    );
    assert!(
        err.contains("not 'test-project'"),
        "full-UUID guard should name the requested project, got: {err}"
    );
    assert!(
        err.contains("scope='global'"),
        "full-UUID guard should suggest scope=global remediation, got: {err}"
    );
}

#[tokio::test]
async fn get_scope_global_allows_cross_project() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("other-project", "X", "Y"))
            .unwrap();

    // scope=global lifts the project scoping so the cross-project fact resolves.
    let out = h
        .get(GetParams {
            memory_id: id[..8].to_string(),
            project_id: None,
            scope: Some("global".to_string()),
        })
        .await;
    assert!(
        out.error.is_none(),
        "expected success, got: {:?}",
        out.error
    );
    let entry = out.entry.expect("expected populated entry");
    assert_eq!(entry.id, id);
    assert_eq!(entry.project_id, "other-project");
}

#[tokio::test]
async fn get_bumps_recall_count_only_not_avg_relevance() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "T", "C"))
            .unwrap();
    let before = h.db.get_memory(&id).unwrap().unwrap();
    assert_eq!(before.recall_count, 0);
    let pre_avg = before.avg_relevance;

    // Call get twice — should bump count to 2, leave avg_relevance untouched.
    for _ in 0..2 {
        let _ = h
            .get(GetParams {
                memory_id: id.clone(),
                project_id: None,
                scope: None,
            })
            .await;
    }

    let after = h.db.get_memory(&id).unwrap().unwrap();
    assert_eq!(
        after.recall_count, 2,
        "memory_get should bump count per call"
    );
    assert!(after.last_recalled.is_some(), "last_recalled should be set");
    assert_eq!(
        after.avg_relevance, pre_avg,
        "avg_relevance MUST NOT change — direct lookup has no relevance signal"
    );
}
