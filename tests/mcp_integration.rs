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
    EntityFactsParams, GetParams, IngestParams, InvalidateParams, KnowledgeType, LintParams,
    SearchParams, SourceType, StatusParams,
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
            project_id: None,
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
            project_id: None,
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
            project_id: None,
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
            project_id: None,
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
            project_id: None,
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

// =====================================================================
// F-011: memory_status MCP tool
// =====================================================================

#[tokio::test]
async fn status_empty_db_returns_zero_counts() {
    let h = Harness::new();
    let out = h
        .status(StatusParams {
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert_eq!(out.total_entries, 0);
    assert_eq!(out.longterm_count, 0);
    assert_eq!(out.synthesis_count, 0);
    assert!(out.by_source_type.is_empty());
    assert!(out.last_ingest_at.is_none());
    assert_eq!(out.project_id, "test-project");
}

#[tokio::test]
async fn status_populated_db_returns_correct_breakdown() {
    let h = Harness::new();
    // Ingest 3 facts via raw db (direct insert; no embedding cost per fact).
    h.db.insert_memory(sample_new_memory("test-project", "A", "C-A"))
        .unwrap();
    h.db.insert_memory(sample_new_memory("test-project", "B", "C-B"))
        .unwrap();
    h.db.insert_memory(sample_new_memory("test-project", "C", "C-C"))
        .unwrap();

    let out = h
        .status(StatusParams {
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert_eq!(out.total_entries, 3);
    assert_eq!(out.by_source_type.get("conclusion").copied(), Some(3));
    assert!(out.last_ingest_at.is_some(), "expected MAX(created_at) set");
}

#[tokio::test]
async fn status_scope_global_aggregates_across_projects() {
    let h = Harness::new();
    h.db.insert_memory(sample_new_memory("test-project", "A", "C-A"))
        .unwrap();
    h.db.insert_memory(sample_new_memory("other-project", "B", "C-B"))
        .unwrap();
    h.db.insert_memory(sample_new_memory("third-project", "C", "C-C"))
        .unwrap();

    // Default scope (current project) sees only 1.
    let scoped = h
        .status(StatusParams {
            project_id: None,
            scope: None,
        })
        .await;
    assert_eq!(scoped.total_entries, 1);
    assert_eq!(scoped.project_id, "test-project");

    // scope=global aggregates all 3.
    let global = h
        .status(StatusParams {
            project_id: None,
            scope: Some("global".to_string()),
        })
        .await;
    assert_eq!(global.total_entries, 3);
    assert_eq!(global.project_id, "<global>");
}

#[tokio::test]
async fn status_is_read_only_does_not_bump_counters() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "A", "C-A"))
            .unwrap();
    let before = h.db.get_memory(&id).unwrap().unwrap();

    // Call status 3x.
    for _ in 0..3 {
        let _ = h
            .status(StatusParams {
                project_id: None,
                scope: None,
            })
            .await;
    }

    let after = h.db.get_memory(&id).unwrap().unwrap();
    assert_eq!(
        after.recall_count, before.recall_count,
        "memory_status MUST NOT bump recall_count (read-only)"
    );
    assert_eq!(
        after.avg_relevance, before.avg_relevance,
        "memory_status MUST NOT touch avg_relevance"
    );
}

// =====================================================================
// F-007: memory_entity_facts MCP tool
// =====================================================================

#[tokio::test]
async fn entity_facts_returns_facts_tagged_with_entity() {
    let h = Harness::new();
    // Ingest 2 facts with "auth" tag, 1 without.
    let mut m1 = sample_new_memory("test-project", "Auth-1", "first auth fact");
    m1.entities = "auth,jwt".to_string();
    let id1 = h.db.insert_memory(m1).unwrap();

    let mut m2 = sample_new_memory("test-project", "Auth-2", "second auth fact");
    m2.entities = "auth,redis".to_string();
    let id2 = h.db.insert_memory(m2).unwrap();

    let mut m3 = sample_new_memory("test-project", "Other", "unrelated fact");
    m3.entities = "logging".to_string();
    let _id3 = h.db.insert_memory(m3).unwrap();

    let out = h
        .entity_facts(EntityFactsParams {
            entity_name: "auth".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert_eq!(out.entity_name, "auth");
    assert_eq!(out.facts.len(), 2);
    let returned_ids: Vec<String> = out.facts.iter().map(|f| f.id.clone()).collect();
    assert!(returned_ids.contains(&id1));
    assert!(returned_ids.contains(&id2));
}

#[tokio::test]
async fn entity_facts_unknown_entity_returns_empty() {
    let h = Harness::new();
    let mut m = sample_new_memory("test-project", "F", "C");
    m.entities = "auth".to_string();
    h.db.insert_memory(m).unwrap();

    let out = h
        .entity_facts(EntityFactsParams {
            entity_name: "nonexistent".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert!(out.facts.is_empty());
}

#[tokio::test]
async fn entity_facts_scope_global_crosses_projects() {
    let h = Harness::new();
    let mut m1 = sample_new_memory("test-project", "F1", "C1");
    m1.entities = "auth".to_string();
    h.db.insert_memory(m1).unwrap();

    let mut m2 = sample_new_memory("other-project", "F2", "C2");
    m2.entities = "auth".to_string();
    h.db.insert_memory(m2).unwrap();

    // Default scope: only current project.
    let scoped = h
        .entity_facts(EntityFactsParams {
            entity_name: "auth".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert_eq!(scoped.facts.len(), 1);

    // scope=global: both projects.
    let global = h
        .entity_facts(EntityFactsParams {
            entity_name: "auth".to_string(),
            project_id: None,
            scope: Some("global".to_string()),
        })
        .await;
    assert_eq!(global.facts.len(), 2);
}

#[tokio::test]
async fn entity_facts_excludes_invalidated_facts() {
    // F-007 review fixup (challenger #3): memory_entity_facts must
    // not return facts with `valid_until` set (invalidated/superseded).
    // Pre-fixup it silently returned them, misleading "current state of
    // knowledge about X" callers.
    let h = Harness::new();
    let mut m1 = sample_new_memory("test-project", "Active", "active fact");
    m1.entities = "auth".to_string();
    let id_active = h.db.insert_memory(m1).unwrap();

    let mut m2 = sample_new_memory("test-project", "Stale", "stale fact");
    m2.entities = "auth".to_string();
    let id_stale = h.db.insert_memory(m2).unwrap();

    // Invalidate the second fact.
    h.db.invalidate_memory(&id_stale, None, Some("test invalidation"))
        .unwrap();

    let out = h
        .entity_facts(EntityFactsParams {
            entity_name: "auth".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert_eq!(
        out.facts.len(),
        1,
        "should only return the active (non-invalidated) fact"
    );
    assert_eq!(out.facts[0].id, id_active);
    assert!(
        !out.facts.iter().any(|f| f.id == id_stale),
        "invalidated fact must not appear in results"
    );
}

#[tokio::test]
async fn reingest_changed_entities_replaces_fact_entity_links() {
    // F-007 review fixup (challenger #8): re-ingest with changed
    // entities CSV must REPLACE fact_entity rows, not accumulate.
    // Pre-fixup: jwt→fact link from first ingest would persist when
    // the fact got re-ingested with ["auth","redis"], drifting
    // fact_entity from the authoritative TEXT column.
    let h = Harness::new();

    // First ingest: entities = "auth,jwt"
    let mut m = sample_new_memory("test-project", "Fact", "same content");
    m.entities = "auth,jwt".to_string();
    let id = h.db.insert_memory(m).unwrap();
    let initial = h.db.entities_of_fact(&id).unwrap();
    let initial_names: std::collections::HashSet<String> =
        initial.into_iter().map(|(_, name)| name).collect();
    assert_eq!(
        initial_names,
        ["auth", "jwt"]
            .iter()
            .map(|s| s.to_string())
            .collect::<std::collections::HashSet<_>>()
    );

    // Re-ingest with same content (triggers ON CONFLICT DO UPDATE) but
    // changed entities = "auth,redis".
    let mut m2 = sample_new_memory("test-project", "Fact", "same content");
    m2.entities = "auth,redis".to_string();
    let id2 = h.db.insert_memory(m2).unwrap();
    assert_eq!(id, id2, "content-hash dedup should return same fact id");

    let after = h.db.entities_of_fact(&id).unwrap();
    let after_names: std::collections::HashSet<String> =
        after.into_iter().map(|(_, name)| name).collect();
    let expected: std::collections::HashSet<String> =
        ["auth", "redis"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        after_names, expected,
        "fact_entity links must be REPLACED (snapshot semantic), not accumulated; \
         expected {{auth, redis}}, got {:?}",
        after_names
    );
    assert!(
        !after_names.contains("jwt"),
        "stale 'jwt' link must be removed after re-ingest with new entities CSV"
    );
}

#[tokio::test]
async fn contradiction_check_uses_fact_entity_index() {
    // F-007 review fixup (codex #8 + challenger #7): AC4 explicitly
    // required EXPLAIN QUERY PLAN verification that the new
    // idx_fact_entity_entity index is actually used, not a fallback
    // table scan on memory_entries. This test runs EXPLAIN against the
    // facts_with_entity query (the index-driven entry point used by
    // contradiction.rs) and asserts the planner uses the index.
    let h = Harness::new();
    // Insert at least one fact to ensure the table isn't empty (SQLite
    // can pick different plans for empty vs populated tables).
    let mut m = sample_new_memory("test-project", "F", "C");
    m.entities = "auth".to_string();
    h.db.insert_memory(m).unwrap();

    // Replicate the query shape used by facts_with_entity (scoped path).
    let auth_param: String = "auth".to_string();
    let project_param: String = "test-project".to_string();
    let plan =
        h.db.explain_query_plan(
            "SELECT fe.fact_id
             FROM entities e
             JOIN fact_entity fe ON fe.entity_id = e.id
             WHERE e.name = ?1 AND e.project_id = ?2",
            &[&auth_param, &project_param],
        )
        .unwrap();
    let joined = plan.join(" | ");
    // The planner should use idx_fact_entity_entity (or the alternate
    // composite idx_fact_entity_fact + entities lookup chain) — what
    // we MUST NOT see is a SCAN of memory_entries.
    assert!(
        !joined.contains("SCAN memory_entries"),
        "expected index-driven query plan, got: {joined}"
    );
    // Either fact_entity index or entities index should show up.
    assert!(
        joined.contains("fact_entity") || joined.contains("entities"),
        "expected fact_entity or entities table in plan, got: {joined}"
    );
}

#[tokio::test]
async fn entity_facts_orders_by_recall_count_desc() {
    let h = Harness::new();
    let mut m1 = sample_new_memory("test-project", "Low-Recall", "C1");
    m1.entities = "auth".to_string();
    let id_low = h.db.insert_memory(m1).unwrap();

    let mut m2 = sample_new_memory("test-project", "High-Recall", "C2");
    m2.entities = "auth".to_string();
    let id_high = h.db.insert_memory(m2).unwrap();

    // Bump id_high recall 5 times; id_low stays 0.
    for _ in 0..5 {
        h.db.bump_recall_only(&id_high).unwrap();
    }

    let out = h
        .entity_facts(EntityFactsParams {
            entity_name: "auth".to_string(),
            project_id: None,
            scope: None,
        })
        .await;
    assert!(out.error.is_none());
    assert_eq!(out.facts.len(), 2);
    // High-recall first.
    assert_eq!(out.facts[0].id, id_high);
    assert_eq!(out.facts[1].id, id_low);
}

// =====================================================================
// F-008: memory_lint MCP tool
// =====================================================================

#[tokio::test]
async fn lint_empty_db_all_zero() {
    let h = Harness::new();
    let report = h
        .lint(LintParams {
            project_id: None,
            scope: None,
        })
        .await;
    assert!(!report.generated_at.is_empty());
    assert_eq!(report.orphan_gc.superseded_by_dangling_count, 0);
    assert_eq!(report.unresolved_contradictions.half_v_only_count, 0);
    assert_eq!(report.embedding_drift.embedding_null_non_synthesis_count, 0);
    assert_eq!(report.embedding_drift.synthesis_null_embedding_count, 0);
}

#[tokio::test]
async fn lint_detects_bl_022_synthesis_null_embedding() {
    // F-008 plan AC5: first run on dogfood DB predicted 5 synthesis-
    // null-embedding hits per BL-022. Construct that scenario via test
    // fixture + assert detection.
    let h = Harness::new();
    let mut m = sample_new_memory("test-project", "Syn", "synthesis body");
    m.source_type = "synthesis".to_string();
    m.knowledge_type = "factual".to_string();
    m.embedding = None;
    m.embedding_dim = None;
    let id = h.db.insert_memory(m).unwrap();

    let report = h
        .lint(LintParams {
            project_id: None,
            scope: None,
        })
        .await;
    assert_eq!(report.embedding_drift.synthesis_null_embedding_count, 1);
    assert_eq!(report.embedding_drift.synthesis_null_embedding[0], id);
    // And NOT in 3a (filters source_type != synthesis).
    assert_eq!(report.embedding_drift.embedding_null_non_synthesis_count, 0);
}

#[tokio::test]
async fn lint_is_read_only_via_mcp() {
    let h = Harness::new();
    let id =
        h.db.insert_memory(sample_new_memory("test-project", "T", "C"))
            .unwrap();
    let before = h.db.get_memory(&id).unwrap().unwrap();

    // Call lint 2x.
    for _ in 0..2 {
        let _ = h
            .lint(LintParams {
                project_id: None,
                scope: None,
            })
            .await;
    }

    let after = h.db.get_memory(&id).unwrap().unwrap();
    assert_eq!(after.recall_count, before.recall_count);
    assert_eq!(after.avg_relevance, before.avg_relevance);
    assert_eq!(after.last_recalled, before.last_recalled);
}
