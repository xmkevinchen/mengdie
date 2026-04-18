---
id: "011"
title: "BL-residuals-reduction — min_size=2 + null-escape-hatch"
type: plan
created: 2026-04-18
status: reviewed
discussion: "docs/discussions/018-residuals-reduction/"
---

<!--
Plan review summary (bl-residuals-plan-review team, 2026-04-18):
  Reviewers: architect, dependency-analyst, engineering-ai-engineer (imported),
  cross-family-fallback (claude sonnet via challenger kit — codex/gemini both
  unavailable this session per CLAUDE.md cross-family fallback).

  Must Fix (all applied):
    - SYSTEM_PROMPT line reference corrected: synthesis.rs:5 → synthesis.rs:4
    - EXPECTED_SYSTEM_PROMPT byte-for-byte update at synthesis.rs:166 explicitly
      called out in Step 2 (dep-analyst: was implicit, would have silently
      failed the regression guard).
    - pair_clusters_processed counted by trimmed_ids.len() pre-DB-load, NOT
      memories.len() post-load (architect: DB-miss edge case would have caused
      silent denominator undercount).
    - Step 3 dissolved: AC5 now verifiable at review time via a committed
      TODO-shaped stub section in BL-clustering-validation.md. Live audit moved
      to an explicit "Post-ship follow-up" section outside the plan's /ae:review
      gate (cross-family: the original Step 3 was a dangling gate that could
      not be verified mechanically and would dead-letter).

  Consider (applied):
    - pair_clusters_processed stays local (tuple return), not on SynthesisResult
      (cross-family: derived display value, no external caller).
    - AC3 fixture uses skip-returning stub provider, NOT --dry-run (architect:
      dry-run bypasses LLM path and can't exercise Skipped variant).
    - Reversibility note distinguishes prompt-only vs full-enum revert paths
      (architect).
    - CLI line shows both absolute counts AND percentage: "N/M pair-clusters
      = X%" (ai-engineer: denominator disambiguation — "50%" alone could be
      1/2 or 50/100).

  Consider (acknowledged, not applied):
    - Skipped could carry source_memory_ids (ai-engineer): low-priority, the
      info-level log line at call site includes cluster_ids, and the counter is
      the primary audit signal. Revisit only if AC5 writeback ergonomics prove
      clunky.
    - Prompt trigger sharpness may cause Sonnet 4.6 to under-reject the 30%
      topic-adjacent pairs, landing skip rate at 10-20% instead of 30%
      (ai-engineer): acknowledged; AC5 post-ship audit is the tuning loop,
      no pre-ship prompt change.
-->


# Feature: BL-residuals-reduction — min_size=2 + null-escape-hatch

## Goal

Drop dream-synthesis residual rate (57-67% → ~35-45% expected) by flipping
`DEFAULT_MIN_SIZE` 3→2 so pair-clusters synthesize, AND instruct the LLM to
return `{"skip": true, "reason": "..."}` on weakly-related pairs so the 30%
topic-adjacent share becomes clean rejections instead of noisy syntheses.
Bundled in one PR (2 code steps + 1 post-ship audit step) because the
escape hatch is load-bearing for the flip to ship without quality regression.

## Prior Art (from Mengdie KB)

- `[discuss]: Bundle parameter flip with null-escape-hatch when empirical data shows "mixed" ratio` (discussions/018, 2026-04-18) — the decisional basis for this plan. 60% near-dup + 30% topic-adj spot-check reframed by the hatch to ~90% useful.
- `[plan]: expose residuals alongside clusters` (plans/009, 2026-04-18) — the `residuals: Vec<String>` contract this plan leans on. No breakage.
- `[plan]: Greedy cosine clustering for all-MiniLM-L6-v2 — threshold 0.75 is SBERT's community_detection default` (plans/009, 2026-04-18) — threshold unchanged; only min_size moves in this plan. Constraint: do not drop threshold below 0.70 without quality validation.
- `[discuss]: /ae:analyze sweep before /ae:discuss when the question is "which parameter value?"` (discussions/018, 2026-04-18) — the empirical basis (7-combo parameter sweep + 10-cluster spot-check) is why this plan is direct rather than exploratory.

