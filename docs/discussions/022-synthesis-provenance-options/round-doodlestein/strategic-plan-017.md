---
id: "022-doodlestein-plan-017"
title: "Doodlestein strategic review — plan 017"
type: strategic-review
created: 2026-04-23
plan: "docs/plans/017-synthesis-cluster-hardening.md"
---

# Doodlestein Strategic Review — Plan 017

## Single Smartest Improvement

**The audit subcommand is framed as scaffolding, but the plan ships no collection discipline.**

The plan names Option 1 (synthesis-audit) as "scaffolding for future Options 2/3 ship-gate data collection" — the explicit instrument for growing the hallucination sample from 10/27 to 100%. Yet the plan contains zero mechanism for that collection to actually happen. No guidance on when to run it, no output format designed for aggregation, no tracking of which syntheses have been audited.

This is not a missing feature request. The missing piece is a **decision about what "audit at 100% sample" means in practice** and whether it should happen before or after Options 2/3 are reconsidered. Without a stated trigger or owner, "use the audit subcommand to grow the sample" degrades to zero-probability aspiration — the same class of deferred-but-never-revisited items that fill `docs/backlog/unscheduled/`.

### Why 5 reviewers wouldn't surface this

Reviewers evaluated the plan against its stated scope (dedup key + audit subcommand + formatter). The subcommand is technically correct — read-only, well-specified, integration-tested. No reviewer had cause to challenge a feature that matched the design intent and passed acceptance criteria.

The gap lives one level up: it's not about *what the subcommand does* but *whether it will fulfill its named purpose*. That reframe requires stepping outside the plan boundary to ask: does plan 017, as shipped, actually advance the data-gating precondition for Options 2/3?

### Concrete improvement

One of three equally lightweight choices:

**A. Add a collection note to Step 6 (no new work)**
When closing BL-synthesis-provenance, add a one-line entry to `.ae/backlog/unscheduled/` titled `BL-audit-collection-discipline` with: (a) trigger condition for revisiting Option 2/3 (e.g., "operator has audited ≥50% of synthesis rows via synthesis-audit and recorded results"), (b) the shell one-liner for batch-running it, (c) pointer to discussion 022's "zero hallucinations in 10/27" baseline. Cost: 10 minutes in Step 6. Effect: the scaffolding becomes a credible future-trigger rather than a wishful footnote.

**B. Add `--batch` flag to synthesis-audit (small scope expansion)**
Allow `mengdie synthesis-audit --batch` to iterate all synthesis rows and emit structured output (one JSON line per synthesis) to stdout. Operators can pipe to `jq` and manually record pass/fail. Cost: doubles Step 3 work. Probably out of scope for this plan.

**C. Document expected audit workflow in the subcommand's --help**
When the operator runs `mengdie synthesis-audit --help`, the description says something like: "Use after each dream run to verify synthesis fidelity. Record pass/fail manually; if failure rate exceeds 1/20, file a ticket to evaluate Option 2 (LLM verification)." Cost: one sentence in Step 3. Zero schema/logic change.

**Recommended**: Option A (Step 6 backlog entry) because it's in-scope, creates a durable future-trigger rather than inline documentation that rots, and matches the codebase's established convention for recording exactly this kind of "revisit when X" condition.

### Why this compounds across future plans

This codebase has at least three other "ship when data says so" deferments (Options 2, 3 from discussion 022; residuals-reduction from discussion 018; power-law decay from discussion 019). All of them depend on data collection happening. If the pattern of "scaffold the instrument but skip the collection discipline" repeats, Options 2 and 3 will live in `unscheduled/` indefinitely because no one knows when the gate has been reached. A single backlog entry with a concrete metric and a named instrument breaks the pattern.
