---
round: 1
date: 2026-04-22
role: TL synthesis (index + orientation only — per-agent files are primary)
---

# Round 1 Synthesis

**Read the per-agent files directly.** This synthesis is an orientation index, not a replacement.

## Per-agent file index

| Agent | File | T1 position | T2 position |
|-------|------|-------------|-------------|
| architect | [architect.md](architect.md) | verify-then-decide (1h time-box) | zigbuild if confirmed; VPS if refuted |
| minimal-change-engineer | [minimal-change-engineer.md](minimal-change-engineer.md) | accept-bypass-now | zigbuild (zero ongoing) |
| challenger | [challenger.md](challenger.md) | no position — 6 challenges | challenges zigbuild's viability vs. ring |
| codex-proxy | [codex-proxy.md](codex-proxy.md) | verify 30–90min (avoid dtruss; use `CC_ENABLE_DEBUG_OUTPUT=1`) | compiler replacement #1 > Docker #2 > External CI #3 > Runner relocation #4 |
| gemini-proxy | [gemini-proxy.md](gemini-proxy.md) | verify 30min | Runner relocation #1 >> zigbuild >> external CI >> Docker |

## 1. Pruned

Pruned: nothing; all 5 inputs advanced. No position was dismissed as off-topic or
evidence-free in Round 1 — every input cites either file/line artifacts or
concrete cost numbers, even dissenting positions. Frame-tension (verify vs.
bypass-now) is kept live per framing.

## 2. Of-framing disposition

Three of-framing challenges raised this round:

| Challenge | Source | Disposition |
|-----------|--------|-------------|
| "Real CI gap is only `cargo test` — pre-commit covers fmt+clippy" (C6) | challenger | **Integrate into Round 2**: this narrows the selection criterion for Topic 2 substantially. Solutions that give `cargo test` coverage are enough; full clippy+test parity is over-scope. |
| "Failure may no longer reproduce — last confirmed 2026-04-17, runner may have been updated" (C5) | challenger | **Integrate into Round 2**: adds a cheap pre-flight retest before Topic 1 even decides. If failure doesn't reproduce now, Topic 1 collapses entirely. |
| "Hybrid multi-runner (mac for lint, VPS for build)" | gemini-proxy | **Integrate into Round 2** as a new Topic 2 option to evaluate. |
| "Zig-as-linker wrapper for Cargo config" (skips runner topology change; test execution still needs Linux container) | codex-proxy | **Integrate into Round 2** as a new Topic 2 option with the test-execution caveat documented. |

No of-framing challenge rejected or deferred-to-backlog this round.

## 3. Verification artifact

Claims requiring verification artifact:

| Claim | Cited artifact? | Status |
|-------|-----------------|--------|
| cc-rs `apple_flags()` target-vendor-gated | architect + codex cite `cc-rs lib.rs:2564` (already in analysis.md) | **verified** |
| `cargo-zigbuild` already installed | multiple cite `.forgejo/workflows/release.yml` existing zigbuild shims; unchecked against actual cargo-zigbuild release/test path | **partially verified** — existence confirmed; functionality for `x86_64-unknown-linux-gnu` ring-compile unverified (challenger C3) |
| "30-min verification" estimate | all 3 verify-then-decide positions cite it; only codex-proxy bounds it to 30–90 min and cites why (use CC_ENABLE_DEBUG_OUTPUT, not dtruss) | **unvalidated** as hard upper bound; codex's methodology refines it |
| "VPS has CPU headroom / contention risk" | architect + gemini assert; minimal-change + challenger challenge as unmeasured | **unvalidated** — concrete measurement required |
| "ring's cmake build may not cooperate with zigbuild Linux target" | challenger C3 raises; no counter-evidence | **unvalidated** — needs dry-run test |

Round 2 reading required for: VPS contention (needs measurement), zigbuild-vs-ring compatibility (needs dry-run), 30-min verification bound (needs methodology agreement).

## 4. Frame-challenge disappearance self-check

Round 0 raised 5 framing challenges. Round 1 check — any silently disappeared?

- "Apple-Clang hypothesis treated as premise" (v1 bias anchoring) → **still honored**: all Round 1 agents explicitly describe the hypothesis as unverified; 3 of 4 substantive positions want to verify first.
- "Runner-mode change blocked by scope-out conflation" (v1 adversarial) → **still honored**: Docker executor and VPS relocation both appear in Round 1 rankings as live candidates.
- "Three coupled decisions" (v1 minimal-change) → **still honored**: no Round 1 agent attempted to pull in `release.yml` race or 006/BL scope split. Minimal-change explicitly flagged readiness to block such creep.
- "Presentation-order primes Apple-Clang hypothesis" (v2 strategic non-blocking) → **still honored**: agents consistently note the hypothesis as unverified rather than working premise.
- "Runner relocation ambiguity (VPS vs second runner on Forgejo host)" (v2 adversarial non-blocking) → **re-surfaced**: gemini-proxy and codex-proxy appear to use "runner relocation" to mean slightly different things. Flag for Round 2 to disambiguate.

No silent disappearance. The runner-relocation ambiguity is explicitly flagged for Round 2.

## Contested claims → Round 2 agenda

1. **T1**: does the choice of bypass actually depend on verified mechanism? architect says yes (Group A vs B); minimal-change says no (zigbuild universal). Which is right depends on whether zigbuild works in ring's cmake path (challenger C3).
2. **T2 mechanism ranking reversal**: codex ranks compiler replacement #1 / relocation #4; gemini ranks relocation #1 / Docker #4. Two cross-family voices, opposite conclusions on the same question. Need evidence-based arbitration.
3. **Pre-flight retest (challenger C5)**: cheap prerequisite that may obviate Topic 1. Address before Round 2 fully engages.
4. **Scope narrowing (challenger C6)**: only `cargo test` is the uncovered gap; fmt+clippy is already local. Does this change T2 selection criteria?
5. **Runner relocation ambiguity**: gemini's "VPS runner" and codex's "runner relocation" — same thing or different topologies?
