---
id: BL-ci-full-clippy-test
status: open
origin: plan 008 Step 3 scope-down (commits 9c03286 + 4a12931); surfaced by /ae:discuss 014 Doodlestein-adversarial
created: 2026-04-18
---

# Extend Forgejo CI to full clippy + test (currently fmt-only)

## Finding

Plan 008 (CI pipeline + lint debt) Step 3 was delivered partially. The shipped
`.forgejo/workflows/ci.yml` runs `cargo fmt --check` only — clippy and `cargo
test` were scoped out in commit `9c03286` due to a ring-crate `-isysroot`
environment bug on the Forgejo host-mode runner (see commits `6658248` and
`4a12931` for the debug trail). Plan 008 was closed as `status: done` in the
progress-audit cleanup (discussion 014) with a header note pointing at this
backlog entry — per the "file to backlog with trigger before closing with
carry" pattern (BL-007 review).

The 3 Step 3 pending checkboxes on plan 008 represent work still intended to
ship:

1. `ci.yml` clippy job (`cargo clippy --all-targets -- -D warnings`) — currently absent
2. `ci.yml` test job (`cargo test`) — currently absent
3. Allow-list audit trigger + escalation process — partially documented, ongoing

## Trigger

Fires when any ONE of:

1. The ring-crate `-isysroot` issue on the Forgejo host-mode runner is
   resolved upstream (ring is patched, or we switch to a runner with
   container support, or we move the CI to a GitHub Actions mirror).
2. A CI-relevant PR lands that would have benefited from clippy/test
   coverage catching something the pre-commit hook missed (concrete
   signal: a regression that shipped past `.githooks/pre-commit` and
   would have been caught by CI-side execution).
3. The next feature plan that touches `.forgejo/workflows/` for any other
   reason — bundle the clippy+test addition in the same commit.

## Fix options

**Option A (preferred)**: resolve the ring -isysroot issue by pinning an
older ring version that precompiles or by moving the runner to container
mode if the host runner config can be adjusted. Then add clippy + test
jobs to ci.yml. This is the intended end-state.

**Option B**: mirror the CI to GitHub Actions (free-tier Linux runners)
and leave Forgejo as fmt-only. Cheaper to stand up, but splits the CI
surface across two providers.

**Option C**: accept fmt-only CI permanently; rely on `.githooks/pre-commit`
+ local `cargo test` discipline. Risk: regressions can ship if contributors
skip hooks (e.g., `--no-verify`) — acceptable for a solo-dev project, risky
if collaborators join.

Prefer A. B is a fallback if ring upstream stays broken. C is defensible
only while mengdie remains solo-dev.

## Why not fixed in plan 008

Plan 008 already burned its time budget debugging the ring issue (see
commit history `df7ba2d` close-out). Two days of `-v` builds in CI still
couldn't reproduce the issue in isolation. Closing the plan without
dragging on further was the right call; this backlog entry preserves the
resume signal.
