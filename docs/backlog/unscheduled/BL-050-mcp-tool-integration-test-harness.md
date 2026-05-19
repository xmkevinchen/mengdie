---
id: BL-050
title: "MCP tool integration test harness — exercise MengdieServer dispatch paths from tests/"
status: open
created: 2026-05-18
origin: "F-009 review (challenger finding 5 + codex finding 5): the new memory_invalidate prefix dispatch has 5 distinct code paths (fast UUID / unique prefix / no match / collision / DB error), none exercised by integration tests because tests/e2e.rs goes through db::* layer, not the McpServer::invalidate async method. AC3 'Verify' clause explicitly required integration test for all 3 prefix paths; deferred at F-009 ship with the note that the harness doesn't exist."
size: M
depends_on: []
v_target: "v0.0.2 cross-cutting — applies to F-008 (memory_lint) / F-010 (memory_get) / F-011 (memory_status) too"
---

# BL-050 — MCP tool integration test harness

## Origin

F-009 review surfaced that the new MCP-layer code paths (`memory_invalidate` prefix dispatch with 5 distinct branches) are unreachable from `tests/e2e.rs` — that file uses `db::memory_search` and `ingest_file` directly, never goes through `mcp_tools::MengdieServer::invalidate`. AC3 "Verify" clause required integration tests; they were deferred citing absence of harness.

The same gap will block F-008 (`memory_lint` MCP tool), F-010 (`memory_get` MCP tool), F-011 (`memory_status` MCP tool) — each adds new MCP-dispatch code paths with similar lack of test coverage at the deployment layer.

## Scope

Build a reusable integration-test helper that:

1. Constructs a `MengdieServer` instance against an in-memory `Db` (Embedder optional — if a test doesn't need embedding it should be skippable to avoid the ~90MB model download cost; mark embedding-dependent tests `#[ignore]` like the existing e2e suite).
2. Provides a typed wrapper for calling each MCP tool synchronously from test code: e.g., `harness.invalidate(InvalidateParams { entry_id, reason, superseded_by })` returns the parsed `InvalidateOutput`.
3. Lives at `tests/mcp_harness.rs` (common module) + `tests/mcp_invalidate.rs`, `tests/mcp_search.rs`, etc. (per-tool test files).

## Acceptance criteria

1. `tests/mcp_harness.rs` exists and exposes a `Harness` struct with helper methods for `search`, `ingest`, `invalidate` (and `get` / `status` / `lint` as those land).
2. F-009 retroactive coverage: `tests/mcp_invalidate.rs` exercises all 5 prefix dispatch paths (fast UUID / unique prefix / no match / collision / DB error). Each branch asserts the error message format exactly.
3. Existing `tests/e2e.rs` is unchanged — no regression, harness is additive.
4. Embedder-required tests cleanly opt in via `#[ignore]` (skipped by default, run via `cargo test -- --ignored`).
5. Cross-tool reuse: F-008/F-010/F-011 plans cite this harness and use it for their MCP-tool integration tests.

## Trigger

**Ready now.** F-009 ship already cited this as a follow-up. Should ship before F-010 / F-011 to set the integration-test pattern before more MCP tools accrete coverage debt.

## Out of scope

- Full MCP stdio transport round-trip (would require spawning the binary; serde + handler-layer testing is sufficient for tool behavior).
- Concurrent-call testing (mengdie is currently single-call serialized via `Arc<Mutex<Db>>`).
- Property-based testing (defer to a later iteration once shape is stable).

## Coordination notes

- Sequencing: ship before F-010 begins so its plan can include integration tests from day one. Cheap retrofit on F-009 once harness lands.
- No new Cargo dep needed; uses existing `tokio` + `rmcp::Parameters` types.
