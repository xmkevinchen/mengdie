---
round: 2
date: 2026-04-22
role: TL synthesis (index + orientation only — per-agent files are primary)
---

# Round 2 Synthesis

Read per-agent files directly. This is an orientation index.

## Per-agent file index

| Agent | File | Round 2 headline |
|-------|------|------------------|
| architect | [architect.md](architect.md) | Replaced verify-then-decide with 2-step pre-flight (retest + dry-run). T2 splits by coverage type. |
| minimal-change-engineer | [minimal-change-engineer.md](minimal-change-engineer.md) | Conceded zigbuild is build-wrapper-only. Adopted zig-cc-as-CC + env-unset belt-and-suspenders. 3-stage plan. |
| challenger | [challenger.md](challenger.md) | 5 cross-agent challenges. Biggest: "dry-run IS itself diagnostic". Gemini's undo-cost claim factually wrong. |
| codex-proxy | [codex-proxy.md](codex-proxy.md) | T1 collapses into "dry-run first". Zigbuild covers compile only; test execution requires VPS/Docker. |
| gemini-proxy | [gemini-proxy.md](gemini-proxy.md) | **Reversed Round 1 ranking**: zigbuild #1, relocation #3-4. Rejects own hybrid multi-runner. C5 mandatory blocker. |

## 1. Pruned

Pruned: three Round 1 positions.
- **Bare `cargo zigbuild`** as T2 answer — pruned per codex's "build-wrapper-only" finding (codex-proxy.md:69, affirmed by minimal-change-engineer R2). Replaced by zig-cc-as-CC via `.cargo/config.toml`.
- **Gemini's hybrid multi-runner** (mac lint + VPS build) — pruned by gemini itself in R2 (self-reversal), plus challenger R2 #4: orchestration overhead, zero new coverage on solo-dev project.
- **Docker-on-Mac-mini** as first-ship candidate — pruned by minimal-change R2 (2-4hr setup + ongoing maintenance) + challenger R2 (license surface, host-mode precedent break). Still available as hypothetical fallback, not primary.

## 2. Of-framing disposition

Of-framing challenges this round + TL disposition:

| Challenge | Source | Disposition |
|-----------|--------|-------------|
| "Dry-run IS itself the diagnostic" — collapse T1 into cheap bypass attempt | challenger R2 #3; reaffirmed by architect R2, codex R2 Q4 | **Integrated into converging direction**. T1 becomes Stage 0 (retest) + Stage 1 (dry-run zig-cc-as-CC). |
| "C5 pre-flight retest is mandatory hard gate, not soft advisory" | challenger R2 #5; affirmed by gemini R2, minimal-change R2 Stage 0, architect R2 | **Integrated**. If failure doesn't reproduce, discussion closes trivially. |
| "Group A/B split was real but narrower than claimed" (leak source d — cmake TARGET propagation — escapes zig-cc too) | challenger R2 #2 | **Integrated**. Motivates belt-and-suspenders env-unset alongside zig-cc (minimal-change R2). |
| "Codex vs Gemini asked different cost questions (setup vs TCO)" | challenger R2 #1 | **Integrated**. Framing's stated criterion is TCO, but setup cost matters when it blocks the sprint. Both dimensions acknowledged; direction picks low-setup-AND-low-TCO mechanism (zig-cc-as-CC). |
| "Zigbuild covers compile but NOT test execution" — real gap on `cargo test` | codex R2 Q2; architect R2 T2; gemini R2 implicit | **Deferred to plan time**. Framing scoped 006/BL boundary as out-of-scope; "what does BL's `cargo test` mean for this CI" is the same class of scope question. Noted in T2 decision as conditional Stage 2. |

Zero of-framing challenges rejected.

## 3. Verification artifact

Claims carried from R1 synthesis + new R2 claims:

