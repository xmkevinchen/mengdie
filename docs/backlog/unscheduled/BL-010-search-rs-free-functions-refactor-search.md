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
