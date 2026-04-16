---
id: "004"
title: "Search Quality Fixes: Dreaming Threshold + FTS5 Tokenization"
type: plan
created: 2026-04-16
status: reviewed
discussion: "docs/discussions/013-what-next-after-pause/"
---

# Feature: Search Quality Fixes — Dreaming Threshold + FTS5 Tokenization

## Goal
Fix two compound bugs that break the core knowledge spiral: (1) Dreaming threshold unreachable due to RRF normalization ceiling, (2) FTS5 phrase-only matching kills multi-term recall.

## Prerequisites
- Run 5 pre-fix benchmark queries and log results (per discussion 013 Topic 3 Step 0) — establishes before/after baseline. This is manual and outside /ae:work scope.

## Steps

### Step 1: Lower Dreaming threshold from 0.65 to 0.45 (AC1a, AC2)
Note: `RRF_MAX = 2.0/61.0` stays unchanged per discussion 013 (Option A — preserve dual-signal semantics).
- [x] Change `DEFAULT_MIN_RELEVANCE` from `0.65` to `0.45` in `src/core/dreaming.rs:11`
- [x] Change `cli.rs` Dream command `min_relevance` field to use `default_value_t = mengdie::core::dreaming::DEFAULT_MIN_RELEVANCE` instead of string literal `"0.65"`
- [x] Apply same `default_value_t` pattern for `min_recall` (link to `mengdie::core::dreaming::DEFAULT_MIN_RECALL`) and `window_days` (link to `mengdie::core::dreaming::DEFAULT_WINDOW_DAYS`) to prevent future divergence
- [x] Update e2e test comment at `tests/e2e.rs:70` to reference new threshold (`>= 0.45` instead of `>= 0.65`)
- [x] Update e2e test assertion at `tests/e2e.rs:77` to assert `> 0.45` (still passes — 9 manual recalls at 0.9 push avg well above 0.45)
Expected files: src/core/dreaming.rs, src/bin/cli.rs, tests/e2e.rs

### Step 2: Replace FTS5 phrase wrapping with AND-term matching (AC3, AC4)
- [x] Add a `sanitize_fts_query` function in `src/core/search.rs` (above `search_fts`)
- [x] Sanitization pipeline (allowlist approach): (a) `split_whitespace()`, (b) for each token: retain only `char::is_alphanumeric()` chars (strips all FTS5 operators including `"`, `*`, `-`, `+`, `^`, `:`, `{`, `}`, `(`, `)`, `,`, `/`), (c) filter empty tokens, (d) filter reserved words `AND`, `OR`, `NOT`, `NEAR` (case-insensitive), (e) join remaining tokens with ` AND `
- [x] Empty result after sanitization → return empty Vec (no FTS query, same as empty-query path at line 38-39)
- [x] Replace the phrase-wrapping logic at `src/core/search.rs:42-44` to call `sanitize_fts_query` instead
- [x] Add unit tests for `sanitize_fts_query`: multi-word query, single-word query, query with FTS operators (`AND`, `OR`), query with special chars (`"rust *** memory"` → `"rust AND memory"`), query that strips to empty (`"***"`), query with consecutive spaces, mixed-case reserved words (`"rust And memory"` → `"rust AND memory"`), query with all reserved words (`"AND OR NOT"` → empty)
Expected files: src/core/search.rs

### Step 3: Update existing search tests (AC3, AC4)
Depends on: Step 2 (tests assert new AND-term matching behavior).
- [x] Update `test_memory_search_scores_normalized` (search.rs:329-344): query `"JWT auth tokens"` should now match via both FTS5 AND vector → score should be higher than current ~0.5. Adjust assertion to test that dual-ranker hits produce scores > 0.5 (confirming FTS5 is now contributing)
- [x] Add new test: multi-word FTS5 query returns results when words appear non-adjacently in content (e.g., title="JWT tokens", content="for authentication" — query "JWT authentication" should match via FTS5)
- [x] Verify all existing tests still pass after both changes
Expected files: src/core/search.rs, tests/e2e.rs

### Step 4: Build, test, and verify promotions (AC1a, AC1b, AC2, AC5)
Depends on: Steps 1 and 3.
- [x] Build: `cargo build`
- [x] Run: `cargo test` — 86 passed, 1 ignored
- [x] Run: `cargo clippy` — no new warnings
- [x] Run: `./target/debug/mengdie dream` — 13 promoted out of 54 eligible
- [x] Query DB to confirm `is_longterm = 1` on 13 promoted entries
Expected files: (no code changes — verification step)

## Acceptance Criteria

### AC1a: Dreaming Threshold Constant Updated (unit-testable)
`DEFAULT_MIN_RELEVANCE` constant = 0.45. Existing tests `test_dreaming_promotes_qualifying` (5 recalls at 0.8) and `test_dreaming_skips_low_recall` (1 recall at 0.9) both still pass. E2e test assertion updated to `> 0.45`.

### AC1b: Dreaming Promotes on Real DB (runtime, manual)
`./target/debug/mengdie dream` on the production DB promotes at least 1 entry (exact count depends on DB state; discussion 013 found 11 qualifying).

### AC2: CLI Default Matches Constant
`mengdie dream --help` shows `--min-relevance` default as `0.45` (not `0.65`). Default values for `--min-recall` and `--window-days` also display correctly from their respective constants.

### AC3: Multi-Word FTS5 Recall
Query `"JWT authentication"` against a corpus containing a memory with title "JWT tokens" and content "for authentication" returns that memory via FTS5 (not just vector). Verified by unit test.

### AC4: FTS5 Operator Safety
`sanitize_fts_query("rust *** memory")` returns `"rust AND memory"`. `sanitize_fts_query("AND OR NOT")` returns `""`. Queries containing any FTS5 syntax (`+`, `^`, `:`, `NEAR/5`, `{col}`, `"phrase"`) are sanitized to safe alphanumeric tokens. Verified by unit tests on the function output.

### AC5: All Existing Tests Pass
`cargo test` passes. `cargo clippy` has no warnings. No regression in existing behavior.
