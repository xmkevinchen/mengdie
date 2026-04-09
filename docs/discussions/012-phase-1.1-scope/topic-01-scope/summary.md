---
id: "01"
title: "What goes into Phase 1.1?"
status: pending
current_round: 1
created: 2026-04-09
decision: ""
rationale: ""
reversibility: ""
---

# Topic: What goes into Phase 1.1?

## Current Status
Need to triage 21 backlog items + AE PRD Phase C into: do now vs defer.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
Phase 1.0 (MVP) built the core: MCP server, hybrid search, ingestion, contradiction detection, dreaming. Plan 002 closed the knowledge loop (AE skills read/write to Mengdie). During validation, 5 analyses found 21 issues (backlog 004). Some are bugs, some are improvements, some are premature.

Key inputs:
- `docs/backlog/004-analyze-findings.md` — 21 items, 5 fixed, 16 deferred with triggers
- `agentic-engineering-mengdie/docs/prd/mengdie-integration.md` — Phase C (wire ae:plan, ae:review, ae:retrospect, ae:think)
- Analysis 011 findings: source_type/knowledge_type should be enums, tool descriptions need improvement
- Discuss 008 conclusion: 4 fixes, 2 already done, 2 remaining (description improvements)

## Constraints
- Single developer (Kai), limited time — scope must be achievable in days, not weeks
- Phase 1.1 should be a consolidation release, not a feature release
- Mengdie is a tool for Kai's own workflow — ROI is personal productivity, not product-market fit
- Don't fix what isn't broken at current scale (~15 memories, 1 project)

## Key Questions
- Which backlog items have fired their trigger conditions already?
- Which items improve daily workflow NOW vs improve at scale LATER?
- Should AE PRD Phase C (remaining skill wiring) be in Phase 1.1 or deferred?
- Is there a coherent theme that makes Phase 1.1 a meaningful release, not just a bug-fix grab bag?
