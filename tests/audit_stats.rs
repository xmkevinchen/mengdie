//! Integration tests for the `mengdie audit-stats` CLI subcommand (F-005).
//!
//! Invokes the binary via `Command::new(env!("CARGO_BIN_EXE_mengdie"))` —
//! `CARGO_BIN_EXE_mengdie` is set automatically by Cargo for any test target
//! that lives next to a `[[bin]]` (verified by F-005 dependency-analyst
//! review #4 — no `Cargo.toml` change needed).
//!
//! The schema seeded in these tests mirrors the one created by
//! `mengdie::core::db::Db::open_in_memory()` — we open an empty file-backed
//! DB via `mengdie stats` (cheap, side-effect-free, runs the migration), then
//! seed by direct SQL when needed. This keeps the integration test free of
//! `mengdie::core::*` linkage (the binary owns the schema; the test just
//! reads its JSON output back).

use std::process::Command;

use rusqlite::params;

/// Set up a temp DB by running `mengdie stats` on a fresh path. Returns the
/// DB path so individual tests can both invoke the binary AND seed extra
/// rows via direct SQL when needed (e.g., to bump audit_write_failures).
fn fresh_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("db.sqlite");
    // First call runs migrations; we discard its output.
    let status = Command::new(env!("CARGO_BIN_EXE_mengdie"))
        .arg("--db-path")
        .arg(&db_path)
        .arg("stats")
        .output()
        .expect("spawn mengdie stats");
    assert!(
        status.status.success(),
        "mengdie stats (migrate) failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    (tmp, db_path)
}

/// Run `mengdie audit-stats --format json` against `db_path` and parse the
/// stdout as a `serde_json::Value`. Asserts process exit success and stdout
/// is a single line (per AC2: "stdout is a single line").
fn run_json(db_path: &std::path::Path) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_mengdie"))
        .arg("--db-path")
        .arg(db_path)
        .arg("audit-stats")
        .arg("--format")
        .arg("json")
        .output()
        .expect("spawn mengdie audit-stats");
    assert!(
        out.status.success(),
        "audit-stats --format json failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("stdout utf8");
    let trimmed = stdout.trim_end_matches('\n');
    assert!(
        !trimmed.contains('\n'),
        "JSON stdout must be one line, got: {trimmed}"
    );
    serde_json::from_str(trimmed)
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}; raw={trimmed}"))
}

/// Run `mengdie audit-stats` (default = table) and return stdout as a single
/// String. Asserts process exit success.
fn run_table(db_path: &std::path::Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_mengdie"))
        .arg("--db-path")
        .arg(db_path)
        .arg("audit-stats")
        .output()
        .expect("spawn mengdie audit-stats");
    assert!(
        out.status.success(),
        "audit-stats (table) failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout utf8")
}

/// AC2 — table output exposes the seven labeled fields plus `status:`. We
/// seed the AC1 reference-case fixture here so the same fixture covers AC2
/// table assertions and the JSON schema check below.
///
/// Each label match uses line-anchored equality (after trimming the leading
/// indent) rather than `stdout.contains(...)`. A bare `contains("audit_count:
/// 3")` would false-pass if a future change ever adds a sibling field like
/// `total_audit_count: 3`; line equality on the trimmed body keeps the
/// assertion exact for the line content (Gemma cross-family review #3).
#[test]
fn test_table_format_emits_seven_fields_and_status() {
    let (_tmp, db_path) = fresh_db();
    seed_ac1_fixture(&db_path);

    let stdout = run_table(&db_path);
    let line_set: std::collections::HashSet<&str> = stdout.lines().map(str::trim).collect();
    let expected_lines = [
        "audit_count: 3",
        "link_count: 5",
        "audit_write_failures: 2",
        // For the AC1 seed (audit_write_failures = 2 > 0), status is
        // `degraded` per the F-005 status-precedence contract
        // (failures-win-over-zero-rows).
        "status: degraded",
    ];
    for expected in expected_lines {
        assert!(
            line_set.contains(expected),
            "expected line {expected:?} in stdout: {stdout}"
        );
    }
    // For oldest/newest/supersession we only assert the field is present
    // (the values themselves are exercised by the JSON-schema test below
    // and the Db-level unit tests; the table view's job is to present
    // them, not to lock in their exact format).
    let prefixes = ["oldest_row:", "newest_row:", "supersession_count_30d:"];
    for prefix in prefixes {
        assert!(
            line_set.iter().any(|l| l.starts_with(prefix)),
            "expected a line starting with {prefix:?} in stdout: {stdout}"
        );
    }
}

