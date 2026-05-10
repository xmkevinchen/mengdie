---
id: "019"
title: "synthesis.rs JSON parser — adopt claude-CLI --json-schema structured output"
type: plan
created: 2026-05-09
status: reviewed
discussion: ""
feature: ""
---

# Feature: synthesis.rs JSON parser — adopt claude-CLI --json-schema structured output

## Goal

Replace `src/core/synthesis.rs::extract_first_json_object` (~30 LoC brace-depth
scanner with string-state tracking) with claude-CLI's native `--json-schema` +
`--output-format json` flags, so the LLM returns a token-decode-constrained
structured payload and the synthesis parse path collapses to a single
`serde_json::from_str` against a pre-validated JSON object.

Preserves: subprocess + credential-delegation design, `LlmProvider` trait
shape, `SynthesisOutcome::{Synthesized, Skipped}` enum, all 5 dreaming-pipeline
result counters (`syntheses_llm_skipped` / `parse_errors` / etc.).

## Source

Standalone plan derived from `docs/backlog/unscheduled/BL-027-rig-extractor-synthesis-json-parser.md`
(Path B section — verified viable 2026-05-09 via smoke tests in `/tmp/headless-verify/`).
No discussion conclusion source: BL-027 reopen-context contains all design decisions.
Plan-review findings (architect + code-reviewer + sec-reviewer + dep-analyst +
codex-proxy + gemini-proxy) and Doodlestein challenge (strategic + adversarial +
regret) merged 2026-05-09 — see commit history for trace.

## Out of scope

Excluded with rationale:

- **Path C — direct Anthropic HTTP API + tool-call structured outputs**:
  rejected. Anthropic API is metered pay-as-you-go (~$0.24 per 40K-token
  call) on a budget separate from Claude Code Pro flat-fee. Switching
  credential model from CLI-delegated to direct `ANTHROPIC_API_KEY`
  doubles per-call cost without offsetting the "cleaner parsing" win.
  Reversible if subscription pricing changes.
- **Path D — codex-CLI as primary synthesis LLM**: rejected. BL-027
  verification shows codex returns raw JSON (cleaner than claude's
  wrapper), but no `--system` equivalent for loading CLAUDE.md context;
  re-implementing context injection violates v0.0.1's "avoid re-inventing
  wheels" thesis. May revisit post-v0.0.1 as a secondary provider.
- **codex-CLI variant using `--output-schema`**: BL-027 marks "Optional,
  not in v0.0.1 scope". File a separate BL if exploring codex as a
  secondary provider.
- **Schema flattening (single shape with `skip_reason: ""` sentinel)**:
  considered; deferred. `oneOf` is correct per BL-027 boundary finding
  (hallucination case is real). Reconsider post-v0.0.1 if observed
  skip-path-rejection rate is < 1%.
- **Version pre-flight probe (`probe_json_schema_flag`)**: rejected per
  Doodlestein-regret + Doodlestein-adversarial convergent finding.
  YAGNI for single-operator Pro-plan auto-update environment; OnceLock
  cache would create a stale-state foot-gun in long-running mengdie-mcp
  daemon (cached `Unsupported` persists across CLI upgrade). Replaced
  with an error-message hint on `StructuredOutputMissing` /
  `StructuredOutputWrapperInvalid` (see Step 2). dep-analyst Risk 1
  downgraded MUST-FIX → CONSIDER and accepted this resolution.
- **`LlmError::CapabilityNotSupported` variant**: rejected as gold-plating.
  Default impl on `LlmProvider` reuses `UnknownProvider` with a code
  comment explaining the semantic stretch.
- **Schema migration tooling**: out of scope. The schema is a static
  `&str` constant; versioning is a future BL if shape ever changes.
- **`--system-prompt-file` to remove all argv exposure**: out of scope.
  Same-class privacy posture preserved by Pre-Step Option A; not a
  regression.

## Pre-Step: Reconnaissance documentation

Per Doodlestein-strategic finding (architect accepted): BL-027's verification
block already executed the smoke test on 2026-05-09 in `/tmp/headless-verify/`.
The discovery is done; this Pre-Step is a documentation write, not a re-run.

