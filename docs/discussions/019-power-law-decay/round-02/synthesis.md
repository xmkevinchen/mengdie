---
round: 02
date: 2026-04-20
author: team-lead
---

# Round 2 Synthesis

Orientation layer only. Authoritative positions in the per-agent files
(`architect.md`, `archaeologist.md`, `challenger.md`, `codex-proxy.md`).
Gemini Round 2 missing (API key auth error); see note below.

## Convergences locked this round

### C1. Discussion rename — "power-law" → "exponential decay"
All 3 agents who commented (architect, challenger, archaeologist) agree.
D5 from Round 1 is **converged**. `architect.md:66–71`, `challenger.md:97–118`
(Round 1), `archaeologist.md:212–214` Round 2.

### C2. Formula form — `2^(-d/H)` with `H = 60`
Architect revised away from the misleading `exp(-d/H)` spelling per codex's
Round 1 flag. Implementation: `(2.0_f64).powf(-days / 60.0)` in Rust.
`architect.md:113–125`, `codex-proxy.md:64–68` (Round 1), agreed by
challenger (`challenger.md:106`).

### C3. Age input = `last_recalled` only. No `created_at` fallback.
Architect Round 2 open Q1 (`architect.md:207–217`): only `is_longterm=1`
memories decay, and those by promotion predicate always have `last_recalled
IS NOT NULL` — so no fallback is needed for the decay path.
Archaeologist V3 empirically confirms: 40/41 long-term memories have
non-null `last_recalled`; 1/41 exception (2.4%). Challenger independently
arrived at "never-recalled memories don't decay" (`challenger.md:42–44`).
This implicitly resolves **OFC-3** — memories with `last_recalled IS NULL`
are excluded from the decay formula entirely.

The 1 exceptional long-term memory with NULL `last_recalled` gets an
explicit skip (no decay, no demotion). Lightly suspicious (possibly
a synthesis memory or test artifact per archaeologist open Q1) —
documented as known edge case.

### C4. Clock injection via `Option<DateTime<Utc>>`
Architect accepted Gemini Round 1's cleaner `Option<>` form over their
own required-param proposal (`architect.md:50–51`). Archaeologist V1
confirms signature change is contained: 2 production callers, 3 trivial
edits total, zero viral spread (`archaeologist.md:50–64`).

### C5. Promotion predicate unchanged — reads raw `avg_relevance`, keeps `last_recalled >= 14d` gate
Codex revised Round 1's implicit asymmetric rule to explicit agreement
(`codex-proxy.md:101–127` Round 2). Architect Round 1 held this
(`architect.md:120–149`). Rationale: switching promotion to
`effective_relevance >= 0.45` would cause immediate mass demotion of
recently-promoted memories since even a day-15 memory has effective ≈
0.41 (below 0.45).

### C6. D2 — ship decay AND demotion in BL-008
Challenger conceded (`challenger.md:51–61` Round 2) given archaeologist's
V3 first-pass demotion = 0 for all floor values. Hard precondition:
`demoted` counter MUST land on `DreamingResult` at ship. If counter
deferred, demotion also defers.

### C7. D3 — `--dry-run-decay` flag IN SCOPE
Architect reversed Round 1 rejection (`architect.md:194–202` Round 2):
~40 LOC implementation, fits 50–100 LOC budget if decay formula is
extracted as pure function (needed for testability anyway). Challenger
accepts implicitly via C6.

### C8. D4 — 3-counter observability set
`DreamingResult` gains:
- `demoted: usize`
- `avg_effective_score_before: f64` (renamed from architect's original
  `avg_effective_relevance` per Gemini Round 1)
- `decay_floor_breaches: usize` (distinct from `demoted` only in
  dry-run: counts would-be demotions even when writes suppressed)

`architect.md:160–179` Round 2.

### C9. F3 — recall_count inflation is out-of-scope, NOT a blocker
Challenger option (a): narrow scope claims. BL-008 conclusion must not
claim to fix burst-bias — only staleness. Codex agreed in Round 1
(`codex-proxy.md:275–279`). Burst-bias tracked elsewhere (prior-art §1,
~session-dedup BL).

