---
id: "01"
title: "Should mengdie add a memory_resolve_conflict MCP tool?"
status: pending
current_round: 1
created: 2026-04-05
decision: ""
rationale: ""
reversibility: ""
---

# Topic: Should mengdie add a memory_resolve_conflict MCP tool?

## Current Status
Analysis 008 identified that contradiction detection fires but has no resolution workflow. Conflicts are flagged in memory_ingest response but nothing happens next — no tool exists to confirm supersession.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
Mengdie's contradiction detection flags conflicts (EvolutionCandidate, RecentConflict) during ingest. The schema already has `valid_until` and `superseded_by` fields. But there's no MCP tool to act on detected conflicts — the user sees "conflicts detected" and can do nothing about it programmatically. The question is whether adding a `memory_resolve_conflict` tool is the right next step, or whether the resolution should happen differently (auto-resolve, batch cleanup via dreaming, manual CLI, etc.).

## Constraints
- MCP tool surface should stay minimal (currently 3 tools: search, ingest, invalidate)
- Resolution must set `valid_until` on old entry + `superseded_by` on new entry (schema already supports this)
- Must work for both human-initiated and AI-agent-initiated resolution
- `memory_invalidate` already exists — is a new tool redundant or complementary?

## Key Questions
- Is explicit conflict resolution needed at MVP scale (10-50 memories), or is it premature?
- Should resolution be a new MCP tool, or extend `memory_invalidate` with a `superseded_by` parameter?
- Who triggers resolution — the AI agent automatically, or the user explicitly?
- What happens to the old memory's recall_count and avg_relevance data?
