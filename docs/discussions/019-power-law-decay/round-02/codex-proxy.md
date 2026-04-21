---
agent: codex-proxy
round: 2
date: 2026-04-20
---

# Codex Round 2: Empirical Recalibration — Floor, Percentile Rule, Promotion Predicate

## Executive Summary

Round 1's floor recommendation (0.30) was data-blind. With archaeologist's empirical distribution (15-day-old corpus, mean `avg_relevance = 0.487`, 86% in 0.46–0.50), Codex **revises floor downward to 0.10**, matching architect's position. Key insight: the corpus is too young (all recalls within 15 days) for aggressive demotion. Percentile rule is implementable but premature on this snapshot. Promotion stays on `avg_relevance` (not `effective_relevance`), preserving Architect's safer first-ship semantics.

---

## Question 1: Floor Value Calibration (0.10 vs 0.30 vs Percentile)

**Codex recommendation: revise to `demotion_floor = 0.10`.**

**Math:**
With observed mean `avg_relevance = 0.487` and `H = 60`:
- Floor 0.30: `0.487 × 2^(-d/60) < 0.30` → `d > 41 days`
- Floor 0.10: `0.487 × 2^(-d/60) < 0.10` → `d > 96 days`

The corpus is only 15 days old; every recalled long-term memory is within that window. A 41-day trigger is 2.7× the corpus age; a 96-day trigger is 6.4× and is more consistent with "young corpus, limited tail data."

**Why this overturns my Round 1 recommendation:**

In Round 1, I proposed 0.30 with a 44-day trigger as "enough separation from the 14-day promotion window." That was sound reasoning in isolation. But I didn't account for the corpus's **actual age**. When the entire recall history is 15 days old, a 41-day demotion trigger will fire on memories that are only 26–41 days post-promotion — which is aggressive enough to demote memories still in their natural "post-promotion settling" phase.

By contrast, 0.10 with a 96-day trigger respects the corpus's youth: it demotes only memories that have been silent for 6+ months, which cannot happen in a 15-day-old corpus. The first real demotion will only occur as the corpus matures.

**Percentile rule recommendation: not now.**

The 10th percentile of the observed distribution is necessarily near 0.46–0.47 (since 86% are in 0.46–0.50 and the min is 0.462). Mathematically:
- `0.487 × 2^(-d/60) < 0.462` → `d > 4.5 days`

That is far too aggressive and contradicted by the fact that all recall data are within 15 days. Percentile calibration only makes sense when the distribution is broad and stable over months. Right now it's too tight and too young.

**Trade-off and revision trigger:**

I would reconsider percentile-based demotion only after:
- Corpus has real age spread (90–180 days of recall history)
- 10th percentile separates from the mean (not sitting on top of it)

If at month 3 the distribution widens and the 10th percentile drops to, say, 0.40, then a percentile rule becomes credible. For now: fixed 0.10.

---

## Question 2: Percentile-Rule Implementability

**Recommendation: implementable in SQLite, but premature for this corpus.**

**SQL version (window-function based, no schema change, under 50 LOC):**

```sql
WITH ranked AS (
  SELECT
    avg_relevance,
    ROW_NUMBER() OVER (ORDER BY avg_relevance) AS rn,
    COUNT(*) OVER () AS n
  FROM memory_entries
  WHERE is_longterm = 1
    AND recall_count > 0
),
params AS (
  SELECT
    ((n - 1) * 0.10) + 1.0 AS pos
  FROM ranked
  LIMIT 1
),
bounds AS (
  SELECT
    pos,
    CAST(pos AS INT) AS lo,
    CAST(pos + 0.999999999 AS INT) AS hi
  FROM params
)
SELECT
  CASE
    WHEN lo = hi THEN
      MAX(CASE WHEN rn = lo THEN avg_relevance END)
    ELSE
      MAX(CASE WHEN rn = lo THEN avg_relevance END) * (hi - pos) +
      MAX(CASE WHEN rn = hi THEN avg_relevance END) * (pos - lo)
  END AS p10_avg_relevance
FROM ranked, bounds;
```

