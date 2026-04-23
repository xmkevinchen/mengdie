---
author: doodlestein
type: regret
discussion: "021"
created: 2026-04-23
---

# Regret Analysis — Most Likely Reversed Decision

## Verdict: Decision 4 — Sprint-commitment policy (file upstream AE BL + one-line CLAUDE.md checklist)

This is the single most regret-prone decision.

## Why

**Condition dependency**: The rationale rests on "n=2 is small, so a prospective-only upstream BL is zero-cost." That framing is only stable if the upstream AE project actually picks up and implements the BL. If AE never acts on it (likely — it's unscheduled, low-priority, the owner is the same person with a full sprint), the policy change reduces to a one-line checklist that was already the minimal-change-engineer's preferred fallback. The "full solution" drifts to "the thing we didn't do."

**Preserved dissent**: Minimal-change-engineer's dissent is on record: "marker is premature abstraction over n=2; checklist line alone is sufficient." That dissent isn't a minority quibble — it correctly identifies the structural weakness. If the upstream BL sits unscheduled for 2–3 sprints, the team will have been proven right and the convergence decision will look like unnecessary overhead.

**Trigger hasn't fired, and may not**: The upstream BL approach only pays off when n grows beyond 2 and the scan-filter matters operationally. Current evidence: 2 instances in the entire project history. For a solo-dev project building toward v1, the probability that `admission_status` becomes a useful scan filter within 6 months is low. The BL-filing action is forward-speculating on growth that hasn't materialized.

**Contrast with Decision 2**: The `/ae:roadmap remove` decision (also "high" reversibility) has already-verified non-destructive tooling and a clear trigger-not-fired rationale that won't change. That decision is stable. Decision 4's upstream action depends on a second project's sprint priorities.

## What reversal looks like

In 2–3 sprints: upstream AE BL remains unscheduled, no new `defer-until-trigger` incidents arise, and someone notices the CLAUDE.md checklist line is doing all the work anyway. Decision gets re-characterized as "the checklist was sufficient; the upstream BL filing was process overhead we didn't need." The upstream BL gets quietly closed or never planned.

## Caveat

No decision here is egregiously regret-prone — this is a well-deliberated conclusion with evidence-shifted positions. Decision 4 is the weakest link only because it has a dependency on an external project's execution that the team cannot control.
