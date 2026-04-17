---
id: "008"
title: "CI Pipeline + Lint Debt Cleanup"
type: plan
created: 2026-04-17
status: reviewed
discussion: "docs/discussions/017-ci-pipeline-and-lint-debt/"
---

# Feature: CI Pipeline + Lint Debt Cleanup

## Goal

Close the "commits land untested" gap: clean the pre-existing clippy debt, install a local pre-commit hook that blocks unclean commits from entering the graph, and ship a CI workflow that runs fmt/clippy/test on every push and PR on the Forgejo Ubuntu runner.

## Scope boundaries

- **In**: 10 clippy fixes (9 warnings + 1 hard error), `.githooks/pre-commit`, `git config core.hooksPath` one-time install documented, `.forgejo/workflows/ci.yml`, trim `release.yml` test job once ci.yml proves green.
- **Out**: `cargo audit` (deferred), Mac runner, Windows support, `rust-toolchain.toml` (explicitly rejected in addendum), `rust-version` MSRV (explicitly rejected), `rustfmt.toml` (no style opinions yet), pre-commit framework dependencies (husky/pre-commit.com — not using).
- **`#[allow(...)]` policy**: last resort, not default. Reach order = fix code first, `#[allow]` only on genuine false positives with an inline comment explaining why. Zero new `#[allow]` added during Step 1.

## Steps

### Step 1: Big-bang clippy cleanup (AC1) — DONE 0adfe37

Fix all 10 clippy items surveyed by rust-archaeologist during discussion 017. One atomic commit. Tests must stay green throughout. **Zero new `#[allow(...)]` attributes** — every item gets a real fix.

- [x] **Hard error** — `src/core/embeddings.rs:116`: replace `3.14159` literal with `std::f32::consts::PI` in the test vec.
- [x] **Trivial** — `src/core/embeddings.rs:75`: `blob.len() % 4 == 0` → `blob.len().is_multiple_of(4)`.
- [x] **Trivial** — `src/core/db.rs:146`: remove redundant closure `|row| row_to_entry(row)` → `row_to_entry`.
- [x] **Trivial** — `src/core/db.rs:296`: same redundant closure pattern → remove.
- [x] **Trivial** — `src/core/schema.rs:122`: collapse nested `if current_version < 3 { if !column_exists(...) {` into `&&` form.
- [x] **Trivial** — `src/core/search.rs:141`: `.min(1.0).max(0.0)` → `.clamp(0.0, 1.0)`.
- [x] **Trivial** — `src/core/search.rs:270`: `results[0].id.len() > 0` → `!results[0].id.is_empty()`.
- [x] **Trivial** — `src/bin/cli.rs:309`: `println!("... {}", "Source")` with empty format string → inline `"Source"` literal.
- [x] **Needs-thought** — `src/core/project.rs:58/60/61/62`: 4 × `manual_strip`. Current code: `if val.starts_with('"')` then slice `val[1..]` + scan ahead for closing quote. Clippy's direct suggestion (`strip_prefix('"')`) is NOT a drop-in — it removes the leading quote but doesn't find the closing quote. Dependency-analyst verified: the correct refactor is `strip_prefix('"').and_then(|s| s.find('"').map(|end| &s[..end]))` (analogously for `'`). Must preserve behavior exactly.
- [x] **New unit test (required, not optional)**: Add a test covering the TOML-ish quoted-value parse path in `project.rs` with at least one input containing an escaped quote inside the value (e.g., `name = "foo\"bar"`) and one Unicode input. Write the test BEFORE the refactor, capture current output, run AFTER the refactor, assert identical output. This test is load-bearing for the `manual_strip` change — do not skip.
- [x] **Needs-thought** — `src/bin/cli.rs:403/404`: 2 × `collapsible_match`. Nested `if let Some(rusqlite_err) = ... { if let SqliteFailure(...) = rusqlite_err {`. Collapsing flattens the intermediate binding. Confirm the merged pattern reads clearly; if not, add `#[allow(clippy::collapsible_match)]` with comment only as last resort.
- [x] Run `cargo fmt --all` to normalize formatting across touched files.
- [x] Verification: `cargo clippy --all-targets -- -D warnings` exits 0. `cargo test` still passes (128+ tests). No new `#[allow(...)]` in the diff.