| Claim | Status after R2 |
|-------|-----------------|
| cc-rs `apple_flags()` target-vendor-gated | **verified** (cc-rs lib.rs:2564, cited in R1) — unchanged |
| cargo-zigbuild already installed | **partially verified**: codex R2 + minimal-change R2 concede bare `zigbuild` is a build wrapper (covers compile path only). Use as `CC_<target>` linker/wrapper via `.cargo/config.toml` is the correct form. |
| "Dry-run zigbuild catches leak source (a)(c) but not (d)" | **artifact-cited**: challenger R2 #2 cites cc-rs/cargo semantics; no counter-evidence. Motivates env-unset belt-and-suspenders. |
| "VPS has CPU headroom" (gemini R1 assertion) | **unvalidated** — gemini R2 conceded it was contingent on unmeasured. Becomes a conditional measurement if Stage 2 ever triggers. |
| "Test execution on zig-cc output requires Linux runtime (no Darwin syscalls leak through cross-target binary)" | **verified** (codex R2 Q2) — zig-cc produces Linux ELF; executing requires Linux env. |
| "Relocation setup cost — 3-8hr standalone vs. much less if adding to already-maintained VPS" | **partially validated**: architect R2 pushes back on codex's 3-8hr as standalone-host estimate. Adding runner to existing VPS is lower cost but still has concrete setup steps. Not measured. |

No unvalidated claims advancing as converged.

## 4. Frame-challenge disappearance self-check

Round 1 of-framing markers to check in Round 2:

- "VPS CPU contention risk is unmeasured" (R1) → **still live**: gemini R2 concedes; becomes conditional measurement for Stage 2.
- "Zigbuild-vs-ring cmake path unverified" (R1 challenger C3) → **REFRAMED**: codex R2 reveals cargo-zigbuild is a build wrapper (not a drop-in replacement), so the R1 question was partly incorrect. The right question becomes "does zig-cc-as-CC compile ring cleanly for Linux target?" — addressable by the dry-run itself.
- "Runner relocation ambiguity (VPS-hosted vs second dedicated VPS)" (R1) → **partially resolved**: gemini R2 Q3 chose (a) = runner on existing Forgejo VPS (same host). Architect R2 aligns.
- "30-min verification bound unvalidated" (R1) → **dissolved**: dry-run collapses verification into the bypass attempt itself. Original 30-min bound becomes moot if Stage 0 retest passes (no verification needed) or Stage 1 dry-run passes (verification is the ship).
- "Real CI gap is only cargo test" (R1 challenger C6) → **integrated**: narrows T2 acceptance criterion; surfaces test-execution gap that zig-cc alone doesn't close.

No silent disappearance.

## Converged direction (for UAG check next)

**T1 — Investigation cadence**:
Stage 0 (C5 pre-flight retest, ~5 min): on the current runner, attempt the
original failure. If it no longer reproduces (runner/act drift since
2026-04-17), close both backlog items and this discussion.
Stage 1 (if failure reproduces, ~15-30 min): run `cargo zigbuild
--target x86_64-unknown-linux-gnu --tests --no-run` or the zig-cc-as-CC
`.cargo/config.toml` form with `CC_ENABLE_DEBUG_OUTPUT=1` on the
runner. Pass → Stage 1 is the bypass and the verification simultaneously.
Fail → root-cause (time-boxed) or jump to VPS/Docker fallback.

This collapses the original verify-vs-bypass dichotomy: the bypass IS
the verification.

**T2 — Mechanism**:
Primary: **zig-cc-as-CC via `.cargo/config.toml`** (wrapper for cc-rs
on the Mac mini runner), with env-unset belt-and-suspenders
(SDKROOT/DEVELOPER_DIR/CFLAGS/CXXFLAGS/CPPFLAGS in the workflow step)
to hedge against leak source (d) — broken TARGET/CARGO_CFG_TARGET
propagation through cmake.

Coverage gap acknowledged: zig-cc covers compile (clippy + test-compile)
but not Linux test execution. If BL-ci-full-clippy-test requires actual
test execution on Linux, **conditional Stage 2** adds either:
- SSH-trigger pattern to the existing Forgejo VPS runner (minimal-change
  R2's proposal, ~30 min bash script), OR
- second runner registered to the Forgejo instance on the same VPS
  host (gemini R2 Q3 form (a)).

Rejected: Docker-on-Mac (setup + maintenance cost disproportionate to
solo-dev benefit); hybrid multi-runner (gemini reversed own proposal —
complexity redistribution, not reduction); bare `cargo zigbuild` as
final form (build wrapper only, wrong abstraction level).

**Test-execution vs test-compile**: the definition-of-done for
`BL-ci-full-clippy-test`'s `cargo test` job is a plan-time scope
question (falls out per framing). Discussion records both shapes.
