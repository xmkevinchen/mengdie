---
id: BL-027
title: "synthesis.rs JSON parser — adopt LLM-native structured output (claude --json-schema / codex --output-schema)"
status: open
created: 2026-05-06
reopened: 2026-05-09
reopened_reason: "Original 2026-05-08 closure was based on the wrong question. The spike asked 'does rig::Extractor fit mengdie's subprocess-streaming?' (answer: no, structural API mismatch). The right question is 'can the underlying goal — replace 100-LoC brace-depth scanner with structured-output guarantee — be achieved another way?' Answer (verified 2026-05-09): yes, via claude/codex CLI's own --json-schema / --output-schema flags. No new OSS dep needed; subprocess + credential-delegation design preserved; ~30 LoC of brace-depth scanner becomes 0."
origin: "discussion 026 OSS-survey analysis (CONTINGENT-ADOPT verdict on rig::Extractor) + post-spike re-investigation 2026-05-09 (chat-discovered CLI-native flags)"
trigger: "ready to pick up — verification complete, no external blockers"
depends_on: []
size: S
v_target: "v0.0.1 — Phase 1 mengdie-side adoption (no new OSS dep)"
---

# BL-027 — synthesis.rs JSON parser: LLM-native structured output adoption

## Real goal (always was)

Replace `src/core/synthesis.rs::extract_first_json_object` (~30-LoC brace-depth scanner with string-state tracking) with a mechanism that **guarantees the LLM produces schema-validated JSON**, so the scanner becomes unnecessary and the parse path collapses to `serde_json::from_str`.

The 2026-05-08 spike conflated this goal with one specific candidate (rig::Extractor). When that candidate failed, the BL was closed. The goal is unchanged; the path is different.

## Path A — rig::Extractor (REJECTED, kept for audit trail)

Original verdict (2026-05-08, `docs/spikes/rig-extractor-subprocess.md`): **FAIL — structural API mismatch.**

`rig::Extractor<M, T>` is generic over `M: CompletionModel`. To adopt mengdie would have had to implement `CompletionModel` for `ClaudeCliProvider` — translating rig's tool-call protocol to claude-CLI subprocess interaction. Estimated cost: 150-200 LoC adapter to replace 30 LoC of brace-depth scanner. Net code increase, not decrease. Plus 233 transitive crates from rig-core.

