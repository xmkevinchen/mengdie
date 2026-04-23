---
role: challenger
round: 1
discussion: "020"
created: 2026-04-22
---

# Challenger — Round 1

## Findings

### Topic 1: Verify-then-decide vs. accept-bypass-now

**Challenge 1.1 — The 30-min estimate is load-bearing and undefended.**

The framing and topic summary treat "<30 min" as near-certain. It isn't. The
specific verification path includes `strace`/`dtruss` on an act-spawned
subprocess. On macOS, `dtruss` requires SIP disabled — which is not the
default. `strace` is Linux-only. Neither tool can observe the env block at the
`execve()` boundary inside an act-spawned subprocess *from outside that
process* under a default macOS security configuration. The 30-min estimate
implicitly assumes the tracing tool works cleanly. If it doesn't, "30 min"
becomes an open-ended investigation with no natural stop condition — and the
sprint has no stated time-box before pivoting to bypass anyway.

Concrete failure mode: Kai opens SSH, runs the compiler-wrapper approach from
Codex's methodology, the wrapper *does* fire but produces garbled output
because ring's build.rs uses `cmake` or a sub-make that re-forks cc. Now the
investigation is at 45 min and the exec boundary is still ambiguous. There
is no plan for this case.

**Challenge 1.2 — "Verification materially changes bypass choice" is asserted, not shown.**

The framing's pivotal claim is: "if verification changes which bypass is
right, verify; otherwise, bypass-now is fine." But the analysis already
ranked bypass options without knowing the confirmed mechanism. Zigbuild
is #1 regardless — it replaces `/usr/bin/cc` for the C compile step
entirely, so it's robust against *all four* candidate leak sources (env vars
at execve, cc shim, Apple Clang internal xcrun, broken TARGET propagation).
If zigbuild is bypass-best under every confirmed-mechanism scenario, then
verification does NOT materially change the bypass choice. The framing
hasn't addressed this: it lists "does verification change the bypass?" as an
open question while the analysis's own ranking implicitly answers it no.

**Challenge 1.3 — Institutional-knowledge argument is weaker than it appears.**

The topic summary mentions institutional value in having the mechanism on
record. But the mechanism, even if confirmed, is specific to: act version,
macOS version, Apple Clang version, ring build.rs version, and the runner's
specific host setup. This tuple changes constantly. The "confirmed mechanism"
will be stale the next time any one of these versions bumps. For a solo-dev
project where CI topology changes are not rare, knowing that `/usr/bin/cc` is
Apple Clang *today* has a shorter half-life than the time invested in
confirming it.

**Challenge 1.4 — The framing does not consider the case where verification confirms the hypothesis but the fix is still bypass.**

"Confirmed hypothesis → bypass is the only remaining option" (topic summary,
verify-then-decide branch). So verification's upside is "confirms the bypass
rationale." But bypass ships anyway. The only genuine branch is: refuted
hypothesis → new investigation. And refutation branches back into open-ended
investigation, not a fast resolution. Verification's actual decision value is
asymmetric and narrow: it only matters if the hypothesis is *wrong*. If the
hypothesis is right (which the analysis judges likely), verification just
adds latency before shipping the same bypass. That's not clearly worth it.

---

### Topic 2: Bypass mechanism selection

**Challenge 2.1 — Zigbuild-for-Linux is unverified against the actual failure.**

`cargo-zigbuild` is installed, but the constraint note says its shims bind
`aarch64-apple-darwin` only — not Linux targets. The claim that
`cargo zigbuild --target x86_64-unknown-linux-gnu` works on this runner for
ring has not been tested. Zig's bundled libc headers are for a specific zig
version; ring's build.rs uses `cmake` + hand-rolled C files that may have
their own implicit sysroot expectations not covered by zig's libc. "Installed"
does not mean "wired and tested for Linux."

A bypass that is ranked #1 but untested on the actual failure is fragile. If
zigbuild for Linux fails on ring specifically, the team discovers this *after*
choosing it over alternatives, costing more time than verifying it up front
would have.

**Challenge 2.2 — Runner relocation's CPU contention risk is under-quantified.**

The Linux VPS already hosts Forgejo. The constraint note says "CPU contention
risk." But this is not quantified. On a small VPS:
- Forgejo serving is mostly idle; spikes on push.
- A `cargo test` CI job for mengdie would spike CPU for 30–60 s per job.
- These spikes might happen simultaneously if a push triggers CI.

