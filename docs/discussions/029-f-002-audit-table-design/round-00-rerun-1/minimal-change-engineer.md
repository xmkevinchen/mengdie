---
agent: minimal-change-engineer
verdict: APPROVED
timestamp: 2026-04-28T16:46:55Z
---

# Minimal-change-engineer framing review verdict (rerun-1)

VERDICT: APPROVED

Reviewed only the rewritten framing.md as instructed. Scope is now minimal.

**3 pre-discussion decisions — all correctly resolved, rationale survives re-litigation:**

1. **No ON DELETE clause** — PRAGMA foreign_keys OFF is verified at db.rs:80-119; ON DELETE semantics for a hypothetical PRAGMA flip is forward-speculation. Default NO ACTION + matching memory_synthesis_links convention is the right call. The "if PRAGMA flips, audit db.rs:636 rename_project DELETE path first" pointer is a clean trigger.

2. **No caller_kind column** — Two operator call sites (mcp_tools.rs:211, cli.rs:609), zero internal callers. One-column ALTER TABLE later with unambiguous backfill rule ('operator') is genuinely cheap. Pure YAGNI elimination.

3. **No v0.0.1 read path** — The A-MEM trigger IS the read consumer, and 028 explicitly defers it. "So the table isn't write-only" is the speculative-feature anti-pattern verbatim. Schema-stable, additive later.

**2 open topics — both genuinely necessary:**

**Topic 1 (hook placement)** — Cannot be eliminated. Binary choice with observable signal-correctness consequence: FTS-fallback path coverage during embedding outages. Wave 2 BL-009/BL-010 consolidation is correctly noted as the future fix; framing acknowledges the hook may move but schema does not. Keep.

**Topic 2 (failure-mode contract)** — Borderline-collapsible but defensible. The framing itself observes that 028's count-threshold trigger (≥5/30d) and the record_recall precedent both lean best-effort + warn, and constraints already mandate METRIC_AUDIT_WRITE_FAILURES. If Round 1 resolves the algorithm-level signal-completeness question as "tolerates probabilistic loss," this collapses to best-effort. However: the algorithm-level research question is real (threshold-wording inference is not the same as algorithm-level requirement), and the three options have differing latency profiles even when correctness collapses. Keep.

No scope creep detected. No further reduction available without losing legitimate decision content. Ready for Sweep.
