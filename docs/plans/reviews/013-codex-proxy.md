---
reviewer: Codex (cross-family, medium reasoning)
date: 2026-04-20
status: concerns-requiring-fixes
angle: test-coverage + AC-correctness-matrix
---

# Review: Plan 013 — Exponential Decay for Dreaming

Cross-family review focusing on **test coverage sufficiency** and **AC matrix correctness**. Evaluation questions:
1. Are AC1 test cases sufficient? Edge cases?
2. Do AC2 and AC3 test the same code path, or are they distinct?
3. Is AC5 regex too brittle? Good contract?
4. Is AC7 "zero demotions" too strict for production smoke?

## Finding Summary

**Plan is not ship-ready without fixes to AC1, AC5, and AC7.** AC2 and AC3 are distinct and both necessary. See detailed findings below.

## Q1: AC1 Table-Driven Test Coverage

### Sufficiency: NOT SUFFICIENT

The 8 `d` values {0, 1, 15, 44, 60, 75, 137, -5} cover some basic cases but **miss boundary conditions** most likely to escape into production:

- No exact floor-boundary cases (`avg_relevance` chosen so `effective` lands just above/below 0.20)
- No parse-failure / non-finite cases (malformed RFC3339, `NULL last_recalled`)
- No very-large `d` causing underflow (e.g., `d=6000` should return near-zero without panic)
- No near-zero negative case (testing clamp behavior)
- No fractional `d` values (if implementation computes non-integer elapsed time before truncation)

### Does `d=60` catch the `exp(-d/H)` vs `2^(-d/H)` error?

**Yes, IF numeric assertion is precise.** At `d=60`:
- Correct: `2^(-60/60) = 0.5` exactly
- Wrong (natural-e form): ≈ 0.3679

The difference is large (36% error), so it will catch that substitution. **However**, `d=60` alone does NOT diagnose other formula mistakes:
- Using `2^(-d)` instead of `2^(-d/60)` (off by factor of 60)
- Integer division `d/60` (truncates)
- Wrong sign
- Unit mismatch (seconds vs minutes vs hours)
- Clamping negative `d` incorrectly

### Will NaN input or underflow be caught?

**No.** The integer-only table will not exercise:
- Timestamp parse failure (malformed RFC3339 string)
- `last_recalled = NULL` edge case
- Exponentiation underflow (very large `d` → 0.0) — needs explicit test to ensure no panic or silent truncation

### Recommended Additions

Expand AC1 with explicit test buckets:

1. **Identity + formula anchors:**
   - `d=0 → 1.0` (identity)
   - `d=30 → ≈0.7071` (quarter half-life, new)
   - `d=60 → 0.5` (half-life, existing)
   - `d=120 → 0.25` (double half-life, new)

2. **Floor-boundary cases:**
   - `(avg=0.20, d=0) → 0.20` (stays, on boundary)
   - `(avg=0.40, d=60) → 0.20` (exactly at floor, stays)
   - `(avg=0.4001, d=60) → 0.20005` (just above floor, stays)
   - `(avg=0.3999, d=60) → 0.1999` (just below floor, demotes)

3. **Underflow + extreme:**
   - `d=600` (10× half-life, should return ≈0.001 without panic)
   - `d=6000` (100× half-life, near-zero without overflow)

4. **Negative / clamp:**
   - `d=-1` (clamp to 1.0)
   - `d=-5` (existing)

5. **Parse failures (integration):**
   - Malformed RFC3339 string → error propagation
   - `last_recalled = NULL` → fallback to 1.0 (no decay)
   - Missing timestamp field → explicit handling

**Action:** Update AC1 to specify these buckets, not just 8 arbitrary integers.

---

## Q2: AC2 vs AC3 Overlap

### Are they testing the same code path?

**No, they are distinct and BOTH necessary.** They exercise different integration surfaces:

- **AC2 (Dreaming demotion):** validates pass ordering, clock injection, `is_longterm` mutation, before/after score averages
- **AC3 (Search re-rank):** validates decay applied to search ranking, interaction with `LONGTERM_BOOST`, query result ordering

The shared primitive (decay helper) is exercised at different call sites with different surrounding logic. **The real risk is not redundancy, but fixture geometry reuse.**

### Do they satisfy the "same-age-clock invariant"?

**Only if fixtures are intentionally non-isomorphic.** The plan states:
> "both sites drive decay off `entry.last_recalled`, satisfying the same-age-clock invariant"

This is correct **design intent**, but **test fixtures must validate it.** Current AC2/AC3 wording does not guarantee that:
- AC2 can pass with Dreaming using a stale clock while search uses a fresh one
- AC3 can pass with search ignoring decay entirely if the fixture reuses AC2's data

### Recommended Additions

1. **Explicit invariant test at helper level:**
   - Same `(avg_relevance, last_recalled)` → same decay multiplier in both contexts
   - Validates that both call sites parse RFC3339 identically

