---
id: "014"
title: "CI Runner Env Fix — remove `.cargo/config.toml` unscoped CFLAGS + expand ci.yml"
type: plan
created: 2026-04-22
status: reviewed
discussion: "docs/discussions/020-ci-runner-env-cleanup/"
---

# Feature: CI Runner Env Fix

## Goal

Fix the root cause of the `-isysroot` leak blocking mengdie's CI expansion
(`.cargo/config.toml` unscoped `[env] CFLAGS`) and ship clippy + `cargo test`
jobs in `.forgejo/workflows/ci.yml`, closing both `006-ci-runner-env-cleanup`
and `BL-ci-full-clippy-test` in one plan.

## Prerequisites

- Discussion 020 conclusion (`docs/discussions/020-ci-runner-env-cleanup/conclusion.md`)
  with Doodlestein post-review applied — per-target `CFLAGS_<triple>` is the
  preferred durable form; write `ci.yml` fresh rather than cherry-pick dead
  scaffolding from reverted commits.
- Mac mini Forgejo runner is reachable and healthy (fmt-only CI is currently
  green per runs on `main`).
- `cargo-zigbuild` shims for `aarch64-apple-darwin` in `release.yml` are
  NOT touched by this plan — separate concern.

## Prior Art (from project knowledge base)

- [plan 008, decisional] No `rust-toolchain.toml` pin; `-D warnings --all-targets`
  policy; local pre-commit runs fmt+clippy (not test). CI is the backstop
  for format drift and the uncovered `cargo test` gate.
- [discuss 017, decisional] Host-mode Forgejo runner on `ckai-macmini.local`;
  `container:` directive unavailable; `source ~/.cargo/env` idiom is the
  canonical way to reach the rustup-installed toolchain.
- [discuss 020, decisional] Root cause = `.cargo/config.toml` `[env] CFLAGS`
  at repo root (added 2026-04-16 commit `af303d5`). Per-target
  `CFLAGS_x86_64_apple_darwin` / `CFLAGS_aarch64_apple_darwin` is cc-rs's
  scoping convention and survives cross-compile cleanly.
- [discuss 020, experiential] Generalizable heuristic: grep `.cargo/config.toml`
  `[env]` before assuming shell-level env leakage in Cargo cross-compile
  investigations.

## Steps

### Step 1: Pre-verify CFLAGS necessity, then convert `.cargo/config.toml` to per-target form (AC1, AC2)

Determine whether the unscoped `CFLAGS` line is needed for local macOS builds
at all, then apply the preferred durable form regardless.

- [x] On Mac mini (local, not via CI): `mv .cargo/config.toml /tmp/mengdie-cargo-config.bak` (executed on M4 Max `ckai-m4x`, not Mac mini — same impact for local dev workflow)
- [x] Run `cargo clean && cargo build` — **ring failed: `'TargetConditionals.h' file not found`** (validates need for CFLAGS)
- [x] Run `cargo test` — not executed (build already failed; CFLAGS need confirmed)
- [x] Replace `.cargo/config.toml` with the per-target form below, regardless
      of pre-verify outcome (Doodlestein regret: per-target is preferred as
      the durable shape; if the line turned out unneeded today, future
      dep upgrades will likely re-surface the need):

```toml
# Per-target CFLAGS (cc-rs convention): macOS host builds get the Xcode SDK
# path injected; Linux cross-compile builds see no matching var and ignore this.
#
# DEBUGGING HEURISTIC (from discussion 020, commit af303d5 + 020-conclusion):
# If you hit an unexplained `-isysroot` or other env flag appearing in cc
# invocations during a Cargo build, GREP `.cargo/config.toml` `[env]` FIRST.
# Cargo [env] injects into every build-script subprocess, bypassing shell
# unsets. This file bit mengdie for ~2 days in plan 008 Step 3 before being
# identified as the root cause.
[env]
CFLAGS_x86_64_apple_darwin = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
CFLAGS_aarch64_apple_darwin = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
```

