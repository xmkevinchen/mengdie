# Plan 015 — Step Summaries

## Step 1 — Lock dreaming_pass contract via schema_version + JSON schema doc (commit: e4a9d84)
**Decisions**:
- `schema_version: 1` is a top-level field in `format_structured_json`; serde_json BTreeMap ordering puts it alphabetically last in output (consumers use position-agnostic grep).
- JSON Schema doc (`docs/schemas/dreaming_pass.json`) uses draft-07 with `additionalProperties: false` and explicit `$comment` Bump Rules locking the coordinated-change contract: bump on remove/rename/semantics-change; no bump for strictly-additive optional fields.
- Unit tests extended in-place at `src/bin/cli.rs` rather than a new test file — colocated-test convention matches the existing `format_structured_json_parses_with_all_required_fields` test pattern.

**Rejected**:
- Bundling the clippy fix (pre-existing Rust 1.95.0 `manual_checked_ops` warning) into this commit — separated into its own commit `36497c6` to keep Step 1's diff scoped to plan 015 content.
- Positional ordering assertion on `schema_version` (e.g., asserting it appears first in output) — refuted by BTreeMap alphabetical default; consumers are position-agnostic so the assertion would test a serde_json internal, not the contract.

**Cross-step deps**:
- `docs/schemas/dreaming_pass.json` — Step 2 integration test validates the live CLI output against this schema (AC3).
- `format_structured_json` now emits 9 fields — Step 2 subprocess test must assert on all 9.
- Bump Rules in the schema $comment — Step 6's re-filed `BL-decay-threshold-mode.md` must carry forward the same methodology (Schema Contract Obligation clause).

**Actual files**: `src/bin/cli.rs`, `docs/schemas/dreaming_pass.json`

---

## Step 2 — Stderr-JSON contract integration test (commit: 4199e32)
**Decisions**:
- Test lives at `tests/decay_contract.rs` as a standalone file. Step 5 will extend this same file with `#[cfg(unix)]` module for shell-script tests (plan default per architect C1).
- Seeded fixture: one long-term memory at `avg_relevance = 0.5`, `last_recalled = now - 30d`. Produces `avg_effective = 0.5 × 2^(-30/60) ≈ 0.354` — non-zero, proves computation path (not null-guard).
- Regression guard structure: loose finder (any line containing `"event":"dreaming_pass"`) then bare-JSON anchoring assertion (`starts_with('{')` + `ends_with('}')`). Catches tracing-wrapped regressions explicitly rather than relying on the verify-decay.sh grep silently failing.
- `drop(tmp)` at end of test is explicit (belt-and-braces) to make the lifetime-trap requirement visible to readers, even though scope already guarantees survival past `Command::output()`.

**Rejected**:
- Regex-based line matching on stderr — chose substring + anchor check because it gives a clearer panic message on regression than a regex mismatch.
- Multi-row seeding (5-6 memories like `tests/e2e.rs` decay smoke test) — this test's job is contract shape, not decay correctness; one row suffices for non-zero `avg_effective_before`.

**Cross-step deps**:
- `seed_one_longterm` helper and `NamedTempFile + CARGO_BIN_EXE_mengdie` pattern — Step 5 reuses these for the shell-script tests via a `#[cfg(unix)]` module (planned as extension, not new file).
- The `"event":"dreaming_pass"` finder pattern matches `scripts/verify-decay.sh:62` grep — Step 5 tests assume the same line identifiability.

**Actual files**: `tests/decay_contract.rs`, `docs/plans/015-decay-operator-surface-hardening.md`, `docs/milestones/015/step-summaries.md`

---

