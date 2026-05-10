# Plan 019 — Step Summaries

## Step 1 — Schema design + SYSTEM_PROMPT update (commit: 0b9bd76)
**Decisions**:
- Schema lives in `resources/synthesis-output-schema.json` + `include_str!`, not inline string literal (operator-steered during execution; same const-&str contract preserved at compile time, but file is editor-highlightable + jq-able + escape-noise-free).
- `skip.reason.minLength: 20` adopted as anti-lazy-skip structural lever (codex-proxy plan-review finding).
- Schema test asserts structural shape (`oneOf` array of 2 object-shaped entries with non-empty `required` + `additionalProperties: false`) instead of bare "is valid JSON" — catches schema-authoring typos without pulling `jsonschema` dev-dep (Doodlestein-adversarial finding #2 mitigation).

**Rejected**:
- Inline string literal for the schema constant (operator pushback: "拼字符串很低级"; rationale stuck — `include_str!` is strictly cleaner with zero runtime cost).
- `jsonschema` dev-dep for metaschema validation (transitive cost ~30-50 crates including reqwest/rustls/aws-lc-rs; Step 4's e2e fixture pair is the runtime guard).

**Cross-step deps**:
- `SYNTHESIS_OUTPUT_SCHEMA` (`pub(crate)` const) is consumed by Step 3 when `dreaming.rs` switches the LLM call from `complete` to `complete_structured(... SYNTHESIS_OUTPUT_SCHEMA)`. The `#[allow(dead_code)]` attribute on the const is removed in Step 3.
- `EXPECTED_SYSTEM_PROMPT` test constant tracks `SYSTEM_PROMPT` body — both updated together; future prompt tweaks must update both.

**Actual files**: src/core/synthesis.rs, resources/synthesis-output-schema.json, docs/plans/019-synthesis-cli-json-schema.md, docs/milestones/019/notes.md

## Step 2 — ClaudeCliProvider complete_structured (commit: fae11f8)
**Decisions**:
- Sibling method `complete_structured` on `LlmProvider` trait (not `Option<&str>` parameter on existing `complete`). Default impl returns `UnknownProvider` so the 4 in-tree mocks (`FixedProvider`/`PanicProvider`/`TimeoutOnFirst`/`ClusterSizeAwareProvider` in dreaming.rs) inherit without modification. Architect-confirmed shape choice.
- `drive_subprocess_no_stdin` as a trimmed parallel of `drive_subprocess` rather than refactoring `drive_subprocess` to take `Option<&[u8]>`. Two siblings cost less than one branchy method.
- `CLAUDE_CLI_STRUCTURED_FLAGS` as a separate constant from `CLAUDE_CLI_FLAGS`. Per Doodlestein-adversarial #1: appending `--json-schema` to the existing constant would break the existing `claude_cli_flags_constant_matches_build_command_argv` drift-guard test.
- No startup version probe. Per Doodlestein-regret + adversarial #3 convergent finding: probe is YAGNI for single-operator Pro-plan auto-update environment AND OnceLock cache creates a stale-state foot-gun in long-running mengdie-mcp daemon. Error-message diagnostic hint replaces the probe — surfaces only on real failure, no stale state.
- `WrapperEnvelope` with `serde(default)` on each field tolerates upstream wrapper changes (claude-CLI's wrapper has many fields mengdie ignores: session_id, uuid, duration_ms, total_cost_usd, etc.).

**Rejected**:
- `build_structured_command_sets_stdin_to_null` test (plan-listed but dropped at execution): tokio::process::Command doesn't expose stdin config back via getter. Behavior verified by spawn-and-observe in integration tests, not at build time.
- `Option<&str>` schema parameter on existing `complete` (architect plan-review finding: leaky abstraction, mixes capability concerns under one method).
- `LlmError::CapabilityNotSupported` new variant (gold-plating; reuse `UnknownProvider` with code comment per architect CONSIDER 4 + Karpathy "no flexibility that wasn't requested").

**Cross-step deps**:
- `ClaudeCliProvider::complete_structured` ready for Step 3's `dreaming.rs` rewire.
- `SYNTHESIS_OUTPUT_SCHEMA` (from Step 1) is still `#[allow(dead_code)]` — Step 3 removes the allow when dreaming.rs references it.
- `LlmError::StructuredOutputMissing` and `StructuredOutputWrapperInvalid` are new error surfaces dreaming.rs must classify (currently dreaming.rs has `result.llm_call_errors += 1` for any `Err(_)` — Step 3 keeps this catch-all behavior; specific classification is post-v0.0.1 work).

**Actual files**: src/core/llm.rs, tests/llm_claude_cli.rs, docs/plans/019-synthesis-cli-json-schema.md

## Step 3 — Delete brace-depth scanner; rewire to complete_structured (commit: 30a516c)
**Decisions**:
- `parse_synthesis_response` body collapses to a single `serde_json::from_str` against `RawEnvelope`. The schema-validated JSON object handed in by `extract_structured_output` is contractually clean — no preamble/postamble/fence tolerance needed.
- 4 in-tree mock providers (`FixedProvider`/`PanicProvider`/`TimeoutOnFirst`/`ClusterSizeAwareProvider`) override `complete_structured` to delegate to their existing `complete` impl (returning the same canned payload, ignoring `_schema`). Trait default impl returns `UnknownProvider` — would have made all 7 dreaming tests fail without the override.
- Audit trail preserved via `// Plan 019: original test name was parser_skip_with_llm_preamble_still_parses; preamble case can no longer arise under --json-schema mode.` inline comment. Architect MUST FIX 2 mitigation.

**Rejected**:
- Keeping `parser_malformed_json` as defense-in-depth (code-reviewer MUST FIX 2 confirmed dead — claude-CLI rejects malformed inner JSON at schema-validation level before mengdie sees it; defending against an unreachable path is dead code masquerading as live coverage).
- `#[ignore]` on the repurposed test (architect-suggested alternative). Chose rename + audit-trail-comment instead because the test still verifies real behavior under the new contract; ignoring would lose runtime coverage.

**Cross-step deps**:
- `SYNTHESIS_OUTPUT_SCHEMA` now wired up at runtime (via `dreaming.rs` import). `#[allow(dead_code)]` on the const removed (clippy would flag it as superfluous now).
- `LlmError::StructuredOutputMissing` and `StructuredOutputWrapperInvalid` (Step 2) flow through `result.llm_call_errors += 1` in dreaming pass — same catch-all error treatment as pre-019. Specific classification is post-v0.0.1 work.
- Step 4's e2e test will exercise the full chain `extract_structured_output` → `parse_synthesis_response` against fixture wrappers.

**Actual files**: src/core/synthesis.rs, src/core/dreaming.rs, docs/plans/019-synthesis-cli-json-schema.md

## Step 4 — Validation: fixtures + e2e + production run (commit: <pending>)
**Decisions**:
- Flat schema with `skip:bool` discriminator replaces the originally-planned `oneOf` design. Anthropic API rejects `oneOf`/`allOf`/`anyOf` at top level of tool `input_schema` ("API Error: 400 ... does not support oneOf, allOf, or anyOf at the top level" — verified 2026-05-10 in `/tmp/claude-probe-stdout.json`). The structural "exactly one of two shapes" guarantee is lost; `parse_synthesis_response`'s runtime validation (`MissingField`, `EmptyTitle`, `EmptyContent`) covers the semantic layer.
- `$schema` and `$comment` JSON-Schema-draft-07 metadata fields dropped from `resources/synthesis-output-schema.json`. Same Anthropic input_schema subset constraint; resolves the Step 1 deferred risk in one edit.
- Fixtures + e2e tests retained — they test `parse_synthesis_response` against the structured-output payload shape, which is invariant across the schema-design change (the parser's RawEnvelope already had all fields as `Option<>`).
- `mengdie::core::llm::classify_output` discards stdout on non-zero exit; the actual claude-CLI diagnostic (`is_error: true` + `result` field) was buried until we ran a probe directly. Filed as future-improvement risk; not changed in this plan (out of v0.0.1 scope).
- Schema test renamed `schema_const_parses_as_oneof_with_two_object_branches` → `schema_const_is_flat_object_with_skip_discriminator`. New body asserts top-level `type:"object"`, `required:["skip"]`, all 5 properties present, NO top-level `oneOf`/`allOf`/`anyOf`.

**Rejected**:
- Original `oneOf [synthesis-shape, skip-shape]` design — rejected by Anthropic API. The 9-reviewer plan-review panel didn't catch this because no reviewer probed the actual API; codex-proxy specifically endorsed `oneOf` as the right token-decode-constrained shape, which is true for OpenAI's `response_format: json_schema strict:true` but NOT for Anthropic's tool input_schema subset.
- Nested `oneOf` inside an `outcome` property (the user explicitly chose flat over nested when the constraint was discovered).
- Reverting the entire plan (operator's call: keep structured-output, adopt flat schema, ship).

**Cross-step deps**:
- Production DB at `~/.mengdie/db.sqlite` now contains 5 new synthesis rows from this run (commit: <pending>). The pre-run backup at `~/.mengdie/db.sqlite.bak-pre-019-1778428737` is the rollback path if any row regrets.
- BL-027 should be marked `closed/done` after this commit lands. Path B implementation succeeded; rate-limit relief BL not needed (operator's KB scale comfortably under daily budget).

**Production-run measurements** (5 syntheses from 5 clusters):
- Total elapsed: ~160 sec / Total cost (USD-equivalent): ~$0.40 / Total tokens: ~275K
- Per-cluster: 24-39 sec, $0.068-$0.089
- 0 LLM-call errors, 0 parse_errors. Quality: all 5 rows have 1000-1500 char substantial content, multi-tag entities, no lazy fallthroughs.

**Actual files**: resources/synthesis-output-schema.json, src/core/synthesis.rs, tests/synthesis_e2e.rs (no change), tests/fixtures/* (no change), docs/spikes/019-rate-limit-measurement.md, docs/milestones/019/notes.md, docs/milestones/019/step-summaries.md, docs/plans/019-synthesis-cli-json-schema.md



