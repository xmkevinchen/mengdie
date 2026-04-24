---
id: "022"
title: "Synthesis provenance — pick fix option(s) from BL-synthesis-provenance"
status: concluded
created: 2026-04-23
pipeline:
  analyze: skipped
  discuss: done
  plan: done
  work: done
plan: "docs/plans/017-synthesis-cluster-hardening.md"
tags: [v0.8.0, synthesis, provenance, fidelity, search-ranking, mini-discuss]
---

# Synthesis Provenance — Fix Option Selection

Mini-discussion scoped per discussion 021 Next Step 7: pick which of the
4 fix directions in `BL-synthesis-provenance` should ship before planning
the v0.8.0 synthesis cluster. The 4 options are already enumerated in the
BL body; the job here is to select (one or a combination) and expose any
hidden dependency with BL-synthesis-dedup-key.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Which fix option(s) ship in the v0.8.0 synthesis plan | [topic-01-option-selection/](topic-01-option-selection/) | converged | Ship Option 1 (audit) + Option 4 (surface source_type). Defer 2+3. Reject 5 on axis discipline. |

## Documents
- [Framing](framing.md)
- [Conclusion](conclusion.md)
- [Source BL](../../../.ae/backlog/v0.8.0/BL-synthesis-provenance.md) *(local-only per project convention)*

## Origin Context

Discussion 021 Topic 1 split the decay cluster into Plan A (shipped as
plan 015) + Plan B (shipped as plan 016) but left the synthesis cluster
for its own planning cycle. Discussion 021 Next Step 7 specified that
BL-synthesis-provenance needs option selection before `/ae:plan`, because
the BL body enumerates 4 fix directions without committing to one. This
mini-discussion answers: which option(s), why, and how much does that
decision impact the adjacent BL-synthesis-dedup-key.
