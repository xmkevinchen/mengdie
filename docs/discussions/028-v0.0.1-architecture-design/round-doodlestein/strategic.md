---
reviewer: doodlestein-sonnet
type: strategic
scope: post-conclusion
---

# Strategic Review — Discussion 028

## Finding: Sprint BL list is flat; needs explicit dependency edges

The conclusion's four decisions are sound and well-supported. The meta-decision
(no MCP ACK) is correctly derived. The backlog trigger conditions are specific.

The single weakest point is the **Next Steps sprint BL list** (items 1–5). It
is presented as a flat enumeration but contains at least two hidden dependency
constraints that, if ignored during `/ae:plan`, will produce a misordered sprint.

### Dependency 1: BL #2 and BL #3 are co-committed, not sequential

BL #2 (mcp_tools two-ingest-paths defect fix) and BL #3 (search.rs
free-functions refactor) both touch the same boundary between `mcp_tools.rs`
and `search.rs`. Shipping one before the other creates a short-lived API shape
that the next PR immediately invalidates. The conclusion's own rationale for
Topic 1 ("alongside" language) implies co-commitment but does not make it
explicit. A sprint plan that queues them sequentially will produce an
intermediate commit state with no stable contract.

**Recommended addition**: Mark BL #2 + BL #3 as `co-commit` — single PR, single
review gate.

### Dependency 2: BL #4 (sqlite-vec spike) gates the Reflection module consolidation backlog item, not the sprint BLs — but this is invisible

The conclusion defers Reflection module consolidation to "after sqlite-vec
spike outcome." BL #4 is sprint work. But nowhere does the conclusion state
that the sqlite-vec spike result must be recorded (not just run) before the
consolidation BL can be filed. If the spike is done but its outcome is not
captured as a decision record, the consolidation BL will be filed against an
unresolved gate — exactly the pattern that produced the "phantom-active
discussion" problem noted in the CLAUDE.md advisory rule.

**Recommended addition**: BL #4 acceptance criteria must include a written
outcome record (PASS/FAIL + rationale), not just code. The outcome record
triggers (or closes) the consolidation backlog item atomically.

### Dependency 3: BL #5 (audit table) is labeled P0 in the conclusion body but listed last

BL #5 is the instrumentation prerequisite for the A-MEM trigger to ever become
computable. The A-MEM trigger condition (corpus floor + supersession signal)
cannot be evaluated without the audit table rows. Yet BL #5 appears fifth in a
list that reads top-to-bottom as execution order.

**Recommended addition**: Promote BL #5 to first in the sprint list (or annotate
it as "P0 — unblocks Topic 4 trigger observability; must land before corpus
grows further"). The longer the audit table is absent, the larger the gap in
the historical signal.

## Recommended reframe for Next Steps

Replace the flat numbered list with a two-tier structure:

**Wave 1 (unblocked, can start immediately):**
- BL #5 — Persisted domain audit table + `returned_fact_ids` logging (P0)
- BL #4 — sqlite-vec compatibility spike (outcome record required as exit criterion)
- BL #1 — AE plugin Round-0 wiring (independent; lives in agentic-engineering/)

**Wave 2 (after Wave 1 BL #4 outcome is recorded):**
- BL #2 + BL #3 — co-committed defect fix + free-functions refactor

This reframe does not change any architectural decision. It converts the same
five BLs into an actionable execution sequence that `/ae:plan` can consume
without re-deriving the dependency graph.

## Verdict

The conclusion is correct as architecture. The improvement is purely
operational: making the sprint ordering and coupling explicit so that the plan
generator does not need to re-derive them from rationale prose.
