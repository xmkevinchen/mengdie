# Plan 013 — Step Summaries

## Step 1 — Extract pure decay primitive + shared timestamp helper (commit: 7245994)

**Decisions**:
- Formula form `(2.0_f64).powf(-days / HALF_LIFE_DAYS)` inlined (not `exp(-ln(2) · d / H)`), per discussion 019 Round 2 — half-life semantics visually exact at `d = H`.
- Non-finite inputs (NaN, ±Inf) all clamp to 0.0 via `is_finite()` guard BEFORE the `<= 0.0` branch, so `NEG_INFINITY` does not escape as `1.0`. Single code path, predictable.
- Shared `MemoryEntry::last_recalled_as_datetime()` lives in `db.rs` (not `decay.rs`) — type-data locality, no cross-module visibility churn.
- Strict equality assertions at `d=60 (=0.5)` and `d=120 (=0.25)` — these values are exact IEEE-754 doubles under `2^(-d/60)`, so strict equality catches reintroduction of `exp(-d/H)`.

**Rejected**:
- `let expected = 0.7071_f64` — clippy `approx_constant` lint; replaced with `std::f64::consts::FRAC_1_SQRT_2`.
- `d=77` as "just-past-floor" test case — actual crossing at `avg=0.487` is `~77.05 days`, so integer `d=77` yields `eff ≈ 0.2001` (above floor). Fixed to `d=78 → eff ≈ 0.1977`. Plan's "~77-day trigger" wording stays as approximate.
- Tuple return for `effective_relevance` with elapsed-days debug info — premature; the demotion path in Step 2 recomputes elapsed days anyway for aggregate logging.

**Cross-step deps**:
- `decay::decay_factor`, `decay::effective_relevance`, `decay::should_demote`, `decay::DEMOTION_FLOOR`, `decay::HALF_LIFE_DAYS` → consumed by Step 2 (Dreaming pass) and Step 3 (search re-rank).
- `MemoryEntry::last_recalled_as_datetime()` → consumed by Step 2 (demotion select loop) and Step 3 (search post-fetch re-rank). Both call sites MUST use this helper — same-age-clock invariant.

**Actual files**: src/core/decay.rs, src/core/mod.rs, src/core/db.rs

## Step 2 — Demotion path + 5 new DreamingResult counters (commit: fcb4f26)

**Decisions**:
- `run_dreaming_with_config` grew TWO params (`now`, `write_demotions`) in one signature change rather than sequencing them across releases. The existing single-caller in `cli.rs` absorbed the change trivially; the wrapper `run_dreaming` hides both from downstream tests (6 dreaming tests + 1 e2e — all untouched).
- Chunked UPDATE at 500 IDs per statement. SQLite's default `SQLITE_LIMIT_VARIABLE_NUMBER` is 999; chunking at 500 gives headroom. Realistic workload is a handful of demoted memories per pass, so this is defensive, not hot.
- `avg_effective_score_after` short-circuits to `_before` when (a) dry-run, or (b) zero demotions fired. Avoids a second SELECT on the unchanged corpus.
- NULL `last_recalled` rows skipped via `AND last_recalled IS NOT NULL` in the SELECT — no in-memory filter. The COUNT query for INFO logging is a separate statement; small cost.

**Rejected**:
- `dry_run_decay(...)` as a sibling function (original plan). Unified via `write_demotions: bool` per architect consider — one code path, one set of tests.
- `DateTime<Utc>` as required param. `Option<DateTime<Utc>>` via `unwrap_or_else(Utc::now)` preserves backwards compat through the `run_dreaming` wrapper.
- Holding `breached_ids` as a bare `Vec<&str>` borrowed from the SELECT rows. The rusqlite row borrow lifetime would tangle with the subsequent UPDATE chunking loop. `Vec<String>` — owned — sidesteps this cleanly.

**Cross-step deps**:
- `DreamingResult.breached_ids` + `decay_floor_breaches` → consumed by Step 4 (CLI structured-JSON output) and Step 5 (`verify-decay.sh` per-memory approval gate).
- `run_dreaming_with_config(..., write_demotions: bool)` → Step 4 will flip the bool based on the `--decay-dry-run` CLI flag.
- LONGTERM_BOOST cliff comment at the demotion site cross-references `search.rs:~142`. Step 3 adds the mirror comment at the boost site.

**Actual files**: src/core/dreaming.rs, src/bin/cli.rs

## Step 3 — Search post-fetch decay + same-age-clock invariant (commit: 12d741a)

**Decisions**:
- Extracted `apply_boost_and_decay` as a pure module-private helper. The original plan called for an inline change at `search.rs:142`; hoisting the logic makes it unit-testable without the embedding pipeline. 8 decay tests run in milliseconds; a hybrid_search integration test would need fastembed + 90MB model download.
- `now = chrono::Utc::now()` captured once at the top of the post-fetch loop in `memory_search`. All results in a single response share that timestamp — cheap, simple, and the test can still assert same-age-clock invariant by driving both `apply_boost_and_decay` and `decay::effective_relevance` with the same frozen `now`.
- Non-longterm memories ALSO receive the decay multiplier, not just longterm. This was implicit in the plan but worth naming — a stale non-longterm memory's score should drop too, otherwise search ranking diverges from the "forgetting" semantics.
- Malformed `last_recalled` strings fall back to `1.0` (no decay) — graceful; matches the NULL-recall skip behavior.

