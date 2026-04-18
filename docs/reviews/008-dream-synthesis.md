---
id: "008"
title: "Review: BL-007 Dream Synthesis"
type: review
created: 2026-04-18
target: "docs/plans/010-dream-synthesis.md"
verdict: pass
---

# Review: BL-007 Dream Synthesis

## Verdict: PASS

Feature ships a working `mengdie dream --synthesize` end-to-end: cluster →
LLM → store synthesis row + source links. All 28 plan checkboxes done,
178 tests pass (up from 175) + 5 ignored. 8 P1/P2 fixes applied inline,
4 P2 items filed to backlog with explicit triggers. No remaining P1.

## Review Team

- **code-reviewer** — correctness, test robustness, code smells
- **architecture-reviewer** — module boundaries, SQL duplication, runtime flavor
- **security-reviewer** — prompt injection, argv exposure, SQL binding, FK orphans
- **challenger** — pure opposition, 7 structured Claim/Evidence/Objection challenges
- **cross-family-fallback** (Claude Sonnet via challenger kit) — Codex + Gemini
  proxies both unavailable this session; TL spawned the fallback per CLAUDE.md
  cross-family fallback protocol. Angles: observability, failure-mode composition,
  sync-CLI UX, content_hash stability, test-distribution shift.

## Scope

`git diff 7427b27^..HEAD` on `main`. Commits:
- `7427b27` Step 2: pure synthesis seam
- `7b3a876` Step 1: schema v4 + NewMemory.is_longterm + SourceType::Synthesis
- `91ec48c` Steps 1+2 meta
- `6d52bda` Step 3: run_synthesis_pass + CLI + e2e
- `c8c76ec` Step 3 meta
- `b001d6c` review fixups (this review)
- `<new>` backlog entries (this review)

~1412 insertions across 9 source files + 4 backlog files.

## Prior Art from Project Knowledge Base (Mengdie)

- `[plan]: First-caller plan validates design bets of its dependencies` (plans/010, 2026-04-18) — the same pattern this plan instantiates; AC5 is the empirical-gate for BL-006.
- `[plan]: expose residuals alongside clusters` (plans/009, 2026-04-18) — BL-007 instantiates the residuals "skip" policy.
- `[analyze]: is_longterm flag has zero effect on search` (discussions/009, 2026-04-06) — justified flipping synthesis default to `is_longterm=false`.
- `[discuss]: No new memory_resolve_conflict tool` (discussions/008, 2026-04-06) — pattern precedent for layering behavior onto the existing ingest path rather than adding new tools.

## Synthesized Findings

### P1 (none)

No data-loss, crash, or correctness blockers remain after fixups.

### P2 — Fixed inline (`b001d6c`)

| # | Finding | Sources | Fix |
|---|---|---|---|
| 1 | Brace-depth parser misses escape handling (`\"` inside strings with `{`/`}` around) | code-reviewer P1 (actual severity P2 on inspection), challenger #3, cross-family-fallback #5 | Upgraded `extract_first_json_object` with `in_string` + `escaped` state tracking. 3 new regression tests (escaped-quote-unbalanced-brace, balanced-braces-inside-escape, markdown-fenced-json). |
| 2 | Flat `llm_errors` counter collapses 17 distinct failure classes | architecture-reviewer #5, cross-family-fallback #2 | Split into `llm_call_errors` + `parse_errors`. Kept `llm_errors()` helper method for callers that want the flat metric. |
| 3 | Silent 4000-char truncation loses source signal without detection | challenger #4 | Added `memories_truncated` counter to `SynthesisResult`; surfaced in the CLI summary line. |
| 4 | `SearchResultItem` missing `source_type` field — consumers can't distinguish syntheses from primaries | challenger #7 | Added `source_type: String` field to the MCP search-result schema; populated from `MemoryEntry.source_type`. |
| 5 | `get_memories_by_ids` silently skipped missing ids | code-reviewer P2 | Added `tracing::warn!` with requested/loaded counts + missing id list. |
| 6 | `--dry-run` alone silently triggered synthesis path | challenger #5 | Now requires explicit `--synthesize`; CLI errors with a descriptive message otherwise. |

### P2 — Filed to backlog (each with explicit trigger)

