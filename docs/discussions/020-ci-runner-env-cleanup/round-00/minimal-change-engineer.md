---
agent: minimal-change-engineer
verdict_state: REVISE
timestamp: 2026-04-22T22:00:00
---

# Minimal-change-engineer — Framing Review Verdict

**REVISE**: framing couples three decisions that should decouple.

The core decision is binary: **keep investigating the `-isysroot` root cause, or accept a bypass?** Everything else is either a cheap prerequisite or unrelated work.

**Specific over-complication**:

1. **Scope-boundary question is premature.** Whether `006-ci-runner-env-cleanup` and `BL-ci-full-clippy-test` "collapse, sequence, or remain independent" is downstream of the investigate-vs-bypass call. If bypass wins, 006 closes as "won't fix, superseded by BL"; if investigation wins, 006 is the investigation and BL is the follow-on. The framing pre-loads a structural decision that falls out of the primary one.

2. **`release.yml` race doesn't belong.** The framing itself admits it's "unrelated mechanically, entangled operationally." Operational entanglement means "touch the same file" — that's a merge-order concern, not a discussion topic. Pull it out as a drive-by fix in whichever plan touches `release.yml` first.

3. **Hypothesis verification is a task, not a scope item.** "How much further investigation to spend before accepting bypass" presumes the Apple-Clang hypothesis stays unverified. One command on the runner (`/usr/bin/cc -v` + strace/dtruss on a trivial `ring` compile) either confirms or kills it in <30min. Run it first, then the decision collapses: confirmed → bypass is the only option; denied → keep digging with a new hypothesis.

**Suggested simpler framing**:

> **Problem:** CI blocked on `-isysroot` during `ring` compile on macOS host runner targeting Linux. cc-rs source rules out cc-rs as the injector; working hypothesis is `/usr/bin/cc` = Apple Clang calling `xcrun`.
>
> **Decision:** Given a <30min verification step exists, do we verify-then-decide, or accept bypass now (containerized runner / cross-compile via `cross` / offload to GitHub Actions free tier)?
>
> **Scope:** In — verification plan + bypass option selection for v0.8.0. Out — scope reshuffle between 006/BL (falls out of decision), `release.yml` race (separate 2-line fix), CI target matrix (settled in 017).

That's one decision with a clear tiebreaker (verification cost vs. bypass cost), not three coupled ones.
