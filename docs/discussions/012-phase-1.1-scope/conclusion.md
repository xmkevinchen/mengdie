---
id: "012"
title: "Phase 1.1 Scope and Execution — Conclusion"
concluded: 2026-04-09
plan: ""
entities: [phase-1.1, scope, api-contract-enums, tool-descriptions, phase-c-skill-wiring, knowledge-capture-completeness]
---

# Phase 1.1 Scope and Execution — Conclusion

3 agents (architect, code-researcher, prioritization-analyst) across 2 rounds. All 3 topics converged.

---

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Phase 1.1 scope | 3 mengdie code items + 4 AE skill wiring items | Theme: "API contract correctness + knowledge capture completeness." Discuss 008 fixes already done. All backlog 004 triggers unfired. | high — items are independent, can drop any without breaking others |
| 2 | Execution approach | One plan, ae:work for Rust enum change, manual for prose/SKILL.md, mid-execution review gate | Cross-repo dependency: mengdie enums must land before AE skills pass valid values. Two natural review points. | high — can switch to ae:work or manual at any step |
| 3 | Acceptance criteria | 9 criteria, AC9 is workflow marker: ae:plan surfaces prior decision unprompted | Code correctness (AC1-3), integration completeness (AC4-8), workflow proof (AC9). AC9 is the "part of workflow" signal. | medium — AC9 is qualitative, may need refinement |

---

## Phase 1.1 Scope Detail

**Theme**: API contract correctness + knowledge capture completeness

### Mengdie repo (3 items)

| Step | Item | Effort |
|------|------|--------|
| 0 | Phase B gate check: verify discuss-ingested memories retrievable in fresh session | 20 min |
| 1 | source_type/knowledge_type → Rust enums with Deserialize + JsonSchema | 2-3h |
| 2 | memory_search description → 3-4 sentences (purpose, 200-char snippet limit, min_score guidance) | 30 min |
| 3 | Move workflow logic from memory_ingest description to ServerHandler::get_info() instructions | 30 min |

### AE repo — PRD Phase C (4 skills)

| Step | Skill | Read step | Write step | Effort |
|------|-------|-----------|------------|--------|
| 4 | ae:think | Step 1.5 Prior Context | None (read-only per PRD) | 30 min |
| 5 | ae:plan | Step 1.5 Prior Context | After Doodlestein, gate on status:reviewed | 1.5h |
| 6 | ae:review | Before Step 1 Create Team | After Output, before PR prompt | 1.5h |
| 7 | ae:retrospect | Step 0.5 Prior Context | After Step 4 Output, skip in --compare mode | 1.5h |

**Mid-execution review gate**: after Steps 1-3 (mengdie changes), before Steps 4-7 (AE changes).

### Explicitly OUT

- All 16 backlog 004 deferred items (no triggers fired at ~15 memories, 1 project)
- memory_get tool (200-char snippets sufficient for current use cases; trigger: agent can't resolve stale conflict from snippet)
- Discuss 008 4 fixes (already implemented in prior session)

---

## Acceptance Criteria

| AC | Criterion | Verification |
|----|-----------|-------------|
| 1 | memory_ingest rejects unknown source_type/knowledge_type with error | cargo test |
| 2 | memory_search description mentions 200-char snippet limit, 3+ sentences | Read mcp_tools.rs |
| 3 | memory_ingest description has no branching workflow logic; server instructions explain resolution pattern | Read mcp_tools.rs |
| 4 | ae:plan surfaces prior context AND writes knowledge | Run once, check mengdie list |
| 5 | ae:review surfaces prior context AND writes knowledge | Run once, check mengdie list |
| 6 | ae:retrospect surfaces prior context AND writes knowledge | Run once, check mengdie list |
| 7 | ae:think surfaces prior context | Run once, check output |
| 8 | All 4 skills work normally with Mengdie MCP disconnected | Disconnect MCP, run skill |
| 9 | ae:plan on a real feature surfaces at least 1 prior decision from Mengdie unprompted | Fresh session, real feature |

---

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| TL | Moderator | Claude | Start |
| architect | System architecture, scope design | Claude | Start |
| code-researcher | Code ground truth, effort estimation | Claude | Start |
| codex-proxy | Cross-family prioritization | Codex | Start (hit quota, shut down) |
| prioritization-analyst | Prioritization patterns (Claude fallback) | Claude Sonnet | Round 1 (replaced codex-proxy) |

## Process Metadata

- Discussion rounds: 2
- Topics: 3 (3 converged)
- Autonomous decisions: 3
- User escalations: 0
- Doodlestein: skipped (prioritization exercise, not architecture decision)
- Deferred resolved in Sweep: 0

## Next Steps

→ `/ae:plan` for Phase 1.1 implementation plan based on these decisions
