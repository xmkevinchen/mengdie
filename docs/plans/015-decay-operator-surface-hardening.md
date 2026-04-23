---
id: "015"
title: "Decay operator surface hardening — JSON schema contract + verify-decay-sh actions 1+2+4"
type: plan
created: 2026-04-23
status: reviewed
discussion: "docs/discussions/021-v0.8.0-bl-dependencies/"
---

# Feature: Decay operator surface hardening — JSON schema contract + verify-decay-sh actions 1+2+4

## Goal

Lock the `dreaming_pass` stderr-JSON contract with a schema version + JSON schema doc, prove it via an integration test that spawns the `mengdie` binary, and harden `scripts/verify-decay.sh` against silent failure modes (silent-bypass when JSON parse fails, hardcoded DB path) plus add CI coverage — so that any regression in the machine contract fails CI instead of being caught by an operator running the approval gate in production.

## Background

Source: discussion 021 `conclusion.md` Topic 1 (Plan A / split 2+1) + Topic 3 (ship hardening actions 1+2+4; action 3 already done at `verify-decay.sh:47`; action 5 defers to BL-010 sprint via new `BL-decay-threshold-mode`).

Two BLs bundled per the split-2+1 decision:

1. **`BL-decay-json-schema-contract`** (M) — all 3 actions in the BL body
2. **`BL-verify-decay-script-hardening`** (M) — actions **1, 2, 4 only**
   - Action 3 (RUST_LOG normalization) already live at `scripts/verify-decay.sh:47` — mark checkbox in BL body, no plan work
   - Action 5 (threshold-mode for daemon) defers — close the BL on plan merge with re-file of `BL-decay-threshold-mode` targeting the BL-010 sprint (trigger: BL-010 daemon plan approved)

**Intra-plan sequencing** (per Topic 3 constraint): action 2 (`--db-path`) must land before action 4 (CI coverage). The CI test spawns `mengdie dream --decay-dry-run --db-path <tmpdir>/test.db` to isolate from the operator's real `~/.mengdie/db.sqlite`.

## Steps

### Step 1: Add `schema_version: 1` field + JSON schema doc (AC1, AC2)

- [x] Edit `format_structured_json` at `src/bin/cli.rs:207-222` to emit `"schema_version": 1`. Note: `serde_json` (v1 in `Cargo.toml` with no `preserve_order` feature) outputs keys in `BTreeMap` alphabetical order by default — so `schema_version` will appear last in serialization regardless of source position. The grep at `scripts/verify-decay.sh:62` uses `^\{.*"event":"dreaming_pass".*\}$` which is position-agnostic. No ordering assertion needed.
- [x] Create new file `docs/schemas/dreaming_pass.json` — JSON Schema draft-07. Required fields: `schema_version` (const 1), `event` (const "dreaming_pass"), `promoted` (integer ≥ 0), `demoted` (integer ≥ 0), `decay_floor_breaches` (integer ≥ 0), `avg_effective_before` (number 0-1), `avg_effective_after` (number 0-1), `dry_run` (boolean), `breaches` (array of string UUIDs). Include a **Bump Rules** section at the top of the schema doc: "Bump `schema_version` when removing a field, renaming a field, or changing the semantics/units of an existing field. Do NOT bump for strictly-additive optional fields (which remain at v1). Version bumps are coordinated with `scripts/verify-decay.sh` (field whitelist) and any other consumer."
- [x] Extend the existing unit tests on `format_structured_json` (colocated with the function in `src/bin/cli.rs`) to assert `schema_version` field is present and equals `1`. This is the Step 1 unit-level check; Step 2 owns the subprocess-level (transport) assertion.

Expected files: `src/bin/cli.rs`, `docs/schemas/dreaming_pass.json` (new)

### Step 2: Stderr-JSON contract integration test (AC3)

