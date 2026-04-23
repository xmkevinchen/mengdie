//! Plan 015 Step 2 — stderr-JSON contract integration test.
//!
//! Spawns the `mengdie` binary (via `CARGO_BIN_EXE_mengdie` — first-ever CLI
//! subprocess test in this repo) against a seeded tempfile DB, captures stderr,
//! parses the `dreaming_pass` JSON event, and asserts the full 9-field
//! contract locked by plan 015.
//!
//! Why subprocess (not library): the contract claim is that `format_structured_json`
//! is emitted on stderr via `eprintln!` as a bare `{...}` line without tracing
//! wrapper. A unit test on the formatter alone would pass even if a regression
//! re-introduced a `tracing::info!(structured=...)` wrapper around the JSON
//! (which was the pre-fixup state of plan 013 Step 4, commit 32e11ef). Only a
//! subprocess test exercises the actual transport.
//!
//! DB seeding: plan 015 requires at least one long-term memory with non-null
//! `last_recalled` so `avg_effective_before > 0.0` is a meaningful assertion.
//! Empty DB yields `0.0` trivially via the null-guard at
//! `src/core/dreaming.rs:203-206`.

use std::process::Command;

use chrono::{Duration, Utc};
use mengdie::core::db::{Db, NewMemory};

/// Seed `db_path` with one long-term memory (avg_relevance=0.5, last_recalled=30d ago).
/// Returns the inserted memory id. Drops the `Db` handle and rusqlite connection
/// before returning, so the caller can spawn a subprocess against the same file.
fn seed_one_longterm(db_path: &std::path::Path) -> String {
    let db = Db::open(db_path).expect("open test db");
    let id = db
        .insert_memory(NewMemory {
            project_id: "plan-015-decay-contract".to_string(),
            source_file: "test.md".to_string(),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: "plan 015 test fixture".to_string(),
            content: "seed memory to produce non-zero avg_effective_before".to_string(),
            entities: "plan-015,test".to_string(),
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
        })
        .expect("insert test memory");

    // Direct connection to force is_longterm + set last_recalled. Pattern mirrors
    // tests/e2e.rs:170-177 (force_longterm helper in test_decay_smoke_on_seeded_corpus).
    let conn = rusqlite::Connection::open(db_path).expect("open rusqlite conn");
    let last_recalled = (Utc::now() - Duration::days(30)).to_rfc3339();
    conn.execute(
        "UPDATE memory_entries SET is_longterm = 1, avg_relevance = ?1, last_recalled = ?2 WHERE id = ?3",
        rusqlite::params![0.5_f64, last_recalled, id],
    )
    .expect("force long-term");

    id
}

