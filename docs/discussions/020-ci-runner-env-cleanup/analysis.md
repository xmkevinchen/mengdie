---
id: "020"
title: "Analysis: CI Runner Env Cleanup — root-cause or bypass the -isysroot leak"
type: analysis
created: 2026-04-22
tags: [ci, forgejo, act, ring, cc-rs, runner-env, isysroot, cross-compile]
---

# Analysis: CI Runner Env Cleanup

## Question

Root-cause the act-spawned `-isysroot` leak on `ckai-macmini.local` host-mode
Forgejo runner that blocked plan 008 Step 3, OR pick a bypass that unblocks
`BL-ci-full-clippy-test` (clippy + `cargo test` on PR).

## Findings

### Prior Art from Project Knowledge Base

1. **[Discussion 017, 2026-04-17 · decisional]** Mengdie CI matrix is
   Linux-x86_64-only. macOS *target* builds rejected (cross-compile blocked
   by fsevent-sys + CoreFoundation). Mac mini serves as the runner *host*;
   macOS-side `cargo test` is a manual pre-release ritual.
   — `docs/discussions/017-ci-pipeline-and-lint-debt/conclusion.md`

2. **[Discussion 017, 2026-04-17 · decisional]** Forgejo runner on
   `ckai-macmini.local` is in host mode per `memory/project_infra.md`.
   `container:` directive is **unavailable** in host mode. Canonical
   pattern in this repo: host-installed Rust + `source ~/.cargo/env`.
   — same source

3. **[Plan 008, 2026-04-17 · factual]** `release.yml` has a known race:
   `test:` and `build-linux:` are independent jobs with no `needs:`
   dependency. Canonical fix is couple-via-`needs:` or inline the gate
   into `build-linux:`. The race was CLOSED in the reverted Step-3 work
   and is currently RE-OPENED as a tolerated trade-off.
   — `docs/plans/008-ci-pipeline-and-lint-debt.md`

### Relevant Code

- `.forgejo/workflows/ci.yml` — current fmt-only workflow; lines 16–29
  document the `-isysroot` mystery as an inline comment block
- `.forgejo/workflows/release.yml` — reverted to pre-Step-3 shape; race
  re-introduced
- `.githooks/pre-commit` — local fmt + clippy gate (CLAUDE.md invariant;
  `--no-verify` is not a normal escape hatch)
- Git commits `e4b8cbf` through `6658248` (all 2026-04-17) — the 8-commit
  debug trail, all reverted and cherry-pickable once the env leak is fixed

### Architecture & Patterns

- **Runner topology**: host-mode Forgejo runner on macOS Mac mini. Linux
  x86_64 is the only target. `cargo-zigbuild` is installed but its shims
  bind only `aarch64-apple-darwin` today — not wired for Linux targets.
  Docker is absent.
- **cc-rs mechanism — corrected** (Codex from cc-rs source):
  `apple_flags()` in cc-rs is gated on `target.vendor == "apple"`. For
  `x86_64-unknown-linux-gnu`, vendor is `unknown` — cc-rs does **not**
  invoke `xcrun` and does **not** synthesize `-isysroot`. The initial
  investigation's assumption that "host PATH contamination via xcrun
  drives cc-rs" is wrong.
- **Real candidate leak sources** (narrowed by elimination, top hypothesis
  first after standards-expert's source-level dive in cc-rs 1.2.59 +
  ring 0.17.14):
  1. **`/usr/bin/cc` on macOS is actually Apple Clang, not GCC** — even
     though `which cc --version` reports "GCC 13.3", on macOS `/usr/bin/cc`
     is typically Apple Clang. Apple Clang **internally** shells out to
     `xcrun` for sysroot discovery and adds `-isysroot` to its own
     invocation — regardless of what cc-rs passes. The `env | sort` clean
     result is irrelevant because xcrun runs *inside* the clang binary.
     (Community reference: llvm/llvm-project#137352, 2025)
  2. `CFLAGS` / `TARGET_CFLAGS` / `CFLAGS_x86_64_unknown_linux_gnu`
     present in the cc-rs process env block (not observable from bash's
     `env`)
  3. CC wrapper/shim earlier on PATH than `/usr/bin/cc`
  4. Wrong `TARGET` / `CARGO_CFG_TARGET_*` reaching build.rs
- **Gotcha on the previously-tried `SDKROOT=''` fix**: cc-rs
  (`lib.rs:4040`) skips any SDKROOT that fails `is_absolute()`. Empty
  string fails the check → cc-rs falls back to `xcrun` anyway. The fix
  attempt was the right idea but the wrong implementation; a correctly
  blocking value would be an absolute-but-nonexistent path OR patching
  the underlying `cc` so it doesn't internally resolve xcrun.
- **Why `env | sort` inside bash was a false negative**: cc-rs and its
  child processes can read env set at `execve()` boundaries that bash's
  `env` command doesn't observe. cc-rs's build-script runs as a cargo
  child process whose env is shaped by cargo, not bash. The env block
  inspected was the bash block, not the cc invocation block.

### Industry Practice Comparison

- **Forgejo officially supports Linux runners only** (amd64/arm64).
  macOS host-mode runner is possible but explicitly "no isolation at
  all" — inherits full host dev environment. No community pattern
  exists for ring+C-deps on macOS-host-mode runners. (standards-expert)
- **cc-rs has a history of SDK-detection bugs**: issues #650 (iOS cross
  via SDKROOT), #948 (Linux→Darwin xcrun misfire). Neither directly
  matches mengdie's case (macOS host, Linux target), but both confirm
  cc-rs SDK logic has been fragile across versions. (standards-expert)
