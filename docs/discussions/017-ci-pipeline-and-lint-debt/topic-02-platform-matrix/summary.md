---
id: "02"
title: "Platform matrix"
status: converged
current_round: 1
created: 2026-04-17
decision: "Linux x86_64 only. Reject Mac mini runner. Documented pre-release manual macOS cargo test checklist. Windows explicitly out of scope; existing #[cfg(unix)] gates retained."
rationale: "Cross-compile Linux→macOS hard-blocked by CoreFoundation / fsevent-sys. Mac runner operational cost not justified solo-dev. #[cfg(unix)] tests on Linux cover the vast majority of macOS paths; divergence risk for mengdie workloads low."
reversibility: "high"
reversibility_basis: "Adding a mac runner later = new job: entry in ci.yml, no restructure. No lock-in."
---

# Topic: Platform matrix

## Current Status
Converged unanimously in Round 1.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | Linux-only, document manual macOS check, Windows deferred |

## Context
See index.md. macOS/Linux dev parity matters but cross-compile is blocked.

## Constraints
- Forgejo runner on Linux x86_64 only
- Cross-compile to macOS blocked per memory/project_infra.md
- User's Mac mini is the dev machine — making it a CI runner has security/ops cost

## Key Questions — resolved
- Linux-only acceptable: yes, with documented manual macOS ritual
- Mac runner: no
- Multi-runner split: unnecessary
- Windows: out of scope; #[cfg(unix)] gates preserved
