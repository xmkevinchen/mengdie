---
agent: architect
round: 1
date: 2026-04-20
---

# Round 1 — Architect

## Findings (with file:line evidence)

### Topic 1: Decay formula & constants

**Position**: Use `effective_relevance = avg_relevance × f(days_since_last_recalled)` where
`f(d) = max(floor, exp(-d / half_life))`. Recommend `half_life = 60` days and `floor = 0.1`
as first-ship defaults.

**Rationale**:

The `0.95^days` sketch in BL-008 is exponential decay with base 0.95, equivalent to a
half-life of ~14 days (`ln(0.5)/ln(0.95) ≈ 13.5`). That is aggressively short for a
knowledge memory system — a decision made 14 days ago drops to 50% weight even if it
was recalled many times in that period. The standard parameterization `exp(-d / H)` with
explicit `H` (half-life) is preferable because:
1. `H` has an intuitive, reviewable meaning ("effective relevance halves every H days").
2. A single knob eliminates bikeshedding over the base constant.
3. 60 days is a reasonable first-ship value for a solo development workflow: a decision
   recalled ~2 months ago still carries meaningful signal; one from 6 months ago warrants
   a 50% haircut.

**Age input**: Drive decay from `last_recalled`, not `created_at`. A memory that was
recalled 3 days ago but created 200 days ago should remain fully weighted — it is still
active. `created_at` ignores the recall signal entirely.

**Floor**: A non-zero floor (suggested: 0.10) prevents effective relevance from reaching
0 via pure time passage. The hard constraint is stored `avg_relevance` is never mutated;
if effective = 0 triggers demotion and the memory is never recalled again, it will never
recover even when it could be relevant. A floor preserves weak-but-alive memories.

**Distribution concern from prior-art §2**: `avg_relevance` clusters at 0.47–0.50 (not
uniform on [0,1]). With `H = 60` days, after 60 days `effective ≈ 0.5 × avg ≈ 0.24`.
After 180 days `effective ≈ 0.125 × avg ≈ 0.06`, hitting the floor. This is acceptable
— an unreferenced 6-month-old memory *should* be considered marginal.

**Formula**: `effective = max(floor, avg_relevance × exp(-(days_since_last_recalled / half_life)))`

where `days_since_last_recalled = max(0, (now - last_recalled).num_days())` and if
`last_recalled` is NULL, substitute `created_at` as a safe fallback.

**No new crate needed**: `chrono::DateTime` subtraction already works in scope; `f64::exp`
is stdlib. Fits the constraint at `dreaming.rs`.

---

### Topic 2: Computation location

**Position**: Compute `effective_relevance` in two places — (a) the Dreaming pass for
demotion decisions, and (b) at search time as a post-fetch re-rank multiplier on the
normalized RRF score. The stored `avg_relevance` is never touched.

**Evidence for split**:

- `search.rs:142`: `LONGTERM_BOOST = 1.2` is already applied post-fetch on `entry.is_longterm`.
  There is already a post-fetch re-ranking step. Adding a decay multiplier here is a
  one-line change in the same pattern.
- `search.rs:147–149`: `record_recall` is called with `normalized` (pre-boost). The boost
  affects only ranking, not the stored signal. Same pattern applies for decay: compute
  `effective = normalized × decay_factor`, use `effective` for ranking, record `normalized`
  for `avg_relevance`.
- `dreaming.rs:56–125`: Promotion SQL does `AND avg_relevance >= ?2`. Demotion logic would
  sit in a new section of `run_dreaming_with_config` after promotion.

**Write amplification concern (hard constraint)**: Dreaming-time demotion runs once daily
on the full corpus. An UPDATE across all N rows where `effective < floor` is proportional
to demoted count, not corpus size — acceptable. Search-time computation is READ-only (no
write); it uses the already-fetched `entry.avg_relevance` + `entry.last_recalled` fields,
both already in the `MemoryEntry` struct (confirmed in schema).

**What search gets**: After decay, the search score becomes:
`final_score = normalized_rrf × decay_factor × longterm_boost_if_applicable`

The LONGTERM_BOOST and decay are complementary: decay brings stale memories down;
the boost remains an incentive for recently-active long-term memories (since decay
on recently-recalled = near 1.0).

**No new stored column needed**: effective relevance is always derived on the fly. This
satisfies the "no new stored state" constraint.

---

### Topic 3: Demotion semantics & threshold

**Position**: Demotion is NOT symmetric with promotion. It uses a dedicated condition:
`is_longterm = 1 AND effective_relevance < demotion_floor` where `demotion_floor = 0.10`
(matching the decay floor). This is asymmetric by design — it avoids flapping.

**Flapping analysis**:

Promotion requires `recall_count ≥ 3 AND avg_relevance ≥ 0.45 AND last_recalled in 14d`.
Demotion requires `effective_relevance < 0.10`. For effective to be < 0.10 with `avg ≈ 0.48`
(median), we need `decay_factor < 0.10/0.48 ≈ 0.21`, which with `H = 60` days means
`exp(-d/60) < 0.21 → d > 93 days`. A memory demoted after 93+ days of no recall cannot be
re-promoted the next day (the `last_recalled in 14d` promotion predicate blocks it). Natural
hysteresis exists — no explicit band needed.

**No new state (`was_longterm`)**: Adding a new flag requires a schema migration (migration
v5), which violates the ~50-100 LOC scope. The existing `is_longterm` 0/1 flag is sufficient:
demotion clears it; the memory can be re-promoted if recalled again meeting promotion thresholds.
This is the correct semantic — demotion is "no longer long-term", not "permanently demoted".

**Threshold calibration**: `demotion_floor = 0.10` with the narrow `avg_relevance` distribution
(0.47–0.50) means demotion triggers at ~93 days of no recall. This is conservative enough to
avoid surprising early demotions on a 238-memory corpus.