### C10. Search-time decay uses same age clock as Dreaming-pass decay
Challenger Q4 correctness requirement (`challenger.md:82–90`): both
compute sites MUST use `last_recalled` (not one `created_at`, one
`last_recalled`). Otherwise search ranking and demotion disagree.
Architect's Round 1 Topic 2 implicitly agrees (same `entry.last_recalled`
referenced for post-fetch re-rank).

### C11. Naming cleanup in code — document the LONGTERM_BOOST cliff
When demotion clears `is_longterm`, next search drops from `effective ×
1.2` to `effective × 1.0`. One-time discontinuity, intentional.
Challenger Q4 open item (`challenger.md:88–90`): add code comment at
both demotion site (`dreaming.rs`) and boost site (`search.rs:142`) so
future readers don't diagnose it as a bug.

## Active disagreement — D1 floor value

**Positions after Round 2**:

| Agent | Floor | Trigger at mean `avg=0.487` | Rationale |
|---|---|---|---|
| Architect | **0.20** | ~77 days (`architect.md:91–92`) | Balance "observable within a quarter" against "conservative on young corpus" |
| Challenger | **0.20** | ~75 days (`challenger.md:42`) | Conservative against 2-month project hiatus; survives distribution shift |
| Codex | **0.10** | claims 96 days (`codex-proxy.md:22`) — **see math note** | Respect corpus youth; delay first demotion to 6+ months |

**Math discrepancy (TL flag)**: Codex's Round 2 reports floor=0.10 triggers
at 96 days. Independent calculation: `0.487 × 2^(-d/60) < 0.10` →
`2^(-d/60) < 0.2054` → `d > 60 × log2(4.868) ≈ 137 days`. Architect's
Round 2 math (`architect.md:81–93`) also yields 137 days. Codex's 96-day
figure appears to be an arithmetic slip — the number 96 does not fall
out of either `avg=0.487` or `avg=0.50` at floor=0.10 under `H=60`.

If codex's stated rationale ("respect corpus youth, delay first demotion")
is applied with the corrected 137-day trigger, architect's Round 2
counter-argument applies: "137 days on a 15-day-old corpus means no
demotion for ~4 months post-ship — too conservative; feature produces no
observable behavior within any reasonable horizon" (`architect.md:80–85`).
That collapses codex's 0.10 position into architect's 0.20.

### TL verdict on D1 (autonomous)

**Floor = 0.20**. Evidence preponderance:

1. Independent convergence: architect and challenger arrived at 0.20
   independently from different axes (architect from
   observation-horizon; challenger from hiatus-resilience). Independent
   arrival is a strong signal.
2. Archaeologist V3 empirical: zero first-pass demotions under any floor
   value (0.10, 0.20, or 0.30). D1 is entirely about future behavior,
   not first-pass safety. No safety argument favors 0.10 over 0.20.
3. Codex's 0.10 is based on "96-day trigger" which is a math error;
   actual trigger at 0.10 is 137 days. Corrected, codex's rationale
   supports 0.20 over 0.10.
4. Percentile alternatives rejected: archaeologist V2 shows
   distribution IQR is only 0.015 — percentile-based and fixed-value
   floors are numerically equivalent on this corpus. Challenger
   withdrew the percentile proposal.

Revisit trigger logged at the end: reassess when `avg_effective_relevance`
across the corpus drops below 0.25 OR when corpus age exceeds 90 days
OR when distribution widens beyond IQR=0.05 (follow up via BL, not this
plan).

Consensus verification for D1: not triggered — the disagreement is
narrow (0.20 vs 0.10), backed by independent convergence at 0.20, and
codex's position contains an identifiable math error that would resolve
on recomputation. Running ae:consensus Debate Mode here would add ritual
without new information.

## Of-framing disposition

| # | Challenge | Raised by | TL disposition | Rationale |
|---|---|---|---|---|
| OFC-1 (R1) | Stale KB memory — `is_longterm` IS read by search | challenger R1 | **integrated R1 via memory_invalidate** | Done; see Round 1 synthesis. Correction memory `e2b0bb63` supersedes stale `2c3122ff`. |
| OFC-2 (R1) | "Forgetting matters" non-question forecloses threshold-as-design-question | challenger R1 | **integrated R1** | D1 tracked as design question not tuning knob; resolved this round at floor=0.20. |
| **OFC-3 (R2)** | `created_at` NULL-fallback is hidden design decision; applies to 66% of corpus | challenger R2 (`challenger.md:96–98`) | **INTEGRATE — resolved this round** | Resolution: decay formula applies ONLY to `is_longterm=1` memories. By promotion predicate, those always have `last_recalled IS NOT NULL` (empirically 40/41 on current corpus). The 1 exception (2.4%) gets an explicit skip — no decay, no demotion. Never-recalled memories (`is_longterm=0`, `last_recalled IS NULL`) are untouched entirely. This eliminates the 66% concern — those 213 memories are `is_longterm=0` and were never in the decay path anyway. |

