---
role: challenger
round: 2
discussion: "020"
created: 2026-04-22
---

# Challenger — Round 2

## Findings

---

### A. Codex vs. Gemini inverted T2 ranking: whose cost framing is load-bearing?

Codex (codex-proxy.md) ranks: compiler replacement #1, Docker #2, External CI #3,
runner relocation #4. Gemini (gemini-proxy.md) ranks: runner relocation #1,
zigbuild #2, External CI #3, Docker #4. Same options, directly reversed on the
top and bottom positions. Both claim to apply "solo-dev TCO."

**The divergence traces to which cost dimension each agent weighted.**

Codex's framing treats setup time as the dominant cost variable. Compiler
replacement wins because it is lowest setup: "30–120 minutes" (codex-proxy.md
line 56). Runner relocation loses because it is highest setup: "3–8 hours ...
another host to patch, secure, back up" (codex-proxy.md line 91). Codex is
explicitly optimizing for "best first move" — a sprint-time framing.

Gemini's framing treats ongoing infrastructure debt as the dominant cost
variable. Runner relocation wins because it "eliminates cross-compilation
entirely" and has "Lowest: just stop using the Mac runner" undo cost
(gemini-proxy.md line 34). Docker loses because it "hides the host's reality"
and creates future debugging debt (gemini-proxy.md line 36).

**Neither framing is wrong, but they are answering different questions.**
Codex answers: "what unblocks CI this sprint?" Gemini answers: "what is
the best long-term CI topology for this project?" These are not the same
question, and the framing (framing.md lines 72–74) explicitly picks
"total cost of ownership" as the criterion — not "fastest unblock." On
the stated criterion, Gemini's cost dimension (ongoing maintenance +
infrastructure debt) is more load-bearing than Codex's (setup time).

**However, Gemini's runner relocation ranking has a concrete defect.**
Gemini (gemini-proxy.md line 34) asserts undo cost is "Low — just stop
using the Mac runner." This is false for the VPS case. Runner relocation
to the Linux VPS *couples* CI reliability to Forgejo serving. If the VPS
goes down — planned maintenance, OOM, disk fill — CI and code hosting fail
simultaneously. "Just stop using the Mac runner" doesn't capture the
blast radius. Gemini's undo-cost claim is wrong in the direction that makes
relocation look better than it is.

