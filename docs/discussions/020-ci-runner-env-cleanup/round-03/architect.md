---
agent: architect
round: 3
created: 2026-04-22
topics: [topic-01, topic-02]
---

# Round 3 — Architect

## Root Cause Confirmed

The leak source is `.cargo/config.toml:2`:

```toml
[env]
CFLAGS = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
```

Added in commit `af303d5` (2026-04-16) as a bundled side-change in the
"Discussion 016: Dreaming evolution analysis" commit. Commit message line:
"`.cargo/config.toml` fixes Xcode sysroot for C deps."

**Why the plan 008 investigation missed it**: the scope-down commit
`9c03286` states "no `.cargo/config.toml` in the repo" — factually wrong.
The file was created one day earlier in `af303d5`. The investigation
checked for the file but either missed it or checked before `af303d5`
landed. Regardless: the cargo `[env]` table injects vars into every
build-script subprocess at `execve()` time, *after* any shell-level
`unset`. Commit `e6a1f06` actually *found* the leak (`CFLAGS =
Some(-isysroot /Applications/Xcode.app/...)` in Run 24 output), added
`unset SDKROOT CFLAGS CXXFLAGS CPPFLAGS LDFLAGS` to the workflow steps,
but the unset had no effect because `cargo [env]` re-injects past the
shell boundary. The investigation was one layer too shallow.

---

## Why the Line Was Added — Intent Reconstruction

The commit message says "fixes Xcode sysroot for C deps." No prior
discussion, no issue reference, no plan context. The file was created from
scratch (0→2 lines). Timing: April 16, the day before plan 008 CI
debugging began. Most likely scenario: Kai hit a build failure on macOS
local (probably ring or libsqlite3-sys failing to find the macOS SDK),
added the CFLAGS line as a quick local fix without recognizing it would
poison CI builds on the Linux target.

**Is the fix still needed for local macOS builds?** Current state:
`xcrun --show-sdk-path` returns the correct path
(`/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk`).
Xcode is installed and the symlinks are current (`MacOSX.sdk →
MacOSX26.4.sdk`). Modern Rust toolchains (via cc-rs) auto-detect the
macOS SDK via `xcrun` when the target vendor is `apple` — the
`apple_flags()` function in cc-rs calls `xcrun --show-sdk-path` itself.
A hardcoded `CFLAGS = -isysroot <path>` is redundant when Xcode is
installed, and actively harmful when it leaks to non-apple targets.

---

## Fix Recommendation: Delete the Line

**Recommended fix**: delete the entire `[env]` block from `.cargo/config.toml`.
If the file has no other content, delete the file.

Rationale:

1. **cc-rs already handles macOS SDK detection.** `apple_flags()` in cc-rs
   (confirmed in analysis.md as target-vendor-gated) calls `xcrun
   --show-sdk-path` for apple-vendor targets. The hardcoded `-isysroot`
   in `CFLAGS` duplicates this and can conflict if the SDK path changes
   (e.g., Xcode upgrade, SDK symlink rotation). The current path value in
   config.toml (`MacOSX.sdk`) resolves to `MacOSX26.4.sdk` — correct
   today, but only because of a symlink. Deleting the line removes the
   maintenance surface.

2. **The line was added to fix a symptom, not a cause.** If macOS local
   builds were failing before the line was added, the real cause was
   something else (Xcode not installed, `xcrun` not on PATH, etc.). That
   condition no longer applies — Xcode is present and `xcrun` works.

3. **Scope is global and cross-target.** `cargo [env]` applies to all
   targets in all build contexts — macOS native, Linux cross-compile, CI.
   A macOS-SDK-specific value for `CFLAGS` has no legitimate use in a
   global config for a project that cross-compiles to Linux. If
   per-target CFLAGS were needed for macOS local builds, the correct
   form would be `CFLAGS_x86_64_apple_darwin` or
   `CFLAGS_aarch64_apple_darwin` (not `CFLAGS` — the generic form applies
   to all targets including Linux).

4. **Delete is reversible.** If local macOS builds break after deletion,
   the regression is immediate and visible (`cargo build` fails locally
   on the first C-dependent crate). The fix is then to investigate why
   cc-rs's auto-detection is failing — not to restore a global CFLAGS
   override.

---