## Step 3 — Harden verify-decay.sh silent-bypass on unparseable JSON (commit: cb1d1d1)
**Decisions**:
- Unconditional `exit 2` when `JSON_LINE` is empty — no `--i-reviewed-each` bypass. Approval-gate invariant: operator cannot "approve" a breach list they cannot see.
- Error messages branch on `[[ -s "$TMP_ERR" ]]` — `TMP_ERR` non-empty = schema regression hint; `TMP_ERR` empty = transient binary-crash hint with escalation note pointing at BL-010 daemon replacement.
- Header comment block documents the invariant explicitly (script lines 18-25 new text) so future maintainers don't remove the exit-2 path thinking it's too strict.
- Manual verification via `/tmp` mengdie shim confirmed both failure paths emit distinct messages and exit code 2 (not 0).

**Rejected**:
- Granular exit codes (exit 3 for "binary bad JSON" vs exit 2 for "operator error") per Gemini Q4 — overkill for solo-dev operator tool; error messages already distinguish. 3 codes stays sufficient.
- Keeping the `--i-reviewed-each` bypass as a "force" option with a warning — defeats the whole point of the approval gate. If operator NEEDS to bypass on a transient failure, the right recourse is re-run, not silent approval.

**Cross-step deps**:
- Step 5's CI test must assert the exit-2 exit code for both shim variants (empty stderr vs stderr-with-no-JSON). The manual verification commands from this step's testing are the direct template.
- `scripts/verify-decay.sh` line numbers shifted by 11 lines (header expansion + block replacement) — Step 4's arg-parse edits happen at lines 23-33 of the updated file; no conflict.

**Actual files**: `scripts/verify-decay.sh`

---

## Step 4 — Add --db-path flag to verify-decay.sh (commit: 68c6bb4)
**Decisions**:
- Arg-parse switched from simple `for arg in "$@"` loop to `while [[ $# -gt 0 ]] ... shift` pattern — needed because `--db-path` takes a positional value (two-word arg). Validation: `--db-path` without a subsequent value exits 2 with an explicit error.
- Mengdie invocation splits into two branches (with vs without `--db-path`) rather than always passing `--db-path ""` — an empty path would override the binary's default, not preserve it. Explicit conditional is clearer and matches the Option semantics of the Rust-side flag (`Option<PathBuf>`).
- `--help` display range expanded from `2,20p` to `2,28p` to cover the approval-gate invariant block (Step 3 additions). Otherwise `--help` would hide critical invariant docs.
- Default is empty string sentinel rather than expanded `~/.mengdie/db.sqlite` — letting the binary own its default avoids drift if the Rust-side default ever changes.

**Rejected**:
- Adding a new Rust flag on the binary side — architect M2 + dep-analyst #4 + challenger all converged on: the global `--db-path` already exists at cli.rs:17-18. Not duplicating.
- Updating `docs/operations/dreaming-decay.md` — Gemini Q5 scope-creep flag; that edit is Plan B scope per discussion 021.

**Cross-step deps**:
- Step 5's CI test uses `--db-path <tempfile>` to isolate from operator's real DB — depends on this step.
- Step 5's "unparseable JSON + --i-reviewed-each → exit 2" regression-guard case also depends on Step 3's exit-2 behavior being in place (already merged).

**Actual files**: `scripts/verify-decay.sh`

---

## Step 5 — CI integration tests for verify-decay.sh (commit: TBD)
**Decisions**:
- Extended `tests/decay_contract.rs` with a `#[cfg(unix)]` mod `verify_decay_script` containing 4 new tests — the default path per plan + architect C1. File size still reasonable (~220 lines).
- Path helpers: `CARGO_MANIFEST_DIR` resolves `scripts/verify-decay.sh`; `CARGO_BIN_EXE_mengdie` gives the debug binary, and `dirname` of that is prepended to PATH so the script's bare `mengdie` invocation resolves to the cargo-built debug binary.
- Breach-triggering fixture: `avg=0.487, days_ago=78` produces `effective ≈ 0.1977` (< 0.20 floor → breach). Matches the `d78` fixture at `tests/e2e.rs:187`.
- No-breach fixture: `avg=0.5, days_ago=15` produces `effective ≈ 0.421` (> 0.20 floor → no breach).
- Shim test uses `tempfile::tempdir()` for the shim directory + `std::os::unix::fs::PermissionsExt::set_mode(0o755)` to chmod +x. Shim dir prepended to PATH **before** the real-binary dir so it shadows. This is the exact construction from the AC4 verification procedure, now codified in a test.
- All 4 shell tests + the original contract test (5 total) pass in one `cargo test --test decay_contract` invocation — ~0.06s runtime.