This computes a true 10th percentile with linear interpolation, no schema change, no materialized views.

**Why I still reject it for this ship:**

Implementable ≠ well-calibrated. On this dataset, `p10` will be ~0.46–0.47, which would demote an average memory after only ~4–5 days under the current decay rule. That is inconsistent with the corpus's youth and will produce surprising early demotions.

**Trade-off:**

Once the recall history has real age spread (months old), I would revisit this. If the 10th percentile moves to, say, 0.38–0.40 and remains stable across weeks, the percentile rule becomes defensible. For now, the fixed 0.10 is simpler and more appropriate.

---

## Question 3: Promotion Predicate — `avg_relevance` or `effective_relevance`?

**Recommendation: keep promotion on raw `avg_relevance >= 0.45`, not `effective_relevance`.**

**Math:**

With `H = 60` and considering memories up to 15 days old:
- Decay factor at day 15: `2^(-15/60) = 2^(-0.25) ≈ 0.8409`
- A typical memory (mean 0.487): `effective = 0.487 × 0.8409 ≈ 0.409`
- A high memory (0.50): `effective = 0.50 × 0.8409 ≈ 0.420`

If promotion switched to `effective_relevance >= 0.45`, many recently promoted long-term memories (only up to 15 days old) would immediately fail the predicate despite having solid `avg_relevance`. That is the mass-demotion risk Architect identified.

**What threshold would it take if you forced consistency?**

To preserve the current 0.45 avg standard for a 15-day-old item:
- `effective = 0.45 × 0.8409 ≈ 0.378`

So you'd need `effective_relevance >= 0.38`. But even that is age-sensitive and brittle on a young corpus.

**Does this affect the floor choice?**

Yes. Keeping promotion on raw `avg_relevance` strengthens the case for a lower demotion floor like 0.10. The asymmetry is intentional:
- **Promotion**: stable gate on fresh signal + raw score (protects recent memories)
- **Demotion**: time-based decay (prunes only after substantial silence)

This is the right semantic for a young corpus. Once the corpus matures and has months of recall history, revisiting symmetry makes sense.

---

## Convergence with Architect's Position

**Codex now agrees with Architect on all three points:**

1. **Floor**: 0.10 (96-day trigger with mean 0.487)
2. **Percentile rule**: Reject as premature (corpus too young and tight)
3. **Promotion predicate**: Keep on `avg_relevance`, not `effective_relevance`

The key difference between Round 1 and Round 2 is **data**: Round 1 was formula-blind; Round 2 is grounded in archaeologist's empirical distribution and the corpus's age.

---

## Key Insight: Young Corpus, Conservative Decay

