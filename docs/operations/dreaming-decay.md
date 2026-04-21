# Operating the Dreaming Decay Pass (BL-008)

This is the operator procedure for the exponential decay mechanism shipped
in plan 013. For the design record, see
[discussion 019](../discussions/019-power-law-decay/conclusion.md); for
the step-by-step implementation, see
[plan 013](../plans/013-exponential-decay.md).

## Quick reference

| Concept | Value |
|---|---|
| Formula | `effective = avg_relevance × 2^(-days_since_last_recalled / 60)` |
| Half-life | 60 days |
| Demotion floor | 0.20 |
| Rough first-demotion trigger | ~77 days of silence at `avg ≈ 0.487` |
| Stored `avg_relevance` | **never mutated** — decay is read-time |
| Age source | `last_recalled` (memories with `NULL` are skipped) |

## Required first-run procedure

Before the first live `mengdie dream` after BL-008 lands, an operator MUST:

1. Run `mengdie dream --decay-dry-run`. This performs the promotion pass as
   usual (writes promotions), then scans long-term memories for the decay
   pass WITHOUT clearing any `is_longterm` flags. Any memories that WOULD
   demote are reported as `decay_floor_breaches` and listed in the
   structured-JSON stderr line's `breaches` array.

2. Inspect the output. The human-readable line looks like:
   ```
   Dreaming pass: 2 promoted, 3 would-demote (DRY RUN) (3 floor breaches, avg effective 0.421 → 0.421)
   ```
   The structured-JSON line on stderr carries the full breach list.

3. **Approval gate**: if `decay_floor_breaches > max(10, 10% of the
   current long-term count)`, HALT. Either (a) recalibrate the floor (edit
   `DEMOTION_FLOOR` in `src/core/decay.rs`, rebuild), or (b) investigate
   each breached memory individually — the list is in the JSON. A spike
   this large on a young corpus means distribution drift; do not proceed
   to a live pass until the cause is understood.

4. If the breach count is within tolerance, proceed with a live
   `mengdie dream`. The human line will now show `N demoted` instead of
   `N would-demote (DRY RUN)`.

This procedure lives as long as `mengdie dream` is operator-invoked. When
the launchd daemon (BL-010 in Phase 2.2) takes over, the approval gate
transitions from an interactive `--i-reviewed-each` flag to a threshold
alarm in `decay_floor_breaches`. See plan 013 "Plan-level revisit trigger"
for the reversal shape.

## Metric interpretation guide

All four new `DreamingResult` fields are visible in the human CLI line and
the structured JSON line on stderr.

| Field | Meaning | Operator signal |
|---|---|---|
| `demoted` | Memories whose `is_longterm` was cleared in this pass. Always `0` in dry-run. | A live-run spike (`demoted > 10%` of long-term count) means the corpus just crossed the staleness threshold en masse — expected after a long hiatus, worth a closer look on a young corpus. |
| `decay_floor_breaches` | Memories whose effective relevance is below the floor. In live mode, equal to `demoted`. In dry-run, `demoted = 0` but this count reflects what WOULD demote. | The steady-state signal. A sustained non-zero here means stale memories keep arriving at the floor — watch the age-profile trend across successive passes. |
| `avg_effective_score_before` | Mean effective relevance across all `is_longterm = 1 AND last_recalled IS NOT NULL` memories, computed BEFORE any demotion write. | A slowly-falling series indicates the corpus is aging without fresh recall — a signal to examine why the AE pipeline isn't re-surfacing old-but-relevant memories. |
| `avg_effective_score_after` | Mean across SURVIVORS after demotions write (live) OR identical to `_before` (dry-run — no writes occurred). | `(after - before)` quantifies how much the demotion raised the per-memory mean. A large positive delta means demotion is working as intended; a zero delta over time means nothing is ever being demoted. |

The `breached_ids` array in the structured JSON line lists the specific
memory IDs that fell below floor this pass — use this to inspect
individual memories via `mengdie list --format json | grep <id>`.

## Data freshness: what "corpus age" means

In plan 013's revisit triggers, "corpus age > 90 days" is shorthand for
**the longest `last_recalled` gap** among `is_longterm = 1` memories —
NOT ingest age. An ingest-heavy recent week does not count as "fresh" for
decay purposes; what matters is whether memories are being recalled.

To check the current longest gap:

```sql
SELECT
  id,
  ROUND((julianday('now') - julianday(last_recalled)) * 86400 / 86400, 1) AS days_since_recall
FROM memory_entries
WHERE is_longterm = 1
  AND valid_until IS NULL
  AND last_recalled IS NOT NULL
ORDER BY days_since_recall DESC
LIMIT 5;
```

If the top row exceeds 90 days, revisit the floor calibration (see
revisit triggers below).

## The LONGTERM_BOOST cliff

When Dreaming demotes a stale memory, its search-time score drops from
`normalized × 1.2 × decay_factor` to `normalized × decay_factor` on the
very next query. This is a **one-time discontinuity** and is the
mechanism by which demotion becomes user-visible. Subsequent queries
just apply the decay without the boost; the scoring then evolves
continuously from that new level.

This is intentional. If you see a search-rank regression correlated with
a Dreaming pass, cross-reference the demoted id against the `breaches`
array from that pass's JSON line. A regression on a NON-demoted memory
is a real bug; a regression on a demoted memory is the system working.

## Revisit triggers

Re-open plan 013 (and by extension discussion 019 Topic 1) if any of:

- `avg_effective_relevance` across the corpus drops below 0.25 on a
  Dreaming pass.
- Longest `last_recalled` gap exceeds 90 days (see "Data freshness").
- `avg_relevance` IQR widens past 0.05 — the compressed-distribution
  assumption no longer holds; a percentile-based floor becomes viable.

First reversal shape: lower `DEMOTION_FLOOR` to 0.15, OR switch to a
corpus-relative rule `mean − 1.5 × IQR`. Both are one-line edits + a
regression-table refresh in `src/core/decay.rs`.

## Baseline

Populated by the first `mengdie dream --decay-dry-run` on the
production DB (`~/.mengdie/db.sqlite`).

| Date | `decay_floor_breaches` | `avg_effective_score_before` | `avg_effective_score_after` | Longest recall gap (days) |
|---|---|---|---|---|
| 2026-04-20 | 0 | 0.4712 | 0.4712 | <15 |

Matches archaeologist V3 simulation from discussion 019 Round 2
(min effective observed = 0.397 > floor of 0.20). Safe to proceed to
live `mengdie dream` — no demotions will fire on the current corpus.

## AE parser audit

At plan-draft time (2026-04-20) a `rg "Dreaming complete"
../agentic-engineering/` returned empty — no AE plugin consumer parses
the pre-BL-008 CLI output format. If a future consumer emerges, the
structured-JSON line on stderr is the stable interface; the human line
is explicitly free to evolve.
