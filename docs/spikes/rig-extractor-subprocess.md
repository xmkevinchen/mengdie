---
id: "rig-extractor-subprocess"
type: spike
status: accepted
date: 2026-05-08
spike_for: BL-027
outcome: FAIL
relates_to:
  - docs/discussions/026-rust-oss-survey/analysis.md  # original CONTINGENT-ADOPT verdict
  - docs/backlog/unscheduled/BL-027-rig-extractor-synthesis-json-parser.md
  - src/core/synthesis.rs  # current brace-depth parser stays
  - src/core/llm.rs  # ClaudeCliProvider (BL-005 shipped)
environment:
  rust: "1.95.0"
  rig_core_version: "0.36.0"
  macos: "26.4 (Tahoe; Darwin 25.4.0)"
  arch: "arm64"
---

# Spike: rig::Extractor + claude-CLI subprocess streaming — FAIL (structural)

This spike is **BL-027 Step 1** (the embedded subprocess-streaming gate).
Outcome: **FAIL — architectural API mismatch**. Keep current brace-depth
JSON parser in `src/core/synthesis.rs`.

## Method

Reviewed `rig-core 0.36.0` source at
`~/.cargo/registry/src/index.crates.io-*/rig-core-0.36.0/src/extractor.rs`.

Key findings from API inspection (no code spike needed; the public API
surface alone makes the verdict deterministic):

```rust
pub struct Extractor<M, T>
where
    M: CompletionModel,                                        // (1)
    T: JsonSchema + for<'a> Deserialize<'a> + WasmCompatSend + WasmCompatSync,
{
    agent: Agent<M>,
    _t: PhantomData<T>,
    retries: u64,
}

impl<M, T> Extractor<M, T> { ... }

pub async fn extract(
    &self,
    text: impl Into<Message> + WasmCompatSend,                 // (2)
) -> Result<T, ExtractionError> { ... }
```

(1) `Extractor` is generic over `M: CompletionModel`. There is no
public path to instantiate `Extractor` without a `CompletionModel`
implementation.

(2) `extract(text)` takes the **prompt** to send to the LLM (wrapped as
`Message`), not arbitrary text to parse. Internally it calls
`Agent.completion()` → `CompletionModel.completion()`, registers a
"submit" tool with the JsonSchema derived from `T`, and deserializes
the LLM's tool-call response into `T`.

## Why this fails the BL-027 mengdie use case

mengdie's synthesis flow is:

1. Build prompt (`build_synthesis_prompt`)
2. Subprocess: `claude -p "<prompt>" --output-format text` via `tokio::process::Command`
3. Capture stdout → string
4. Parse with brace-depth scanner (`extract_first_json_object`) +
   `serde_json::from_str` (~100 LoC)

The string mengdie has at step 4 is the **LLM output text** containing
JSON (possibly with preamble/postamble noise), not a prompt for another
LLM call. mengdie wants to PARSE existing text, not CALL an LLM.

`rig::Extractor.extract(prompt)` takes a prompt and CALLS an LLM. To
use Extractor for synthesis parse, mengdie would need to either:

### Option A — Wrap claude-CLI as a `CompletionModel`

- Implement `CompletionModel` trait for `ClaudeCliProvider`
- Translate rig's tool-call protocol → claude CLI subprocess
  interaction (claude CLI text-mode does not have native tool-call
  output; would need to fake it by parsing LLM text response into a
  synthetic `ToolCall` envelope that rig's Agent expects)
- THEN replace `synthesis.rs::parse_synthesis_response` with
  `extractor.extract(prompt).await`

Estimated cost: 150-200 LoC for CompletionModel adapter + protocol
translation + async/error mapping. Replace 100-LoC current parser →
net **+50 to +100 LoC** of mengdie-side complexity, plus an external
dependency (rig-core + 233 transitive crates per cargo lock).

This is the **opposite** of "minimum OSS-replacement" (CLAUDE.md FINAL
section thesis). 026 OSS Rust survey explicitly SKIPed rig Piece 1
(CompletionModel) for the same reason.

### Option B — Use rig::json_utils standalone helper

`rig::json_utils` exposes `parse_tool_arguments(text)` and a few
serde-derive utilities, but no public "extract first JSON object from
arbitrary text" helper equivalent to mengdie's `extract_first_json_object`.
rig assumes input text is already pure JSON (tool-call protocol
guarantees this). Mengdie's case (stdout with possible preamble) is not
a rig design point.

This option does not exist as a usable adoption path.

### Option C — Side-step rig entirely

Not adoption. Keep current parser. This is the recommended outcome.

## Verdict: **FAIL**

Per BL-027 Step 1 outcome enum:
- (PASS — adopt Extractor with current ClaudeCliProvider): not possible
  per API mismatch.
- (PASS_WITH_BUFFERING — adopt with buffering wrapper): even buffered,
  the API mismatch holds. Buffering doesn't help when the input format
  isn't supported.
- ✅ **FAIL — keep brace-depth parser in `src/core/synthesis.rs`**.

Reopen trigger: only fires if **mengdie's synthesis path moves to an
HTTP-API LLM provider** (where tool-call protocol is native), OR if a
future rig version adds a standalone "extract<T>(text: &str)" helper
that operates on already-LLM-text (no second call). Neither is in the
v0.0.1 thesis.

## Implications for BL-027 final close

Step 2 (Adoption) is **not run** — outcome FAIL means no src/ change.
BL-027 close-out is just this outcome doc + frontmatter status update
on BL-027 file. No `Cargo.toml` change, no `synthesis.rs` change.

## Implications for v0.0.1 PRD AC2

Per `docs/milestones/v0.0.1.prd.md` AC2 ("BL-027 disposition resolved
— spike runs (PASS or FAIL); ... Disposition recorded, not adoption"):

✅ **AC2 disposition: resolved as FAIL.** Current brace-depth parser
stays. AC2 is satisfied by this spike outcome regardless of FAIL
verdict.

## Related observations

- The brace-depth parser's "tolerate LLM preamble/postamble" capability
  is a load-bearing differentiator vs rig's tool-call assumption. mengdie
  observed this tolerance is used in practice — synthesis tests
  (`src/core/synthesis.rs::tests`) include `parser_tolerates_preamble`,
  `parser_tolerates_postamble`, `parser_skip_with_llm_preamble_still_parses`.
  Replacing with rig would lose this tolerance and require fragile
  prompt-engineering to suppress LLM chattiness.
- The 100-LoC brace-depth parser is **already the minimum** for
  mengdie's use case. Net "OSS replacement" yields negative LoC savings
  for this surface. Karpathy "if you write 200 lines and it could be
  50, rewrite it" applies in reverse: the 100-LoC parser is the right
  size; rig adoption would inflate to 200+.

## Follow-ups

- BL-027 status → `closed` (outcome resolved, no Step 2 work needed).
  Move to `docs/backlog/done/v0.0.1/` after F-004 / v0.0.1.prd.md
  closure flow.
- Spike outcome documented; reopen trigger explicit (above).
- BL-027 final close requires no `BL-011`-style follow-up (no Linux CI
  verification needed since no library was adopted).
