---
id: "01"
title: "Bundle boundary for the decay-cluster plan"
status: converged
current_round: 2
created: 2026-04-23
decision: "Split 2+1 — Plan A bundles json-schema-contract + verify-decay-hardening (~100 LOC, M+M, hard-dep inside); Plan B is ops-doc-polish alone (S, ~20-30 LOC, pure docs + 1-line format_dreaming_line tweak)"
rationale: "Unanimous Round 2 after archaeologist factual corrections: ops-doc-polish edits `format_dreaming_line` at cli.rs:226, NOT `format_structured_json` at cli.rs:207 (different functions 19 lines apart) — architect's 'same function' bundling argument was refuted. Hard dep (schema-contract → verify-decay-hardening via the integration test that asserts on schema_version) stays atomic in Plan A. Ops-doc's coupling is soft (references breaches[] field). Split preserves 4-agent review depth; challenger cited plan 013 evidence that bundled plans caught cross-step blockers pre-/ae:work, applicable to the M+M bundle. Codex independently revised to 2+1 split."
reversibility: "high"
reversibility_basis: "Plan organization is decided at /ae:plan time; if split turns out wrong, can expand Plan A or merge docs into a later plan. No persistent artifact commits to the boundary."
---

# Topic: Bundle boundary for the decay-cluster plan

## Current Status

**Converged**: split 2+1. Plan A = json-schema-contract + verify-decay-hardening. Plan B = ops-doc-polish.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 3 positions: bundle-all (architect, codex), split-ops-doc (challenger), split-2+1 (minimal-change); gemini/archaeologist neutral |
| 2 | converged | Archaeologist's Round 1 factual correction (ops-doc edits different function) drove architect and codex to revise. All 6 agents converged on split 2+1. |

## Context

The 3 candidate BLs touch overlapping decay-operator-surface files:
- `BL-decay-json-schema-contract` → `src/bin/cli.rs::format_structured_json` (:207) + new `docs/schemas/dreaming_pass.json` + integration test
- `BL-verify-decay-script-hardening` → `scripts/verify-decay.sh` + new test file (actions 2+4 active; actions 1 revisited; action 3 already done)
- `BL-decay-ops-doc-polish` → `docs/operations/dreaming-decay.md` + minor `src/bin/cli.rs::format_dreaming_line` (:226) tweak (different function from schema contract's target)

## Decision Details

**Plan A — "decay operator surface hardening" (schema + verify)**:
- BL-decay-json-schema-contract (M)
- BL-verify-decay-script-hardening (M, effective work = actions 1 + 2 + 4; action 3 already done; action 5 defers per Topic 3)
- Shared: integration test harness that spawns `mengdie dream --decay-dry-run` and parses stderr JSON
- Hard-dep locked inside one plan (schema contract writes `schema_version: 1`; verify-decay integration test asserts on it)

**Plan B — "ops doc polish"**:
- BL-decay-ops-doc-polish (S)
- Can ship after Plan A lands (rollback procedure references `breaches[]` from the schema) or in parallel if plan order is maintained during review

**Rejected alternatives**:
- Bundle all 3: refuted by archaeologist (different cli.rs functions, soft coupling, docs-review scope differs from code-review scope)
- Split all 3: refuted by shared test harness between schema-contract and verify-decay
