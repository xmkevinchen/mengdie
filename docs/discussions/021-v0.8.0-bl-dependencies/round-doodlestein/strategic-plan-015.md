---
agent: doodlestein-strategic
round: post-review
type: strategic-review-plan
target: docs/plans/015-decay-operator-surface-hardening.md
date: 2026-04-23
verdict: one-improvement
---

# Strategic Review — Plan 015 Post-Review

## Verdict

One smart improvement. The plan is technically tight — sequencing is correct, the AC matrix is verifiable, and the dependency graph was sharpened by codex's review (Step 3 → Step 5 edge added). No step should be reordered or removed.

## The Single Smartest Improvement

**Name `decay_spike` as a schema-contract obligation in Step 6's BL re-file, so the pattern plan 015 establishes doesn't orphan its sibling event.**

### What the plan does

Plan 015 establishes a repeatable schema-contract methodology for machine-readable stderr events:

1. Emit a `schema_version` field on the event
2. Publish a JSON Schema doc (`docs/schemas/dreaming_pass.json`) with Bump Rules
3. Harden the consumer script against silent parse failure
4. Cover the contract with a subprocess integration test

This is the right methodology. The five reviewers verified that it's correctly scoped to `dreaming_pass`.

### What none of them flagged

`BL-decay-threshold-mode` — re-filed in Step 6 — specifies a second machine-readable event: `decay_spike`. The action 5 body text (carried forward verbatim per Step 6) describes `decay_spike` as a JSON emission on stderr, parallel in structure to `dreaming_pass`. It is a new versioned contract, not a new field on the existing one.

Plan 015 is silent on whether `BL-decay-threshold-mode` should follow the same schema-contract pattern (schema doc + version field + subprocess integration test). The re-filed BL will land in the BL-010 daemon sprint. A BL-010 planning agent reading `BL-decay-threshold-mode.md` will see the event design but won't see a requirement to replicate the methodology — because plan 015 established the methodology as a one-time deliverable scoped to `dreaming_pass`, not as a standing convention.

The compounding consequence: `decay_spike` ships in BL-010 without a `schema_version` field, without a `docs/schemas/decay_spike.json`, and without a subprocess test — creating a second unversioned machine contract of exactly the kind plan 015 was written to prevent. The methodology regresses on its first reuse.

### Concrete change

In **Step 6**, add one subtask to the `BL-decay-threshold-mode.md` creation checklist:

> Add a "Schema Contract Obligation" section to the BL body: "`decay_spike` must follow the same schema-contract pattern as `dreaming_pass` (plan 015): emit `schema_version: 1`, publish `docs/schemas/decay_spike.json` with Bump Rules, and add a subprocess integration test asserting all fields. See `docs/schemas/dreaming_pass.json` for template."

This is a one-paragraph addition to the BL body during Step 6 — zero implementation cost to plan 015. It makes the methodology self-propagating: the BL-010 planning agent will see the obligation when reading the BL, and the plan reviewer for the BL-010 sprint will have a specific AC to check.

### Why this compounds

Without it, the schema-contract pattern is a local convention documented only inside plan 015. Plan 015 is a completed plan — the methodology won't be visible in the BL body that actually enters the BL-010 sprint. With it, the obligation travels with the deferred work. The next reviewer doesn't need to know plan 015 existed; the BL is self-contained.

The five reviewers were correctly focused on whether plan 015 is internally correct. None had a reason to look at what `BL-decay-threshold-mode` will say after Step 6 writes it — that's a post-plan artifact, outside the plan-review frame. This is the blind spot.

## Why No Other Improvement Ranks Higher

The plan's only other structural gap is that AC2's schema-validation step is manual (`ajv` / `jsonschema` CLI, not CI-executed). Making it mechanical would require a jsonschema tool in the CI image — a new CI dependency that's out of scope for a hardening plan. The gap is real but it's accepted scope-not-now, not a reframe. The `decay_spike` issue compounds across sprints; the AC2 manual verification is a one-sprint caveat that doesn't replicate.
