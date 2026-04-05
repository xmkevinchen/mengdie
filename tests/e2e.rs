//! End-to-end smoke tests for the Mengdie pipeline.
//! These tests require the fastembed model (~90MB, downloaded on first run).
//! Run with: cargo test --test e2e

use std::io::Write;

use mengdie::core::db::Db;
use mengdie::core::embeddings::Embedder;
use mengdie::core::ingest::ingest_file;
use mengdie::core::parser::is_ingestable;

/// E2E: Create a conclusion.md → ingest → search → find it → Dreaming promotes it.
/// Requires fastembed model (~90MB download). Run with: cargo test --test e2e -- --ignored
#[test]
#[ignore]
fn test_full_pipeline() {
    // Setup
    let db = Db::open_in_memory().unwrap();
    let mut embedder = Embedder::new().expect("failed to load embedding model (first run downloads ~90MB)");
    let project_id = "test-e2e-project";

    // 1. Create a test conclusion file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conclusion.md");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "id: \"001\"").unwrap();
        writeln!(f, "title: \"Auth Middleware Decision\"").unwrap();
        writeln!(f, "tags: [auth, middleware, jwt]").unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# Auth Middleware Decision").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "Use JWT tokens with Redis session store for authentication.").unwrap();
        writeln!(f, "Session tokens expire after 24 hours.").unwrap();
    }
    assert!(is_ingestable(&path));

    // 2. Ingest the file
    let result = ingest_file(&db, &mut embedder, &path, project_id).unwrap();
    let entry_id = result.entry_id;
    assert!(!entry_id.is_empty());

    // 3. Verify it's stored correctly
    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert_eq!(entry.title, "Auth Middleware Decision");
    assert_eq!(entry.knowledge_type, "decisional");
    assert_eq!(entry.entities, "auth,middleware,jwt");
    assert!(entry.embedding.is_some());
    assert_eq!(entry.embedding_dim, Some(384));
    assert_eq!(entry.recall_count, 0);
    assert!(!entry.is_longterm);

    // 4. Search for it
    let query = "JWT authentication middleware";
    let query_embedding = embedder.embed_text(query).unwrap();
    let results = db
        .memory_search(query, &query_embedding, Some(project_id), 10)
        .unwrap();
    assert!(!results.is_empty(), "search should return the ingested memory");
    assert_eq!(results[0].entry.id, entry_id);

    // 5. Verify recall was updated
    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert_eq!(entry.recall_count, 1);
    assert!(entry.avg_relevance > 0.0);

    // 6. Simulate enough recalls for Dreaming promotion
    // Need avg_relevance >= 0.65 — search RRF score is small (~0.03),
    // so add high-relevance recalls to bring average up.
    for _ in 0..9 {
        db.record_recall(&entry_id, 0.9).unwrap();
    }
    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert_eq!(entry.recall_count, 10); // 1 from search + 9 manual
    assert!(entry.avg_relevance > 0.65, "avg_relevance should be above dreaming threshold: {}", entry.avg_relevance);

    // 7. Run Dreaming
    let dream_result = db.run_dreaming(None).unwrap();
    assert_eq!(dream_result.promoted, 1);

    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert!(entry.is_longterm);

    // 8. Test contradiction detection with a new similar memory
    let path2 = dir.path().join("conclusion-v2.md");
    {
        let mut f = std::fs::File::create(&path2).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "title: \"Updated Auth Decision\"").unwrap();
        writeln!(f, "tags: [auth, middleware, oauth]").unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# Updated Auth Decision").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "Switch from JWT to OAuth2 with PKCE for authentication.").unwrap();
    }

    // Ingest second file — should detect conflict with first
    let result2 = ingest_file(&db, &mut embedder, &path2, project_id).unwrap();
    assert!(!result2.entry_id.is_empty());

    // ingest_file now returns conflicts directly
    assert!(
        !result2.conflicts.is_empty(),
        "should detect conflict between old and new auth decisions"
    );

    eprintln!("E2E test passed: ingest → search → recall → dream → contradiction ✓");
}
