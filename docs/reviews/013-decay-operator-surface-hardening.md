---
id: "013"
title: "Review: decay operator surface hardening (plan 015)"
type: review
created: 2026-04-23
target: "docs/plans/015-decay-operator-surface-hardening.md"
verdict: pass
---

# Review: Plan 015 — Decay Operator Surface Hardening

## Scope

Deep review of 7 commits in `52e1899..HEAD` landing plan 015 (discussion 021 Topic 1 / Plan A). Covers:
- `src/bin/cli.rs` — `format_structured_json` + `schema_version: 1` field + extended unit tests
- `docs/schemas/dreaming_pass.json` — new JSON Schema draft-07 with Bump Rules
- `tests/decay_contract.rs` — new first-ever CLI subprocess integration test + `#[cfg(unix)]` shell-script test module
- `scripts/verify-decay.sh` — silent-bypass removal (action 1) + `--db-path` flag (action 2) + header invariant doc
- `.ae/backlog/v0.8.0/BL-decay-json-schema-contract.md` + `.ae/backlog/v0.8.0/BL-verify-decay-script-hardening.md` + `.ae/backlog/unscheduled/BL-decay-threshold-mode.md` — BL close-out + re-file (local-only, `.ae/` is gitignored)
- `docs/discussions/021-v0.8.0-bl-dependencies/index.md` — Completion Invariant writeback

## Team

| Agent | Angle | Backend |
|-------|-------|---------|
| architecture-reviewer | schema contract locking, module boundaries, methodology-as-pattern | Claude |
| security-reviewer | approval-gate bypass, path injection, `set -euo pipefail` interaction, TOCTOU, data exposure | Claude |
| code-reviewer | Rust idioms, test determinism, shell script quality, commit discipline | Claude |
| challenger | blind spots + targeted challenges (pure opposition, no synthesis) | Claude |
| codex-proxy | cross-family: contract stability and review discipline | OpenAI Codex (high reasoning) |
| gemini-proxy | cross-family: shell script security + edge cases | Google Gemini → local gemma-4-26b-a4b-it-4bit (quota exhausted, fell back) |

## Findings Summary

| Severity | Count | Resolution |
|----------|-------|------------|
| P1       | 1     | Fixed in `56613bb` |
| P2       | 6     | 5 fixed in `56613bb`; 1 folded into hardening |
| P3       | 11    | Accepted or noted |
| Rejected | 1     | Gemini trap+`-u` interaction (evidence-refuted) |

## P1 — Fixed

### P1.1 — `--db-path` to nonexistent file silently approves empty DB (codex)

**Finding**: `rusqlite::Connection::open` creates the DB file if absent. A typoed `--db-path /tmp/typo.db` invocation would produce an empty corpus → `decay_floor_breaches: 0` → `exit 0`. The operator believes they approved a clean corpus when they actually approved nothing. Same silent-approval anti-pattern the plan fixed for the unparseable-JSON case, different entry point.

**Fix** (`56613bb`, `scripts/verify-decay.sh`): Added existence check before the binary invocation:

```bash
if [[ -n "$DB_PATH" ]] && [[ ! -f "$DB_PATH" ]]; then
  echo "error: --db-path \"$DB_PATH\" does not exist." >&2
  echo "       Refusing to create an empty DB for the approval gate." >&2
  exit 2
fi
```

Also added a regression test at `tests/decay_contract.rs::verify_decay_script::nonexistent_db_path_exits_two`.

## P2 — Fixed

### P2.1 — `additionalProperties: false` declared in schema but not enforced in test (triple-confirmed: architect Q4 + codex 2 + challenger C4)

**Finding**: `docs/schemas/dreaming_pass.json` declares a closed object. The integration test asserts the 9 known fields exist but does NOT assert "no other fields exist." A 10th field added to `format_structured_json` without a schema-doc update would slip through silently.

**Fix** (`56613bb`): Added expected-key-set assertion in `dreaming_pass_stderr_json_matches_plan_015_contract`:

```rust
let expected_keys: HashSet<&str> = [...].into_iter().collect();
let unexpected: Vec<&String> = actual_keys.iter()
    .filter(|k| !expected_keys.contains(k.as_str()))
    .collect();
assert!(unexpected.is_empty(), ...);
```

Now any schema drift fires in CI with a pointer to the Bump Rules.

### P2.2 — Breach-triggering test fixture on marginal wall-clock boundary (code-reviewer)

