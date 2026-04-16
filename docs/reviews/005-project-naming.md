---
id: "005"
title: "Review: Human-Readable Project Naming"
type: review
created: 2026-04-16
target: "docs/plans/005-project-naming.md"
verdict: pass
---

# Review: Human-Readable Project Naming

## Summary

Plan 005 implements human-readable project naming via `.mengdie.toml` config files and a `mengdie rename` CLI command. 4 steps, all complete. Review found 2 P1 issues (transaction atomicity, TOML comment parsing), both fixed and squashed into original commits.

## Team Composition

| Agent | Role | Backend |
|-------|------|---------|
| TL | Synthesizer | Claude |
| code-reviewer | General code review | Claude |
| challenger | Opposition / blind spots | Claude |
| codex-proxy | Edge cases / robustness | Codex |

## Findings

### P1 — Fixed

1. **Transaction atomicity** (db.rs `rename_project`) — DELETE + UPDATE not wrapped in transaction. Crash between DELETE and UPDATE would leave DB half-migrated. **Fixed**: wrapped in `conn.transaction()` / `tx.commit()`.

2. **TOML inline comments** (project.rs `read_project_name`) — `trim_matches('"')` corrupted values when inline comments present (`name = "value" # comment` → `value" # comment`). **Fixed**: proper quoted-string extraction (find matching close-quote).

### P2 — Fixed

3. **Silent error suppression** (db.rs collision query) — `.filter_map(|r| r.ok())` hid row errors. **Fixed**: `.collect::<Result<Vec<_>, _>>()?` propagates errors.

4. **Underflow in dry_run** (db.rs) — `(total - collision_count) as usize` could underflow on corrupt data. **Fixed**: `.max(0)`.

### P2 — Backlog

5. **Hand-rolled TOML parser limitations** — doesn't handle escaped quotes, multiline strings, `project.name = "x"` dotted-key syntax. Sufficient for MVP (one field). Backlog: consider `toml` crate if more fields added.

6. **Collision merge discards old recall stats** — intentional design (target project wins). Should be documented in code comment.

### False Positive (Rejected)

- Code-reviewer flagged `strip_prefix("name")` as matching `name_alt`, `rename`. Verified false: `rename` doesn't start with `name`; `name_alt` → `_alt` → `strip_prefix('=')` returns None. The `=` check catches all false-prefix cases.

## Disagreement Value Assessment

Code-reviewer vs Challenger on TOML parser word boundary: Challenger's deeper analysis was correct. The `=` gate after `strip_prefix` prevents false matches. Code-reviewer's recommendation to add explicit word-boundary checking was unnecessary.

## Outcome Statistics

- Steps completed: 4/4
- Rework rate: 2/4 steps needed fixup (50% — steps 1 and 2)
- P1 escape rate: 2 (transaction + TOML comment — not caught by ae:work pre-commit)
- Drift events: 0
- Fix loop triggers: 0
- Auto-pass rate: 4/4 (100%)
- Deferred findings: 0
- Tests: 101 passed, 1 ignored

## Commits

| Commit | Step | Description |
|--------|------|-------------|
| e6d509b | 1 | .mengdie.toml support + 11 tests (includes comment fix) |
| 2edf8da | 2 | rename_project + list_projects (includes transaction fix) |
| 8e3a72f | 3 | rename CLI subcommand |
| 51427c6 | 4 | .mengdie.toml configs + live migration |
| 17cb083 | — | Progress audit cleanup (separate from plan 005) |
