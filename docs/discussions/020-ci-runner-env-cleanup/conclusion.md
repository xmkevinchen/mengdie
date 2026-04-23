---
id: "020"
title: "CI Runner Env Cleanup — Conclusion"
concluded: 2026-04-22
plan: ""
entities: [ci, runner, env, cargo, config-toml, cflags, isysroot, apple-clang, zig-cc, bypass, forgejo, act, ring, cc-rs, cargo-env-injection, .cargo-config-toml-env-cflags-leak, ci-runner-env-cleanup]
---

# CI Runner Env Cleanup — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Verify-then-decide vs. accept-bypass-now | **Neither — direct root-cause fix supersedes both**. Delete the `[env]` block from `.cargo/config.toml` (or the whole file; it contains only 2 lines), pre-verified by local `cargo build + cargo test` on Mac mini. Fallback shapes: (a) replace with `CFLAGS_x86_64_apple_darwin` / `_aarch64_apple_darwin` in `[env]` (cc-rs per-target var convention), or (b) move CFLAGS export to Kai's `~/.zshenv`. | UAG falsification of the Round 2 zig-cc-as-CC direction uncovered `.cargo/config.toml` at repo root containing `[env] CFLAGS = "-isysroot /Applications/.../MacOSX.sdk"` (added 2026-04-16 commit `af303d5`). Plan 008's "no `.cargo/config.toml` in the repo" claim was factually wrong — the file predated its investigation by 1 day. Cargo `[env]` injects into every build-script subprocess, bypassing all shell-level unsets (which is why plan 008's `unset CFLAGS` in commit `e6a1f06` didn't help). cc-rs reads `CFLAGS` from the propagated env and appends `-isysroot` to every cc invocation, including Linux-target cross-compile. The whole Apple-Clang-xcrun hypothesis was a red herring; cc-rs's `apple_flags()` is target-vendor-gated and does not fire for Linux targets (verified in R1). | **high** — single file change, git-revertable; if post-fix issues emerge, reverting is trivial and R1/R2 mechanism rankings are archived for rapid re-activation. |
| 2 | Bypass mechanism selection | **Does not activate**. Contingent on Topic 1 selecting bypass per framing; Topic 1 selected direct fix. | Mechanism rankings (zig-cc-as-CC, Docker executor, runner-on-VPS, external CI mirror, hybrid multi-runner, cc wrapper shim) are preserved in `topic-02-bypass-mechanism/summary.md` as future-reference context should a similar scenario recur. | **high** — archival decision; no mechanism commitment. |

## Doodlestein Review

**doodlestein-strategic (2026-04-22)**: Found one valid improvement — the original Next Steps sequencing had step 4 push a test commit BEFORE step 5 restored the CI expansion, meaning the green signal would validate fmt-only CI rather than the full clippy + cargo test target state. **Applied inline**: swapped the order, specified which artifacts to drop, and folded the `release.yml` race fix. See current "Next Steps" section.

**doodlestein-regret (2026-04-22)**: Identified a 6-month reversal risk — the Topic 1 primary decision "delete the `[env]` block" is correct as a first-step confirmation, but risks being regretted when a future dep upgrade, machine re-image, or new dev host surfaces a fresh `-isysroot` need. The original commit's "fixes Xcode sysroot for C deps" reflected a real build-env need, not cargo-cult. If the line is simply deleted, the same problem will likely re-appear later and the fix will be re-added in global form (re-introducing the cross-compile leak). **Applied inline**: promoted Fallback A (per-target `CFLAGS_x86_64_apple_darwin` + `CFLAGS_aarch64_apple_darwin`) from "contingency if delete fails" to **preferred durable form** in step 2 of Next Steps. The per-target form survives future re-additions while keeping cross-compile clean.

**doodlestein-adversarial (2026-04-22)**: Identified a real-use failure in the original "cherry-pick reverted commits" framing — commits `e4b8cbf`–`6658248` were authored against the wrong hypothesis and contain dead scaffolding (`CARGO_BUILD_TARGET` exports, `unset SDKROOT/CFLAGS*` blocks, `CC_x86_64_unknown_linux_gnu` overrides) that would land in `ci.yml` as unexplained cruft. **Applied inline**: replaced cherry-pick guidance with "write `ci.yml` fresh; use those commits as shape references only" in step 4. The base was only ~53 lines; fresh write is cleaner than filtering dead code.

**Net effect on the conclusion**: two of the three Doodlestein findings (regret, adversarial) led to meaningful Next-Steps guidance updates that affect what shape the plan will take. Neither reopens a topic; both refine the how, not the what. The conclusion's Decision Summary (direct root-cause fix over bypass) is unchanged and holds under all three reviews.

## Spawned Discussions
*None.* Root cause confirmed in-discussion; no separate deep-dive needed.

## Deferred Resolutions
*None.* Zero topics deferred.

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude | Start |
| architect | Decision framework + tradeoffs | Claude | Start |
| minimal-change-engineer | Scope discipline | Claude (imported project agent) | Start |
| challenger | Adversarial stress test | Claude | Start |
| codex-proxy | OpenAI-family technical lens | Codex via MCP (reasoning=medium in R1/R2, low in R3 + UAG) | Start |
| gemini-proxy | Google-family tradeoff lens | Gemini API (R1); local gemma4:26b fallback (R2/R3 after API 503) | Start |

