# Plan 019 — Milestone Notes

DEFERRED [Step 4]: Verify claude-CLI accepts `$schema` and `$comment` JSON-Schema-spec metadata fields in `resources/synthesis-output-schema.json`.
Reason: BL-027 verification used a schema without these fields; claude-CLI's `--json-schema` validator may reject them. If Step 4 e2e exposes rejection, strip both fields from the JSON file (high reversibility — single-file edit). If Step 4 passes, leave fields as documentation value.