- [x] Verify no `-isysroot` leak to Linux target via **log inspection** — **result: 0 matches** in `/tmp/mengdie-xbuild.log` (build fails at link as expected; cc invocations clean)
- [x] Run `cargo build && cargo test` locally — confirm still green — **228 passed, 5 ignored, 8.64s build**
- [x] Commit the `.cargo/config.toml` change on feature branch `plan-014-ci-fix` — **commit 1780198**

**Expected files**: `.cargo/config.toml`

**Fallback** (if pre-verify fails AND per-target form also fails the Linux-target
log-inspection check): move CFLAGS to Kai's `~/.zshenv` (Fallback B per
conclusion) and delete `.cargo/config.toml`. Under fallback:
- [ ] Smoke-test: re-run `cargo clean && cargo build && cargo test` locally
      AND the Linux-target `-vv | rg -c 'running:.*isysroot'` check (still
      must be zero) BEFORE proceeding to Step 2
- [ ] Document the fallback in commit message + append a line to discussion
      020 conclusion's Doodlestein Review section noting which fallback was taken

### Step 2: Rewrite `.forgejo/workflows/ci.yml` fresh with clippy + cargo test + cross-check (AC3)

Do NOT cherry-pick `e4b8cbf`–`6658248`. Those commits were written against
the now-falsified Apple-Clang hypothesis and contain dead scaffolding
(`CARGO_BUILD_TARGET` exports, inline `unset SDKROOT CFLAGS CXXFLAGS
CPPFLAGS LDFLAGS` blocks, `env: SDKROOT: ''` at job level, `env | sort`
debug step, `CC_x86_64_unknown_linux_gnu` overrides). Use the existing
fmt-only `ci.yml` as the base (44 lines) and add jobs.

- [ ] Remove the inline "Revisit when: the CFLAGS leak source is identified"
      comment block from `ci.yml` (lines 15–32 of current file) — obsolete
