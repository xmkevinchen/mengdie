---
id: "020"
stage: framing
created: 2026-04-22
round_0: approved
round_0_reviewers: [minimal-change-engineer, codex-proxy, gemini-proxy]
round_0_notes: |
  v3: 3/3 APPROVED (re-check after surgical Scope-In delete). Combined
  coverage across v2+v3: codex-proxy ✓, gemini-proxy ✓ (recovered in v3),
  doodlestein-strategic ✓ (v2 APPROVED, non-blocking note), doodlestein-
  adversarial ✓ (v2 APPROVED, non-blocking note), minimal-change-engineer
  ✓ (v2 REVISE → v3 APPROVED). Round 0 passes. Framing locked; proceed
  to Round 1 discussion.
round_0_iteration: 3
round_0_prior_iterations: |
  v1 (2026-04-22): unanimous REVISE (5/5). Framing treated Apple-Clang
  hypothesis as premise rather than unverified; conflated discussion 017's
  target decision with runner-mode (blocking Docker executor as an option);
  coupled three decisions that should decouple. Per-agent verdicts in
  round-00/ (codex-proxy.md, gemini-proxy.md, doodlestein-strategic.md,
  doodlestein-adversarial.md, minimal-change-engineer.md).
  v2 (2026-04-22): 4 APPROVED + 1 REVISE (minimal-change-engineer) +
  1 unavailable (gemini-proxy: API 503 + local gemma4 hung). Surgical
  REVISE: delete Scope-In runner-mode bullet (re-elevated sub-option to
  first-class, miniature v1 coupling pattern). Edit applied; runner-mode
  annotation preserved as parenthetical on the contingent mechanism
  bullet. Per-agent verdicts in round-00/v2-*.md.
---

# Framing — CI Runner Env Cleanup

## Problem Statement

Mengdie's CI runs `cargo fmt --check` only. Expanding CI to clippy +
`cargo test` on PR is blocked: the Forgejo host-mode runner on the macOS
Mac mini fails to compile `ring` with an `-isysroot` error pointing at
the Xcode SDK, even when the target is `x86_64-unknown-linux-gnu` and
the bash environment inside the act-spawned step is clean.

The 2026-04-22 analysis (`analysis.md`) narrowed the mechanism. cc-rs
source (`lib.rs:2564`, `apple_flags()` target-vendor-gated) rules out
cc-rs as the injector for Linux targets. A working hypothesis — that
`/usr/bin/cc` on the runner resolves to Apple Clang (regardless of the
version string), which shells to `xcrun` internally and synthesizes
`-isysroot` inside its own invocation — is **unverified**. One targeted
verification command on the runner (`/usr/bin/cc -v` + a minimal ring
compile under `strace`/`dtruss`) either confirms or refutes it in
<30 min.

Sprint v0.8.0 committed two items against this problem:
`006-ci-runner-env-cleanup` (M, 3 pt) and `BL-ci-full-clippy-test`
(L, 5 pt). Both were sized before the mechanism narrowing. Whether the
two items collapse, sequence, or remain independent is **downstream**
of the decision framed here — not a question to discuss in parallel.

## The Decision

Given a <30 min verification step exists, does v0.8.0 go:

(a) **Verify-then-decide** — run the verification, let the result pick
the fix path (confirmed hypothesis → bypass; refuted → new investigation
with a fresh hypothesis), OR

(b) **Accept-bypass-now** — skip verification, pick a bypass mechanism
immediately, trade possible over-investment in bypass machinery for
faster CI unblock.

If bypass is selected (under either (a) or (b)), which mechanism fits
mengdie best — compiler replacement (`cargo zigbuild` /
`CC_<target>=<linux-gcc>`), runner-mode change (Docker executor on the
same Forgejo instance — discussion 017 settled the CI target matrix
Linux-x86_64, not the executor mode), runner relocation (Linux VPS
that already hosts Forgejo), or external CI provider — remains to be
picked on total-cost-of-ownership criteria, not technical elegance.

## Scope

In:
- The verify-then-decide vs. accept-bypass-now decision for v0.8.0
- Bypass mechanism selection (contingent — only decides if bypass wins)

Out:
- The scope boundary between `006-ci-runner-env-cleanup` and
  `BL-ci-full-clippy-test` (falls out of the core decision and will be
  handled at `/ae:plan` time)
- The `release.yml` `test:`/`build-linux:` race (2-line `needs:` fix,
  ships as drive-by in whichever plan next touches `release.yml`)
- CI target matrix — settled in discussion 017, remains Linux-x86_64
- Replacing Forgejo as the Git platform
- The `.githooks/pre-commit` local gate — remains the primary local
  quality gate regardless of CI outcome

## Reference Material

- `docs/discussions/020-ci-runner-env-cleanup/analysis.md` — 5-agent
  analysis with source-level cc-rs and ring verification; refined
  mechanism hypothesis
- `.ae/backlog/v0.8.0/006-ci-runner-env-cleanup.md` — plan 008 Step 3
  evidence dump
- `.ae/backlog/v0.8.0/BL-ci-full-clippy-test.md` — the expansion BL
- `docs/discussions/017-ci-pipeline-and-lint-debt/conclusion.md` — prior
  CI matrix decisions (Linux-x86_64 target; does NOT settle runner mode)
- `docs/plans/008-ci-pipeline-and-lint-debt.md` — prior CI plan, Step 3
  scope-down history
- Reverted commits `e4b8cbf` through `6658248` (2026-04-17)
