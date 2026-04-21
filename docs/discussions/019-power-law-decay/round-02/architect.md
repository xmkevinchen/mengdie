---
agent: architect
round: 2
date: 2026-04-20
---

# Round 2 — Architect

## Findings (with file:line evidence)

### Peer claims verified this round

- **archaeologist.md:52–73**: Confirms `is_longterm` IS read by `search.rs:142` with
  `LONGTERM_BOOST = 1.2`. My Round 1 Topic 2 finding was based on this same code observation
  and is consistent. My Round 1 statement "demotion would reduce a memory's search score
  from 1.2× to 1.0×" (`architect.md:Topic 2, search.rs:142 paragraph`) is confirmed by
  both archaeologist and challenger.

- **archaeologist.md:137–176**: Empirical distribution: 86% of recalled memories have
  `avg_relevance` in 0.46–0.50, min 0.462, max 0.746, mean 0.487. Corpus has 323 live
  memories, 41 long-term (12.7%), 110 with `last_recalled` set, and all recalled memories
  were recalled within 15 days (corpus is 15 days old at most). This is load-bearing for
  the floor decision.

- **codex-proxy.md:64–65**: `exp(-d/half_life)` is NOT equivalent to a half-life of
  `half_life` days — it makes the half-life `half_life × ln(2) ≈ 0.693 × half_life`.
  My Round 1 formula was spelled `exp(-days_since_last_recalled / half_life)` without
  `ln(2)`. This is a correctness error in my notation. I accept the correction.

- **codex-proxy.md:266–273**: With `H=60` and `avg=0.5`, crossing `0.30` happens at
  `d > 60 × log2(1/0.6) ≈ 44.2 days`. With `H=60` and `avg=0.487` (actual mean),
  crossing `0.10` requires `exp(-ln2 × d / 60) < 0.10/0.487 ≈ 0.205`, solving:
  `d > 60 × log2(1/0.205) ≈ 60 × 2.28 ≈ 137 days`. My Round 1 calculation of 93 days
  used `exp(-d/H)` (without `ln2`) which gives `exp(-93/60) ≈ 0.21 × 0.48 ≈ 0.10`.
  Under the correct `2^(-d/H)` form, the 0.10 floor triggers at ~137 days, not 93.

- **gemini-proxy.md:22–32**: `Option<DateTime<Utc>>` signature with `unwrap_or_else(Utc::now)`
  in production. My Round 1 proposed a required `now` parameter; Gemini's `Option<>` form
  is strictly cleaner — callers not needing test control pass `None` without API churn.

- **challenger.md:185–190**: "Observe decay distribution before committing to thresholds"
  argument. Addressed in D2 below.

---

## Agreements

- **Formula form**: Accept `2^(-d/H)` (equivalently `exp(-ln(2) × d / H)`). Codex's
  correctness nit is valid. My Round 1 `exp(-d/H)` spelling was subtly wrong.
- **Clock injection**: Gemini's `Option<DateTime<Utc>>` is better than my required parameter.
  Production passes `None`; tests pass `Some(frozen_dt)`. Agreed.
- **Age input = `last_recalled`**: Confirmed. Archaeologist's data shows `last_recalled` is
  reliable for recalled memories; my Round 1 fallback to `created_at` for NULL-`last_recalled`
  memories remains as the only open question (addressed below).

---

## Disagreements

### Remaining: D1 floor value (addressed below), D2 ship scope (addressed below)

---

## Responses to the 5 directed questions

### Q1: D5 rename — accept "power-law" → "exponential" / "time-weighted" decay?

**Yes.** One sentence: The formula is exponential decay, not power-law; rename to
"exponential decay" (or "time-weighted decay" if the author prefers generality) everywhere
the discussion ID, topics, and BL-008 title appear.

---

### Q2: D1 floor value — defend or revise 0.10

**Revised position: 0.20, with the following decision rule.**

My Round 1 floor of 0.10 was based on the `exp(-d/H)` form (without `ln2`), giving a
93-day trigger. Under the correct `2^(-d/H)` form with the actual distribution mean of
0.487, `floor=0.10` triggers at `d > 137 days`. With the corpus being 15 days old
(`archaeologist.md:158–160`), no memory will cross `floor=0.10` for another 122+ days
after BL-008 ships. That is too conservative — the feature ships and produces no
observable demotion event within any reasonable horizon.

The correct calibration question is: **what floor produces first demotion at a horizon
that is observably meaningful without being aggressive?**