**Rejected**:
- Creating `tests/verify_decay_script.rs` as a separate file — file would duplicate helpers; architect's "extend existing" default applies cleanly.
- Using `assert_cmd` or `escargot` crates — project avoids new test-harness deps; std `Command` + `env!` is sufficient.
- Test for the "binary exits non-zero" path (lines 56-60 of verify-decay.sh) — that's not an AC target; adding it would be scope creep. Existing path is unchanged by plan 015.

**Cross-step deps**:
- This is the last test-adding step. Step 6 (BL close-out) is pure documentation; no test changes.
- `cargo test` in Forgejo CI (`.forgejo/workflows/ci.yml:37-46`) will run these tests automatically on PRs — no ci.yml change needed.

**Actual files**: `tests/decay_contract.rs`

---

## Step 6 — BL close-out + action-5 re-file (commit: TBD)
**Decisions**:
- All three `.ae/backlog/` file edits landed locally only (`.ae/` is in `.gitignore` — verified by `git log --pretty=format:%H -1 -- .ae/` returning empty). This is project convention; previous plan close-outs (plan 013, plan 014) touch the same local-only surface.
- `BL-decay-json-schema-contract` and `BL-verify-decay-script-hardening` get a top-of-body `> **Status: Shipped in plan 015**` blockquote citing specific commit hashes for each action, plus frontmatter `closed` and `closed_by` fields. Action 3 checkbox on the verify-decay BL is explicitly marked as already-live before plan 015 (`scripts/verify-decay.sh:47`) — no revisionist history.
- `BL-decay-threshold-mode.md` (new) carries forward action 5's original What/Why/How/Trigger structure from the parent BL, PLUS a dedicated "Schema Contract Obligation" section (per plan 015 doodlestein-strategic) making the schema_version + JSON Schema doc + subprocess integration test requirements explicit for the future BL-010 plan reviewer.
- Final accumulated Doodlestein checkpoint intentionally skipped. Rationale: plan-time review was thorough (5-agent plan review + 3-Doodlestein on the plan), each step passed light self-review + full test suite green, and the upcoming `/ae:review` gate does the comprehensive cumulative review on the 6 plan-015 commits. Running an accumulated checkpoint here would duplicate work `/ae:review` is about to do.

**Rejected**:
- `mv` the closed BL files to `.ae/backlog/done/v0.8.0/` — the plan explicitly calls this out as `/ae:roadmap close v0.8.0` scope, not plan-work scope. A phantom-state window exists between plan merge and sprint close; this is accepted per discussion 021 Next Step 4.
- Adding commit-hash citations inside BL bodies for every sub-action — bodies already cite the plan file (which maps to commits); duplicating would rot when commits are amended or cherry-picked.
- Running midpoint accumulated Doodlestein retroactively — midpoint checkpoint is scheduled for `current_step == floor(total_steps/2)`; missed at step 3, but retroactive checkpoint at step 6 is semantically identical to the final checkpoint and not worth a separate invocation.

**Cross-step deps**: none — Step 6 is the terminal step. Outputs (BL body updates + new BL file + plan checkboxes + this summary) close the plan-015 lifecycle.

**Actual files** (tracked): `docs/plans/015-decay-operator-surface-hardening.md`, `docs/milestones/015/step-summaries.md`
**Actual files** (local-only, in `.ae/`): `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md`, `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md`, `.ae/backlog/unscheduled/BL-decay-threshold-mode.md`
