---
id: "02"
title: "Phase Sequencing and Dependency Chain"
status: pending
current_round: 1
created: 2026-04-16
decision: ""
rationale: ""
reversibility: ""
---

# Topic: Phase Sequencing and Dependency Chain

## Current Status
Five capabilities identified, need sequencing: LLM provider, entity extraction, knowledge graph, RAG search, Dreaming intelligence.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
Can't build everything at once. Need to identify the dependency chain and define phases that each deliver standalone value. User is solo developer — each phase must be small enough to ship in 1-2 plan cycles.

From analysis: LLM provider is foundation. Entity extraction enables knowledge graph. Knowledge graph enables better contradiction detection and RAG. RAG and Dreaming intelligence are parallel once graph exists.

Field landscape: Engram (closest sibling) has decay but no LLM. CortexGraph has forgetting curves. LLM Wiki v2 has full lifecycle. Karpathy pattern has compile+lint.

## Constraints
- Solo developer — phases must be achievable in days, not weeks
- Each phase must deliver measurable improvement over current state
- LLM costs accumulate — phases that add API calls need cost-awareness
- Existing 194 memories need migration path, not a rewrite
- MCP tool API (memory_search, memory_ingest, memory_invalidate) is the public contract — backward compatibility matters

## Key Questions
- What's the dependency graph between the 5 capabilities?
- How many phases? What's in each?
- What's the minimum viable first phase that proves the LLM integration works?
- Should decay/lint (no LLM needed) be Phase 2.0 while LLM integration is Phase 2.1? Or go straight to LLM?