## Scope boundaries

- **In**: `DEFAULT_MIN_SIZE` constant flip + doc comment; `SYSTEM_PROMPT` update; `SynthesisOutcome` enum replacing `SynthesisDraft` as `parse_synthesis_response` return; new `syntheses_llm_skipped` counter in `SynthesisResult`; `run_synthesis_pass` Skipped-variant handling; CLI output line with pair-cluster skip %; migration of ≥10 existing BL-007 tests; `BL-clustering-validation.md` trigger wording update; post-ship audit writeback as an additional step.
- **Out**: threshold change (0.75→0.70) — deferred pending real-LLM quality data at new defaults. Second-pass clustering — deferred, null-escape covers the case. Ingestion-time plan/backlog dedup (the root-cause fix ai-engineer surfaced) — separate future discussion. `synthesis_hit_rate` instrumentation — separate future plan when search logging is needed.

## Synthesis Flow (post-flip; minor delta from BL-007)

```
mengdie dream [--synthesize]
  ├─ promotion pass (unchanged)
  └─ if --synthesize:
       ├─ cluster_memories(db, project, 0.75, 2)   ← min=2 this plan
       ├─ for each Cluster:
       │    ├─ build prompt (SYSTEM_PROMPT updated this plan)
       │    ├─ provider.complete(system, user)
       │    ├─ parse_synthesis_response → SynthesisOutcome
       │    │    ├─ Synthesized(draft) → insert row + links (as today)
       │    │    └─ Skipped { reason } → no row, no links,
       │    │                             increment syntheses_llm_skipped,
       │    │                             log at info with reason
       │    └─ (continue to next cluster)
       └─ Print: "Synthesis: N created, S LLM-skipped (X% of pair-clusters),
                 K residuals skipped, E errors"
```

## Steps

### Step 1: Flip `DEFAULT_MIN_SIZE` 3→2 + backlog trigger rewording (AC1)

- [ ] Change `DEFAULT_MIN_SIZE: usize = 3` → `2` in `src/core/clustering.rs:23`.
- [ ] Add a doc comment on the constant (alongside the existing `DEFAULT_THRESHOLD` justification):
  ```rust
  /// DEFAULT_MIN_SIZE = 2: lowered from 3 (discussion 018) based on
  /// empirical spot-check — ~60% of pair-clusters in a solo-dev AE
  /// corpus are near-duplicates (plan↔backlog, discuss↔conclusion,
  /// analyze↔analyze pairs) that benefit from consolidation. The
  /// remaining ~30% topic-adjacent pairs are filtered by the
  /// null-escape-hatch in synthesis.rs (see Step 2).
  ///
  /// Revisit if, across 3–5 dream runs: skip rate > 25% of
  /// pair-clusters OR manual spot-check shows majority-weak
  /// syntheses. Ladder: first revert min_size=3; if residuals
  /// stay > 50%, pursue threshold drop to 0.70 with real-LLM
  /// validation first.
  pub const DEFAULT_MIN_SIZE: usize = 2;
  ```
- [ ] Update `docs/backlog/BL-clustering-validation.md` trigger #2 wording:
  - Old: ">50% residuals = signal"
  - New: ">50% residuals AND synthesis_hit_rate < 10% = signal
    (synthesis_hit_rate instrumentation deferred; use residual-%
    only until search logging exists)"
