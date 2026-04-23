---
agent: minimal-change-engineer
iteration: 2
verdict_state: REVISE
timestamp: 2026-04-22T22:03:00
---

# Minimal-change-engineer — v2 Framing Review Verdict

**REVISE (one small cut)**.

**What v2 got right**:
- Core decision is now a clean binary (verify-then-decide vs. accept-bypass-now).
- Bypass mechanism explicitly contingent ("only decides if bypass wins").
- 006/BL scope boundary + release.yml race correctly moved to Out.
- Apple-Clang properly marked unverified hypothesis.

**Residual coupling — one line**:
Scope-In lists "runner-mode changes (host-mode vs. Docker executor)" as in-scope, flat. But runner-mode is already one of the four bypass mechanisms enumerated elsewhere (compiler replacement / runner-mode / runner relocation / external CI). Hoisting it to its own Scope-In bullet re-elevates a sub-option back to first-class, which is exactly the v1 coupling pattern in miniature — a reader can reasonably ask "are we deciding runner-mode in this discussion or not?" and get inconsistent answers.

**Suggested fix**: delete the Scope-In bullet about runner-mode entirely. Scope-In then reads cleanly as "core decision + contingent mechanism pick," with runner-mode sitting inside the contingent mechanism list where it belongs. The 017 annotation ("target matrix settled, not executor mode") can stay as a parenthetical on the mechanism bullet.

Everything else is appropriately simple. One delete and this is APPROVED.
