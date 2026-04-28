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
