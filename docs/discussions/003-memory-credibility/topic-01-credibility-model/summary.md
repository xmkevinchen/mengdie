---
id: "01"
title: "Memory credibility model"
status: pending
current_round: 1
created: 2026-04-05
decision: ""
rationale: ""
reversibility: ""
---

# Topic: Memory Credibility Model

## Current Status
Initial analysis done. Three approaches identified, one preferred. Needs team discussion to validate.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context

When two users on the same project both have Second Brain, their individual brain quality differs along three axes:

**Ingestion quality**: Alice runs full AE pipeline (analyze → discuss with Doodlestein → plan → work → review). Her conclusion.md has multi-agent discussion, adversarial challenges, Process Metadata. Bob skips discuss, goes straight to plan. His conclusions lack the "why" depth.

**Usage behavior**: Dreaming scores by recall frequency. Alice searches often — rich signal, accurate filtering. Bob rarely searches — all his memories sit at recall_count=0, Dreaming can't distinguish good from bad.

**Judgment quality**: Contradiction detection flags conflicts for human resolution. Alice resolves them carefully. Bob dismisses all. Alice's brain has clean, up-to-date knowledge. Bob's accumulates stale contradictions.

## Constraints
- Engineering culture is sensitive to "rating people" — any solution must evaluate knowledge, not users
- AE pipeline already produces quality signals in its output (Process Metadata, pipeline status, Outcome Statistics)
- Phase 1 schema can pre-populate fields for this; actual team logic is Phase 3
- Must work without requiring all team members to use Second Brain identically

## Key Questions
- Should team promotion be based on "how many people have it" (democratic) or "how good is the memory itself" (meritocratic)?
- What signals are available at ingestion time to assess memory quality?
- How do we handle the case where one expert's unique knowledge is more valuable than three novices' shared misconception?

## Initial Analysis (pre-discussion)

Three approaches were considered:

### Approach A: Memory Quality Score (source-based)

Quality baseline at ingestion time based on source signals:

| Source Signal | Quality Score |
|---------------|--------------|
| AE conclusion.md with discuss + Doodlestein | High |
| AE conclusion.md with discuss skipped | Medium |
| AE review.md with Outcome Statistics | High |
| Claude Code session extraction (unstructured) | Low |
| Manual entry | Unknown |

Low-quality memories need more people to have similar entries before team promotion (e.g., 5 instead of 3).

AE frontmatter naturally provides this: `pipeline.discuss: done` vs `skipped`, `Doodlestein challenges: N` in Process Metadata.

### Approach B: User Credibility Score (behavior-based)

Track per-user behavior: adoption rate of their memories by others, contradiction resolution speed, how often their memories get overturned later.

Problem: quickly becomes a social scoring system. Engineers won't accept it.

### Approach C: Memory Credibility Score (knowledge-based) — PREFERRED

Don't evaluate people. Evaluate each memory entry:

```
memory_credibility =
  source_quality (AE pipeline depth) ×
  validation_count (how many people cited and adopted it) ×
  contradiction_ratio (how often overturned — lower is better) ×
  freshness (time decay)
```

Team promotion threshold based on credibility score, not headcount:

- High credibility memory (full AE pipeline + 3 people cited and adopted + never overturned) → can enter team memory even if only one person originally had it
- Low credibility memory (unstructured session extraction + never cited + unknown source) → can't enter team memory even if 5 people have similar entries

**Why this is preferred**:
- Evaluates knowledge, not people — culturally acceptable
- AE Process Metadata provides `source_quality` naturally
- Dreaming recall tracking provides `validation_count` naturally
- Contradiction detection provides `contradiction_ratio` naturally
- All four signals are natural extensions of existing design — no new systems needed

### MVP Phase 1 Implication

Phase 1 doesn't do team-level, but schema should pre-populate `source_quality` at ingestion time. AE watcher extracts from frontmatter:
- `pipeline.discuss` status (done/skipped)
- `pipeline.review` status
- Doodlestein challenge count (from Process Metadata if present)
- Review Outcome Statistics (rework rate, P1 escape rate)

Cost: near-zero. Just read a few extra frontmatter fields during ingestion. Gives Phase 3 a quality foundation from Day 1.

### Team Promotion Without Shared Docs

The memory credibility model enables a key scenario: two users on the same project, each with their own Second Brain, NOT sharing docs/ via git. Knowledge flows through the team memory layer instead:

```
Alice's AE → Alice's Second Brain → Team Memory ← Bob's Second Brain ← Bob's AE
```

Advantages over git-shared docs/:
- No merge conflicts
- Automatic quality filtering (only credible memories promote)
- On-demand relevance (Bob only sees Alice's knowledge when it's relevant to his current task)
- Conflict detection across users ("Alice found X about this module, Bob found Y — possible misalignment")