**Finding**: `breaches_no_flag_exits_one` and `breaches_with_approval_exits_zero` used `(avg=0.487, days_ago=78)` which produces `effective ≈ 0.1977` (only 1% below 0.20 floor). Under wall-clock drift at day boundaries, could flip pass/fail.

**Fix** (`56613bb`): Fixture changed to `(avg=0.4, days_ago=80)` → `effective ≈ 0.159` (20% below floor). Comfortable margin.

### P2.3 — `scripts/verify-decay.sh` dual-branch `mengdie ... dream` invocation (code-reviewer)

**Finding**: The `if [[ -n "$DB_PATH" ]] / else` blocks duplicated the entire `RUST_LOG=... mengdie ... dream --decay-dry-run >... 2>...` invocation. Two copies of the same logic — future edits risk one-branch-only mistakes.

**Fix** (`56613bb`): Collapsed to a single invocation via bash arg array:

```bash
MENGDIE_ARGS=()
if [[ -n "$DB_PATH" ]]; then MENGDIE_ARGS+=("--db-path" "$DB_PATH"); fi
MENGDIE_ARGS+=("dream" "--decay-dry-run")
RUST_LOG="${RUST_LOG:-info}" mengdie "${MENGDIE_ARGS[@]}" >"$TMP_OUT" 2>"$TMP_ERR" || { ... }
```

No behavior change; single maintenance surface.

### P2.4 — Schema Contract Obligation lives only in gitignored `.ae/` (challenger C3)

**Finding**: `BL-decay-threshold-mode.md` (the re-file for action 5) carries the Schema Contract Obligation clause, but `.ae/` is in `.gitignore` per project convention. Fresh clone / machine wipe / accidental `rm -rf .ae/` loses the obligation text silently. The BL-010 plan reviewer who inherits this obligation would have to reconstruct it from plan-015 history.

**Fix** (`56613bb`, `docs/plans/015-decay-operator-surface-hardening.md`): Duplicated the Schema Contract Obligation into plan 015's tracked content under a new "Methodology Carryforward for BL-010" section. Source of truth stays in the BL body; the plan file is the durability hedge.

### P2.5 — Bump Rules silent on type drift and invariant changes (architect + codex)

**Finding**: The `$comment` block in `docs/schemas/dreaming_pass.json` listed 3 bump triggers (remove / rename / semantic change). It did NOT cover: (a) type narrowing/widening (e.g., integer → number; scalar → array), (b) structural invariant changes (e.g., the `breaches.length == decay_floor_breaches` guarantee being loosened), or (c) adding a NEW REQUIRED field (only "strictly-additive optional" is correctly no-bump).

**Fix** (`56613bb`): Bump Rules expanded from 3 triggers to 6 — added type drift, invariant change, new-required-field. Language also clarified that the `tests/decay_contract.rs` expected-keys set is now part of the coordinated-change surface along with the schema doc + producer + consumer.

### P2.6 — Exit-code matrix coverage incomplete (codex)

**Finding**: Step 5's integration tests covered no-breach/breach × with/without `--i-reviewed-each` + unparseable-JSON. They did NOT test: missing `--db-path` value → exit 2, unknown arg → exit 2, `mengdie` not on PATH → exit 2 (all documented but not regression-guarded).

**Fix** (`56613bb`): Added 4 new tests in `mod verify_decay_script`: `nonexistent_db_path_exits_two` (also backs P1.1), `db_path_without_value_exits_two`, `unknown_arg_exits_two`, `missing_mengdie_on_path_exits_two`. Full documented exit-code matrix now CI-guarded.

## Rejected

### REJECTED — Gemini: trap + `set -u` unset-variable interaction (with evidence)

**Finding (gemini)**: "If the script exits before `TMP_OUT`/`TMP_ERR` are assigned, the trap on line 64 will fire with unset variables under `set -u` and error out."

**Rejection rationale**: The trap is INSTALLED on line 64, AFTER the `TMP_OUT=$(mktemp)` (line 62) and `TMP_ERR=$(mktemp)` (line 63) assignments. If either `mktemp` fails, `set -e` exits the script BEFORE line 64 is reached — the trap is never registered, and the unset-variable scenario gemini described cannot occur. The `security-reviewer`'s independent analysis reached the same conclusion ("trap is never installed, TMP_OUT is leaked [as a benign zero-byte file]" — P3, benign).

