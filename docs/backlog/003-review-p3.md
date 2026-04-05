---
id: "003"
title: "Deep review P3 findings"
status: open
created: 2026-04-05
tags: [review, p3, deferred]
---

# Deep Review P3 Findings

From /ae:review of Plan 001 (MVP Phase 1).

| ID | Status | Issue | Source | Trigger |
|----|--------|-------|--------|---------|
| P3-1 | open | Path traversal via symlink in parser: `parse_ae_file` reads path from watcher without verifying it's under expected base directory. | Security reviewer | Daemon integration (Phase 2) |
| P3-2 | ✅ fixed | `source_type`/`knowledge_type` validated in MCP ingest tool. Unknown values normalized with warning log. | Security reviewer | — |
| P3-3 | open | User input may appear in tracing logs: error messages transitively include query text. | Security reviewer | Logs shipped to external service |
| P3-4 | ✅ fixed | E2E test now `#[ignore]`. Run with `cargo test --test e2e -- --ignored`. | Architecture reviewer | — |
| P3-5 | ✅ fixed | Dead snippet variable in FTS fallback path (`mcp_tools.rs`). | Architecture reviewer | — |
| P3-6 | ✅ fixed | Replaced with `walkdir` crate (`follow_links(false)`), handles cycles safely. | Architecture reviewer | — |
