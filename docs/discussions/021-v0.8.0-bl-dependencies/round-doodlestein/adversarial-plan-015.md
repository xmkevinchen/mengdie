---
author: doodlestein-adversarial
plan: "015"
date: 2026-04-23
---

# Adversarial Review — Plan 015: First-Failure Point

## Verdict

**One clear first-failure point exists**: Step 2's integration test will fail at runtime because `tempfile::NamedTempFile` deletes the underlying file when the variable goes out of scope — but a `std::process::Command` subprocess cannot hold an open `rusqlite::Connection` handle to the same file across the drop boundary.

More precisely: the test must simultaneously hold `tmp` (the `NamedTempFile` — so the file is not deleted) AND allow the subprocess to open the same path. The plan says to seed via "direct `rusqlite::Connection` on the tempfile path" — so the seeder must *close* its connection before spawning the binary, and `tmp` must *stay in scope* until after `Command::output()` returns. A coding agent following the e2e.rs pattern at lines 140-177 will write:

```rust
let tmp = tempfile::NamedTempFile::new().unwrap();
let db_path = tmp.path().to_path_buf();
let db = Db::open(&db_path).unwrap();
// ... seed ...
// Agent may drop `db` and then `tmp` before spawning — file deleted before subprocess starts
let output = Command::new(env!("CARGO_BIN_EXE_mengdie"))
    .arg("--db-path").arg(&db_path)   // path is now dangling
    .arg("dream").arg("--decay-dry-run")
    .output().unwrap();
```

The plan does not state the ownership invariant: **`tmp` must remain in scope for the full duration of the subprocess**. An agent following the e2e.rs template will copy `db_path` (correct) but may let `db` and `tmp` drop before spawning (wrong). The file vanishes, `Db::open` in the subprocess gets a fresh empty DB, `avg_effective_before` is `0.0`, and the seeding assertion `avg_effective_before > 0.0` fails — silently diagnosable but not obvious from the error message.

This is not a test of the binary or the schema. It is a silent seeding race that the plan's wording does not guard against.

## Supporting evidence

- `e2e.rs:145-147`: the pattern holds `tmp` and `db` in the same scope, but e2e.rs never spans a subprocess — the agent has no reference implementation for cross-process tempfile lifetime.
- `src/bin/cli.rs:152-153`: `Db::open` creates the schema if missing, so an empty DB at `db_path` is a valid (not erroring) open — the subprocess succeeds, just with zero rows.
- Plan Step 2 says "follows pattern at `tests/e2e.rs:140-177`" without distinguishing the single-process vs. cross-process lifetime difference.

## Severity

**Step 2 fails on first write, not compile**. The test compiles and runs but the `avg_effective_before > 0.0` assertion fails with no hint that the file was deleted. The agent sees a test failure, reads the assertion, re-reads the seeding code, and may incorrectly conclude the dreaming logic is broken rather than the tempfile lifetime.

## Fix (minimal)

Add one sentence to Step 2's seeding subtask:

> `tmp` must be bound as a named variable and remain in scope until after `Command::output()` returns — dropping it before the subprocess starts silently deletes the DB file.

No code change required in the plan, just that constraint stated explicitly.

## No other first-failure points

After applying that fix, the remaining steps are well-specified:

- `env!("CARGO_BIN_EXE_mengdie")` is valid — Cargo.toml line 33 declares `name = "mengdie"`.
- The `--db-path` flag is confirmed present at `cli.rs:17-18`, and the `global = true` attribute means it works before the subcommand.
- Step 3's `[[ -s "$TMP_ERR" ]]` branch logic is straightforward shell.
- Step 4's arg-parse loop extension has no hidden footguns (the script already uses `for arg in "$@"` positional loop).
- Step 5's shim pattern (`chmod +x` shim dir on PATH) is portable on Linux/macOS.
- Step 6's BL close is admin-only with no compile-time dependency.
