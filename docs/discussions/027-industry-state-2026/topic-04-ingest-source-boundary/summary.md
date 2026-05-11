---
id: "04"
title: "Ingest source boundary — extraction-discipline (not AE-files-only)"
type: ratify
prior_commitment: "CLAUDE.md Project Status (2026-04-27 strategic reframe) — mengdie = AE 的大脑; mengdie receives AE-distilled propositional facts as ingest input"
gates: topic-01-ingest-mechanism
status: converged
current_round: 2
created: 2026-05-05
decision: "Ratify the spirit of 'AE-only' — mengdie does not become a generic memex — interpreted as **extraction discipline**, NOT physical AE-files-only enforcement. Existing source_type enum is the typed marker. memory_ingest accepts text payloads from any caller IF the caller asserts AE-style structured extraction was applied. Latent bug fix: rename source_type 'unknown' → 'direct' (archaeologist round-01:269-275 latent rejection bug). Ad-hoc facts (e.g., debug-session conclusions outside AE pipeline) are valid via the direct source_type when the caller carries the extraction discipline."
rationale: "User decision after team split (3 extraction-discipline vs 3 AE-files-only). Extraction-discipline FOR (system-architect + challenger + ai-engineer R2): existing source_type enum sufficient as typed marker; AE-discipline is quality gate not physical-source restriction; archaeologist's latent-bug evidence (memory_ingest already accepts any text; non-AE rejected by trigger) confirms extraction-discipline is what the code already does. AE-files-only AGAINST (codex + gemini + minimal-change R2): tighten parser allowlist + clear MCP error; ~25 LoC bug fix with no schema; reopening trigger only. Both sides agreed: ratify spirit of AE-only; latent bug must be fixed. Both sides have evidence."
reversibility: medium
reversibility_basis: "Switching post-ship requires re-classification of memories ingested under one stance to match the other. Storage shape is identical between readings (source_type column exists in both); behavior at the ingest API boundary differs."
---

# Topic: Ingest source boundary — ratify AE-only

## Current Status
**CONVERGED Round 2 with user decision on split.** Extraction-discipline (not physical AE-files-only). source_type enum kept; rename `unknown` → `direct`. Ad-hoc facts valid via `direct` source_type.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 4 ratify-AE-only with variants (with/without fwd-compat), 1 ratify-with-fwd-compat (codex), 1 challenger-extraction-discipline reframe |
| 2 | split → user-decided | After R1 fact (memory_ingest accepts any text; AE-only is policy not enforcement; non-AE files rejected by v5 trigger), R2 updates: system-architect + ai-engineer absorbed challenger's extraction-discipline reframe; codex + gemini updated to NO fwd-compat; minimal-change reinforced AE-files-only. 3-vs-3 split escalated to user. User: extraction-discipline. |

## Reopening trigger (record in conclusion)
- ≥3 high-value facts per quarter that cannot be retrofitted into AE pipeline AND cannot reasonably be wrapped with the `direct` source_type

## Type: ratify
**Not an open from-scratch decision.** CLAUDE.md Project Status
(2026-04-27 reframe) already commits to AE-only ingest for v0.0.1:
"mengdie = AE 的大脑 ... AE plugin handles in-session LLM-driven
processing ... mengdie receives AE-distilled propositional facts as
ingest input." Round 1 is an evidence-check.

**Why ratify before topic 1 in dependency order:** if AE-only is
ratified, topic 1 (mechanism) operates in a single-producer design
space (simpler). If topic 4 revises and broadens the ingest source
set, topic 1 reopens with multi-producer constraints.

Acceptable outcomes:
- **Ratify** — confirm AE-only as v0.0.1 boundary; record what would
  trigger broadening later (post-v0.0.1).
- **Revise** — only with concrete evidence that AE-only is
  underserving the loop (e.g., a class of facts the operator
  consistently wants captured but AE doesn't produce).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
Blueprint §3.1 currently restricts mengdie inputs to AE pipeline
artifacts (plan / review / conclusion / retrospect / discussion).
This is part of mengdie's identity — the high signal-to-noise ratio
of structured pipeline outputs is what makes the niche unique
(analysis.md "Industry Practice Comparison" point 1).

The blueprint §8 question asks whether to permit broader sources
later: chat summaries, commit messages, issue/PR content, and what
discipline keeps mengdie from sliding into a generic memex.

The decision matters now because:
- The AE plugin is the only confirmed v0.0.1 source. If broader
  inputs are eventually permitted, the ingest API needs forward
  compatibility (typed source markers, per-source filters) baked in
  from v0.0.1, not retrofitted.
- "Generic memex" is exactly the failure mode that destroyed Quivr's
  OSS positioning and that NotebookLM works around with rigid
  notebook silos.
- Karpathy's LLMWiki philosophy (which this project quotes) is
  about high-quality propositional facts, not raw content firehose.

## Constraints
- §3.1 is currently locked (drafted into blueprint v0.2); this
  discussion can either confirm the lock or recommend amendment
- Storage / search infrastructure is source-agnostic — the boundary
  is conceptual, not technical
- AE plugin already does the LLM-mediated extraction work upstream;
  any new source needs equivalent extraction discipline or it
  pollutes the corpus
- Adding a source isn't free: each source needs its own ingest
  schema, provenance contract, contradiction-detection rules
- Operator's stated direction: mengdie = AE 的大脑, AE plugin
  handles in-session LLM-driven processing (see CLAUDE.md Project
  Status). New sources that bypass AE conflict with this thesis.

## Key Questions
- Is "AE-only" a permanent identity boundary or a v0.0.1-pragmatic
  starting point? (These produce different API designs.)
- If broader sources are ever permitted, what discipline / filter
  prevents the corpus from devolving (e.g., must source X go through
  AE-style structured extraction, must source X carry
  provenance-checkable origins)?
- What's the cost of building forward-compat for sources we may
  never add? (YAGNI vs the cost of a v1 API break later.)
- Are there sources that are obviously valuable but obviously not
  AE pipeline artifacts (e.g., commit messages with conventional
  prefixes — "fix: X", "feat: Y" — that already encode propositional
  facts)? Where does the line go?
- What does the comparable boundary look like in OSS frameworks
  (mem0's "memory" vs "fact", Letta's "core" vs "archival",
  Graphiti's episode types) — is there a pattern transferable here?