/// AC2 — JSON output is one-line, parses, and has exactly the seven
/// canonical keys plus `status`. Asserts the script-facing contract.
#[test]
fn test_json_format_schema() {
    let (_tmp, db_path) = fresh_db();
    seed_ac1_fixture(&db_path);

    let v = run_json(&db_path);
    let obj = v.as_object().expect("JSON root must be object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort();
    let mut expected = [
        "audit_count",
        "link_count",
        "oldest_row",
        "newest_row",
        "supersession_count_30d",
        "audit_write_failures",
        "status",
    ];
    expected.sort();
    assert_eq!(keys, expected, "JSON must have exactly these keys");

    // Type checks on the AC1 seed.
    assert_eq!(obj["audit_count"].as_i64(), Some(3));
    assert_eq!(obj["link_count"].as_i64(), Some(5));
    assert!(obj["oldest_row"].is_string(), "oldest_row must be string");
    assert!(obj["newest_row"].is_string(), "newest_row must be string");
    assert!(
        obj["supersession_count_30d"].as_i64().is_some(),
        "supersession_count_30d must be i64"
    );
    assert_eq!(obj["audit_write_failures"].as_i64(), Some(2));
    let status = obj["status"].as_str().expect("status is string");
    assert!(
        matches!(status, "ok" | "not_yet_triggered" | "degraded"),
        "status must be one of the three allowed values, got {status}"
    );
}

/// AC3 row 1 — Fresh DB, 0 audit rows, 0 failures → `not_yet_triggered`.
#[test]
fn test_status_not_yet_triggered_on_fresh_db() {
    let (_tmp, db_path) = fresh_db();
    let v = run_json(&db_path);
    assert_eq!(v["status"], "not_yet_triggered");
    assert_eq!(v["audit_count"], 0);
    assert_eq!(v["audit_write_failures"], 0);
}

/// AC3 row 2 — 1+ audit rows, 0 failures → `ok`.
#[test]
fn test_status_ok_when_clean() {
    let (_tmp, db_path) = fresh_db();
    {
        let conn = open_seed_conn(&db_path);
        conn.execute(
            "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
             VALUES ('q', 'proj', 1, '2026-05-08T08:00:00+00:00')",
            [],
        )
        .unwrap();
    }
    let v = run_json(&db_path);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["audit_count"], 1);
    assert_eq!(v["audit_write_failures"], 0);
}

