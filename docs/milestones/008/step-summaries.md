# Plan 008 — Step Summaries

## Step 1 — Big-bang clippy cleanup (commit: 0adfe37)
**Decisions**:
- Wrote the 2 new `project.rs` regression tests BEFORE the `manual_strip` refactor (TDD discipline) — captures the "parser is intentionally NOT escape-aware" behavior so any future mechanical clippy refactor cannot silently "fix" it.
- `collapsible_match` in `cli.rs`: collapsed to single `if let Some(rusqlite::Error::SqliteFailure(ffi_err, _)) = cause.downcast_ref::<rusqlite::Error>()` — reads clearly, no `#[allow]` needed.
- `cargo fmt --all` was run per the plan; it reformatted 9 additional files beyond the 6 Expected (pure whitespace / import-grouping changes, no logic). Accepted as approved drift because without it, CI's `cargo fmt --check` would fail on the untouched files.

**Rejected**:
- Splitting into two commits (logic fixes + fmt sweep): plan says "one atomic commit". Intermediate state would fail fmt-check anyway, so no benefit.
- Using `#[allow(clippy::collapsible_match)]` for the cli.rs pattern: reviewers had flagged this as a last-resort option. Not needed — the merged pattern reads cleanly. Zero new `#[allow]` in the diff confirmed.

**Cross-step deps**:
- Clippy + fmt baseline is now clean — Step 2 (pre-commit hook) and Step 3 (CI) can both run `cargo clippy --all-targets -- -D warnings` as a hard gate without pre-existing noise.
- 132 tests passing (was 130; +2 regression guards for `read_project_name`).

**Actual files** (18 total — 6 Expected + 9 fmt-sweep + 3 unrelated-untracked accidentally picked up by `git add -A`):
- Expected: src/bin/cli.rs, src/core/embeddings.rs, src/core/db.rs, src/core/schema.rs, src/core/search.rs, src/core/project.rs
- fmt-sweep (whitespace only): src/core/config.rs, src/core/contradiction.rs, src/core/dreaming.rs, src/core/ingest.rs, src/core/llm.rs, src/core/mcp_tools.rs, src/core/parser.rs, src/core/vector.rs, tests/e2e.rs
- Incidentally captured (pre-existing untracked, predate session): docs/discussions/005-hybrid-search-analysis/analysis.md, docs/discussions/005-hybrid-search-analysis/index.md, docs/milestones/002-close-the-loop/step-summaries.md

Note: use explicit file paths (not `git add -A`) on future commits to avoid scope bleed.

**Allow-audit baseline**: `rg '#\[allow' src/ tests/` returns 0 matches. Clean starting point for Step 4's monitor.
