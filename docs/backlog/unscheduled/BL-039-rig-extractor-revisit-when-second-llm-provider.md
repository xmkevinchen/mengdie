---
id: BL-039
title: "Re-evaluate rig::Extractor adoption when mengdie ships its second LLM provider"
status: open
created: 2026-05-10
origin: "plan 019 retrospective — Path B (claude-CLI --json-schema) shipped flat-schema workaround for Anthropic's input_schema subset (no top-level oneOf/allOf/anyOf); per-provider schema-translation is exactly what rig::Extractor encapsulates"
trigger: "concrete code-artifact signal — fires the moment a SECOND match arm is being added to `src/core/llm.rs::build_provider`, OR a SECOND `impl LlmProvider for X` block is being authored in src/core/llm.rs, OR Cargo.toml gains a non-claude HTTP client / SDK dep (async-openai, async-anthropic, etc.). The `build_provider` site carries an inline NOTE pointing at this BL — that comment IS the tripwire."
depends_on: [BL-027]
size: M
v_target: "post-v0.0.1 — when N_providers >= 2"
---

# BL-039 — Re-evaluate rig::Extractor when mengdie's second LLM provider lands

## Context

Plan 019 / BL-027 Path B shipped structured-output via claude-CLI's
`--json-schema` flag with a flat JSON Schema (`{type: object,
required: [skip], properties: {...}}`). The plan originally specified
`oneOf [synthesis-shape, skip-shape]` for structural mutual exclusivity;
Anthropic API rejects `oneOf`/`allOf`/`anyOf` at the top level of tool
`input_schema`, so the design was reduced to flat with a `skip:bool`
discriminator. See `docs/spikes/019-rate-limit-measurement.md` "Schema-
shape post-mortem" for the incident trace.

The flat-schema workaround works for the single-provider case. It does
NOT generalize: each LLM provider mengdie eventually integrates will
have its own JSON-Schema subset rules (OpenAI's `response_format:
json_schema strict:true` accepts full schema including `oneOf`; codex-CLI
is unverified; future HTTP APIs vary). Each new provider would force
either:

(a) A per-provider schema variant in `resources/`, OR
(b) A per-provider schema-fixup function in `src/core/llm.rs`, OR
(c) Adopt rig::Extractor and let its `CompletionModel` impl handle
    per-provider translation natively.

## What rig::Extractor encapsulates

`rig::Extractor<M, T>` is generic over `M: CompletionModel` (provider)
and `T: serde + JsonSchema` (output type). The library:

1. Generates the JSON Schema from `T`'s schemars derivation.
2. Translates the schema into the provider's tool-call format (Anthropic
   tool-use, OpenAI structured outputs, Gemini function-calling, etc.).
3. Handles per-provider quirks (Anthropic's no-top-level-oneOf,
   OpenAI's `strict: true` opt-in, schema-version differences).
4. Parses the provider's response back into `T`.

The original BL-027 spike (2026-05-08) rejected rig because adopting it
required a 150-200 LoC `CompletionModel` impl that wraps subprocess
(claude-CLI). rig's protocol assumes HTTP-API-shaped backends. Net code
delta then was negative (+150-200 LoC adapter to replace 30 LoC
brace-depth scanner).

## Why the calculus changes when N_providers >= 2

The 150-200 LoC adapter cost is paid ONCE. If mengdie ships its second
provider impl, the alternatives are:

- **Without rig**: 2x schema variants OR 2x schema-fixup functions, plus
  per-provider error-mapping (`StructuredOutputMissing` /
  `StructuredOutputWrapperInvalid` are claude-CLI-specific today;
  HTTP-API providers have different shapes).
- **With rig**: 1 type `T` in mengdie's source, rig handles N providers.

Rough breakeven (estimate, validate at trigger time):

| N providers | Without rig (LoC) | With rig (LoC) | rig wins? |
|---|---|---|---|
| 1 (today) | 30 (flat schema + fixup) | 200 (adapter) | no |
| 2 | 80 (2x fixup + error-mapping) | 220 (adapter + 1 model impl) | borderline |
| 3+ | 130+ | 240 | yes |

## Trigger (sharpened 2026-05-10 per plan 019 review)

**Concrete code-artifact tripwires** — fires the moment any of these
shows up in a PR / commit / WIP branch:

1. **`src/core/llm.rs::build_provider` gains a second `match` arm**
   beyond the current `"claude-cli"` arm. The inline NOTE on
   `build_provider` (added by plan 019 review) points operators at this
   BL exactly at that junction; the comment IS the operational
   tripwire — no external review cadence needed.
2. **A second `impl LlmProvider for X` block** anywhere in `src/core/llm.rs`
   beyond the existing `impl LlmProvider for ClaudeCliProvider`. Grep
   target: `rg "^impl LlmProvider for" src/core/llm.rs | wc -l > 1`.
3. **Cargo.toml gains a non-claude LLM SDK / HTTP client dep** —
   `async-openai`, `async-anthropic`, `genai`, `rig-core`,
   `openai-api-rs`, etc. Grep target on `Cargo.toml` `[dependencies]`.

Common landing scenarios (still informational, just no longer the
trigger itself):

- **codex-CLI as primary or fallback** — different `--output-schema`
  subset (raw JSON output vs claude's wrapper); BL-027 marks this
  "Optional, not in v0.0.1 scope".
- **async-openai HTTP** — would also unlock non-claude budgets and
  bypass the `--bare` + `ANTHROPIC_API_KEY` migration risk.
- **Local oMLX endpoint** — already used elsewhere in this project;
  could become a fallback provider for mengdie's synthesis when
  rate-limited.

When the trigger fires, evaluate:

1. Run a 1-day spike: implement a minimal `CompletionModel` for
   `ClaudeCliProvider` that wraps the existing subprocess code.
2. Compare lines-of-code delta vs writing the second provider's schema-
   fixup directly.
3. Compare test surface (rig has its own test infrastructure for
   provider quirks).
4. Decide: adopt rig OR stay with per-provider fixup.

## Why this is a BL not a fix-now

Today, with one provider, the simpler path (flat schema + fixup) wins.
Adopting rig pre-emptively for a hypothetical second provider violates
Karpathy "no flexibility that wasn't requested" + v0.0.1's "avoid
re-inventing wheels" interpreted as "avoid pre-investing in
infrastructure that doesn't have a current customer".

The BL exists so that when the trigger fires, the team doesn't
re-discover the failed Path B + flat-schema-as-tax pattern from scratch.

## Reversibility

HIGH. Adopting rig later is straightforward:
- Add `rig-core` to Cargo.toml
- Implement `CompletionModel` for `ClaudeCliProvider` (~150-200 LoC)
- Refactor `complete_structured` callers to use `rig::Extractor<M, T>`
- Delete `resources/synthesis-output-schema.json` + flat-schema fixup
- Delete `extract_structured_output` helper

Rejecting rig at trigger time is also easy: stay with per-provider
fixup, file the second provider's schema variant, ship.

## Out of scope

- Adopting rig today, before a second provider exists — premature
  abstraction.
- Using rig for non-LLM concerns (RAG, agents, tool-use orchestration)
  — those are different rig modules and would need separate evaluation.

## History

- **2026-05-10**: Filed after plan 019 Step 4 production validation. Path B
  (claude-CLI --json-schema) shipped, but Anthropic's input_schema subset
  forced a flat-schema workaround. Operator: "rig::Extractor 不就是怎么做的
  嘛，大不了不同平台走不同的template就是了" — yes, that's exactly its
  scope; documenting the trigger so future-mengdie doesn't re-discover.