2. **Make AC2 and AC3 intentionally non-isomorphic:**
   - AC2: test a memory that crosses the floor AFTER promotion (ordering matters)
   - AC3: test two memories above the floor, where decay changes ranking without demotion
   - Use different ages and different `avg_relevance` shapes in the two tests

3. **Add ordering test in AC2:**
   - Prove that demotion happens AFTER promotion (if reversed, promotion might re-boost a demoted memory)

---

## Q3: AC5 Output Format Regex

### Is the regex a good contract?

**No. It is too brittle for external contracts and too weak for semantic validation.**

Current regex:
```
^Dreaming pass: \d+ promoted, \d+ demoted \(\d+ floor breaches, avg effective 0\.\d+ → 0\.\d+\)$
```

**Brittleness (will break on safe changes):**
- Wording changes ("demotions" vs "demoted", "average" vs "avg")
- Future fields (scan time, operator, corpus size)
- Precision changes (0.XXX vs 0.XXXXX)
- Spacing or punctuation tweaks

**Weakness (will NOT catch semantic errors):**
- Wrong counts reported but matching format
- Inverted before/after scores
- Missing demotion count

### Recommended Fix

**Split human output from machine assertions:**

1. **Looser human regex** (for operator readability):
   ```
   Dreaming pass: \d+ promoted, \d+ demoted.*floor breaches.*avg effective.*→
   ```
   Requires presence of fields but not exact punctuation.

2. **Structured output** (preferred):
   Add `--json` flag to emit:
   ```json
   {
     "promoted": 5,
     "demoted": 2,
     "floor_breaches": 2,
     "avg_effective_before": 0.487,
     "avg_effective_after": 0.523,
     "dry_run": false
   }
   ```
   Then assert exact fields in AC5.

3. **If regex-only, make it looser:**
   - Drop `0\.\d+` and use `[\d\.]+` (allows any precision)
   - Drop hardcoded punctuation around floor breaches
   - Anchor only on presence of numbers in right order

**Action:** Update AC5 to specify either structured output contract or looser regex + semantic field validation.

---

## Q4: AC7 Production Smoke "Zero Demotions"

### Is this too strict?

**Yes. It conflates code correctness with corpus cleanliness.**

The plan requires:
```
Running scripts/verify-decay.sh ... prints 0 would-demote (DRY RUN)
```

**Risks blocking a correct ship:**

1. **Real corpus drift:** If the production corpus contains legitimately stale memories (corpus age > 90 days, or changed use patterns), they SHOULD demote. Requiring zero demotions blocks a correct rollout.

2. **NULL last_recalled edge case:** The plan acknowledges ~1 such memory exists. If another edge case appears, zero-demotions AC will fail unnecessarily.

3. **Non-frozen corpus:** The corpus grows daily. The "archaeologist V3 simulation" used a snapshot; production corpus may have evolved since then.

4. **Arbitrary numeric guarantee:** `<= 1` is only marginally better because it still encodes an unstable assumption about live data.

### Recommended AC7 Rewrite

Replace "zero demotions" with **observational + approval contract:**

```
- scripts/verify-decay.sh executes successfully
- Reports exact counts: would-demote, floor-breaches
- Reports identifiers and reasons for every affected memory
- Human review of all nonzero counts against approved baseline before rollout
- For each nonzero demotion, document the reason in CHANGELOG or release notes
```

**Alternative (if numeric guardrail required):**
```
- No demotions beyond those pre-approved in docs/operations/decay-baseline.md
- Baseline snapshot captured at ship date
- Any new demotions must be reviewed and added to baseline
```

**Best form:**
> "All would-demote candidates must be reviewed and documented before rollout. Zero demotions is expected; if nonzero, pause and investigate corpus drift."

This is "document every demotion" rather than "zero demotions" — much stronger operationally.

**Action:** Rewrite AC7 to decouple code correctness (script runs, counts are accurate) from corpus state (no demotions required). Make nonzero counts a blocker on *review*, not on *execution*.

---

## Ship Blockers Summary

1. **AC1:** Missing floor-boundary, parse-failure, and underflow test cases
2. **AC5:** Brittle regex should be loosened or split into structured output
3. **AC7:** "Zero demotions" is not a code-correctness criterion; reframe as approval gate

**AC2 and AC3 are sound** — they are distinct and both necessary.

---

## Recommendation

**Do not ship without addressing AC1, AC5, AC7.** These are not nitpicks; they are gaps that can mask real bugs or false-fail production smoke on correct code.

Suggested path:
1. Expand AC1 test buckets (floors, underflow, parse failures)
2. Emit structured JSON output in addition to human text; assert on JSON in AC5
3. Rewrite AC7 as "document every would-demote" approval gate, drop hard zero

Plan is otherwise sound. Fix these three and proceed.
