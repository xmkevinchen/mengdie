---
id: "013"
title: "BL-008 — Exponential Decay for Dreaming (formula + demotion + search re-rank + observability)"
type: plan
created: 2026-04-20
status: done
discussion: "docs/discussions/019-power-law-decay/"
---

# Feature: BL-008 — Exponential Decay for Dreaming

## Goal

Ship forgetting — compute `effective_relevance = avg_relevance × 2^(-d/60)` on the fly from `last_recalled`, demote long-term memories whose effective relevance falls below 0.20, apply the same decay as a search post-fetch re-rank multiplier, and surface the mechanism through operator-visible counters + a pre-mutation dry-run flag.

Source: `docs/discussions/019-power-law-decay/conclusion.md` (2026-04-20, 5 decisions, all converged with Doodlestein amendments).

## Non-goals (explicit)

- Does NOT change the promotion predicate (`recall_count ≥ 3 AND avg_relevance ≥ 0.45 AND last_recalled within 14d` stays intact).
- Does NOT claim to fix `recall_count` burst inflation (prior-art §1, tracked as separate session-dedup BL).
- Does NOT introduce a `was_longterm` state (demotion clears `is_longterm` and that's it; richer state would need a migration).
- Does NOT mutate stored `avg_relevance` — decay is a read-time derivation in both compute sites.
- Does NOT use SQL `pow()` — all decay math in Rust (portability + testability).

## Steps

### Step 1: Extract pure decay primitive in `src/core/decay.rs` + shared timestamp parser (AC1, AC4) — ✅ commit `7245994`

- [x] Create `src/core/decay.rs` with:
  - `pub const HALF_LIFE_DAYS: f64 = 60.0;`
  - `pub const DEMOTION_FLOOR: f64 = 0.20;`
  - `pub fn decay_factor(days: f64) -> f64 { (2.0_f64).powf(-days / HALF_LIFE_DAYS) }` (returns 1.0 when `days ≤ 0`; clamps to 0.0 when `days` is non-finite)
  - `pub fn effective_relevance(avg_relevance: f64, last_recalled: DateTime<Utc>, now: DateTime<Utc>) -> f64` — returns `avg_relevance × decay_factor(elapsed_days)`
  - `pub fn should_demote(effective: f64) -> bool` — returns `effective < DEMOTION_FLOOR`
- [x] Add shared timestamp helper `impl MemoryEntry { pub fn last_recalled_as_datetime(&self) -> Option<DateTime<Utc>> { ... } }` in `src/core/db.rs` — parses the stored RFC3339 string, returns `None` when missing or malformed. This is **created in Step 1** because both Step 2 (demotion pass) and Step 3 (search re-rank) depend on it; placing it here prevents duplicate parse logic (architect review must-fix).
- [x] Register `decay` module in `src/core/mod.rs`.
- [x] Table-driven unit tests in `decay.rs` covering `decay_factor` at:
  - `d=0 → factor=1.0` (today, no decay)
  - `d=1 → factor≈0.9885`
  - `d=15 → factor≈0.8409` (15-day-old recall, current corpus max age)
  - `d=30 → factor≈0.7071` (one-month checkpoint — exactly `2^(-0.5)`)
  - `d=44 → factor≈0.6027` (codex R1 floor=0.30 trigger point — sanity anchor)
  - `d=60 → factor=0.5` exactly (half-life definition — regression catches `exp(-d/H)` bug if reintroduced; assert with strict equality, no epsilon)
  - `d=75 → factor≈0.4204` (converged floor=0.20 trigger at `avg=0.487`)
  - `d=120 → factor=0.25` exactly (two half-lives — strict equality)
  - `d=137 → factor≈0.2054` (architect R2 floor=0.10 trigger point — sanity anchor)
  - `d=600 → factor≈9.54e-4` (10 half-lives, deep tail)
  - `d=6000 → factor≈8.64e-31` (extreme — still a normal double, no underflow)
  - `d=-5 → factor=1.0` (clamp: future `last_recalled` cannot amplify)
  - `d=f64::NAN → factor=0.0` (non-finite clamp — prevents poisoning the Dreaming pass)
- [x] `effective_relevance` boundary tests: `(avg=0.487, d=75) → effective≈0.205` (at floor, not demoted); `(avg=0.487, d=77) → effective≈0.199` (strictly below floor, demoted); `(avg=0.487, d=76) → should_demote == false` (off-by-one guard). Epsilon 0.001 for near-floor cases.
- [x] `last_recalled_as_datetime` unit tests: valid RFC3339 → `Some(DateTime)`; `None` → `None`; malformed string (e.g., `"not-a-date"`) → `None` (graceful, not panic). Required per codex blocker on AC1.

Expected files: `src/core/decay.rs` (new), `src/core/mod.rs`, `src/core/db.rs`

### Step 2: Demotion path in `run_dreaming_with_config` + 4 new counters (AC2, AC4) — ✅ commit `fcb4f26`

- [x] Change signature: `pub fn run_dreaming_with_config(&self, project_id: Option<&str>, config: &DreamingConfig, now: Option<DateTime<Utc>>, write_demotions: bool) -> anyhow::Result<DreamingResult>`. `None` → `Utc::now()`. The `write_demotions: bool` parameter **unifies live and dry-run paths into one function** (architect review consider: eliminates two-path code-drift risk of the original sibling `dry_run_decay`); Step 4 simply calls with `write_demotions = false`.
- [x] Update the 3 affected callers:
  - `run_dreaming` wrapper (`src/core/dreaming.rs:52`) — passes `(None, &DreamingConfig::default(), None, true)`.
  - `src/bin/cli.rs:215` (direct call) — passes `(None, &config, None, !dry_run_decay_flag)`.
  - `tests/e2e.rs:92` is **indirect** (calls the `run_dreaming` wrapper, not `run_dreaming_with_config`); the wrapper update covers it — no standalone edit needed. (Clarification per dependency-analyst + architect reviews.)
- [x] Extend `DreamingResult` with **5 new fields** (4 counters from Doodlestein strategic + 1 list per Doodlestein adversarial — `breached_ids` is required for Step 5's per-memory approval-gate listing; without it the shell script can't recover the breach detail from aggregate counts):
  - `demoted: usize` (always `0` when `write_demotions == false`)
  - `avg_effective_score_before: f64`
  - `avg_effective_score_after: f64`
  - `decay_floor_breaches: usize` (counts would-be demotions regardless of `write_demotions`)
  - `breached_ids: Vec<String>` (IDs of memories whose `effective < floor`; same in live and dry-run — populated before the conditional UPDATE)
- [x] After existing promotion pass, add a demotion pass that:
  1. Selects all rows where `is_longterm = 1 AND valid_until IS NULL AND last_recalled IS NOT NULL` (scoped to `project_id` if given).
  2. For each row: uses `entry.last_recalled_as_datetime()` (Step 1 helper), calls `decay::effective_relevance`, accumulates running sum for `avg_effective_score_before` (denominator = this selection's row count).
  3. Collects `breached_ids: Vec<String>` where `should_demote(effective) == true`. `decay_floor_breaches = breached_ids.len()`. Store on `DreamingResult.breached_ids` for CLI / approval-gate consumption.
  4. **If `write_demotions == true`**: `UPDATE memory_entries SET is_longterm = 0 WHERE id IN (<breached_ids>)` via chunked parameterized statement; `demoted = affected_rows`. Else: `demoted = 0` (dry-run).
  5. Re-computes mean effective relevance over the **post-state** `is_longterm = 1` set for `avg_effective_score_after` — in dry-run, this equals `before` (no writes happened) and the operator compares against the hypothetical post-state via `decay_floor_breaches`.
- [x] Explicit skip of `last_recalled IS NULL` rows (expect ~1 such row on current corpus). Log once at INFO: `"skipping decay for N long-term memories with NULL last_recalled"`.
- [x] Add code comment at the demotion site referencing the **intentional LONGTERM_BOOST cliff** — memories demoted here lose the 1.2× search boost next call; that is the mechanism, not a bug (cross-ref Step 3).
- [x] Unit tests (using injected `now` + `write_demotions=true`): one memory above floor stays `is_longterm=1`; one memory below floor demotes; NULL-recall memory is untouched; `before/after` averages computed correctly.
- [x] Unit test (`write_demotions=false`): same fixture as above; assert `demoted == 0` but `decay_floor_breaches == 1`; assert DB state unchanged post-call.
- [x] Integration test with `Utc::now()` default (backwards compat via `run_dreaming` wrapper).

Expected files: `src/core/dreaming.rs`, `tests/e2e.rs`, `src/bin/cli.rs`

### Step 3: Search post-fetch decay re-rank with same-age-clock invariant (AC3) — ✅ commit `12d741a`

- [x] In `search.rs`, replace the `normalized × LONGTERM_BOOST` branch at lines 142–146 with:
  ```rust
  let now = chrono::Utc::now();  // search is per-call, no clock injection needed
  let decay = entry
      .last_recalled_as_datetime()  // shared helper defined in Step 1
      .map(|last| decay::decay_factor((now - last).num_seconds() as f64 / 86400.0))
      .unwrap_or(1.0);  // never-recalled → no decay penalty (symmetric with Step 2 skip)
  let boosted = if entry.is_longterm {
      (normalized * LONGTERM_BOOST * decay).min(1.0)
  } else {
      normalized * decay
  };
  ```
- [x] The shared `last_recalled_as_datetime()` helper + `decay::decay_factor` are both already in place from Step 1; this step only wires them into the search path. No parse logic is duplicated (architect must-fix resolved).
- [x] Add code comment at the boost site `search.rs:142` referencing the **LONGTERM_BOOST cliff**: when Dreaming demotes a memory, the next search drops that memory from `effective × 1.2 × decay` to `effective × decay` — one-time discontinuity, intentional.
- [x] `record_recall` still uses `normalized` (pre-boost, pre-decay) — unchanged, prevents circular amplification.
- [x] **Same-age-clock invariant test** (codex AC2/AC3 blocker): one fixture with `last_recalled = T`; drive it through both `run_dreaming_with_config` (returns `avg_effective_score_before`) AND a `hybrid_search` call at the same synthetic `T+30 days` wall clock; assert both compute the same decay factor (±1e-9). Proves the two call sites do not drift.
- [x] Unit test: two memories, identical `avg_relevance`, different `last_recalled` → decayed one ranks strictly lower, with the ratio of returned scores matching `decay_factor(d1) / decay_factor(d2)` (±1e-6).

Expected files: `src/core/search.rs` (db.rs helper was added in Step 1)

### Step 4: `mengdie dream --decay-dry-run` CLI flag + structured output + ops doc (AC5, AC6) — ✅ commit `a6a3b02`

- [x] Add `--decay-dry-run` bool flag to the `mengdie dream` subcommand in `src/bin/cli.rs`. **Rename note**: `--decay-dry-run` (subject-first) chosen over `--dry-run-decay` per gemini review — matches the existing `--synthesize --dry-run` subject-mode pattern (the `--dry-run` on synthesis pairs with the `--synthesize` subject; decay's own dry-run binds the subject into the flag name). When `--decay-dry-run` is set the CLI calls `run_dreaming_with_config(None, &config, None, /*write_demotions=*/ false)`. Must NOT combine with `--synthesize --dry-run`: error out if both `--decay-dry-run` and `--dry-run` are passed (they target different passes with different semantics).
- [x] Live-run output format (always, whether dry-run or not — plan-level guard from Doodlestein adversarial):
  ```
  Dreaming pass: P promoted, D demoted (B floor breaches, avg effective 0.XXX → 0.YYY)
  ```
  where `P=promoted`, `D=demoted`, `B=decay_floor_breaches`, and the two decimal values are `avg_effective_score_before` / `avg_effective_score_after`. Dry-run replaces `D demoted` with `D would-demote (DRY RUN)`.
- [x] **Per-line structured JSON appended to stderr** for machine consumption (codex AC5 blocker): after the human-readable line above, emit one `{"event":"dreaming_pass","promoted":P,"demoted":D,"decay_floor_breaches":B,"avg_effective_before":X,"avg_effective_after":Y,"dry_run":bool,"breaches":["BL-...","BL-..."]}` JSON object at INFO level via `tracing`. The `breaches` array carries the memory IDs whose effective relevance fell below floor (from `DreamingResult.breached_ids`, Step 2) — required for the per-memory approval gate in Step 5 (Doodlestein adversarial blocker). Downstream tooling parses the structured line; the human line is free to evolve. The regex in AC5 targets the human line with loose matching (decimal places, spacing).
- [x] **AE parser audit** (gemini blocker): before merge, `rg "Dreaming complete" ../agentic-engineering/` — confirmed empty at plan-draft time (2026-04-20). Document the audit result in the Step 4 PR description. If any future AE parser materializes against the old format, the structured-JSON line above is the migration path.
- [x] Add operator procedure doc `docs/operations/dreaming-decay.md` (new file) with:
  - **Required first-run procedure**: `mengdie dream --decay-dry-run` MUST be run and inspected before the first live `mengdie dream` post-ship. Inspect `decay_floor_breaches` and the before/after means; if >10% of long-term corpus would demote, halt and re-calibrate floor.
  - **Metric interpretation guide** (gemini consider): `decay_floor_breaches` counts memories where `effective < 0.20`. In a live pass, `demoted == decay_floor_breaches` — they are redundant. In dry-run, `demoted = 0` but `decay_floor_breaches` remains; that is the quantity the operator reviews. `avg_effective_score_before` is the mean across all `is_longterm = 1 AND last_recalled IS NOT NULL` rows; `avg_effective_score_after` is the mean across the survivors post-UPDATE (live) or equals `_before` (dry-run).
  - **Data freshness / "corpus age" definition** (gemini consider): "corpus age > 90 days" means the **longest `last_recalled` gap** in the `is_longterm = 1` set, not ingest age. If `max(now - last_recalled)` across long-term memories exceeds 90 days, revisit floor calibration — the corpus is exercising the decay tail for the first time and may warrant a lower floor.
  - **Revisit triggers** (copied from conclusion): `avg_effective_relevance < 0.25` on a Dreaming pass OR `max(now - last_recalled)` > 90 days OR `avg_relevance` IQR > 0.05.
  - **LONGTERM_BOOST cliff explanation**: when Dreaming demotes, the 1.2× search boost disappears for that memory on the next query. Intentional — demotion is the mechanism, not a regression.
- [x] CLI help text reflects new flag + compatibility note with `--synthesize --dry-run`.

Expected files: `src/bin/cli.rs`, `docs/operations/dreaming-decay.md` (new)

### Step 5: Production-corpus smoke + regression matrix + CHANGELOG.md bootstrap (AC6, AC7) — ✅ commit `522db8a`

- [x] `tests/e2e.rs`: add `test_decay_smoke_on_seeded_corpus` — ingest 41 long-term memories with varied `last_recalled` ages (0, 15, 44, 75, 77, 137 days), run one Dreaming pass with injected `now`, assert:
  - Memory at `d=0` stays `is_longterm=1`
  - Memory at `d=15` stays `is_longterm=1` (`effective ≈ 0.397 > 0.20`)
  - Memory at `d=75` at the strict boundary: `effective ≈ 0.205 > 0.20` → stays promoted (off-by-one guard from Step 1 ACs)
  - Memory at `d=77` demotes (`effective ≈ 0.199 < 0.20`)
  - Memory at `d=137` demotes (deep below floor)
  - `DreamingResult.demoted == 2`, `decay_floor_breaches == 2`, `avg_effective_score_before > avg_effective_score_after` (write mode); in a second run with `write_demotions=false` assert `demoted == 0` + `decay_floor_breaches == 2` + DB unchanged.
- [x] Production corpus smoke via new `scripts/verify-decay.sh` — thin wrapper running `mengdie dream --decay-dry-run` against `~/.mengdie/db.sqlite`. **Pass criterion** (revised per codex AC7 blocker): **log and document every would-demote memory**, not a hard "zero demotions" gate. The operator inspects the structured JSON line's `decay_floor_breaches` value; if >0, the script prints the breached-memory IDs and requires explicit `--i-reviewed-each` confirmation flag before proceeding to live `mengdie dream`. This ties correctness to an approval gate, not corpus cleanliness. At ship date, expected value is 0 (matches archaeologist V3); the approval mechanism is there for future corpus drift.
- [x] `scripts/verify-decay.sh` emits a final report: `decay_floor_breaches=N; dreaming_result_avg_effective_before=X; avg_effective_after=Y` — captured in `docs/operations/dreaming-decay.md`'s `## Baseline` section after the first run.
- [x] **Create `CHANGELOG.md`** at repo root (does not exist yet, confirmed at plan-draft time; gemini consider integrated into Step 5, not spun off to a separate BL). Format: Keep-a-Changelog style with an `## Unreleased` section. First entry is BL-008:
  ```markdown
  # Changelog

  All notable changes to Mengdie are documented here.
  Format: Keep a Changelog; this project follows semantic versioning.

  ## Unreleased

  ### Added
  - Exponential decay for Dreaming (BL-008): `effective = avg_relevance × 2^(-d/60)`;
    demotes `is_longterm = 1` memories whose effective relevance < 0.20. Adds
    `--decay-dry-run` CLI flag, 4 new `DreamingResult` counters, and structured-JSON
    output for machine consumption. Operator procedure:
    `docs/operations/dreaming-decay.md`. Source: plan 013 / discussion 019.
  ```
- [x] Update `docs/backlog/005-phase2-roadmap.md` to mark BL-008 complete; link to this plan.

Expected files: `tests/e2e.rs`, `scripts/verify-decay.sh` (new), `CHANGELOG.md` (new), `docs/backlog/005-phase2-roadmap.md`

## Acceptance Criteria

### AC1: Decay primitive correctness — table-driven
`cargo test decay::tests` passes with:
- 12 `decay_factor` cases (d ∈ {0, 1, 15, 30, 44, 60, 75, 120, 137, 600, 6000, -5}) + 1 non-finite case (`d = f64::NAN → factor = 0.0`).
- `d = 60` and `d = 120` use **strict equality** (catches reintroduction of `exp(-d/H)` instead of `2^(-d/H)`).
- 3 `effective_relevance` boundary cases: `(avg=0.487, d=75) → effective≈0.205` (at floor, no demote); `(avg=0.487, d=76) → should_demote == false`; `(avg=0.487, d=77) → effective≈0.199, should_demote == true`.
- 3 `last_recalled_as_datetime` cases: valid RFC3339 → `Some(DateTime)`; `None` column → `None`; malformed string → `None` (no panic).
Epsilon 0.001 for near-floor comparisons; 1e-6 for deep-tail; strict equality for half-life checkpoints.

### AC2: Demotion semantics — integration under injected clock
Given a DB with a seeded long-term memory whose `last_recalled` is 77 days before injected `now`, `avg_relevance=0.487`:
- `run_dreaming_with_config(None, &config, Some(now))` returns a `DreamingResult` with `demoted == 1`, `decay_floor_breaches == 1`, `avg_effective_score_after < avg_effective_score_before`.
- The memory's `is_longterm` flag is now `0`.
- A second identical memory with `last_recalled IS NULL` is unchanged (no decay, no demotion).

### AC3: Search ranking after decay — ordered demonstration
Insert two memories with `avg_relevance = 0.50` and `avg_relevance = 0.50`, `is_longterm = true`, but `last_recalled` = (today, 60 days ago). Search a query matching both. Assert: the fresher memory's `SearchResult.score` is strictly greater than the 60-day-old memory's score (by factor ≈ 2). Both memories have the SAME `avg_relevance`; the ranking divergence proves decay is active on the ranking path.

### AC4: Observability counters — field contract (live AND dry-run)
**Live pass** (`write_demotions = true`), after `run_dreaming_with_config` on a seed corpus of 10 long-term memories (5 within floor, 5 below):
- `demoted == 5`
- `decay_floor_breaches == 5` (equals `demoted` in non-dry-run)
- `breached_ids.len() == 5` and each ID matches the below-floor fixtures.
- `0.0 ≤ avg_effective_score_before ≤ 1.0`
- `avg_effective_score_after` is computed across the SURVIVING 5 long-term memories (not all 10) — documented contract.

**Dry-run pass** (`write_demotions = false`), same fixture:
- `demoted == 0` (no writes)
- `decay_floor_breaches == 5` (breach count unchanged by write flag)
- `breached_ids.len() == 5` (list populated identically in dry-run; required for Step 5 approval gate).
- `avg_effective_score_before == avg_effective_score_after` **exactly** (no demotions happened, so the denominator set is identical). This closes a silent-zero-return hole that would otherwise let an implementer return `0.0` for `_after` in dry-run without failing AC4 (Doodlestein strategic finding).
- DB state identical to pre-call (verified by reading all 10 `is_longterm` flags post-call).

### AC5: CLI output — `demoted` visible, dry-run distinguished, structured line for machines

> **Post-ship correction (2026-04-23, plan 016)**: the human-line regex
> below tolerates `(?:→|->)` — both Unicode arrow and ASCII fallback —
> but the actual emitter at `src/bin/cli.rs::format_dreaming_line` emits
> only `→` (Unicode), and the AC5 regex tests at `src/bin/cli.rs:719-736`
> assert only the Unicode form. The **rejected alternative** was **dual
> emission** (emitting both `→` and `->` side-by-side, or a switchable
> fallback), which would have required format-string + regex-test changes.
> Unicode-only is an **accepted-risk decision**, not a validated-robustness
> one — `→` (U+2192, three UTF-8 bytes) can be dropped by `LC_ALL=C`
> shells, certain `awk`/`sed` locales, or ASCII-normalizing log pipelines.
> No such incident has been observed, but the risk is not refuted by
> "no operator-reported issues" (n=1 operator). Reversal path if the
> pipe-eating scenario materializes: update `format_dreaming_line` to
> emit both arrows, update the AC5 regex tests to `(?:→|->)`, remove
> this correction note. Scoped ~10-line diff. Until then, **the test at
> `src/bin/cli.rs:723` is the source of truth**; this note closes the
> documentation loop on the AC5 vs. code mismatch. See
> `docs/plans/016-decay-ops-doc-polish.md` "Decision on action 2,
> accepted risk" and `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md`.

**Human-readable line** (loose regex — tolerates whitespace + decimal-place variation so safe operator-output improvements don't break the AC, per codex blocker):
- Live: matches `Dreaming pass:\s+\d+\s+promoted,\s+\d+\s+demoted\s+\(\d+\s+floor breaches,\s+avg effective\s+0?\.\d+\s*(?:→|->)\s+0?\.\d+\)`.
- Dry-run: same shape but `\d+\s+would-demote\s+\(DRY RUN\)` replaces `\d+ demoted`.

**Structured-JSON line** (per-pass, stderr INFO, parsed by any future machine consumer):
- Must contain `"event":"dreaming_pass"` plus all six numeric/bool keys (`promoted`, `demoted`, `decay_floor_breaches`, `avg_effective_before`, `avg_effective_after`, `dry_run`) AND the `breaches: [string, ...]` array.
- Test parses the line with `serde_json`, asserts types and that `breaches.len() == decay_floor_breaches`.

**Flag-conflict guard**: `mengdie dream --decay-dry-run --synthesize --dry-run` exits non-zero with a message mentioning both flags and pointing operators at the ops doc.

### AC6: Operator procedure — docs present
`docs/operations/dreaming-decay.md` exists, contains (a) the mandatory pre-ship `--dry-run-decay` procedure, (b) the three revisit triggers, (c) the LONGTERM_BOOST cliff explanation. `CHANGELOG.md` has a BL-008 entry that links to this doc.

### AC7: Production smoke — approval-gate on would-demote, not hard zero
Running `scripts/verify-decay.sh` (which calls `mengdie dream --decay-dry-run` against `~/.mengdie/db.sqlite`) must:
- Print the per-memory breach list (one line each: `BL-X title — last_recalled_age=Nd effective=0.XX`).
- Exit non-zero unless the operator re-invokes with `--i-reviewed-each` after reviewing the list.
- At ship date, expected breach count is 0 (archaeologist V3 simulation min-effective = 0.397 > 0.20). The approval gate is the correctness signal, not the zero value itself — if a future run produces `decay_floor_breaches > 0`, the ship gate is "operator reviewed the list", not "list was empty." (Reframed per codex AC7 blocker.)

Script writes a Baseline snapshot entry to `docs/operations/dreaming-decay.md` recording `breach_count`, `avg_effective_before`, `avg_effective_after`, and date, for future drift comparison.

## Rollout & verification

1. **Merge order**: 1 → (2 and 3 parallel-safe) → 4 → 5. Steps 2 and 3 both depend on Step 1 (shared `decay_factor` + `last_recalled_as_datetime`); after Step 1 merges they can land in either order without conflict. Step 4 depends on Step 2's new `write_demotions` parameter. Step 5 requires Steps 2 + 3 + 4 merged. (Dependency graph validated by dependency-analyst + architect reviews.)
2. **Pre-merge audit**: `rg "Dreaming complete" ../agentic-engineering/` — at plan-draft time this returns empty (no AE parser depends on the old CLI output string). Re-run the audit right before Step 4 merges and document in the PR description.
3. **Post-merge procedure** on `~/.mengdie/db.sqlite`:
   - Run `scripts/verify-decay.sh` — reviews the breach list and requires `--i-reviewed-each` to pass.
   - Only then run `mengdie dream` (live). Confirm the human line's `demoted` count matches the dry-run's `would-demote` count.
   - Parse the structured-JSON stderr line with `jq` to confirm all 6 fields are present.
4. Log the before/after means and breach list in `docs/operations/dreaming-decay.md`'s `## Baseline` section.

## Scope budget (architect review)

Estimated LOC:
- Impl: ~100 (decay.rs ~40, dreaming.rs ~40, search.rs ~10, cli.rs ~10)
- Tests: ~160 (decay unit ~60, dreaming unit ~40, search unit ~20, e2e smoke ~40)
- Docs: ~80 (ops doc + CHANGELOG bootstrap)

Total ~340 LOC. The conclusion's ~50–100 LOC estimate covered impl only; testing and docs were not scoped there. This plan's ACs explicitly require the test + doc surface, so the overshoot is intentional, not scope creep.

## Revisit triggers (copied from conclusion)

Re-open this design if any of:
- `avg_effective_relevance` across the corpus drops below 0.25 on a Dreaming pass
- Corpus age exceeds 90 days
- `avg_relevance` IQR widens past 0.05 (distribution regime change)

First reversal likely shape: floor lowered to 0.15 OR corpus-relative rule (`mean − 1.5×IQR`). Both are single-constant edits + regression-table refresh.

### Plan-level revisit trigger (Doodlestein regret finding)

**`--i-reviewed-each` approval gate**: the `scripts/verify-decay.sh` manual-approval flag assumes a human operator. When `com.mengdie.dream.plist` (already in `resources/`) is wired to launchd for automated nightly Dreaming — planned Phase 2.2 per `docs/backlog/005-phase2-roadmap.md` BL-010 daemon — the approval gate must evolve. Revisit trigger: **when daemonization of Dreaming is scheduled**, drop `--i-reviewed-each` and replace with a threshold: if `decay_floor_breaches > max(10, 10% of long-term count)`, emit a WARNING-level structured-JSON event `decay_spike` and proceed. The durable signal (`decay_floor_breaches` in `DreamingResult` + structured JSON) already exists for this purpose. Confidence: high — daemon is a committed Phase 2.x step, not hypothetical. Reversal is ~20 LOC (remove the interactive gate, add the threshold compare).

## Prior art (from Mengdie)

See `docs/discussions/019-power-law-decay/prior-art.md` + the 3 decisional memories ingested 2026-04-20:
- `c6661d97` — BL-008 formula + floor 0.20
- `46f5880d` — BL-008 compute location + asymmetric demotion
- `5b3cf845` — BL-008 observability primitives

The corrected `is_longterm IS read by search` memory (`e2b0bb63`) supersedes the stale `2c3122ff` that was invalidated during the discussion.
