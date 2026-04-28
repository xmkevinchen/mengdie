---
round: 1
date: 2026-04-28
team: 028-council
agents_reporting: [architecture-reviewer, minimal-change-engineer, challenger, codex-proxy, gemini-proxy]
---

# Round 1 — TL Synthesis (orientation only; per-agent files are primary)

This file is **orientation**, not a replacement for the per-agent
`round-01/<name>.md` files. Round 2 agents are required to read the
per-agent files directly and cite by file:line.

## Pruned

Pruned: nothing; all 5 agents advanced under the rerun-1-approved
framing. No off-topic exploration to pull back; no tangents to
dismiss.

## Of-framing disposition

Of-framing challenges raised this round: **none**. No agent
challenged the framing's scope, the 4-topic structure, the in-scope
resolution types (commit / reject permanently / defer with trigger),
or the "Analyze-phase inputs (not closed)" categorization. The Round
0 rerun's framing fixes held — no agent re-surfaced the prior REVISE
concerns (anchoring, mechanism assumption, missing reject option,
Reflector trait misclassification).

## Verification artifact

Per-claim verification status:

| Claim | Source | Status |
|---|---|---|
| "search.rs:80 defines `impl Db { fn memory_search() }`" | arch-reviewer round-01.md citing src code | **verified** (code reference) |
| "mcp_tools.rs:306-331 reimplements ingest pipeline inline" | round-01 references analysis.md | **verified** (carried from analyze archaeologist empirical) |
| "ANN-based clustering does not constitute a 2nd reflection strategy" | arch-reviewer + minimal-change-engineer + challenger Round 1 | **unvalidated as design claim** — relies on architectural definition of "strategy" not yet formalized |
| "AE artifact creation-time and ingestion-time agree within seconds" | codex Round 1 + topic-02 framing | **unvalidated empirically** — no production data in synthesis; operator domain knowledge required |
| "MCP protocol does not currently support ACK feedback" | challenger Round 1 | **verified** (rmcp current contract for `memory_search` returns results, no use-acknowledgment) |
| Topic 4 trigger numerics (corpus 500 / 1k / 5k / 10k thresholds) | each agent | **unvalidated** — none cite operator-set scale targets; all are heuristics |
| "Reflector trait still YAGNI even with sqlite-vec succeeding" | 5-of-5 agents | converging position; **unvalidated formally** until Round 2 cross-examines architectural-strategy definition |

Pattern: structural claims about current code are well-verified;
trigger numerics and architectural-definition claims are unvalidated
heuristics requiring Round 2 stress test.

## Frame-challenge disappearance self-check

Round 0 (rerun-1) generated 6 of-framing markers via REVISE feedback:
A (shape anchoring), B (trait verdict closure), C (missing reject
option), D (mechanism assumption), E + F (item 4 wording).

Round 1 outputs:
- A: ✓ no agent claimed shape was settled — arch-reviewer + minimal
  flipped Storage from CONDITIONAL ACCEPT to NO; codex flipped to
  DEFER. Shape genuinely re-opened.
- B: ✓ all agents engaged with Reflector trait re-open under topic 3
  per the framing's permission. None of the 5 took the analyze-phase
  CONDITIONAL ACCEPT as binding.
- C: ✓ "reject permanently" used substantively — Topic 2 has 2
  explicit REJECT votes (arch-reviewer, minimal-change) and 1
  reject-until-evidence (challenger).
- D: ✓ mechanism opened — 4 of 5 agents recommended free functions
  over `&Db` instead of trait. Trait is no longer the assumed answer.
- E + F: ✓ Topic 4 treated as concrete A-MEM trigger definition, not
  abstract presumption. All 5 gave specific trigger conditions.

No silent disappearance of Round 0 challenges. All 6 markers
addressed substantively.

---

## Position summary by topic (orientation)

### Topic 1 — Storage abstraction timing + mechanism

| Agent | Storage trait | Search-split | Mechanism |
|---|---|---|---|
| architecture-reviewer | NO | YES (alongside ingest defect fix) | free functions over `&Db` |
| minimal-change-engineer | NO | YES (free cleanup) | free functions over `&Db` |
| challenger | NO (default) | YES (open) | free functions; trait only after 2nd impl named |
| codex-proxy | DEFER (1 impl rule) | YES (separate justification) | concrete internally |
| gemini-proxy | CONDITIONAL ACCEPT | YES (earned by refactor) | trait |