Expected files: `src/core/embeddings.rs`, `src/core/db.rs`, `src/core/schema.rs`, `src/core/search.rs`, `src/core/project.rs`, `src/bin/cli.rs`

### Step 2: Local pre-commit hook (AC2)

Add a committed hook script + one-line setup. Hook runs the fast checks only (fmt + clippy). Test is CI's job — pre-commit must not take >10s warm.

- [ ] Create `.githooks/pre-commit` (executable shell script, `chmod +x`):
  - Run `cargo fmt --all -- --check` — fail with clear message on format drift
  - Run `cargo clippy --all-targets --quiet -- -D warnings` — fail on any warning
  - Do NOT run `cargo test` (fastembed cold start would make commits unbearable; CI covers test)
  - Exit non-zero on any failure so the commit is rejected
- [ ] Create `.githooks/README.md` (one paragraph):
  - Explains the hook purpose
  - Install command: `git config core.hooksPath .githooks`
  - Note: this is a one-time per clone; not auto-applied by `git clone`
  - Note: per project CLAUDE.md, `--no-verify` is NOT a normal escape hatch — fix the issue, don't skip
- [ ] Update project CLAUDE.md "Development" section to mention running `git config core.hooksPath .githooks` once after clone.
- [ ] Verification: after running `git config core.hooksPath .githooks`, introduce a deliberate clippy violation (e.g., `.len() > 0` somewhere), attempt `git commit`, confirm commit is rejected with the clippy output. Revert the test violation.

Expected files: `.githooks/pre-commit`, `.githooks/README.md`, `CLAUDE.md`

### Step 3: CI workflow + release.yml trim (AC3)

Ship the push/PR CI workflow and retire the redundant release-time test job. Both in one atomic commit since they're coupled (release.yml test removal is only safe once ci.yml covers the path).

- [ ] Create `.forgejo/workflows/ci.yml` with the following exact shape:
  - `name: CI`
  - Triggers — use this exact YAML (per codex + dependency-analyst verification: `branches: ['**']` inherently excludes tag refs; no separate `tags-ignore` needed):
    ```yaml
    on:
      push:
        branches:
          - '**'
      pull_request:
        types:
          - opened
          - synchronize
          - reopened
    ```
  - Single job `ci:` on `runs-on: ubuntu-latest`
  - Step sequence:
    1. `uses: actions/checkout@v4`
    2. `actions/cache@v4` restore (see cache spec below)
    3. **Components check** — `source ~/.cargo/env && rustup component add rustfmt clippy` (idempotent — ensures the runner has `rustfmt` and `clippy` binaries; does NOT pin a toolchain version, does NOT mutate the default channel. This is required because we cannot assume the runner was provisioned with those components.)
    4. **Version log** — `rustc --version && cargo fmt --version && cargo clippy --version` (sanity log for debugging, not a guard)
    5. `cargo fmt --all -- --check`
    6. `cargo clippy --all-targets -- -D warnings`
    7. `cargo test`
  - `actions/cache@v4` paths: `~/.cargo/registry`, `~/.cargo/git`, `target`, `~/.cache/fastembed`
  - Cache key with `restore-keys` fallback (per codex consider — improves hit rate on Cargo.lock bumps):
    ```yaml
    key: rust-cache-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      rust-cache-${{ runner.os }}-
    ```
    The `runner.os` prefix is defensive against a future second runner (Mac/Windows) — cheap insurance, no effect today.
  - Bare `actions/*` syntax matching release.yml (works via DEFAULT_ACTIONS_URL).
  - Do NOT add `rustup show`, `rust-toolchain.toml`, `rustup default stable`, or any toolchain-pinning/mutation step. We rely on the runner's pre-installed stable channel AND the component-add step above.
