---
round: 2
date: 2026-04-28
team: 028-council
agents_reporting: [architecture-reviewer, minimal-change-engineer, challenger, codex-proxy, gemini-proxy]
---

# Round 2 — TL Synthesis (orientation only)

## Pruned

Round 2 had several agents update positions or prune their own
proposals based on peer evidence:

- **Pruned: arch-reviewer's RRF-score-< 0.35 trigger**, reason: in
  Round 2 they swapped to minimal-change-engineer's stale-delivery
  count, citing "score-distribution is server-side signal-quality
  proxy, not end-to-end quality."
- **Pruned: minimal-change-engineer's "1 independent A-MEM
  replication paper" clause**, reason: not programmatically
  observable; conceded.
- **Pruned: codex-proxy's eval-dependent trigger ("insufficient_context
  ≥15% on eval", "offline ablation ≥8pp gain")**, reason: required
  v0.0.1 to build eval infrastructure not in scope; updated to
  audit-native 3-AND.
- **Pruned: challenger's ACK-protocol-required path for Topic 4**,
  reason: challenger themselves ruled out MCP ACK in v0.0.1 (used
  signal ambiguous; contractual burden too high) — forces all Topic
  4 triggers to be server-side observable.
- **Pruned: gemini's Topic 2 agnosticism**, reason: updated to
  REJECT permanently (concur with arch-reviewer/minimal-change/
  challenger).

## Of-framing disposition

One of-framing meta-decision surfaced this round:

- **MCP `memory_search` ACK feedback channel — should v0.0.1
  contract include it?** Raised by challenger as load-bearing across
  Topic 4. Disposition: **integrated as internal architectural
  decision**. challenger Round 2 argued NO; rationale: "used" signal
  is ambiguous (AI exclusion-discard still counts as used);
  contractual burden on every integrator > value of noisy precision
  estimate. Other agents' Round 2 positions are consistent with
  no-ACK (all converged on audit-log-derived signals that don't
  require caller acknowledgment). Resolved in-team, no escalation.

No agent challenged the framing structure or scope.

## Verification artifact

| Claim | Source | Status |
|---|---|---|
| "Storage trait + search-split refactor are independent decisions" | arch-reviewer round-02 + minimal-change round-02 | **converged design claim** — all 4 of 5 majority cite same evidence (search-split is mcp_tools defect-fix cleanup; trait introduction is YAGNI question with no second impl) |
| "ANN-based clustering is similarity primitive swap, not 2nd reflection strategy" | 5-of-5 Round 2 | **converged** — falsification ("name a v0.0.1 call site selecting ≥2 strategies at runtime") attempted by minimal-change; no agent named one |
| "AE artifact creation-time and ingestion-time agree within seconds in production" | not produced | **unvalidated empirically**; remaining open question for operator-level confirmation; affects Topic 2 governance argument (DEFER vs REJECT) |
| "Audit table must log returned_fact_ids per search call" | arch-reviewer + 4-agent Topic 4 trigger consensus | **integration constraint identified** — flagged as v0.0.1 instrumentation requirement; not a verification artifact, an instrumentation TODO |
| "DEFER trigger '>60s gap' is unobservable from v0.0.1 instrumentation" (chicken-and-egg) | minimal-change-engineer round-02 | **verified design claim** — codex did not refute; codex's response acknowledged governance-only difference |

## Frame-challenge disappearance self-check

Round 1 of-framing markers: none.
Round 2 of-framing markers: 1 (MCP ACK protocol meta-decision; integrated).

No silent disappearance.

---

## Position evolution by topic

### Topic 1 — Storage abstraction timing + mechanism

Round 1 → Round 2:
- arch-reviewer: NO trait + free fns → **same**, sharpened ("decisions are independent")
- minimal-change: NO trait + free fns → **same**, added Rust-nominal-typing-cost argument against gemini's Google pattern
- challenger: NO trait default → **HOLD**; engaged gemini's Google pattern, ruled it pattern-evidence-not-mengdie-evidence
- codex: defer trait + concrete internally → **clarified** = free fns over &Db; convergent with majority
- gemini: CONDITIONAL ACCEPT trait (if search-split refactor) → **HOLD** but conditioned on "if search-split is in v0.0.1, define trait"

