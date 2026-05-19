---
id: BL-052
title: "F-011 + F-008 batch review deferred findings — 5 observability/configurability gaps with explicit triggers"
status: open
created: 2026-05-19
origin: "F-011 + F-008 batch /ae:review on feature/v0.0.2 — codex-proxy + challenger surfaced 5 findings that are non-blocking for ship but should re-fire when specific trigger conditions hold"
size: S
depends_on: []
v_target: "trigger-deferred — do NOT promote until trigger fires"
---

# BL-052 — F-011 + F-008 batch review deferred findings

## Origin

F-011 (memory_status) + F-008 (Memory Lint) batch review on
feature/v0.0.2 produced 2 fix-now items (shipped as part of F-008
commit + F-011 follow-up) and 5 defer-to-trigger items consolidated
here. All 5 are observability or configurability gaps — the features
ship correctly today; the gaps would matter under specific dogfood
patterns that haven't manifested yet.

## Scope (5 deferred findings)

### Finding 1 — F-011 status_breakdown non-atomic snapshot

`mcp_tools::status` composes three independent DB calls under
separate locks: `status_breakdown` + `list_metrics` + `audit_stats`.
The `status_breakdown` doc comment claims "single connection lock so
the snapshot is consistent" — true within its own call, but the
composed `StatusOutput` is not atomic. A concurrent ingest between
the three calls produces slightly-inconsistent snapshot (e.g.,
total_entries from breakdown reflects pre-ingest count, while
metrics.ingest_count reflects post-ingest).

**Severity**: low (diagnostic tool, race window microseconds).
**Trigger**: any consumer treats `memory_status` as an atomic snapshot
(e.g., dashboard alerting based on consistency of fields).
**Fix**: either remove the "consistent snapshot" claim from
`status_breakdown` doc, OR compose all three queries under a single
lock in `mcp_tools::status` (refactor to `db::full_status_breakdown`
or pass conn into helpers).

### Finding 2 — F-011 metrics: BTreeMap vs typed fields

`StatusOutput.metrics: BTreeMap<String, i64>` carries 4-5 known
counter keys (search_count, ingest_count, conflict_count,
audit_write_failures, search_nonempty_count). BTreeMap shape signals
"future expansion expected"; if the counter set is stable, typed
fields would be more cross-family-SDK-idiomatic.

**Severity**: low (API ergonomics).
**Trigger**: counter set stable for 1 full release cycle without
additions, OR an LLM SDK consumer reports parsing-ambiguity issues
with the dynamic-keys shape.
**Fix**: replace BTreeMap with explicit fields (one per counter);
optionally retain a `custom_metrics: BTreeMap<...>` for future
additions.

### Finding 3 — F-008 LintReport count + samples dichotomy

Each F-008 check carries `_count: i64` + `_samples: Vec<String>`.
Dual shapes add parser complexity for LLM consumers (which to
trust? when do they diverge?). Single findings list (`findings:
Vec<Finding>` with embedded count via `len()`) is more idiomatic for
tool outputs.

**Severity**: low (API design refactor).
**Trigger**: second LLM consumer beyond the operator's Claude Code
session ships against `memory_lint` AND parser ambiguity surfaces as
a real bug.
**Fix**: refactor each `XxxCheck` struct to `findings: Vec<Finding>`;
expose `total` as a derived field for backwards-compat if needed.

### Finding 4 — F-008 SAMPLE_CAP hardcoded to 5

`const SAMPLE_CAP: usize = 5` in lint.rs. No MCP param to override.
Operators with high-entropy orphan counts (e.g., 20+ dangling
references after a `rename_project` collision) can only see 5 sample
IDs; remaining IDs require direct DB inspection or repeated
lint-cycle-and-fix runs.

**Severity**: low (operator-workflow friction).
**Trigger**: `mengdie lint` CLI subcommand ships (F-008 deferred Step
5), OR operator reports needing >5 samples regularly.
**Fix**: add `sample_cap: Option<usize>` to `LintParams`; thread
through to check methods. Default stays 5.

### Finding 5 — F-008 entity_overlap_unsuperseded_count truncates silently at 100-fact window

Check 2d caps pair-scan at 100 active facts per run. At scale (>100
active facts in project), `entity_overlap_unsuperseded_count=0` is
ambiguous: "no overlapping pairs" vs "all overlapping pairs outside
scanned window". Code comment documents the cap; wire format does
not expose `facts_scanned` / `scan_complete` to callers.

**Severity**: medium (false-negative observability gap at scale).
**Trigger**: corpus reaches >100 active facts per project (~10× current
dogfood corpus), OR any dashboard / automated alert consumes
`memory_lint.unresolved_contradictions.entity_overlap_unsuperseded_count`.
**Fix**: add to `ContradictionCheck`:
- `entity_overlap_facts_scanned: i64`
- `entity_overlap_scan_complete: bool`
Callers can detect partial scans + re-run with offset.

## Acceptance criteria (per finding, when triggered)

Each finding is independently shippable when its trigger fires. The
BL is a tracking aggregate — promote individual findings to their own
F-NNN dirs if multiple trigger simultaneously.

## Non-goals

- Not a "fix all 5 at once" feature. Each finding triggers
  independently; promote one at a time.
- Not an architecture refactor. All 5 are local API / wire-format
  adjustments inside existing module boundaries.

## Coordination notes

- F-011 P3 (idempotency test asserts on counts not samples — challenger
  P4) is NOT included here; it's a test-strengthening item that should
  land as part of any refactor of `lint_query_ids`. Filed as part of
  the refactor's own AC list, not as a standalone BL.
- F-011 by_project HashMap breakdown (plan AC deferred): tracked
  inside F-011 plan completion note; promote to its own BL when
  cross-project workflow ships.

## Estimated size

S (~50-100 LoC delta + ~30 LoC tests per finding). Total if all 5
ship: ~300-500 LoC. But explicitly NOT one feature — five separate
trigger-fired increments.
