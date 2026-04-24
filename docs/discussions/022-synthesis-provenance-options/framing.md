---
id: "022"
stage: framing
created: 2026-04-23
round_0: skipped  # proportionality: BL body already enumerates 4 well-defined options; framing-review would be ceremony for a pick-from-list decision
round_0_notes: |
  Per user direction (proportionality signal from plan-014 + plan-015 + plan-016 reviews):
  skip Round 0 framing review. The BL enumerates 4 options with stable
  labels; the decision shape is pick-one-or-combination, not open-ended
  problem-framing. Spawn Round 1 directly with a 3-agent team.
  If Round 1 surfaces framing concerns, revise then.
---

# Framing — Synthesis Provenance Option Selection

## Problem Statement

`run_synthesis_pass` stores every LLM-generated synthesis row with
`source_type=synthesis` and `knowledge_type=factual`. The parser checks
structure (non-empty title/content, valid JSON) but does not verify that
the synthesis accurately reflects its source memories. A hallucinated
synthesis — inverted decision, dropped constraint, fabricated entity tag
— passes validation and lands in the DB as a "factual" row, where it:

- ranks in FTS + vector search alongside primary sources (conclusions,
  reviews, plans)
- can be injected as `ae:analyze` Round 0 prior context with no signal
  distinguishing it from a primary source

The BL identifies two sub-problems:
1. **Provenance visibility**: `SearchResultItem` now carries
   `source_type` (added in BL-007 review fixup), but `mengdie search` CLI
   output and MCP snippets don't visually distinguish syntheses yet.
2. **Fidelity detection**: no code path compares synthesized content
   against source memories; no way for an operator to tell which
   syntheses are accurate without manually reading each one.

The BL lists 4 candidate fix directions. They are partially orthogonal
— some can combine. This discussion picks the combination that ships in
v0.8.0.

## Options (verbatim from BL body, for reference)

1. **Flag-based audit subcommand**: `mengdie synthesis audit <syn-id>`
   prints synthesis content alongside its source memories. Operator
   eyeballs fidelity. Cheapest; manual.
2. **LLM-based verification**: second LLM pass scores fidelity (0-10),
   stored in new `memory_quality_score REAL` column. Expensive per-run
   cost; automates fidelity signal.
3. **Explicit lower trust**: downrank synthesis rows in search
   (`score *= 0.5` or similar) so primary conclusions win on ties.
   Cheapest; contentious — some operators will want syntheses to rank
   high when they are accurate.
4. **CLI prefix**: prefix `[SYN]` on titles in `mengdie search` /
   `mengdie list` output. Pure UX; zero algorithm change.

## Scope

In:
- Select which of options 1-4 ship in the v0.8.0 synthesis plan
  (can be one or a combination — options are partially orthogonal)
- Identify any hard dependency on `BL-synthesis-dedup-key` (the
  parallel-but-separate synthesis-cluster BL) — e.g., if option 2
  requires a schema migration, does that block or co-land with
  dedup-key's v5 migration?
- Surface what "ships" means operationally: is option 4 (CLI prefix)
  gated by option 1 (audit subcommand) being available to make the
  prefix actionable, or can it stand alone?

Out:
- Implementation details of any chosen option (that's the plan stage)
- Changes to `BL-synthesis-dedup-key`'s content or fix options (that BL
  is unambiguous at its core — replace content_hash as the dedup key —
  and only needs the schema-migration dimension surfaced here)
- Re-litigating the BL's trigger condition or failure mode description

## Reference Material

- `.ae/backlog/v0.8.0/BL-synthesis-provenance.md` — the source BL (local-only)
- `.ae/backlog/v0.8.0/BL-synthesis-dedup-key.md` — parallel BL; dep check
  is in-scope
- `src/core/dreaming.rs` `run_synthesis_pass` — the producer
- `src/core/search.rs` — where a downrank (option 3) would land
- `src/bin/cli.rs` — where an `audit` subcommand (option 1) or a
  `[SYN]` prefix (option 4) would land
- `docs/plans/010-dream-synthesis.md` — the plan that shipped the
  synthesis pass; has context on why `knowledge_type: factual` was
  chosen at ship time
- `docs/reviews/008-dream-synthesis.md` — the review that raised these
  concerns as BL candidates