- [ ] Verify: `cargo test --lib clustering::` passes. `cargo clippy --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean.

Expected files: `src/core/clustering.rs`, `docs/backlog/BL-clustering-validation.md`

### Step 2: Null-escape-hatch — SYSTEM_PROMPT + SynthesisOutcome + counter + CLI (AC2, AC3, AC4, AC5-stub)

- [ ] Update `SYSTEM_PROMPT` in `src/core/synthesis.rs:4` (literal new text, keep as a single `const`):
  ```rust
  const SYSTEM_PROMPT: &str = "You are consolidating related engineering memories. Most clusters have a genuine common thread; when they do, output ONLY a JSON object with keys title, content, entities. title ≤ 80 chars. content 3–6 sentences, self-contained, cites the underlying decisions without naming file paths. entities is an array of 2–6 compound tags (lowercase, hyphen-separated, no spaces). No markdown, no prose outside the JSON. If the memories do NOT share a meaningful common thread (they are merely adjacent topics or share vocabulary without shared intent), output exactly the JSON object {\"skip\": true, \"reason\": \"<one short sentence>\"} instead. Do not invent a consolidation when none exists.";
  ```
- [ ] Update the regression-guard `const EXPECTED_SYSTEM_PROMPT` at `src/core/synthesis.rs:166` (inside `#[cfg(test)] mod tests`) to match byte-for-byte. Referenced by two test assertions at `synthesis.rs:204` and `synthesis.rs:253`; both tests will fail loudly if the prod prompt and test const drift out of sync, which is the intended regression guard.

- [ ] Introduce `SynthesisOutcome` enum in `synthesis.rs`, replacing `SynthesisDraft` as the return type of `parse_synthesis_response`:
  ```rust
  pub enum SynthesisOutcome {
      Synthesized(SynthesisDraft),
      Skipped { reason: String },
  }
  pub fn parse_synthesis_response(
      raw: &str,
      source_ids: &[String],
  ) -> Result<SynthesisOutcome, SynthesisError>;
  ```
  `SynthesisDraft` struct keeps its existing shape (title, content, entities,
  source_memory_ids). Parse logic:
  1. Run the existing brace-depth extractor to isolate the JSON object.
  2. Deserialize into `RawEnvelope { skip: Option<bool>, reason: Option<String>, title: Option<String>, content: Option<String>, entities: Option<Vec<String>> }`.
  3. If `skip == Some(true)` → return `Ok(SynthesisOutcome::Skipped { reason: reason.unwrap_or_default() })` (empty reason is allowed; parser never hard-errors on missing reason).
  4. Else: validate title+content+entities as today and return `Ok(Synthesized(draft))`.

- [ ] In `src/core/dreaming.rs`: extend `SynthesisResult` with `pub syntheses_llm_skipped: usize`. Keep existing fields (`clusters_processed`, `syntheses_created`, `llm_call_errors`, `parse_errors`, `residuals_skipped`, `memories_truncated`).

