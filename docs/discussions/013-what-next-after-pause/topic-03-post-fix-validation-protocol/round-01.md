---
round: 1
date: 2026-04-16
score: converged
---

# Round 1

## Discussion

**Round 1 (independent research):**
- Architect: minimal 3-session validation. Pre-fix baseline (5 queries), post-fix regression check, then 3 real AE sessions. Pass: 1 confirmed useful retrieval + 1 Dreaming promotion in 2 weeks.
- Codex-proxy: elaborate 4-week protocol with counterfactual sessions, memory lifecycle states, automated metrics. 6 core metrics with specific thresholds.
- Code-researcher: MCP permissions blocker — `mcp__mengdie__*` tools NOT in settings.json allow list. Every invocation requires per-use confirmation. 31 recall events in DB prove MCP IS being used but with friction.

**Round 2 (convergence):**
- Architect confirmed MCP permissions is the actual blocker — must be fixed before any validation protocol.
- Codex-proxy agreed to strip protocol to MVP minimum: 1-2 weeks, manual scorecard, 5 benchmark prompts, 1-2 counterfactual paired runs.
- All agreed: circular validation (Mengdie tested on itself) must be broken by using real non-Mengdie project sessions.
- Code-researcher clarified: MCP IS being invoked (31 recall events) so it's not completely blocked, but per-use confirmation creates friction that contaminates adoption metrics.

**Validation protocol agreed:**

Prerequisites (before validation starts):
1. Add `mcp__mengdie__memory_search`, `mcp__mengdie__memory_ingest`, `mcp__mengdie__memory_invalidate` to settings.json allow list
2. Apply Dreaming threshold fix (Topic 1) and FTS5 tokenization fix (Topic 2)
3. Run `mengdie dream` to trigger first promotion pass (11 entries should promote)

Validation (2 weeks):
- 5 benchmark prompts (human-written, NOT from Mengdie analysis)
- Manual scorecard per search: date, project, query, top-3 useful (0-3), influenced output (no/cited/changed)
- 1-2 counterfactual paired runs (same task with/without memory)
- Target: ≥5 real-project sessions, ≥10 total memory_search calls

Pass criteria:
- ≥60% of searches return useful top-3 results
- ≥50% of useful searches influenced work output
- No clearly harmful/misleading retrievals
- ≥1 counterfactual shows memory run noticeably better

## Outcome
- Score: converged
- Decision: Fix MCP permissions → apply code fixes → run Dreaming → 2-week forced-use validation with manual scorecard + counterfactual
- Reversibility: HIGH (process, not code — protocol can be adjusted mid-validation)
