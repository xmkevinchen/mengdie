---
agent: minimal-change-engineer
verdict: REVISE
timestamp: 2026-04-28T16:40:07Z
---

# Minimal-change-engineer framing review verdict

Verdict: **REVISE**

The framing legitimizes forward-speculation that v0.0.1 should refuse. v0.0.1's contract from 028 Topic 4 is "ship the audit table so supersession-rate becomes computable when A-MEM trigger fires." 3 of the 5 topics fail that test.

## Topics to KEEP (2)

**Topic 1 (FTS-fallback coverage)** — keep, but reframe as binary, not a design space. The decision is: hook at `Db::memory_search` (hybrid only) or hook at `mcp_tools.rs` handler (covers fallback too). Pick one with rationale tied to supersession-rate semantics. Don't need 5 reviewers' worth of disagreement.

**Topic 4 (audit-write failure mode)** — keep. Genuine semantic choice (best-effort + warn vs hard error vs transactional) with downstream contract implication (probabilistic vs strict signal). The `record_recall` precedent at db.rs:259 is probably the right answer, but worth confirming once.

## Topics to KILL by YAGNI (3)

**Topic 2 (ON DELETE policy + PRAGMA foreign_keys)** — refuse. PRAGMA foreign_keys is OFF project-wide; FK clauses are doc-only today. Designing `ON DELETE` semantics for a hypothetical future flip is exactly the forward-speculation Topic 3 calls out. Decision: **no FK clause, period**. Schema comment notes the convention. If PRAGMA ever gets flipped on, that's a separate BL with its own trigger.

**Topic 3 (caller_kind column)** — refuse. Archaeologist found zero internal callers. The framing literally asks "is this YAGNI?" — answer is yes, drop the column. Adding a column later is a cheap v(N+1) migration. No discussion needed.

**Topic 5 (v0.0.1 read path / `mengdie audit-stats` CLI)** — refuse hard. The framing itself states "no v0.0.1 caller, A-MEM deferred 6+ months" then proposes building a CLI subcommand "so it isn't write-only infrastructure." That's textbook speculative-feature anti-pattern. The A-MEM trigger IS the read trigger. Ship write-only.

## Recommended revision

Reframe to **2 topics**, with a "Decided pre-discussion (YAGNI)" section listing the 3 killed items so the rationale isn't lost:

> **Decided pre-discussion (YAGNI, no Sweep needed):**
> - No FK clause on `fact_id` (PRAGMA off; document convention only)
> - No `caller_kind` column (zero internal callers; v(N+1) migration cheap)
> - No v0.0.1 read path (A-MEM trigger is the read trigger; write-only is fine)
>
> **For Sweep:**
> 1. Hook placement: `Db::memory_search` vs `mcp_tools.rs` handler
> 2. Audit-write failure mode: best-effort+warn vs hard-error vs transactional

This shrinks Sweep from "5 disagreements to converge" to "2 genuine binary choices" and explicitly refuses 3 forward-speculation traps. Net win for the v0.0.1 ship discipline 028 conclusion mandated.
