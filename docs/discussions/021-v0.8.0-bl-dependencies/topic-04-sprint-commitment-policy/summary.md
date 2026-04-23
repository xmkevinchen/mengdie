---
id: "04"
title: "Sprint-commitment policy for unresolved pre-conditions"
status: converged
current_round: 2
created: 2026-04-23
decision: "File one upstream AE backlog item proposing `admission_status: defer-until-trigger` frontmatter field with /ae:roadmap plan scan-filter (prospective-only, no retroactive marking). Do NOT apply any marker or local YAML field to mengdie BLs until AE ships the feature. In the meantime, one-line mitigation in mengdie CLAUDE.md 'Review Rules' section: 'Before /ae:roadmap plan, skim candidate BL bodies for explicit not-now / filed-for-trigger language; remove such items via /ae:roadmap remove first.'"
rationale: "5-1 split: architect, codex, gemini, challenger, archaeologist-evidence support the upstream-AE-BL approach (codex estimated 70-120 LOC implementation, gemini scoped it prospective-only to avoid retroactive-marking cost); minimal-change-engineer dissents (argues one-line checklist is sufficient, marker is premature abstraction over n=2). Compromise captures both: upstream BL gets filed (majority), but mengdie-local action is just the checklist line (minority-compatible). Rationale for prospective-only: false-positive risk (BL marker fires but trigger would actually have met in sprint window) and false-negative risk (existing BLs without marker slip through) balance out by only applying to NEW BLs after AE ships the feature."
reversibility: "high"
reversibility_basis: "Filing an upstream BL is a cheap commitment — AE project owns if/when/how. No code change in mengdie beyond one CLAUDE.md line. Upstream BL can be dropped if AE declines. The checklist line is a single edit, trivially revertable."
---

# Topic: Sprint-commitment policy for unresolved pre-conditions

## Current Status

**Converged (with noted dissent)**: file upstream AE BL for `admission_status: defer-until-trigger` marker (prospective-only); add one-line checklist to mengdie CLAUDE.md Review Rules.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 3 positions: marker-needed (architect, codex leaning), no-policy (minimal-change, challenger, gemini), case-by-case (architect as fallback) |
| 2 | converged-with-dissent | 5 agents converge on "file upstream AE BL + prospective-only scope"; minimal-change-engineer preserves dissent: "no marker; one-line checklist sufficient" |

## Decision Details

**Upstream AE backlog item to file** (in `/Users/ckai/Workspace/Projects/agentic-engineering/.ae/backlog/unscheduled/`):

```
Title: BL-admission-status-defer-until-trigger
Problem: /ae:roadmap plan commits BLs to sprints even when the BL body says
"not now" / "filed for trigger" — see mengdie v0.8.0 where 2 of 9 committed
items had bodies explicitly deferring work.
Proposal: add optional `admission_status: defer-until-trigger` frontmatter
field. /ae:roadmap plan scans candidate BLs and excludes items with this
field (offers review to user). Prospective-only: no retroactive marking on
existing BLs.
Size estimate (codex): 70-120 LOC skill change.
Trigger: file now; schedule when AE roadmap owner prioritizes.
Origin: mengdie discussion 021.
```

**Mengdie-local action** (in `CLAUDE.md` Review Rules section):

```
Add one line:
"Before /ae:roadmap plan: skim candidate BL bodies for explicit 'not now'
or 'filed for trigger' language; /ae:roadmap remove such items before sprint-commit."
```

**Dissent preserved** (minimal-change-engineer):
> "The marker is premature abstraction over n=2. Maintenance cost of the YAML field (across AE, mengdie, and any other downstream projects) exceeds the cost of the one-line checklist. If AE ships the feature anyway, adoption should be opt-in, not default."

This dissent is honored in the outcome: the mengdie-local action is the checklist line (minimal-change's preferred solution). The upstream BL filing is a zero-cost-to-mengdie action (AE owns the followthrough).

**Rejected alternatives**:
- Full policy document: refuted by 5 of 6 agents as ceremony.
- Apply `admission_status` to existing mengdie BLs now: refuted by gemini's retroactive-marking-cost argument.
- Nothing at all (not even upstream BL): minimal-change's pure position; outvoted 5-1 but preserved in dissent.

## Sub-question resolution: close-state semantics

`/ae:roadmap close` uses warn-by-default on open items (archaeologist Round 2 verification). v0.8.0 CAN close with open items; `--strict` is required to refuse. This makes Topic 2's remove-both action a cleanliness choice, not a tooling-forced one. The converged approach (remove both + gate update) eliminates the warning path.

## Sub-question resolution: admission-gate for future sprints

The upstream AE BL addresses the admission-gate question. Mengdie does not need its own admission gate; it inherits whatever AE ships.