The team cites this risk as a reason to rank runner relocation lower, but it
hasn't established whether the VPS is actually resource-constrained or has
headroom. "Risk of contention" is not the same as "contention will happen."
This could be a non-issue that is being used to eliminate the simplest
long-term option (a real Linux runner with no host env bleed).

**Challenge 2.3 — Docker executor option has a hidden prerequisite dependency gap.**

Round 0 restored Docker executor as a live option. But the constraints note
"No Docker installed on the Mac mini; install is possible but carries ongoing
maintenance." The "ongoing maintenance" assertion is unsubstantiated. On
macOS, `colima` or Docker Desktop both require periodic updates; Lima-managed
VMs occasionally need manual intervention after macOS upgrades. This is not
free maintenance. More critically: a Docker executor requires the CI job to
pull or prebuild a Rust image. Cold-pull latency for `rust:latest` (~1GB
compressed) on a home network hits CI per-run time and can cause timeout
failures. This ongoing friction is absent from the TCO comparison.

**Challenge 2.4 — The analysis dismisses the `CC_x86_64_unknown_linux_gnu` option too quickly.**

The analysis ranks zigbuild #1 and `CC_x86_64_unknown_linux_gnu=<linux-cross-gcc>` #2,
but the #2 option requires "Homebrew `x86_64-linux-gnu-gcc` or equivalent"
which may not be installed. However, the framing and analysis don't mention
whether the Mac mini has any cross-compilation toolchain for x86_64-linux
*other than* zig. If Homebrew cross-gcc is already present (or trivially
installable), this is a single env var in the workflow — no new toolchain
runtime, no zig version dependency, no bundled libc version dependency. The
analysis skips checking the runner state before ranking options, which means
the rank is based on assumptions about what's installed.

**Challenge 2.5 — "One bypass mechanism" framing ignores staged options.**

All bypass candidates are framed as mutually exclusive choices. But a
pragmatic sequence is: try zigbuild-for-Linux first (cheapest, already
installed), with an explicit fallback to VPS runner if zigbuild fails on
ring. This isn't in scope as a "mechanism" in the topic. If the team picks
zigbuild as "the bypass" and it silently fails in a non-obvious way (e.g.,
ring compiles but links against zig's bundled libc, causing runtime failures
in `cargo test`), there's no escape hatch articulated.

---

### Cross-cutting: What the framing may have MISSED

**Missed concern — The reproduced failure may not be reproducible after any env change.**

The original debugging (006 backlog) established that the failure is specific
to act's subprocess context. But that context includes act's version,
runner's act-runner version, and the specific way act constructs the subprocess
env. Any of these could change between the original failure and now. If the
runner has been updated since the failure was observed (forgejo-runner v6.3.1
was the version at time of debugging — the current version is unknown), the
failure may no longer be reproducible at all. The entire discussion is premised
on a failure that was last confirmed in April 2026 debugging commits, which
were reverted. No one has confirmed the failure is still present before
planning a bypass.

**Missed concern — Pre-commit hook already covers clippy; the CI gap is exclusively `cargo test`.**

The analysis flags this as a meta-challenge: "the pre-commit hook covers
fmt+clippy locally. Only `cargo test` is a genuine uncaught gap." But neither
the framing nor the topic summaries incorporate this narrowing. If the real
gap is *only* `cargo test`, then the bypass selection criterion should be
"what's the cheapest CI path that runs `cargo test` reliably" — not "what
cleans up the whole CI env problem." A targeted fix (e.g., a dedicated
`cargo test` job that uses zigbuild only for ring's C compile, while the rest
of the job runs normally) is not mentioned.

---

## Agreements

N/A (adversarial role — proposing challenges, not synthesis)

## Disagreements

N/A (adversarial role — proposing challenges, not synthesis)

## Open Questions

1. Is the `-isysroot` failure still reproducible on the current runner state?
   When was it last confirmed?
2. Does `cargo zigbuild --target x86_64-unknown-linux-gnu` on this runner
   produce a working `ring` build — tested, not assumed?
3. What is the VPS's actual CPU headroom? RAM? Is the concern evidence-based
   or precautionary?
4. What is the current act/forgejo-runner version on the runner vs. the
   version at the time the failure was logged?
5. Is Homebrew `x86_64-linux-gnu-gcc` installed on the Mac mini? If yes,
   does `CC_x86_64_unknown_linux_gnu` rank above zigbuild on simplicity?
6. If verification overruns 30 min, what is the stated time-box before
   the team pivots to bypass — and who decides?