With `H=60` and `avg_relevance` mean = 0.487:
- `floor=0.30`: demotes at `0.487 × 2^(-d/60) < 0.30` → `d > 44 days` (Codex computation,
  `codex-proxy.md:266–273`)
- `floor=0.20`: demotes at `0.487 × 2^(-d/60) < 0.20` → `d > 60 × log2(0.487/0.20) ≈
  60 × 1.28 ≈ 77 days`
- `floor=0.10`: demotes at `d > 137 days` (computed above)

The Challenger's distribution-percentile argument (`challenger.md:171–178`) is sound but
adds complexity: a percentile-derived floor changes every time the corpus grows, requiring
re-derivation at tuning time. The simpler decision rule is a fixed value calibrated to
the known distribution.

**Decision rule**: fix floor at a value where `floor / mean(avg_relevance) ≈ 0.40`. With
mean ≈ 0.487, that gives `floor ≈ 0.20`. This means a memory decays to 40% of its average
relevance before demotion — roughly 77 days without recall under `H=60`. This is:
- Conservative enough to survive the 15-day-old corpus without premature mass demotions.
- Aggressive enough to produce observable first-demotions within a quarter of shipping.
- Expressed as a fixed constant, not a percentile, so it's auditable in code.

**I revise from 0.10 to 0.20.** Codex's 0.30 (44-day trigger) is plausible but feels
slightly aggressive for a first-ship on a corpus where every memory is under 15 days old.
0.20 (77-day trigger) gives more runway to observe the mechanism before real demotions arrive.

---

### Q3: D1 formula form — `2^(-d/H)` or `exp(-ln(2)·d/H)`?

**Use `2^(-d/H)` in implementation, expressed as `(2.0_f64).powf(-days / H)` in Rust.**

Codex is correct that my Round 1 `exp(-d/half_life)` spelling was misleading. Both forms
are numerically identical but differ in readability. The `2^(-d/H)` form makes the
half-life semantics visually apparent — a reader who knows nothing else can see that at
`d=H`, the factor becomes `2^(-1) = 0.5`, exactly half. The `exp(-ln2·d/H)` form requires
the reader to mentally evaluate `ln2 ≈ 0.693` to check that invariant.

In Rust, `(2.0_f64).powf(-days / half_life_days)` is clear, cheap (one FP operation), and
correct. No `use std::f64::consts::LN_2` needed, no opportunity to drop the `ln2` factor.

---

### Q4: D2 ship scope — decay-only first or decay + demotion?

**Hold: ship both decay and demotion in BL-008.**

Challenger's argument (`challenger.md:185–190`): "ship decay-only, observe the distribution,
add demotion in follow-up BL." The observation point this creates is: the Dreaming pass runs
and `DreamingResult` reports decay statistics, but `is_longterm` is never cleared.

The structural problem with decay-only first: **decay without demotion has zero behavioral
effect on either search or the Dreaming promotion gate.** The only use of `is_longterm = 1`
in the runtime is the 1.2× search boost (`search.rs:142`). If decay does not clear
`is_longterm` and does not affect search ranking (since my Round 1 Topic 2 proposed adding
decay as a search re-rank multiplier SEPARATELY from `is_longterm`), then the "decay-only"
ship delivers no observable behavior change for users — only counter numbers in `DreamingResult`.

That is observing a shadow, not the mechanism. The Challenger's "observe before deciding"
argument is sound in principle but the observation target (DreamingResult counters showing
what WOULD have been demoted) is achievable with a `--dry-run-decay` flag, not a separate
partial ship.

Given that demotion is ~10 LOC in the Dreaming pass (`UPDATE memory_entries SET is_longterm = 0
WHERE is_longterm = 1 AND effective < floor`), and given that with `floor=0.20` and `H=60`
the first demotion will not occur for ~77 days on the current corpus — demotion can ship
now with low risk of premature mass demotion. The `--dry-run-decay` flag (D3, see below)
provides the safety valve.

**Condition**: This position depends on D3 (dry-run flag) being in scope. If dry-run is
rejected as scope creep, I would reconsider — because without a pre-mutation validation
path, the Challenger's "observe first" is the safer choice.

---

### Q5: D4 counters — reconcile 2 vs 3 fields

**`avg_effective_relevance` (mine) = `avg_effective_score_before` (Gemini). Accept Gemini's
name as clearer. `decay_floor_breaches` is distinct from `demoted` in dry-run scope — adopt
all 3 Gemini fields.**

