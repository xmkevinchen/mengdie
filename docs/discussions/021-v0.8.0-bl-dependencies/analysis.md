---
id: "021"
title: "Analysis: v0.8.0 remaining BL dependencies"
type: analysis
created: 2026-04-23
tags: [v0.8.0, sprint-planning, dependency-analysis, decay, synthesis]
---

# Analysis: v0.8.0 remaining BL dependencies

## Question

Of the 7 open BLs in sprint v0.8.0, what dependencies exist? Which are
hard-sequential, which overlap in code surface, which are independent?

## Findings

### Subsystem clustering

The 7 open BLs fall into two non-interacting subsystems:

**Decay cluster (4 items, 9 pt)** — all touch the BL-008 decay surface
(`src/bin/cli.rs` `format_structured_json`, `scripts/verify-decay.sh`,
`docs/operations/dreaming-decay.md`, `src/core/dreaming.rs` decay pass):

| BL | Size | Primary file(s) |
|----|------|-----------------|
| BL-decay-json-schema-contract | M | `src/bin/cli.rs` (+ new `docs/schemas/dreaming_pass.json`) |
| BL-verify-decay-script-hardening | M | `scripts/verify-decay.sh` + test infra |
| BL-decay-ops-doc-polish | S | `docs/operations/dreaming-decay.md` (+ minor cli.rs format tweak) |
| BL-decay-dreaming-pass-optim | S | `src/core/dreaming.rs::run_dreaming_with_config` |

**Synthesis cluster (3 items, 11 pt)** — all touch the BL-007 synthesis
surface (`src/core/dreaming.rs::run_synthesis_pass`, `insert_synthesis_with_links`,
search result shape):

| BL | Size | Primary file(s) |
|----|------|-----------------|
| BL-synthesis-dedup-key | L | `src/core/schema.rs` (v5 migration) + `src/core/db.rs::insert_synthesis_with_links` |
| BL-synthesis-provenance | L | `src/core/dreaming.rs` + search output CLI + possibly `src/core/schema.rs` (v6 if quality-score option) |
| BL-synthesis-preload-db-miss-edge | XS | `src/core/dreaming.rs::run_synthesis_pass` (4-line compensation) |

**Cross-cluster**: decay and synthesis BLs touch different code. Zero
cross-cluster dependencies.

### Hard dependencies (must sequence)

**Decay cluster**:
- `BL-decay-json-schema-contract` → `BL-verify-decay-script-hardening`
  — the hardening BL adds a CI integration test that parses the
  structured JSON line. If the test asserts on `schema_version: 1`
  (added by the contract BL), the contract BL must ship first. OR
  both ship together in one plan.
- `BL-decay-json-schema-contract` → `BL-decay-ops-doc-polish` — the
  ops doc polish includes a rollback procedure that references the
  `breaches[]` array from the JSON event. Docs reference schema; schema
  should be locked first. Soft dependency — could co-ship.

**Synthesis cluster**:
- `BL-synthesis-dedup-key` → `BL-synthesis-provenance` — **conditional**.
  Dedup-key adds schema v5. If provenance chooses Option 2 (LLM fidelity
  verification + new `memory_quality_score` column) it adds v6; must
  follow v5. If provenance chooses Option 3/4 (downrank score / CLI
  prefix), no schema change, no dependency. The BL body doesn't commit
  to an option — discussion needed to resolve.

### Soft dependencies (benefit from ordering)

- All 3 actively-workable decay BLs could bundle into a single plan.
  Natural coherent unit: "harden the decay operator surface" — JSON
  schema doc + script that consumes it + ops doc that references both.
  3 files + 1 new schema doc + 1 integration test. ~8 pt total
  (M+M+S). Shipping separately would duplicate plan/review cycles for
  what's really one PR.

