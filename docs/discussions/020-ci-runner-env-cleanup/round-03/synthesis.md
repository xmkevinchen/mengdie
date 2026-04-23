---
round: 3
date: 2026-04-22
role: TL synthesis (index + orientation only — per-agent files are primary)
note: |
  Round 3 was triggered by UAG-found counterexample (challenger +
  minimal-change-engineer both independently discovered `.cargo/config.toml`
  [env] CFLAGS leak during UAG falsification of the Round 2 converged
  direction). The original T1/T2 dichotomy (verify-vs-bypass,
  mechanism-selection) is overtaken by the root-cause find: neither
  verification nor bypass is needed; direct fix supersedes both.
---

# Round 3 Synthesis

## Per-agent file index

| Agent | File | Round 3 headline |
|-------|------|------------------|
| architect | [architect.md](architect.md) | Delete the `[env]` block. cc-rs auto-detects SDK via xcrun for apple-vendor targets — hardcoded `-isysroot` is redundant. 5-step fix plan, ~20 min. |
| minimal-change-engineer | [minimal-change-engineer.md](minimal-change-engineer.md) | Delete the whole file (2 lines). Pre-condition: test locally. Fallback: Kai's `~/.zshenv`. Rejected `[target.<triple>.env]` as invalid syntax. |
| challenger | [challenger.md](challenger.md) | 6 challenges on "1-line fix". Key: may break local builds (no record why added); `libsqlite3-sys` also affected; per-target scoping via `[env]` is illusory (but cc-rs per-target var works). |
| codex-proxy | [codex-proxy.md](codex-proxy.md) | Confirmed mechanism. Cargo does NOT support `[target.<triple>.env]`. Fix: delete outright OR scope via `CFLAGS_x86_64_apple_darwin`. Sanity-check: `cargo check -vv --target x86_64-unknown-linux-gnu`. |
| gemini-proxy | [gemini-proxy.md](gemini-proxy.md) | Bypass discussion moot. Codify debugging heuristic: "Check `.cargo/config.toml` `[env]` before assuming shell leakage". |

## 1. Pruned

Pruned from Round 2 convergence:
- **Entire zig-cc-as-CC direction** — pruned. Root cause found is `.cargo/config.toml` `[env]`. No compiler replacement needed.
- **Pre-flight retest as hard gate (Stage 0)** — pruned as a T1 step. The retest is now trivially informative: run it after the fix to confirm. Not a pre-decision gate.
- **VPS SSH-trigger / runner-on-VPS Stage 2** — pruned. Test execution runs on the same Mac mini runner post-fix; no topology change.
- **Env-unset belt-and-suspenders (SDKROOT/DEVELOPER_DIR/CFLAGS*)** — pruned. The leak source is a cargo-level config, not a shell env var; unsetting at the shell level doesn't help anyway (as plan 008 already discovered).
- **Codex's `CC_ENABLE_DEBUG_OUTPUT=1` institutional diagnostic** — pruned (per minimal-change R3). The diagnosis was `rg CFLAGS .cargo/` — one grep found the leak. Future investigations need the heuristic, not a full re-run.

Pruned from Round 1:
- **Architect's Group A/B split** — retrospectively pruned. The split framed bypass options by mechanism assumption; once mechanism is known to be cargo `[env]` (not any of the original four candidates), the split dissolves.

No pruning disputed.

## 2. Of-framing disposition

Challenges this round + disposition:

| Challenge | Source | Disposition |
|-----------|--------|-------------|
| "Removing `-isysroot` may break local macOS builds (no record of why added)" | challenger R3 C1 | **Integrated as pre-condition**: minimal-change R3 already added the local `cargo build + cargo test` pre-check. Direction holds; plan executes the check. |
| "`libsqlite3-sys` also compiles C via cc-rs — affected too" | challenger R3 C3 | **Integrated**: plan's pre-condition test must include `cargo build` which exercises `libsqlite3-sys` bundle. If local build passes post-delete, sqlite bundled build is fine. |
| "Per-target scoping in cargo `[env]` is illusory" (cargo has no target filter on `[env]`) | challenger R3 C2 | **Partially accepted**: cargo's `[env]` has no target scope, but cc-rs's `CFLAGS_<target-triple>` convention provides functional scoping. The var name `CFLAGS_x86_64_apple_darwin` is only consumed by cc-rs when the matched target is being built. Codex R3 confirmed. |
| "`[target.<triple>.env]` is not valid Cargo syntax" | challenger + codex + minimal-change R3 | **Integrated**: remove any mention of this shape from the fix option space. |
| "Codify debugging heuristic for future" | gemini-proxy R3 | **Integrated into conclusion + knowledge capture**: the "ruled-out list was factually wrong" pattern is worth preserving. |
| "1-line fix conflates three distinct actions with different safety profiles" | challenger R3 C5 | **Integrated**: the plan must pick one explicit action — (a) delete, (b) `CFLAGS_<triple>` per-target scope, or (c) move to user shell env. Decision at plan time based on pre-condition check outcome. |
| "Recurrence guardrail: could this leak class happen again?" | challenger R3 C6 | **Integrated as conclusion note**: once CI actually runs `cargo build --target x86_64-unknown-linux-gnu`, it becomes self-guarding against this class of `[env]` leak. The guard is the CI expansion itself. |

