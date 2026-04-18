---
id: BL-synthesis-provenance
status: open
origin: BL-007 /ae:review (challenger #2, #7)
created: 2026-04-18
---

# Synthesis rows claim `knowledge_type: factual` with no fidelity check

## Finding

`run_synthesis_pass` stores each LLM synthesis as `source_type=synthesis`,
`knowledge_type=factual` (`src/core/dreaming.rs` in the `NewMemory`
construction). The parser validates structure (non-empty title/content,
valid JSON) but does NOT verify that the synthesis accurately reflects
its source memories.

A hallucination — inverted decisions, dropped constraints, fabricated
entity tags — will pass all validations and land in the database as
a factual-tagged row. It will rank in FTS/vector search alongside
primary sources. `ae:analyze` Round 0 will inject it as prior context
with no signal distinguishing it from a `conclusion` row.

Two sub-problems:

### Provenance visibility

`SearchResultItem` now carries `source_type` (added in BL-007 review
fixup), so a machine consumer can filter. But `mengdie search` CLI
output and MCP snippets don't visually distinguish syntheses yet.

### Fidelity detection

No code path compares synthesized content against source memories.
No way for an operator to tell which syntheses are accurate without
manually reading each one against its link-table sources.

## Trigger

Fires when:
- First `mengdie dream --synthesize` run on real production data (the
  plan's AC5 writeback also creates a cluster-quality judgment — use
  that as the signal), OR
- User reports bad ae:analyze Round 0 suggestions traceable to
  synthesis rows (requires the source_type field in search output,
  already added), OR
- Corpus exceeds ~50 synthesis rows (ad-hoc auditability breaks down).

## Fix directions

1. **Flag-based detection**: add a `--audit` subcommand:
   `mengdie synthesis audit <syn-id>` — prints the synthesis content
   alongside its source memories, letting the operator eyeball fidelity.
2. **LLM-based verification**: run a second LLM pass to score fidelity
   (0-10), store in a new `memory_quality_score REAL` column. Expensive
   — only justify if manual audit proves syntheses are unreliable.
3. **Explicit lower trust**: downrank synthesis rows in search
   (`score *= 0.5` or similar) so primary conclusions win when both
   match a query. Cheapest; also most likely to be contentious.
4. **Surface syntheses differently in CLI**: prefix `[SYN]` on titles
   in `mengdie search` output, distinguish in `mengdie list` format.

Pick whichever matches operator workflow once real-data evidence exists.

## Why not fixed in BL-007

The MVP ships the synthesis pass with full DB/CLI plumbing. Fidelity
detection is a second-order concern — we need to see actual bad
syntheses before over-engineering. BL-007 review fixup already added
`source_type` to `SearchResultItem` as the minimum affordance for
downstream consumers to discriminate.