- [x] Create `tests/decay_contract.rs` (new integration test file). Before writing, confirm `Cargo.toml` `[[bin]]` section names the binary exactly `mengdie` — `env!("CARGO_BIN_EXE_mengdie")` depends on that exact name.
- [x] Test spawns `mengdie` binary via `std::process::Command`, locating via `env!("CARGO_BIN_EXE_mengdie")` — first-ever CLI subprocess test pattern in this repo (per archaeologist Round 3 verification). Runs on the same host as `cargo test` (Linux x86_64 on Forgejo CI; native macOS on local dev per discussion 017 platform matrix) — no cross-compile concern.
- [x] Test seeds a tempfile SQLite DB using `tempfile::NamedTempFile` — opens a direct `rusqlite::Connection` on the tempfile path to insert **at least one long-term memory** with known `avg_relevance` (e.g., 0.5) and known `last_recalled` (e.g., 90 days ago) before invoking the binary. Follows pattern at `tests/e2e.rs:140-177`. Seeding is required so `avg_effective_before` / `avg_effective_after` assertions are on a non-zero value — without seeding these fields are exactly `0.0` (per `src/core/dreaming.rs:203-206` null-guard), which type-checks trivially and does not prove the computation path.
- [x] **Lifetime trap** (per plan 015 doodlestein-adversarial): `NamedTempFile` deletes the file on drop. The `tmp` binding must remain in scope until AFTER `Command::output()` returns — hold it in a `let tmp = …` at the top of the test function and use `tmp.path()` when invoking the binary. If the binding is dropped before the subprocess runs (e.g., shadowed in a nested block), the binary opens an empty DB, all fields become zero via the null-guard, and the `avg_effective_before > 0.0` assertion silently reports the wrong failure mode.
- [x] Test invokes `mengdie --db-path <tempfile> dream --decay-dry-run` (the `--db-path` global arg is confirmed present at `src/bin/cli.rs:17-18` — DO NOT add a second flag; the clap-derived flag name is exactly `--db-path`). Captures stderr. Greps for `^\{.*"event":"dreaming_pass".*\}$` line (same pattern as `scripts/verify-decay.sh:62`).
- [x] Test parses JSON with `serde_json::from_str`, asserts: `schema_version == 1`, `event == "dreaming_pass"`, all 9 fields present by name, type-checks each per the JSON schema at `docs/schemas/dreaming_pass.json`, and asserts `avg_effective_before > 0.0` (non-trivial value from seeded data).
- [x] Test must fail if a regression wraps the JSON line in tracing output (the Step 4-of-plan-013 regression class) — verify by grep-matching the bare `^{...}$` line, NOT a tracing-prefixed line with timestamp.

Expected files: `tests/decay_contract.rs` (new)

### Step 3: Harden verify-decay.sh silent-bypass on unparseable JSON (AC4) — BL-verify action 1

- [ ] Edit `scripts/verify-decay.sh` lines 64-73 — the `if [[ -z "$JSON_LINE" ]]` block currently exits 0 when `--i-reviewed-each` is passed AND JSON is unparseable. This is a silent-bypass: operator "approves" breaches they cannot see.
- [ ] Change behavior: when `JSON_LINE` is empty, **always exit 2** regardless of `--i-reviewed-each`. Rationale: approval is conditional on seeing the breach list; an unparseable JSON line invalidates the approval semantics. The preceding block at lines 48-52 already handles the "binary exited non-zero" path with its own exit 2, so this branch specifically covers "binary exited 0 but emitted no JSON line" — which is either a format regression (caught here) or an edge-case crash during stderr flush (rare; operator must investigate, not bypass).
- [ ] Error message MUST distinguish the two failure causes: (a) `TMP_ERR` is empty → "mengdie produced no stderr output — binary may have crashed before emitting the dreaming_pass event" (transient failure hint); (b) `TMP_ERR` is non-empty but contains no `dreaming_pass` line → "mengdie emitted stderr but no dreaming_pass JSON line — schema contract regression, verify mengdie binary output format" (format regression hint). Use `if [[ -s "$TMP_ERR" ]]` to branch the message. Both paths still exit 2.
- [ ] Update script header comment block (lines 1-20) to note the approval-gate invariant: `--i-reviewed-each` requires a parseable JSON line. State that a repeated exit 2 with the "transient failure" message is an operator-escalation signal, not a script bug — the BL-010 daemon (when it lands) will replace this interactive gate with a threshold alarm.

