---
id: "003"
title: "Deep review P3 findings (plan 001 rollup)"
status: closed
closed: 2026-04-19
closed_reason: "Rollup archive: 4 of 6 items fixed in Phase 1. 2 dormant triggers remain: P3-1 (path traversal via symlink — daemon/Phase 2 trigger, not live because ingest is CLI-only today), P3-3 (user input in tracing logs — external-log-shipping trigger, not live because logs are stderr-only). If either trigger fires, promote to BL-*.md with the specific threat model. Rollup retained as-is."
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
