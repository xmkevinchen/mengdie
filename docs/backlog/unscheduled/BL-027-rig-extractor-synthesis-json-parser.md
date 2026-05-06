---
id: BL-027
title: "rig::Extractor verification spike + conditional adoption — replace synthesis.rs brace-depth JSON parser if subprocess-streaming PASSes"
status: open
created: 2026-05-06
origin: "discussion 026 OSS-survey analysis (CONTINGENT-ADOPT verdict) + 027 conclusion 2026-05-06 caveat (narrow OSS-replacement scope) + /ae:code-review Track 4 strategic finding (sprint candidate needs filed home)"
trigger: "v0.0.1 Phase 1 work — fires immediately upon sprint commitment; the subprocess-streaming spike is the gate inside this BL itself"
depends_on: []
size: S
v_target: "v0.0.1 — Phase 1 mengdie-side OSS swap (paired with BL-026)"
---

# BL-027 — rig::Extractor verification spike + conditional adoption

## Origin

Discussion 026 OSS-survey analysis (`docs/discussions/026-rust-oss-survey/analysis.md` L66-74, "rig — three separable pieces, three verdicts") verdict on Piece 2:

> **Piece 2 — replace synthesis.rs hand-rolled brace-depth JSON parser with `rig::Extractor<SynthesisDraft>`.** Standards-rag proposes this saves ~100 lines. **Challenger: unverified.** rig::Extractor is designed for REST-API-response structured outputs. mengdie's synthesis path uses subprocess-streamed text from `claude -p`. It is unverified that rig::Extractor handles streaming subprocess output. **TL synthesis: requires a concrete spike — write a 50-line proof that rig::Extractor parses claude-CLI subprocess output correctly. If yes, adopt for synthesis.rs only. If no, keep brace-depth parser.**

Pieces 1 (CompletionModel trait wrap) and 3 (Agent dynamic_context for reflection) of rig are SKIP per 026 analysis; this BL covers Piece 2 only.

## Acceptance criteria (sequenced)

### Step 1 — Subprocess-streaming spike (~50 LoC, embedded gate)

- [ ] Fresh workspace: `cargo new --lib bl027-spike && cd bl027-spike`
- [ ] `cargo add rig-core` (current latest, MIT)
- [ ] `cargo add tokio --features full`
- [ ] Define a `SynthesisDraft` struct mirroring mengdie's current `synthesis.rs::SynthesisDraft` (title / content / supersedes / valid_from / etc.) deriving `serde::Deserialize + JsonSchema`
- [ ] Spawn `claude -p "<test prompt that produces a SynthesisDraft JSON>"` as a tokio subprocess; capture stdout as a stream
- [ ] Pipe the stream into `rig::Extractor<SynthesisDraft>` and observe whether it (a) parses correctly, (b) handles partial JSON across stream chunks, (c) produces a sensible error when malformed
- [ ] Document outcome at `docs/spikes/rig-extractor-subprocess.md` (PASS / PASS_WITH_BUFFERING / FAIL)

### Step 2 — Adoption (PASS only; ~50 LoC src change)

- [ ] Add `rig-core` to mengdie root `Cargo.toml`
- [ ] In `src/core/synthesis.rs`, replace the brace-depth JSON parser (~100 LoC) with `rig::Extractor<SynthesisDraft>` — net ~50 LoC removed
- [ ] Adapt `src/core/llm.rs::ClaudeCliProvider` if needed so its subprocess stdout stream feeds Extractor cleanly
- [ ] All existing synthesis tests pass with new parser
- [ ] Run a real `mengdie dream --synthesize` end-to-end; verify produced syntheses are byte-identical to brace-depth-parser output for the same input cluster

### Step 2 alternative — Adoption (PASS_WITH_BUFFERING)

If `rig::Extractor` requires buffering the entire stdout before parsing (rather than streaming), evaluate: does this add unacceptable latency on long syntheses? If yes, treat as FAIL. If no, adopt with a buffering wrapper.

### Step 2 alternative — Adoption (FAIL)

If `rig::Extractor` does not parse `claude -p` subprocess output correctly, keep the brace-depth JSON parser in `synthesis.rs`. Document failure mode in `docs/spikes/rig-extractor-subprocess.md`. Reopen only if a future rig version adds first-class subprocess-stream support OR mengdie's synthesis path moves to HTTP-API LLM providers (in which case rig::Extractor's REST-API-friendly design becomes natural fit).

## Trigger

Fires immediately upon v0.0.1 sprint commitment (Phase 1 mengdie-side OSS swap). Independent of BL-026 — both can land in the same sprint regardless of which one's spike returns first.

## Reversibility

**HIGH**. The brace-depth JSON parser stays in git history. If `rig::Extractor` adoption ships and later proves problematic (e.g., rig pre-1.0 breaking changes per minor version, parser regression on edge-case synthesis output), revert the synthesis.rs change + remove the rig dep. No data migration — synthesis rows are stored independently of which parser produced them.