/// AC3 row 3 — 1+ audit rows, ≥1 failures → `degraded`.
#[test]
fn test_status_degraded_when_failures_with_audit_rows() {
    let (_tmp, db_path) = fresh_db();
    {
        let conn = open_seed_conn(&db_path);
        conn.execute(
            "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
             VALUES ('q', 'proj', 1, '2026-05-08T08:00:00+00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO metrics (key, value_int, updated_at) \
             VALUES ('audit_write_failures', 4, '2026-05-08T08:00:01+00:00')",
            [],
        )
        .unwrap();
    }
    let v = run_json(&db_path);
    assert_eq!(v["status"], "degraded");
    assert_eq!(v["audit_count"], 1);
    assert_eq!(v["audit_write_failures"], 4);
}

/// AC3 corner — 0 audit rows, ≥1 failures → `degraded` (failures-win-
/// over-zero-rows precedence per Codex Track 2 finding on Steps 2+3 commit
/// `de65dc2`). The JSON consumer must be able to detect "hook fired but
/// every write was dropped" without relying on the table-format hint.
#[test]
fn test_status_degraded_when_failures_without_audit_rows() {
    let (_tmp, db_path) = fresh_db();
    {
        let conn = open_seed_conn(&db_path);
        conn.execute(
            "INSERT INTO metrics (key, value_int, updated_at) \
             VALUES ('audit_write_failures', 2, '2026-05-08T08:00:00+00:00')",
            [],
        )
        .unwrap();
    }
    let v = run_json(&db_path);
    assert_eq!(v["status"], "degraded");
    assert_eq!(v["audit_count"], 0);
    assert_eq!(v["audit_write_failures"], 2);
}

/// Open a raw rusqlite connection for seeding, with the same per-connection
/// `PRAGMA foreign_keys = OFF` setting that the production migration uses.
/// See `seed_ac1_fixture` for the longer rationale.
fn open_seed_conn(db_path: &std::path::Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn
}

/// AC2 — clap rejects an invalid `--format` value with exit code 2 and a
/// helpful error message, before `cmd_audit_stats` runs.
#[test]
fn test_invalid_format_rejected_by_clap() {
    let (_tmp, db_path) = fresh_db();
    let out = Command::new(env!("CARGO_BIN_EXE_mengdie"))
        .arg("--db-path")
        .arg(&db_path)
        .arg("audit-stats")
        .arg("--format")
        .arg("yaml")
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "clap should reject 'yaml'");
    assert_eq!(
        out.status.code(),
        Some(2),
        "clap usage errors exit with code 2"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid value 'yaml' for '--format"),
        "stderr should explain the rejection: {stderr}"
    );
    assert!(
        stderr.contains("[possible values: table, json]"),
        "stderr should list the allowed values: {stderr}"
    );
}

// ---- Fixtures ----------------------------------------------------------

/// AC1 reference-case seed: 3 rows in `memory_search_audit` (timestamps
/// 2026-05-01T00:00:00+00:00, 2026-05-04T12:00:00+00:00,
/// 2026-05-08T08:00:00+00:00), 5 rows in `audit_returned_facts` (with
/// non-existent `fact_id` strings — the FK is unenforced in SQLite by
/// default, and `Db::audit_stats()` only does a `COUNT(*)` over the link
/// table, so dangling FKs don't affect AC1 assertions), and one `metrics`
/// row (`audit_write_failures`, 2). All five link rows attach to the first
/// audit row.
///
/// **Why no `memory_entries` rows**: a raw `rusqlite::Connection` opens
/// without the sqlite-vec extension registered, so any INSERT into
/// `memory_entries` errors at trigger-parse time on `vec_memories` (the
/// schema-v7 vec0 virtual table referenced by `vec_memories_insert`).
/// AC1's six fields don't require real fact rows — they only count audit /
/// link / metrics rows + read MIN/MAX(searched_at) — so the fixture skips
/// `memory_entries` entirely.
fn seed_ac1_fixture(db_path: &std::path::Path) {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    // Match the production migration's per-connection FK setting (see
    // src/core/schema.rs:105). Mengdie's audit_returned_facts has FK clauses
    // that are intended as documentation, not runtime enforcement; without
    // this PRAGMA, rusqlite's bundled SQLite default would reject our
    // dangling fact_id seeds.
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    let timestamps = [
        "2026-05-01T00:00:00+00:00",
        "2026-05-04T12:00:00+00:00",
        "2026-05-08T08:00:00+00:00",
    ];
    let mut first_aid: Option<i64> = None;
    for t in &timestamps {
        conn.execute(
            "INSERT INTO memory_search_audit (query, scope, took_ms, searched_at) \
             VALUES ('q', 'proj', 1, ?1)",
            params![t],
        )
        .unwrap();
        if first_aid.is_none() {
            first_aid = Some(conn.last_insert_rowid());
        }
    }
    let aid = first_aid.unwrap();
    for i in 0..5 {
        let fact_id = format!("ac1-fact-{i}");
        conn.execute(
            "INSERT INTO audit_returned_facts (audit_id, fact_id, rank) \
             VALUES (?1, ?2, ?3)",
            params![aid, fact_id, i as i64],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO metrics (key, value_int, updated_at) \
         VALUES ('audit_write_failures', 2, '2026-05-08T08:00:01+00:00')",
        [],
    )
    .unwrap();
}