**Final 4-of-5**: free functions over `&Db`, NO Storage trait in v0.0.1. **gemini outlier** — but gemini's position reduces to "if search-split → trait." Other 4 say search-split + Storage trait are independent decisions; even with search-split in v0.0.1, no trait.

### Topic 2 — Bi-temporal `event_time` column

Round 1 → Round 2:
- arch-reviewer: REJECT permanently → **same**; alternative `valid_from` override on `memory_ingest`
- minimal-change: REJECT permanently → **same**; new chicken-and-egg argument (DEFER trigger can't fire without the column it gates)
- challenger: maintain reject until evidence → **UPDATED to REJECT permanently**; conceded "DEFER with trigger that has never fired and requires new observability is operationally indistinguishable from never"
- codex: DEFER with trigger → **HOLD**; argues for solo-operator-lower-friction governance
- gemini: agnostic → **UPDATED to REJECT permanently**

**Final 4-of-5**: REJECT permanently. **codex outlier** — only governance-style argument (DEFER auto-fires; REJECT requires re-discussion). Operationally indistinguishable per minimal-change's analysis.

### Topic 3 — Reflection consolidation + Reflector trait (UAG candidate)

5-of-5 affirm:
- Defer Reflection module consolidation pending sqlite-vec compatibility spike outcome
- NO Reflector trait in v0.0.1 regardless of sqlite-vec outcome

Falsification attempts:
- minimal-change: "name a v0.0.1 call site that selects between ≥2 reflection strategies at runtime" — none named
- arch-reviewer: ANN swap doesn't change algorithm identity — none refuted
- challenger: absence of runtime call site selecting strategies definitively closes the door — none refuted
- gemini: ANN is backend swap, not algorithmic divergence — concurred

**UAG passed.**

### Topic 4 — A-MEM bidirectional update deferral trigger

5 Round 1 proposals → significant Round 2 convergence:
- arch-reviewer: 3-AND adopting minimal-change's stale-delivery + corpus ≥500 + avg cluster >5
- minimal-change: corpus ≥1k + ≥5 supersession-within-7-days/30d window
- challenger: stale-retrieval count (≥5 superseded-within-14-days/30d) + corpus ≥500; corpus number flagged for operator calibration
- codex: corpus ≥1k + (5+ stale-fact cases OR 15% zero-fact searches) + failure pattern = updates/contradictions
- gemini: corpus ≥1k + ≥5 superseded-within-7-days/30d (matches minimal-change)

**Convergent shape, divergent specifics.** All 5 use "corpus floor + audit-log supersession signal." Differences:
- corpus floor: 500 (arch-reviewer, challenger) vs 1k (minimal-change, codex, gemini)
- supersession window: 7d (minimal-change, gemini) vs 14d (challenger)
- additional precondition: avg cluster > 5 (arch-reviewer); 15% zero-fact searches alternative (codex)

**TL judgment**: shape converged sufficiently to score `converged`. Specific numbers flagged for operator calibration when filing the A-MEM BL.

**Integration constraint**: audit table schema must log `returned_fact_ids` per search call (and link to subsequent supersession events on those fact IDs). This is a v0.0.1 instrumentation design requirement, derived from Topic 4 trigger needs.

---

## Round 2 closure assessment

| Topic | Status | Action |
|---|---|---|
| 1 | 4-of-5 converged; gemini outlier | TL autonomous decide (cite majority + arch-reviewer's independence argument + minimal-change's nominal-typing argument); note gemini dissent in conclusion |
| 2 | 4-of-5 converged; codex outlier | TL autonomous decide (cite minimal-change's chicken-and-egg argument); note codex dissent |
| 3 | 5-of-5 UAG passed | converged |
| 4 | shape converged; specifics flagged | converged (specifics → operator calibration in BL) |

No `revisit` items. No `deferred` items. **Sweep is empty.** Proceed
to Conclusion (Step 8) + post-conclusion Doodlestein (Step 9).

Required reading for any future Round (none anticipated):
all `round-02/<agent-name>.md` files.