**Rejected**:
- Passing `now` as a parameter to `memory_search` (for test determinism). Search is a per-call synchronous API; callers don't thread clocks. Unit test for the invariant goes through the helper directly.
- Recording the BOOSTED score in `record_recall` (as a "signal-the-system-knows" shortcut). Existing behavior preserved: `record_recall` gets the pre-boost, pre-decay normalized RRF score. Reason: `avg_relevance` is a stable long-term signal; if it absorbs the decay, the same memory's stored score drifts with every recall. That poisons the Dreaming pass's later decay computation and creates a recursive bias.

**Cross-step deps**:
- Step 4 (`--decay-dry-run` CLI flag) consumes the demotion path from Step 2 only; search decay is unrelated to dry-run semantics.
- Step 5 (e2e smoke) will exercise `apply_boost_and_decay` indirectly through `memory_search` on the production corpus; Step 3's helper-level tests are sufficient for unit correctness.

**Actual files**: src/core/search.rs

## Step 4 — `--decay-dry-run` CLI flag + structured JSON + ops doc (commit: a6a3b02)

**Decisions**:
- Human line AND structured-JSON line are emitted together every pass, not conditionally. Operators see the human line on stdout; tooling consumes the JSON on stderr via `tracing::info!`. No mode switch — both land always.
- The `demoted` field in JSON stays 0 in dry-run (consistent with struct semantics); the `dry_run: bool` flag on the JSON tells consumers which axis to read. Script `verify-decay.sh` keys off `decay_floor_breaches` for the approval gate because that count is unchanged across modes.
- `format_dreaming_line` extracted as a pure helper to make the AC5 regex contract unit-testable without process-spawning — `std::process::Command` tests are fragile under CI (binary path, env). 4 regex tests cover live/dry-run shape + edge counts.
- `--decay-dry-run --dry-run` explicitly errors out with a helpful message. The two dry-run modes target orthogonal passes; combining them would be ambiguous.

**Rejected**:
- Adding `assert_cmd` / `predicates` dev-deps for full CLI subprocess tests. Overhead outweighs value at this scope — unit-testing the pure `format_dreaming_line` covers the regex contract, and Step 5's e2e smoke exercises the live path end-to-end.
- Capturing tracing output for JSON-line unit test. Requires a custom subscriber harness; skipped because the JSON is a `serde_json::json!` literal — its structure is guaranteed by the macro, not by ordering.
- Making the flag `--dry-run-decay` (verb-object). Gemini's `--decay-dry-run` (subject-first) matches the existing `--synthesize --dry-run` pattern — the `--dry-run` in the existing flag BINDS TO `--synthesize`; the BL-008 flag is subject-scoped and self-contained.

**Cross-step deps**:
- `scripts/verify-decay.sh` (Step 5) parses the Step 4 structured-JSON line for `decay_floor_breaches` + `breaches[]`. The JSON key names are a stable contract — changing them breaks the script.
- `docs/operations/dreaming-decay.md` (also this step) references Step 5's approval gate procedure; they ship together.

**Actual files**: src/bin/cli.rs, docs/operations/dreaming-decay.md

## Step 5 — E2e smoke + verify-decay approval gate + CHANGELOG + roadmap (commit: 522db8a)

**Decisions**:
- E2e smoke uses a parallel `rusqlite::Connection::open` on the same file to force `is_longterm = 1` + specific `(avg_relevance, last_recalled)` values on seeded memories. Integration tests can't reach the private `Db::lock_conn`; the file-based parallel conn is clean and matches SQLite's native concurrency model.
- Test boundary set `{0, 15, 75, 77, 78, 137}` days chosen to pin the floor-boundary arithmetic: d=75 eff≈0.205 survives, d=77 eff≈0.2001 survives, d=78 eff≈0.1977 demotes. Catches any off-by-one in the integer-day math future implementers might introduce.
- `verify-decay.sh` parses via `jq` when present, sed fallback otherwise — no new runtime dep.
- CHANGELOG format: Keep-a-Changelog with `## Unreleased` section; version assignment deferred to first release cut. Currently the project has no tagged versions, so the unreleased-only format is correct.
- Phase 2 roadmap entry for BL-008 expanded with final constants (60-day half-life, floor=0.20) plus revisit triggers — keeps the roadmap authoritative for future scope readers.

**Rejected**:
- Seeding 41 long-term memories (original plan). 6 at the critical boundaries tests the same invariants without the setup overhead.
- A production-smoke test that runs against `~/.mengdie/db.sqlite` directly in CI. That path isn't stable in CI/sandbox envs. `verify-decay.sh` is the live-corpus entry point; e2e uses a tmp DB.
- A unit test for `verify-decay.sh`'s JSON extraction. Shell script testing is out of scope; operator procedure lives in the ops doc.

**Cross-step deps**:
- Plan 013 complete — all 5 steps merged. `status: reviewed` stays (ae:review flips to `done`). Discussion 019's `pipeline.work: done` writeback is the Completion Invariant's responsibility below.

**Actual files**: tests/e2e.rs, scripts/verify-decay.sh, CHANGELOG.md, docs/backlog/005-phase2-roadmap.md
