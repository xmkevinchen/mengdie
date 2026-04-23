---
role: minimal-change-engineer
round: 2
discussion: "020"
created: 2026-04-22
---

# Round 2 — Minimal-Change Engineer

Three of my Round 1 claims took direct challenges. I need to concede
some, sharpen others, and update my recommended minimal path in light
of what peers surfaced.

## Response to direct challenges

### On Challenger C3 (zigbuild-vs-ring unverified)

`challenger.md:71-84` is right that my Round 1 "universal zigbuild"
framing was unverified. Codex-proxy provides the concrete evidence
that narrows it further: `codex-proxy.md:69` cites the
`cargo-zigbuild` README — "`cargo zigbuild` is mainly a build wrapper,
not a general `cargo clippy` wrapper." So `cargo zigbuild` as I
proposed in Round 1 does not wrap `cargo clippy` or `cargo test`
cleanly. That's a real defect in my Round 1 proposal.

But Codex-proxy Section 3 (`codex-proxy.md:94-122`) resolves this with
a **different** minimal-change mechanism I had missed: install a
zig-cc wrapper script, point `.cargo/config.toml` at it as
`linker` + `CC_x86_64_unknown_linux_gnu`. This works for
`cargo clippy --target ...` and `cargo test --target ... --no-run`
because it substitutes at the cc-rs invocation level, not the cargo
subcommand level.

**Updated minimal-change proposal**: not `cargo zigbuild`, but
zig-cc-as-CC via `.cargo/config.toml` (per codex Section 3).
Evidence: `codex-proxy.md:94-122` cites Cargo config and Cargo test
docs directly. Still a one-file change (add `.cargo/config.toml` +
commit a `ci/zigcc-x86_64-linux-gnu` shell wrapper). Still zero
ongoing infrastructure maintenance. Concedes my Round 1 wording but
preserves the minimal-change verdict under a corrected mechanism.

### On Architect's Group A/B split (`architect.md:34-61`)

