---
role: minimal-change-engineer
round: 3
discussion: "020"
created: 2026-04-22
---

# Round 3 — Minimal-Change Engineer

Root cause confirmed: `.cargo/config.toml` committed 2026-04-16
(commit `af303d5`) contains an unconditional `[env] CFLAGS = "-isysroot
..."` that cargo injects into every build-script subprocess — including
Linux cross-compiles on the Forgejo runner. All five Round 1 agents
(me included) accepted `006-ci-runner-env-cleanup.md:37`'s claim of
"no `.cargo/config.toml` in the repo" without rechecking. The file
was added one day before the debug trail started.

My job as scope-discipline lead: recommend the minimal correct fix
and block creep.

## Evidence review

### The committed file

`/Users/ckai/Workspace/Projects/mengdie/.cargo/config.toml`:

```toml
[env]
CFLAGS = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
```

### The provenance is weak

Commit `af303d5` (2026-04-16 00:16 -0500) is titled "Discussion 016:
Dreaming evolution analysis + Phase 2 roadmap." The `.cargo/config.toml`
addition is a drive-by entry in the 11-file commit body:
"`.cargo/config.toml` fixes Xcode sysroot for C deps."

Discussion 016 (`docs/discussions/016-dreaming-evolution/`) is about
LLM provider selection and Phase 2 roadmap. No mention of `isysroot`,
`xcrun`, `cc`, or C compilation anywhere in its analysis/conclusion
/topic summaries. **The CFLAGS line has no discussion-level rationale
captured**. Kai added it in passing to fix a local-dev compile issue
that day; it bled into CI the next day.

### The revert trail confirms the mechanism

The 2026-04-17 debug sequence (reverted 006 backlog :46-49 + the
`git log` chain) includes commits:
- `6343e6b` "blank SDKROOT on Linux runner — ring build fails otherwise"
- `1e6acae` "unset SDKROOT inline in each run step"
- `e6a1f06` "unset **CFLAGS** too — the actual leaked host env, not SDKROOT"

Commit `e6a1f06`'s title shows Kai identified CFLAGS as the variable
but attributed it to "host env" leaking from shell rc files. He tried
`unset CFLAGS` at the bash level in the workflow step — but cargo
re-reads `.cargo/config.toml`'s `[env]` table at its own process
boundary and re-injects CFLAGS for build-scripts. This is precisely
the `analysis.md:83-88` "bash env vs cc-rs execve env" mechanism.
The diagnosis was one grep away (`rg -n CFLAGS .cargo/`).

## Question 1 — Can we just delete the line?

**Answer: yes, almost certainly. Not quite trivially, but the risk is
bounded.**

Why it might have been needed: Kai's local macOS dev environment (Xcode
command-line tools, whatever Homebrew state he had on 2026-04-16) was
failing to find `stdint.h` without an explicit `-isysroot`. The usual
cause is a partially-installed Xcode CLT or a `SDKROOT` that was
unset when Xcode rotated SDKs. This happens after macOS updates.

Why deletion is likely safe *now*:
1. A week has passed (2026-04-16 → 2026-04-22). macOS ship state
   settles. If Kai's local environment re-stabilized (e.g., Xcode
   update completed, or `xcode-select --install` was run in the
   interim), the explicit CFLAGS is obsolete.
2. `release.yml` successfully built and released Linux binaries
   historically — but all of those runs were from *before* the file
   existed (or cached builds). Need to confirm whether a release has
   been cut since 2026-04-16. Per `git log --oneline | head`, the
   latest tag is `v0.1.0` on `33d1917` which predates the CFLAGS
   commit — so Linux release build success *post-CFLAGS-file* is
   unverified.
3. The `cargo-zigbuild` workflow release path goes through the same
   config.toml. If Linux releases have been shipping (need to check
   Forgejo release page), then zigbuild tolerates the CFLAGS somehow
   — but CI act-spawned jobs don't. That asymmetry is worth a single
   sentence in Round 3 synthesis.

**Recommended minimal fix: delete the line.** Then run `cargo build`
locally on Mac mini. If it breaks on `stdint.h`-like errors, that
tells us the local env still needs help; in that case fall back to
Question 2 (scope the CFLAGS). If it builds cleanly, ship deletion.

This is 1 line removed, 1 commit, no conditional logic, no
`cfg(target_os=...)` complexity, no compatibility surface.

## Question 2 — If it must stay, smallest-scope form

**Answer: scope it by target triple, not by OS cfg.**

