---
role: archaeologist
round: 2
date: 2026-04-20
---

# Round 2 — Archaeologist Findings

Three concrete verifications requested by synthesis.md Round 2 agenda (clock injection
caller enumeration, percentile distribution query, long-term `last_recalled` NULL rate).

---

## Verification 1: Callers of `run_dreaming` / `run_dreaming_with_config`

### Enumeration of all callers

**`run_dreaming` (no-config wrapper):**

| Location | Type | Line |
|---|---|---|
| `src/core/dreaming.rs:427` | test: `test_dreaming_promotes_qualifying` | internal test |
| `src/core/dreaming.rs:442` | test: `test_dreaming_skips_low_recall` | internal test |
| `src/core/dreaming.rs:459` | test: `test_dreaming_skips_low_relevance` | internal test |
| `src/core/dreaming.rs:473` | test: `test_dreaming_skips_already_longterm` (first call) | internal test |
| `src/core/dreaming.rs:477` | test: `test_dreaming_skips_already_longterm` (second call) | internal test |
| `src/core/dreaming.rs:491` | test: `test_dreaming_skips_invalidated` | internal test |
| `tests/e2e.rs:92` | e2e smoke test | external test |

**`run_dreaming_with_config`:**

| Location | Type | Line |
|---|---|---|
| `src/core/dreaming.rs:52` | called by `run_dreaming` wrapper | internal |
| `src/bin/cli.rs:215` | `mengdie dream` CLI command | production path |

**Total external callers** (outside `dreaming.rs` itself): `src/bin/cli.rs:215` and
`tests/e2e.rs:92`.

### Impact analysis for `Option<DateTime<Utc>>` signature change

Architect and Gemini both propose threading `now` as a parameter. The synthesis
converged on this (synthesis.md line cites `architect.md:155–168`, `gemini-proxy.md:14–32`).

**Gemini's specific proposal** (`gemini-proxy.md:18–28`): add `now: Option<DateTime<Utc>>`
to `run_dreaming_with_config`, defaulting to `Utc::now()` when `None`.

Under this signature:

| Caller | Change needed? | Notes |
|---|---|---|
| `src/bin/cli.rs:215` | **Yes, trivial** — pass `None` | one-line change: add `, None` argument |
| `tests/e2e.rs:92` | **Yes, trivial** — `run_dreaming` wrapper handles it | `run_dreaming` calls `run_dreaming_with_config`; if `run_dreaming` also passes `None`, no change needed at `tests/e2e.rs` |
| `src/core/dreaming.rs:52` | **Yes, trivial** — `run_dreaming` passes `None` | one-line change in the wrapper body |
| `dreaming.rs` tests (×5) | **No change needed** if using `run_dreaming` | all 5 tests call `run_dreaming(None)` which resolves internally |

**Verdict**: The signature change is **contained, not viral**. Only 2 production-path
callers exist (`cli.rs:215` and the `run_dreaming` wrapper at `dreaming.rs:52`). Both
require a single trivial argument addition. All 5 internal dreaming tests call
`run_dreaming` (not `run_dreaming_with_config` directly) and require no changes if the
wrapper's `None` propagation is handled there. `tests/e2e.rs:92` calls `run_dreaming`
directly and requires no change at all.

**No viral spread** — zero callers in `mcp_tools.rs`, `search.rs`, or other modules.

---

## Verification 2: Percentile distribution of `avg_relevance` and `recall_count`

Query run against live `~/.mengdie/db.sqlite`. Population: 110 memories with
`valid_until IS NULL AND recall_count > 0`.

### `avg_relevance` percentiles

| Percentile | Value |
|---|---|
| p10 | 0.4717 |
| p25 | 0.4766 |
| p50 (median) | 0.4841 |
| p75 | 0.4919 |
| p90 | 0.5000 |

**Interpretation**: The distribution is extremely compressed. The interquartile range
(p25–p75) spans only **0.015** (0.4766 → 0.4919). The p10–p90 range is **0.028**
(0.4717 → 0.5000). This is a much tighter cluster than Round 1's bucket analysis
suggested — the "86% in 0.46–0.50" finding was correct but understated the compression.
Even p90 is exactly at 0.50.

**Critical implication for D1 (floor value)**:

Under `H=60` half-life exponential:
- At `avg_relevance = p10 = 0.4717` (the weakest recalled memory):
  - effective crosses floor=0.30 at: `0.4717 × 2^(-d/60) = 0.30` → `d = 60 × log2(0.4717/0.30) ≈ 42 days`
  - effective crosses floor=0.10 at: `d = 60 × log2(0.4717/0.10) ≈ 131 days`
- At `avg_relevance = p90 = 0.50`:
  - effective crosses floor=0.30 at: `d ≈ 44 days` (matches codex-proxy.md:266–273)
  - effective crosses floor=0.10 at: `d ≈ 139 days`

Challenger's "percentile-based floor" framing (`challenger.md:171–178`) can now be
grounded: any fixed floor in 0.10–0.30 applies uniformly across the corpus because the
spread is only 2.8 percentage points between p10 and p90. There is no regime where
"stronger memories" get meaningfully more runway — the distribution is too narrow. A
fixed-value floor is effectively equivalent to a percentile-based floor for this corpus.

### `recall_count` percentiles

| Percentile | Value |
|---|---|
| p10 | 1 |
| p25 | 1 |
| p50 (median) | 2 |
| p75 | 3 |
| p90 | 7 |

**Interpretation**: 50% of recalled memories have been recalled ≤ 2 times. Only 10%
have been recalled ≥ 7 times. The prior-art §1 burst-inflation concern is real: the
p90 of 7 could represent a single ae:analyze session. The promotion threshold of
`recall_count ≥ 3` sits right at the median-to-p75 transition — meaning the 25–50%
of recalled memories around the median are not promoted and may not be at all with the
current threshold.