**Defense-in-depth folded in anyway**: `${TMP_OUT:-}` and `${TMP_ERR:-}` substituted in the trap body — zero cost, belt-and-braces against any future ordering changes.

## P3 Accepted (not blocking)

Noted for future backlog or deferred:

| # | Finding | Severity | Disposition |
|---|---------|----------|-------------|
| 1 | Bump Rules type-drift gap (architect) | P3 | Superseded by P2.5 fix (expanded Bump Rules) |
| 2 | Test assertions use `is_number()` not `is_u64()` (architect) | P3 | Fold into BL-decay-threshold-mode; schema-doc validation in that plan will catch it |
| 3 | Schema doc is descriptive not machine-validated against live output (architect) | P3 | Promote when BL-decay-threshold-mode lands (two events on same stderr channel makes "is the doc authoritative" operationally urgent) |
| 4 | Three independent field lists (producer, consumer, test) | P3 | Observation; no action at n=9. Revisit when contract grows |
| 5 | Helper duplication in tests (`seed_one_longterm` vs `seed_longterm`) | P3 | Stylistic; both documented and intentional |
| 6 | PATH hijacking of bare `mengdie` (security) | P3 | Operator-only concern; backlog item if verify-decay.sh is ever CI-automated |
| 7 | `mktemp` partial-failure leak (security) | P3 | Benign zero-byte file; not worth pre-initializing guards |
| 8 | TOCTOU in shim binary creation (security) | P3 | 0700 tempdir closes the window; no action |
| 9 | Breach UUIDs in CI test failure messages (security) | P3 | Synthetic test data; no production exposure |
| 10 | POSIX `--` end-of-options separator unsupported (gemini) | P3 | Internal ops script; no action |
| 11 | Stale installed `mengdie` on PATH surfaces as uninformative exit-2 (challenger C2) | P3 | Pre-existing; backlog item if operator UX becomes a concern |
| 12 | SQLite `datetime()` format ≠ RFC3339 → silent exclusion from decay (challenger C5) | P3 | Latent in plan-013 dreaming code; out of plan-015 scope. Backlog item |

## Outcome Statistics

```
Steps completed: 6/6
Rework rate: 0 steps needed mid-work fixup commits (0/6 = 0%)
P1 escape rate: 1 P1 escaped to /ae:review (codex's --db-path silent-approve) — target is 0
Drift events: 0 Check B drift during /ae:work (all steps matched Expected files)
Fix loop triggers: 0 circuit breaker activations during /ae:work
Auto-pass rate: 5/6 steps auto-continued (Step 6 paused briefly for user input when step-summaries ordering was noted; user confirmed continue)
Deferred resolution rate: N/A (no deferred findings created during /ae:work)
```

**P1 escape analysis**: The codex P1 (nonexistent `--db-path`) was not caught by plan-time review or any pre-commit check during `/ae:work`. It IS a direct analog of the unparseable-JSON silent-bypass the plan fixed — same failure shape, different input. Plan 015's own framing should have surfaced it ("silent approval of unseen state" applies to both JSON unparseability AND corpus emptiness). Recording as a review-pattern memory: when fixing one silent-approval path, systematically enumerate ALL upstream inputs that could produce the same downstream symptom.

## Fixups Landed

One consolidated fixup commit `56613bb` instead of four separate `--fixup=<hash>` targets + autosquash. Rationale: working on `main` (no feature branch); project convention for /ae:review fixups on main is a single `fixups` commit (see `7401e9c` "BL-014 /ae:review fixups"). One commit keeps the fixup scope reviewable atomically.

## Deferred Findings Audit

No `docs/milestones/015/notes.md` was created during `/ae:work`, meaning no findings were deferred mid-step. Audit: **no unresolved deferred findings**.

## Verdict

**PASS** — All P1 and P2 findings resolved in `56613bb`. P3 items accepted or backlogged. Plan 015 ships as-is.

## Knowledge Capture Targets

Three review-pattern memories to ingest (plan 015 review yielded more reusable discipline than one-off bug fixes):

1. Silent-approval enumeration: the "silent approval of unseen state" pattern has multiple entry points (unparseable JSON, nonexistent DB, empty corpus) — enumerate all upstream inputs that produce the same downstream symptom.
2. Schema-doc `additionalProperties: false` must be paired with test-level expected-key enforcement — declaration alone doesn't protect against drift.
3. Gitignored BL files holding obligations for future sprints need a tracked-file duplicate as durability hedge.
