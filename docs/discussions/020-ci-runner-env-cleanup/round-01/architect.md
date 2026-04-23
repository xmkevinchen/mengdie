---
agent: architect
round: 1
created: 2026-04-22
topics: [topic-01, topic-02]
---

# Round 1 — Architect

## Findings (with file:line evidence)

### Framing of the Decision Framework

The framing correctly identifies that Topic 1 and Topic 2 are sequenced,
not parallel: Topic 2 is contingent on Topic 1 choosing bypass. The
decision criterion for Topic 2 is explicit: **total cost of ownership for
a solo dev**, not technical elegance. This is the right lens. I will apply
it strictly.

---

### Topic 1: Verify-then-decide vs. Accept-bypass-now

**Position: Verify-then-decide.**

The key evaluative question in topic-01/summary.md (line 54) is correct:
"Does the choice of bypass depend on whether Apple-Clang is confirmed as
the injector? If yes → verify. If no → bypass-now is fine."

My evaluation: **yes, the bypass choice is mechanism-dependent**.

Here is why:

**Candidate bypasses split into two groups by assumption:**

Group A — bypasses that work *regardless* of the leak source:
- Runner relocation to Linux VPS (eliminates host macOS entirely)
- Docker executor on Mac mini (isolates from host env)

Group B — bypasses that only work *if the hypothesis is confirmed*:
- `cargo zigbuild` (replaces `/usr/bin/cc` with zig cc — only fixes the
  problem if the leak comes from Apple Clang invoked as `/usr/bin/cc`; if
  the leak comes from a `CFLAGS_*` env var in the cc-rs subprocess block
  or from a wrong `TARGET` propagation, zigbuild does not help)
- `CC_x86_64_unknown_linux_gnu=<linux-gcc>` (forces cc-rs to use a
  specific compiler — same caveat: only fixes the leak if the problem is
  compiler identity, not env vars in the cargo process block)

The analysis (analysis.md lines 64–88) lists four plausible leak sources
ranked by likelihood. The Apple-Clang xcrun-internal mechanism is the top
hypothesis, but the other three (CFLAGS* in execve env, CC wrapper/shim,
wrong TARGET propagation) are live alternatives. Group B bypasses fail
silently against candidates 2, 3, 4 — the CI would still break.

Group A bypasses are mechanism-agnostic and would succeed regardless.
But Group A options carry higher setup cost and maintenance surface.

**Therefore**: if verification confirms Apple-Clang (hypothesis 1), the
team can pick cheap Group B options (zigbuild single workflow-line change)
with confidence. If verification refutes it, the team avoids wasting
effort on Group B and targets the actual mechanism.

**The cost asymmetry is decisive**: verification is bounded at <30 min on
a machine with full SSH access (topic-01/summary.md line 43). The bypass
ships regardless — verification does not delay the bypass; it informs
*which* bypass. Under verify-then-decide, the worst case is: 30 min
spent, hypothesis refuted, one of the other leak sources identified, fix
targeted precisely. Under accept-bypass-now, the worst case is: Group B
bypass selected, deployed, CI still breaks because leak source is
CFLAGS* env or wrong TARGET — and the 30 min is spent anyway debugging
*why the bypass didn't work*, now with more moving parts.

**One caveat on the time estimate**: topic-01/summary.md line 45 notes
that `strace` may not be available on macOS for act subprocesses. The
analysis (analysis.md lines 126–128) references Codex's diagnostic
methodology as an alternative: compiler-wrapper logger + `CC_ENABLE_DEBUG_OUTPUT=1
cargo build -vv`. This is macOS-compatible and doesn't depend on strace.
If `/usr/bin/cc -v` returns early signal and the compiler-wrapper
technique runs cleanly, the estimate holds. If the wrapper approach fails
to capture the relevant env, the time-box should be 1 hour (analysis.md
line 150), after which the team falls to bypass regardless.

**Recommendation: verify-then-decide with a 1-hour hard time-box.**
If the compiler-wrapper diagnostic (analysis.md lines 126–128) produces
a definitive result before the box expires, let it pick the bypass. If
the box expires without a result, fall immediately to `cargo zigbuild`
as the first bypass attempt (cheapest Group B option; already installed
per analysis.md line 55).

**Institutional value argument** (secondary, but real): having the
mechanism documented means future cc-rs/ring/clang version bumps can be
evaluated against a known root cause rather than re-deriving it from
scratch. For a solo dev who may not touch this code for months, that
documented root cause has non-trivial value. The 30 min investment pays
forward.

---

### Topic 2: Bypass Mechanism Selection (contingent on bypass)

**Position: `cargo zigbuild --target x86_64-unknown-linux-gnu` as first
choice, with `CC_x86_64_unknown_linux_gnu` as second; both contingent on
verification confirming hypothesis 1.**

**If verification is skipped (Topic 1 → accept-bypass-now), the decision
changes**: the team cannot rely on Group B bypasses and must choose
Group A. In that case my recommendation shifts to the Linux VPS runner
relocation (see below).

Applying the solo-dev TCO criterion to each candidate:

**1. `cargo zigbuild --target x86_64-unknown-linux-gnu`**
- Setup cost: single workflow-line change. `cargo-zigbuild` is already
  installed (analysis.md line 55). Zero new dependencies.
- Ongoing maintenance: near-zero. `cargo-zigbuild` version bumps are
  infrequent; zig cc headers are self-contained.