**Note**: The `recall_count` distribution does NOT bear directly on the decay formula
(which operates on `avg_relevance × time_factor`), but it contextualizes
`avg_relevance`'s signal quality. With p50 recall_count = 2, most memories have
extremely sparse signal — their `avg_relevance` is an average of 1–2 search scores,
not a stable empirical mean.

---

## Verification 3: `last_recalled` NULL rate for `is_longterm=1` memories

### NULL rate

Live query on `is_longterm = 1 AND valid_until IS NULL`:

| Total long-term | `last_recalled IS NULL` | `last_recalled IS NOT NULL` |
|---|---|---|
| 41 | **1** (2.4%) | **40** (97.6%) |

**There is exactly 1 long-term memory with `last_recalled IS NULL`.**

This is a meaningful finding: the proposed decay formula's `created_at` fallback
(`architect.md:42–43`: "if `last_recalled` is NULL, substitute `created_at`") would
apply to only 1 out of 41 long-term memories on the current corpus. The hole is
real but minimal. The 1 affected memory is a promoted synthesis or early-ingested
memory that was promoted without ever being returned by a search. Under the
`created_at` fallback, it decays from its creation date — which is aggressive but
not problematic for 1 memory.

**Flag**: The formula should document the `created_at` fallback explicitly so it's
not invisible behavior.

### Days-since-last-recalled distribution for 40 long-term memories

| Days since last recalled | Count |
|---|---|
| 0 (today) | 8 |
| 1 | 7 |
| 2 | 12 |
| 3 | 6 |
| 4 | 5 |
| 14 | 1 |
| 15 | 1 |

All 40 long-term memories with `last_recalled` set have been recalled within **15
days**. The oldest last-recall is 15 days ago.

### First-pass demotion simulation

Using `effective = avg_relevance × 2^(-days/60)` (H=60 half-life, `last_recalled`-based):

| Floor | Would demote (first pass) | % of 40 |
|---|---|---|
| 0.10 (architect) | **0** | 0% |
| 0.30 (codex) | **0** | 0% |

**Min effective**: 0.3969 (memory last recalled 15 days ago, `avg_relevance=0.472`)
**Max effective**: 0.5650 (memory last recalled 1 day ago, `avg_relevance=0.572`)
**Mean effective**: 0.4733 (vs. mean `avg_relevance=0.487` — ~2.8% decay at current ages)

**Zero long-term memories would be demoted on the first pass under either floor value**
given the current corpus. The youngest long-term memories were promoted from memories
recalled within 15 days, so their effective relevance stays well above both proposed
floors (minimum observed = 0.397, vs. floor=0.30 → still 0.097 above threshold).

**Implication for D1 (architect floor=0.10 vs. codex floor=0.30)**:

The floor debate is **entirely about future behavior**, not first-pass safety. Neither
floor triggers demotions on the current corpus. The distinction matters only once
memories go unrecalled for 42+ days (floor=0.30) or 131+ days (floor=0.10). Since
the corpus is 15 days old at most for recalled memories, this is purely a
calibration-for-the-future question.

**Implication for D2 (ship demotion now vs. decay-only first)**:

With zero first-pass demotions, there is no mass-demotion risk to protect against
on today's corpus. The Gemini dry-run safety concern (`gemini-proxy.md:143–160`) is
valid in principle but not triggered by the current corpus state. Both the architect's
"ship both" and challenger's "decay-only first" positions are safe on this data —
there is no universe where the first pass after shipping silently demotes a cohort of
valid memories, regardless of which floor value is chosen.

---

## Agreements / Disagreements

**Agreements (Round 2):**
- Confirms synthesis.md convergence C3 (clock injection as parameter): the change
  affects ≤ 3 sites, none viral. `Option<DateTime<Utc>>` approach is clean.
- Confirms `LONGTERM_BOOST = 1.2` finding from Round 1 (stale KB item OFC-1 correctly
  flagged and dispositioned by synthesis).
- D5 (naming): no stake — the archaeologist only observes the formula is used as an
  exponential in `dreaming.rs` (`chrono::Duration::days` subtraction → f64 exponent).
  Challenger's correction is factually accurate.

**Disagreements (Round 2):**
- None with peer claims. The `0.30` floor (codex) and `0.10` floor (architect) are both
  safe for the current corpus; the choice is about future aggressiveness, not first-pass
  correctness. This is a design preference question, not a falsifiable empirical one
  from the current data.

## Open Questions

1. **The 1 long-term memory with `last_recalled IS NULL`**: what is it? Likely a
   synthesis row (synthesis memories start with `is_longterm = false` per
   `dreaming.rs:382`, so it was promoted without search recall — possibly a test
   artifact or manually promoted entry). Worth verifying but not blocking.

2. **Future corpus aging**: all first-pass simulations show 0 demotions. The earliest
   date at which demotion first fires depends on when the oldest long-term memory was
   last recalled (currently day 15) plus the floor threshold. At floor=0.30: day 15 +
   ~42 days ≈ 57 days from creation (mid-June 2026 at current rates). At floor=0.10:
   day 15 + ~131 days ≈ 146 days from today (mid-September 2026). Neither is urgent.

3. **`recall_count` p50 = 2**: promotion threshold requires ≥ 3 recalls. Exactly 50%
   of recalled memories have 2 or fewer recalls — meaning most of the 110 recalled
   memories are below the promotion gate by count alone, regardless of `avg_relevance`.
   This is context for how tight the promotion funnel already is before decay layers on.
