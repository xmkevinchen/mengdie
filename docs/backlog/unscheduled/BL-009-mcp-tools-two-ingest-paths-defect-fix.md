---
id: BL-009
title: "mcp_tools two-ingest-paths defect fix — consolidate the inline reimplementation at mcp_tools.rs:306-331 to call ingest::ingest_document. CLI/watcher path uses ingest::ingest_file with content-hash dedup; MCP path currently bypasses this. Per discussion 028 archaeologist finding. Wave 2, co-commit with BL-010."
status: open
created: 2026-04-28
sprint: v0.0.1
wave: 2
co_commit_with: BL-010
---

# BL-009 — mcp_tools two-ingest-paths defect fix (co-commit with BL-010)

## Wave 2 preconditions (added 2026-04-29 from F-002 /ae:review)

When BL-009/BL-010 are picked up, the implementing plan MUST explicitly
resolve the following preconditions surfaced during F-002 (Wave 1) work:

1. **FTS-fallback semantic decision** (Codex Track 4 + challenger
   Challenge 3 in F-002 /ae:review): the new `search::memory_search_audited`
   free function will own embedding generation and search-result construction.
   Today the two Wave 1 call sites have **asymmetric** embed-failure
   behavior — `mcp_tools::search` falls through to FTS-only and audits
   the FTS results; `cli::cmd_search` propagates the embedding error
   via `?` and writes no audit row. The Wave 2 refactor must decide:
   - **Option A**: preserve MCP's FTS-fallback path inside the free
     function, and align CLI to the same behavior. Audit rows fire for
     both surfaces on embed-fail.
   - **Option B**: simplify to CLI's "embed-fail = error" semantic.
     MCP loses its FTS-fallback safety net for embed outages.
   Plan F-002 R5 says "schema unchanged; pure code-move" — but the
   semantic of the moved code is NOT free. Decide explicitly before
   implementation; don't inherit accident.
2. **Audit hook preservation** (F-002 plan R2 + R5): `Db::record_search_audit`
   strict helper and `Db::record_search_audit_best_effort` wrapper from
   F-002 Step 2 stay in place — Wave 2 reuses them unchanged. The
   `returned_fact_ids` extraction from post-filter results must be
   preserved in the free function. Refer to F-002 plan Step 3 for the
   M1-ownership concern.
3. **Operator-visibility for CLI embed-fail** (BL-019): if Option A is
   chosen and audit rows fire on CLI embed-fail too, BL-019 may be
   superseded by Wave 2. If Option B, BL-019 stays open as the
   operator-visibility gap. Wave 2 plan should explicitly note
   which BL state results.