- Within synthesis cluster: dedup-key's schema migration is the
  biggest thing. Provenance work is cleaner to slot in AFTER migration
  is stable (even if provenance doesn't add its own schema column).

### Explicit "defer until trigger" items

Two of the 7 BLs explicitly say "not now" in their bodies — they're
filed to record the trigger, not to be worked on in v0.8.0:

- **BL-decay-dreaming-pass-optim** (S): body says "NOT pursuing this
  now. The current implementation's clarity is worth more than the
  premature optimization at current scale. This backlog item exists
  so the trigger is recorded." Triggers: corpus > 50k long-term
  memories, `mengdie dream` p95 > 1s, or BL-010 daemon lands. None of
  those are v0.8.0 concerns.

- **BL-synthesis-preload-db-miss-edge** (XS): body says "Why filed as
  backlog rather than fixed now... edge is documented in
  `src/core/dreaming.rs` attribution-invariant docstring but not
  fixed." Trigger requires `mengdie delete`/`memory_invalidate` CLI
  subcommand to land (doesn't exist yet) OR an observed real-world
  arithmetic mismatch. Neither is a v0.8.0 concern.

These 2 items being in v0.8.0 is a **sprint composition artifact**,
not active work. Options:
1. Leave in v0.8.0 → sprint closes with 2 "wontfix-yet" items; velocity
   counts against them
2. `/ae:roadmap remove` them → back to `unscheduled/` until trigger fires
3. Close them explicitly as `superseded-by-trigger` with the trigger
   condition preserved

### Actively-workable items (5)

After excluding the 2 defer-until-trigger items:

| BL | Size | Cluster | Must precede / follow |
|----|------|---------|----------------------|
| BL-decay-json-schema-contract | M | decay | precedes verify-decay (test), ops-doc (references breaches[]) |
| BL-verify-decay-script-hardening | M | decay | follows json-schema-contract |
| BL-decay-ops-doc-polish | S | decay | follows json-schema-contract (soft) |
| BL-synthesis-dedup-key | L | synthesis | conditionally precedes provenance |
| BL-synthesis-provenance | L | synthesis | conditionally follows dedup-key |

## Summary

**3 real dependencies** among the 7 open BLs:

1. `BL-decay-json-schema-contract` → `BL-verify-decay-script-hardening`
   (hard — script tests the schema)
2. `BL-decay-json-schema-contract` → `BL-decay-ops-doc-polish` (soft —
   docs reference the schema)
3. `BL-synthesis-dedup-key` → `BL-synthesis-provenance` (conditional —
   depends on provenance fix option choice)

**2 explicit "defer-until-trigger" items** that technically open but
shouldn't count as active v0.8.0 work:
- `BL-decay-dreaming-pass-optim` (scale trigger)
- `BL-synthesis-preload-db-miss-edge` (delete-path trigger)

**Zero cross-cluster dependencies** — decay and synthesis can be
worked in parallel or in either order without interaction.

## Recommendation

**Cheapest execution plan for v0.8.0's active slice**:

1. **Plan A: "Decay operator surface hardening"** — bundle all 3
   active decay BLs into one plan (~8 pt, M+M+S). One PR, one review,
   one ci.yml trigger. Scope: `src/bin/cli.rs` + `scripts/verify-decay.sh`
   + `docs/operations/dreaming-decay.md` + `docs/schemas/dreaming_pass.json`
   + integration test. Shipping as 3 separate plans wastes cycle
   overhead.

2. **Plan B: "Synthesis dedup-key + provenance"** — after Plan A
   lands. ~10 pt (L+L). Prerequisite: `/ae:discuss` on provenance fix
   options (the BL lists 4; no commitment yet) before `/ae:plan`. If
   provenance picks schema-free option (downrank / CLI prefix), plans
   could split; otherwise co-plan after dedup-key's v5 migration.

3. **Defer-decision for the 2 trigger-gated items**: either
   `/ae:roadmap remove` them back to unscheduled, or close as
   `superseded-by-trigger`. Recommend remove — preserves the BL text
   cleanly for when the trigger actually fires.

**Dependency graph (actively-workable items)**:

```
                    ┌─→ BL-decay-ops-doc-polish (S)
BL-decay-json-      │
schema-contract ────┤
    (M)             │
                    └─→ BL-verify-decay-script-hardening (M)


BL-synthesis-    ──────→ BL-synthesis-provenance (L, conditional)
dedup-key (L)           [only if provenance picks schema option]
```

## Possible Next Steps

- `/ae:plan` on "decay operator surface hardening" (Plan A) —
  bundles 3 BLs, deps already satisfied by within-plan ordering
- OR `/ae:discuss docs/discussions/021-v0.8.0-bl-dependencies/`
  if the defer-decision on the 2 trigger-gated items needs team
  consideration before acting
- OR `/ae:roadmap remove BL-decay-dreaming-pass-optim` + same for
  `BL-synthesis-preload-db-miss-edge` to clean up sprint composition
  first, then proceed to Plan A