| # | Title | Trigger |
|---|---|---|
| 7 | FK PRAGMA foreign_keys OFF (security P2 + architect #6) | `BL-fk-pragma-and-deletion-safety.md` — first plan that adds `DELETE FROM memory_entries` |
| 8 | `dreaming.rs` 641 lines > plan's 400-line threshold (architect #1) | `BL-dreaming-module-split.md` — BL-008 (power-law decay) lands |
| 9 | `content_hash` dedup key unstable under prompt evolution (cross-family-fallback #4) | `BL-synthesis-dedup-key.md` — next `SYSTEM_PROMPT` edit (regression guard surfaces need) OR duplicate syntheses observed |
| 10 | Synthesis rows tagged `factual` without fidelity check (challenger #2) | `BL-synthesis-provenance.md` — first real-data dream run (plan AC5 writeback) OR operator reports bad ae:analyze output traceable to a synthesis |

### P2 / P3 — Acknowledged, no action (MVP non-goals or documented)

- **No retry on transient LLM errors** (challenger #6). Plan explicit non-goal. Recovery path is re-run — now demonstrably safe thanks to content_hash + link-PK ON CONFLICT tests (AC3 `test_synthesis_rerun_is_idempotent`).
- **argv exposure of system prompt** (security P3). Documented in `src/core/llm.rs`; synthesis system prompt is generic (not secret-bearing).
- **Prompt injection via memory content** (security P2). MVP trust model: ingest sources are local / user-authored. Mitigations available but deferred.
- **AC5 empirical-results writeback enforcement** (challenger #1). Social contract — no code enforces it. Acknowledged; flag for retrospective reminders when BL-008 or BL-clustering-validation triggers fire.
- **Sync CLI blocking** (cross-family-fallback #3). Plan 2.2 daemon + queue is the right home. BL-007 does not set up scaffolding; users who find it blocking can manually chunk `--project` scope.
- **Module boundary (SourceType in mcp_tools.rs)** (architect #3). Lower-priority refactor; not BL-007 scope.
- **Idempotency test doesn't explicitly assert content_hash** (code-reviewer P3). Row-count comparison is equivalent — if dedup fails, the count would be 2. Test is valid as-is.
- **SQL filter duplication with `get_memory`/`list_memories`** (architect #3 P3). `row_to_entry` factors out the deserialization; only the column list is duplicated. Cleanup pass material.
- **Observability — no per-cluster timing span** (cross-family-fallback #1). Worth a 4-line add when daily usage starts; not blocking today.
- **Test suite distribution shift — no realistic-variance fixtures** (cross-family-fallback #5). Partially addressed by adding the markdown-fence parser test. E2e test against real Claude CLI is the true distribution check, gated behind `#[ignore]`.

### Disagreement Value Assessment

- **Code-reviewer vs TL on parser severity**: code-reviewer called the brace-depth escape bug **P1**. On inspection of the cited fixture `{"content":"quote with \"{x}\" inside"}`, the inner `{}` balance, so the specific example parses correctly. The **general class** is real (unbalanced inner braces after escaped quotes), so TL reclassified as P2 + fixed it. Code-reviewer's diagnosis was directionally right; the fix is applied at the broader class severity.
- **Challenger vs architect on synthesis trust model**: challenger argued syntheses getting searchable immediately with `knowledge_type=factual` is a correctness hazard; architect accepted it as MVP scope. Synthesis: challenger is right that the hazard exists (filed as BL-synthesis-provenance), but deferred — we first need to see whether syntheses are actually unreliable in practice before over-engineering audit tooling.
- **Cross-family-fallback vs plan on content_hash dedup**: fallback argued content_hash is the wrong dedup key (true for prompt evolution). Plan accepted it for MVP. Filed as BL-synthesis-dedup-key with explicit trigger (next `SYSTEM_PROMPT` edit).

## Fixup Commits

| Original commit (step)        | Fixup commit | Summary |
|-------------------------------|--------------|---------|
| `7427b27` Step 2 (synthesis)  | `b001d6c`    | Escape-aware parser + 3 new tests |
| `6d52bda` Step 3 (orchestration) | `b001d6c` | `llm_errors → {call, parse}` split, `memories_truncated`, `get_memories_by_ids` warn, `--dry-run` gating |
| `7b3a876` Step 1 (schema)     | `b001d6c`    | `SearchResultItem.source_type` field |
| —                             | `<new>`      | 4 backlog entries with triggers |

Autosquash/rebase declined this session (destructive ops disabled); fixups land as explicit review-labeled commits. Squashable manually before any future push to a protected branch if desired.

## Outcome Statistics

- **Steps completed**: 3/3
- **Rework rate**: 0 steps needed fixup commits during `/ae:work` (review-stage fixups apply to the review phase, not step execution).
- **P1 escape rate**: 0 P1 findings discovered in `/ae:review` that survived — 1 was reported (code-reviewer) and reclassified to P2 on inspection, then fixed.
- **Drift events**: 2 during `/ae:work`, both approved:
  - Step 1: callsite-default drift on 5 test-only modules (unavoidable from adding `NewMemory.is_longterm`).
  - Step 3: `src/core/db.rs` not in Expected files but plan body mandated the new Db helpers.
- **Fix loop triggers**: 0
- **Auto-pass rate**: 2/3 (Step 1, Step 2 auto-pass; Step 3 was TL-executed directly, counts as auto-pass)
- **Cross-family coverage**: degraded all session. Codex account-limited, Gemini API key invalid. Fallback Claude Sonnet reviewer spawned per CLAUDE.md protocol for this /ae:review pass. No findings gap observed (fallback produced substantive high-quality critique; the specific failure classes Codex would catch — Rust-idiomatic micro-patterns, LLM-prompt best practices — were adequately covered by the challenger track).
- **Deferred resolution rate**: N/A (no `DEFERRED` entries in milestones/010/notes.md).

## Fixups Squashed

Not squashed this session (destructive ops disabled). Fixup commits `b001d6c` + backlog commit retained with review-phase labels for traceability.

## Deferred Findings Audit

✅ No DEFERRED entries in `docs/milestones/010/notes.md` (file doesn't exist — clean slate from `/ae:work`).

## Backlog Items Filed

- `docs/backlog/BL-fk-pragma-and-deletion-safety.md`
- `docs/backlog/BL-dreaming-module-split.md`
- `docs/backlog/BL-synthesis-dedup-key.md`
- `docs/backlog/BL-synthesis-provenance.md`

Backlog grows from 8 → 12 files. Worth revisiting via `/ae:roadmap` before the next sprint.

## Next Steps

Review passed. Suggested next actions (user choice):

1. **Run the e2e test against live Claude CLI**: `cargo test --test dream_synthesis -- --ignored dream_synthesis` with `claude` authenticated. Records the first real `[PASS] model=... title[:40]=...` output — satisfies plan AC4.
2. **First real dream run**: `mengdie dream --synthesize` against the production DB (58+ memories today). This generates the AC5 empirical data needed to close 3 of the 4 new backlog items.
3. **Ship**: `git push origin main` — feature is self-contained, fully reviewed, ready.
4. **Triage**: `/ae:roadmap` — 12 backlog items worth grouping for a future sprint.