**Round 0 (framing review) — separate team** (`020-framing-review`, `-v2`, `-v3`): 5 agents, 3 iterations before APPROVED. v1 unanimous REVISE; v2 4 APPROVED + 1 REVISE (surgical delete) + 1 unavailable (Gemini 503 + gemma hung); v3 3/3 APPROVED. Full per-agent verdict history in `round-00/`.

## Process Metadata

- Discussion rounds: 3 (+ Round 0 framing review w/ 3 iterations)
- Topics: 2 total (1 converged on direct fix; 1 converged as not-activated)
- Autonomous decisions: 2 (both topics)
- User escalations: 0 (root cause found in-team)
- UAG outcomes: 1 falsification (zig-cc-as-CC direction overturned — this is the discussion's pivotal moment)
- Doodlestein post-conclusion: pending

## The Pivot (worth preserving as institutional knowledge)

Plan 008's backlog item `006-ci-runner-env-cleanup.md:37` states: **"cargo config: no `~/.cargo/config.toml`, no `.cargo/config.toml` in the repo, no `/etc/cargo/*`."** This was factually wrong at the time of writing: `.cargo/config.toml` was added to the repo root on 2026-04-16 in commit `af303d5` ("Discussion 016: Dreaming evolution analysis + Phase 2 roadmap") with the commit-message justification `.cargo/config.toml fixes Xcode sysroot for C deps`. Plan 008 Step 3 began investigating the `-isysroot` issue on 2026-04-17 — one day later — and its ruled-out list missed the file entirely. Every Round 1 and Round 2 agent (including TL) took the ruled-out list at face value.

The counterexample surfaced only during Unanimous Agreement Gate falsification, when challenger and minimal-change-engineer independently did one additional check: grep `CFLAGS` across the repo's config files. Both found the `.cargo/config.toml` line within minutes.

**Generalizable lesson**: a ruled-out list is only as reliable as its most recent revalidation. When an investigation's premise (here: "no cargo-level env injection") depends on a file's absence, that absence must be verified at the time of investigation, not cited from prior notes. For cross-compilation debugging on Cargo projects specifically, **always check `.cargo/config.toml` `[env]` before assuming shell-level env leakage**.

## Next Steps

→ `/ae:plan` for the fix. Suggested plan shape:

**Phase A — root-cause fix on `.cargo/config.toml`**:

1. Verify locally by renaming the file aside: `mv .cargo/config.toml /tmp/` then `cargo build && cargo test` on Mac mini. Blocks subsequent steps if fails. Exercises `libsqlite3-sys` bundle per challenger R3 C3.
2. **If step 1 passes cleanly**: the CFLAGS line may have been unnecessary all along. Restore the file and convert to **per-target form** (the preferred durable shape):
   ```toml
   [env]
   CFLAGS_x86_64_apple_darwin = "-isysroot /Applications/Xcode.app/.../MacOSX.sdk"
   CFLAGS_aarch64_apple_darwin = "-isysroot /Applications/Xcode.app/.../MacOSX.sdk"
   ```
   cc-rs reads per-target CFLAGS and applies them only to matching targets. The Linux cross-compile ignores them; local macOS builds see them. This is **preferred over outright delete** (per Doodlestein regret): the original commit message "`.cargo/config.toml` fixes Xcode sysroot for C deps" captured a real dev-env need that a future re-imaging / new machine / dep upgrade may resurface. Per-target form survives that class of future change without re-introducing the cross-compile leak.
3. **If step 1 fails**: the CFLAGS line is actually needed for local builds. Use per-target form (same as step 2) OR move to Kai's `~/.zshenv` (Fallback B). Per-target form is still preferred for keeping the env hint in-repo + machine-reproducible.

**Phase B — restore expanded CI**:

4. **Write `ci.yml` fresh** from the fmt-only current state, adding clippy (`cargo clippy --all-targets -- -D warnings`) and test (`cargo test`) jobs. Per Doodlestein adversarial: do NOT cherry-pick `e4b8cbf`–`6658248` wholesale — those commits were written against the now-falsified Apple-Clang hypothesis and carry dead scaffolding (`CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu` exports, `unset SDKROOT/CFLAGS*` blocks, `CC_x86_64_unknown_linux_gnu` overrides). Use those commits only as references for shape; write a clean file. Also fix `release.yml`'s `test:`/`build-linux:` race by adding `needs: [test]` to `build-linux:` (or inline the gate) as a bundled drive-by.
5. Push the resulting commit (or open a PR) to exercise the **full target state** — expanded `ci.yml` running on the fixed `.cargo/config.toml`. The green signal must come from clippy + cargo test actually executing on the Linux cross-compile, not from fmt-only.

**Phase C — close-out**:

6. Close `006-ci-runner-env-cleanup` as superseded (root cause was different than its backlog documented; the `[env]` leak wasn't in its ruled-out list) and `BL-ci-full-clippy-test` as done. Log to `v0.8.0` `## Notes` as `close-scope-delta` entries.
