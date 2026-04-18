---
id: "03"
title: "CLAUDE.md drift — Project Status, rmcp version, rusqlite features, VectorStore trait"
status: converged
current_round: 1
created: 2026-04-18
decision: "Update CLAUDE.md in the same cleanup commit (or split into a separate commit for traceability). Scope: (a) extend Completed plan cycles list with plans 005/007/008/009/010; (b) add llm.rs, clustering.rs, synthesis.rs, config.rs to Project Structure; (c) update Architecture bullet on Dreaming to include LLM-driven synthesis via claude CLI; (d) rewrite 'Next step: 2-week forced-use scorecard' to reflect Phase 2 in progress; (e) prune 'Deferred discussions' list → pointer to docs/backlog/. Do NOT add function/type names (SourceType::Synthesis, run_synthesis_pass, etc.) — CLAUDE.md is architectural orientation, not API reference."
rationale: "Archaeologist verified all 4 original audit items are already fixed. 4 new drift items accumulated from plans 005/007/008/009/010. Architect: layer-level descriptions are correct granularity; identifier-level names are drift bait. Challenger agreed CLAUDE.md fix is mechanical — do it in the same commit, don't re-discuss."
reversibility: "high — doc-only edits, all revertable via git"
reversibility_basis: "No code changes; the new text can be edited or reverted freely as the project evolves."
---

# Topic: CLAUDE.md drift — tech stack and project status stale

## Current Status

Original 4 audit items already fixed (archaeologist confirmed). 4 new drift items need addressing this pass. Round 1 converged on mechanical in-commit fix with architect's scope boundaries.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | Update Completed plan cycles + Project Structure + Architecture bullet + Next-step rewrite + prune Deferred discussions; describe layers, not identifiers |

## Context

CLAUDE.md is the first thing any agent (Claude Code or subagent) reads on session start. Drift there propagates into every `/ae:analyze`, `/ae:discuss`, `/ae:plan` response. 4 specific items from the audit:

| Line (old) | Says | Reality |
|---|---|---|
| 35 | `rusqlite features include "fts5"` | Cargo.toml has `["bundled", "load_extension"]` only |
| 37 | `rmcp v0.16` | Cargo.toml is `v1.3` (line 91 also says 1.3 — contradiction within the file) |
| 36 | `sqlite-vec (optional, behind VectorStore trait)` | No VectorStore trait exists, no sqlite-vec in Cargo.toml |
| 158 | "Phase 1 MVP — plan reviewed, ready for implementation" | 5 completed plan cycles (including BL-007 just shipped) |

## Constraints

- CLAUDE.md is user-authored ground truth — cleanup should preserve intent, not rewrite style.
- Some drift is ambiguous: line 36 said VectorStore trait is "optional, behind the interface" — the intent may have been "planned, not built" rather than "exists today". Fix should preserve the forward-looking intent if that's what was meant.
- Updates to CLAUDE.md should use the existing heading structure; don't reorganize.

## Key Questions

1. Re-verify current CLAUDE.md state after the post-2026-04-16 edits. Which of the 4 items are still drifted? Are there new drift items from plans 007/009/010 that need to be reflected?
2. Is there now a need to document the new public surfaces (SourceType::Synthesis, run_synthesis_pass, cluster_memories, LlmProvider trait) in CLAUDE.md, or are they sufficiently documented in code + plans?
3. The "Deferred discussions" bullet list in CLAUDE.md (lines 160+) is stale — 006, 007, 009, 010, 011 have been deferred for 2 weeks with no triggers; their continued listing may be unhelpful clutter or valuable Phase 2 signposts.