- [ ] Add a `clippy` job running `cargo clippy --all-targets -- -D warnings`
      (matches plan 008's `-D warnings --all-targets` policy)
- [ ] Add a `test` job running `cargo test` (on the host, no cross-compile
      needed — the runner is macOS, binary runs there natively)
- [ ] Add a `cross-check` job running `cargo check --target x86_64-unknown-linux-gnu`.
      **Purpose**: mechanical enforcement of the `.cargo/config.toml`
      `[env]` scoping discipline from discussion 020. Compile-phase only
      (no link), so it works on the Mac mini without a Linux linker. If a
      future regression re-adds unscoped `CFLAGS` to `.cargo/config.toml`,
      this job fails at the ring cc step exactly as plan 008's failure did —
      catching the leak automatically rather than relying on developers
      remembering to grep. Covers the monitoring gap flagged by
      dependency-analyst.
- [ ] All four jobs (fmt, clippy, test, cross-check) run on `push` to all
      branches and on `pull_request` (matches current `on:` trigger shape)
- [ ] Each job uses `source ~/.cargo/env` before cargo invocations (plan
      008 host-mode idiom; matches `release.yml`)
- [ ] NO env manipulation (`CARGO_BUILD_TARGET`, `SDKROOT`, `CFLAGS`,
      `CC_x86_64_unknown_linux_gnu`) — the `.cargo/config.toml` fix from
      Step 1 makes all of that unnecessary
- [ ] NO debug/diagnostic steps (`env | sort`, `rustup show`, etc.) — the
      workflow is production, not investigation
- [ ] Run `.githooks/pre-commit` locally before committing to confirm the
      workflow file itself doesn't break fmt/clippy on any workflow-
      adjacent Rust file

**Expected files**: `.forgejo/workflows/ci.yml`

### Step 3: Fix `.forgejo/workflows/release.yml` `test:` / `build-linux:` race (AC4)

The pre-Step-3 shape of `release.yml` shipped on 2026-04-17 has independent
`test:` and `build-linux:` jobs with no `needs:` dependency — release
binaries can ship before/while tests finish. 2-line fix.

- [ ] Add `needs: [test]` to `build-linux:` job (preferred — simpler than
      inlining the gate; keeps the two jobs modular)
- [ ] Verify that adding `needs:` doesn't break the existing `FORGEJO_TOKEN`
      secret scoping or the release asset upload block
- [ ] No other changes to `release.yml` — this is strictly the race fix

**Expected files**: `.forgejo/workflows/release.yml`

### Step 4: Push on feature branch + verify full CI green on runner (AC5)

Trigger the runner to exercise the expanded workflow against the fixed
`.cargo/config.toml`. Use a feature branch throughout (not main) — if CI
fails on the expanded workflow, rollback is a branch delete, not a
forced-push on main.

- [ ] Steps 1–3 should be on a feature branch (e.g., `plan-014-ci-fix`);
      Step 4's trigger push is a push to that branch or a PR open.
      If the feature branch already has a meaningful diff from Steps 1–3,
      no additional trigger commit is needed — pushing the branch triggers
      CI. If not, add a trivial source-file comment touch (exercises
      pre-commit hook path too); revert that touch after verification.
- [ ] Push (or open PR). Record the commit SHA that triggered the run
- [ ] Wait for all four jobs (fmt, clippy, test, cross-check) to complete
      on the Forgejo Actions runner
- [ ] Pull each of the four raw logs and grep. Two executable paths:

      **Path A — UI download** (simplest):
      1. In the Forgejo Actions run page for the triggering commit,
         click each job to open its log view, then click the "Download
         log" icon. Save to `/tmp/mengdie-ci-logs/` with filenames
         `fmt.log`, `clippy.log`, `test.log`, `cross-check.log`
      2. Run: `for f in /tmp/mengdie-ci-logs/*.log; do echo "$f: $(rg -c 'running:.*isysroot' "$f" || echo 0)"; done`
      3. Every line must end in `: 0`

      **Path B — API (if scripting)**: the Forgejo Actions raw log
      endpoint is `GET /api/v1/repos/{owner}/{repo}/actions/tasks/{task_id}/logs`.
      task_id is obtained by first listing runs then listing jobs:
      `GET /api/v1/repos/{owner}/{repo}/actions/runs?head_sha={sha}` →
      `GET /api/v1/repos/{owner}/{repo}/actions/runs/{run_id}/jobs`.
      Auth via `Authorization: token <FORGEJO_TOKEN>` (same token used
      in `release.yml` secrets). Path A is preferred for a one-off
      verification; don't build a script unless this gate is going to
      be repeated.
- [ ] If any job fails (non-success status) or any `rg` returns non-zero:
      record the failure mode (compile error / clippy warn-as-error /
      test failure / cross-check failure / `-isysroot` appearance) and
      execute Rollback. Do NOT proceed to Step 5
- [ ] If all four jobs green AND all four `rg` checks return zero:
      merge the feature branch to main

**Expected files**: (CI verification step — no code changes in this step
unless a trigger commit is needed; the files actually changing are the
ones from Steps 1–3, already committed on the feature branch)

**Monitoring gap note** (resolved by strategic-Doodlestein): the
cross-check job added in Step 2 closes this gap. `cargo check --target
x86_64-unknown-linux-gnu` runs compile-phase only (no link, so no Linux
linker needed on the Mac mini), and it exercises the exact code path
where the original `-isysroot` leak appeared (ring via cc-rs with target=
Linux). A future regression re-adding unscoped `CFLAGS` to
`.cargo/config.toml` will now fail CI's cross-check job automatically,
not silently pass on the native-macOS `test` job.

### Step 5: Close `006-ci-runner-env-cleanup` + `BL-ci-full-clippy-test`; log v0.8.0 scope delta (AC6)

**Dependency**: do NOT execute Step 5 until Step 4's three `rg -c 'running:.*isysroot'` checks all return zero AND all three CI jobs land green. If Step 4 fails, execute Rollback — do not close out.

- [ ] Set `status: done` in `.ae/backlog/v0.8.0/006-ci-runner-env-cleanup.md`
      frontmatter (with a one-line note in the body about the root cause
      being different than documented — `.cargo/config.toml` `[env]`, not
      a runner env issue)
- [ ] Set `status: done` in `.ae/backlog/v0.8.0/BL-ci-full-clippy-test.md`
      frontmatter
- [ ] Append two `close-scope-delta` lines to `.ae/roadmaps/v0.8.0.md`
      `## Notes`:
      ```
      - 2026-04-XX | close-scope-delta | 006-ci-runner-env-cleanup | superseded — root cause was .cargo/config.toml [env] CFLAGS, not runner/act env leak; same fix closed both items
      - 2026-04-XX | close-scope-delta | BL-ci-full-clippy-test | plan 014 expanded ci.yml with clippy + cargo test jobs
      ```
- [ ] Update `docs/discussions/020-ci-runner-env-cleanup/index.md` frontmatter
      `plan:` to `"docs/plans/014-ci-runner-env-fix.md"` and `pipeline.work: done`

**Expected files**: `.ae/backlog/v0.8.0/006-ci-runner-env-cleanup.md`,
`.ae/backlog/v0.8.0/BL-ci-full-clippy-test.md`,
`.ae/roadmaps/v0.8.0.md`,
`docs/discussions/020-ci-runner-env-cleanup/index.md`

## Acceptance Criteria

### AC1: Local `cargo build + cargo test` on Mac mini passes with updated `.cargo/config.toml`

With `.cargo/config.toml` converted to per-target form (or deleted under the
fallback), running `cargo build && cargo test` on Mac mini produces no
errors. Exercises `ring`, `libsqlite3-sys`, and any other cc-using crates
in the dep tree via their normal build paths.

**Verification**: on Mac mini, `cargo clean && cargo build && cargo test`
exits 0 and all tests pass.

### AC2: Linux cross-compile locally shows no `-isysroot` injection into cc invocations

With the per-target `CFLAGS_<triple>` form in place, a verbose Linux-target
build on the Mac mini emits zero `-isysroot` strings on cc invocation lines.
(Note: the Linux-target build may still fail to link on macOS without a
cross-linker — link failure is acceptable for this check; we only care
about the cc invocation contents, which come BEFORE linking.)

**Verification**: on Mac mini,
`cargo clean && cargo build --target x86_64-unknown-linux-gnu -vv 2>&1 | tee /tmp/mengdie-xbuild.log; rg -c 'running:.*isysroot' /tmp/mengdie-xbuild.log`
outputs `0`. (If non-zero, Step 1 fix didn't hold; do NOT proceed.)
cc-rs emits `running: "cc" ... "-isysroot" ...` lines before attempting
compilation, so the grep captures the flag regardless of downstream link success.

### AC3: `ci.yml` runs fmt + clippy + cargo test + cross-check on push and PR, with no dead scaffolding

The shipped `.forgejo/workflows/ci.yml`:
- Runs on `push` (all branches) and `pull_request`
- Has four job names matching `fmt`, `clippy`, `test`, `cross-check`
  (or similar)
- Each job's body invokes `source ~/.cargo/env` followed by the respective
  cargo subcommand
- Clippy job uses `cargo clippy --all-targets -- -D warnings` exactly
- Test job uses `cargo test` (no `--no-run`, no target flag — runs on
  macOS host natively)
- Cross-check job uses `cargo check --target x86_64-unknown-linux-gnu`
  (compile-phase only; mechanical `.cargo/config.toml` regression guard)
- Contains no dead scaffolding from the pre-Step-3 reverted commits:
  no `CARGO_BUILD_TARGET` export, no `SDKROOT=` or `unset SDKROOT`,
  no `CFLAGS=` / `CXXFLAGS=` / `CPPFLAGS=` / `LDFLAGS=` manipulation,
  no `unset CFLAGS`, no `CC_x86_64_unknown_linux_gnu` override, no
  `env | sort` debug step, no `rustup show` as a debug step

**Verification**: inspect `.forgejo/workflows/ci.yml` — no executable
lines (i.e., not lines inside YAML `#` comments) contain any of:
`CARGO_BUILD_TARGET`, `SDKROOT`, `unset `, `CC_x86_64_unknown_linux_gnu`,
`env | sort`. Explanatory comments referencing these names by way of
"why removed" are fine and expected; the ban is on active use.

### AC4: `release.yml` `build-linux:` job depends on `test:` via `needs:`

The shipped `.forgejo/workflows/release.yml` `build-linux:` job has
`needs: [test]` set. On a tag push, `build-linux:` cannot start until
`test:` completes successfully.

**Verification**: `rg -A1 'build-linux:' .forgejo/workflows/release.yml`
shows `needs: [test]` (or equivalent YAML list form) in the next
non-blank lines.

### AC5: Triggered CI run on the real runner is green end-to-end + no `-isysroot` in any job log

A real commit pushed to a feature branch triggers the expanded `ci.yml`.
All four jobs (fmt, clippy, test, cross-check) land green on the Mac mini
runner. For each job, its log contains zero `running:.*isysroot` matches.

The cross-check job is the primary mechanical guard against `.cargo/config.toml`
`[env]` regressions: if it fails with `-isysroot` in its log, the scoping
discipline has been violated (or ring/cc-rs behavior has changed in a way
that re-triggers the leak).

**Verification**: for each of the four jobs from the triggering commit:
1. Pull the raw job log from the Forgejo Actions UI or API
2. Run `rg -c 'running:.*isysroot' <job-log-file>` — must output `0`
3. Confirm job status is `success` via the UI or API

All four jobs must pass both checks (status + grep). Any failure
triggers Rollback.

### AC6: Backlog + roadmap state is consistent post-plan

- `.ae/backlog/v0.8.0/006-ci-runner-env-cleanup.md` frontmatter has
  `status: done`
- `.ae/backlog/v0.8.0/BL-ci-full-clippy-test.md` frontmatter has
  `status: done`
- `.ae/roadmaps/v0.8.0.md` `## Notes` contains exactly two new entries
  with action `close-scope-delta` (one per BL)
- `docs/discussions/020-ci-runner-env-cleanup/index.md` frontmatter has
  `plan: "docs/plans/014-ci-runner-env-fix.md"` and `pipeline.work: done`

**Verification**: read each file, grep the expected fields/entries,
confirm presence.

## Out of Scope

- CI target matrix changes (Linux-x86_64 only per discussion 017 — unchanged)
- Moving the runner off `ckai-macmini.local` — discussion 020 rejected
- Docker executor or zigbuild-as-compiler-replacement — discussion 020 rejected
- `cargo-zigbuild` configuration for `aarch64-apple-darwin` release builds
  in `release.yml` — not touched by this plan
- Security audit of the runner host

## Rollback

If Step 4 fails (jobs red or any `rg -c 'running:.*isysroot'` non-zero)
and the failure is not obviously fixable in-plan:

1. Do NOT merge the feature branch to main. Delete it locally + remotely:
   `git branch -D plan-014-ci-fix && git push origin --delete plan-014-ci-fix`
2. Keep the Step 1 `.cargo/config.toml` per-target form on main regardless
   — it's a correctness improvement for local dev whether or not the CI
   expansion ships. Cherry-pick it onto main separately if not already there.
3. Re-open `006-ci-runner-env-cleanup` + `BL-ci-full-clippy-test`; append
   a failure-mode note to the discussion 020 conclusion's Doodlestein
   Review section (document which job failed, what `rg isysroot` returned,
   what log content was unexpected)
4. Plan status → `cancelled` (not `done`). v0.8.0 `## Notes` entries NOT
   written (Step 5 didn't execute)

**Note**: because Steps 1-3 live on a feature branch (not main), rollback
is a branch delete, not a history rewrite. Green CI on main is never
disrupted by a failed attempt.