The split is real. Group B mechanisms (zigbuild, `CC_<target>=`)
replace the compiler identity — they only repair the failure if the
leak source is compiler identity. If the leak is candidate 2
(`CFLAGS_*` in cc-rs's execve env block), Group B does not fix it
because the bogus `-isysroot` would be re-injected via `CFLAGS`
regardless of which compiler we point cc-rs at.

Concession: my Round 1 claim that zigbuild is "robust against all
four candidate leak sources" was too strong. It's robust against
candidates 1, 3, 4 (compiler identity class). It is **not**
necessarily robust against candidate 2 (env-var injection class).

But Architect's conclusion — that this *requires* verification — is
not the only response. A cheaper response is available: in the same
workflow step that invokes the zig-cc wrapper, **explicitly unset
`CFLAGS`, `CFLAGS_x86_64_unknown_linux_gnu`, `SDKROOT`,
`DEVELOPER_DIR`, `MACOSX_DEPLOYMENT_TARGET`** before cargo runs.
Codex-proxy independently proposes this (`codex-proxy.md:32-35`) as
part of verification; I propose it as a belt-and-suspenders bypass
ingredient instead. Unconditionally. Now:

- If leak is candidate 1 (Apple Clang as cc) → zig-cc wrapper fixes it.
- If leak is candidate 2 (CFLAGS*) → explicit unset fixes it.
- If leak is candidate 3 (cc shim on PATH) → explicit CC path fixes it.
- If leak is candidate 4 (wrong TARGET) → explicit `--target` +
  `.cargo/config.toml` target section fixes it.

This is belt-and-suspenders: a single workflow step with the wrapper
+ the unsets covers all four hypotheses simultaneously. No
verification needed to pick which subset to apply, because all four
measures are cheap enough to apply together.

**Concession with counter-proposal**: Architect is right that Group
B alone is mechanism-dependent. But Group B + an env-unset prelude is
mechanism-agnostic, and still a single workflow file diff — no
infrastructure change. The minimal-change verdict survives under this
combined form.

### On Challenger C6 (only `cargo test` is the gap)

`challenger.md:151-160` is the most consequential framing
narrowing. The pre-commit hook already runs fmt+clippy locally
(per CLAUDE.md). The actual CI uplift is `cargo test` — catching the
"refactor passes hooks, breaks `tests/e2e.rs`, ships" class, which
`analysis.md:117` flags as the only genuine gap.

This sharpens Topic 2's selection criterion. The goal is not "full
CI parity" — it is "reliable `cargo test` execution on CI, with
minimum infrastructure."

And here's where Codex-proxy's evidence reshapes the recommendation:
`codex-proxy.md:119-122` cites the Cargo docs to establish that
`cargo test --target x86_64-unknown-linux-gnu --no-run` **compiles
the tests but cannot execute them on macOS**. Linux test binaries
need a Linux runtime to run. A `target.<triple>.runner` hook can
work, but that means Docker/VM/SSH-to-Linux.

So **zig-cc-as-CC covers test-compile but not test-execute**. For
test-execute, something Linux-side is needed.

This is a real hole in my Round 1 minimal-change proposal. I did
not address test execution. I owe a response.

**My updated position** (see Topic 2 below): a staged minimum.
Stage 1 covers clippy + `cargo test --no-run` via zig-cc-as-CC on
the existing Mac mini runner — catches compile-time regressions in
tests (type errors, wrong use of test helpers, missing feature
flags). Stage 2, if Stage 1 doesn't catch enough, adds actual
test-execute via gemini-proxy's SSH-trigger pattern
(`gemini-proxy.md:67-69`) to the existing Linux VPS — a ~20-line
shell script, no new runner topology. Only promote to Stage 2 if a
concrete regression slips past Stage 1.

This preserves minimal-change discipline: only add infrastructure
when there's evidence the cheaper stage missed something.

## Topic 1 — Revised position

**Position: pre-flight retest first (per challenger C5), then
accept-bypass-now with the combined mechanism-agnostic bypass.**

Challenger C5 (`challenger.md:138-149`) surfaces a ~5-minute test I
missed in Round 1: the failure was last confirmed 2026-04-17. The
runner may have been updated. Before any debate on verify-vs-bypass
is meaningful, rerun the failing cargo invocation on the current
runner state.

Plan:
1. **Step 0 (5 min)**: SSH to runner, rerun the minimal failing
   invocation from the 006 backlog. If it no longer reproduces →
   ship `cargo test` in CI unchanged, close this entire discussion.
2. **Step 1 (conditional, if Step 0 still reproduces — 15 min)**:
   Ship the zig-cc-as-CC + env-unset workflow per Topic 2. Observe
   CI run. If green → done.
3. **Step 2 (conditional, if Step 1 fails — then verify)**: Only at
   this point run codex's `CC_ENABLE_DEBUG_OUTPUT=1` diagnostic
   (`codex-proxy.md:27-29`). Time-box hard at 60 min.

This flips my Round 1 "bypass-now, verify as recovery" into
"pre-flight, bypass-now, verify as last-resort recovery." Step 0 is
new and cheap enough to do unconditionally. Steps 1–2 match my
Round 1 position with the concession that verify exists as a
recovery lane, not as a prerequisite.

**On the "verification is action-neutral" claim I made in Round 1**:
I partially concede to architect. Under the pure zigbuild proposal,
verification *was* action-neutral — but only because I was ignoring
candidate 2 (CFLAGS*). Under the combined zig-cc + env-unset
proposal, verification is still action-neutral for Step 1 because
the combined prelude covers all four candidates. But verification
becomes action-relevant if Step 1 fails — at that point the leak is
something outside the four-candidate list, and diagnostic work is
the only path. So: verification is not unconditionally discarded;
it is deferred behind a cheap first-try bypass. Same bottom line,
more honest accounting.

## Topic 2 — Revised mechanism

**Position: zig-cc-as-CC via `.cargo/config.toml` + explicit env
unset, on the existing Mac mini runner. Staged test-execute via VPS
SSH-trigger only if Stage 1 proves insufficient.**

Peer ranking reversal to arbitrate:
- Codex-proxy: compiler replacement #1, Docker #2, ext CI #3, VPS
  relocation #4 (`codex-proxy.md:48-92`) — cost-ranked
- Gemini-proxy: VPS relocation #1, zigbuild #2, ext CI #3, Docker #4
  (`gemini-proxy.md:31-36`) — infrastructure-debt-ranked

Why codex is right about ordering for mengdie specifically, but
gemini's "eliminate the problem" frame has merit we can harvest:

**Against gemini's VPS-first framing**: gemini's frame is
"eliminate rather than layer." Sound principle, but the
elimination has a concrete cost gemini doesn't price:
- `gemini-proxy.md:33` scores VPS "complexity to undo: Low" — but
  this elides the *forward* cost of runner install, secrets
  migration, Forgejo→runner auth re-config, and ongoing OS patching.
- Codex's time estimate for VPS is "3–8 hours"
  (`codex-proxy.md:90`). That's 20–50x my zig-cc estimate.
- Gemini's architectural soundness argument applies to a team
  project, not a solo dev. For Kai, 3–8 hours of infra work has a
  real opportunity cost against the actual ML project work
  (`gemini-proxy.md:14`).
- Unquantified VPS CPU headroom (challenger C2,
  `challenger.md:86-98`) is a risk gemini asserts away.

**For gemini's SSH-trigger harvest**: the Option C pattern
(`gemini-proxy.md:67-69`) is the minimal way to get Linux
test-execute without a full runner install. Script on Mac mini
rsyncs + SSHs + runs cargo test on VPS. This is the Stage 2 fallback
I proposed above. It preserves the "VPS does the Linux work"
principle without the "stand up a second forgejo-runner" tax.

**Against codex Docker #2**: codex ranks Docker #2 but notes "2–4
hours if Docker/Colima already works; longer if not"
(`codex-proxy.md:83`). Docker is not installed
(`topic-02/summary.md:45`). Add ARM-on-x86 emulation concerns
(`codex-proxy.md:83`). This is strictly worse than zig-cc-as-CC for
Stage 1 on every axis. Reject for mengdie.

**Updated Topic 2 ranking** (criterion: get `cargo test` coverage
with minimum infrastructure):

| Stage | Mechanism | Setup | Ongoing | Addresses |
|---|---|---|---|---|
| **1 (ship now)** | zig-cc-as-CC via `.cargo/config.toml` + env unset | 15 min | zero | clippy-on-CI + test-compile on CI |
| **2 (only if Stage 1 insufficient)** | SSH-trigger to VPS for test-execute | 30–60 min (bash script, rsync, SSH key) | low | test-execute on Linux |
| Reject | Docker on Mac mini | 2–4 hr | medium | — |
| Reject | Full VPS runner relocation | 3–8 hr | medium (new host) | — |
| Reject | GitHub Actions mirror | 1–3 hr | medium (split CI) | — |

## Findings

1. My Round 1 `cargo zigbuild` proposal was mechanism-incomplete.
   `cargo zigbuild` is a build wrapper, not a clippy/test wrapper
   (`codex-proxy.md:69`). Corrected proposal: zig-cc-as-CC via
   `.cargo/config.toml` per codex Section 3.
2. My Round 1 "universal zigbuild" claim was overstated. Group A/B
   split is real (per architect). Counter: combining Group B with
   explicit env unset covers all four candidate mechanisms without
   verification.
3. Challenger C6 correctly narrows the gap to `cargo test` only.
   This reshapes Topic 2 into a two-stage problem (compile-in-CI
   vs execute-on-Linux), not a single-mechanism choice.
4. Challenger C5's pre-flight retest is a 5-minute precondition that
   may obviate the entire discussion. Adopted as Step 0.
5. Gemini's architectural "eliminate don't layer" frame prices
   forward setup cost at zero; for solo-dev, 3–8h of infra work is
   not free. Reject VPS-relocation-first. Harvest SSH-trigger as
   Stage 2 fallback.
6. Codex's zig-cc-as-CC (`codex-proxy.md:94-122`) is the strongest
   specific mechanism proposal in Round 1. Adopt directly, cite by
   reference.

## Agreements

- With **architect**: Group A/B split is real; Group B alone is
  mechanism-dependent (`architect.md:34-61`). Concede the bare
  zigbuild claim.
- With **challenger**: C5 pre-flight retest (adopted as Step 0);
  C6 narrowing of gap to `cargo test` only (reshapes Topic 2).
- With **codex-proxy**: compiler replacement ranking #1 for TCO;
  specifically the zig-cc-as-CC mechanism from Section 3; CFLAGS/
  SDKROOT explicit unset as part of the fix; `dtruss` avoided in
  favor of `CC_ENABLE_DEBUG_OUTPUT=1`.
- With **gemini-proxy**: SSH-trigger pattern for Stage 2
  test-execute (`gemini-proxy.md:67-69`) harvested as the
  minimum-viable Linux test lane.

## Disagreements

- With **architect** on T1 recommendation: architect's
  "verify-then-decide with 1-hour time-box"
  (`architect.md:83-88`) is a reasonable budget, but the combined
  zig-cc + env-unset bypass is mechanism-agnostic against all four
  candidates, so verification's decision value collapses. Verify
  becomes a Step 2 recovery lane, not a Step 1 prerequisite.
- With **architect** on T2 "if bypass-now, pivot to VPS relocation"
  (`architect.md:183-186`): this is only correct for a purely
  Group-B bypass. With env-unset included, Group A/B becomes a
  false dichotomy. Stage 1 on existing runner is viable.
- With **gemini-proxy** on T2 runner-relocation-first ranking
  (`gemini-proxy.md:33`): the forward setup cost (3–8 hr per
  codex) is not priced into "complexity to undo: Low." For
  solo-dev TCO it's strictly worse than Stage 1 + conditional
  Stage 2.
- With **gemini-proxy** on Docker rank (`gemini-proxy.md:36`):
  concur it's worst; already rejected in Round 1.
- With **codex-proxy** on runner relocation #4
  (`codex-proxy.md:89-91`): concur the time estimate is 3–8h.

## Open Questions

1. **Step 0 outcome**: does the 2026-04-17 failure still reproduce
   on the current runner? Every subsequent decision depends on this.
   Proposed: Kai runs the minimal failing invocation over SSH before
   Round 3.
2. **Does zig-cc-as-CC's bundled libc match the runtime target**?
   Codex notes "Zig glibc targeting has caveats, including static
   glibc not supported" (`codex-proxy.md:69`). Mengdie deploys
   dynamically linked via release.yml — should be fine, but worth
   a one-line sanity check in Stage 1.
3. **Is Stage 2 ever needed**? Stage 1 gives clippy-on-CI +
   test-compile-on-CI. The "refactor passes hooks, breaks e2e
   test" class is partially covered (compile-time test breakage
   caught). Missed: runtime test failures that only appear at
   execution. How often does mengdie's test suite catch runtime-
   only failures that compile-only would miss? If rarely, Stage 2
   may be permanently unnecessary.
4. **`.cargo/config.toml` scope**: codex's proposed config would
   apply to *all* invocations, including local dev. Does this
   interfere with Kai's aarch64-apple-darwin native builds?
   Proposed: keep the CI-specific zig-cc wrapper in
   `.ci/` or similar, and set the env var only in the workflow
   step, not in `.cargo/config.toml`. Avoids polluting local dev.
   (This is a Round 3 plan-level detail, not a Round 2 decision.)
5. **Runner-relocation ambiguity flagged in synthesis**: gemini's
   "VPS runner" (install forgejo-runner on VPS) ≠ my proposed
   Stage 2 "SSH-trigger" (no runner on VPS, just shell access).
   Different topologies; "relocation" conflates them. Round 3
   should pick terminology and distinguish.