- [ ] In `run_synthesis_pass` (`src/core/dreaming.rs`), handle the new Skipped variant from `parse_synthesis_response`:
  - Match on `SynthesisOutcome`: `Synthesized(draft)` → existing insert-row path; `Skipped { reason }` → log at `info` level with `cluster_ids` + `reason` + cluster_size (so `reason` is observable per-run, not only aggregated). Increment `syntheses_llm_skipped`. Do NOT insert memory row. Do NOT insert link rows.
  - Pair-cluster attribution: track `pair_clusters_processed: usize` as a **local variable** inside `run_synthesis_pass` (not a new field on `SynthesisResult`; it's a derived display value with no external caller). Count by `trimmed_ids.len() == 2` **pre-DB-load**, not by `memories.len()` post-load (architect must-fix: a 2-member cluster where DB bulk-load returns fewer than 2 rows falls through the `if memories.len() < min_size` continue and would silently miss both `syntheses_llm_skipped` and the pair-cluster denominator — count pre-load so the denominator is consistent with `clusters_processed`, which is also pre-loop, pre-load). Pass the local count to the CLI printer via a return tuple or small helper; do not promote to a public struct field.

- [ ] Update the `cmd_dream` CLI output in `src/bin/cli.rs` to the new format. `pair_clusters_processed` is returned from `run_synthesis_pass` as a local-variable tuple component (not on `SynthesisResult`; see pair-cluster attribution note above):
  ```rust
  let (syn, pair_clusters_processed) = run_synthesis_pass(...).await?;
  let pair_skip_pct = if pair_clusters_processed == 0 {
      0
  } else {
      (syn.syntheses_llm_skipped * 100) / pair_clusters_processed
  };
  println!(
      "Synthesis: {} syntheses created from {} clusters \
       ({} residuals skipped, {} LLM-skipped ({}/{} pair-clusters = {}%), \
        {} LLM-call errors, {} parse errors, {} memories truncated)",
      syn.syntheses_created,
      syn.clusters_processed,
      syn.residuals_skipped,
      syn.syntheses_llm_skipped,
      pair_clusters_processed,
      pair_skip_pct,
      syn.llm_call_errors,
      syn.parse_errors,
      syn.memories_truncated
  );
  ```
  The line now shows both absolute counts AND the percentage (ai-engineer Consider: denominator disambiguation — "50%" alone could be 1/2 or 50/100; the absolute pair denominator resolves this).

- [ ] **API migration (Step 2 drift surface)**: every existing test or internal caller that destructures `SynthesisDraft` directly now needs to pattern-match `SynthesisOutcome::Synthesized(draft)`. Grep `SynthesisDraft {` across `src/` + `tests/` — expected hits: ~10 test assertions in `src/core/synthesis.rs` tests module + usage inside `run_synthesis_pass`'s happy path + e2e test in `tests/dream_synthesis.rs`. This is mechanical; batch-edit with a single sweep.

- [ ] New unit tests in `src/core/synthesis.rs`:
  1. `parser_skip_happy_path` — `{"skip": true, "reason": "unrelated"}` → `Ok(SynthesisOutcome::Skipped { reason: "unrelated" })`.
  2. `parser_skip_missing_reason` — `{"skip": true}` → `Ok(SynthesisOutcome::Skipped { reason: "" })`.
  3. `parser_skip_with_llm_preamble` — `"Here you go:\n\n{\"skip\":true,\"reason\":\"...\"}"` → parses cleanly (brace-depth extractor still works).
  4. `parser_skip_false_is_synthesis` — `{"skip": false, "title": "T", "content": "...", "entities": []}` → `Synthesized(_)` (belt-and-suspenders: explicit false means "do synthesize").

- [ ] New unit tests in `src/core/dreaming.rs`:
  1. `test_synthesis_skip_increments_counter_no_db_write` — stub provider returns the skip JSON for one cluster → `syntheses_llm_skipped == 1`, `syntheses_created == 0`, no row in `memory_entries`, no rows in `memory_synthesis_links`.
  2. `test_synthesis_pair_skip_percentage_computed_against_pairs` — 4 clusters (2 pair, 2 triple), stub skips 1 pair cluster only → printed/returned pair_skip_pct = 50 (1 skip / 2 pair-clusters), NOT 25 (1 skip / 4 total).

- [ ] **AC5 stub** — commit a placeholder `## BL-residuals-reduction empirical results` section to `docs/backlog/BL-clustering-validation.md` with literal `TODO` fields:
  ```markdown
  ## BL-residuals-reduction empirical results

  _Populated after the first real `mengdie dream --synthesize` run at the new defaults. Do NOT delete this section; either fill it in or explicitly mark "insufficient data, re-run"._

  - **Run date**: TODO
  - **Corpus size at run**: TODO memories
  - **Skip rate**: TODO % of pair-clusters (target < 25% hatch working, 25-40% monitor, > 40% revisit min_size)
  - **Skip classification (N skips audited)**: CORRECT-SKIP: TODO, FALSE-NEGATIVE: TODO, UNCLEAR: TODO
  - **Non-skipped synthesis quality (3 spot-checked)**: good / mixed / poor — TODO
  - **Decision**: keep new defaults / revert min_size / revert prompt only / tune further — TODO
  ```
  This makes AC5 verifiable at `/ae:review` time (the stub exists and grep for `TODO` succeeds); the live audit itself is a post-ship follow-up task outside the plan cycle.

- [ ] Verify: `cargo test` passes (all existing + new). `cargo clippy --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean.

Expected files: `src/core/synthesis.rs`, `src/core/dreaming.rs`, `src/bin/cli.rs`, `tests/dream_synthesis.rs` (mechanical `SynthesisOutcome::Synthesized` wrapping), `docs/backlog/BL-clustering-validation.md` (AC5 stub)

## Post-ship follow-up (NOT a plan step — outside /ae:review gate)

After this plan completes and the first real `mengdie dream --synthesize` runs at the new defaults:

1. Run the dream command, capture stdout.
2. Audit ALL skipped clusters (target N≥5; if N<5, re-run after next dream pass before concluding).
3. Classify each skip as CORRECT-SKIP / FALSE-NEGATIVE / UNCLEAR by reading source memory_ids' titles and content.
4. Spot-check 3 non-skipped syntheses for quality.
5. Fill in the `TODO` fields in the stub section committed by Step 2.
6. Decide: keep defaults / revert min_size / revert prompt only / tune further.

Executor: record this as a scheduled task / calendar event so it happens. The stub's `Do NOT delete this section` instruction is the tripwire that catches a forgotten audit.

## Parallel strategy

Step 1 and Step 2 have nearly-zero file overlap — Step 1 touches `clustering.rs` + `BL-clustering-validation.md` (trigger wording); Step 2 touches `synthesis.rs`, `dreaming.rs`, `cli.rs`, `tests/dream_synthesis.rs`, AND `BL-clustering-validation.md` (AC5 stub, appends a new section). The two edits to `BL-clustering-validation.md` are in different sections (Step 1 edits trigger #2 wording mid-file; Step 2 appends a new `## BL-residuals-reduction empirical results` section at end) — no merge conflict in practice, but the executor should land Step 1 first and Step 2 rebased on top to avoid auto-merge noise. Both steps in one PR, two commits. Parallel dev is possible but sequential commit is cleaner for the markdown edits.

## Reversibility

- **Step 1 alone (parameter flip revert)**: one-line revert of `DEFAULT_MIN_SIZE` back to 3 + doc-comment removal. Zero migration cost. Default CLI flag `--min-cluster-size` lets any run opt to either value without code change.
- **Step 2 prompt-only revert** (keep enum, restore old prompt): restore `SYSTEM_PROMPT` + `EXPECTED_SYSTEM_PROMPT` to their pre-plan text. `SynthesisOutcome::Skipped` variant simply never triggers (the LLM has no instruction to emit skip JSON). One `const` edit. No test migration rollback needed. This is the expected revert path if audit shows the LLM over-rejects (skip rate > 25% with false-negative classifications dominating).
- **Step 2 full revert** (unwind enum): restore `parse_synthesis_response` return type to `Result<SynthesisDraft, SynthesisError>` and unwind the ~12 `SynthesisOutcome::Synthesized(_)` wrappings in the test module + `run_synthesis_pass`. Mechanical sweep, ~15 minutes. Only needed if the enum itself is structurally unwanted (e.g., a future refactor wants a different variant set).
- **AC5 stub removal**: delete the appended section in `BL-clustering-validation.md`. Trivial.

## Acceptance Criteria

### AC1: `DEFAULT_MIN_SIZE` flip is correct and reversible

- `src/core/clustering.rs` contains `pub const DEFAULT_MIN_SIZE: usize = 2;`
- Doc comment on the constant references "discussion 018" and includes the "3–5 dream runs; skip rate > 25% or majority-weak spot-check" revisit trigger verbatim.
- `docs/backlog/BL-clustering-validation.md` trigger wording updated to the new conditional form ("AND synthesis_hit_rate < 10%") with the "(pending instrumentation)" annotation.
- `cargo test --lib clustering::` passes (no existing clustering test should regress — the constant change only affects defaults, not any explicitly-specified argument).
- Reverting is exactly one line + doc-comment removal; no data migration required.

### AC2: Null-escape-hatch path works end-to-end

- `SynthesisOutcome` enum with `Synthesized(SynthesisDraft)` and `Skipped { reason: String }` variants exists.
- `parse_synthesis_response` returns `Result<SynthesisOutcome, SynthesisError>` (changed from `Result<SynthesisDraft, SynthesisError>`).
- `SYSTEM_PROMPT` contains the skip-instruction sentence exactly as specified in Step 2.
- `EXPECTED_SYSTEM_PROMPT` regression-guard matches byte-for-byte.
- Unit tests in `src/core/synthesis.rs`: skip-happy, skip-missing-reason, skip-with-preamble, skip-false-is-synthesis — all pass.
- Unit tests in `src/core/dreaming.rs`: stub returning skip-JSON → `syntheses_llm_skipped == 1` AND zero new rows in `memory_entries` or `memory_synthesis_links`.
- `SynthesisResult` struct has `syntheses_llm_skipped: usize` field that matches the counter incremented by the orchestration pass.

### AC3: Pair-cluster skip percentage is printed at run completion

- The `cmd_dream` stdout format contains a line matching the regex:
  `Synthesis: \d+ syntheses created from \d+ clusters \(\d+ residuals skipped, \d+ LLM-skipped \(\d+/\d+ pair-clusters = \d+%\), \d+ LLM-call errors, \d+ parse errors, \d+ memories truncated\)`
- The percentage uses pair-clusters (`trimmed_ids.len() == 2` at pass time) as the denominator, NOT total clusters.
- Verified by a unit test in `src/core/dreaming.rs` (`test_synthesis_pair_skip_percentage_computed_against_pairs`) that runs a **stub provider returning the skip JSON** for one of the pair-clusters (not `--dry-run`, which bypasses the LLM path entirely and would never exercise the Skipped variant). Fixture: 4 clusters (2 pair of 2 members, 2 triple of 3 members), stub returns `{"skip":true,"reason":"test"}` for one pair cluster and valid synthesis JSON for the rest. Expected output: `pair_clusters_processed == 2`, `syntheses_llm_skipped == 1`, printed `"1/2 pair-clusters = 50%"`.
- `grep LLM-skipped <output>` returns a non-empty match.

### AC4: API migration is clean — no regression in existing BL-007 tests

- `cargo build` succeeds.
- `cargo test` — every existing test in `src/core/synthesis.rs` (≥10 parser/prompt tests from BL-007), `src/core/dreaming.rs` (5 synthesis-pass tests from BL-007), and `tests/dream_synthesis.rs` (1 ignored e2e) continues to pass after migration to `SynthesisOutcome::Synthesized(_)`.
- `cargo clippy --all-targets -- -D warnings` clean — no new `#[allow]` attributes added.
- `cargo fmt --all -- --check` clean.

### AC5: Audit stub committed (verifiable at review time)

- `docs/backlog/BL-clustering-validation.md` contains a new section titled `## BL-residuals-reduction empirical results` with at least these 6 lines of `TODO`-shaped placeholders (run date, corpus size, skip rate, skip classification counts, non-skipped quality judgment, decision) and a "Do NOT delete this section" preamble. Verifiable via `grep '## BL-residuals-reduction empirical results' docs/backlog/BL-clustering-validation.md` + `grep -c 'TODO' docs/backlog/BL-clustering-validation.md` returning ≥ 6.
- The actual data (live skip rate, classification, quality judgment, decision) is a post-ship operator task documented in the plan's "Post-ship follow-up" section, NOT a prerequisite for `/ae:review` to pass. The stub is the forcing function: a live dream run that leaves the section untouched means the audit was skipped, and the `TODO` count in grep provides a discoverable signal.

## Non-goals (explicit)

- **No threshold change in this plan**. Deferred pending AC5 data.
- **No second-pass clustering strategy**. Deferred — the null-escape handles the pair-adjacency case and collapses the second-pass value proposition.
- **No `synthesis_hit_rate` instrumentation**. Deferred — the backlog entry is updated to note this as pending; separate plan when search logging becomes a feature need.
- **No CLI `--enable-null-escape` flag** (ai-engineer Round 3 proposal). Post-ship qualitative audit is the cheaper measurement-attribution path.
- **No new `SynthesisMode` enum on the prompt builder**. The prompt is one string (with the skip branch inline); a mode enum would be engineering scope-creep for a single-prompt change.
- **No pre-LLM cosine-cohesion filter**. Considered as a Step 4 addition but deferred — the LLM's self-judgment via skip is the first-pass filter; pre-filter would add code for marginal gain until/unless skip rate is persistently high.
- **No ingestion-time plan/backlog dedup**. ai-engineer flagged the plan↔backlog pairs as root-cause at the ingestion layer; separate future discussion.