- [x] Write `docs/spikes/019-synthesis-cli-stdin-vs-argv-probe.md`. Record
      **Option A** (prompt as positional argv, stdin null/closed) as the
      chosen path. Cite BL-027 verification evidence (2026-05-09,
      `/tmp/headless-verify/`) as the smoke-test record. State the
      privacy-posture conclusion: **same class as existing `--system-prompt`
      argv exposure** per `src/core/llm.rs:5-10` module-doc; not a class
      escalation.

Step 1 (schema design) and Step 2 (subprocess implementation) can proceed
in parallel with this Pre-Step write — there is no real gating dependency.

Expected files: `docs/spikes/019-synthesis-cli-stdin-vs-argv-probe.md` (new).

## Steps

### Step 1: Schema design + SYSTEM_PROMPT update (AC1) ✅ 0b9bd76

The schema covers both `SynthesisOutcome` arms (synthesis vs skip) because
schema-constrained generation will force the model to fabricate syntheses
for clusters that lack a common thread (BL-027 boundary finding). The
prompt body explicitly discourages lazy skips (codex-proxy finding) so the
schema's "I refuse" path is not abused.

- [x] Add `pub(crate) const SYNTHESIS_OUTPUT_SCHEMA: &str` in
      `src/core/synthesis.rs` immediately after the existing `SYSTEM_PROMPT`
      constant. JSON Schema document (string-encoded JSON) with top-level
      `oneOf`:
      ```json
      {"oneOf": [
        {"type":"object",
         "properties":{"title":{"type":"string","maxLength":80},
                       "content":{"type":"string"},
                       "entities":{"type":"array","items":{"type":"string"},
                                   "minItems":2,"maxItems":6}},
         "required":["title","content","entities"],
         "additionalProperties":false},
        {"type":"object",
         "properties":{"skip":{"const":true},
                       "reason":{"type":"string","minLength":20}},
         "required":["skip","reason"],
         "additionalProperties":false}
      ]}
      ```
      `skip.reason` `minLength: 20` raises the cost of lazy-skip decisions.
- [x] Update `SYSTEM_PROMPT` body so the natural-language instruction
      (a) names the two output shapes ("synthesis shape" and "skip shape"),
      (b) carries the anti-lazy language verbatim: *"Only skip if the
      cluster demonstrates fundamental semantic incoherence (state
      specifically what prevents synthesis). Otherwise you MUST synthesize
      even a minimal common thread."* The schema constraint and the prose
      must not contradict; if they drift, the schema wins.
