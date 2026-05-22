//! F-014 / BL-022 — backfill e2e test for `reembed_synthesis_rows`.
//!
//! Seeds 3 synthesis rows with `embedding=None` (mimics pre-fix
//! dreaming.rs behavior) → backfill → assert all 3 now have non-NULL
//! embedding + dim=384 → second invocation finds 0 (idempotency).
//!
//! Calls the library fn directly (NOT the CLI subcommand — integration
//! tests cannot import `src/bin/cli.rs` per Cargo's lib/bin boundary).

use std::sync::{Arc, Mutex};

use mengdie::core::db::{Db, NewMemory};
use mengdie::core::embeddings::Embedder;
use mengdie::core::reembed::reembed_synthesis_rows;

fn seed_null_synthesis(db: &Db, project: &str, title: &str) -> String {
    db.insert_memory(NewMemory {
        project_id: project.to_string(),
        source_file: format!("{title}-{}.md", uuid::Uuid::new_v4()),
        source_type: "synthesis".to_string(),
        knowledge_type: "factual".to_string(),
        title: title.to_string(),
        content: format!(
            "Synthesis content for {title}. Pre-fix dreaming.rs stored this with embedding=None."
        ),
        entities: "bl-022-backfill,reembed-test".to_string(),
        embedding: None,
        embedding_dim: None,
        is_longterm: false,
    })
    .unwrap()
}

#[test]
fn reembed_backfill_repairs_3_rows_then_is_idempotent() {
    let db = Db::open_in_memory().unwrap();
    let project = "backfill-proj";

    let id1 = seed_null_synthesis(&db, project, "Source A");
    let id2 = seed_null_synthesis(&db, project, "Source B");
    let id3 = seed_null_synthesis(&db, project, "Source C");
    let seeded_ids = [id1.clone(), id2.clone(), id3.clone()];

    let embedder = Arc::new(Mutex::new(
        Embedder::new().expect("Embedder::new failed in backfill e2e"),
    ));

    // First invocation: dry-run preview.
    let preview = reembed_synthesis_rows(&db, Arc::clone(&embedder), Some(project), true)
        .expect("dry-run should succeed");
    assert!(preview.dry_run);
    assert_eq!(preview.affected.len(), 3);
    for id in &seeded_ids {
        assert!(
            preview.affected.contains(id),
            "dry-run should list seeded id {id} as affected"
        );
    }

    // Confirm dry-run wrote nothing.
    let after_dry = db.list_memories(Some(project)).unwrap();
    for row in after_dry.iter().filter(|m| m.source_type == "synthesis") {
        assert!(
            row.embedding.is_none(),
            "dry-run must not write; row {} should still be NULL",
            row.id
        );
    }

    // Second invocation: real backfill.
    let real = reembed_synthesis_rows(&db, Arc::clone(&embedder), Some(project), false)
        .expect("backfill should succeed");
    assert!(!real.dry_run);
    assert_eq!(real.affected.len(), 3);

    // BL-022 / AC2 core assertion: all 3 rows now have non-NULL embedding.
    let after_real = db.list_memories(Some(project)).unwrap();
    let synthesis_rows: Vec<_> = after_real
        .iter()
        .filter(|m| m.source_type == "synthesis")
        .collect();
    assert_eq!(synthesis_rows.len(), 3);
    for row in &synthesis_rows {
        assert!(
            row.embedding.is_some(),
            "row {} should be embedded after backfill",
            row.id
        );
        assert_eq!(
            row.embedding_dim,
            Some(384),
            "row {} should have embedding_dim=384",
            row.id
        );
    }

    // Third invocation: idempotency. Re-running finds 0 NULL rows.
    let idempotent = reembed_synthesis_rows(&db, Arc::clone(&embedder), Some(project), false)
        .expect("second backfill (idempotent) should succeed");
    assert!(
        idempotent.affected.is_empty(),
        "idempotency: second backfill must find 0 rows; got {:?}",
        idempotent.affected
    );
}

#[test]
fn reembed_backfill_scope_filter_respects_project() {
    let db = Db::open_in_memory().unwrap();

    // Seed 2 NULL syntheses in project-A and 1 in project-B.
    seed_null_synthesis(&db, "project-A", "A-1");
    seed_null_synthesis(&db, "project-A", "A-2");
    seed_null_synthesis(&db, "project-B", "B-1");

    let embedder = Arc::new(Mutex::new(
        Embedder::new().expect("Embedder::new failed in scope-filter test"),
    ));

    // Project-A scope finds exactly 2.
    let a = reembed_synthesis_rows(&db, Arc::clone(&embedder), Some("project-A"), true).unwrap();
    assert_eq!(a.affected.len(), 2);

    // Global scope (None) finds all 3.
    let all = reembed_synthesis_rows(&db, Arc::clone(&embedder), None, true).unwrap();
    assert_eq!(all.affected.len(), 3);
}
