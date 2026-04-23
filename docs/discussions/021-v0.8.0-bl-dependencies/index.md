---
id: "021"
title: "v0.8.0 — dependency map for remaining 7 open BLs"
status: active
created: 2026-04-23
pipeline:
  analyze: done
  discuss: done
  plan: done
  work: pending
plan: "docs/plans/015-decay-operator-surface-hardening.md"
tags: [v0.8.0, sprint-planning, dependency-analysis, decay, synthesis]
---

# v0.8.0 remaining BLs — dependency map

Tactical analysis: of the 7 open BLs in sprint v0.8.0 (after plan 014
closed 006-ci-runner-env-cleanup + BL-ci-full-clippy-test), which share
code surface, which block which, and what's the cheapest execution
ordering.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Bundle boundary for the decay-cluster plan | [topic-01-bundle-boundary/](topic-01-bundle-boundary/) | converged | Split 2+1 — Plan A (json-schema + verify-decay), Plan B (ops-doc-polish) |
| 2 | Fate of the 2 defer-until-trigger items | [topic-02-defer-trigger-items/](topic-02-defer-trigger-items/) | converged | /ae:roadmap remove both + gate-text update in one commit |
| 3 | Which hardening actions ship in v0.8.0 | [topic-03-hardening-scope/](topic-03-hardening-scope/) | converged | Ship actions 1+2+4 (action 3 already done; action 5 defers to BL-010 sprint) |
| 4 | Sprint-commitment policy for unresolved pre-conditions | [topic-04-sprint-commitment-policy/](topic-04-sprint-commitment-policy/) | converged-with-dissent | File upstream AE BL + one-line mengdie CLAUDE.md checklist (minimal-change dissent preserved) |

## Documents
- [Framing](framing.md)
- [Analysis](analysis.md)
- [Conclusion](conclusion.md)

## Origin Context

After plan 014 (v0.8.0 item 1/9 done), user ran `/ae:next` and got 7
candidate BLs with no explicit dependencies recorded on them. Asked
"any dependency among these BLs?" to inform next-plan scoping (bundle
vs. pick-smallest vs. critical-path).

TL ran analysis solo — narrow scope, evidence gathered from reading
each BL body directly. No agent team spawned; cross-family skipped
(no design judgment needed; pure dependency graph extraction).