Frame-challenges from prior rounds: OFC-1 and OFC-2 both dispositioned
in R1 synthesis. Keyword scan of R2 per-agent files confirms neither
silently reappeared or re-opened — both remain integrated.

## Verification artifacts

| Claim | Artifact |
|---|---|
| 0.20 floor triggers at ~77 days (mean 0.487) | Architect computation `architect.md:91–92`; independent check: `0.487 × 2^(-77/60) = 0.487 × 0.411 = 0.200` ✓ |
| 0.10 floor triggers at 137 days, NOT 96 | TL computation above; architect `architect.md:81–85` independently. Codex `codex-proxy.md:22` reports 96 (flagged as arithmetic slip) |
| IQR of `avg_relevance` = 0.015 | Archaeologist V2 live DB query (`archaeologist.md:74–81`) |
| 0 first-pass demotions regardless of floor | Archaeologist V3 simulation (`archaeologist.md:170–201`) |
| 1/41 long-term memories has `last_recalled IS NULL` | Archaeologist V3 live query (`archaeologist.md:130–148`) |
| `run_dreaming*` callers: 2 non-test callsites | Archaeologist V1 enumeration (`archaeologist.md:18–40`) |
| `exp(-d/H)` ≠ `2^(-d/H)` (half-life off by `ln2`) | Codex R1 `codex-proxy.md:64–65`; math: `exp(-1) = 0.368 ≠ 0.5 = 2^(-1)` at `d=H` |
| LONGTERM_BOOST=1.2 at `search.rs:142` | Grep-confirmed in Round 1 by archaeologist and challenger |

Unvalidated:
- Gemini's regenerated test table under new formula — Gemini R2 missing
  (auth error). TL will regenerate the 3 test cases in the plan, not
  the conclusion. Not a blocker.

## Frame-challenge disappearance self-check

Regex/keyword scan of Round 1 OFCs (OFC-1, OFC-2) against Round 2
per-agent files: both OFCs remain integrated. No silent drop. OFC-3
newly raised in Round 2 (challenger) and dispositioned in this
synthesis.

## Pruned

Pruned:
- **Percentile-based floor** — rejected. Archaeologist V2 (IQR=0.015)
  shows percentile and fixed-value floors are numerically
  indistinguishable for this corpus. Challenger withdrew proposal
  (`challenger.md:25–26`). Codex rejected as premature
  (`codex-proxy.md:32–46`).
- **`created_at` fallback** — rejected per OFC-3 resolution. No memory
  path uses it.
- **Decay-only-first-ship** — superseded by C6. Challenger conceded on
  empirical safety.
- **`exp(-d/H)` formula spelling** — superseded by `2^(-d/H)` per C2.

## Gemini R2 absence — TL disposition

Gemini-proxy correctly reported API key auth error and stopped
(improvement over R1 silent fallback — protocol followed). TL decision:
do NOT reroute Gemini R2. Rationale:
- Gemini R1 observability positions all adopted by architect R2
  (3-counter set, Option<> clock injection, dry-run reversal).
- Only outstanding Gemini R2 item is regenerating the 3 test cases
  under `2^(-d/60)`. That's plan-work, not design. Will appear in the
  `/ae:plan` output, not the conclusion.
- Spawning a Claude fallback agent with "observability lens" is
  redundant with existing coverage.

## Round 3 gate — NOT triggered

All R2 disagreements resolved via TL verdict + empirical evidence:
- D1 floor: 0.20 via evidence preponderance (documented above)
- OFC-3: integrated

Remaining items all scorable. Moving to Step 5 (TL scoring) without
Round 3. All agents stay in team for Sweep + Doodlestein.
