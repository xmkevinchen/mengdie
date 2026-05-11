---
agent: doodlestein-strategic-post
verdict: P2 (plan-time TODO)
timestamp: 2026-04-28T20:51:59Z
---

# Doodlestein-strategic post-conclusion review

**Finding**: Next Steps conflates "supersession SQL is the v0.0.1 acceptance test" with "supersession SQL has no in-binary caller," creating a silent gap where the plan author could ship the schema without any assertion that the supersession query is correct.

**Severity**: P2 (should integrate into plan)

**Recommendation**:

> The plan's acceptance criteria must include a Rust integration test (or `#[test]` in `db.rs`) that seeds `memory_search_audit` + `audit_returned_facts` + `memory_entries` with a known supersession scenario and asserts the supersession SQL from F-002 `analysis.md` returns the expected rows. This is distinct from the deferred CLI read path — it's a schema correctness gate, not a user-facing command. Without it, the schema ships untested by anything other than the DDL compiling.

No other hidden couplings found. The rename_project/PRAGMA-OFF coupling (R6), Wave 2 call-site migration (R5), and A-MEM volume-metric tolerance argument (Topic 2 rationale) are all correctly surfaced and cross-linked.

**TL disposition**: Plan-time TODO #1 in conclusion's "Plan-time TODOs" section.
