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

## Step 2 — Stderr-JSON contract integration test (commit: TBD)
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