- [ ] Edit `.forgejo/workflows/release.yml` — TWO changes:
  - **Remove the standalone `test:` job entirely** (the second job in the file). That coverage moves to ci.yml's push trigger, which runs on the commit that was subsequently tagged.
  - **Add belt-and-braces gate to `build-linux:`** — put `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` as the FIRST three run steps of `build-linux:`, before `cargo build --release`. Rationale (codex must-fix): release.yml's `test:` and `build-linux:` are currently independent jobs (no `needs:` dep), so even today an untested release could ship if jobs run concurrently. Removing `test:` without adding this gate widens that race; adding the gate inside `build-linux:` closes it AND makes the workflow self-contained (release can't produce an unvalidated binary even if ci.yml wasn't run on the tagged commit for any reason).
- [ ] Verification:
  - Push this commit to a non-main branch. CI workflow runs, all three checks pass.
  - Push a commit with a deliberate clippy violation on a throwaway branch. CI fails on the clippy step. Revert.
  - Cut a throwaway tag `v0.0.0-test-ci` to verify release.yml still fires without its test job and that ci.yml does NOT also fire (or: verify both fire; if tag double-runs ci.yml, the branches filter needs adjusting). Delete the test tag.

Expected files: `.forgejo/workflows/ci.yml`, `.forgejo/workflows/release.yml`

### Step 4: Monitor + `#[allow]` audit (AC4)

First-run observations AND ongoing `#[allow]` discipline. Not code changes — a monitoring commitment with a concrete artifact.

- [ ] Watch the first 3–5 CI runs in the Forgejo web UI. Record for each in `docs/milestones/008/notes.md`:
  - Total wall time
  - Cache hit/miss for `~/.cache/fastembed` (should be miss on run 1, hit on runs 2+)
  - Cache hit/miss for `target`
  - Any spurious failures
- [ ] If fastembed cache is NOT hit on subsequent runs, diagnose: is the cache key wrong? Is the path wrong (might be `~/.cache/fastembed/models` or similar — inspect the runner filesystem post-run)? Record findings.
- [ ] If CI wall time > 5 min warm, investigate and document. Record findings in notes.md.
- [ ] **`#[allow(clippy::*)]` audit** — baseline: 0 matches in `src/` after Step 1 (verified by `rg '#\[allow' src/`). Audit procedure going forward:
  - After every commit that changes the `#[allow]` count (up or down), note the change in notes.md with the file:line and a one-line justification copy-pasted from the `#[allow]`'s inline comment
  - Monthly: run `rg -n '#\[allow' src/ tests/` and review each hit. Any `#[allow]` without an inline comment explaining WHY = must be fixed or commented immediately.
- [ ] **Escalation trigger** (not a hard threshold — a prompt to reopen the question): if `#[allow]` additions exceed 3 within a 30-day window OR 20-commit window (whichever comes first), open a follow-up `/ae:discuss` to decide: (a) fix the lint-violating patterns properly, (b) disable specific lints project-wide via `rustfmt.toml`/`clippy.toml`, or (c) accept a documented subset as permanent exceptions. Do NOT silently let the count grow.

Expected files: `docs/milestones/008/notes.md`

Parallel strategy:
- Step 1 is standalone (just code fixes), must complete before any CI that enforces `-D warnings`.
- Step 2 (pre-commit hook) is independent of Step 1 in the code sense but only useful AFTER Step 1 because otherwise the hook itself would fail immediately on existing warnings.
- Step 3 depends on Step 1 (clippy must pass) but not on Step 2. Can run in parallel with Step 2 writing.
- Step 4 strictly serial — needs 1, 2, 3 done and commits pushed to observe real CI runs.

Recommended execution order: 1 → 2 → 3 → 4. No parallelism attempted; the total code change is small and ordering keeps verification clean.

## Acceptance Criteria