- Failure mode: if the leak is NOT from Apple Clang as cc replacement
  (hypothesis 2/3/4 confirmed), the CI still fails. Failure is noisy
  (same error). Debugging now has more moving parts (zigbuild layer added).
- Mechanism dependency: YES — only correct if hypothesis 1 is confirmed.
- TCO verdict: lowest total cost *when mechanism is confirmed*. Best pick
  post-verification.

**2. `CC_x86_64_unknown_linux_gnu=<linux-cross-gcc>`**
- Setup cost: one workflow env var + Homebrew install of
  `x86_64-linux-gnu-gcc` (or equivalent). ~15 min.
- Ongoing maintenance: Homebrew package updates. Low but nonzero.
- Failure mode: same as zigbuild if hypothesis 1 is wrong.
- Mechanism dependency: YES.
- TCO verdict: slightly higher than zigbuild (extra dependency). Use if
  zigbuild has a compatibility issue with the specific ring version.

**3. Docker executor on Mac mini**
- Setup cost: Docker Desktop install (~10 min) + runner reconfiguration.
  One-time, not reversible without downtime.
- Ongoing maintenance: Docker Desktop updates, container image pulls
  (rust:latest or pinned). Non-trivial for solo dev.
- Failure mode: if Docker is misconfigured, CI fails in a new way.
  Docker adds a thick debugging layer.
- Mechanism dependency: NO — isolates from host macOS entirely.
- TCO verdict: medium setup, medium ongoing. Correct option if hypothesis
  is wrong AND runner cannot be relocated. analysis.md line 168 ranks it
  3rd. I concur.

**4. Linux VPS runner relocation**
- Setup cost: install forgejo-runner on the VPS, configure, test. ~30-60 min.
- Ongoing maintenance: VPS OS updates, runner binary updates. Already
  maintaining the VPS (hosts Forgejo). Marginal cost is low.
- Failure mode: adds CPU contention risk to Forgejo serving
  (topic-02/summary.md line 39). Risk is real but measurable.
- Mechanism dependency: NO — eliminates macOS host class of bugs entirely.
- TCO verdict: medium setup, low ongoing (marginal on existing VPS
  maintenance). The CPU contention risk is the primary concern. For
  mengdie's current load (solo dev, infrequent CI runs), the risk is
  low in practice. This is the mechanism-agnostic option with the lowest
  ongoing maintenance *if* the VPS has headroom.

**5. GitHub Actions mirror**
- analysis.md line 133 + topic-02/summary.md line 47: private repo privacy
  concern, split CI surface. Cross-team consensus to avoid.
- I agree. Not recommended.

**6. cc wrapper shim**
- analysis.md line 130: treats symptom, brittle against cc-rs/ring version
  bumps. Cross-team consensus to avoid.
- I agree. Not recommended.

**Summary for Topic 2**:

| Candidate | Mech-dep | Setup | Ongoing | Recommended |
|---|---|---|---|---|
| zigbuild | YES | ~1 min | near-zero | Yes (if hyp confirmed) |
| CC_xxx=linux-gcc | YES | ~15 min | low | Fallback after zigbuild |
| Docker on Mac mini | NO | ~30 min | medium | If VPS unavailable |
| Linux VPS runner | NO | ~45 min | low (marginal) | If hyp refuted / bypass-now |
| GitHub Actions | NO | ~60 min | medium | Avoid (privacy) |
| cc wrapper shim | NO | ~20 min | high | Avoid |

**If Topic 1 → verify-then-decide and hypothesis confirmed**: zigbuild.
**If Topic 1 → verify-then-decide and hypothesis refuted**: pivot to Linux
VPS runner (mechanism-agnostic, lowest ongoing marginal cost, already
maintaining the VPS).
**If Topic 1 → accept-bypass-now**: Linux VPS runner (cannot assume
hypothesis is correct; zigbuild risks silent non-fix; VPS runner is
mechanism-agnostic and has the best ongoing TCO of the mechanism-agnostic
options).

---

## Agreements (N/A Round 1)

## Disagreements (N/A Round 1)

## Open Questions

1. **VPS CPU headroom**: What is the actual load on the Linux VPS hosting
   Forgejo? A single-user Forgejo instance with infrequent CI runs likely
   has ample headroom, but this should be confirmed before recommending
   runner relocation as a primary option (not just fallback).

2. **Verification time-box discipline**: If the compiler-wrapper diagnostic
   is run and produces ambiguous results (e.g., `-isysroot` does not
   appear in the logged argv, pointing to hypothesis 2/3/4), does the team
   have a clear next step? The analysis provides four candidates but no
   secondary diagnostic methodology for them.

3. **`cargo zigbuild` version pinning**: The analysis notes zigbuild shims
   currently bind only `aarch64-apple-darwin` (analysis.md line 55). Using
   zigbuild for `x86_64-unknown-linux-gnu` requires explicitly specifying
   the target in the workflow. Is there any risk that the existing
   aarch64 shims interfere? Needs one quick test before committing.

4. **006 vs BL-ci-full-clippy-test consolidation**: The analysis recommends
   closing 006 as superseded (analysis.md line 182). The framing explicitly
   defers this to `/ae:plan` time. I agree with the deferral — this is a
   plan-time bookkeeping decision, not a design decision.

5. **`release.yml` race fix**: Framing marks this out-of-scope as a
   drive-by fix (framing.md line 88). The analysis agrees (analysis.md
   lines 173–175). This should be tracked as a plan-time addition.
