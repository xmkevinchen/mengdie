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
    let mut embedder =
        Embedder::new().expect("failed to load embedding model (first run downloads ~90MB)");
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
        writeln!(
            f,
            "Use JWT tokens with Redis session store for authentication."
        )
        .unwrap();
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
    assert!(
        !results.is_empty(),
        "search should return the ingested memory"
    );
    assert_eq!(results[0].entry.id, entry_id);

    // 5. Verify recall was updated
    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert_eq!(entry.recall_count, 1);
    assert!(entry.avg_relevance > 0.0);

    // 6. Simulate enough recalls for Dreaming promotion
    // Need avg_relevance >= 0.45 (DEFAULT_MIN_RELEVANCE) — search RRF score is small (~0.03),
    // so add high-relevance recalls to bring average up.
    for _ in 0..9 {
        db.record_recall(&entry_id, 0.9).unwrap();
    }
    let entry = db.get_memory(&entry_id).unwrap().unwrap();
    assert_eq!(entry.recall_count, 10); // 1 from search + 9 manual
    assert!(
        entry.avg_relevance > 0.45,
        "avg_relevance should be above dreaming threshold: {}",
        entry.avg_relevance
    );

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

/// BL-008 Step 5 — smoke test the decay/demotion pass on a seeded corpus.
///
/// Seeds 6 long-term memories with varied `last_recalled` ages and a frozen
/// `now`, then runs one `run_dreaming_with_config` pass. Verifies the exact
/// demotion outcome across the boundary cases from plan 013 AC1:
/// d=0 and d=15 stay promoted; d=75 rides the floor boundary and survives
/// (effective ≈ 0.205 > 0.20); d=77 is just above the floor (eff ≈ 0.2001,
/// survives); d=78 just crosses (eff ≈ 0.1977, demotes); d=137 demotes hard.
///
/// Note: integration tests can't reach private `Db::lock_conn`, so the
/// raw UPDATE that forces `is_longterm = 1` + specific `avg_relevance` +
/// specific `last_recalled` goes through a parallel `rusqlite` connection
/// on the same file. The `Db` handle's `Arc<Mutex<Connection>>` is
/// released between operations so SQLite serializes correctly.
#[test]
fn test_decay_smoke_on_seeded_corpus() {
    use chrono::TimeZone;
    use mengdie::core::db::{Db, NewMemory};
    use mengdie::core::dreaming::DreamingConfig;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    let db = Db::open(&db_path).unwrap();

    let now = chrono::Utc.with_ymd_and_hms(2026, 10, 1, 12, 0, 0).unwrap();

    let days_before = |d: i64| now - chrono::Duration::days(d);

    let insert = |title: &str| -> String {
        let uid = uuid::Uuid::new_v4();
        db.insert_memory(NewMemory {
            project_id: "proj".to_string(),
            source_file: format!("seed-{uid}.md"),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: format!("{title} {uid}"),
            content: format!("content {uid}"),
            entities: "test".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .unwrap()
    };

    let force_longterm = |id: &str, avg: f64, last: chrono::DateTime<chrono::Utc>| {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "UPDATE memory_entries SET is_longterm = 1, avg_relevance = ?1, last_recalled = ?2 WHERE id = ?3",
            rusqlite::params![avg, last.to_rfc3339(), id],
        )
        .unwrap();
    };

    let m_d0 = insert("d0");
    force_longterm(&m_d0, 0.50, days_before(0));
    let m_d15 = insert("d15");
    force_longterm(&m_d15, 0.487, days_before(15));
    let m_d75 = insert("d75");
    force_longterm(&m_d75, 0.487, days_before(75));
    let m_d77 = insert("d77");
    force_longterm(&m_d77, 0.487, days_before(77));
    let m_d78 = insert("d78");
    force_longterm(&m_d78, 0.487, days_before(78));
    let m_d137 = insert("d137");
    force_longterm(&m_d137, 0.487, days_before(137));

    let result = db
        .run_dreaming_with_config(None, &DreamingConfig::default(), Some(now), true)
        .unwrap();

    // d=78 (eff ≈ 0.1977) and d=137 (eff ≈ 0.100) demote — exactly 2.
    // d=0, d=15, d=75 (eff ≈ 0.205), d=77 (eff ≈ 0.2001) stay promoted.
    assert_eq!(
        result.demoted, 2,
        "expected exactly 2 demotions (d=78, d=137), got {}",
        result.demoted
    );
    assert_eq!(result.decay_floor_breaches, 2);
    assert_eq!(result.breached_ids.len(), 2);
    assert!(result.breached_ids.contains(&m_d78));
    assert!(result.breached_ids.contains(&m_d137));

    for (label, id) in [
        ("d0", &m_d0),
        ("d15", &m_d15),
        ("d75", &m_d75),
        ("d77", &m_d77),
    ] {
        let e = db.get_memory(id).unwrap().unwrap();
        assert!(
            e.is_longterm,
            "{label} (id={id}) should still be long-term after the pass"
        );
    }

    for (label, id) in [("d78", &m_d78), ("d137", &m_d137)] {
        let e = db.get_memory(id).unwrap().unwrap();
        assert!(
            !e.is_longterm,
            "{label} (id={id}) should be demoted (is_longterm=0)"
        );
    }

    eprintln!("decay smoke passed: 2 demoted (d=78, d=137), 4 survived (d=0, 15, 75, 77) ✓");
}
