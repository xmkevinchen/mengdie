---
id: BL-010
title: "search.rs free-functions refactor — move impl Db { fn memory_search() } and search_vector (currently impl Db in vector.rs) to module-level free functions over &Db. Establishes Retrieval as a real layer at the type level. Per discussion 028 Topic 1. Wave 2, co-commit with BL-009 (single PR, same mcp_tools↔search boundary)."
status: open
created: 2026-04-28
sprint: v0.0.1
wave: 2
co_commit_with: BL-009
---

# BL-010 — search.rs + search_vector free-functions refactor (co-commit with BL-009)

## Wave 2 preconditions (added 2026-04-29 from F-002 /ae:review)

See BL-009's "Wave 2 preconditions" section — the same three points
apply to BL-010 since BL-009 and BL-010 co-commit. Cross-reference here
to keep both BL bodies aligned without duplicating the full text.

Summary:
1. **FTS-fallback semantic decision** for the new
   `search::memory_search_audited` free function (asymmetry documented
   in F-002 plan Step 3 + Codex Track 4 + challenger Challenge 3).
2. **Audit hook preservation** — `Db::record_search_audit` strict +
   `Db::record_search_audit_best_effort` wrapper from F-002 Step 2 stay
   in place; Wave 2 reuses them unchanged.
3. **Operator-visibility cross-reference**: BL-019 (CLI embed-fail
   visibility gap) status depends on which Wave 2 option is chosen
   (preserve asymmetry vs unify).
