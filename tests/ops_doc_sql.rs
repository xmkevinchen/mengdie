//! Plan 016 Step 4 — lightweight drift guard for the SQL embedded in
//! `docs/operations/dreaming-decay.md`. Parses the doc, extracts the SQL
//! query from between the locked HTML-comment markers, asserts the
//! required filter conditions are present, and runs the query against a
//! seeded 3-row fixture DB to confirm the filter is semantically correct.
//!
//! No coupling to internal DreamingResult fields (dropped per plan 016
//! Doodlestein regret — those field names are likely to evolve during
//! Phase 2 work). Doc-facing only: filter substring check + fixture count.

use std::path::PathBuf;

use chrono::{Duration, Utc};
use mengdie::core::db::{Db, NewMemory};

const DOC_REL: &str = "docs/operations/dreaming-decay.md";
const THRESHOLD_BEGIN: &str = "<!-- threshold-snippet:begin -->";
const THRESHOLD_END: &str = "<!-- threshold-snippet:end -->";

/// Read the ops doc as a string; `CARGO_MANIFEST_DIR` resolves the
/// workspace root at compile time regardless of `cargo test` cwd.
fn read_doc() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DOC_REL);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

/// Extract the first ```sql fenced block between `begin` and `end` markers.
/// Returns the SQL body with the fences stripped.
fn extract_sql_between_markers(doc: &str, begin: &str, end: &str) -> String {
    let begin_pos = doc
        .find(begin)
        .unwrap_or_else(|| panic!("marker not found in doc: {begin}"));
    let end_pos = doc
        .find(end)
        .unwrap_or_else(|| panic!("marker not found in doc: {end}"));
    assert!(
        begin_pos < end_pos,
        "begin marker must precede end marker: {begin} at {begin_pos}, {end} at {end_pos}"
    );
    let section = &doc[begin_pos..end_pos];

    // Single-block invariant: if a future edit introduces a SECOND ```sql
    // fence between the markers, our "first block" heuristic would silently
    // pick the wrong one. Fail loudly instead so the doc author notices.
    let sql_fence_count = section.matches("```sql").count();
    assert_eq!(
        sql_fence_count, 1,
        "expected exactly one ```sql fence between {begin} and {end}, found {sql_fence_count}. \
         The test-extraction logic assumes a single block; split the doc or update the test."
    );

    // Find the first ```sql fence within the section.
    let sql_start = section
        .find("```sql")
        .unwrap_or_else(|| panic!("no ```sql fence between markers {begin} / {end}"));
    // Skip past the "```sql\n" opener.
    let after_fence = &section[sql_start + "```sql".len()..];
    let after_fence = after_fence
        .trim_start_matches('\n')
        .trim_start_matches('\r');

    // Find the matching closing ``` fence.
    let close = after_fence
        .find("```")
        .unwrap_or_else(|| panic!("unclosed ```sql fence between markers {begin} / {end}"));

    after_fence[..close].to_string()
}

#[test]
fn threshold_snippet_contains_required_filter_triple() {
    let doc = read_doc();
    let sql = extract_sql_between_markers(&doc, THRESHOLD_BEGIN, THRESHOLD_END);

    // Literal-substring checks — simple and robust against whitespace /
    // newline variation across doc edits.
    for required in [
        "is_longterm = 1",
        "valid_until IS NULL",
        "last_recalled IS NOT NULL",
    ] {
        assert!(
            sql.contains(required),
            "threshold SQL must include filter condition `{required}` (matches decay pass at src/core/dreaming.rs:163-167). \
             Extracted SQL:\n{sql}"
        );
    }
}

#[test]
fn threshold_snippet_counts_only_decay_eligible_rows() {
    // Hold `tmp` past the rusqlite connection (lifetime trap from plan 015).
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();

    // Create schema via Db::open, then seed 3 rows with direct rusqlite
    // connection. This gives us precise control over is_longterm,
    // valid_until, and last_recalled values — the dimensions the filter
    // should discriminate on.
    let db = Db::open(&db_path).expect("open test db");

    // Content must differ per row — `insert_memory` UPSERTs on
    // (project_id, content_hash), so identical content collapses all
    // three inserts into a single row. Use the title as content too to
    // guarantee distinct hashes.
    let insert = |title: &str| -> String {
        db.insert_memory(NewMemory {
            project_id: "plan-016-step4".into(),
            source_file: format!("{title}.md"),
            source_type: "conclusion".into(),
            knowledge_type: "decisional".into(),
            title: title.into(),
            content: format!("seed content for {title}"),
            entities: "plan-016".into(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .expect("insert seed")
    };

    let eligible = insert("decay-eligible");
    let immune = insert("null-last_recalled immune");
    let invalid = insert("invalidated");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let recent = (Utc::now() - Duration::days(10)).to_rfc3339();
    let invalid_at = (Utc::now() - Duration::days(5)).to_rfc3339();

    // Row 1: decay-eligible. is_longterm=1, valid_until=NULL, last_recalled set.
    conn.execute(
        "UPDATE memory_entries SET is_longterm = 1, avg_relevance = 0.5, last_recalled = ?1
         WHERE id = ?2",
        rusqlite::params![recent, eligible],
    )
    .unwrap();
    // Row 2: immune (NULL last_recalled). is_longterm=1, valid_until=NULL, last_recalled=NULL.
    conn.execute(
        "UPDATE memory_entries SET is_longterm = 1, avg_relevance = 0.5, last_recalled = NULL
         WHERE id = ?1",
        rusqlite::params![immune],
    )
    .unwrap();
    // Row 3: invalidated. is_longterm=1, valid_until set, last_recalled set.
    conn.execute(
        "UPDATE memory_entries SET is_longterm = 1, avg_relevance = 0.5,
            last_recalled = ?1, valid_until = ?2 WHERE id = ?3",
        rusqlite::params![recent, invalid_at, invalid],
    )
    .unwrap();

    // Extract the doc's threshold SQL and run it. Expected: exactly 1 row
    // (only the decay-eligible one passes all three filter conditions).
    let doc = read_doc();
    let sql = extract_sql_between_markers(&doc, THRESHOLD_BEGIN, THRESHOLD_END);

    // Debug: dump all rows to see the actual state.
    let debug_rows: Vec<(String, i64, Option<String>, Option<String>)> = conn
        .prepare("SELECT id, is_longterm, last_recalled, valid_until FROM memory_entries")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let debug_dump = debug_rows
        .iter()
        .map(|(id, lt, lr, vu)| {
            format!("  id={id} is_longterm={lt} last_recalled={lr:?} valid_until={vu:?}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let count: i64 = conn
        .query_row(sql.trim().trim_end_matches(';'), [], |r| r.get(0))
        .expect("extracted SQL runs and returns an integer count");

    assert_eq!(
        count, 1,
        "doc's threshold SQL must count exactly 1 decay-eligible row \
         (3 rows seeded: 1 eligible + 1 immune + 1 invalidated). \
         Got {count}.\nExtracted SQL:\n{sql}\nSeeded rows:\n{debug_dump}"
    );

    // Keep `tmp` alive past the rusqlite connection.
    drop(conn);
    drop(tmp);
}