## Option 2 Analysis: Scope to Apple Targets via Per-Target Var

The TL's option 2 (use `CFLAGS_x86_64_apple_darwin` and
`CFLAGS_aarch64_apple_darwin`) is structurally correct — cc-rs reads
per-target `CFLAGS_<triple>` and skips them for non-matching targets.
This would fix the CI leak while preserving the macOS-local behavior.

However, it still embeds a hardcoded SDK path that:
- Will drift as Xcode upgrades
- Duplicates cc-rs's own xcrun-based detection
- Requires updating when the SDK version changes (e.g., macOS 26.4 → 27.x)

This option is defensible only if cc-rs's auto-detection is broken for
apple-vendor targets on this specific machine. There is no evidence of
that. Prefer delete.

---

## Option 3 Analysis: Investigate Why It Was Added

The TL's option 3 is the safer due-diligence path, but the evidence
already answers it:

- Xcode is installed, `xcrun` returns the correct path, SDK symlinks are
  current.
- cc-rs auto-detection works for apple-vendor targets (analysis.md
  confirms `apple_flags()` calls xcrun for apple-vendor; nothing suggests
  the runner lacks Xcode for macOS-native builds).
- The most likely cause of the original failure (if any) was a temporary
  Xcode/CommandLineTools state that has since been resolved.

Additional investigation would mean: check out the commit just before
`af303d5`, run `cargo build` locally, observe whether it succeeds. This
is a 5-minute test, worth doing before merging the fix. But the prior
probability that the line is still needed is low given the current Xcode
state.

---

## Revised Fix Sequence

**Step 1 (pre-flight, 5 min)**: Confirm current failure reproduces — run
`cargo clippy --target x86_64-unknown-linux-gnu` or `cargo build --target
x86_64-unknown-linux-gnu` on the Mac mini runner (outside act) and observe
the `-isysroot` error. This validates the root cause before touching
anything.

**Step 2 (fix, 1 min)**: Remove the `[env]` block from `.cargo/config.toml`.
If the file becomes empty, delete it.

**Step 3 (local regression check, 2 min)**: Run `cargo build` on macOS
locally (native target). Verify it still succeeds. If it fails → re-add
the per-target form (option 2) as a fallback and open a follow-up
investigation into why cc-rs auto-detection is broken.

**Step 4 (CI validation, 10 min)**: Push a test commit that adds
`cargo clippy --all-targets -- -D warnings` and `cargo test` to `ci.yml`.
Observe CI run. If green → proceed to cherry-pick and clean up the
reverted commits (`e4b8cbf` through `6658248`). If still fails → the
mechanism hypothesis is still wrong; escalate to Codex methodology.

**Step 5**: Restore the full `ci.yml` from the reverted commits, clean up
the inline comments about the mystery.

---

## Impact on Prior Rounds

- All bypass work (zigbuild, zig-cc-as-CC, VPS runner) was solving the
  wrong problem. The fix is a 1-line deletion in a config file.
- The UAG falsification (candidate 2, env-unset safety) was correct in
  principle but missed that `cargo [env]` defeats shell-level unsets.
  The "env-unset is safe" conclusion was right; the "env-unset will work"
  conclusion would have been wrong.
- Challenger C5 (failure may not reproduce) remains valid as a pre-flight
  check — step 1 above.

---

## Open Questions

1. **Why did `unset CFLAGS` in `e6a1f06` not fix it?** Answered:
   `cargo [env]` injects after the shell step's env is established.
   `unset` in the run block operates on the shell process env; cargo
   injects into child process env at `execve()`. The unset is invisible
   to cargo's `[env]` injection. This is the mechanism that made the
   investigation conclude "env is clean" — it was clean at the bash level,
   not at the cargo child process level.

2. **Does `.cargo/config.toml` need any other content?** Current content
   is 2 lines, both part of the `[env]` block. After deletion the file
   is empty. Delete the file entirely to avoid confusion. If future
   cross-compile config is needed (e.g., `[target.x86_64-unknown-linux-gnu]
   linker = ...`), recreate it then.

3. **Should the fix include the `release.yml` race (test vs build-linux,
   no `needs:`)?** Framing marks this out-of-scope as a drive-by. Confirmed
   position: include it in the same plan as a drive-by since it's 2 lines
   and touching `release.yml` anyway.