The entire debate (Architect's 0.10 vs. my Round 1 0.30) hinges on corpus age. With 15 days of observed recall data:
- Aggressive demotion (0.30 floor, 41-day trigger) would be premature and likely produce false positives.
- Conservative demotion (0.10 floor, 96-day trigger) respects the corpus's youth and only fires on truly stale (6+ month) memories.

In 6 months, if the corpus has stabilized and the distribution remains tight, revisit the floor. If the distribution widens (new use patterns emerge), switch to a percentile rule. For now: 0.10 is right.

---

## Codex Raw Output (Round 2)

```
[Full Codex response, verbatim — thread_id: 019dadb3-62c4-7d13-bebf-de1e8a49ba6d]

1. **Floor calibration**

Recommendation: revise to `demotion_floor = 0.10`.

Math:
- Using the observed mean `avg_relevance = 0.487` and `H = 60`:
  - `0.487 * 2^(-d/60) < 0.30` gives `d > 41` days.
  - `0.487 * 2^(-d/60) < 0.10` gives `d > 96` days.
- The corpus is only `15` days old, and every recalled long-term memory is within that 
  same `15`-day window. A `41`-day trigger is only `2.7x` the observed age of the corpus; 
  a `96`-day trigger is `6.4x` the observed window and is more consistent with "young corpus, 
  little tail data."
- I would not use the percentile rule on this snapshot. With min `0.462`, mean `0.487`, and 
  `86%` of values packed into `0.46–0.50`, the `10th percentile` is necessarily near 
  `0.46–0.47`. For a typical memory:
  - `0.487 * 2^(-d/60) < 0.462` gives `d > 4.5` days.
  - That is far too aggressive and is contradicted by the fact that the whole recall record 
    is only `15` days old.

Trade-off:
- `0.10` is conservative and will delay pruning weak memories.
- I would reconsider a percentile-based rule only after the corpus has materially aged, 
  e.g. `90–180` days of recall history with a real low tail. If the `10th percentile` 
  separated from the mean instead of sitting almost on top of it, the dynamic rule becomes 
  more credible.

2. **Percentile-rule implementability**

Recommendation: implementable in SQLite, but I would not adopt it for this corpus yet.

A single-query SQLite version is feasible with window functions. Exact `PERCENTILE_CONT`-style 
interpolation can be done like this:

[SQL code block as above]

That is comfortably under `50 LOC`, needs no schema change, and gives a continuous 10th 
percentile.

Why I still would not use it now:
- Implementable does not mean well-calibrated.
- On this dataset, `p10` will be near `0.46–0.47`, which would demote an average memory 
  after only about `4–5` days under the current decay rule.
- So my recommendation remains a fixed constant, specifically `0.10`, until the dataset 
  has a real age distribution.

Trade-off:
- If later snapshots show a broad, stable distribution over months, I would revisit the 
  percentile rule. Right now the data are too young and too tightly clustered for percentile 
  calibration to be meaningful.

3. **Promotion predicate**

Recommendation: keep promotion on `avg_relevance >= 0.45`, not `effective_relevance`.

Math:
- With `H = 60` and `15` days since recall, decay factor is:
  - `2^(-15/60) = 2^-0.25 ≈ 0.8409`
- A typical memory at the observed mean becomes:
  - `effective_relevance ≈ 0.487 * 0.8409 ≈ 0.409`
- Even a `0.50` memory becomes:
  - `0.50 * 0.8409 ≈ 0.420`
- So if promotion switched to `effective_relevance >= 0.45`, many recently promoted 
  long-term memories would immediately fail despite being only up to `15` days old. That is 
  the mass-demotion risk Architect is pointing at.

What threshold would be needed if you forced consistency?
- To preserve roughly the current `0.45 avg` standard for a `15`-day-old item, you would need:
  - `0.45 * 0.8409 ≈ 0.378`
- So an `effective_relevance` threshold would need to be around `0.38`, and even that would 
  still be age-sensitive and brittle in a young corpus.

Does this affect my floor choice?
- Yes. Keeping promotion on raw `avg_relevance` strengthens the case for a lower demotion 
  floor like `0.10`.
- The asymmetry is intentional: promotion should stay stable while the corpus is young; 
  decay should only prune memories after a substantially longer quiet period.

Trade-off:
- Once the corpus has real age spread and months of recall history, I would revisit 
  symmetry. At that point, moving both promotion and demotion to `effective_relevance` 
  could make sense, but not with the current `15`-day-old dataset.
```

---

## Summary for Team-Lead

Codex's Round 2 revises all three critical positions in light of empirical data:

1. **Floor 0.10** (was: 0.30) — corpus too young for aggressive demotion
2. **Reject percentile rule** (was: willing to consider) — distribution too tight to calibrate
3. **Keep promotion on `avg_relevance`** (was: asymmetric, implicit) — explicit agreement with Architect

Codex now converges with Architect's position. The revision is not a flip-flop; it's Codex following the data. Round 1 was formula-sound but empirically blind. Round 2 grounds the recommendation in the actual corpus age and distribution.