This path stays rejected. It would re-open only if mengdie switches to HTTP-API LLM providers (then rig's tool-call assumption fits natively).

## Path B — CLI-native structured output (VERIFIED VIABLE, 2026-05-09)

Both `claude` and `codex` CLI headless modes already expose structured-output flags. The CLI translates them to the underlying provider's tool-call / structured-outputs API — token-level decode constraint applies, validated payload comes back through stdout.

### Verification evidence (2026-05-09)

Smoke tests run in `/tmp/headless-verify/`. Outputs preserved as local-only files; this BL records the conclusion.

**claude (`-p --json-schema --output-format json`):**

```bash
$ SCHEMA='{"type":"object","properties":{"name":{"type":"string"},"age":{"type":"integer"}},"required":["name","age"]}'
$ claude -p --json-schema "$SCHEMA" --output-format json "Give me a fictional person Alice age 30." < /dev/null
```

stdout (excerpt):

```json
{
  "type":"result", "subtype":"success", "is_error":false,
  "result":"已返回:Alice,30 岁。",
  "structured_output":{"name":"Alice","age":30},
  "duration_ms":3961, "total_cost_usd":0.077602, ...
}
```

- Structured payload lives in the **`.structured_output` field** of a wrapper JSON object on stdout.
- `is_error` field reports model-level errors (auth fail, refusal).
- `result` field is the model's natural-language response (separate from the structured payload).
- `total_cost_usd` is reported but is **API-equivalent cost**, not actual subscription billing — under Claude Code subscription, additional spend is $0; rate-limit token budget IS consumed (~40K tokens per call including loaded CLAUDE.md / plugin context).

**codex (`exec --skip-git-repo-check --output-schema FILE`):**

```bash
$ codex exec --skip-git-repo-check --output-schema person_schema.json "Give me a fictional person Carol age 28." < /dev/null
```

stdout:

```json
{"name":"Carol","age":28}
```

- Stdout is the **raw schema-validated JSON, no wrapper**.
- Cleaner integration than claude — `serde_json::from_str(stdout)` directly yields T.
- ~17K tokens per call (lighter than claude default which loads full Claude Code context).

### Boundary finding — schema validation is structural, not semantic

Claude test 3: prompt was "tell me a programming joke", schema was `{name, age}`. Result:

```
result:             "为什么程序员总是分不清万圣节和圣诞节？因为 Oct 31 == Dec 25。"
structured_output:  {"name": "Programming Joke", "age": 31}
is_error:           false
```

Model produced **both** — natural-language joke in `result`, **hallucinated** schema-fitting values in `structured_output`. **No error.** Schema-validated does not mean semantically appropriate.

**Implication for synthesis path**: when the model decides "these memories don't share a meaningful common thread" (current prompt's `{"skip": true, "reason": "..."}` escape hatch), schema-constrained mode would force it to fabricate a synthesis. Mitigation: schema must be `oneOf` covering both `SynthesisDraft` AND `SkipDecision` shapes, giving the model a legal "I refuse" path within the schema.

## Acceptance criteria (sequenced; revised 2026-05-09)

### Step 1 — Schema design (oneOf for synthesize-or-skip)

- [ ] Define a top-level JSON Schema with `oneOf`:
  ```json
  {"oneOf": [
    {"type":"object","properties":{"title":{"type":"string","maxLength":80},"content":{"type":"string"},"entities":{"type":"array","items":{"type":"string"},"minItems":2,"maxItems":6}},"required":["title","content","entities"],"additionalProperties":false},
    {"type":"object","properties":{"skip":{"const":true},"reason":{"type":"string"}},"required":["skip","reason"],"additionalProperties":false}
  ]}
  ```
- [ ] Persist the schema as a const string in `src/core/synthesis.rs` (next to the code that consumes it; avoids file-loading at runtime).
- [ ] Update the synthesis prompt body to reference the schema's two output shapes explicitly (so the natural-language instructions and the schema constraint agree).

### Step 2 — Subprocess invocation update (in `src/core/llm.rs::ClaudeCliProvider`)

- [ ] Modify the `claude -p` invocation to add `--json-schema "$SCHEMA"` + `--output-format json` flags.
- [ ] Close stdin explicitly (`stdin(Stdio::null())` in `tokio::process::Command`).
- [ ] Parse stdout as the wrapper object first; check `is_error` field; extract `.structured_output` if success.
- [ ] Map wrapper's error states to existing `SynthesisError` variants:
  - `is_error: true` → `SynthesisError::LlmError(reason: result_field)`
  - `structured_output` missing → `SynthesisError::NoStructuredOutput`
  - `structured_output` doesn't deserialize into oneOf union → `SynthesisError::SchemaShapeMismatch`
- [ ] Optional (not in v0.0.1 scope): codex-CLI provider variant using `--output-schema`.

### Step 3 — Delete brace-depth scanner

- [ ] Remove `extract_first_json_object` (~30 LoC) from `src/core/synthesis.rs`.
- [ ] Update `parse_synthesis_response` to take the already-validated structured_output payload directly.
- [ ] Run all 8 existing parser tests — they should still pass for the contract, but the noise-tolerance tests (`parser_tolerates_preamble` / `parser_tolerates_postamble` / `parser_markdown_fenced_json_extracts_cleanly`) become obsolete. Decision at impl time: delete them, OR repurpose them to test the new error-mapping path (Step 2).

### Step 4 — Rate-limit measurement

- [ ] Run one full `mengdie dream --synthesize` pass on a representative cluster set.
- [ ] Record total token count (input + cache_creation + cache_read + output) and elapsed time.
- [ ] If subscription rate-limit looks tight (>50% of daily quota for one Dreaming pass), document as a known limit; consider `--bare` + `ANTHROPIC_API_KEY` as fallback option (separate BL — would change credential-delegation design).
- [ ] If rate-limit looks comfortable, no action.

### Step 5 — End-to-end equivalence check

- [ ] Run `mengdie dream --synthesize` on the same input cluster set with both old (current main) and new (Path B) parser path.
- [ ] Diff the produced synthesis rows. Differences are expected (model output is non-deterministic across invocations) but **shape** (field names, types, no error rows where old path succeeded) should match.

## Trigger

Fires when picked into a sprint. No external blockers — verification done, code path clear, ~50 LoC change. Pair-able with the AE Phase 2 sprint (BL-008 / BL-025 / BL-023) or stand alone.

## Reversibility

**HIGH**. `git revert` the adoption commit restores the brace-depth scanner. No schema migration; synthesis rows are not bound to which parser produced them.

## Out of scope (reopen as separate BL if/when needed)

- Switching mengdie to direct Anthropic / OpenAI HTTP API (would unlock rig::Extractor reopen and change credential model).
- Adopting rig for non-synthesis paths (Piece 1 + Piece 3 from 026 analysis stay SKIP).
- Codex-CLI as primary LLM provider (separate provider integration BL).
- Schema migration tooling for evolving the synthesis schema across future versions.

## History

- **2026-05-06**: filed (CONTINGENT-ADOPT pending rig spike outcome).
- **2026-05-08**: closed as `spike outcome FAIL` per `docs/spikes/rig-extractor-subprocess.md`.
- **2026-05-09**: reopened. Original closure was correct for "rig fits or not" but that was the wrong question. Real goal (replace brace-depth scanner with LLM-native structured output guarantee) is achievable via Path B (CLI-native flags `--json-schema` / `--output-schema`). Verification smoke-tested on both claude and codex CLI; both work. BL re-scoped to Path B adoption.