- **Community patterns** for Rust-with-ring on macOS CI: `cross`
  (Docker-backed; requires Docker), `cargo zigbuild` (zig cc bundles
  libc headers; bypasses cc-rs entirely for the C step), explicit
  per-target `CFLAGS_<triple>` overrides, or move CI off macOS entirely.

### Challenges & Disagreements

- **Resolved contradiction on mechanism**: Standards-expert's initial
  hypothesis — "cc-rs calls xcrun on any Apple host regardless of
  target" — was refuted by Codex citing cc-rs source (`apple_flags()`
  target-vendor-gated). Challenger Step 2 independently self-invalidated
  the same hypothesis: SSH-direct on the same PATH succeeds, and act's
  `bash --noprofile --norc` is a *cleaner* env than a login shell, not
  dirtier.
- **Meta-challenge from Challenger Step 1**: "Is CI worth fighting for
  at all?" Pre-commit hook covers fmt+clippy locally. Only `cargo test`
  is a genuine uncaught gap. Concrete regression class: refactor
  passes hooks, breaks `tests/e2e.rs`, ships. That's the real case for
  expanding CI — and it's narrow.
- **Gemini (via local gemma4 — Gemini API hit limit this session)**
  ranked bypass options for solo-dev: (1) Linux VPS runner, (2) Docker
  on Mac mini, (3) GitHub Actions mirror, (4) cc wrapper shim,
  (5) zigbuild, (6) fmt-only permanent. Gemma underrated zigbuild
  because it didn't know `cargo-zigbuild` is already installed.
- **Codex diagnostic methodology** (not present in prior investigation):
  compiler-wrapper at `/tmp/cc-probe/cc-log` logs argv+env at the real
  exec boundary + `CC_ENABLE_DEBUG_OUTPUT=1 cargo build -vv` + 10-line
  reproducer crate. First exec boundary where `SDKROOT`/`CFLAGS`/
  `-isysroot` appears is the leak point. Estimated 30–60 min to run.
- **Cross-team consensus on fix-quality ordering**:
  - **Avoid**: cc wrapper shim — treats symptom, brittle against cc-rs/
    ring version bumps. Flagged by both codex and gemma.
  - **Avoid**: pinning old ring — security liability dressed as
    engineering. Flagged by challenger.
  - **Avoid**: GitHub Actions mirror — private repo privacy concern,
    splits CI surface across two providers. Flagged by challenger + gemma.

## Summary

The `-isysroot` leak is **not** what the original investigation thought
it was. cc-rs does not call `xcrun` for Linux targets — the target
vendor gate is definitive. The leak therefore comes from one of:
(a) `CFLAGS*` env vars present at cc-rs's execve block (not bash's),
(b) a `cc` shim on PATH, (c) `cc` resolving to Apple-clang which reads
`SDKROOT` internally, or (d) a broken `TARGET`/`CARGO_CFG_TARGET_*`
propagation. The `env | sort` dump was a false negative because it
inspected bash's env, not cc-rs's subprocess env block.

**Two viable routes**:

**Route 1 — diagnose definitively (recommended first, time-boxed 1h)**.
Run Codex's methodology: compiler-wrapper logger + `CC_ENABLE_DEBUG_OUTPUT=1
cargo build -vv` + minimal reproducer crate. Pinpoint the exec boundary
where `-isysroot` first appears. Fix at that boundary — likely a 5-line
env manipulation in the workflow step, once the leak point is known.

**Route 2 — bypass without root-causing (if route 1 stalls)**. Ranked:
1. `cargo zigbuild --target x86_64-unknown-linux-gnu`. `cargo-zigbuild`
   is already installed; zigcc bundles Linux libc headers, replaces
   `/usr/bin/cc` for the C-compile step, and bypasses the Apple-Clang
   xcrun-internal mechanism entirely. Single workflow-line change.
2. `CC_x86_64_unknown_linux_gnu=<linux-cross-gcc>` env var — forces
   cc-rs to use a specific non-Apple compiler (requires Homebrew
   `x86_64-linux-gnu-gcc` or equivalent). Highest cc-rs precedence,
   target-specific.
3. Install Docker on the runner + `container: rust:latest` — 10-min
   runner install, completely isolates from host macOS, canonical
   community approach for ring+CI. Linux VM overhead on macOS is real
   but tolerable.
4. Move the runner to the existing Linux VPS (already hosts Forgejo
   per `memory/project_infra.md`) — sidesteps every macOS-host class
   of bug but adds CPU contention risk to the Forgejo instance.

**Secondary issue — ship unconditionally**: `release.yml` race (test vs
build-linux, no `needs:`) — 2-line fix. Should be included in whatever
plan lands from this analysis, regardless of route.

**Anti-patterns to avoid** (cross-team consensus): cc wrapper shim
(symptom, not cause), pinning old ring (security), GitHub Actions mirror
(privacy + split surface), fmt-only permanent (gives up `cargo test` gate).

**On the `006-ci-runner-env-cleanup` vs `BL-ci-full-clippy-test` split**:
these are the same problem sliced differently. Recommend closing `006` as
superseded by `BL-ci-full-clippy-test` when the plan lands. Gate on either
route 1's diagnostic outcome or route 2's bypass actually shipping.

## Possible Next Steps

Ready for `/ae:discuss docs/discussions/020-ci-runner-env-cleanup/` to
converge on:

- **Topic 1**: Diagnose-first (route 1, Codex methodology) or
  bypass-first (route 2, zigbuild)?
- **Topic 2**: If bypass, `cargo zigbuild` (minimal change) vs Linux
  VPS runner (bigger change, full sidestep)?
- **Topic 3**: Close `006-ci-runner-env-cleanup` as superseded by
  `BL-ci-full-clippy-test`? Or keep both and sequence?
- **Topic 4**: Include the `release.yml` race fix in this plan or
  handle separately?

Then `/ae:plan` for the chosen path.