**Convergence**: 4-of-5 NO trait, 5-of-5 YES search-split. **gemini
outlier on trait** — Round 2 disagreement to resolve.

### Topic 2 — Bi-temporal `event_time` column

| Agent | Position |
|---|---|
| architecture-reviewer | REJECT permanently; alternative = optional `valid_from` override on memory_ingest for bulk import |
| minimal-change-engineer | REJECT permanently; standard re-open-via-new-discussion |
| challenger | Maintain reject until operator names concrete case |
| codex-proxy | DEFER with trigger: "first artifact where creation-time differs from decision-time by >60s in production" |
| gemini-proxy | Agnostic; falsifiable test determines |

**Convergence direction**: 3-of-5 reject; 1 defer-with-specific-
trigger; 1 agnostic. **Reject-permanently vs defer-with-trigger is
the operative disagreement.**

### Topic 3 — Reflection module collapse + Reflector trait

| Agent | Consolidation | Reflector trait |
|---|---|---|
| architecture-reviewer | defer until sqlite-vec spike | NO (ANN swap ≠ 2nd strategy) |
| minimal-change-engineer | defer (cosmetic; may delete entirely) | NO (same reasoning) |
| challenger | defer | NO (demand runtime call site selecting strategies) |
| codex-proxy | defer | NO (only 1 strategy) |
| gemini-proxy | defer (may disappear) | (not directly addressed) |

**5-of-5 defer consolidation. 4-of-5 explicit Reflector NO; gemini
silent.** UAG candidate (likely passes if gemini affirms).

### Topic 4 — A-MEM bidirectional update deferral trigger

| Agent | Trigger structure | Specific conditions |
|---|---|---|
| architecture-reviewer | 3-AND composite | top-3 score 30d-rolling avg < 0.35 (≥20 queries) + corpus > 500 + avg entity cluster > 5 |
| minimal-change-engineer | 3-AND composite | corpus ≥1k + ≥5 stale-retrieval/30d + 1 independent A-MEM replication paper |
| challenger | single-condition | precision-based if MCP-ACK in v0.0.1 contract; corpus-only fallback if not |
| codex-proxy | 4-AND composite | corpus ≥10k OR ≥1M tokens AND insufficient_context ≥15% on eval AND simpler retrieval tuning <5pp improvement AND offline ablation ≥8pp gain (+ feature flag) |
| gemini-proxy | composite | corpus > 5k + retrieval quality measurably degrading (measurement unspecified) |

**Topic 4 is the most fragmented**: composite vs single, corpus
thresholds spanning 500–10k (20× range), ACK protocol dependency
unresolved.

---

## Round 2 dispatch (TL determines)

Three substantive disagreements require Round 2 discussion + (where
unresolved) consensus verification:

**A. Storage abstraction mechanism — gemini vs others.** 4-of-5
recommend free functions, gemini holds CONDITIONAL ACCEPT trait.
gemini must engage the YAGNI argument on its merits or update.

**B. Topic 2 — REJECT permanently vs DEFER with trigger.** Operationally
similar (both fire on same evidence) but governance-different
(reject-and-reopen-via-new-discussion vs defer-with-trigger-fires-
automatically). Pick one.

**C. Topic 4 — trigger structure resolution.** 5 different specifics;
underlying issue is whether MCP `memory_search` contract should
include ACK feedback in v0.0.1. If ACK is in contract, retrieval-
quality measurement becomes tractable; if not, only corpus-size and
score-distribution are server-side measurable.

**Topic 3** — 5-of-5 defer; 4-of-5 explicit Reflector NO. Eligible
for UAG (Unanimous Agreement Gate) in Round 2 if gemini affirms.

Required reading for Round 2 (each agent must read all peer Round 1
files; cite by file:line):

- `round-01/architecture-reviewer.md`
- `round-01/minimal-change-engineer.md`
- `round-01/challenger.md`
- `round-01/codex-proxy.md`
- `round-01/gemini-proxy.md`

This synthesis is orientation; do NOT derive arguments from it.
