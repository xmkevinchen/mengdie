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
