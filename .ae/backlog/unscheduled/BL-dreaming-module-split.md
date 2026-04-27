---
id: BL-dreaming-module-split
status: open
origin: BL-007 /ae:review (architecture-reviewer)
created: 2026-04-18
---

# Split `src/core/dreaming.rs` before BL-008 lands

## Finding

`src/core/dreaming.rs` is currently 641 lines and contains two distinct
concerns:

- **Scoring / promotion** (sync, `impl Db` method) — existing
  `run_dreaming_with_config`.
- **LLM orchestration** (async free function) — new `run_synthesis_pass`.

Plan 010 Step 3 acknowledged this threshold explicitly:

> "or new `src/core/dream_pipeline.rs` if dreaming.rs grows too large —
> judgment call during execution, prefer keeping it in dreaming.rs unless
> it exceeds ~400 lines"

We exceeded that threshold. The split was deferred to keep BL-007's
diff focused.

BL-008 (power-law decay) will add a third concern — a decay pass that
shares the `dream` CLI command but is independent of both promotion and
synthesis. That's the right moment to refactor.

## Trigger

Fires when BL-008 plan lands. The first commit of BL-008 should
preferentially:

1. Move `run_synthesis_pass`, `SynthesisResult`, and the stub-provider
   test module into a new `src/core/synthesis_pipeline.rs`.
2. Keep `run_dreaming_with_config` + `DreamingConfig` + `DreamingResult`
   in `src/core/dreaming.rs` (the original scoring subsystem).
3. Add BL-008's decay pass in a new `src/core/decay_pipeline.rs` rather
   than a third tenant in `dreaming.rs`.

## Scope

- Pure refactor: no behavior change, no schema change.
- `cmd_dream` in `src/bin/cli.rs` imports from all three modules and
  orchestrates them sequentially.
- `cargo test` and `cargo clippy` stay clean.

Estimated effort: one small commit (< 100 lines of imports + module
moves), no new tests needed.