No of-framing rejected.

## 3. Verification artifact

| Claim | Status |
|-------|--------|
| `.cargo/config.toml` exists at repo root with `[env] CFLAGS = "-isysroot ..."` | **verified** (file read directly by challenger + minimal-change independently; TL confirmed with `Read`) |
| File added 2026-04-16 commit `af303d5` "`.cargo/config.toml` fixes Xcode sysroot for C deps" | **verified** (TL confirmed via `git log --follow -p`) |
| Plan 008's "no `.cargo/config.toml` in the repo" claim was wrong | **verified** (file predates plan 008 Step 3 investigation by 1 day) |
| Cargo `[env]` propagates to build-script subprocesses, bypassing shell unsets | **verified** (codex R3 from cargo source; aligns with observed behavior: reverted commits tried shell `unset CFLAGS`, which failed) |
| Cargo does NOT support `[target.<triple>.env]` | **verified** (codex + challenger + minimal-change R3 all confirm) |
| cc-rs reads `CFLAGS_<target-triple-underscored>` for target-scoped flags | **verified** (codex R3; this is cc-rs's documented env var convention) |
| Deleting the line is safe for local macOS builds | **unvalidated** (challenger C1) — plan must pre-verify with `cargo build + cargo test` on Mac mini before shipping |
| cc-rs auto-detects SDK via xcrun for apple-vendor targets, making the hardcoded line redundant | **partially verified** (architect R3 claim, aligns with cc-rs source cited in analysis) — final confirmation comes from the pre-verification step |

Nothing converged on unvalidated evidence.

## 4. Frame-challenge disappearance self-check

R2 markers to check in R3:

- "Zig-cc-as-CC vs bare zigbuild distinction" → **dissolved**: the whole mechanism discussion moots when root cause is not compiler-side. Pruned, not silently dropped.
- "VPS CPU headroom unmeasured" → **dissolved**: no topology change needed. Preserved in conclusion as a hypothetical fallback note only.
- "Test-execution vs test-compile gap" → **resolved**: after delete, test execution runs on Mac mini (Apple Clang handles Linux-target via cargo/cc-rs normal paths without the injection). The "VPS for test exec" Stage 2 is no longer necessary.
- "Retest-then-dry-run collapse into single step" → **supplanted**: the dry-run isn't needed; the fix is direct. Retest becomes post-fix verification, not pre-decision gate.

R1 markers to re-check:
- "Runner relocation ambiguity" → **moot**: no relocation needed.
- "30-min verification bound" → **moot**: no formal verification step remaining.

No silent disappearance.

## Converged direction (ready for TL scoring)

**The original topics are overtaken by the root-cause find.** Scoring reflects this:

- **T1 (verify-then-decide vs accept-bypass-now)**: overtaken. The right answer is neither — it's **direct root-cause fix**. The UAG falsification uncovered `.cargo/config.toml` `[env]` as the leak; no bypass needed.
- **T2 (bypass mechanism selection)**: **does not activate**. Contingent on T1 selecting bypass; T1 didn't. Mechanism rankings (zig-cc-as-CC, Docker, VPS) preserved in the conclusion as context, not as decisions.

**Convergent fix direction (for plan time)**:
1. **Primary**: delete the `[env]` block from `.cargo/config.toml` (or delete the file; it contains only the one block).
2. **Pre-condition check** (blocks the primary if it fails): run `cargo build` and `cargo test` locally on Mac mini with the line removed. Must pass cleanly (including `libsqlite3-sys` bundle build).
3. **Fallback if pre-condition fails**: either (a) replace with `CFLAGS_x86_64_apple_darwin = "-isysroot ..."` in `[env]` (cc-rs per-target scoping), OR (b) move the CFLAGS export to Kai's user shell env (`~/.zshenv`).
4. **After fix lands**: verify on the runner by pushing a test commit that exercises the full ci.yml workflow (clippy + cargo test).
5. **Then**: cherry-pick the reverted commits from plan 008 (`e4b8cbf` through `6658248`) to restore the full CI workflow.

**Bundled in-scope for plan**:
- The `.cargo/config.toml` fix
- ci.yml expansion (clippy + cargo test on PR)
- Closure of `BL-ci-full-clippy-test` and `006-ci-runner-env-cleanup` (they both resolve with this single change)

**Still out-of-scope per framing**:
- `release.yml` race fix (drive-by for whoever touches `release.yml` next)
- BL boundary consolidation decision (both items close post-plan; the "supersede" bookkeeping is a plan-time artifact)