**Unresolved tension**: if setup time is discounted (Gemini's view) and
ongoing debt is the criterion, then the Linux VPS runner is the long-term
winner *if and only if* the VPS has adequate headroom. That's the same
unmeasured claim from challenger Round 1 C4. Gemini's ranking is
evidence-free on the headroom question — it asserts the VPS is the golden
path without verifying the premise it depends on.

---

### B. Architect's Group A/B split: real decision axis or false framing?

Architect (architect.md lines 36–53) splits bypass candidates into:
- Group A: mechanism-agnostic (VPS runner, Docker) — "succeed regardless"
- Group B: mechanism-dependent (zigbuild, CC_env) — "only correct if
  hypothesis 1 confirmed"

Minimal-change-engineer (minimal-change-engineer.md lines 27–44) directly
challenges this by claiming zigbuild is Group A: "zigbuild still works
[if hypothesis refuted] — it brings its own bundled libc headers and zig-cc,
sidesteps the host cc and its env."

**The split is real but architect's boundary condition is overstated.**

Minimal-change's universal-zigbuild claim holds for leak sources (a) and (c)
— CFLAGS* env vars in cc-rs's execve block and Apple Clang as `/usr/bin/cc`.
Zigbuild replaces the C compiler invoked by cc-rs entirely; it doesn't read
CFLAGS* from the host env because it uses zig cc's own bundled headers.

But minimal-change's claim **does not hold for leak source (d)**: "Wrong
`TARGET` / `CARGO_CFG_TARGET_*` reaching build.rs" (analysis.md line 76).
If ring's build.rs receives `CARGO_CFG_TARGET_VENDOR=apple` or a broken
target triple from cargo's process env, ring's own CMakeLists.txt may
still select Apple-specific compilation paths *regardless of which cc
binary zig provides*. Zigbuild wraps the C compiler but not the cargo
process env or CMake logic. If source (d) is the actual leak, zigbuild
fails silently.

**The Group A/B split is therefore partially real.** Group B is narrower
than architect draws (it collapses under sources a/c but not under d), and
Group A is the correct hedge against source (d). But the practical
consequence is the same as architect's conclusion: if verification is
skipped, Group A is the safer pick because it is mechanism-agnostic across
all four candidate sources, not just the top hypothesis.

**Architect's conditional recommendation (architect.md lines 179–186)
follows from this correctly**: bypass-now → VPS runner. Minimal-change's
universal-zigbuild claim works for the 75% case (sources a/c) but isn't
provably universal for source (d). The split is a real decision axis;
it is just narrower than architect drew it.

---

### C. Minimal-change's zigbuild-is-universal: what evidence confirms it?

Minimal-change's position requires zigbuild to work under **all four** leak
sources. As shown above, source (d) breaks the universality claim. But
the other three sources are partially addressable by evidence:

**What a dry-run actually confirms:**

A dry-run (`cargo zigbuild --target x86_64-unknown-linux-gnu`, no-run, on
the failing runner) has two possible outcomes:
1. Build succeeds — confirms zigbuild sidesteps whatever the leak is. Does
   not tell you which source caused the original failure, but establishes
   the bypass works in practice.
2. Build fails — confirms zigbuild is not universal; narrows the leak to
   source (d) (broken TARGET propagation or CMake logic), since that is the
   only source that survives zigbuild replacement.

**The dry-run is therefore itself diagnostic.** It is not merely a bypass
test; it is a bounded verification that answers the Group A/B question
*without* needing the full compiler-wrapper diagnostic. Outcome 2
immediately moves the team to Group A options.

**The cost of a dry-run is less than 30 minutes.** It requires no SIP
changes, no dtruss, no compiler-wrapper scaffolding. SSH into the runner,
run one command.

**The unresolved question is whether the dry-run is expensive.** The TL
asked: "What if the dry-run is expensive?" On this runner, the first
`cargo zigbuild --target x86_64-unknown-linux-gnu` cold-downloads the zig
toolchain and Linux libc headers. On a slow home network (ckai-macmini.local,
constrained upstream), this could take 5–15 minutes before the actual build
fires. The download is one-time; subsequent runs are cached. This is not
"expensive" in any meaningful sprint-budget sense. The dry-run cost claim
does not rescue the "skip-verification" argument against a Group A fallback.

---

### D. Gemini's hybrid multi-runner: simpler or hiding complexity?

Gemini (gemini-proxy.md lines 62–68) proposes a hybrid strategy: Runner A
(macOS) for clippy/check, Runner B (Linux VPS) for build + integration tests.

**This is not simpler. It is two independent problems presented as a solution
to one.**

Specifically:

1. **Runner A on macOS is already the existing setup.** Pre-commit hook
   covers fmt+clippy locally. The proposal adds a macOS CI runner for
   something the local hook already handles. Challenger Round 1 C6 established
   that the only uncovered CI gap is `cargo test`. Runner A in Gemini's hybrid
   does not close any new gap.

2. **Runner B requires the same runner relocation work that is under evaluation
   as a standalone option.** The hybrid doesn't simplify runner relocation; it
   adds orchestration on top of it. The workflow must now coordinate across two
   runners: if Runner A (clippy) fails, should Runner B (test) be skipped? That
   requires a `needs:` dependency in the workflow, which is the same `release.yml`
   race the framing explicitly marks out-of-scope.

3. **"Separates code quality from deployment readiness"** (gemini-proxy.md
   line 67) is a valid design pattern for large teams. For a solo-dev project
   where the pre-commit hook already provides the "code quality" signal, this
   separation adds no new value and costs a second runner to maintain.

The hybrid multi-runner hides its complexity in the orchestration layer
(workflow coordination, runner label routing, two runner lifecycles) and
provides no benefit over a single Linux VPS runner that runs both clippy and
test natively. Gemini's "benefit" (separation of concerns) is a solution to
a problem solo-dev mengdie does not have.

---

### E. C5 escalation — should "failure may not reproduce" be a hard blocker?

Challenger Round 1 C5 flagged: the failure was last confirmed in April 2026
debug commits (e4b8cbf through 6658248), all reverted. The runner version,
act version, and macOS version are all potentially stale. If the failure
doesn't reproduce now, Topic 1 collapses entirely.

**This should be a hard preflight, not a soft advisory.**

The synthesis (synthesis.md line 35) marks this as "adds a cheap pre-flight
retest before Topic 1 even decides." But framing it as "cheap" understates
what it changes. If the failure no longer reproduces:

- The entire verify-then-decide vs. bypass-now debate is moot.
- Both backlog items (006 and BL-ci-full-clippy-test) resolve trivially.
- The team avoids shipping a bypass for a problem that no longer exists —
  which would add a permanent zigbuild dependency to the CI workflow with
  no corresponding benefit.

**"Retest first, then decide" is itself a decision — the right one.** It is
not a bypass of Topic 1; it is a prerequisite that validates whether Topic 1
needs to happen at all. Skipping it to preserve momentum in the discussion
is exactly the kind of "we decided without looking" that the framing
identified as a weakness of accept-bypass-now.

**The hard blocker form**: before any Round 2 agent positions lock in on
Topic 1 or Topic 2, the runner should be retested with the current ci.yml
workflow expanded to include a `cargo build --target x86_64-unknown-linux-gnu`
step (add a temporary debug job). If the build succeeds, close both BL items
as "no longer reproducing, resolved by upstream change." If the build fails,
proceed with Round 2 as normal. Either outcome takes less time than the
current discussion.

**What "retest first" does NOT resolve**: even if the failure still reproduces,
the question of which bypass to pick remains open. Retest is a gate, not a
solution. The discussion continues as scoped if the gate fails.

---

### F. Runner-relocation ambiguity still unresolved

Synthesis (synthesis.md line 63) flagged that gemini-proxy and codex-proxy
appear to mean different things by "runner relocation." Reading both files:

- **Gemini** (gemini-proxy.md lines 33–34): "just stop using the Mac runner"
  — implies registering the existing Linux VPS as the *sole* Forgejo runner
  and decommissioning the Mac mini runner.
- **Codex** (codex-proxy.md lines 89–92): "Linux VM/mini box plus runner
  registration/cache/secrets" — implies provisioning a new, dedicated runner
  host, separate from the Forgejo host.

These are different proposals with different risk profiles. Gemini's version
shares the VPS host with Forgejo (contention risk). Codex's version implies
a third machine that doesn't exist in mengdie's current infrastructure.
Codex's "runner relocation" ranking of #4 may partly be driven by the
assumption that this requires new hardware — which it doesn't if the Forgejo
VPS is the target. Gemini's #1 ranking may partly be driven by underestimating
the contention risk of co-locating with Forgejo.

Both agents are ranking different things under the same label. The discussion
cannot converge on runner relocation without pinning which topology is meant.

---

## Agreements

N/A (adversarial role)

## Disagreements

N/A (adversarial role)

## Open Questions

1. **Preflight retest gate**: run `cargo build --target x86_64-unknown-linux-gnu`
   inside a CI job on the current runner before any other decision. Result
   determines whether Topic 1 is live or moot.
2. **Runner relocation topology**: does "Linux VPS runner" mean (a) move the
   runner *onto* the existing Forgejo VPS (contention risk), or (b) provision
   a second, dedicated runner host (requires new infrastructure)? These have
   different cost profiles and neither agent has pinned the answer.
3. **Zigbuild dry-run as binary verification**: does the team agree that a
   single `cargo zigbuild --target x86_64-unknown-linux-gnu` run on the
   failing runner is sufficient to determine whether zigbuild is Group A or
   Group B? If yes, this supersedes the longer compiler-wrapper diagnostic.
4. **Source (d) test coverage**: if zigbuild bypasses but ring's `cargo test`
   later fails at runtime (zig-libc vs. VPS glibc mismatch), what is the
   escalation path? Neither minimal-change nor architect's position covers this.
5. **VPS headroom measurement**: what is the concrete VPS spec (CPU, RAM)?
   Every Gemini-side argument for runner relocation depends on this, and it
   remains unverified across two rounds.
