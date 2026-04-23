# Round 3 — Gemini Proxy: Root Cause Found, Decision Collapses

**Model**: Gemma 4 (via ollama, Gemini free tier exhausted)  
**Date**: 2026-04-22  
**Context**: UAG falsification failed; root cause identified in `.cargo/config.toml`

---

## Summary

The entire discussion of "bypasses" becomes **moot**. The mechanism is known, controlled, and within the codebase: a single `[env]` line in `.cargo/config.toml` added in commit af303d5 propagates to all Cargo targets, including Linux cross-compiles. The fix is not a "bypass" — it's a bug fix: remove or scope the entry.

Additionally: **codify the pattern**. "Check `.cargo/config.toml` `[env]` before assuming shell leakage" is a high-value heuristic for debugging cross-compilation failures, not a one-off lesson.

---

## Question 1: Mechanism Known, Bypass Needed?

**Answer: No bypass needed. It's a bug fix.**

A "bypass" is a strategy for circumventing an uncontrollable or unknown mechanism. Now that the mechanism is:
- **Known**: `.cargo/config.toml` `[env]` section
- **Identified**: Specific entry propagating to all targets
- **Owned**: Within the codebase, editable

...the problem is no longer "how do we work around a mysterious environment leak?" The problem is simply **"remove this incorrect configuration"**.

The 1-line fix:
- Either delete the problematic `[env]` block
- Or scope it target-specifically: `[target.'cfg(target_os = "macos")'.env]`

**Verdict on R1 relocation ranking**: The relocation-first logic was justified when the mechanism was unverified and risk was unquantified. With the mechanism now known and the fix obvious, the entire architectural decision framework (relocation vs. compiler replacement vs. Docker) becomes irrelevant. This was a **configuration error**, not a cross-compilation limitation.

---

## Question 2: Institutional Knowledge — Codify the Pattern?

**Answer: Yes, absolutely. It's not a one-off.**

This is a **shadow configuration trap** — a high-value debugging heuristic for anyone working with Cargo cross-compilation or CI/CD pipelines.

**Pattern to codify:**
> "When debugging environment leakage in Cargo-based cross-compilation builds, audit `.cargo/config.toml` `[env]` entries **before assuming leakage from the host shell or CI runner.** Cargo's `[env]` section propagates globally to all targets unless explicitly scoped via `[target.'cfg(...)'.env]`."

**Why it matters:**
- Developers debugging CI failures often look "outward" (shell, Jenkins, Docker) and forget to look "inward" (tool configuration).
- This is a frequent source of subtle bugs in Rust cross-compilation workflows.
- The fix is simple once identified, but discovery requires checking a file that's easy to overlook.

**Where to document:**
- Conclusion of this discussion (post-mortem section)
- `.claude/CLAUDE.md` or project debugging guide for future reference

**Cost of codifying:** Near-zero (one paragraph).  
**Value of codifying:** High (saves hours on the next cross-compilation debugging cycle).

---

## Implications for Discussion 020

This discussion's original framing (Topic 1: verify vs. bypass, Topic 2: which bypass) is now **historical**. The real outcome:

1. **Root cause**: Configuration, not mechanics.
2. **Fix**: One line, plus scoping if needed.
3. **Lesson**: Check `.cargo/config.toml` first.

The elaborate reasoning about runner relocation, zigbuild complexity, Docker trade-offs, and VPS CPU contention was all **contingent on the mechanism being unverified**. With verification complete and the cause simple, the discussion arc has resolved cleanly: unverified risk → verified root cause → straightforward fix.

**This is actually the ideal outcome for a design discussion:** thorough reasoning under uncertainty, then facts cut through the analysis and point to the simplest solution.