- `avg_effective_relevance` (my name) and `avg_effective_score_before` (Gemini `gemini-proxy.md:47`)
  are the same computation: mean effective relevance across all `is_longterm = 1` memories
  computed during the demotion scan pass. Gemini's name `avg_effective_score_before` is more
  precise — "before" clarifies it is measured before the demotion write, not after.
  **Accept `avg_effective_score_before`.**

- `decay_floor_breaches` (`gemini-proxy.md:53`) is distinct from `demoted` specifically in
  dry-run scope: in a dry-run pass, no `is_longterm` writes happen, so `demoted = 0` always.
  But `decay_floor_breaches` counts the memories THAT WOULD HAVE been demoted (effective <
  floor, regardless of whether the write fires). This is the key signal for pre-mutation
  validation (`gemini-proxy.md:143–160`). The two fields collapse to the same value in a
  normal (non-dry-run) pass, but the dry-run use case makes `decay_floor_breaches` load-bearing.
  **Adopt all 3 fields**: `demoted: usize`, `avg_effective_score_before: f64`,
  `decay_floor_breaches: usize`.

---

## Updated Positions Summary

| Topic | Round 1 | Round 2 (updated) |
|---|---|---|
| D5 naming | N/A | Accept rename to "exponential decay" |
| D1 floor | 0.10 (93-day trigger, wrong form) | **0.20** (77-day trigger, correct `2^(-d/H)` form) |
| D1 formula form | `exp(-d/H)` (incorrect) | **`2^(-d/H)` = `(2.0_f64).powf(-d/H)` in Rust** |
| D2 ship scope | Both decay + demotion | Hold: both — conditional on D3 dry-run being in scope |
| D4 counters | 2 fields: `demoted`, `avg_effective_relevance` | **3 fields**: `demoted`, `avg_effective_score_before`, `decay_floor_breaches` |
| D3 dry-run | Scope creep, reject | **Reverse: accept as in-scope** — see below |

**D3 reversal**: I said `--dry-run-decay` was scope creep in Round 1. Gemini's
`gemini-proxy.md:143–160` makes a concrete case: `--dry-run-decay` is the ONLY way to
validate population-level mass-demotion risk before the first real pass on the production
corpus. Given that my revised floor (0.20, 77-day trigger) means the first real demotions
arrive ~77 days post-ship — the dry-run flag is the operator's only tool to pre-validate
the mechanism in the interim. The implementation is ~20 LOC (iterate `is_longterm=1`
memories, compute effective, count floor breaches, print report, write nothing). This is
within the ~50-100 LOC BL-008 scope budget. I reverse my Round 1 rejection.

---

## Open Questions

1. **NULL `last_recalled` for 213/323 memories (`archaeologist.md:197–200`)**: 66% of live
   memories have never been recalled and have `last_recalled IS NULL`. For these, should
   decay be driven by `created_at` (ages out never-recalled memories), or should never-recalled
   memories be excluded from decay entirely (only recalled memories can be demoted)? The
   current promotion guard `AND last_recalled IS NOT NULL` (`dreaming.rs:88`) implies
   never-recalled memories are excluded from promotion — by symmetry, excluding them from
   demotion is consistent. But if the goal is to demote stale LONG-TERM memories, and
   a memory was somehow promoted without a `last_recalled` timestamp (which the promotion
   predicate currently prevents), this is a non-issue. Recommend: decay formula only applies
   to `is_longterm = 1` memories, which by construction all have `last_recalled IS NOT NULL`
   (the promotion predicate requires it). No fallback to `created_at` needed.

2. **`run_dreaming` wrapper signature**: The public `run_dreaming` wrapper currently calls
   `run_dreaming_with_config` with defaults (`dreaming.rs:51–53`). With Gemini's
   `Option<DateTime<Utc>>` parameter, `run_dreaming` passes `None`. But `cli.rs` also calls
   `run_dreaming_with_config` directly (or through `run_dreaming`). Need to confirm all
   callers in Round 3 or implementation plan.

3. **D3 scope budget**: `--dry-run-decay` adds a CLI flag, a loop over long-term memories,
   and print logic. Estimate: ~25 LOC in `cli.rs` + ~15 LOC in `dreaming.rs` for the scan
   function. This is inside the 50-100 LOC budget but only if the decay formula function
   itself is extracted as a pure function (which it should be for testability anyway).
