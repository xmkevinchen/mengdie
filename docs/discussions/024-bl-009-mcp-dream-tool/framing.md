---
id: "024"
stage: framing
created: 2026-04-27
round_0: overridden
round_0_override_reason: "User override after attempt 3. Attempt 3 was 4 APPROVED + 1 REVISE (gemini-proxy via oMLX fallback). Gemini's REVISE asked to fill mechanism trade-off (wrapper vs structural refactor) into framing, but that's exactly what Round 1 is supposed to decide. The other 4 reviewers explicitly rejected further changes: minimal-change said 'cutting more would start removing load-bearing context'; strategic said 'Reference constraint is now purely structural'; adversarial said 'all five round-0 corrections remain intact'; codex confirmed 'no mechanism bias'. Gemini's preference is reviewer-aesthetic divergence, not a structural framing defect. Per spec rerun-limit (3 attempts) and override-with-reason path. Same shape as discussion 023 attempt 3 escalation."
round_0_reviewers: [codex-proxy, gemini-proxy, doodlestein-strategic, doodlestein-adversarial, minimal-change-engineer]
round_0_notes: "Attempt 1: 1 APPROVED + 4 REVISE. Attempt 2: 4 APPROVED + 1 REVISE (strategic 1-sentence fix — Reference constraint still named `memory_ingest`, should state structural requirement only). Attempt 3: applied strategic's fix. Per-attempt verdicts in round-00/ and round-00/attempt-2/."
---

# Framing — BL-009: MCP Dream Tool

## Problem Statement

mengdie's dream synthesis (`mengdie dream --synthesize`) shells out to
the `claude` CLI per cluster. When mengdie runs as an MCP server inside
a Claude session, the host Claude IS the LLM. Two questions follow:

1. **Whether** this indirection is worth eliminating in the
   MCP-attached case.
2. **What** mengdie must enforce regardless of who runs synthesis, and
   what it can safely delegate.

If (1) answers "no" the work stops; if "yes" then (2) determines the
mechanism shape.

## Scope

In:
- The invariant boundary between mengdie and the synthesis caller
- The mechanism (or absence thereof) that follows from that boundary
- Coexistence with the existing `dream --synthesize` CLI path

Out:
- Synthesis prompt content (`src/core/synthesis.rs`, separate concern)
- Quality measurement (BL-audit-collection-discipline)
- Multi-LLM support (the `LlmProvider` trait stays as it is)
- Daemon mode (BL-010), RAG retrieval (BL-012)

## Reference Material

- BL-009 stub: `docs/backlog/005-phase2-roadmap.md:66-71`
- Current synthesis path: `src/core/dreaming.rs:399` (`run_synthesis_pass`),
  `src/core/synthesis.rs` (prompt builder), `src/core/llm.rs`
  (`LlmProvider` + `ClaudeCliProvider`)
- **Constraint to honor**: persisted synthesis rows MUST set
  `synthesis_cluster_hash` and link to source memories via
  `memory_synthesis_links`. v0.8.5
  BL-synthesis-cluster-hash-not-null-enforcement closes this at the
  DB layer. Whatever shape BL-009 takes, it must not write synthesis
  rows that bypass this constraint.

Optional (read if your design path needs them):
- MCP tool pattern: `src/core/mcp_tools.rs` (`memory_search`,
  `memory_ingest`, `memory_invalidate`)
- Clustering input: `src/core/clustering.rs` (BL-006)
- Phase 2 chain: BL-009 → BL-010 → BL-011 / BL-013 (don't foreclose)
