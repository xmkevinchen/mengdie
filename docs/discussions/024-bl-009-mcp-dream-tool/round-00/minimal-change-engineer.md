---
agent: minimal-change-engineer
attempt: 1
verdict: REVISE
timestamp: 2026-04-27T19:58:24Z
---

# minimal-change-engineer attempt 1: REVISE — framing over-built

Framing is bigger than problem requires. Core problem is 2 lines:

> dream synthesis shells out to `claude` CLI; when mengdie runs as an MCP server inside Claude, host Claude IS the LLM — design the MCP-native shape.

Current framing buries this under ~30 lines of subprocess mechanics, design pre-judgment, and constraint-as-problem.

**Specific issues**:

1. **Problem statement is 3 paragraphs (~30 lines), should be 3 lines.** Para 1 details current path mechanics — reviewers can read `dreaming.rs`. Para 2 ("not just a delivery-path question…responsibilities re-distribute") is solution-prejudgment masquerading as framing. Para 3 (memory_ingest cluster-hash bypass) is a constraint, not the problem.

2. **Cluster-hash + memory_ingest paragraph is premature complication for Round 1.** Leaks solution shape. Reviewers should be free to propose designs that don't go through memory_ingest at all. Move to Reference as "constraint to honor."

3. **Scope "in" list has 6 items but redundant.** Items 1/2/3 are one design question — collapse to "API shape and data flow for MCP-native synthesis." Item 4 duplicates. Item 6 is constraint, not scope. Reduce to 3 items.

4. **Reference count (3+4) is right; load-bearing entries over-narrated.** 1 line + path suffices instead of 3-4 lines of editorializing.

**Proposed minimal framing** included verbatim in original verdict (~30 lines vs 70). Drops solution-prejudgment, keeps every load-bearing fact, opens design space wider for Round 1.
