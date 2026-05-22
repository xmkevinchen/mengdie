# .githooks/

Project-local git hooks. Activated by a one-time command per clone:

```bash
git config core.hooksPath .githooks
```

## pre-commit

Runs on every `git commit`. Blocks the commit if either fails:

1. `cargo fmt --all -- --check` — format drift. Fix with `cargo fmt --all`.
2. `cargo clippy --all-targets -- -D warnings` — clippy violations or warnings.

**Does NOT run `cargo test`** — `cargo test` pulls the fastembed ONNX model (~90MB cold download), which would make commits unbearable. Tests are CI's job. See `.forgejo/workflows/ci.yml`.

Warm runtime target: <5 seconds on a clean cached workspace. If the hook becomes slow, check `target/` cache state.

## Bypass policy

Per project `CLAUDE.md`:

> NEVER skip hooks (`--no-verify`) unless the user has explicitly asked for it.

If the hook fails, **fix the underlying issue**, don't bypass. If a clippy lint is a genuine false positive, add `#[allow(clippy::lint_name)]` with an inline comment explaining why — and flag it in the commit message for review. `#[allow]` is LAST RESORT, not the default escape hatch.

## Why not a pre-commit framework?

We deliberately don't use husky / pre-commit.com / lefthook. A bash script + one git config line has:
- zero dependencies
- no version-bump maintenance
- nothing to `npm install` or `brew install`
- visible + reviewable source

If the hook ever grows beyond ~50 lines of shell, reconsider.
