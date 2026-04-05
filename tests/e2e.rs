//! End-to-end smoke tests for the Second Brain pipeline.
//! These tests require the fastembed model (~90MB, downloaded on first run).
//! Run with: cargo test --test e2e

use std::io::Write;

use second_brain::core::db::Db;
use second_brain::core::embeddings::Embedder;
use second_brain::core::ingest::ingest_file;
use second_brain::core::parser::is_ingestable;

/// E2E: Create a conclusion.md → ingest → search → find it → Dreaming promotes it.
#[test]
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
    let entry_id = ingest_file(&db, &mut embedder, &path, project_id).unwrap();
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
    let entry2_id = ingest_file(&db, &mut embedder, &path2, project_id).unwrap();
    assert!(!entry2_id.is_empty());

    // Check contradictions manually (ingest_file doesn't return conflicts directly)
    let entry2 = db.get_memory(&entry2_id).unwrap().unwrap();
    let entities: Vec<String> = entry2.entities.split(',').map(|s| s.trim().to_string()).collect();
    let emb2 = second_brain::core::embeddings::blob_to_embedding(
        entry2.embedding.as_ref().unwrap()
    ).unwrap();
    let conflicts = db
        .check_contradictions(&entities, Some(&emb2), &entry2.knowledge_type, project_id)
        .unwrap();
    // Should find the first entry as a conflict (same entities, both decisional)
    assert!(
        !conflicts.is_empty(),
        "should detect conflict between old and new auth decisions"
    );

    eprintln!("E2E test passed: ingest → search → recall → dream → contradiction ✓");
}
