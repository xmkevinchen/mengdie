---
id: BL-051
title: "test-utils feature gate — refactor F-013 test API leaks for library-publication readiness"
status: open
created: 2026-05-19
origin: "F-013 review (challenger F1 + F3 + codex echo): F-013 bumped 4 MengdieServer tool methods from private to pub, and exposed Db::insert_memory_with_id as pub #[doc(hidden)]. Both choices are pragmatic for mengdie as bin-only but become public-API leaks the moment mengdie ships as a library crate."
size: S
depends_on: []
v_target: "trigger-fired (do NOT do until trigger; see Trigger section)"
---

# BL-051 — test-utils feature gate for test API leaks

## Origin

F-013 (MCP integration test harness) made two structural changes that expanded the public Rust API surface specifically for test access:

1. **4 `MengdieServer` tool methods bumped from private to `pub`** (`search`, `ingest`, `get`, `invalidate`). `#[tool_router]` macro dispatch is unchanged by visibility, so production MCP routing is unaffected; but the methods are now part of the crate's public surface forever (binary compat / semver concern in a library context).
2. **`Db::insert_memory_with_id`** added as `pub #[doc(hidden)]`. Integration tests in `tests/` cannot reach `#[cfg(test)]` items in `src/`, so the workaround exposes the function publicly with only a docs-hidden signal as the "don't use in production" guard. Compile checker doesn't enforce; IDE auto-complete still shows the function.

Both choices were the right pragmatic call for mengdie's current state (bin-only, single-operator, no published crate). Both become library-publication concerns.

## Scope

Refactor both test-API leaks behind a `test-utils` cargo feature flag:

1. **Cargo.toml**: add
   ```toml
   [features]
   test-utils = []
   ```

2. **`src/core/mcp_tools.rs`**: gate the 4 method `pub` annotations:
   ```rust
   #[cfg(feature = "test-utils")]
   pub async fn search(&self, ...) -> Json<SearchOutput> { ... }
   #[cfg(not(feature = "test-utils"))]
   async fn search(&self, ...) -> Json<SearchOutput> { ... }
   ```
   Or — cleaner — use `pub(crate)` if all in-tree callers can reach it (the macro-generated `tool_router` likely can; verify before committing to this path).

3. **`src/core/db.rs`**: gate `insert_memory_with_id` behind the feature:
   ```rust
   #[cfg(feature = "test-utils")]
   #[doc(hidden)]
   pub fn insert_memory_with_id(&self, id: &str, mem: NewMemory) -> Result<String> { ... }
   ```

4. **Cargo.toml `[dev-dependencies]`**: enable the feature for integration tests:
   ```toml
   [dev-dependencies]
   mengdie = { path = ".", features = ["test-utils"] }
   ```
   Or equivalent — verify the right invocation for `tests/` to opt into the feature.

5. **Test harness**: no change needed; it already calls the methods, just under a different feature gate.

## Acceptance criteria

1. `cargo build` (no features) compiles a binary where `MengdieServer::search` etc are **not** public — verified by attempting `use mengdie::core::mcp_tools::MengdieServer; let _ = server.search;` in a fresh crate and watching the compile fail.
2. `cargo test` (which auto-enables dev features) continues to pass all 334 tests including F-013 integration tests.
3. `cargo clippy --all-targets` clean.
4. F-013 harness code unchanged; only the upstream `pub` annotations are gated.
5. `Db::insert_memory_with_id` no longer present in the default-features API surface (verified same as #1).

## Trigger

**Do NOT do this until one of the following fires**:

1. **mengdie is published as a library crate** (e.g., `cargo publish` happens, or external Rust code starts depending on the `mengdie` crate as a path dep / git dep with the intent of consuming the library surface).
2. **A real production caller of `Db::insert_memory_with_id` appears in the codebase** — verified by `git log --all -S 'insert_memory_with_id'` showing usage outside `tests/` or `src/core/db.rs` (the original definition site).
3. **A real production caller of `MengdieServer::search`/`ingest`/`get`/`invalidate` as a direct method (not through the tool_router) appears** — verified similarly.

Until one of these fires, the current state is acceptable: bin-only crate with documented test workaround. Karpathy "don't fix what isn't broken".

## Non-goals

- Per-method test wrappers (`MengdieServer::invoke_search` etc) — over-engineered for current scale.
- Moving test helpers to a separate sub-crate (`mengdie-test-utils`) — heavier refactor than the feature-gate approach; only worth it if mengdie grows multiple library subsystems.
- Refactoring `MengdieServer::new` to accept `Arc<Mutex<Embedder>>` for embedder sharing across Harness instances — orthogonal concern; per-Harness load is sub-second post-cache; only fix if test load times become painful.

## Estimated size

S (~50 LoC across Cargo.toml + db.rs + mcp_tools.rs annotations + verification tests). Trigger-deferred; do NOT pre-promote.