Cargo's `[env]` table in `.cargo/config.toml` is documented as
applying to **all** invocations (see Cargo docs, `configuration.md`
section "[env]"). There is **no** supported
`[target.'cfg(...)'.env]` syntax — the `[target.<triple>.env]` form
works only for `linker`, `runner`, `rustflags`, and similar, **not**
for `[env]`. Verifying this is important because the team might
propose `[target.'cfg(target_os="macos")'.env]` as a natural
minimal-scope fix and discover it silently does nothing.

Given that constraint, the options ranked by minimality:

**Option A — Delete (recommended, Q1 above).**

**Option B — Move to the developer's shell profile.** `export
CFLAGS="-isysroot ..."` in Kai's `~/.zshenv` or per-shell profile.
Zero repo surface. Local dev gets the env var; CI does not. Risk:
future collaborators or CI jobs run on Kai's machine would still see
the env var via shell inheritance — but (a) solo dev today, (b) act
spawns with `--noprofile --norc` per the 006 backlog, so shell
profile env does *not* reach act-spawned workflow steps. Clean
separation.

**Option C — Wrap in a build.rs conditional.** Add a `build.rs` at
crate root that detects host OS and sets CFLAGS only for macOS
host. Rejected: introduces a build.rs where there is none today
(grep confirmed: no `build.rs` in the workspace root). That's a
new maintenance surface to solve a problem Option A or B solves
with zero code.

**Option D — Per-target workflow env in CI.** Add `CFLAGS=` (empty)
to the workflow step in ci.yml to override the config.toml value.
Rejected: cargo `[env]` has `force = false` by default and lets
process env override it — but the reverse happens: `[env]` without
`force` sets the var ONLY if not already set in the process env. So
setting `CFLAGS=` in the workflow step *should* suppress the
config.toml value, but this requires verification against the exact
cargo version. Also: this is exactly what commit `e6a1f06` already
tried; it evidently didn't work. Almost certainly because
`[env.CFLAGS]` was later re-set by cargo as it descended into build-
scripts even when bash env was cleared. Reject as "already tried,
failed, moot once we fix the config.toml itself."

**Recommendation**: Q1 deletion. If it breaks local, Q2 Option B.
Nothing else.

## Question 3 — "While we're at it" creep to push back on

The CFLAGS root cause + 1-line fix is a genuinely boring
result. Predict the creep attempts and name them now:

**Creep 1: "Since we have `.cargo/config.toml` open, let's add the
Linux target config block for zigbuild while we're here."** Reject.
The zigbuild direction was falsified by finding the root cause. No
zig-cc wrapper is needed. Adding `[target.x86_64-unknown-linux-gnu]`
entries preemptively creates future maintenance for a hypothetical
need.

**Creep 2: "Let's add a comment explaining what CFLAGS was for, for
future archaeologists."** Reject if we go with Q1 deletion —
obviously nothing to comment on. Reject if we go with Q2 Option B
— comment belongs in the shell profile, not tracked in repo. Only
accept if we end up with Option D or C (neither recommended). The
comment itself is low-risk, but every additional file we touch is
additional diff to review; keep the change surface tight.

**Creep 3: "Let's add `cargo test` AND clippy AND a matrix across
debug/release AND GitHub Actions mirror AND …"** The BL-ci-full-
clippy-test charter was originally L (5 pt) because it accrued
compiler-bypass machinery in its scope. With the root cause fixed,
it collapses to approximately:
- Remove/correct `.cargo/config.toml` (1 line)
- Add `cargo clippy --all-targets -- -D warnings` job to ci.yml
- Add `cargo test` job to ci.yml
- (drive-by) Fix `release.yml` race via `needs:` — explicitly out-of-
  scope per framing; goes in a separate commit when next touching
  release.yml.

Reject anything beyond those three + the out-of-scope drive-by.
Total plan-level change surface: `.cargo/config.toml` (1 line),
`.forgejo/workflows/ci.yml` (add 2 steps, ~10 lines).

**Creep 4: "Now that we have evidence, let's go do the Codex diagnostic
(`CC_ENABLE_DEBUG_OUTPUT=1`) for institutional knowledge."** Reject.
Diagnosis was achieved by `rg CFLAGS .cargo/`. Retroactive
verification has zero decision value.

**Creep 5: "Close 006, BL-ci-full-clippy-test as invalid and re-file."**
Marginal — I think this is correct bookkeeping, not creep. The
framing is "leak in runner env"; actual cause is "committed `[env]`
table in `.cargo/config.toml`". Those are different factual claims.
Keep the backlog IDs for audit trail, but the resume-trigger sections
become wrong. Recommend: add a 2026-04-22 note at the top of both
backlog files pointing at discussion 020's conclusion + the actual
fix commit, and flip status to `done` when the fix + CI expansion
ships. Not creep; required audit-trail hygiene.

## The minimal plan

Not a plan document — just the acceptance-criteria-sized view for
`/ae:plan` time:

1. **Verify** (10 min): `cd /Users/ckai/Workspace/Projects/mengdie &&
   git mv .cargo/config.toml /tmp/ && cargo build && cargo test` on
   Mac mini to confirm nothing local breaks.
   - If clean → proceed to step 2 with deletion.
   - If breaks → restore file, proceed with Option B (move to
     shell profile).
2. **Fix** (2 min): apply chosen fix. Single commit.
3. **Expand CI** (15 min): add clippy + cargo test steps to
   `.forgejo/workflows/ci.yml`. Keep fmt step.
4. **Observe** (CI run, ~5-10 min cold): confirm CI goes green on a
   throwaway branch before landing.
5. **Close out** (5 min): update `.ae/backlog/v0.8.0/006-...md` +
   `BL-ci-full-clippy-test.md` with a 2026-04-22 root-cause note +
   flip to done.

Total estimate: 40 min, one PR, two commits (fix + CI expansion),
one backlog closeout commit.

**Not in this plan**: zigbuild, VPS relocation, Docker, SSH-trigger,
`.cargo/config.toml` target-scoped rewrites, Codex diagnostic
methodology, `release.yml` race fix.

## Findings

1. The CFLAGS root cause reduces the entire Topic 1/Topic 2 decision
   tree to a single 1-line fix. All five R1/R2 bypass mechanisms
   (verify, zigbuild, zig-cc-as-CC, Docker, VPS, SSH-trigger) are
   now over-engineering.
2. The file has weak provenance: added as a drive-by in commit
   `af303d5` with a one-line commit-message note and no
   discussion-level rationale. That's why it evaded detection — no
   one searched for it because its reason for existing was never
   documented.
3. `[target.'cfg(...)'.env]` is not a supported Cargo syntax. Must
   not propose it as a scope-down in a plan without testing.
4. Option A (delete) is safe **conditional on** `cargo build`
   succeeding on Mac mini after removal. Option B (shell profile) is
   the fallback. Options C/D rejected as either new surface or
   already-tried-and-failed.
5. Closing 006 + BL-ci-full-clippy-test as `done` with a root-cause
   note is required audit hygiene, not scope creep.

## Agreements

- With **team-lead**'s collapse: zigbuild was a decoy; no compiler
  replacement or runner-topology change needed.
- With **challenger C5** (R1): the pre-flight retest was the right
  instinct. It would not have fired (the failure still reproduces),
  but checking the repo state *first* would have.
- With **analysis.md:73-75** candidate 2 (CFLAGS* in execve env
  block): identified correctly in R0 analysis, then incorrectly
  scoped to "host shell env" rather than "project config". The
  mechanism taxonomy was right; the location search was wrong.

## Disagreements

- With my own **R1/R2 proposals**: the zig-cc-as-CC Stage 1 +
  SSH-trigger Stage 2 scaffold was over-engineered. I accept this
  and withdraw both stages. Any R3 synthesis should cite this as a
  worked example of "converge on elegant workaround before reading
  the config" failure mode.
- With the framing's ruled-out list (`006-ci-runner-env-cleanup.md:37`):
  "no `.cargo/config.toml` in the repo" was false at the time of
  writing (the file was added 2026-04-16, backlog authored
  2026-04-17). The single most load-bearing observation in the
  backlog was wrong. Future investigations should re-verify ruled-out
  claims as a first pass, not trust the narrative.

## Open Questions

1. **Does `cargo build` + `cargo test` pass on Mac mini after
   deletion?** Must be tested before PR lands. If yes → delete. If
   no → Kai's local Xcode/CLT state needs investigation separately,
   and the fix becomes Option B.
2. **Has any Linux release been cut since 2026-04-16?** If yes, that
   release used the CFLAGS-polluted config successfully somehow
   (cargo-zigbuild may tolerate it differently than cc-rs does).
   Check Forgejo release page. Minor point; doesn't change the fix
   recommendation.
3. **Should the `release.yml` race fix ride along?** Per framing,
   out of scope. Per minimal-change discipline, also no — ride it
   in whichever plan next touches release.yml. Confirm no one
   sneaks it into this plan.