Expected files: `scripts/verify-decay.sh`

### Step 4: Add `--db-path <path>` flag to verify-decay.sh (AC5) — BL-verify action 2

- [ ] Edit `scripts/verify-decay.sh` arg-parse loop (lines 23-33) to accept `--db-path <path>`; default remains `~/.mengdie/db.sqlite` when flag absent. Mirror the CLI's flag name exactly — the shell script's `--db-path` passes through to the binary's `--db-path` global arg.
- [ ] Thread the path through to the `mengdie dream --decay-dry-run` invocation at line 47 using the **existing** `--db-path` global arg on the binary (confirmed present at `src/bin/cli.rs:17-18` as `#[arg(long, global = true)] db_path: Option<PathBuf>`; clap derives `--db-path` from the field name). **DO NOT add a second flag on the Rust side.** The invocation becomes: `mengdie --db-path "$DB_PATH" dream --decay-dry-run >"$TMP_OUT" 2>"$TMP_ERR"` (global arg before subcommand — position is flexible with `global = true`, but before-subcommand is the conventional placement in this repo's script).
- [ ] Update script header comment block (lines 1-20) usage block to document the new `--db-path` flag and its default.
- [ ] Do NOT edit `docs/operations/dreaming-decay.md` in this plan. The ops doc update for the `--db-path` addition (if any operator procedure actually changes) is Plan B / `BL-decay-ops-doc-polish` scope per discussion 021 conclusion. Plan A's doc surface is the script header comment only.

Expected files: `scripts/verify-decay.sh` (only)

### Step 5: CI integration test invoking verify-decay.sh (AC4, AC5) — BL-verify action 4

- [ ] **Default: extend `tests/decay_contract.rs`** from Step 2 with a new `#[cfg(unix)]` module for the shell-script tests. Rationale: seed helpers, `CARGO_BIN_EXE_mengdie` lookup, and tempfile setup are all already in `decay_contract.rs`; a separate file would duplicate them. Only create `tests/verify_decay_script.rs` as a new file if the `#[cfg(unix)]` module makes `decay_contract.rs` exceed ~300 lines or if helper reuse proves awkward.
- [ ] Test shells out to `scripts/verify-decay.sh --db-path <tmp-seeded-db>` with and without `--i-reviewed-each`, asserting the exit-code matrix:
  - No breaches, no flag → exit 0
  - Breaches, no flag → exit 1
  - Breaches, `--i-reviewed-each` → exit 0
  - Unparseable JSON + `--i-reviewed-each` → exit 2 (regression guard for Step 3's silent-bypass fix — **this is why Step 5 depends on Step 3 as well as Step 4**; see dependency graph below)
- [ ] For the "unparseable JSON" test case, place a shim `mengdie` executable earlier in `$PATH` than the real binary — shim exits 0 with empty stderr. Script must then see empty `JSON_LINE` and exit 2 per Step 3's hardening. **Record the exact shim construction in the test** so AC4's verification procedure is reproducible (fixes the `$PATH_HEAD` issue in AC4's sample).
- [ ] Test must use `--db-path` for the seeded-DB cases (Step 4 dependency) to isolate from operator's real DB.
- [ ] No changes to `.forgejo/workflows/ci.yml` required — the `cargo test` job at `.forgejo/workflows/ci.yml:37-46` already runs integration tests; this test lands inside that job automatically.

Expected files: `tests/decay_contract.rs` (extended) OR `tests/verify_decay_script.rs` (new, fallback only)

### Step 6: BL close + action 5 re-file + in-place BL body updates (AC6)

- [ ] Update `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md` — add `status: done` to frontmatter and a "Shipped in plan 015" note at bottom of body.
- [ ] Update `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md` — mark action 3 checkbox as already-done (citation: `scripts/verify-decay.sh:47`); mark actions 1, 2, 4 done; set `status: done`; add a "Shipped in plan 015 (actions 1+2+4); action 5 re-filed as BL-decay-threshold-mode" note.
- [ ] Create new BL file `.ae/backlog/unscheduled/BL-decay-threshold-mode.md` — carries forward action 5 body text verbatim (the `--threshold=N` flag + `decay_spike` JSON event design). Frontmatter trigger: "BL-010 daemon plan approved / work starts." Origin: "split from BL-verify-decay-script-hardening per discussion 021 conclusion." Preserves plan 013 review citation chain.
- [ ] **Add a "Schema Contract Obligation" section** to the new BL body (per plan 015 doodlestein-strategic): a one-paragraph clause stating that when this BL ships, the `decay_spike` event MUST follow the schema-contract methodology established by plan 015 — i.e., include a `schema_version` field, land alongside a JSON Schema doc (`docs/schemas/decay_spike.json` or equivalent), and carry an integration test that spawns the binary and asserts the contract. The BL-010 plan reviewer reading this BL should see the obligation front-and-center, not have to reconstruct it from plan-015 history. Without this clause the deferred work could reinvent the exact regression plan 015 was written to prevent.
- [ ] **Do NOT `mv` the closed BL files** from `.ae/backlog/v0.8.0/` to `.ae/backlog/done/v0.8.0/` in this step. File location is the `/ae:roadmap` skill's authoritative routing key, and moving files is the `/ae:roadmap close v0.8.0` invocation's job, not this plan's. A phantom-state window exists between plan merge (frontmatter says `status: done`) and sprint close (file moves) — this is accepted per discussion 021 conclusion which sequences sprint close AFTER all plan work lands. The gate condition in `.ae/roadmaps/v0.8.0.md` ("All N BL-decay-* closed") is evaluated by the human running `/ae:roadmap close`, not mechanically. Document this explicitly in the plan-completion commit message: "BLs marked status: done in-place; sprint-close will move them per discussion 021 Next Step 4."
- [ ] Verify the new `BL-decay-threshold-mode.md` file is well-formed: YAML frontmatter parses, body has "What / Why / How to apply / Trigger" sections matching the existing backlog convention (see `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md` for the template).

Expected files: `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md`, `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md`, `.ae/backlog/unscheduled/BL-decay-threshold-mode.md` (new)

## Acceptance Criteria

### AC1: `format_structured_json` emits `schema_version: 1`

**Verification**:
```bash
cargo run --bin mengdie -- dream --decay-dry-run 2>&1 >/dev/null | grep -E '"event":"dreaming_pass"' | jq '.schema_version'
# Expected output: 1
```
Also: unit test on `format_structured_json` asserts the presence and value of `schema_version` in the returned string.

### AC2: JSON schema file exists and validates a sample event

**Verification**: `docs/schemas/dreaming_pass.json` exists, is valid JSON Schema draft-07, and validates against a sample event with all 9 fields (schema_version, event, promoted, demoted, decay_floor_breaches, avg_effective_before, avg_effective_after, dry_run, breaches). Manual check: pipe the CLI's actual output through a JSON-schema validator (`ajv`, `jsonschema` Python, or `python -c "import jsonschema"`) and confirm zero validation errors.

### AC3: Integration test spawns `mengdie` binary and asserts full JSON contract on a seeded non-zero DB

**Verification**: `cargo test --test decay_contract` passes. The test must:
- Spawn the binary via `env!("CARGO_BIN_EXE_mengdie")` (NOT via library call — transport must be real subprocess)
- Seed the tempfile DB with at least one long-term memory (direct `rusqlite::Connection` path per `tests/e2e.rs:140-177`) — without seeding, `avg_effective_before` is `0.0` by null-guard and the assertion is trivially passing. Non-zero seeded value is required.
- Assert on the presence of all 9 contract fields by exact name
- Assert `schema_version == 1`
- Assert `avg_effective_before > 0.0` (proves computation path, not just null-guard)
- Fail with a specific error message if stderr contains a tracing-prefixed log line wrapping the JSON (regression guard for plan 013 Step 4's pre-fixup state)

This AC also covers the Step 3 regression-guard exit-code matrix when Step 5's shell-script cases are added to the same test file (same `cargo test --test decay_contract` invocation runs both sets).

### AC4: verify-decay.sh exits non-zero when JSON is unparseable — regardless of `--i-reviewed-each`

**Verification** (executable as written):
```bash
SHIM_DIR=$(mktemp -d)
cat > "$SHIM_DIR/mengdie" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$SHIM_DIR/mengdie"
PATH="$SHIM_DIR:$PATH" ./scripts/verify-decay.sh --i-reviewed-each
echo "Exit code: $?"   # Expected: 2
# The error message must mention either "no stderr output" (if TMP_ERR empty)
# or "no dreaming_pass JSON line" (if TMP_ERR has other content) — distinguishing
# transient binary failure from schema regression per Step 3's message-branching fix.
```
Replaces the previous behavior where this path exited 0 silently. Also replaces the `$PATH_HEAD` copy-paste error in the earlier draft of this AC — the variable is now `$SHIM_DIR` and is defined before use.

This AC is additionally covered mechanically by Step 5's CI test which exercises the same exit-code assertion via `cargo test` (fixing the "manual procedure only" limitation of the earlier AC6 draft).

### AC5: verify-decay.sh `--db-path` flag threads through to the binary and script does not touch default DB when flag passed

**Verification**:
```bash
TMPDB=$(mktemp)
./scripts/verify-decay.sh --db-path "$TMPDB"
# Expected: script uses $TMPDB; ~/.mengdie/db.sqlite is NOT opened during this invocation
# Verify via: (before run) stat -f %m ~/.mengdie/db.sqlite ; (after run) same mtime
```

### AC6: BL bodies closed + action 5 re-filed with trigger preserved

**Verification** (all review-time checkable, no post-merge dependency):
- `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md` frontmatter has `status: done`
- `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md` frontmatter has `status: done` and body has action 3/1/2/4 checkboxes marked, action 5 explicitly noted as split out
- `.ae/backlog/unscheduled/BL-decay-threshold-mode.md` exists with frontmatter trigger "BL-010 daemon plan approved / work starts" and a citation back to plan 015 + discussion 021
- New BL's body has the standard "What / Why / How to apply / Trigger" sections (parses as YAML frontmatter + markdown; reviewer can confirm at review time by reading the file)

**Note**: The earlier-draft AC that included `git log --oneline -n 1` is removed. That was post-merge-only verifiable and therefore violated the review-time-verifiability rule (prior mengdie guidance: ACs must be verifiable at review time, not post-ship). The file-level checks above are fully verifiable at review time.

**Note**: The earlier-draft separate "CI catches regression" AC has also been folded into AC3 — that AC was functionally a post-ship manual exercise subsuming AC3's integration test, not a distinct checkable criterion.

## Step Dependency Graph

```
Step 1 (schema_version + schema doc)
  ├─→ Step 2 (unit + integration test assert on schema_version)
  └─→ (soft) Step 5 via the shell script — once Step 1 ships, mengdie's stderr
             includes schema_version; Step 5's "breaches, --i-reviewed-each → exit 0"
             path invokes the real binary which emits the schema_version. Soft
             dependency: Step 5 compiles and its exit-code matrix passes without
             Step 1 merged, but AC3's full regression story requires both.

Step 3 (harden silent-bypass, edits verify-decay.sh lines 64-73)
  └─→ Step 5 (HARD — Step 5's "unparseable JSON + --i-reviewed-each → exit 2"
             case is the regression guard for Step 3's fix. Without Step 3
             merged, this case exits 0 instead of 2 and the test fails.)

Step 4 (--db-path flag, edits verify-decay.sh lines 23-33 + invocation at :47)
  └─→ Step 5 (HARD — the seeded-DB cases need --db-path to isolate from the
             operator's real ~/.mengdie/db.sqlite)

Step 6 (BL close + re-file) — depends on Steps 1-5 all landed
```

**Serial path (correct execution order)**: Step 1 → Step 2 → Step 3 → Step 4 → Step 5 → Step 6

Steps 3 and 4 both edit `scripts/verify-decay.sh` in non-overlapping regions (Step 3 at lines 64-73, Step 4 at lines 23-33 + :47). They are semantically independent — a human could do them in parallel with no merge conflict. But serializing them (3 first, then 4) avoids `/ae:work` drift-detection complaints about a file touched by two consecutive steps. Both Steps 3 and 4 also update the script header comment block (lines 1-20). Serialization is the pragmatic call for an automated work agent.

**Note on Step 3 → Step 5 edge** (added per codex plan-quality review): the initial graph only showed Step 4 → Step 5. The missing edge is real: Step 5's exit-code matrix includes a case that specifically validates Step 3's silent-bypass fix. Skipping Step 3 or reordering Step 5 before Step 3 would cause that test case to fail.

## Parallel Strategy

None within `/ae:work` — the step chain is linear per the dependency graph. No parallelism benefit for 6 small steps.

## Out of Scope

- Plan B (ops-doc-polish) — trails this plan per discussion 021 Next Step 6
- Action 5 design (threshold-mode / `decay_spike` event schema) — deferred to BL-010 sprint via `BL-decay-threshold-mode`
- Synthesis cluster planning — discussion 021 Next Step 7 requires a separate mini-discuss on provenance options first
- Updating `.ae/roadmaps/v0.8.0.md` gate text or issuing `/ae:roadmap remove` for the 2 defer-items — those are Next Step 3 + 4 of discussion 021, NOT part of this plan

## Known Risk / Review Focus

- **First-ever CLI subprocess test in this repo**: `tests/decay_contract.rs` establishes the `env!("CARGO_BIN_EXE_mengdie")` pattern. Cargo auto-generates this env var for integration test binaries when `[[bin]]` declares `mengdie` in `Cargo.toml` — confirm that name first (Step 2 subtask). Forgejo CI runs `cargo test` on `ubuntu-latest` (Linux x86_64 native, no cross-compile per discussion 017 platform matrix) — the pattern works on the same host the binary is built and tested on. No cross-compilation adapter needed.
- **Schema version stability commitment**: shipping `schema_version: 1` commits to following the Bump Rules documented at the top of `docs/schemas/dreaming_pass.json` (Step 1): bump on remove/rename/semantics-change; no bump on additive optional fields. Reviewers should verify this rule block is present and unambiguous before approving the plan.
- **`--db-path` is the confirmed-present flag name**: `src/bin/cli.rs:17-18` declares the global arg as `db_path: Option<PathBuf>` which clap renders as `--db-path` (kebab-case derivation). The plan's Step 2, Step 4, and Step 5 all reference `--db-path` — DO NOT add a second flag. An earlier draft mistakenly referred to this as `--db` in multiple places; that error is fixed but reviewers should grep the plan for `--db\b` to confirm no stragglers remain.
- **Operator recovery path on Step 3's exit 2**: a legitimate operator could see exit 2 if the `mengdie` binary crashes before emitting the `dreaming_pass` JSON line (OOM, SIGPIPE, disk-full mid-write). Step 3 addresses this by distinguishing the error message — `[[ -s "$TMP_ERR" ]]` branches "transient binary failure" vs "schema regression" messaging. Neither path offers a bypass flag (intentional — bypassing a missing breach-list is exactly the silent-approval anti-pattern this plan fixes). Recovery for the transient case is "re-run"; if that fails too, the operator escalates and BL-010 daemon is the eventual threshold-based alternative.
- **BL phantom-state window on merge**: Step 6 sets `status: done` in BL frontmatter but does NOT `mv` the files to `.ae/backlog/done/v0.8.0/`. That move belongs to the `/ae:roadmap close v0.8.0` invocation per discussion 021 Next Step 4. Between this plan's merge and sprint close, `/ae:dashboard` or `/ae:next` tooling that keys on directory location may display the BLs as still-active. Acknowledged constraint; matches the convention throughout the project. The plan-completion commit message documents this (Step 6 subtask).
