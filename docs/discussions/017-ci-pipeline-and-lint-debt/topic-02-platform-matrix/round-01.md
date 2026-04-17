---
round: 01
date: 2026-04-17
score: converged
---

# Round 01 — Platform matrix

## Discussion

### Architect (Round 1)
Unanimous: Linux x86_64 CI is correct. Mac mini runner rejected — operational cost (daemon, security exposure, maintenance) not justified for solo dev. `#[cfg(unix)]` gates mean Linux tests cover macOS-shared paths; divergence risk for mengdie workloads (sqlite, file watching, embedding) is low. Pre-release manual macOS verification is the right mitigation. Windows explicitly out-of-scope — `#[cfg(all(test, unix))]` gates in BL-005 acknowledge it as a future concern, not today's.

### Rust-archaeologist (Round 1)
Verified: 3 `#[cfg(unix)]` tests in `llm.rs` use `/usr/bin/yes` + `/bin/sh` — present on both macOS and Linux. No Windows-specific code. No `cfg(windows)` anywhere.

### Codex (Round 1)
Not directly addressed in Round 1 (focused on workflow design + lint), but no objection to Linux-only in Round 2.

## Outcome

- Score: **converged** (unanimous — no Round 2 needed for this topic)
- **Decision**: Linux x86_64 only in CI. No Mac mini runner. Documented pre-release checklist: "run `cargo test` locally on Mac before cutting tag `v*`". Windows explicitly out of scope; `#[cfg(unix)]` gates remain in place for future-proofing without current action.
- **Rationale**: Cross-compile Linux→macOS blocked by CoreFoundation/fsevent-sys (per memory/project_infra.md). No Forgejo runner on the Mac mini, and the cost of adding one is disproportionate for a solo-dev project that already does local macOS testing informally. `#[cfg(unix)]` coverage on Linux catches the vast majority of macOS divergence risk.
- **Reversibility**: high
- **Reversibility basis**: adding a mac runner later is a purely additive change — new `job:` entry in ci.yml, no workflow restructure. No lock-in from Linux-only.
