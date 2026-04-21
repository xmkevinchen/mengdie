---
agent: codex-proxy
round: 1
date: 2026-04-20
---

# Codex Round 1: Power-Law Decay Formula & Correctness

## Synthesis

Codex strongly recommends moving away from the BL-008 sketch (`0.95^days`) toward a **half-life exponential with a 60-day half-life**. The sketch is mathematically sound but functionally too aggressive for Mengdie's compressed `avg_relevance` distribution. Codex's concrete evaluation reveals:

1. **Formula**: Use `effective(d) = avg_relevance * 2^(-d / 60)` instead of `avg_relevance * 0.95^d`
2. **Why**: At `avg_relevance = 0.5`, the sketch gives ~0.005 by day 90 (effectively dead); Codex's formula gives ~0.177 (still active but decayed)
3. **Numeric stability**: No IEEE 754 pitfalls up to 1000+ days; real concern is computing the factor correctly (Rust, not SQL)
4. **Demotion hysteresis**: Necessary by design (recall resets `last_recalled`, creating a sawtooth); recommend asymmetric promotion + separate demotion floor

---

## Key Findings (Per Question)

### 1. Formula Choice & Concrete Evaluation

**Codex recommendation**: Half-life exponential, not raw `0.95^days`, not power-law proper.

```
effective(d) = avg_relevance * 2^(-d / H)
```
with `H = 60 days` (true half-life).

**Concrete math for `avg_relevance = 0.5`**:

| Days | Codex Formula | BL-008 Sketch (`0.95^d`) |
|------|---------------|-----------------------|
| 7    | 0.461         | 0.322                 |
| 30   | 0.354         | 0.008                 |
| 90   | 0.177         | **0.005 (dead)**      |
| 180  | 0.0625        | **2.6e-8 (dead)**     |

**Why this form**:
- One interpretable knob (half-life in days)
- Well-behaved at `d = 0` (returns `avg_relevance` unchanged)
- Power-law proper (`1/d^k`) is wrong-shaped:
  - Undefined at `d = 0`
  - Requires arbitrary shift like `(1 + d/τ)^-k`, adding design choices
  - Heavy tail keeps stale burst memories alive too long
- BL-008's sketch is too aggressive: at 90 days, it collapses month-old memories to noise, which is incompatible with a promotion window of only 14 days (memories promoted in week 2 are already decay-dead by day 30)

**Rationale for H = 60 days**:
- `avg_relevance` already inflates from burst recalls (intra-session duplication)
- That argues for recentness decay faster than for cleaner signals, but not BL-008-level aggressive
- `H = 60` reaches ~33% effectiveness at month, ~18% at quarter, ~6% at 6 months
- Separates from the 14-day promotion window enough to avoid daily flapping

---

### 2. Half-Life Parameterization Trade-Off

**Codex verdict**: Yes, half-life parameterization is far more reviewable. But do it correctly.

**Correct forms**:
- `2^(-d / H)` — H is the true half-life
- `exp(-ln(2) * d / H)` — same, different base
- ❌ Do NOT use `exp(-d / half_life)` — this makes half-life = `half_life * ln(2)`, not what you named

**What half-life to pick**: `H = 60 days` as above.

**vs. BL-008's implicit 13.5 days**: Solving `0.5 = 0.95^t` gives `t = log(0.5) / log(0.95) ≈ 13.5`. This is too short for a corpus with burst-inflated signal—it treats 30-day-old useful context as decay-dead.

---

### 3. Numeric Correctness in SQLite

**Does `0.95^1000` underflow?**

Exact value: `0.95^1000 = 5.29e-23`
- Does NOT underflow to zero in IEEE 754 doubles
- Still a normal positive number
- Underflow only occurs around day 13,810 (subnormal range floor)
- Literal zero around day 14,513

**Real pitfalls** (not IEEE 754):
1. **SQLite math portability**: `pow()` not guaranteed to exist in minimal SQLite builds
2. **Timezone semantics**: `julianday()` differences are FP and timezone-sensitive; compute elapsed days in Rust, not SQL
3. **Read-time vs. write-time**: Never mutate `avg_relevance`; compute decay on-the-fly from stored `avg_relevance + last_recalled`
4. **Search/ingest path**: If decay moves into frequent reads, prefer computing the decay factor in Rust and comparing in log-space rather than relying on SQL `pow()`

**Numeric stability recommendation**: Codex would still prefer the half-life exponential and compute the factor in Rust (tokio::task::spawn_blocking with cheap computation, or precompute hourly), avoiding repeated calls to SQLite's FP math.

---

### 4. Demotion Hysteresis & Flapping

**Does the smooth decay curve itself prevent flapping?**

No. The system is not smooth; it is a **sawtooth**:
- Between recalls: monotone downward decay
- On recall: `last_recalled` resets to now, and `avg_relevance` can jump upward (or stay flat if that recall didn't add new ranking signal)

So after a recall, a memory can re-cross the demotion threshold upward, re-entering long-term status.

**Codex recommendation**: Structural hysteresis via **asymmetry**, not a narrow band.

Promotion rule (existing):
- `recall_count >= 3`
- `avg_relevance >= 0.45`
- `last_recalled within 14 days`

Demotion rule (new):
- `effective_relevance < demotion_floor` (e.g., 0.30)

This asymmetry is sufficient because the conditions are different: promotion requires fresh signal (`last_recalled within 14d` + multiple recalls), while demotion is purely time-based.

**If insisting on same-metric demotion band**:
- Use `X = 0.45`, `δ = 0.15` (demote below 0.30)
- Do NOT use a tiny `δ` (e.g., 0.05); with compressed `avg_relevance` distribution, narrow bands are mostly false precision

**Flapping timeline**: With `H = 60` and `avg = 0.5`, crossing 0.30 happens at ~44 days. That is enough separation from the 14-day promotion window to avoid daily churn, but still reversible after renewed use.

---

## Cross-Family Note

Codex's key departure from BL-008's sketch is not about exponential vs. power-law (both are exponential families), but about **magnitude**: a 60-day half-life instead of 13.5-day. This reflects Codex's independent assessment that the corpus's burst-inflated signal needs gentler decay than the sketch proposes. The formula is correct; the constant is off.

---

## Codex Raw Output (Round 1)

```
[Full Codex response as returned, verbatim — see Round 1 thread thread_id: 019dadac-876d-7b60-8c01-ce6f30bfac09]
```

---

## Recommendations for Discussion Consensus

1. **Formula**: Move from `0.95^days` to `2^(-days / 60)` with review by the architect
2. **Computation**: Compute the decay factor in Rust (not SQL), on-the-fly (not stored)
3. **Demotion**: Use asymmetric rule (existing promotion + new demotion floor) to avoid hysteresis complexity
4. **Threshold**: Suggest demotion floor of 0.30 (derived from 60-day half-life + compressed distribution)
5. **Caveat**: Acknowledge that decay alone cannot fix the burst-recall bias in `avg_relevance`; that is separate work (BL-014)
