---
id: BL-025
title: "AE plugin per-skill memory_ingest wiring + explicit scope parameter (Topic 1 + Topic 3 implementation)"
status: open
created: 2026-05-05
origin: "discussion 027 conclusion (Topic 1 push-primary + Topic 3 per-project default with explicit scope per skill)"
trigger: "v0.0.1 ingest contract finalized (parameters added to memory_ingest + memory_search). NOT gated on Topic 2 ReflectionTrigger trait — the trait is architecturally independent of per-skill ingest wiring (corrected post-conclusion per doodlestein-adversarial finding in 027 conclusion.md)."
depends_on: ["BL-023 (project_id cwd-switch fix lands as co-commit)"]
size: M
v_target: "v0.0.1 — primary AE-side integration work"
---

# BL-025 — AE plugin per-skill memory_ingest wiring

## Origin

Discussion 027 Topic 1 settled on **push-primary**: AE skills explicitly call `memory_ingest` after each pipeline phase produces output. Topic 3 settled on **ratify per-project default scope** with the refinement that "AE skills should specify `scope` per-skill explicitly" (system-architect Round 2; absorbed challenger's reframe).

Both decisions imply concrete AE-plugin-side work that mengdie itself cannot ship — the AE plugin is a separate project at `../agentic-engineering/`.

## Scope

Two work-items, both AE-plugin-side:

### 1. Per-skill `memory_ingest` calls (Topic 1)

Wire each terminal AE skill to explicitly call `memory_ingest` on its output:

- `/ae:work` → after each commit, ingest the commit's plan/review/conclusion artifacts
- `/ae:review` → after the review file is written, ingest review.md
- `/ae:discuss` → after conclusion.md is written, ingest conclusion.md (+ entity extraction)
- `/ae:retrospect` → after retrospect output, ingest retrospect.md
- `/ae:analyze` → after analysis.md is written, ingest analysis.md (+ Round 0 prior-context query already exists per Step 1.6 of the corresponding skill)

Each call is synchronous (push), errors visible to caller (failure mode = AE skill reports ingest failure to operator; does not silently drop).

### 2. Explicit `scope` parameter per call (Topic 3)

Each MCP `memory_search` call from an AE skill should specify `scope: Project | Global` explicitly. Default-when-omitted is `Project` per Topic 3 ratify, but the ratify decision's refinement is that AE skills should NOT rely on the default — they should pass `scope` based on the skill's intent:

- `/ae:analyze` Round 0 prior-context query → `scope: Project` (project-specific decision history)
- `/ae:plan` candidate prior-art query → `scope: Project` typically; `scope: Global` for cross-project Rust idioms / MCP patterns / etc.
- `/ae:retrospect` → `scope: Project` (current project's recent shipping)

The skill author is in the best position to choose; mengdie's MCP API simply accepts the parameter.

## Co-commit with BL-023

Per BL-023 (`project_id` cwd-switch staleness), the natural fix path is **explicit `project_id` parameter per call**. BL-023 + BL-025 should land together because:

- `memory_search` and `memory_ingest` both gain `project_id` (BL-023) AND `scope` (BL-025) parameters
- AE skills call them with both populated from the skill's session context

## Acceptance criteria

- Each terminal AE skill (work / review / discuss / retrospect / analyze) calls `memory_ingest` after producing its primary output artifact
- Each `memory_search` call from AE skills passes `scope` explicitly
- Each `memory_ingest` and `memory_search` call passes `project_id` explicitly (co-commit with BL-023)
- Failure mode: ingest errors surface to the AE skill's operator-visible output; do not silently drop

## Trigger

Fires when v0.0.1 ingest contract is finalized in mengdie (`memory_ingest` + `memory_search` accept the `project_id` and `scope` parameters). The work is split between mengdie (parameter accepting) and AE plugin (parameter passing).

**NOT gated on Topic 2 `ReflectionTrigger` trait shipping.** That trait is the output of Topic 2 (synthesis trigger model) and is architecturally independent of per-skill ingest wiring. doodlestein-adversarial flagged this dependency as a false-AND in the post-conclusion review of discussion 027. Wiring can land before, after, or alongside the trait — the trait governs WHEN synthesis runs; wiring governs WHAT gets ingested.
