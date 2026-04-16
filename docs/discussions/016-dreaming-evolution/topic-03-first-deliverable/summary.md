---
id: "03"
title: "First Deliverable — What Ships First"
status: pending
current_round: 1
created: 2026-04-16
decision: ""
rationale: ""
reversibility: ""
---

# Topic: First Deliverable — What Ships First

## Current Status
Need to define the first concrete deliverable that proves the new direction works.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
The user's critique: "dreaming is just a concept, ingest is just an action, recall is just an action — no intelligence." The first deliverable must visibly add intelligence that wasn't there before.

Candidates from analysis:
- A: LLM provider + entity extraction at ingest → improved contradiction detection coverage
- B: LLM provider + RAG search → memory_query returns synthesized answers
- C: Decay + lint (no LLM) → Dreaming actually maintains knowledge health
- D: LLM provider + Dreaming compilation → dream pass synthesizes cluster summaries
- E: Knowledge graph schema + entity extraction → typed relationships between memories

## Constraints
- Must be demonstrably better than current state (not just infrastructure)
- Solo dev — scope for 1 plan cycle (4-8 steps)
- Should validate the LLM integration architecture for everything that follows
- User needs to feel "mengdie is thinking, not just storing"

## Key Questions
- Which candidate delivers the most visible intelligence with the least infrastructure?
- Should the first deliverable include LLM, or start with non-LLM improvements first?
- What's the acceptance test that proves "intelligence was added"?
