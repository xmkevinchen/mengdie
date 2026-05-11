---
id: BL-030
title: "Bind specs to code — `as_of_commit` drift check"
status: open
created: 2026-05-08
origin: F-004 council review (codex-proxy)
trigger: "Any signature change in src/core/mcp_tools.rs / search.rs / ingest.rs / src/bin/cli.rs"
size: S
v_target: v0.0.1
---

# BL-030 — Bind specs to code

## Origin

F-004 Step 3 produced 4 engineering specs in `docs/specs/`, each
carrying frontmatter `as_of_commit: 1b48c92` (later autosquashed to
`ebb1602` etc. per F-004 review fixup). These specs document API
signatures that **must match** `src/core/mcp_tools.rs` / `search.rs` /
`ingest.rs` / `src/bin/cli.rs` current state.

The single-source-of-truth (SoT) rule per `docs/blueprint.md §14`:
> "Code wins for current signature; specs reflect code (not vice versa)."

But there is **no automated check** binding the SoT rule. If
`src/core/mcp_tools.rs` signature changes, specs silently drift until
operator manually notices.

## Problem

Spec drift undermines the Layer 4 contract guarantee. AE plugin and
future MCP hosts read `#[tool(description=...)]` strings at
tool-discovery time; if specs claim a different signature than code
exposes, agents make wrong calls.

## Proposed scope

Two layers of binding (cheap to expensive):

### Layer 1 (cheap — file)

Pre-commit hook addition: when any of the 4 source files changes,
warn if any `docs/specs/*.md` `as_of_commit:` is older than the
last src change:

```bash
# .githooks/pre-commit (extend existing)
SRC_CHANGED=$(git diff --cached --name-only | grep -E '^src/(core/(mcp_tools|search|ingest)\.rs|bin/cli\.rs)$')
if [ -n "$SRC_CHANGED" ]; then
  for spec in docs/specs/*.md; do
    AS_OF=$(grep '^as_of_commit:' "$spec" | awk '{print $2}')
    if ! git merge-base --is-ancestor "$AS_OF" HEAD; then
      echo "[WARN] spec $spec as_of_commit=$AS_OF; src changed but spec not updated"
    fi
  done
fi
```

### Layer 2 (more — generate)

CI check: parse `#[tool(description=...)]` strings + Rust signatures
from src/core/mcp_tools.rs, generate JSON snapshot, diff against
spec's documented signature. Failure = signal mismatch.

(Layer 2 is more work; Layer 1 is sufficient as MVP.)

## Trigger

**Fires when**: any of the 4 source files (`src/core/mcp_tools.rs`,
`src/core/search.rs`, `src/core/ingest.rs`, `src/bin/cli.rs`) gets a
signature change that affects the public surface (params / returns /
error types).

Mechanical detection: `git diff --stat` on these files between two
commits. If diff includes `pub fn`, `pub struct`, `pub enum` lines →
fire.

## Reversibility

High. Layer 1 is a hook addition (can be removed). Layer 2 is a CI
job (can be disabled).

## Why deferred (not in F-004)

F-004 ships specs at `as_of_commit: <F-004 final commit>`; v0.0.1
hasn't started signature changes yet. Drift risk = 0 until first
src change. Trigger is the first signature change post-F-004, which
will likely happen during BL-026 sqlite-vec integration or BL-027
rig::Extractor integration.
