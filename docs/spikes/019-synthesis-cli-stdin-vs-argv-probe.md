# Spike 019 — synthesis CLI: stdin vs argv prompt mechanism

**Status**: resolved — Option A confirmed
**Decided**: 2026-05-10
**Plan**: `docs/plans/019-synthesis-cli-json-schema.md`

## Question

When invoking `claude -p --json-schema <schema> --output-format json`, where
should the user prompt be passed?

- **Option A**: positional argv argument, stdin closed (`< /dev/null`)
- **Option B**: piped on stdin, argv positional empty

The decision matters because today's `ClaudeCliProvider::build_command` pipes
the user prompt on stdin to keep it off `ps aux`. Only the `--system-prompt`
value is on argv. Switching to Option A would put the user prompt on argv as
well — same privacy class, but worth verifying we don't escalate.

## Decision: Option A

Smoke-test evidence is recorded in BL-027's verification block (`docs/backlog/
unscheduled/BL-027-rig-extractor-synthesis-json-parser.md` lines 41–60,
captured 2026-05-09 in `/tmp/headless-verify/`):

```
$ SCHEMA='{"type":"object","properties":{"name":{"type":"string"},
           "age":{"type":"integer"}},"required":["name","age"]}'
$ claude -p --json-schema "$SCHEMA" --output-format json \
    "Give me a fictional person Alice age 30." < /dev/null

stdout (excerpt):
  {"type":"result","subtype":"success","is_error":false,
   "result":"已返回:Alice,30 岁。",
   "structured_output":{"name":"Alice","age":30},
   "duration_ms":3961, "total_cost_usd":0.077602, ...}
```

Prompt was passed as the last positional argv argument, stdin was closed via
`< /dev/null`, and the wrapper returned `is_error: false` with a populated
`structured_output`. **Option A works.**

Option B (stdin path under `--json-schema`) was not separately verified and
is not needed — Option A's evidence is sufficient and matches BL-027's
chosen invocation pattern.

## Privacy posture

**Same class** as the existing `--system-prompt` argv exposure (see
`src/core/llm.rs:5-10` module-doc comment). Both `--system-prompt` value
and the new positional prompt are visible to `ps aux` for the duration of
the subprocess; this is acceptable for a single-user personal tool.
**Not** a class escalation.

If mengdie ever runs multi-tenant, both `--system-prompt` and the
positional prompt would need to migrate to `--system-prompt-file` and a
prompt-file analog respectively. Out of scope for v0.0.1 (see plan 019
"Out of scope" section).

## Consumed by

- Step 2 of plan 019: `build_structured_command` argv shape uses Option A
  (positional argv prompt + `stdin(Stdio::null())`).
- Pre-Step checkbox in plan 019 is satisfied by this document.