### AC1: Clippy debt fully resolved
- `cargo clippy --all-targets -- -D warnings` exits 0 after Step 1 (was: 1 hard error + 9 warnings before)
- `cargo test` still passes all tests (expected count: ≥128, whatever the current lib/test count is)
- `cargo fmt --all -- --check` exits 0 (touched files formatted consistently)
- Zero new `#[allow(clippy::*)]` or `#[allow(dead_code)]` attributes added in this step's diff — verified by `git diff | grep -E '^\+.*#\[allow' | grep -v '^+++'` returning empty
- **A dedicated unit test exists in `src/core/project.rs`** (or `tests/project.rs`) asserting that the TOML-ish quoted-value parser returns the correct substring for (a) an input with an escaped quote inside the value and (b) a Unicode input. The test exists and passes both BEFORE the `manual_strip` refactor (as a regression guard captured against current behavior) AND AFTER the refactor (same expected output). No "if missing" wiggle — write the test even if loose coverage exists elsewhere.

### AC2: Pre-commit hook blocks dirty commits
- After running `git config core.hooksPath .githooks`, attempting to `git commit` a file with a deliberate clippy violation (e.g., `let _ = v.len() > 0;` in any source file) is rejected
- The rejection message includes the clippy output so the user knows what failed
- Commit time on a clean cached workspace: <5 seconds warm (dependency-analyst-2 measured <1s on this codebase; record actual number in the Step 2 commit message)
- A clean commit (no fmt drift, no clippy warnings) is accepted normally
- `CLAUDE.md` has a one-line note about running `git config core.hooksPath .githooks` after clone
- `.githooks/README.md` exists and contains (a) the install command `git config core.hooksPath .githooks`, (b) a note that the hook runs `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` (NOT `cargo test`), (c) a reminder that `--no-verify` is not a normal escape hatch per project CLAUDE.md

### AC3: CI workflow runs on push + PR, release workflow self-gates, no races
- Pushing any non-tag branch triggers `.forgejo/workflows/ci.yml`
- A pushed commit that passes local hooks also passes CI (verified with the first Step 3 push)
- A pushed commit with a clippy violation fails CI at the clippy step (verified manually — push to throwaway branch, observe red, revert)
- A pushed tag `v*` fires `release.yml` but does NOT fire `ci.yml` (`branches: ['**']` inherently excludes tag refs per dependency-analyst verification)
- `release.yml` no longer contains a standalone `test:` job (verified by `grep -c '^  test:' .forgejo/workflows/release.yml` returning 0)
- `release.yml`'s `build-linux:` job has `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` as its first three run steps BEFORE `cargo build --release` (verified by reading the file — belt-and-braces against the race window codex flagged)
- `rustup component add rustfmt clippy` step is present in ci.yml BEFORE any fmt/clippy invocation (verified — idempotent component install, no toolchain mutation)

### AC4: Cache + timing behavior is acceptable
- After 2 CI runs, the `~/.cache/fastembed` cache shows a cache hit on run 2 (Forgejo UI reports this)
- CI wall time on a cache hit: <3 minutes for a typical commit (fmt + clippy + test, no fastembed re-download)
- CI wall time on cache miss (run 1): <8 minutes (includes fastembed model download + full target build)
- `docs/milestones/008/notes.md` has observation entries for at least 3 runs

## Non-goals (explicit)

- No `rust-toolchain.toml`, no `rust-version` in Cargo.toml — explicitly rejected in the conclusion addendum.
- No `rustfmt.toml` — default rustfmt is fine; add opinions only when we have opinions.
- No `#[allow(...)]` additions in Step 1. `#[allow]` is a last-resort escape hatch, not the default way to silence clippy. If Step 1's needs-thought items (`manual_strip`, `collapsible_match`) cannot be cleanly fixed, re-examine the lint's intent and write a real fix; only if that fails, add `#[allow]` with an explaining comment AND flag it in the Step 1 commit message for reviewer attention.
- No `cargo audit`, `cargo deny`, or SBOM generation in CI — deferred.
- No pre-commit framework (husky, pre-commit.com, lefthook) — a shell script in `.githooks/` is sufficient and has zero dependencies.
- No Mac mini runner, no Windows runner.
- No changes to `release.yml`'s build/release steps — only the redundant `test:` job is removed.
