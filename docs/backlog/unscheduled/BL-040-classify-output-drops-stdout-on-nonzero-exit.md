---
id: BL-040
title: "classify_output drops stdout on non-zero exit — operability gap surfaced during plan 019 debugging"
status: open
created: 2026-05-10
origin: "plan 019 final review — convergent finding from security-reviewer, performance-reviewer, gemini-proxy (3/6 reviewers); root cause of multi-hour Step 4 debugging detour"
trigger: "ready to pick up — concrete, scoped, no external blockers; v0.0.1 acceptable as-is so this is post-v0.0.1 cleanup"
depends_on: []
size: S
v_target: "post-v0.0.1 polish"
---

# BL-040 — classify_output discards subprocess stdout on non-zero exit

## Problem

`src/core/llm.rs::classify_output` returns `Err(LlmError::NonZeroExit
{ code, stderr, kind })` when the subprocess exits with a non-zero
code, and **drops stdout entirely** in the process. Plan 019 Step 4's
"5/5 LLM-call errors" debugging session burned multiple hours because:

1. claude-CLI's `--output-format json` emits `is_error: true` wrappers
   on stdout WITH exit code 1 (we discovered this empirically during
   Step 4 — see `/tmp/claude-probe-stdout.json` from 2026-05-09).
2. mengdie's `classify_output` saw non-zero exit and mapped to
   `LlmError::NonZeroExit { stderr: "" }` — stderr WAS empty;
   the actual diagnostic ("API Error: 400 ...") was on STDOUT, lost.
3. Operators see `error=CLI exited with code 1 (kind: Other); stderr:`
   in tracing output — empty stderr makes the failure look like a
   black hole.

The fix during Step 4 was to run claude-CLI directly via a one-shot
shell command outside mengdie's pipeline. That recovered the buried
stdout and pointed at the actual constraint (no top-level `oneOf`).

## Why this is a BL, not a fix-now

- **v0.0.1 single-operator acceptable**: the operator now knows the
  workaround (run claude-CLI directly with the same argv to see real
  stdout). Documented in `docs/spikes/019-rate-limit-measurement.md`.
- **`--output-format json` is structured-output specific**: pre-plan-019
  text-output path didn't hit this because text-output errors typically
  DO go to stderr. The bug is silently latent in `classify_output`
  but surfaces only on the structured path.
- **Architectural blast radius**: changing `classify_output`'s return
  shape (currently `Result<String, LlmError>`) to carry both stdout
  and stderr in the error case touches every error-handling site.
  Worth doing right, not patching.

## Proposed fix

Two options for the design discussion when this BL is picked up:

### Option A: Always capture both on non-zero exit

Change `LlmError::NonZeroExit` to carry both fields:

```rust
NonZeroExit {
    code: i32,
    stdout: String,   // NEW — was discarded
    stderr: String,
    kind: ExitKind,
}
```

Error `Display` format: include both in the message (stdout first,
then stderr; truncate each to ~500 chars to avoid log spam).

Migration: every match arm on `NonZeroExit` needs the new field.
Existing `parse_wrapper_is_error_true_maps_to_non_zero_exit` test
already constructs `NonZeroExit { code: 0, stderr: env.result, ... }`
in `extract_structured_output` — would gain a `stdout: "(via wrapper)"`
or similar.

### Option B: New variant for structured-path errors

Keep `NonZeroExit` as-is for text-output failures. Add:

```rust
StructuredCallFailedWithOutput {
    code: i32,
    stdout: String,
    stderr: String,
}
```

Trigger: `complete_structured_impl` post-`classify_output` — if
non-zero exit AND stdout is non-empty, surface this variant instead.
Surgical, no migration cost.

## Trigger

Ready to pick up. The Step 4 debugging detour proves the failure mode
is real but operator now has the manual workaround documented.
Schedule into a polish sprint when convenient.

## Out of scope

- Capturing stdout/stderr on the WRITER side (BrokenPipe path) —
  that's already handled by the `BrokenPipe` variant having its own
  error precedence.
- Adding stdout to the `Timeout` variant — when timeout fires the
  child was killed; stdout buffer may be incomplete and misleading.
