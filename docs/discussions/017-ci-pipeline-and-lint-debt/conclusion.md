---
id: "017"
title: "CI Pipeline + Lint Debt Cleanup — Conclusion"
concluded: 2026-04-17
plan: ""
entities: [ci, scope, triggers, ci-scope-and-triggers, platform, matrix, platform-matrix, lint, debt, strategy, lint-debt-strategy]
---

# CI Pipeline + Lint Debt Cleanup — Conclusion

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | CI scope and triggers | New `.forgejo/workflows/ci.yml` separate from `release.yml`. Triggers: `push` (all branches except tags) + `pull_request`. Single serial job with `rustup show` as first step, then `cargo fmt --all -- --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`. Host Rust via `source ~/.cargo/env` (matches release.yml's proven pattern). Cache `~/.cargo/{registry,git}`, `target/`, `~/.cache/fastembed` keyed on `hashFiles('**/Cargo.lock', 'rust-toolchain.toml')`. Bare `actions/checkout@v4` syntax. `cargo audit` + pre-commit hooks deferred. Follow-on after ci.yml is green: drop the redundant `test:` job from `release.yml`. | Solo-dev + single self-hosted runner → fail-fast serial beats parallel. Fastembed cache is the single biggest cold-start cost (~90MB). Host mode confirmed (no Docker). Bare action syntax works today via DEFAULT_ACTIONS_URL. Removing release.yml's test job after ci.yml ships eliminates quality-gate divergence between workflows without forcing a merge. | high — single YAML file; splitting jobs / adding audit / switching to full-URL refs / removing release.yml test are all purely additive changes. |
| 2 | Platform matrix | Linux x86_64 only. Reject Mac mini runner. Documented pre-release checklist: "run `cargo test` locally on Mac before cutting tag `v*`". Windows explicitly out of scope; existing `#[cfg(unix)]` gates preserved. | Cross-compile Linux→macOS hard-blocked by CoreFoundation / fsevent-sys (memory/project_infra.md). Mac runner operational cost (daemon, security exposure, maintenance) not justified for solo-dev. `#[cfg(unix)]` tests on Linux cover the shared-paths majority; divergence risk for mengdie workloads (sqlite, file watching, embedding inference) is low. | high — adding a mac runner later is purely additive (new `job:` entry); no lock-in from Linux-only. |
| 3 | Lint debt strategy | Big-bang cleanup PR first addressing all 10 clippy items (1 hard error `approx_constant` at `embeddings.rs:116`, 6 trivial warnings, 2 needs-thought warnings — 14 lines affected total). Then add `rust-toolchain.toml` pinning `channel = "1.94.1"`, `profile = "minimal"`, `components = ["rustfmt", "clippy"]`. Then ship `ci.yml` with `cargo clippy --all-targets -- -D warnings` enforced from day 1. `cargo fmt --all -- --check` (check-only, no auto-format). No `rustfmt.toml` unless specific style opinions emerge. No project-wide `#![allow(...)]`; case-by-case `#[allow(...)]` only on genuine false positives. | Starting with non-blocking `-W warnings` leads to permanent background noise (codex ecosystem evidence — rust-analyzer, tokio projects). Big-bang cleanup is cheap on solo-dev (no review bottleneck). Exact-version pin (1.94.1 matches local verified) prevents silent 6-week-release clippy surprises. `--all-targets` is the minimum scope for honest build health — anything less hides bin-target warnings. | high — all changes in-file + in-YAML. Relaxing the gate to `-W warnings` or adding allow-lists is trivial. Toolchain bump is a one-line edit. Reverting cleanup commits is standard git. |

## Doodlestein Review

| Agent | Challenge | Resolution |
|---|---|---|
| Strategic | After ci.yml ships, release.yml still has its own `test:` job but no fmt/clippy gate — two partially-overlapping workflows create drift surface where lint debt can re-enter via the release path. | Adopted as follow-on action: drop `test:` job from release.yml after ci.yml is green. ~5 lines of YAML. Architect confirmed no side effects (no artifacts, no notifications tied to that job). Added to decision 1. |
| Adversarial | `rust-toolchain.toml` pin behavior never verified on runner — if runner has system Rust only (no rustup), pin is silently ignored and we're not actually pinned. The cited "proven pattern" (release.yml) has NO rust-toolchain.toml, so the pattern doesn't evidence pin semantics. | Resolved with evidence + mitigation: (1) release.yml's `source ~/.cargo/env` is a rustup-specific idiom — system Rust doesn't create that file — so rustup is near-certainly present (archaeologist); (2) codex confirmed rustup toolchain-file behavior is automatic via proxy binaries on first `cargo` invocation; (3) belt-and-braces: added explicit `rustup show` as the first CI step — forces resolution AND fails loudly if rustup is absent. Fallback if the runner turns out to be system-Rust-only: `rustup override set 1.94.1` explicit step (architect's Fallback A). Decision 3 updated to include `profile = "minimal"` and `rustup show` first step. |
| Regret | `-D warnings --all-targets` from day 1 is most likely to flip. Trigger: first agent-written commit that trips an existing deny-by-default clippy lint. Replacement path: `-W warnings` + `clippy.toml` with selective denies. | Partially accepted, decision NOT reversed. Mitigation baked in: exact-version pin (1.94.1) prevents the "new lint on unchanged code" failure mode — agents writing against this codebase hit a stable lint-set. Remaining risk (agent tripping existing lints on new code) is contained to occasional `#[allow(...)]` on genuine false positives. Monitor: if `#[allow(...)]` additions exceed 3 in the first 30 days of CI, revisit via a new discussion. Documented as known risk in topic-03 summary. |

## Spawned Discussions

None — all decisions are plannable without further sub-discussion.

## Deferred Resolutions

None — no topic was marked `deferred` during scoring. Sweep not triggered.

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude Opus 4.7 | Start |
| architect | Workflow design, job decomposition, cache strategy | Claude | Start |
| rust-archaeologist | Ground truth survey (clippy warnings, release.yml, config files, test gates) | Claude | Start |
| codex-proxy | Forgejo/Rust CI best practices, ecosystem conventions, cache semantics | Codex (OpenAI, medium reasoning) | Start |
| doodlestein-strategic | Strategic improvement challenge | Claude | Doodlestein |
| doodlestein-adversarial | Blind spot / blunder detection | Claude | Doodlestein |
| doodlestein-regret | Regret prediction | Claude | Doodlestein |

Gemini was unavailable this session (MCP auth failure in a prior cycle, not retried). Codex alone covered cross-family.

## Process Metadata

- Discussion rounds: 2 (Round 1 independent research, Round 2 tension resolution on runner env / pin version / clippy scope)
- Topics: 3 total (3 converged, 0 spawned, 0 explained-and-assumed)
- Autonomous decisions: 3
- User escalations: 0
- Doodlestein challenges: 3 raised, 3 resolved (2 adopted as plan refinements, 1 accepted with mitigation + risk monitor)
- Deferred resolved in Sweep: 0 (no deferred items)

## Addendum (2026-04-17, post-conclusion revision)

Two decisions were revised by the user after this conclusion was written. The revisions are incorporated directly into the downstream plan; both amount to "TL missed these during scoring, user caught them."

### Revision 1: Add local pre-commit hook
**Original**: "Pre-commit hooks: out of scope for this discussion. That's a local developer ergonomics choice, not a CI concern." (topic-01)

**Revised**: Add `.githooks/pre-commit` shell script running `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`. `cargo test` NOT in the hook (fastembed cold-start too slow). Install via `git config core.hooksPath .githooks` (one-time per clone). Rationale: AI-agent commit velocity is high; relying only on CI for lint means every red commit enters the graph and pollutes history. CLAUDE.md already says "NEVER skip hooks (--no-verify)" — the codebase ethos treats hooks as hard gates, not advisory. The "local ergonomics" framing was wrong for this workflow.

### Revision 2: Drop `rust-toolchain.toml` pin AND `rust-version` MSRV
**Original**: "Pin `rust-toolchain.toml` to `channel = "1.94.1"`, `profile = "minimal"`, `components = ["rustfmt", "clippy"]`." (topic-03, decided after Round 2 tension on `stable` vs exact)

**Revised**: Do NOT pin the toolchain. Do NOT add `rust-version` to Cargo.toml. Rationale: (a) pinning blocks new clippy lints from firing on agent-written code, which is the one type of automated code review we most benefit from; (b) `rust-version` MSRV is meaningful only for crates published to crates.io or with downstream consumers — mengdie is a solo-dev tool with neither, so it's decorative; (c) the ~5-min-every-6-weeks cost of an unexpected CI red from a new stable release is lower than the permanent cost of staleness. When a new lint fires, **default action is to fix the code** — `#[allow(...)]` is last resort.

**`#[allow(...)]` policy (clarified by user)**: `#[allow]` is LAST RESORT, not the default escape hatch. Reach order: (1) fix the code, (2) if the lint is a genuine false positive or blocked by external constraint (macro we don't own, upstream API), then `#[allow]` with a comment explaining the specific reason. Every `#[allow]` in the tree is reviewable debt; code review should question each one. Do not treat `#[allow]` as "CI is annoying, turn off the warning."

Doodlestein-regret's prediction that `-D warnings` would flip turns out to be partially vindicated: the mitigation we chose (pin the version) was itself wrong, and removing it reveals the real mitigation is the `#[allow]` escape hatch, which was always there.

### What the plan should do

1. **Big-bang clippy cleanup** — fix all 10 items. Keep `cargo test` green. One atomic commit.
2. **Local pre-commit hook** — add `.githooks/pre-commit` + one-line install instructions; runs `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`. User runs `git config core.hooksPath .githooks` once.
3. **CI workflow** — add `.forgejo/workflows/ci.yml` triggering on push (all branches) + pull_request. Same fmt/clippy/test checks as the hook, plus `cargo test` which the hook skips. Host Rust via `source ~/.cargo/env` (pattern from release.yml). No `rust-toolchain.toml`, no `rustup show` step (only needed if pinning), no `rust-version`.
4. **Trim release.yml** — remove the redundant `test:` job once ci.yml is green.
5. **Verify + monitor** — first 5 runs, fastembed cache behavior, document any `#[allow]` additions.

## Next Steps

→ `/ae:plan` — see "What the plan should do" above. Follow the 5-step structure from the addendum, not the original 3-step proposal.