#[test]
fn dreaming_pass_stderr_json_matches_plan_015_contract() {
    // LIFETIME TRAP (plan 015 doodlestein-adversarial): `tmp` must stay in scope
    // until after `Command::output()` returns. NamedTempFile deletes on drop.
    let tmp = tempfile::NamedTempFile::new().expect("create tempfile");
    let db_path = tmp.path().to_path_buf();

    let _seeded_id = seed_one_longterm(&db_path);

    // Spawn mengdie binary. `CARGO_BIN_EXE_mengdie` is populated by Cargo for
    // integration tests; binary name resolved from Cargo.toml [[bin]] (confirmed "mengdie").
    let output = Command::new(env!("CARGO_BIN_EXE_mengdie"))
        .args([
            "--db-path",
            db_path.to_str().expect("tempfile path is utf-8"),
            "dream",
            "--decay-dry-run",
        ])
        .output()
        .expect("spawn mengdie binary");

    assert!(
        output.status.success(),
        "mengdie dream --decay-dry-run failed: exit={:?}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");

    // Loose finder: any line containing the event discriminator (tolerant of
    // regression formats — this catches the line whether it's bare JSON or a
    // tracing-wrapped variant, then the bare-JSON assertion below distinguishes).
    let dream_line = stderr
        .lines()
        .find(|l| l.contains(r#""event":"dreaming_pass""#))
        .unwrap_or_else(|| {
            panic!(
                "no dreaming_pass event line on stderr.\nFull stderr:\n{}",
                stderr
            )
        });

    // Regression guard: the contract is BARE JSON (single `{...}` on its own line),
    // NOT a tracing-wrapped line like `2026-04-23T... INFO structured=...`. If a
    // future change introduces tracing wrapping, this assertion fails with a
    // specific error message. The grep in scripts/verify-decay.sh:62 uses
    // `^\{.*\}$` and would silently stop finding the line — this test makes the
    // contract break loud rather than silent.
    assert!(
        dream_line.trim_start().starts_with('{') && dream_line.trim_end().ends_with('}'),
        "dreaming_pass line must be bare JSON (plan 015 AC3 regression guard). \
         Found tracing-wrapped or otherwise malformed output: {:?}",
        dream_line
    );

    let v: serde_json::Value =
        serde_json::from_str(dream_line.trim()).expect("dreaming_pass line must parse as JSON");

    // Assert all 9 contract fields present by exact name (AC3).
    assert_eq!(v["schema_version"], 1, "schema_version (plan 015 AC1)");
    assert_eq!(v["event"], "dreaming_pass");
    assert!(v["promoted"].is_number(), "promoted must be integer");
    assert!(v["demoted"].is_number(), "demoted must be integer");
    assert!(
        v["decay_floor_breaches"].is_number(),
        "decay_floor_breaches must be integer"
    );
    assert!(
        v["avg_effective_before"].is_number(),
        "avg_effective_before must be number"
    );
    assert!(
        v["avg_effective_after"].is_number(),
        "avg_effective_after must be number"
    );
    assert_eq!(
        v["dry_run"], true,
        "dry_run should be true in --decay-dry-run"
    );
    assert!(v["breaches"].is_array(), "breaches must be an array");

    // Plan 015 dep-analyst requirement: avg_effective_before must be > 0.0
    // to prove the computation path, not just the null-guard. Seeded memory
    // (avg_relevance=0.5, last_recalled=30d ago) produces ~0.354 effective.
    let avg_before = v["avg_effective_before"]
        .as_f64()
        .expect("avg_effective_before parses as f64");
    assert!(
        avg_before > 0.0,
        "avg_effective_before must be > 0.0 (seeded data should produce non-trivial value; got {}). \
         Possible causes: tempfile dropped before subprocess ran (lifetime trap), \
         or seeding failed silently.",
        avg_before
    );

    // Keep `tmp` alive past here (enforced by scope, but explicit drop makes the
    // requirement visible to readers).
    drop(tmp);
}

// ---------------------------------------------------------------------------
// Plan 015 Step 5 — verify-decay.sh shell-script integration tests (BL-verify
// action 4). Shells out to the script against seeded tempfile DBs + a shim
// mengdie for the unparseable-JSON regression case.
//
// cfg(unix) because verify-decay.sh is bash-only; Windows CI would skip these.
// ---------------------------------------------------------------------------
#[cfg(unix)]
mod verify_decay_script {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use chrono::{Duration, Utc};
    use mengdie::core::db::{Db, NewMemory};

    const SCRIPT_REL: &str = "scripts/verify-decay.sh";

    /// Absolute path to scripts/verify-decay.sh. CARGO_MANIFEST_DIR is the
    /// workspace root at test compile time; reliable across cargo test invocations.
    fn script_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SCRIPT_REL)
    }

    /// Directory containing the `mengdie` binary built by cargo for these tests.
    /// Prepending this to PATH lets the shell script's bare `mengdie` invocation
    /// resolve to the just-built debug binary.
    fn mengdie_bin_dir() -> PathBuf {
        let bin = PathBuf::from(env!("CARGO_BIN_EXE_mengdie"));
        bin.parent()
            .expect("CARGO_BIN_EXE_mengdie has a parent dir")
            .to_path_buf()
    }

    /// PATH env var with the mengdie binary dir prepended.
    fn path_with_mengdie() -> String {
        let current = std::env::var("PATH").unwrap_or_default();
        format!("{}:{}", mengdie_bin_dir().display(), current)
    }

    /// Seed `db_path` with one long-term memory; caller controls `avg` and the
    /// days-ago for `last_recalled`. The (avg, days) combination controls whether
    /// the seeded memory breaches the decay floor:
    ///   avg=0.5, days=15  → effective ≈ 0.421 (no breach)
    ///   avg=0.487, days=78 → effective ≈ 0.1977 (breach — below 0.20 floor)
    fn seed_longterm(db_path: &Path, avg: f64, days_ago: i64) {
        let db = Db::open(db_path).expect("open test db");
        let id = db
            .insert_memory(NewMemory {
                project_id: "plan-015-verify-script".to_string(),
                source_file: "test.md".to_string(),
                source_type: "conclusion".to_string(),
                knowledge_type: "decisional".to_string(),
                title: format!("plan 015 seed avg={avg} days={days_ago}"),
                content: "seed memory for verify-decay.sh test".to_string(),
                entities: "plan-015,test".to_string(),
                embedding: None,
                embedding_dim: None,
                is_longterm: false,
            })
            .expect("insert seed memory");

        let conn = rusqlite::Connection::open(db_path).expect("rusqlite open");
        let last_recalled = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
        conn.execute(
            "UPDATE memory_entries SET is_longterm = 1, avg_relevance = ?1, last_recalled = ?2 WHERE id = ?3",
            rusqlite::params![avg, last_recalled, id],
        )
        .expect("force longterm");
    }

    /// Invoke verify-decay.sh with the given extra args + DB path. Returns
    /// (exit_code, stdout, stderr). `mengdie_bin_dir()` is prepended to PATH so
    /// the script's bare `mengdie` call resolves to the cargo-built binary.
    fn run_script(db_path: &Path, extra_args: &[&str]) -> (i32, String, String) {
        let mut args: Vec<String> = vec![
            "--db-path".to_string(),
            db_path.to_str().unwrap().to_string(),
        ];
        for a in extra_args {
            args.push((*a).to_string());
        }
        let out = Command::new(script_path())
            .args(&args)
            .env("PATH", path_with_mengdie())
            .output()
            .expect("spawn verify-decay.sh");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    // --- Exit-code matrix (AC4 + AC5) ---

    #[test]
    fn no_breaches_no_flag_exits_zero() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // avg=0.5, days=15 → effective ≈ 0.421; above 0.20 floor, no breach.
        seed_longterm(tmp.path(), 0.5, 15);
        let (code, _stdout, _stderr) = run_script(tmp.path(), &[]);
        assert_eq!(
            code, 0,
            "no breaches should exit 0 without --i-reviewed-each"
        );
    }

    #[test]
    fn breaches_no_flag_exits_one() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // avg=0.487, days=78 → effective ≈ 0.1977; below 0.20 floor → breach.
        seed_longterm(tmp.path(), 0.487, 78);
        let (code, _stdout, _stderr) = run_script(tmp.path(), &[]);
        assert_eq!(
            code, 1,
            "breaches without --i-reviewed-each should exit 1 (require approval)"
        );
    }

    #[test]
    fn breaches_with_approval_exits_zero() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        seed_longterm(tmp.path(), 0.487, 78);
        let (code, _stdout, _stderr) = run_script(tmp.path(), &["--i-reviewed-each"]);
        assert_eq!(
            code, 0,
            "breaches + --i-reviewed-each should exit 0 (operator approved)"
        );
    }

    /// Plan 015 Step 3 regression guard (AC4): when the binary emits no JSON
    /// line, the script MUST exit 2 regardless of `--i-reviewed-each`. The
    /// prior silent-bypass exited 0 — that's the regression this case protects
    /// against.
    ///
    /// Shim pattern: place a fake `mengdie` that prints empty stderr earlier
    /// on PATH than the real binary. Fix for the `$PATH_HEAD` copy-paste error
    /// in an earlier draft of AC4 — variable is `shim_dir`, defined before use.
    #[test]
    fn unparseable_json_with_approval_exits_two() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let shim_dir = tempfile::tempdir().expect("create shim dir");
        let shim_path = shim_dir.path().join("mengdie");
        std::fs::write(&shim_path, "#!/bin/sh\nexit 0\n").expect("write shim");
        let mut perms = std::fs::metadata(&shim_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms).expect("chmod shim");

        // Shim dir must come FIRST on PATH so it shadows the real mengdie.
        let path_env = format!(
            "{}:{}",
            shim_dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = Command::new(script_path())
            .args([
                "--db-path",
                tmp.path().to_str().unwrap(),
                "--i-reviewed-each",
            ])
            .env("PATH", path_env)
            .output()
            .expect("spawn verify-decay.sh with shim");

        let code = out.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            code, 2,
            "unparseable JSON + --i-reviewed-each must exit 2 (plan 015 Step 3 regression guard). \
             Got exit={code}, stderr={stderr}"
        );
        // Error message must mention empty-stderr or format-regression hint
        // (the two Step-3-emitted branches). Either specific phrase is acceptable.
        assert!(
            stderr.contains("no stderr output") || stderr.contains("no dreaming_pass JSON line"),
            "stderr should distinguish transient failure vs format regression. \
             Got: {stderr}"
        );

        drop(tmp); // hold tempfile past subprocess
        drop(shim_dir);
    }
}