- [x] Add unit test `schema_const_parses_as_oneof_with_two_object_branches`
      (more precise than the original `schema_const_is_valid_json` per
      Doodlestein-adversarial #2). Test parses the constant via
      `serde_json::from_str::<serde_json::Value>` AND asserts:
      (a) result has key `oneOf`,
      (b) `oneOf` value is a JSON array,
      (c) the array has exactly 2 elements,
      (d) each element has `type: "object"`, a non-empty `required` array,
          and `additionalProperties: false`.
      This catches the schema-typo failure modes
      (`{"oneOf": "not-an-array"}`, missing `required` on one branch,
      etc.) that a bare `oneOf-key-exists` check would let through.
      **Do not** add the `jsonschema` dev-dep (transitive cost ~30-50 crates
      including `reqwest` / `rustls` / `aws-lc-rs`); structural shape
      assertions are sufficient. Step 4's e2e fixture pair is the
      end-to-end shape guard.

Expected files: `src/core/synthesis.rs`.

### Step 2: ClaudeCliProvider — `complete_structured` sibling method (AC2) ✅ fae11f8

Add a structured-output sibling method on `LlmProvider`. NO version
pre-flight probe — failures surface via error-message hint instead
(Doodlestein-regret + Doodlestein-adversarial convergent finding,
dep-analyst accepted).

- [x] Add `complete_structured` to the `LlmProvider` trait body **with a
      default implementation** that returns
      `Err(LlmError::UnknownProvider("structured output not supported by
      this provider".into()))`. Test mocks (`FixedProvider`, `PanicProvider`,
      `TimeoutOnFirst`, `ClusterSizeAwareProvider` in `dreaming.rs`)
      automatically inherit the default — zero changes to mocks. Add a
      1-line code comment above the default impl explaining the
      `UnknownProvider` reuse: *"Reusing UnknownProvider here is a
      semantic stretch (the provider is known, the capability isn't);
      add a `CapabilityNotSupported` variant if a second non-supporting
      provider lands."*
- [x] Add `build_structured_command(&self, system: &str, schema: &str) ->
      tokio::process::Command` as a sibling of `build_command`. Argv
      shape (Option A per Pre-Step):
      `claude -p --json-schema <schema> --output-format json
      --no-session-persistence --permission-mode bypassPermissions
      --tools "" --model <model> --system-prompt <system> <prompt-positional>`
      where the prompt is appended as the last positional argv argument.
      Set `stdin(Stdio::null())` explicitly. Per sec-reviewer advisory:
      emit `--json-schema` as TWO `.arg()` calls
      (`cmd.arg("--json-schema").arg(schema)`), never as a single combined
      string.
- [x] Add NEW constant `pub const CLAUDE_CLI_STRUCTURED_FLAGS: &[&str]`
      listing flags emitted ONLY by `build_structured_command` (i.e.,
      `--json-schema`). The existing `CLAUDE_CLI_FLAGS` constant remains
      unchanged — it tracks flags emitted by `build_command` (the
      text-output path). Per Doodlestein-adversarial #1: appending
      `--json-schema` to `CLAUDE_CLI_FLAGS` would break the existing
      drift-guard test `claude_cli_flags_constant_matches_build_command_argv`
      (llm.rs:616-628), which iterates the constant and asserts every
      entry appears in `build_command` argv. Two argv paths → two
      constants; clear ownership; no test logic forks.
- [x] Add a parallel drift-guard test
      `claude_cli_structured_flags_constant_matches_build_structured_command_argv`
      that iterates `CLAUDE_CLI_STRUCTURED_FLAGS` and asserts every entry
      appears in `build_structured_command` argv.
- [x] Add `LlmError::StructuredOutputMissing` (wrapper parsed but
      `.structured_output` field is null/absent) and
      `LlmError::StructuredOutputWrapperInvalid` (stdout is not parseable
      as the claude wrapper envelope). Both distinct from `EmptyOutput`
      (no stdout at all) and `InvalidUtf8` (bytes-level). **Both error
      messages MUST end with the diagnostic hint** `"(verify claude
      >= 2.1.138 supports --json-schema)"` so the operator sees the
      version-mismatch hypothesis without a startup probe.
- [x] In `complete_structured`, after `classify_output` succeeds:
      1. Parse stdout as `WrapperEnvelope { is_error: bool, result: String,
         structured_output: Option<serde_json::Value> }`. Accept any
         additional fields (no `additionalProperties: false` on the
         envelope; claude wrapper carries many fields the synthesis path
         ignores).
      2. Parse failure → `StructuredOutputWrapperInvalid` with the version
         hint suffix.
      3. `is_error: true` → `LlmError::NonZeroExit { code: 0,
         stderr: result, kind: ExitKind::Other }` — model-level error;
         CLI exit code is 0; `result` field carries the model's error text.
      4. `structured_output` is `None` → `StructuredOutputMissing` with
         the version hint suffix.
      5. Otherwise return `serde_json::to_string(&structured_output)?`
         so the caller can deserialize into the synthesis-or-skip union
         without re-parsing the wrapper.
- [x] Add a code comment above `WrapperEnvelope` definition: *"Wrapper
      shape pinned to claude-CLI 2.1.138 (verified 2026-05-09). If
      Anthropic renames `is_error` to `error` in a future version, this
      code silently treats `is_error` as false, and a model-level error
      propagates as StructuredOutputMissing rather than NonZeroExit.
      Acceptable for personal-use single-operator tool. Re-verify on
      claude-CLI version bump."*
- [x] Unit tests:
   - `build_structured_command_argv_includes_json_schema_flag`
   - `build_structured_command_uses_output_format_json` (vs text path)
   - `build_structured_command_passes_prompt_as_positional_argv` (Option A
     specific)
   - `build_structured_command_sets_stdin_to_null`
   - `parse_wrapper_extracts_structured_output_happy_path`
   - `parse_wrapper_is_error_true_maps_to_NonZeroExit`
   - `parse_wrapper_missing_structured_output_maps_to_StructuredOutputMissing`
   - `parse_wrapper_malformed_envelope_maps_to_StructuredOutputWrapperInvalid`
   - `error_messages_contain_version_hint` — asserts that the error
     `Display` strings for both new variants end with the diagnostic
     suffix. Pins the operator-facing diagnostic contract.
   - `claude_cli_structured_flags_constant_matches_build_structured_command_argv`

Expected files: `src/core/llm.rs`, `tests/llm_claude_cli.rs`.

### Step 3: Replace synthesis parse path; delete brace-depth scanner (AC3) ✅ 14ff5ef

With Step 2's `complete_structured` returning a pre-validated JSON object
string, `parse_synthesis_response` no longer needs preamble / postamble
tolerance.

- [x] Update `src/core/dreaming.rs` line 493 region: switch the call from
      `provider.complete(&system, &user)` to
      `provider.complete_structured(&system, &user, SYNTHESIS_OUTPUT_SCHEMA)`.
- [x] In `src/core/synthesis.rs::parse_synthesis_response`:
      1. Delete `extract_first_json_object` (lines 153-183).
      2. Delete `SynthesisError::NoJsonObject` variant.
      3. Pass `raw` directly to `serde_json::from_str::<RawEnvelope>`.
      4. Keep `RawEnvelope` struct shape and skip / synthesis branch logic.

**Test handling:**

- [x] **Delete** these tests (brace-depth-scanner-specific; cannot arise
      under structured-output mode):
   - `parser_tolerates_preamble`
   - `parser_tolerates_postamble`
   - `parser_markdown_fenced_json_extracts_cleanly`
   - `parser_escaped_quote_with_unbalanced_inner_brace_is_handled`
   - `parser_balanced_braces_inside_escaped_string`
   - `parser_inner_braces_in_content`
- [x] **Repurpose** `parser_skip_with_llm_preamble_still_parses` (preserve
      audit trail per architect MUST FIX). Rename to
      `skip_response_without_preamble_parses_cleanly`; body tests that a
      clean structured-output skip JSON (no preamble — that's the new
      contract) parses to `Skipped { reason }`. Add inline comment:
      `// Plan 019: original test name was
      parser_skip_with_llm_preamble_still_parses; preamble case can no
      longer arise under --json-schema mode.`
- [x] **Delete** `parser_malformed_json` (dead code under structured-output
      mode; claude rejects malformed inner JSON at the schema-validation
      level).
- [x] **Convert** `parser_not_json_at_all` to
      `parser_empty_string_returns_invalid_json`.
- [x] **Keep unchanged**: `parser_happy_path`, `parser_missing_title`,
      `parser_empty_title`, `parser_empty_content`,
      `parser_entities_as_objects_rejected`, `parser_skip_happy_path`,
      `parser_skip_missing_reason_returns_empty_string`,
      `parser_skip_false_is_treated_as_synthesis`.

Expected files: `src/core/synthesis.rs`, `src/core/dreaming.rs`.

### Step 4: Validation — fixtures + e2e + production run (AC4, AC5)

Merged from architect's CONSIDER. One production-DB pass captures both
the rate-limit measurement and the e2e equivalence evidence.

- [ ] **Backup the production DB** before any live LLM call (Doodlestein-
      adversarial #4):
      ```
      cp ~/.mengdie/db.sqlite ~/.mengdie/db.sqlite.bak-pre-019-$(date +%s)
      ```
      The plan does not introduce schema migration tooling, so a botched
      `--json-schema` run that produces low-quality synthesis rows (e.g.,
      lazy-skip-fallthroughs) cannot be auto-rolled-back. The backup
      gives the operator a deterministic recovery path: if the run
      regrets, restore from the timestamped backup. State the backup
      path in `docs/spikes/019-rate-limit-measurement.md` so it's
      recoverable.
- [ ] **Create** `tests/fixtures/` directory (does not exist today).
      Add `tests/fixtures/.gitkeep`.
- [ ] **Hand-craft** minimal wrapper fixtures
      `tests/fixtures/synthesis-019-wrapper-success.json` (synthesis-shape
      `structured_output`) and `tests/fixtures/synthesis-019-wrapper-skip.json`
      (skip-shape). Include only fields the parser reads: `is_error`,
      `result`, `structured_output`. Do NOT capture live output.
- [ ] **Integration test** `tests/synthesis_e2e.rs::wrapper_to_synthesis_outcome`:
      load each fixture, parse via the `complete_structured` extraction
      path, then through `parse_synthesis_response`. Both fixtures must
      produce the expected `SynthesisOutcome` variant. Match on **shape**
      (variant + field types + non-emptiness), not on exact values; use
      `serde_json::Value::get()` chains where field-presence is checked.
- [ ] **Rate-limit instrumentation** — extend `complete_structured` to
      log via `tracing::info!` per-call: `total_cost_usd`,
      `cache_creation_input_tokens`, `cache_read_input_tokens`,
      `output_tokens`, `duration_ms` from the wrapper's `usage`
      sub-object. Verify exact field names against actual claude-CLI
      output on first invocation.
- [ ] **Run** `mengdie dream --synthesize` ONCE on the (now-backed-up)
      production DB. Capture before/after timestamps. Aggregate per-call
      logs into one summary: total tokens, total elapsed, per-cluster
      average. Also capture: total clusters processed, syntheses written,
      syntheses skipped, parse_errors (target: 0).
- [ ] Write `docs/spikes/019-rate-limit-measurement.md` with: (a) backup
      file path, (b) total tokens for one full Dreaming pass, (c) total
      elapsed, (d) per-cluster average, (e) parse_errors count
      (must be 0), (f) one-sentence verdict on whether subscription-
      budget rate-limit relief is needed.
- [ ] If one full Dreaming pass consumes > 50% of operator's typical daily
      session budget → file `docs/backlog/unscheduled/BL-NNN-synthesis-rate-
      limit-relief.md`. Do NOT change credential model in this plan.
- [ ] If under 50% → no follow-up BL. Capture as acceptable in the
      spike-doc verdict line.
- [ ] **Manual quality spot-check**: read the 3 most recent synthesis rows
      written by this run from `~/.mengdie/db.sqlite`. Confirm none have
      empty `entities`, none have title >80 chars, none have `content`
      that reads like a lazy skip-shape fallthrough (e.g., generic "these
      memories share..." with no specific decision content). If any row
      smells lazy → restore from the backup file before any further
      Dreaming runs, file a follow-up BL on schema/prompt tuning.

Expected files: `tests/fixtures/.gitkeep` (new),
`tests/fixtures/synthesis-019-wrapper-success.json` (new),
`tests/fixtures/synthesis-019-wrapper-skip.json` (new),
`tests/synthesis_e2e.rs` (new), `docs/spikes/019-rate-limit-measurement.md`
(new), optionally `docs/backlog/unscheduled/BL-NNN-synthesis-rate-limit-
relief.md` or `BL-NNN-synthesis-prompt-tuning.md`.

## Acceptance Criteria

### AC1: Pre-Step result documented; oneOf schema structurally valid

`docs/spikes/019-synthesis-cli-stdin-vs-argv-probe.md` exists, naming
Option A with the BL-027 verification citation. The schema constant
`SYNTHESIS_OUTPUT_SCHEMA` parses as JSON AND has a `oneOf` array of
exactly 2 object-shaped entries each with non-empty `required` and
`additionalProperties: false` (`schema_const_parses_as_oneof_with_two_
object_branches` test passes). SYSTEM_PROMPT contains the anti-lazy-skip
language verbatim. **Note**: the test confirms structural correctness of
the schema constant; it does NOT validate the constant against the
JSON Schema metaschema (no `jsonschema` dev-dep), so subtle
schema-authoring bugs that produce a structurally valid but semantically
incorrect schema can still ship — Step 4's e2e fixture pair is the
runtime guard for that class.
**Verification**: `test -f docs/spikes/019-synthesis-cli-stdin-vs-argv-probe.md`
exits 0; `cargo test --package mengdie schema_const_parses_as_oneof_with_two_object_branches`
passes; `grep "Only skip if the cluster demonstrates fundamental"
src/core/synthesis.rs` returns 1 hit.

### AC2: ClaudeCliProvider emits structured-output flags + diagnostic hint

`build_structured_command` includes `--json-schema` and `--output-format
json` in argv, sets `stdin(Stdio::null())`, appends prompt as positional
argv. Text path (`build_command`) remains intact. `LlmError::Structured
OutputMissing` and `StructuredOutputWrapperInvalid` exist; both error
`Display` strings end with `(verify claude >= 2.1.138 supports
--json-schema)`. `CLAUDE_CLI_STRUCTURED_FLAGS` constant exists alongside
the unchanged `CLAUDE_CLI_FLAGS`; both drift-guard tests pass. NO version
pre-flight probe code (no `OnceLock`-cached startup `--help` call).
**Verification**: `cargo test --package mengdie -- llm::tests` passes;
`rg "StructuredOutputMissing\|StructuredOutputWrapperInvalid" src/core/llm.rs`
returns ≥ 4 hits; `rg "verify claude >= 2.1.138" src/core/llm.rs` returns
≥ 2 hits (one per error variant); `rg "probe_json_schema_flag\|fn probe_"
src/core/llm.rs` returns 0 hits; `rg "CLAUDE_CLI_STRUCTURED_FLAGS"
src/core/llm.rs tests/llm_claude_cli.rs` returns ≥ 3 hits (def + tests).

### AC3: brace-depth scanner deleted; tests handled per merge plan

`extract_first_json_object` is gone. `parse_synthesis_response` body has
no manual byte-scanning — only `serde_json::from_str` + existing field
validation. Test handling matches Step 3 spec exactly.
**Verification**: `rg "extract_first_json_object" src/` returns 0 hits;
`cargo test --package mengdie -- synthesis::tests` passes; `rg
"Plan 019: original test name was" src/core/synthesis.rs` returns 1 hit.

### AC4: rate-limit measurement documented; backup recorded; follow-up BL filed only if needed

`docs/spikes/019-rate-limit-measurement.md` exists with: backup file path,
total tokens, total elapsed, per-cluster average, `parse_errors=0`, and
a one-sentence verdict line.
**Verification**: file exists; `grep -E "backup|total_tokens|elapsed|verdict|
parse_errors" docs/spikes/019-rate-limit-measurement.md` returns ≥ 5
matches; `ls ~/.mengdie/db.sqlite.bak-pre-019-*` lists at least one
timestamped backup file.

### AC5: e2e equivalence on fixtures + production run clean

`tests/fixtures/synthesis-019-wrapper-{success,skip}.json` exist as
hand-crafted minimal envelopes. `tests/synthesis_e2e.rs::wrapper_to_
synthesis_outcome` passes. Production-DB run records zero `parse_errors`.
Manual quality spot-check on 3 most-recent synthesis rows passes (no
empty `entities`, no >80-char titles, no lazy-shape `content`).
**Verification**: `cargo test --package mengdie --test synthesis_e2e`
passes; `grep "parse_errors=0\|parse_errors: 0" docs/spikes/019-rate-
limit-measurement.md` returns 1 hit; manual spot-check verdict recorded
in the same spike doc.

## Decisions not implemented

(none — this plan implements all 5 ACs from BL-027 plus the Pre-Step
documentation. Items reviewers and Doodlestein flagged as out-of-scope
or rejected are listed in the top-level "Out of scope" section above
with rejection rationale.)
