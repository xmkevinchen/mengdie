---
agent: minimal-change-engineer
review_angle: over-complication / scope creep
verdict_state: REVISE
timestamp: 2026-05-06T01:22:51Z
---

# minimal-change-engineer verdict

**REVISE**: framing manufactures decision points already resolved elsewhere and treats one underlying decision as two | suggested edit: collapse 5 topics → 2 ("AE↔mengdie ingest contract" + "loop-closure signal"); record the other three as already-decided defaults, not open questions.

## Specific findings

1. **Topic 4 (ingest source boundary) has an already-recorded default.** CLAUDE.md Project Status (2026-04-27 strategic reframe) states: "mengdie = AE 的大脑 ... mengdie receives AE-distilled propositional facts as ingest input." The boundary is AE-only for v0.0.1. This topic doesn't survive Round 1 — it's a confirm-the-default ack, not a discussion. Move to "Out of scope — already decided" with one-line citation.

2. **Topic 3 (cross-project default scope) was decided in prior discussions.** CLAUDE.md Key Design Decisions §5: "Global storage, per-project default search — avoid migration cost when adding cross-project later." The v0.0.1 rebuild framing doesn't claim to reopen this. Same treatment as Topic 4: move to out-of-scope with citation, unless the framing explicitly says "rebuild reopens this and here's why."

3. **Topic 2 (reflection trigger) has an obvious Round-1 default.** Five options listed (cron / salience / composite / debounced / on-demand), but salience/composite/debounced require runtime data mengdie doesn't have yet. For a fresh v0.0.1 rebuild the only defensible defaults are on-demand or cron. This is a 1-round pick-the-default item, not a 5-option deliberation. Either narrow the option set in framing ("cron vs on-demand for v0.0.1, others deferred until ingest volume data exists") or drop it to a BL with trigger.

4. **Topics 1 and 4 are variants of the same decision.** Both define the AE↔mengdie interface — Topic 1 is the *mechanism* (push/pull), Topic 4 is the *content boundary* (AE-only vs broader). Once you fix "AE-only for v0.0.1" (per CLAUDE.md), the mechanism question simplifies because the producer is one well-defined system. Combine into a single "AE↔mengdie ingest contract" topic.

5. **Reference Material has non-load-bearing entries.** "Discussions 025 and 026 ... inform but do not gate this one" — if they don't gate, they shouldn't be in the framing's reference list; that's exactly the context-loading the prompt warned about. Drop them; if they become relevant a topic body can cite them.

6. **Karpathy load-bearing test:** the framing's lines 22–26 ("These questions are not blocking §1–§5 ... they are blocking concrete implementation choices ...") are scaffolding, not decisions. Could compress to one sentence, but minor.

## Suggested minimal framing

- **Topic A**: AE↔mengdie ingest contract (push vs pull mechanism; AE-only boundary already locked per CLAUDE.md, just confirming)
- **Topic B**: Loop-closure signal — minimum instrumentation that proves mengdie delivers value
- **Acknowledged defaults** (no discussion needed, cited inline):
  - Per-project default scope (Decision §5)
  - AE-only ingest source (2026-04-27 reframe)
  - Reflection trigger = on-demand or cron for v0.0.1, advanced triggers deferred to BL with trigger condition

Net: 5 topics → 2 active + 3 acknowledged defaults. Less machinery, same problem coverage.
