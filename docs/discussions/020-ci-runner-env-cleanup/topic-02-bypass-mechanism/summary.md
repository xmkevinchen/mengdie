---
id: "02"
title: "Bypass mechanism selection (contingent on bypass being selected)"
status: converged
current_round: 3
created: 2026-04-22
decision: "Does not activate. Contingent on Topic 1 selecting bypass; Topic 1 selected direct fix instead. No bypass mechanism needed."
rationale: "Topic 2 was explicitly contingent ('only decides if bypass wins') per framing. Once Topic 1 resolved to direct root-cause fix (the `.cargo/config.toml` [env] discovery), the contingency never triggered. Mechanism rankings gathered in Rounds 1-2 are preserved in the discussion record as context should a future bypass scenario recur, but no mechanism is selected."
reversibility: "high"
reversibility_basis: "If the root-cause fix fails downstream (e.g., removing the line breaks builds and the fallbacks don't suffice), the discussion record retains complete mechanism rankings (zig-cc-as-CC / Docker / VPS / external CI) that can be revived without redoing research."
---

# Topic: Bypass mechanism selection

## Current Status
**Converged** (as not-activated). Topic was explicitly contingent on Topic 1 → bypass, which didn't happen.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | Mechanism rankings surfaced: codex #1 compiler replacement; gemini #1 runner relocation; minimal-change zigbuild; architect zigbuild-or-VPS-depending-on-mechanism. New options surfaced: hybrid multi-runner, Zig-as-linker. |
| 2 | pending | Converged on zig-cc-as-CC via `.cargo/config.toml` + env-unset belt-and-suspenders. Conditional Stage 2 (VPS SSH-trigger or runner-on-VPS) if test execution on Linux required. |
| UAG | **OVERTAKEN** | Topic 1 root-cause find made Topic 2 irrelevant. |
| 3 | converged (not-activated) | No mechanism selected. Rankings archived as future-reference context. |

## Archived Mechanism Rankings (not active decisions — future reference only)

| Mechanism | Pros | Cons | Ranking context |
|-----------|------|------|-----------------|
| Delete unconditional global `CFLAGS` in `.cargo/config.toml` | Root-cause fix; 1 line; reversible | Requires local build verification | **Actual ship decision (Topic 1)** |
| zig-cc-as-CC via `.cargo/config.toml` | Already-installed toolchain; bypasses cc-rs Apple detection | Masks rather than fixes root cause; covers compile but not test execution | R2 convergent bypass |
| Runner-on-VPS (Linux runner added to Forgejo-hosting VPS) | Fully sidesteps macOS class of issues | CPU contention risk (unmeasured); deprecates Mac mini runner | Gemini R1 #1 |
| Docker executor on Mac mini | Community-canonical; container has no Xcode | Linux VM overhead on macOS; install + maintenance | Codex R1 #2 |
| External CI mirror (GitHub Actions) | Free-tier infra; separate from Forgejo | Privacy (private repo); splits CI surface | Codex R1 #3 |
| Hybrid multi-runner (mac lint, VPS build) | Distributes load | Orchestration overhead; zero new coverage over pre-commit | Gemini R1; self-reversed R2 |
| cc wrapper shim (strip `-isysroot`) | No runner change | Treats symptom; fragile against version bumps | Rejected across all rounds |

**Recurrence note**: if a `.cargo/config.toml` `[env]` leak recurs after this fix (or a different cargo-level injection surfaces), the Round 1+2 research is the starting point, not a redo.