**Is demotion necessary for BL-008 to deliver value?** No — decay alone (via search re-rank)
provides the "forgetting" property for search results. Demotion is the complementary mechanism
for the Dreaming promotion gate. Both can ship together; demotion is ~10 LOC.

---

### Topic 4: Interaction with existing promotion thresholds

**Position**: Promotion keeps reading `avg_relevance` (NOT `effective_relevance`). The
`last_recalled >= now - window_days` predicate is preserved and remains necessary.

**Why promotion should NOT switch to effective_relevance**:

The `last_recalled >= 14d` cutoff in promotion (`dreaming.rs:88`) is a boolean gate that
ensures a memory was recently active. Decay is a continuous multiplier — they encode
different constraints. Removing the boolean gate in favor of a continuous value would:
1. Allow a memory recalled 30 days ago with high `avg_relevance` to drop below threshold
   from decay, even though it had significant recent activity.
2. Introduce mass demotion risk on first pass: existing 238 promoted memories would be
   re-evaluated against decayed effective values immediately. Those promoted > 30 days ago
   with no recent recalls would demote on day 1.

**Safer first-ship: demotion-only, promotion unchanged**:

- Promotion: unchanged — still reads `avg_relevance` + `last_recalled` gate.
- Demotion: new — reads `effective_relevance` (computed at Dreaming time), demotes if below floor.
- No mass disruption on first pass: demotion only fires for long-term memories with 93+ days
  of no recall (conservative enough).

**Migration concern**: 238 existing memories were promoted under old rules. First Dreaming
pass after BL-008 ships will check all `is_longterm = 1` memories for demotion. Memories
promoted recently (within 60 days) will have effective > floor. Memories that are legitimately
stale (90+ days no recall) will be demoted — which is the correct behavior.

**Scope check**: Changing promotion predicate AND adding demotion in one PR would be two
behavior changes. Keeping promotion unchanged makes BL-008 a single-surface addition.

---

### Topic 5: Observability & testing strategy

**Position**: Inject `now: DateTime<Utc>` parameter into `run_dreaming_with_config`; add
`demoted: usize` and `avg_effective_relevance: f64` to `DreamingResult`.

**Clock injection strategy**:

`dreaming.rs:62`: `let now = chrono::Utc::now();` — change to accept a parameter:
```rust
pub fn run_dreaming_with_config(
    &self,
    project_id: Option<&str>,
    config: &DreamingConfig,
    now: chrono::DateTime<chrono::Utc>,  // injectable for tests
) -> anyhow::Result<DreamingResult>
```

The public-facing `run_dreaming` wrapper continues to call `Utc::now()` internally and pass
it down. No new trait, no new crate — pure parameter injection. This gives deterministic
time in tests without the complexity of a mock-clock abstraction.

**`DreamingResult` additions**:
- `demoted: usize` — count of `is_longterm` memories cleared this pass. Operators see this
  in CLI output; a sudden spike signals calibration issues.
- `avg_effective_relevance: f64` — mean effective relevance across all `is_longterm = 1`
  memories, computed during the demotion scan. Trend over successive runs indicates whether
  the corpus is healthy (stable), slowly decaying (old memories aging out normally), or
  sharply dropping (over-aggressive).

**`metrics.rs` counters**: Add `dreaming.demotions.total` as a monotonically increasing
counter in the metrics table. The existing metrics infrastructure supports this without a
new crate.

**Empirical calibration signal**:
- Over-aggressive: `demoted > 10%` of long-term memories on first pass.
- Under-aggressive: no demotion after 6 months of operation.
- Correct: gradual demotions of memories with `last_recalled > 90 days` of no recall.

**`--dry-run-decay` flag**: Explicitly out of scope for BL-008 (~50-100 LOC target).
The `dry_run` flag already exists for synthesis. If decay logging is sufficient, a separate
`--dry-run-decay` mode is scope creep. Tracing at INFO level during the demotion scan
(memory_id, effective, threshold) is sufficient for manual inspection.

---

## Agreements

N/A — Round 1, no peer files read.

## Disagreements

N/A — Round 1, no peer files read.

## Open Questions

1. **`last_recalled` NULL handling**: What fraction of the 238 production memories have
   `last_recalled IS NULL`? (Memories ingested but never searched hit this case.) The
   fallback to `created_at` is conservative but means a just-ingested, never-recalled memory
   decays from birth. Is that the intended behavior or should never-recalled memories skip decay?

2. **Search score ordering after decay**: After applying `decay_factor` at search time, the
   sort order of results can change (a stale high-`avg_relevance` memory drops below a fresh
   low-`avg_relevance` one). This is the intended behavior, but the `record_recall` call
   (`search.rs:148`) still records the normalized-RRF score (pre-decay). That means
   `avg_relevance` accumulates the undecayed signal. Over time, a memory that keeps showing
   up in search results but for stale reasons will inflate `avg_relevance`. This is a
   pre-existing issue (prior-art §1 — `recall_count` inflation) but decay makes it slightly
   worse for search-surface memories. Flag for a future BL, not a blocker.

3. **Promotion predicate and decay interaction in the long run**: Once the corpus grows (1000+
   memories), many memories will have decayed below the effective threshold. Promotion will
   still fire on `avg_relevance` (not decayed), so new memories will promote easily while old
   ones demote. This is directionally correct but should be validated after 3+ months of use.
   Not a design change needed now.

4. **Half-life tuning**: `H = 60` days is a first-ship estimate. The discussion should
   acknowledge this needs empirical tuning. The backlog item for final constant selection
   (mentioned in framing.md `Non-questions`) should carry the trigger: "revisit after first
   30-day operational report shows avg_effective_relevance trend."
