---
id: "012"
title: "BL-synthesis-cli-skip-metric — fix pair-cluster skip rate display"
type: plan
created: 2026-04-19
status: reviewed
discussion: ""
---

# Feature: `mengdie dream --synthesize` pair-cluster skip rate — fix the math

## Goal

Fix the CLI output so the displayed "N/M pair-clusters = X%" metric is the
true pair-cluster skip rate, by adding `pair_clusters_skipped` and
`pair_clusters_processed` as fields on the existing `SynthesisResult`
struct and removing the tuple return.

## Context

The 2026-04-19 post-ship audit (`docs/backlog/BL-clustering-validation.md`
AC5 section) found the CLI displayed `(11/11 pair-clusters = 100%)` when
the true pair-cluster skip rate was 3/11 = 27%. Root cause in
`src/bin/cli.rs:248-262`: numerator `syntheses_llm_skipped` counts skips
across ALL cluster sizes, denominator `pair_clusters_processed` is
pair-only. Details in `docs/backlog/BL-synthesis-cli-skip-metric.md`.

### Plan review reshape (Challenger C, concurred by architect + code-reviewer)

The initial draft proposed a two-step plan: (1) promote
`(SynthesisResult, usize)` → `SynthesisPassResult` wrapper struct, migrating
11 tuple destructure sites; (2) add the counter and fix the CLI. The plan
review's challenger argued — with architect + code-reviewer concurring —
that the wrapper struct is unnecessary: since `SynthesisResult` already
holds display-layer counters (`syntheses_llm_skipped`, `llm_call_errors`,
etc.), putting the two pair_* counters there too is the simpler shape.
The wrapper would separate a metric's numerator (on wrapper) from its
denominator sibling (off wrapper, as tuple `.1`) — a coherence regression.
Collapsing both counters onto `SynthesisResult` keeps the metric's two
fields co-located and drops the return type from `Result<(SynthesisResult,
usize)>` to `Result<SynthesisResult>`.

This supersedes `BL-synthesis-result-struct-promotion.md` — its trigger
("second display-layer counter becomes tuple `.2`") is avoided entirely.

### Coherence note (acknowledged tradeoff)

Architect's review flagged a minor coherence point: `SynthesisResult`
becomes a hybrid of "synthesis outcome" (created, errors, skips,
truncations) + "cluster geometry observation" (pair count, pair skips).
Acceptable at current scale with one caller (CLI). Revisit if a caller
outside the CLI ever reads these fields for non-display purposes.

## Steps

### Step 1: Add pair_* fields to SynthesisResult, remove tuple return, fix CLI, add discrimination test (AC1, AC2, AC3) ✓ (commit 63d83b0)

- [x] Add two fields to `SynthesisResult` in `src/core/dreaming.rs:141`:
  - `pub pair_clusters_processed: usize`
  - `pub pair_clusters_skipped: usize`
- [x] Change `run_synthesis_pass` return type from `anyhow::Result<(SynthesisResult, usize)>` to `anyhow::Result<SynthesisResult>`
- [x] Update doc comment at `dreaming.rs:180-184` — remove tuple language, describe fields
- [x] In `run_synthesis_pass`:
  - Replace local `let mut pair_clusters_processed: usize = 0;` (line 199) with increment of `result.pair_clusters_processed` at the existing increment site (line 232)
  - In the `SynthesisOutcome::Skipped` match arm (around line 325), add: `if trimmed_ids.len() == 2 { result.pair_clusters_skipped += 1; }` next to the existing `result.syntheses_llm_skipped += 1`
  - Change final `Ok((result, pair_clusters_processed))` to `Ok(result)`
- [x] Migrate all callsites (compiler-enforced):
  - `src/bin/cli.rs:237` — `let (syn, pair_clusters_processed) = ...` → `let syn = ...`
  - `src/bin/cli.rs:248-262` — numerator changes from `syn.syntheses_llm_skipped` to `syn.pair_clusters_skipped`; denominator accesses `syn.pair_clusters_processed`. Format string keeps the same `"{} LLM-skipped ({}/{} pair-clusters = {}%)"` shape.
  - `tests/dream_synthesis.rs` — `let (result, _pair_clusters_processed) = ...` → `let result = ...`
  - All ~9 test callsites in `src/core/dreaming.rs` — `let (r, _) = ...` / `let (r, pair_count) = ...` → `let r = ...`; update `pair_count` references to `r.pair_clusters_processed`
- [x] Add test `test_pair_clusters_skipped_excludes_non_pair_skips` in `src/core/dreaming.rs` tests module: seeds 2 pair-clusters + 2 triple-clusters, uses `FixedProvider::new(SKIP_JSON)` (all 4 skip), asserts the discrimination (see AC2)
- [x] Update existing `test_synthesis_pair_skip_percentage_computed_against_pairs`: add assertion `r.pair_clusters_skipped == 2` alongside the existing `pair_count == 2` check (now accessed as `r.pair_clusters_processed`). Update test body to reflect the single-return shape.
- [x] `cargo test` green — expect 186 tests (one new)
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `cargo fmt --check` clean
- [x] Manual smoke test: run `./target/release/mengdie dream --synthesize` on production DB; confirm the displayed numerator matches the true pair-cluster skip count (manual recount from RUST_LOG=info stderr log lines with `cluster_size=2`)

Expected files: src/core/dreaming.rs, src/bin/cli.rs, tests/dream_synthesis.rs

## Acceptance Criteria

### AC1: Behavior-neutral field addition for existing metrics
After Step 1, all pre-existing 185 tests pass unchanged — no existing test
assertion values change. `cargo clippy --all-targets -- -D warnings` is
clean. `cargo fmt --check` is clean. A reviewer reading the diff should see:
(a) two new `usize` fields on `SynthesisResult`, (b) removal of the local
`pair_clusters_processed` binding with field-increment equivalents, (c) the
return type simplification, (d) mechanical callsite migrations.

### AC2: `pair_clusters_skipped` counts only size==2 skips (discrimination)
Reference test `test_pair_clusters_skipped_excludes_non_pair_skips`:
- Seeds 2 pair-clusters (2 memories each) + 2 triple-clusters (3 memories each)
- Uses `FixedProvider::new(SKIP_JSON)` so all 4 clusters take the Skipped branch
- Asserts `r.pair_clusters_processed == 2`
- Asserts `r.pair_clusters_skipped == 2` (not 4 — the 2 triple-cluster skips MUST NOT increment this counter)
- Asserts `r.syntheses_llm_skipped == 4` (total skips across all sizes)
- A buggy implementation that incremented `pair_clusters_skipped` on every skip would yield `pair_clusters_skipped == 4` — this test discriminates the bug.

### AC3: CLI displays true pair-cluster skip rate
After Step 1, grep `src/bin/cli.rs`:
- The pair_skip_pct arithmetic references `syn.pair_clusters_skipped` as numerator (NOT `syn.syntheses_llm_skipped`)
- The denominator references `syn.pair_clusters_processed`
- The format string retains the shape `"{} LLM-skipped ({}/{} pair-clusters = {}%)"`
- Manual smoke-test on production DB: the displayed numerator equals the count of "LLM skipped cluster" log lines with `cluster_size=2` at RUST_LOG=info. On the 2026-04-19 audit corpus, the displayed number should read 3 (not 11), giving 3/11 = 27% instead of 11/11 = 100%.
